// @vitest-environment jsdom

import { readFileSync } from "node:fs";

import {
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { I18nProvider } from "../i18n";
import type { Locale } from "../i18n";
import { LanguageMenu } from "./LanguageMenu";

afterEach(cleanup);

describe("compact language switcher", () => {
  it("opens as an out-of-flow popover instead of widening the header", () => {
    const css = readFileSync("src/app.css", "utf8");
    expect(css).not.toMatch(/\.language-menu:hover \.language-options/);
    expect(css).not.toMatch(/\.language-menu:focus-within \.language-options/);
    const options = css.slice(
      css.indexOf(".language-options {"),
      css.indexOf(".language-menu[data-open"),
    );
    expect(options).toContain("position: absolute");
    expect(options).toContain("right: 0");
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

  it("closes on an outside pointer press or Escape", () => {
    render(
      <I18nProvider initialLocale="en">
        <LanguageMenu
          locale="en"
          locales={["en", "ru", "uz"]}
          onLocaleChange={() => undefined}
        />
      </I18nProvider>,
    );
    const trigger = screen.getByRole("button", { name: "Language" });

    fireEvent.click(trigger);
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    fireEvent.pointerDown(document.body);
    expect(trigger.getAttribute("aria-expanded")).toBe("false");

    fireEvent.click(trigger);
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    fireEvent.keyDown(document, { key: "Escape" });
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
  });

  it("highlights only the active language or the hovered/focused language", () => {
    render(
      <I18nProvider initialLocale="en">
        <LanguageMenu
          locale="en"
          locales={["en", "ru", "uz"]}
          onLocaleChange={() => undefined}
        />
      </I18nProvider>,
    );

    const english = screen.getByRole("option", { name: "English" });
    const russian = screen.getByRole("option", { name: "Russian" });
    expect(english.className).toContain("is-highlight");
    expect(russian.className).not.toContain("is-highlight");

    fireEvent.pointerEnter(russian);
    expect(russian.className).toContain("is-highlight");
    expect(english.className).not.toContain("is-highlight");

    fireEvent.pointerLeave(russian);
    expect(english.className).toContain("is-highlight");
    expect(russian.className).not.toContain("is-highlight");

    fireEvent.focus(russian);
    expect(russian.className).toContain("is-highlight");
    expect(english.className).not.toContain("is-highlight");

    fireEvent.blur(russian);
    expect(english.className).toContain("is-highlight");
    expect(russian.className).not.toContain("is-highlight");
  });
});
