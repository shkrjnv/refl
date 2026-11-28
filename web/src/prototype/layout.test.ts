// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";

import {
  LAYOUT_KEY,
  defaultLayout,
  normalizeLayout,
  readLayout,
  storeLayout,
} from "./layout";

afterEach(() => {
  window.localStorage.clear();
});

describe("navigation layout persistence", () => {
  it("stores and restores only the navigation position", () => {
    storeLayout({ nav: "bottom" });
    expect(window.localStorage.getItem(LAYOUT_KEY)).toBe('{"nav":"bottom"}');
    expect(readLayout()).toEqual({ nav: "bottom" });
  });

  it("reads the previous widget layout without re-enabling widget editing", () => {
    window.localStorage.setItem(
      LAYOUT_KEY,
      JSON.stringify({
        nav: "right",
        order: ["activity", "hero"],
        colSpan: { activity: 2 },
        rowSpan: { hero: "tall" },
      }),
    );
    expect(readLayout()).toEqual({ nav: "right" });
  });

  it("repairs missing or unknown navigation positions", () => {
    expect(normalizeLayout(null)).toEqual(defaultLayout());
    expect(normalizeLayout({ nav: "sideways" })).toEqual(defaultLayout());
    expect(normalizeLayout({ nav: "left" })).toEqual({ nav: "left" });
  });
});
