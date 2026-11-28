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

const X_MIN = 260;
const X_MAX = 740;
const Y_MIN = 160;
const Y_MAX = 840;

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
  // Two columns keep a 140 px card readable on a 360 px stage; the taller
  // row keeps three-line titles from touching the next row.
  const rowHeight = 300;
  const groupGap = 100;
  let cursor = Y_MIN;
  for (const group of groups.values()) {
    const rows = Math.ceil(group.length / 2);
    for (let row = 0; row < rows; row += 1) {
      const rowNodes = group.slice(row * 2, row * 2 + 2);
      const columns = rowNodes.length;
      rowNodes.forEach((node, column) => {
        const x =
          columns === 1
            ? 500
            : X_MIN + ((X_MAX - X_MIN) * column) / Math.max(1, columns - 1);
        positions.set(node.id, {
          x: clampX(x),
          y: cursor + row * rowHeight,
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
        const force = 24000 / (distance * distance);
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
      const force = (distance - 240) * 0.005 * cooling;
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
  // Hard separation for the narrow stage: a 140 px card is ~450 layout
  // units wide, so five cards need a wide ring. Larger sets fall back to a
  // tighter packing rather than pushing every card onto the clamp.
  const area = (X_MAX - X_MIN) * (Y_MAX - Y_MIN);
  const minimum = Math.round(
    Math.max(300, Math.min(440, Math.sqrt(area / Math.max(1, ids.length)) * 1.7)),
  );
  for (let pass = 0; pass < 24; pass += 1) {
    let moved = false;
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
        let dx = right.x - left.x;
        let dy = right.y - left.y;
        let distance = Math.hypot(dx, dy);
        if (distance >= minimum) {
          continue;
        }
        if (distance < 1) {
          const angle = ((i + 1) * 1.7 + (j + 1) * 0.9) % (Math.PI * 2);
          dx = Math.cos(angle);
          dy = Math.sin(angle);
          distance = 1;
        }
        const push = (minimum - distance) / 2;
        const ux = dx / distance;
        const uy = dy / distance;
        left.x = clampX(left.x - ux * push);
        left.y = clampY(left.y - uy * push);
        right.x = clampX(right.x + ux * push);
        right.y = clampY(right.y + uy * push);
        moved = true;
      }
    }
    if (!moved) {
      break;
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

/** Rendered card size in the same 0…1000 units as graph positions. */
export interface CardSize {
  width: number;
  height: number;
}

const CARD_PADDING = 10;
const CARD_GAP = 14;

/**
 * Fits measured card boxes inside the stage and separates overlapping cards.
 * Positions are percentages of the stage, so widths are converted with the
 * stage width and heights with the stage height before this call.
 */
export function fitCardPositions(
  positions: ReadonlyMap<string, Point>,
  sizes: ReadonlyMap<string, CardSize>,
  gap = CARD_GAP,
): Map<string, Point> {
  const out = new Map(
    [...positions.entries()].map(([id, point]) => [id, { ...point }]),
  );
  const ids = [...out.keys()].filter((id) => sizes.has(id));
  const clampPoint = (id: string) => {
    const point = out.get(id);
    const size = sizes.get(id);
    if (!point || !size) {
      return;
    }
    const halfWidth = Math.min(500, size.width / 2);
    const halfHeight = Math.min(500, size.height / 2);
    point.x = Math.max(
      halfWidth + CARD_PADDING,
      Math.min(1000 - halfWidth - CARD_PADDING, point.x),
    );
    point.y = Math.max(
      halfHeight + CARD_PADDING,
      Math.min(1000 - halfHeight - CARD_PADDING, point.y),
    );
  };
  for (const id of ids) {
    clampPoint(id);
  }
  for (let pass = 0; pass < 80; pass += 1) {
    let moved = false;
    for (let i = 0; i < ids.length; i += 1) {
      const left = out.get(ids[i] ?? "");
      const leftSize = sizes.get(ids[i] ?? "");
      if (!left || !leftSize) {
        continue;
      }
      for (let j = i + 1; j < ids.length; j += 1) {
        const right = out.get(ids[j] ?? "");
        const rightSize = sizes.get(ids[j] ?? "");
        if (!right || !rightSize) {
          continue;
        }
        const requiredX = (leftSize.width + rightSize.width) / 2 + gap;
        const requiredY = (leftSize.height + rightSize.height) / 2 + gap;
        let dx = right.x - left.x;
        let dy = right.y - left.y;
        if (Math.abs(dx) >= requiredX || Math.abs(dy) >= requiredY) {
          continue;
        }
        if (Math.abs(dx) < 0.01 && Math.abs(dy) < 0.01) {
          const angle = ((i + 1) * 1.7 + (j + 1) * 0.9) % (Math.PI * 2);
          dx = Math.cos(angle);
          dy = Math.sin(angle);
        }
        const pushX = (requiredX - Math.abs(dx)) / 2;
        const pushY = (requiredY - Math.abs(dy)) / 2;
        if (!Number.isFinite(dx) || !Number.isFinite(dy)) {
          continue;
        }
        if (pushX / requiredX <= pushY / requiredY) {
          const direction = dx < 0 ? -1 : 1;
          left.x -= direction * pushX;
          right.x += direction * pushX;
        } else {
          const direction = dy < 0 ? -1 : 1;
          left.y -= direction * pushY;
          right.y += direction * pushY;
        }
        clampPoint(ids[i] ?? "");
        clampPoint(ids[j] ?? "");
        moved = true;
      }
    }
    if (!moved) {
      break;
    }
  }
  return out;
}
