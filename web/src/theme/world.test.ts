import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const appCss = readFileSync(new URL("../app.css", import.meta.url), "utf8");
const tokensCss = readFileSync(new URL("./tokens.css", import.meta.url), "utf8");
const tokensTs = readFileSync(new URL("./tokens.ts", import.meta.url), "utf8");
const locales = ["en", "ru", "uz"].map((locale) =>
  readFileSync(
    new URL(`../i18n/locales/${locale}.ftl`, import.meta.url),
    "utf8",
  ),
);

describe("W14 visual system", () => {
  it("restores a full-width photographic hero with a directional scrim", () => {
    expect(appCss).toMatch(
      /\.today-hero-banner \{[\s\S]*?min-height: 280px[\s\S]*?grid-column: 1 \/ -1/,
    );
    expect(appCss).toMatch(
      /\.today-hero-banner \.hero-illustration \{[\s\S]*?object-fit: cover/,
    );
    expect(appCss).toMatch(/\.today-hero-scrim \{[\s\S]*?linear-gradient/);
  });

  it("renders the auth title as solid ink without a gradient fill", () => {
    expect(appCss).toMatch(
      /\.auth-form-title \{[\s\S]*?color: var\(--ink-primary\)[\s\S]*?letter-spacing: -0\.03em/,
    );
    expect(appCss).not.toContain("auth-title-liquid");
    expect(appCss).not.toContain("background-clip: text");
    expect(appCss).not.toContain("-webkit-text-fill-color: transparent");
  });

  it("keeps only the four streamlined skins and no Pixel World traces", () => {
    expect(tokensTs).toContain('"living-studio"');
    expect(tokensTs).toContain('"soft-bento"');
    expect(tokensTs).toContain('"laboratory"');
    expect(tokensTs).toContain('"neo-brutalism"');
    expect(tokensTs).not.toContain("pixel-world");
    expect(tokensTs).not.toContain("soft-neo-brutalism");
    expect(tokensCss).not.toContain("pixel-world");
    expect(tokensCss).not.toContain("soft-neo-brutalism");
    expect(appCss).not.toContain("pixel-world");
    expect(appCss).not.toContain("soft-neo-brutalism");
    for (const source of locales) {
      expect(source).not.toContain("skin-pixel-world");
      expect(source).not.toContain("skin-soft-neo-brutalism");
    }
  });

  it("replaces metadata capsules with inline text and a six-pixel dot", () => {
    const statusBlock = appCss.match(/\.status-meta \{[\s\S]*?\n\}/)?.[0] ?? "";
    expect(statusBlock).toContain("display: inline-flex");
    expect(statusBlock).not.toContain("border");
    expect(statusBlock).not.toContain("background");
    expect(statusBlock).not.toContain("border-radius");
    expect(appCss).toMatch(
      /\.status-meta-dot::before \{[\s\S]*?width: 6px[\s\S]*?height: 6px[\s\S]*?border-radius: 50%/,
    );
    expect(appCss).not.toContain(".ui-badge");
  });
});
