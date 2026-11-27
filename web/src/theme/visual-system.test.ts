import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const appCss = readFileSync(new URL("../app.css", import.meta.url), "utf8");
const tokensCss = readFileSync(new URL("./tokens.css", import.meta.url), "utf8");
const w18Start = appCss.indexOf("/* W18");

function block(css: string, selector: string, from = 0): string {
  const start = css.indexOf(selector, from);
  expect(start, selector).toBeGreaterThanOrEqual(0);
  const open = css.indexOf("{", start);
  const close = css.indexOf("}", open);
  return css.slice(open + 1, close);
}

describe("visual system geometry", () => {
  it("keeps quiet skins flat and gives Neo-brutalism only crisp 1:1 offsets", () => {
    const rootFallback = block(tokensCss, ":root {", tokensCss.indexOf("/* W18"));
    expect(rootFallback).toContain("--shadow-card: none;");
    expect(
      tokensCss.match(/--shadow-(?:button|primary|card|hero|soft):/g),
    ).toHaveLength(15);

    const quiet = block(
      tokensCss,
      ':root[data-skin]:not([data-skin="neo-brutalism"])[data-palette][data-theme] {',
    );
    for (const shadow of [
      "button",
      "primary",
      "card",
      "hero",
      "soft",
    ]) {
      expect(quiet).toContain(`--shadow-${shadow}: none;`);
    }

    const neo = block(
      tokensCss,
      ':root[data-skin="neo-brutalism"][data-palette][data-theme] {',
    );
    const offsets = [
      ...neo.matchAll(
        /--shadow-[a-z]+: (\d+)px (\d+)px 0 var\(--text\);/g,
      ),
    ];
    expect(offsets).toHaveLength(5);
    for (const match of offsets) {
      expect(match[1]).toBe(match[2]);
    }
    expect(neo).not.toMatch(/\d+px \d+px [1-9]\d*px/);
  });

  it("uses borders, not shadows, for secondary actions and navigation", () => {
    const secondary = block(appCss, ".ui-button-secondary {", w18Start);
    expect(secondary).toContain("border: 1px solid var(--border)");
    const danger = block(appCss, ".ui-button-danger {", w18Start);
    expect(danger).toContain("box-shadow: var(--shadow-button)");
    const mobileNav = block(
      appCss,
      ".studio-shell[data-nav-position] .mobile-nav {",
      w18Start,
    );
    expect(mobileNav).toContain("box-shadow: none");
    const tour = block(appCss, ".iella-tour-highlight {", w18Start);
    expect(tour).toContain("box-shadow: none");
    const primaryFocus = block(
      appCss,
      ".auth-form-region .ui-button-primary:focus-visible {",
      w18Start,
    );
    expect(primaryFocus).toContain("box-shadow: var(--shadow-primary)");
    const secondaryFocus = block(
      appCss,
      ".auth-form-region .ui-button-secondary:focus-visible {",
      w18Start,
    );
    expect(secondaryFocus).toContain("box-shadow: none");
  });

  it("keeps the Laboratory grid calm and its controls roomy", () => {
    const labLight = block(tokensCss, ':root[data-skin="laboratory"] {');
    expect(labLight).toContain(
      "--bg-size: 28px 28px, 28px 28px, 140px 140px, 140px 140px;",
    );
    expect(labLight).toContain("rgba(16, 78, 84, 0.045)");
    expect(labLight).toContain("rgba(16, 78, 84, 0.085)");
    expect(
      tokensCss.match(
        /--bg-size: 28px 28px, 28px 28px, 140px 140px, 140px 140px;/g,
      ),
    ).toHaveLength(3);

    const content = block(
      appCss,
      '[data-skin="laboratory"] .studio-content {',
      w18Start,
    );
    expect(content).toContain("padding: 28px 48px 64px");
    const card = block(appCss, '[data-skin="laboratory"] .ui-card,', w18Start);
    expect(card).toContain("padding: 24px");
    const choice = block(
      appCss,
      '[data-skin="laboratory"] .choice-option {',
      w18Start,
    );
    expect(choice).toContain("min-height: 60px");
    expect(choice).toContain("padding: 12px 18px");
    const button = block(
      appCss,
      '[data-skin="laboratory"] .ui-button {',
      w18Start,
    );
    expect(button).toContain("min-height: 46px");
    expect(button).toContain("padding-inline: 20px");
    const tab = block(
      appCss,
      '[data-skin="laboratory"] .settings-tab {',
      w18Start,
    );
    expect(tab).toContain("min-height: 46px");
    expect(tab).toContain("padding: 0 16px");
  });
});
