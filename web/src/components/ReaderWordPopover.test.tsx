// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../i18n";
import { ReaderWordPopover } from "./ReaderWordPopover";

afterEach(cleanup);

function renderPopover(onClose = vi.fn()) {
  render(
    <I18nProvider initialLocale="en">
      <ReaderWordPopover
        word="his"
        anchor={{ top: 100, right: 130, bottom: 120, left: 100 }}
        sense={null}
        locale="en"
        sentenceText="A lion is in his house."
        customDefinition=""
        onCustomDefinitionChange={() => undefined}
        busy={false}
        inVocabulary={false}
        onAdd={() => undefined}
        onClose={onClose}
      />
    </I18nProvider>,
  );
  return onClose;
}

describe("reader word popover", () => {
  it("uses the reader field and a labelled 44 px close control", () => {
    const onClose = renderPopover();
    expect(screen.getByRole("dialog", { name: "Word card" })).toBeTruthy();
    const field = screen.getByRole("textbox", { name: "English definition" });
    expect(field.classList.contains("reader-word-input")).toBe(true);
    const close = screen.getByRole("button", { name: "Close word card" });
    fireEvent.click(close);
    expect(onClose).toHaveBeenCalled();
  });
});
