import { describe, expect, it } from "vitest";

import {
  CARD,
  boxExit,
  boxesOverlap,
  cardSize,
  edgeGeometry,
  estimateTextWidth,
  fitTransform,
  hasUsableCoordinates,
  layeredLayout,
  placeEdgeLabels,
  requiresDepth,
  resolveGraphLayout,
  separateBoxes,
  wrapWords,
  zoomAt,
  type Box,
  type GraphLayout,
  type GraphNodeInput,
} from "./graphLayout";

function node(
  id: string,
  x: number,
  y: number,
  title: string,
  levelRank = 1,
): GraphNodeInput {
  return {
    id,
    x,
    y,
    levelRank,
    level: levelRank === 1 ? "A1" : "A2",
    domain: "grammar",
    title,
    statusLabel: "Not started",
  };
}

// The five live Articles KCs with their authored layers.
const live = [
  node("en.gram.noun.countability", 500, 0, "Countable and uncountable nouns"),
  node("en.gram.article.first-mention", 250, 500, "First mention: a / an"),
  node("en.gram.article.zero-general", 750, 500, "No article for things in general", 2),
  node("en.gram.article.second-mention", 250, 1000, "Second mention: the"),
  node("en.gram.article.a-an-sound", 750, 1000, "a or an: the first sound decides"),
];

const edges = [
  { from: "en.gram.article.first-mention", to: "en.gram.noun.countability", kind: "requires" as const },
  { from: "en.gram.article.second-mention", to: "en.gram.article.first-mention", kind: "requires" as const },
  { from: "en.gram.article.zero-general", to: "en.gram.noun.countability", kind: "requires" as const },
  { from: "en.gram.article.a-an-sound", to: "en.gram.article.first-mention", kind: "requires" as const },
  { from: "en.gram.article.first-mention", to: "en.gram.article.second-mention", kind: "contrasts" as const },
];

// A long-title set in Russian and Uzbek, wider than one phone row.
const crowded = [
  ...live,
  node("a", 100, 500, "Неисчисляемые существительные без артикля"),
  node("b", 900, 500, "Sanaladigan va sanalmaydigan otlar"),
  node("c", 500, 1000, "Притяжательный падеж и предлоги места"),
];

function assertClean(layout: GraphLayout, count: number) {
  const boxes = [...layout.boxes.values()];
  expect(boxes).toHaveLength(count);
  for (const box of boxes) {
    expect(box.x - box.width / 2).toBeGreaterThanOrEqual(0);
    expect(box.y - box.height / 2).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width / 2).toBeLessThanOrEqual(layout.width);
    expect(box.y + box.height / 2).toBeLessThanOrEqual(layout.height);
    expect(box.height).toBeGreaterThanOrEqual(44);
  }
  for (let i = 0; i < boxes.length; i += 1) {
    for (let j = i + 1; j < boxes.length; j += 1) {
      const left = boxes[i] as Box;
      const right = boxes[j] as Box;
      expect(boxesOverlap(left, right, 8)).toBe(false);
    }
  }
}

describe("card text", () => {
  it("wraps only at spaces, never inside a word", () => {
    const lines = wrapWords("Countable and uncountable nouns", 90, 13, 650);
    expect(lines.length).toBeGreaterThan(1);
    expect(lines.join(" ")).toBe("Countable and uncountable nouns");
    for (const line of lines) {
      for (const word of line.split(" ")) {
        expect("Countable and uncountable nouns".split(" ")).toContain(word);
      }
    }
  });

  it("widens a card so its longest word fits beside the status dot", () => {
    const word = "Sanalmaydiganlarning";
    const size = cardSize(word, "Not started", CARD.minWidth);
    const room =
      size.width - 2 * CARD.padX - 2 * CARD.border - CARD.dot - CARD.dotGap;
    expect(estimateTextWidth(word, CARD.titleSize, CARD.titleWeight)).toBeLessThanOrEqual(room);
  });

  it("grows the card height with the number of title lines", () => {
    const one = cardSize("Plural", "Learning", 184);
    const three = cardSize("Countable and uncountable nouns in general statements", "Learning", 140);
    expect(three.height).toBeGreaterThan(one.height);
  });
});

