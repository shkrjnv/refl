// @vitest-environment jsdom

import { readFileSync } from "node:fs";

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import type { Locale } from "../i18n";
import { LanguageMenu } from "./LanguageMenu";

afterEach(cleanup);

describe("compact language switcher", () => {
  it("keeps the hover and focus expansion rules in the stylesheet", () => {
    const css = readFileSync("src/app.css", "utf8");
    expect(css).toMatch(/\.language-menu:hover \.language-options/);
    expect(css).toMatch(/\.language-menu:focus-within \.language-options/);
  });

  it("shows the active code and exposes every language to keyboard and pointer users", () => {
    let selected: Locale = "en";
    render(
      <I18nProvider initialLocale="en">
        <LanguageMenu
          locale="en"
          locales={["en", "ru", "uz"]}
          onLocaleChange={(next) => {
            selected = next;
          }}
        />
      </I18nProvider>,
    );

    expect(screen.getAllByText("EN").length).toBeGreaterThan(0);
    const trigger = screen.getByRole("button", { name: "Language" });
    fireEvent.click(trigger);
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    const russian = screen.getByRole("option", { name: "Russian" });
    fireEvent.click(russian);
    expect(selected).toBe("ru");
  });
});
