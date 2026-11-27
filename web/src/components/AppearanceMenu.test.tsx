// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import type { PaletteName, SkinName, ThemeName } from "../theme/tokens";
import { AppearanceMenu, PaletteToggle } from "./AppearanceMenu";

afterEach(cleanup);

describe("appearance controls", () => {
  it("exposes all five palettes directly", () => {
    let theme: ThemeName = "deep-umber";
    const selected: PaletteName[] = [];
    render(
      <I18nProvider initialLocale="en">
        <PaletteToggle
          theme={theme}
          onThemeChange={(next) => {
            theme = next;
            selected.push(next);
          }}
        />
      </I18nProvider>,
    );

    for (const name of [
      "Deep Umber",
      "Burgundy Studio",
      "Midnight Cobalt",
      "Cream Paper",
      "Sand & Forest",
    ]) {
      expect(screen.getByRole("button", { name })).toBeTruthy();
    }
    fireEvent.click(screen.getByRole("button", { name: "Cream Paper" }));
    fireEvent.click(screen.getByRole("button", { name: "Burgundy Studio" }));
    expect(selected).toEqual(["cream-paper", "burgundy"]);
    expect(theme).toBe("burgundy");
  });

  it("keeps only the four streamlined layout skins", () => {
    let skin: SkinName = "living-studio";
    render(
      <I18nProvider initialLocale="en">
        <AppearanceMenu
          skin={skin}
          onSkinChange={(next) => {
            skin = next;
          }}
        />
      </I18nProvider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Skin" }));
    const skinDialog = screen.getByRole("dialog", { name: "Skin" });
    for (const name of [
      /Living Studio/,
      /Soft Bento/,
      /Laboratory/,
      /Neo-brutalism/,
    ]) {
      expect(within(skinDialog).getByRole("button", { name })).toBeTruthy();
    }
    expect(
      within(skinDialog).queryByRole("button", { name: /Pixel World/ }),
    ).toBeNull();
    expect(
      within(skinDialog).queryByRole("button", { name: /Soft neo-brutalism/ }),
    ).toBeNull();
    expect(
      within(skinDialog).queryByRole("button", { name: /Deep Umber/ }),
    ).toBeNull();

    fireEvent.click(within(skinDialog).getByRole("button", { name: /Soft Bento/ }));
    expect(skin).toBe("soft-bento");
    fireEvent.click(within(skinDialog).getByRole("button", { name: /Neo-brutalism/ }));
    expect(skin).toBe("neo-brutalism");
  });
});
