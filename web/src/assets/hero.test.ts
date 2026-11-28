import { readFileSync, statSync } from "node:fs";

import { describe, expect, it } from "vitest";

const WIDTHS = [480, 768, 1120, 1600, 2048] as const;
const FORMATS = ["avif", "webp", "jpg"] as const;

/** One hero download must stay small; the whole set must fit one MiB. */
const BUDGET = {
  avif: 70 * 1024,
  webp: 150 * 1024,
  jpg: 200 * 1024,
  total: 1024 * 1024,
} as const;

function asset(path: string): { bytes: Buffer; size: number } {
  const url = new URL(path, import.meta.url);
  return { bytes: readFileSync(url), size: statSync(url).size };
}

function assertMagic(format: (typeof FORMATS)[number], bytes: Buffer): void {
  switch (format) {
    case "avif":
      expect(bytes.subarray(4, 12).toString("ascii")).toContain("ftyp");
      expect(bytes.subarray(8, 32).toString("ascii")).toContain("avif");
      break;
    case "webp":
      expect(bytes.subarray(0, 4).toString("ascii")).toBe("RIFF");
      expect(bytes.subarray(8, 12).toString("ascii")).toBe("WEBP");
      break;
    case "jpg":
      expect([...bytes.subarray(0, 3)]).toEqual([0xff, 0xd8, 0xff]);
      break;
  }
}

describe("Today hero asset budget", () => {
  it("ships responsive AVIF, WebP, and JPEG candidates", () => {
    let total = 0;
    for (const format of FORMATS) {
      let largest = 0;
      for (const width of WIDTHS) {
        const { bytes, size } = asset(`./hero-nature-${width}.${format}`);
        assertMagic(format, bytes);
        expect(size, `${format} ${width}`).toBeGreaterThan(0);
        largest = Math.max(largest, size);
        total += size;
      }
      expect(largest, `largest ${format}`).toBeLessThanOrEqual(
        BUDGET[format],
      );
    }
    expect(total).toBeLessThanOrEqual(BUDGET.total);
  });
});
