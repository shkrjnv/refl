import type { Edge } from "../generated/core/Edge";

/**
 * Knowledge-graph layout in one pixel coordinate space.
 *
 * Every card is sized from its text before layout, so positions, SVG edges
 * and HTML cards share the same numbers: a box is its centre plus its
 * rendered width and height in CSS pixels. Nothing is measured from the DOM
 * after rendering, and nothing is stretched by a non-uniform `viewBox`.
 */

export interface GraphNodeInput {
  id: string;
  /** Authored `layout_x`/`layout_y` in 0…1000. */
  x: number;
  y: number;
  levelRank: number;
  level: string;
  domain: string;
  title: string;
  statusLabel: string;
}

export interface Point {
  x: number;
  y: number;
}

/** A card: centre point and size, in CSS pixels. */
export interface Box extends Point {
  width: number;
  height: number;
}

export interface GraphLayout {
  boxes: Map<string, Box>;
  width: number;
  height: number;
  /** The bundle had no usable coordinates and tiers were derived from edges. */
  fallback: boolean;
}

export type GraphMode = "layered" | "web";

/** Measures a single line of text at a font size and weight, in pixels. */
export type TextMeasure = (text: string, fontSize: number, weight: number) => number;

/** Card typography and spacing; the CSS for `.graph-node` uses the same values. */
export const CARD = {
  titleSize: 13,
  titleWeight: 650,
  titleLineHeight: 1.3,
  statusSize: 11,
  statusWeight: 600,
  statusLineHeight: 1.3,
  padX: 14,
  padY: 12,
  border: 2,
  dot: 8,
  dotGap: 8,
  rowGap: 4,
  minWidth: 132,
  baseWidth: 184,
  maxWidth: 264,
} as const;

const NARROW = 520;

interface Spacing {
  margin: number;
  gapX: number;
  rowGap: number;
  tierGap: number;
}

function spacingFor(viewportWidth: number): Spacing {
  return viewportWidth < NARROW
    ? { margin: 12, gapX: 12, rowGap: 16, tierGap: 44 }
    : { margin: 28, gapX: 28, rowGap: 22, tierGap: 64 };
}

const NARROW_CHARS = new Set([..."iljtfrI.,:;'’!|()[] "]);
const WIDE_CHARS = new Set([..."mwMWжшщюфыЖШЩЮФЫМм"]);

/**
 * Conservative per-character width estimate used when no font metrics are
 * available (tests, very old browsers). It overestimates Geologica slightly
 * so a card never ends up shorter than its wrapped title.
 */
export function estimateTextWidth(text: string, fontSize: number, weight = 400): number {
  let em = 0;
  for (const char of text) {
    if (char === " ") {
      em += 0.28;
    } else if (NARROW_CHARS.has(char)) {
      em += 0.32;
    } else if (WIDE_CHARS.has(char)) {
      em += 0.9;
    } else if (/[0-9]/.test(char)) {
      em += 0.6;
    } else if (char !== char.toLocaleLowerCase()) {
      em += 0.72;
    } else {
      em += 0.6;
    }
  }
  return em * fontSize * (weight >= 600 ? 1.06 : 1);
}

/** Greedy word wrap that only ever breaks at whitespace. */
export function wrapWords(
  text: string,
  maxWidth: number,
  fontSize: number,
  weight: number,
  measure: TextMeasure = estimateTextWidth,
): string[] {
  const words = text.split(/\s+/).filter(Boolean);
  const lines: string[] = [];
  let current = "";
  for (const word of words) {
    const candidate = current ? `${current} ${word}` : word;
    if (!current || measure(candidate, fontSize, weight) <= maxWidth) {
      current = candidate;
    } else {
      lines.push(current);
      current = word;
    }
  }
  if (current) {
    lines.push(current);
  }
  return lines.length > 0 ? lines : [""];
}

