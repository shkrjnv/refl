import type { Edge } from "../generated/core/Edge";

export interface GraphNodeLike {
  id: string;
  levelRank: number;
  level: string;
  domain: string;
}

export interface Point {
  x: number;
  y: number;
}

const X_MIN = 160;
const X_MAX = 840;
const Y_MIN = 90;
const Y_MAX = 910;

function clampX(value: number): number {
  return Math.max(X_MIN, Math.min(X_MAX, value));
}

function clampY(value: number): number {
  return Math.max(Y_MIN, Math.min(Y_MAX, value));
}

export function hasUsableCoordinates(
  nodes: readonly { x: number; y: number }[],
): boolean {
  if (nodes.length === 0) {
    return true;
  }
  if (nodes.length === 1) {
    return nodes[0]?.x !== 0 || nodes[0]?.y !== 0;
  }
  const positions = new Set(nodes.map((node) => `${node.x}:${node.y}`));
  return positions.size === nodes.length;
}

/**
 * Deterministic fallback when a cached bundle has no layout (or every node
 * has the same coordinates). Groups by level and domain, wraps at three
 * columns, and leaves a vertical gap between groups. The result is in the
 * same 0…1000 space as the authored layout.
 */
export function fallbackPositions(
  nodes: readonly GraphNodeLike[],
): Map<string, Point> {
  const sorted = [...nodes].sort(
    (left, right) =>
      left.levelRank - right.levelRank ||
      left.level.localeCompare(right.level) ||
      left.domain.localeCompare(right.domain) ||
      left.id.localeCompare(right.id),
  );
  const groups = new Map<string, GraphNodeLike[]>();
  for (const node of sorted) {
    const key = `${node.levelRank}:${node.level}:${node.domain}`;
    groups.set(key, [...(groups.get(key) ?? []), node]);
  }
  const positions = new Map<string, Point>();
  const rowHeight = 190;
  const groupGap = 90;
  let cursor = Y_MIN;
  for (const group of groups.values()) {
    const rows = Math.ceil(group.length / 3);
    for (let row = 0; row < rows; row += 1) {
      const rowNodes = group.slice(row * 3, row * 3 + 3);
      const columns = rowNodes.length;
      rowNodes.forEach((node, column) => {
        const x =
          columns === 1
            ? 500
            : X_MIN + ((X_MAX - X_MIN) * column) / Math.max(1, columns - 1);
        positions.set(node.id, {
          x: clampX(x),
          y: clampY(cursor + row * rowHeight),
        });
      });
    }
    cursor += rows * rowHeight + groupGap;
  }
  // A single group of many nodes would run past the bottom: compress it
  // linearly into the available vertical band.
  const ys = [...positions.values()].map((point) => point.y);
  const lowest = Math.max(...ys, Y_MIN);
  if (lowest > Y_MAX) {
    const span = lowest - Y_MIN;
    for (const point of positions.values()) {
      point.y = clampY(Y_MIN + ((point.y - Y_MIN) * (Y_MAX - Y_MIN)) / span);
    }
  }
  return positions;
}

/** Authored coordinates are clamped away from the stage edges for labels. */
export function authoredPositions(
  nodes: readonly { id: string; x: number; y: number }[],
): Map<string, Point> {
  return new Map(
    nodes.map((node) => [
      node.id,
      {
        x: clampX(X_MIN + (node.x / 1000) * (X_MAX - X_MIN)),
        y: clampY(Y_MIN + (node.y / 1000) * (Y_MAX - Y_MIN)),
      },
    ]),
  );
}

function random(seed: number): () => number {
  let value = seed;
  return () => {
    value += 0x6d2b79f5;
    let t = value;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function seedFrom(text: string): number {
  let seed = 2166136261;
  for (let index = 0; index < text.length; index += 1) {
    seed ^= text.charCodeAt(index);
    seed = Math.imul(seed, 16777619);
  }
  return seed >>> 0;
}

/** Deterministic 300-step force layout seeded by `contentVersion`. */
export function webPositions(
  nodes: readonly GraphNodeLike[],
  initial: Map<string, Point>,
  edges: readonly Pick<Edge, "from" | "to">[],
  contentVersion: string,
): Map<string, Point> {
  const next = random(seedFrom(contentVersion));
  const positions = new Map(
    nodes.map((node) => {
      const start = initial.get(node.id) ?? { x: 500, y: 500 };
      return [
        node.id,
        { x: start.x + next() * 50 - 25, y: start.y + next() * 50 - 25 },
      ];
    }),
  );
  const ids = nodes.map((node) => node.id);
  const links = edges.filter(
    (edge) => positions.has(edge.from) && positions.has(edge.to),
  );
  for (let step = 0; step < 300; step += 1) {
    const cooling = 1 - step / 300;
    for (let i = 0; i < ids.length; i += 1) {
      const left = positions.get(ids[i] ?? "");
      if (!left) {
        continue;
      }
      for (let j = i + 1; j < ids.length; j += 1) {
        const right = positions.get(ids[j] ?? "");
        if (!right) {
          continue;
        }
        let dx = left.x - right.x;
        let dy = left.y - right.y;
        const distance = Math.max(1, Math.hypot(dx, dy));
        const force = 9000 / (distance * distance);
        dx /= distance;
        dy /= distance;
        left.x += dx * force * cooling;
        left.y += dy * force * cooling;
        right.x -= dx * force * cooling;
        right.y -= dy * force * cooling;
      }
    }
    for (const edge of links) {
      const left = positions.get(edge.from);
      const right = positions.get(edge.to);
      if (!left || !right) {
        continue;
      }
      const dx = right.x - left.x;
      const dy = right.y - left.y;
      const distance = Math.max(1, Math.hypot(dx, dy));
      const force = (distance - 220) * 0.004 * cooling;
      const ux = dx / distance;
      const uy = dy / distance;
      left.x += ux * force;
      left.y += uy * force;
      right.x -= ux * force;
      right.y -= uy * force;
    }
    for (const id of ids) {
      const point = positions.get(id);
      if (point) {
        point.x = clampX(point.x * 0.995 + 500 * 0.005);
        point.y = clampY(point.y * 0.995 + 500 * 0.005);
      }
    }
  }
  return positions;
}

export function resolveGraphPositions(
  nodes: readonly { id: string; x: number; y: number; levelRank: number; level: string; domain: string }[],
  edges: readonly Pick<Edge, "from" | "to">[],
  contentVersion: string,
  mode: "layered" | "web",
): { positions: Map<string, Point>; fallback: boolean } {
  const fallback = !hasUsableCoordinates(nodes);
  const base = fallback
    ? fallbackPositions(
        nodes.map((node) => ({
          id: node.id,
          levelRank: node.levelRank,
          level: node.level,
          domain: node.domain,
        })),
      )
    : authoredPositions(nodes);
  if (mode === "web") {
    return {
      positions: webPositions(nodes, base, edges, contentVersion),
      fallback,
    };
  }
  return { positions: base, fallback };
}
