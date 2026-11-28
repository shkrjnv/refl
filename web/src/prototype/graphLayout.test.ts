import { describe, expect, it } from "vitest";

import {
  authoredPositions,
  fallbackPositions,
  hasUsableCoordinates,
  resolveGraphPositions,
} from "./graphLayout";

const nodes = [
  { id: "en.a", levelRank: 1, level: "A1", domain: "grammar", x: 0, y: 0 },
  { id: "en.b", levelRank: 1, level: "A1", domain: "grammar", x: 0, y: 0 },
  { id: "en.c", levelRank: 1, level: "A1", domain: "grammar", x: 0, y: 0 },
  { id: "en.d", levelRank: 2, level: "A2", domain: "grammar", x: 0, y: 0 },
  { id: "en.e", levelRank: 2, level: "A2", domain: "lexis", x: 0, y: 0 },
];

describe("knowledge graph layout fallback", () => {
  it("detects missing and identical coordinates", () => {
    expect(hasUsableCoordinates(nodes)).toBe(false);
    expect(
      hasUsableCoordinates(
        nodes.map((node, index) => ({ ...node, x: index * 10, y: index * 10 })),
      ),
    ).toBe(true);
  });

  it("places every zero-coordinate node on a distinct level/domain grid", () => {
    const positions = fallbackPositions(nodes);
    const points = [...positions.values()];
    expect(points).toHaveLength(nodes.length);
    expect(new Set(points.map((point) => `${point.x}:${point.y}`)).size).toBe(
      nodes.length,
    );
    for (const point of points) {
      expect(point.x).toBeGreaterThanOrEqual(260);
      expect(point.x).toBeLessThanOrEqual(740);
      expect(point.y).toBeGreaterThanOrEqual(160);
      expect(point.y).toBeLessThanOrEqual(840);
    }
    // A1 fills two columns first; the A2 groups start lower.
    const a1 = positions.get("en.a");
    const a2 = positions.get("en.d");
    expect(a1).toBeTruthy();
    expect(a2).toBeTruthy();
    expect((a2?.y ?? 0) > (a1?.y ?? 0)).toBe(true);
  });

  it("clamps authored coordinates away from the stage edges", () => {
    const positions = authoredPositions([
      { id: "zero", x: 0, y: 0 },
      { id: "full", x: 1000, y: 1000 },
    ]);
    expect(positions.get("zero")).toEqual({ x: 260, y: 160 });
    expect(positions.get("full")).toEqual({ x: 740, y: 840 });
  });

  it("uses the fallback before the deterministic web simulation", () => {
    const first = resolveGraphPositions(nodes, [], "content-v1", "web");
    const second = resolveGraphPositions(nodes, [], "content-v1", "web");
    expect(first.fallback).toBe(true);
    expect([...first.positions.values()]).toEqual([...second.positions.values()]);
    expect(
      new Set([...first.positions.values()].map((point) => `${point.x}:${point.y}`))
        .size,
    ).toBe(nodes.length);
  });
});

describe("graph stage clearance", () => {
  const live = [
    { id: "a", levelRank: 1, level: "A1", domain: "grammar", x: 0, y: 1000 },
    { id: "b", levelRank: 1, level: "A1", domain: "grammar", x: 0, y: 500 },
    { id: "c", levelRank: 1, level: "A1", domain: "grammar", x: 1000, y: 1000 },
    { id: "d", levelRank: 2, level: "A2", domain: "grammar", x: 1000, y: 500 },
    { id: "e", levelRank: 1, level: "A1", domain: "grammar", x: 500, y: 0 },
  ];

  it("keeps a 140×78 card fully inside a 310 px stage in both modes", () => {
    for (const mode of ["layered", "web"] as const) {
      const positions = resolveGraphPositions(live, [], "content-v1", mode).positions;
      const points = [...positions.values()];
      for (const point of points) {
        const x = (point.x / 1000) * 310;
        const y = (point.y / 1000) * 310;
        expect(x).toBeGreaterThanOrEqual(70);
        expect(x).toBeLessThanOrEqual(240);
        expect(y).toBeGreaterThanOrEqual(39);
        expect(y).toBeLessThanOrEqual(271);
      }
      for (let i = 0; i < points.length; i += 1) {
        for (let j = i + 1; j < points.length; j += 1) {
          const left = points[i];
          const right = points[j];
          if (!left || !right) {
            continue;
          }
          const dx = (Math.abs(left.x - right.x) / 1000) * 298;
          const dy = (Math.abs(left.y - right.y) / 1000) * 298;
          expect(dx < 140 && dy < 78).toBe(false);
        }
      }
    }
  });
});

describe("web explorer separation", () => {
  it("keeps nodes farther apart than a card on a small stage", () => {
    const resolved = resolveGraphPositions(nodes, [], "content-v1", "web");
    const points = [...resolved.positions.values()];
    for (let i = 0; i < points.length; i += 1) {
      for (let j = i + 1; j < points.length; j += 1) {
        const left = points[i];
        const right = points[j];
        if (!left || !right) {
          continue;
        }
        expect(Math.hypot(left.x - right.x, left.y - right.y)).toBeGreaterThan(
          320,
        );
      }
    }
  });
});
