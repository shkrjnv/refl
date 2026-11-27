import { describe, expect, it } from "vitest";

import { palette, themes } from "./tokens";

describe("Living Studio tokens", () => {
  it("keeps the reference palette in the TypeScript and CSS tokens", () => {
    expect(Object.keys(palette).sort()).toEqual([
      "bg",
      "blue",
      "coral",
      "green",
      "ink",
      "purp",
      "sun",
    ]);
    for (const key of Object.keys(palette) as Array<keyof typeof palette>) {
      expect(themes.light[key]).toBe(palette[key]);
    }
  });

  it("defines both themes with tactile surface and shadow tokens", () => {
    for (const theme of [themes.light, themes.dark]) {
      expect(theme.surface).toMatch(/^#/);
      expect(theme.border).toMatch(/^#/);
      expect(theme.shadowButton).toContain("0 4px 0");
      expect(theme.shadowCard).toContain("0 6px 0");
    }
  });
});