describe("layered layout", () => {
  it("uses authored layers top to bottom", () => {
    const layout = layeredLayout(live, edges, 1100);
    const y = (id: string) => layout.boxes.get(id)?.y ?? 0;
    expect(layout.fallback).toBe(false);
    expect(y("en.gram.noun.countability")).toBeLessThan(y("en.gram.article.first-mention"));
    expect(y("en.gram.article.first-mention")).toBe(y("en.gram.article.zero-general"));
    expect(y("en.gram.article.first-mention")).toBeLessThan(
      y("en.gram.article.second-mention"),
    );
  });

  it("derives tiers from requires edges when the bundle has no layout", () => {
    const zero = live.map((entry) => ({ ...entry, x: 0, y: 0 }));
    expect(hasUsableCoordinates(zero)).toBe(false);
    const depth = requiresDepth(
      zero.map((entry) => entry.id),
      edges,
    );
    expect(depth.get("en.gram.noun.countability")).toBe(0);
    expect(depth.get("en.gram.article.a-an-sound")).toBe(2);
    const layout = layeredLayout(zero, edges, 1100);
    expect(layout.fallback).toBe(true);
    assertClean(layout, zero.length);
  });

  it("keeps the starter map at full size on a phone", () => {
    const layout = layeredLayout(live, edges, 326);
    expect(layout.width).toBe(326);
    assertClean(layout, live.length);
  });
});

describe("both modes fit at 360 and 1440 px", () => {
  const viewports = [
    { width: 326, height: 420 },
    { width: 1080, height: 620 },
  ];
  for (const viewport of viewports) {
    for (const mode of ["layered", "web"] as const) {
      it(`${mode} at ${viewport.width} px has no clipped or overlapping card`, () => {
        for (const set of [live, crowded]) {
          const layout = resolveGraphLayout(set, edges, "3f2a9c01", mode, viewport);
          assertClean(layout, set.length);
          const view = fitTransform(layout, viewport);
          expect(layout.width * view.scale).toBeLessThanOrEqual(viewport.width + 0.5);
          if (mode === "web") {
            expect(layout.height * view.scale).toBeLessThanOrEqual(viewport.height + 0.5);
          }
        }
      });
    }
  }

  it("is deterministic for one content version", () => {
    const viewport = { width: 1080, height: 620 };
    const first = resolveGraphLayout(crowded, edges, "3f2a9c01", "web", viewport);
    const second = resolveGraphLayout(crowded, edges, "3f2a9c01", "web", viewport);
    expect([...first.boxes.entries()]).toEqual([...second.boxes.entries()]);
  });
});

describe("geometry helpers", () => {
  it("separates boxes inside horizontal bounds by moving vertically", () => {
    const boxes = new Map<string, Box>([
      ["a", { x: 100, y: 100, width: 180, height: 40 }],
      ["b", { x: 100, y: 100, width: 180, height: 40 }],
      ["c", { x: 110, y: 104, width: 180, height: 40 }],
    ]);
    expect(separateBoxes(boxes, { gap: 8, minX: 0, maxX: 200 })).toBe(true);
    const all = [...boxes.values()];
    for (const box of all) {
      expect(box.x - box.width / 2).toBeGreaterThanOrEqual(-0.01);
      expect(box.x + box.width / 2).toBeLessThanOrEqual(200.01);
    }
    expect(boxesOverlap(all[0] as Box, all[1] as Box, 8)).toBe(false);
    expect(boxesOverlap(all[1] as Box, all[2] as Box, 8)).toBe(false);
  });

  it("clips edges to card borders", () => {
    const box = { x: 100, y: 100, width: 80, height: 40 };
    expect(boxExit(box, { x: 300, y: 100 })).toEqual({ x: 140, y: 100 });
    expect(boxExit(box, { x: 100, y: 0 })).toEqual({ x: 100, y: 80 });
    const curve = edgeGeometry(box, { x: 100, y: 300, width: 80, height: 40 }, "layered", 0);
    expect(curve.d.startsWith("M 100 120 C")).toBe(true);
    expect(curve.d.endsWith("100 280")).toBe(true);
  });

  it("moves edge labels off cards and off each other", () => {
    const cards = new Map<string, Box>([["a", { x: 100, y: 100, width: 120, height: 60 }]]);
    const placed = placeEdgeLabels(cards, [
      { id: "one", at: { x: 100, y: 120 }, width: 90, height: 18 },
      { id: "two", at: { x: 104, y: 122 }, width: 90, height: 18 },
    ]);
    const boxes = [...placed.values()].map((point) => ({ ...point, width: 90, height: 18 }));
    for (const box of boxes) {
      expect(boxesOverlap(box, cards.get("a") as Box)).toBe(false);
    }
    expect(boxesOverlap(boxes[0] as Box, boxes[1] as Box)).toBe(false);
  });

  it("zooms around a fixed anchor", () => {
    const view = zoomAt({ scale: 1, x: 0, y: 0 }, 2, { x: 100, y: 50 });
    expect(view).toEqual({ scale: 2, x: -100, y: -50 });
  });
});
