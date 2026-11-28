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
    expect(
      screen.getByText(
        "This word is not in the course bank. Add it with your own definition.",
      ),
    ).toBeTruthy();
    const example = screen.getByText("A lion is in his house.");
    expect(example.tagName).toBe("BLOCKQUOTE");
    expect(screen.getByText("Example")).toBeTruthy();
    const field = screen.getByRole("textbox", { name: "English definition" });
    expect(field.classList.contains("reader-word-input")).toBe(true);
    // The example sits above the definition field, not after the action.
    expect(
      example.compareDocumentPosition(field) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    const close = screen.getByRole("button", { name: "Close word card" });
    fireEvent.click(close);
    expect(onClose).toHaveBeenCalled();
  });
});
