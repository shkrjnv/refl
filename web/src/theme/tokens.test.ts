import { describe, expect, it } from "vitest";

import {
  DEFAULT_SKIN,
  DEFAULT_THEME,
  isSkinName,
  palette,
  skinLabelKeys,
  skinNames,
  themes,
} from "./tokens";

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

  it("defines the Deep Umber and Cream Paper signature palettes", () => {
    expect(DEFAULT_THEME).toBe("dark");
    expect(palette.bg).toBe("#FAF4E8");
    expect(palette.blue).toBe("#0D7A78");
    expect(palette.coral).toBe("#FF2E7E");
    expect(themes.light.surfaceCard).toBe("#FFFDF8");
    expect(themes.light.microAccent).toBe("#FF2E7E");
    expect(themes.dark.bg).toBe("#161210");
    expect(themes.dark.surface).toBe("#201A17");
    expect(themes.dark.blue).toBe("#2B9AF3");
    expect(themes.dark.text).toBe("#EFE6D5");
  });

  it("defines the selectable non-ASCII skins with localized label keys", () => {
    expect(skinNames).toEqual([
      "living-studio",
      "neo-brutalism",
      "soft-neo-brutalism",
      "soft-bento",
      "laboratory",
    ]);
    expect(DEFAULT_SKIN).toBe("living-studio");
    for (const skin of skinNames) {
      expect(isSkinName(skin)).toBe(true);
      expect(skinLabelKeys[skin]).toMatch(/^skin-/);
    }
    expect(isSkinName("pure-ascii")).toBe(false);
    expect(isSkinName(null)).toBe(false);
  });
});
