// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";

import {
  applyAppearance,
  applySkin,
  initialSkin,
  initialTheme,
  readStoredSkin,
  readStoredTheme,
  storeSkin,
  storeTheme,
} from "./tokens";

afterEach(() => {
  window.localStorage.clear();
  document.documentElement.removeAttribute("data-skin");
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.removeAttribute("data-palette");
  document.documentElement.style.removeProperty("color-scheme");
});

describe("skin and palette selection", () => {
  it("persists the chosen skin and restores it on the next start", () => {
    expect(initialSkin()).toBe("living-studio");
    storeSkin("laboratory");
    expect(readStoredSkin()).toBe("laboratory");
    expect(initialSkin()).toBe("laboratory");
  });

  it("defaults to Deep Umber and persists every custom palette", () => {
    expect(initialTheme()).toBe("deep-umber");
    for (const palette of [
      "cream-paper",
      "sand-forest",
      "midnight-cobalt",
      "burgundy",
      "deep-umber",
    ] as const) {
      storeTheme(palette);
      expect(readStoredTheme()).toBe(palette);
      expect(initialTheme()).toBe(palette);
      expect(window.localStorage.getItem("refl.palette")).toBe(palette);
    }
  });

  it("keeps palette and skin choices independent", () => {
    storeSkin("laboratory");
    storeTheme("sand-forest");
    expect(readStoredSkin()).toBe("laboratory");
    expect(readStoredTheme()).toBe("sand-forest");
    storeSkin("soft-bento");
    expect(readStoredTheme()).toBe("sand-forest");
  });

  it("migrates the previous day/night storage values", () => {
    window.localStorage.setItem("refl.theme", "light");
    expect(readStoredTheme()).toBe("cream-paper");
    window.localStorage.setItem("refl.theme", "dark");
    expect(readStoredTheme()).toBe("deep-umber");
  });

  it("independently applies palette and skin on the document root", () => {
    applyAppearance("midnight-cobalt", "soft-bento");
    expect(document.documentElement.dataset["palette"]).toBe("midnight-cobalt");
    expect(document.documentElement.dataset["theme"]).toBe("dark");
    expect(document.documentElement.dataset["skin"]).toBe("soft-bento");
    expect(document.documentElement.style.colorScheme).toBe("dark");

    applyAppearance("burgundy", "living-studio");
    expect(document.documentElement.dataset["palette"]).toBe("burgundy");
    expect(document.documentElement.dataset["theme"]).toBe("dark");
    expect(document.documentElement.dataset["skin"]).toBe("living-studio");
    expect(document.documentElement.style.colorScheme).toBe("dark");

    applyAppearance("sand-forest", "laboratory");
    expect(document.documentElement.dataset["palette"]).toBe("sand-forest");
    expect(document.documentElement.dataset["theme"]).toBe("light");
    expect(document.documentElement.dataset["skin"]).toBe("laboratory");
    expect(document.documentElement.style.colorScheme).toBe("light");

    applySkin("neo-brutalism");
    expect(document.documentElement.dataset["skin"]).toBe("neo-brutalism");
  });

  it("persists and applies the Pixel World skin", () => {
    storeSkin("pixel-world");
    expect(readStoredSkin()).toBe("pixel-world");
    expect(initialSkin()).toBe("pixel-world");
    applySkin("pixel-world");
    expect(document.documentElement.dataset["skin"]).toBe("pixel-world");
  });

  it("ignores unknown or reserved values", () => {
    storeSkin("laboratory");
    window.localStorage.setItem("refl.skin", "pure-ascii");
    expect(readStoredSkin()).toBeNull();
    expect(initialSkin()).toBe("living-studio");
  });
});