/** Size of one card for a title and status label at a target width. */
export function cardSize(
  title: string,
  statusLabel: string,
  targetWidth: number,
  measure: TextMeasure = estimateTextWidth,
): { width: number; height: number } {
  const chrome = 2 * CARD.padX + 2 * CARD.border;
  const titleChrome = chrome + CARD.dot + CARD.dotGap;
  const longestWord = Math.max(
    0,
    ...title
      .split(/\s+/)
      .filter(Boolean)
      .map((word) => measure(word, CARD.titleSize, CARD.titleWeight)),
  );
  const statusWidth = measure(statusLabel, CARD.statusSize, CARD.statusWeight);
  const width = Math.ceil(
    Math.min(
      CARD.maxWidth,
      Math.max(targetWidth, longestWord + titleChrome, Math.min(statusWidth, 120) + chrome),
    ),
  );
  const titleLines = wrapWords(
    title,
    width - titleChrome,
    CARD.titleSize,
    CARD.titleWeight,
    measure,
  ).length;
  const statusLines = wrapWords(
    statusLabel,
    width - chrome,
    CARD.statusSize,
    CARD.statusWeight,
    measure,
  ).length;
  const height =
    2 * CARD.border +
    2 * CARD.padY +
    titleLines * CARD.titleSize * CARD.titleLineHeight +
    CARD.rowGap +
    statusLines * CARD.statusSize * CARD.statusLineHeight;
  return { width, height: Math.ceil(height) };
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

/** Longest `requires` path from a root, the same rule content-build uses. */
export function requiresDepth(
  ids: readonly string[],
  edges: readonly Pick<Edge, "from" | "to" | "kind">[],
): Map<string, number> {
  const known = new Set(ids);
  const prerequisites = new Map<string, string[]>();
  for (const edge of edges) {
    if (edge.kind === "requires" && known.has(edge.from) && known.has(edge.to)) {
      prerequisites.set(edge.from, [...(prerequisites.get(edge.from) ?? []), edge.to]);
    }
  }
  const depth = new Map<string, number>();
  const visiting = new Set<string>();
  const visit = (id: string): number => {
    const cached = depth.get(id);
    if (cached !== undefined) {
      return cached;
    }
    if (visiting.has(id)) {
      // Cycles are rejected by content-build; stay finite if one slips in.
      return 0;
    }
    visiting.add(id);
    const parents = prerequisites.get(id) ?? [];
    const value = parents.length === 0 ? 0 : 1 + Math.max(...parents.map(visit));
    visiting.delete(id);
    depth.set(id, value);
    return value;
  };
  for (const id of ids) {
    visit(id);
  }
  return depth;
}

interface Tier {
  ids: string[];
}

function tiersFor(
  nodes: readonly GraphNodeInput[],
  edges: readonly Pick<Edge, "from" | "to" | "kind">[],
  usable: boolean,
): Tier[] {
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const keyed = new Map<number, string[]>();
  if (usable) {
    // Authored layers share one `layout_y`; order inside a layer is `layout_x`.
    for (const node of nodes) {
      keyed.set(node.y, [...(keyed.get(node.y) ?? []), node.id]);
    }
    for (const ids of keyed.values()) {
      ids.sort(
        (left, right) =>
          (byId.get(left)?.x ?? 0) - (byId.get(right)?.x ?? 0) || left.localeCompare(right),
      );
    }
  } else {
    const depth = requiresDepth(
      nodes.map((node) => node.id),
      edges,
    );
    for (const node of nodes) {
      const key = (depth.get(node.id) ?? 0) * 1000 + node.levelRank;
      keyed.set(key, [...(keyed.get(key) ?? []), node.id]);
    }
    for (const ids of keyed.values()) {
      ids.sort((left, right) => {
        const a = byId.get(left);
        const b = byId.get(right);
        return (a?.domain ?? "").localeCompare(b?.domain ?? "") || left.localeCompare(right);
      });
    }
  }
  return [...keyed.entries()]
    .sort(([left], [right]) => left - right)
    .map(([, ids]) => ({ ids }));
}

function sizesFor(
  nodes: readonly GraphNodeInput[],
  viewportWidth: number,
  measure: TextMeasure,
): Map<string, { width: number; height: number }> {
  const spacing = spacingFor(viewportWidth);
  // Two cards side by side on a phone; a comfortable fixed width elsewhere.
  const target = Math.max(
    CARD.minWidth,
    Math.min(
      CARD.baseWidth,
      Math.floor((viewportWidth - 2 * spacing.margin - spacing.gapX) / 2),
    ),
  );
  return new Map(
    nodes.map((node) => [node.id, cardSize(node.title, node.statusLabel, target, measure)]),
  );
}

/**
 * Top-to-bottom tiers. A tier that is wider than the viewport wraps into
 * several centred rows, so a phone never needs to shrink the cards.
 */
export function layeredLayout(
  nodes: readonly GraphNodeInput[],
  edges: readonly Pick<Edge, "from" | "to" | "kind">[],
  viewportWidth: number,
  measure: TextMeasure = estimateTextWidth,
): GraphLayout {
  const usable = hasUsableCoordinates(nodes);
  const spacing = spacingFor(viewportWidth);
  const sizes = sizesFor(nodes, viewportWidth, measure);
  const available = Math.max(CARD.minWidth, viewportWidth - 2 * spacing.margin);
  const rows: { ids: string[]; width: number; height: number; tierEnd: boolean }[] = [];
  for (const tier of tiersFor(nodes, edges, usable)) {
    let row: string[] = [];
    let rowWidth = 0;
    const flush = (tierEnd: boolean) => {
      if (row.length === 0) {
        return;
      }
      rows.push({
        ids: row,
        width: rowWidth,
        height: Math.max(...row.map((id) => sizes.get(id)?.height ?? 0)),
        tierEnd,
      });
      row = [];
      rowWidth = 0;
    };
    for (const id of tier.ids) {
      const width = sizes.get(id)?.width ?? CARD.baseWidth;
      const next = row.length === 0 ? width : rowWidth + spacing.gapX + width;
      if (row.length > 0 && next > available) {
        flush(false);
        row = [id];
        rowWidth = width;
      } else {
        row.push(id);
        rowWidth = next;
      }
    }
    flush(true);
  }
  const widest = Math.max(0, ...rows.map((row) => row.width));
  const width = Math.ceil(Math.max(viewportWidth, widest + 2 * spacing.margin));
  const boxes = new Map<string, Box>();
  let cursor = spacing.margin;
  rows.forEach((row, index) => {
    let left = (width - row.width) / 2;
    for (const id of row.ids) {
      const size = sizes.get(id) ?? { width: CARD.baseWidth, height: 60 };
      boxes.set(id, {
        x: left + size.width / 2,
        y: cursor + row.height / 2,
        width: size.width,
        height: size.height,
      });
      left += size.width + spacing.gapX;
    }
    cursor += row.height;
    if (index < rows.length - 1) {
      cursor += row.tierEnd ? spacing.tierGap : spacing.rowGap;
    }
  });
  return {
    boxes,
    width,
    height: Math.ceil(cursor + spacing.margin),
    fallback: !usable,
  };
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

/** First 32 bits of `content_version` (hex) or an FNV hash of the string. */
export function seedFrom(text: string): number {
  if (/^[0-9a-f]{8}/i.test(text)) {
    return Number.parseInt(text.slice(0, 8), 16) >>> 0;
  }
  let seed = 2166136261;
  for (let index = 0; index < text.length; index += 1) {
    seed ^= text.charCodeAt(index);
    seed = Math.imul(seed, 16777619);
  }
  return seed >>> 0;
}

export function boxesOverlap(left: Box, right: Box, gap = 0): boolean {
  return (
    Math.abs(left.x - right.x) < (left.width + right.width) / 2 + gap &&
    Math.abs(left.y - right.y) < (left.height + right.height) / 2 + gap
  );
}

export interface SeparateOptions {
  gap: number;
  /** Horizontal bounds for box edges; vertical space is unbounded. */
  minX?: number;
  maxX?: number;
  /** Boxes that never move (the centre of a local graph). */
  fixed?: ReadonlySet<string>;
  maxPasses?: number;
}

/**
 * Pushes overlapping rectangles apart along the axis of least penetration.
 * With horizontal bounds, a push that would leave the bounds goes vertical
 * instead, so the loop always converges. Returns `true` when no overlap is
 * left.
 */
export function separateBoxes(boxes: Map<string, Box>, options: SeparateOptions): boolean {
  const ids = [...boxes.keys()].sort();
  const { gap, minX = -Infinity, maxX = Infinity, fixed = new Set<string>() } = options;
  const passes = options.maxPasses ?? 800;
  const fitsX = (box: Box, x: number) =>
    x - box.width / 2 >= minX - 0.01 && x + box.width / 2 <= maxX + 0.01;
  for (let pass = 0; pass < passes; pass += 1) {
    let moved = false;
    for (let i = 0; i < ids.length; i += 1) {
      const a = boxes.get(ids[i] ?? "");
      if (!a) {
        continue;
      }
      for (let j = i + 1; j < ids.length; j += 1) {
        const b = boxes.get(ids[j] ?? "");
        if (!b || !boxesOverlap(a, b, gap)) {
          continue;
        }
        const aFixed = fixed.has(ids[i] ?? "");
        const bFixed = fixed.has(ids[j] ?? "");
        if (aFixed && bFixed) {
          continue;
        }
        const shareA = aFixed ? 0 : bFixed ? 1 : 0.5;
        const shareB = 1 - shareA;
        const dx = b.x - a.x;
        const dy = b.y - a.y;
        const penetrationX = (a.width + b.width) / 2 + gap - Math.abs(dx);
        const penetrationY = (a.height + b.height) / 2 + gap - Math.abs(dy);
        // Ties and coincident centres resolve by id order, never by chance.
        const signX = dx === 0 ? 1 : Math.sign(dx);
        const signY = dy === 0 ? 1 : Math.sign(dy);
        const nextAX = a.x - signX * penetrationX * shareA;
        const nextBX = b.x + signX * penetrationX * shareB;
        const horizontal =
          penetrationX <= penetrationY && fitsX(a, nextAX) && fitsX(b, nextBX);
        if (horizontal) {
          a.x = nextAX;
          b.x = nextBX;
        } else {
          a.y -= signY * penetrationY * shareA;
          b.y += signY * penetrationY * shareB;
        }
        moved = true;
      }
    }
    if (!moved) {
      return true;
    }
  }
  return false;
}

/** Moves every box so the set starts at `margin` and reports the extent. */
function normalize(
  boxes: Map<string, Box>,
  margin: number,
  minWidth: number,
): { width: number; height: number } {
  const all = [...boxes.values()];
  if (all.length === 0) {
    return { width: minWidth, height: 2 * margin };
  }
  const left = Math.min(...all.map((box) => box.x - box.width / 2));
  const top = Math.min(...all.map((box) => box.y - box.height / 2));
  const right = Math.max(...all.map((box) => box.x + box.width / 2));
  const bottom = Math.max(...all.map((box) => box.y + box.height / 2));
  const contentWidth = right - left;
  const width = Math.ceil(Math.max(minWidth, contentWidth + 2 * margin));
  const offsetX = (width - contentWidth) / 2 - left;
  for (const box of all) {
    box.x += offsetX;
    box.y += margin - top;
  }
  return { width, height: Math.ceil(bottom - top + 2 * margin) };
}

/**
 * Force-directed "web" view: a deterministic 300-step Fruchterman–Reingold
 * simulation seeded by `content_version`, started from the layered layout,
 * then a rectangle collision pass so no two cards overlap.
 */
export function webLayout(
  nodes: readonly GraphNodeInput[],
  edges: readonly Pick<Edge, "from" | "to" | "kind">[],
  contentVersion: string,
  viewport: { width: number; height: number },
  measure: TextMeasure = estimateTextWidth,
): GraphLayout {
  const base = layeredLayout(nodes, edges, viewport.width, measure);
  const spacing = spacingFor(viewport.width);
  const next = random(seedFrom(contentVersion));
  const boxes = new Map<string, Box>();
  const ids = nodes.map((node) => node.id).sort();
  for (const id of ids) {
    const start = base.boxes.get(id);
    if (start) {
      boxes.set(id, {
        ...start,
        x: start.x + next() * 40 - 20,
        y: start.y + next() * 40 - 20,
      });
    }
  }
  const all = [...boxes.values()];
  const meanWidth = all.reduce((sum, box) => sum + box.width, 0) / Math.max(1, all.length);
  const meanHeight = all.reduce((sum, box) => sum + box.height, 0) / Math.max(1, all.length);
  const ideal = Math.hypot(meanWidth, meanHeight) * 0.85;
  const aspect = Math.max(0.5, Math.min(2.4, viewport.width / Math.max(1, viewport.height)));
  const gravityX = 0.005 / Math.sqrt(aspect);
  const gravityY = 0.005 * Math.sqrt(aspect);
  const links = edges.filter((edge) => boxes.has(edge.from) && boxes.has(edge.to));
  for (let step = 0; step < 300; step += 1) {
    const temperature = ideal * 0.35 * (1 - step / 300) + 1;
    const shift = new Map(ids.map((id) => [id, { x: 0, y: 0 }]));
    for (let i = 0; i < ids.length; i += 1) {
      const a = boxes.get(ids[i] ?? "");
      const sa = shift.get(ids[i] ?? "");
      if (!a || !sa) {
        continue;
      }
      for (let j = i + 1; j < ids.length; j += 1) {
        const b = boxes.get(ids[j] ?? "");
        const sb = shift.get(ids[j] ?? "");
        if (!b || !sb) {
          continue;
        }
        let dx = a.x - b.x;
        let dy = a.y - b.y;
        let distance = Math.hypot(dx, dy);
        if (distance < 0.01) {
          dx = Math.cos(i + j);
          dy = Math.sin(i + j);
          distance = 1;
        }
        const force = (ideal * ideal) / distance;
        sa.x += (dx / distance) * force;
        sa.y += (dy / distance) * force;
        sb.x -= (dx / distance) * force;
        sb.y -= (dy / distance) * force;
      }
    }
    for (const edge of links) {
      const a = boxes.get(edge.from);
      const b = boxes.get(edge.to);
      const sa = shift.get(edge.from);
      const sb = shift.get(edge.to);
      if (!a || !b || !sa || !sb) {
        continue;
      }
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const distance = Math.max(0.01, Math.hypot(dx, dy));
      const force = (distance * distance) / ideal;
      sa.x += (dx / distance) * force;
      sa.y += (dy / distance) * force;
      sb.x -= (dx / distance) * force;
      sb.y -= (dy / distance) * force;
    }
    const centerX = base.width / 2;
    const centerY = base.height / 2;
    for (const id of ids) {
      const box = boxes.get(id);
      const s = shift.get(id);
      if (!box || !s) {
        continue;
      }
      s.x -= (box.x - centerX) * gravityX * ideal;
      s.y -= (box.y - centerY) * gravityY * ideal;
      const length = Math.hypot(s.x, s.y);
      if (length > 0) {
        const limited = Math.min(length, temperature);
        box.x += (s.x / length) * limited;
        box.y += (s.y / length) * limited;
      }
    }
  }
  separateBoxes(boxes, { gap: spacing.gapX });
  const extent = normalize(boxes, spacing.margin, viewport.width);
  return { boxes, ...extent, fallback: base.fallback };
}

export function resolveGraphLayout(
  nodes: readonly GraphNodeInput[],
  edges: readonly Pick<Edge, "from" | "to" | "kind">[],
  contentVersion: string,
  mode: GraphMode,
  viewport: { width: number; height: number },
  measure: TextMeasure = estimateTextWidth,
): GraphLayout {
  return mode === "web"
    ? webLayout(nodes, edges, contentVersion, viewport, measure)
    : layeredLayout(nodes, edges, viewport.width, measure);
}

/** Where the segment from `box` centre toward `toward` leaves the box. */
export function boxExit(box: Box, toward: Point, gap = 0): Point {
  const dx = toward.x - box.x;
  const dy = toward.y - box.y;
  if (dx === 0 && dy === 0) {
    return { x: box.x, y: box.y };
  }
  const tx = dx === 0 ? Infinity : box.width / 2 / Math.abs(dx);
  const ty = dy === 0 ? Infinity : box.height / 2 / Math.abs(dy);
  const t = Math.min(tx, ty);
  const length = Math.hypot(dx, dy);
  return {
    x: box.x + dx * t + (dx / length) * gap,
    y: box.y + dy * t + (dy / length) * gap,
  };
}

export interface EdgeGeometry {
  d: string;
  mid: Point;
}

/**
 * Edge path between two cards. Cards on different rows of the layered view
 * get a vertical S-curve from bottom edge to top edge; everything else is a
 * straight segment clipped to both card borders.
 */
export function edgeGeometry(from: Box, to: Box, mode: GraphMode, gap = 3): EdgeGeometry {
  const verticalGap = Math.abs(to.y - from.y) - (from.height + to.height) / 2;
  if (mode === "layered" && verticalGap > 8) {
    const down = to.y > from.y ? 1 : -1;
    const x1 = from.x;
    const y1 = from.y + (down * from.height) / 2 + down * gap;
    const x2 = to.x;
    const y2 = to.y - (down * to.height) / 2 - down * gap;
    const bend = (y2 - y1) / 2;
    return {
      d: `M ${x1} ${y1} C ${x1} ${y1 + bend} ${x2} ${y2 - bend} ${x2} ${y2}`,
      mid: { x: (x1 + x2) / 2, y: (y1 + y2) / 2 },
    };
  }
  const start = boxExit(from, to, gap);
  const end = boxExit(to, from, gap);
  return {
    d: `M ${start.x} ${start.y} L ${end.x} ${end.y}`,
    mid: { x: (start.x + end.x) / 2, y: (start.y + end.y) / 2 },
  };
}

export interface ViewTransform {
  scale: number;
  x: number;
  y: number;
}

export const MIN_ZOOM = 0.35;
export const MAX_ZOOM = 2;

/** Largest scale (≤ 1) at which the whole world fits, centred. */
export function fitTransform(
  world: { width: number; height: number },
  viewport: { width: number; height: number },
): ViewTransform {
  if (world.width <= 0 || world.height <= 0 || viewport.width <= 0 || viewport.height <= 0) {
    return { scale: 1, x: 0, y: 0 };
  }
  const scale = Math.max(
    MIN_ZOOM,
    Math.min(1, viewport.width / world.width, viewport.height / world.height),
  );
  return {
    scale,
    x: (viewport.width - world.width * scale) / 2,
    y: (viewport.height - world.height * scale) / 2,
  };
}

/** Zooms by `factor` keeping the viewport point `anchor` fixed. */
export function zoomAt(view: ViewTransform, factor: number, anchor: Point): ViewTransform {
  const scale = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, view.scale * factor));
  const ratio = scale / view.scale;
  return {
    scale,
    x: anchor.x - (anchor.x - view.x) * ratio,
    y: anchor.y - (anchor.y - view.y) * ratio,
  };
}

export interface EdgeLabel {
  id: string;
  at: Point;
  width: number;
  height: number;
}

/**
 * Nudges edge labels off cards and off each other. Cards are fixed; a label
 * moves the shortest way out, so it stays next to its edge.
 */
export function placeEdgeLabels(
  cards: ReadonlyMap<string, Box>,
  labels: readonly EdgeLabel[],
  gap = 3,
): Map<string, Point> {
  const boxes = new Map<string, Box>();
  const fixed = new Set<string>();
  for (const [id, box] of cards) {
    boxes.set(`card:${id}`, { ...box });
    fixed.add(`card:${id}`);
  }
  for (const label of labels) {
    boxes.set(`label:${label.id}`, {
      x: label.at.x,
      y: label.at.y,
      width: label.width,
      height: label.height,
    });
  }
  separateBoxes(boxes, { gap, fixed });
  return new Map(
    labels.map((label) => {
      const box = boxes.get(`label:${label.id}`);
      return [label.id, box ? { x: box.x, y: box.y } : label.at];
    }),
  );
}
