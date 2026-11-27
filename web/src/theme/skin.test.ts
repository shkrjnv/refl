// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";

import {
  applyAppearance,
  applySkin,
  initialSkin,
  initialTheme,
  readStoredSkin,
  storeSkin,
  storeTheme,
} from "./tokens";

afterEach(() => {
  window.localStorage.clear();
  document.documentElement.removeAttribute("data-skin");
  document.documentElement.removeAttribute("data-theme");
  document.documentElement.style.removeProperty("color-scheme");
});

describe("skin selection", () => {
  it("persists the chosen skin and restores it on the next start", () => {
    expect(initialSkin()).toBe("living-studio");
    storeSkin("laboratory");
    expect(readStoredSkin()).toBe("laboratory");
    expect(initialSkin()).toBe("laboratory");
  });

  it("defaults to Deep Umber and remembers an explicit palette", () => {
    expect(initialTheme()).toBe("dark");
    storeTheme("light");
    expect(initialTheme()).toBe("light");
  });

  it("applies skin and mode together on the document root", () => {
    applyAppearance("dark", "soft-bento");
    expect(document.documentElement.dataset.skin).toBe("soft-bento");
    expect(document.documentElement.dataset.theme).toBe("dark");
    expect(document.documentElement.style.colorScheme).toBe("dark");

    applySkin("neo-brutalism");
    expect(document.documentElement.dataset.skin).toBe("neo-brutalism");
  });

  it("ignores unknown or reserved values", () => {
    storeSkin("laboratory");
    window.localStorage.setItem("refl.skin", "pure-ascii");
    expect(readStoredSkin()).toBeNull();
    expect(initialSkin()).toBe("living-studio");
  });
});
