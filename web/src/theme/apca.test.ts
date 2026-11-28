import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import { apcaAbsolute, apcaLc } from "./apca";
import {
  paletteNames,
  themes,
  type ThemeTokens,
} from "./tokens";

const cssSource = readFileSync(new URL("./tokens.css", import.meta.url), "utf8");

type SurfaceKey =
  | "bg"
  | "surface"
  | "surfaceRaised"
  | "surfaceMuted"
  | "surfaceCard"
  | "infoSurface"
  | "successSurface"
  | "warningSurface"
  | "dangerSurface";

const SURFACES: SurfaceKey[] = [
  "bg",
  "surface",
  "surfaceRaised",
  "surfaceMuted",
  "surfaceCard",
  "infoSurface",
  "successSurface",
  "warningSurface",
  "dangerSurface",
];

function expectLc(
  palette: string,
  label: string,
  text: string,
  background: string,
  target: number,
): void {
  const lc = apcaAbsolute(text, background);
  expect(lc, `${palette} ${label} (${text} on ${background})`).toBeGreaterThanOrEqual(
    target,
  );
}

describe("APCA perceptual contrast", () => {
  it("matches the APCA reference endpoints", () => {
    expect(apcaAbsolute("#000000", "#ffffff")).toBeCloseTo(106.04, 1);
    expect(apcaLc("#ffffff", "#000000")).toBeCloseTo(-107.88, 1);
  });

  it("calibrates every palette for body, metadata, actions, inputs, and display type", () => {
    for (const palette of paletteNames) {
      const theme: ThemeTokens = themes[palette];
      for (const surface of SURFACES) {
        expectLc(palette, `body copy on ${surface}`, theme.text, theme[surface], 75);
        expectLc(
          palette,
          `muted metadata on ${surface}`,
          theme.textMuted,
          theme[surface],
          60,
        );
      }

      expectLc(palette, "display heading on bg", theme.text, theme.bg, 50);
      expectLc(
        palette,
        "display heading on surface",
        theme.text,
        theme.surface,
        50,
      );
      expectLc(
        palette,
        "display heading on auth panel",
        theme.text,
        theme.authPanel,
        50,
      );

      expectLc(
        palette,
        "primary button text",
        theme.actionPrimaryText,
        theme.actionPrimaryBg,
        65,
      );
      expectLc(
        palette,
        "secondary button text",
        theme.text,
        theme.surface,
        65,
      );
      expectLc(
        palette,
        "danger button text",
        theme.actionDangerText,
        theme.actionDangerBg,
        65,
      );
      expectLc(
        palette,
        "ghost button text on surface",
        theme.actionGhostText,
        theme.surface,
        65,
      );
      expectLc(
        palette,
        "ghost button text on panel",
        theme.actionGhostText,
        theme.authPanel,
        65,
      );

      expectLc(
        palette,
        "input text",
        theme.text,
        theme.surfaceCard,
        75,
      );
      expectLc(
        palette,
        "input metadata",
        theme.textMuted,
        theme.surfaceCard,
        60,
      );
      expectLc(
        palette,
        "auth panel text",
        theme.authPanelText,
        theme.authPanel,
        75,
      );
      expectLc(
        palette,
        "auth panel metadata",
        theme.authPanelMuted,
        theme.authPanel,
        60,
      );
    }
  });

  it("keeps the CSS palette blocks in step with the TypeScript tokens", () => {
    for (const palette of paletteNames) {
      const theme = themes[palette];
      const start = cssSource.indexOf(
        `:root[data-palette="${palette}"][data-theme] {`,
      );
      expect(start, `${palette} CSS block`).toBeGreaterThanOrEqual(0);
      const end = cssSource.indexOf("\n}", start);
      const block = cssSource.slice(start, end);
      for (const [cssName, value] of [
        ["--text-muted", theme.textMuted],
        ["--auth-panel-muted", theme.authPanelMuted],
        ["--action-primary-bg", theme.actionPrimaryBg],
        ["--action-primary-text", theme.actionPrimaryText],
        ["--action-danger-bg", theme.actionDangerBg],
        ["--action-danger-text", theme.actionDangerText],
        ["--action-ghost-text", theme.actionGhostText],
      ] as const) {
        expect(block, `${palette} ${cssName}`).toContain(
          `${cssName}: ${value};`,
        );
      }
    }
  });
});
