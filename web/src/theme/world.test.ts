import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

const appCss = readFileSync(new URL("../app.css", import.meta.url), "utf8");
const tokensCss = readFileSync(new URL("./tokens.css", import.meta.url), "utf8");

describe("Pixel World skin", () => {
  it("draws pixel borders, hard offset shadows, a sky grid, and crisp Iella", () => {
    expect(appCss).toMatch(/\[data-skin="pixel-world"\] \.bento-card[\s\S]*?border: 2px solid var\(--border-strong\)[\s\S]*?box-shadow: 4px 4px 0 var\(--border-strong\)/);
    expect(appCss).toMatch(/\[data-skin="pixel-world"\] \.studio-main[\s\S]*?background-size: 24px 24px, 24px 24px, 100% 100%/);
    expect(appCss).toMatch(/\[data-skin="pixel-world"\] \.iella-sprite[\s\S]*?image-rendering: pixelated/);
    expect(appCss).toMatch(/\[data-skin="pixel-world"\] \.ui-button[\s\S]*?box-shadow: 3px 3px 0 var\(--border-strong\)/);
    expect(tokensCss).toMatch(/\.skin-preview-pixel-world/);
    expect(tokensCss).toMatch(/\.skin-swatch-pixel-world/);
  });
});

describe("liquid auth title", () => {
  it("clips an eight-second flowing gradient and freezes it for reduced motion", () => {
    expect(appCss).toMatch(/background-clip: text/);
    expect(appCss).toMatch(/-webkit-text-fill-color: transparent/);
    expect(appCss).toMatch(/@keyframes auth-title-liquid/);
    expect(appCss).toMatch(/animation: auth-title-liquid 8s linear infinite/);
    expect(appCss).toMatch(/prefers-reduced-motion: reduce[\s\S]*?animation: none/);
  });
});
