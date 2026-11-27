import { describe, expect, it } from "vitest";

import {
  DEFAULT_SKIN,
  DEFAULT_THEME,
  isDarkPalette,
  isPaletteName,
  isSkinName,
  palette,
  paletteLabelKeys,
  paletteNames,
  skinLabelKeys,
  skinNames,
  themes,
} from "./tokens";

describe("Living Studio tokens", () => {
  it("keeps the cream-paper reference palette in the TypeScript tokens", () => {
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
      expect(themes["cream-paper"][key]).toBe(palette[key]);
    }
  });

  it("defines four independent palettes with tactile tokens", () => {
    expect(paletteNames).toEqual([
      "deep-umber",
      "cream-paper",
      "sand-forest",
      "midnight-cobalt",
    ]);
    expect(DEFAULT_THEME).toBe("deep-umber");
    expect(isDarkPalette("deep-umber")).toBe(true);
    expect(isDarkPalette("midnight-cobalt")).toBe(true);
    expect(isDarkPalette("cream-paper")).toBe(false);
    expect(isDarkPalette("sand-forest")).toBe(false);
    for (const name of paletteNames) {
      expect(isPaletteName(name)).toBe(true);
      expect(paletteLabelKeys[name]).toMatch(/^palette-/);
      const theme = themes[name];
      expect(theme.surface).toMatch(/^#|^rgba/);
      expect(theme.border).toMatch(/^#|^rgba/);
      expect(theme.shadowButton).toContain("0 4px 0");
      expect(theme.shadowCard).toContain("0 6px 0");
    }
    expect(isPaletteName("pure-ascii")).toBe(false);
    expect(isPaletteName(null)).toBe(false);
  });

  it("keeps the four signature color stories", () => {
    expect(themes["cream-paper"].bg).toBe("#FAF4E8");
    expect(themes["cream-paper"].blue).toBe("#0D7A78");
    expect(themes["cream-paper"].coral).toBe("#FF2E7E");
    expect(themes["deep-umber"].bg).toBe("#161210");
    expect(themes["deep-umber"].surface).toBe("#201A17");
    expect(themes["deep-umber"].blue).toBe("#2B9AF3");
    expect(themes["sand-forest"].blue).toBe("#1F5C48");
    expect(themes["sand-forest"].sun).toBe("#E7A438");
    expect(themes["midnight-cobalt"].blue).toBe("#3D6BFF");
    expect(themes["midnight-cobalt"].sun).toBe("#E9B45E");
  });

  it("keeps the canonical Iella colors on every palette", () => {
    for (const name of paletteNames) {
      expect(themes[name].iellaInk).toBe("#24346A");
      expect(themes[name].iellaBeak).toBe("#E0773C");
      expect(themes[name].iellaCream).toBe("#F3E3C3");
      expect(themes[name].iellaBrown).toBe("#8B5A2B");
      expect(themes[name].iellaEye).toBe("#0E1020");
    }
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
