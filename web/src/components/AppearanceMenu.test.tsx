// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import type { SkinName, ThemeName } from "../theme/tokens";
import { AppearanceMenu } from "./AppearanceMenu";

afterEach(cleanup);

describe("custom appearance selector", () => {
  it("opens the popover and switches skin and palette", () => {
    let skin: SkinName = "living-studio";
    let theme: ThemeName = "dark";
    render(
      <I18nProvider initialLocale="en">
        <AppearanceMenu
          theme={theme}
          skin={skin}
          onThemeChange={(next) => {
            theme = next;
          }}
          onSkinChange={(next) => {
            skin = next;
          }}
        />
      </I18nProvider>,
    );

    fireEvent.click(screen.getByRole("button", { name: "Appearance" }));
    expect(screen.getByRole("dialog", { name: "Appearance" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Laboratory/ }));
    expect(skin).toBe("laboratory");
    fireEvent.click(screen.getByRole("button", { name: /Cream Paper/ }));
    expect(theme).toBe("light");
  });
});
