// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../i18n";
import type { RecordStore } from "../prototype/records";
import type { BookCoverageResult } from "../wasm/types";
import { LibraryScreen } from "./LibraryScreen";
import { testContent } from "./testContent";

afterEach(cleanup);

const state = "{}";

function fakeStore(): RecordStore {
  return {
    saveNote: vi.fn(async () => state),
    deleteNote: vi.fn(async () => state),
    saveCustomWord: vi.fn(async () => state),
    archiveCustomWord: vi.fn(async () => state),
    reviewLexeme: vi.fn(async () => state),
    createBookMark: vi.fn(async () => ({
      stateJson: state,
      markId: "01930000-0000-7000-8000-000000000002",
    })),
    deleteBookMark: vi.fn(async () => state),
    selectCompanion: vi.fn(async () => state),
  } as unknown as RecordStore;
}

function coverage(): Promise<BookCoverageResult> {
  return Promise.resolve({
    book: "book.first-day",
    version: "v",
    coverage: { known_tokens: 23, total_tokens: 25, percent: 0.92 },
  });
}

function renderLibrary() {
  const store = fakeStore();
  render(
    <I18nProvider initialLocale="en">
      <LibraryScreen
        content={testContent()}
        stateJson={state}
        locale="en"
        recordsWriter={store}
        onStateChange={() => undefined}
        bookCoverage={coverage}
        onPractice={() => undefined}
      />
    </I18nProvider>,
  );
  return store;
}

describe("library and reader", () => {
  it("shows live grammar depths and L1 traps", () => {
    renderLibrary();
    fireEvent.click(screen.getByRole("tab", { name: "Grammar" }));
    expect(screen.getByRole("heading", { name: "a or an" })).toBeTruthy();
    expect(screen.getByText("Use *a* before a consonant sound.")).toBeTruthy();
    fireEvent.click(screen.getByRole("tab", { name: "L1 traps" }));
    expect(
      screen.getByRole("heading", { name: "Article omitted" }),
    ).toBeTruthy();
    expect(
      screen.getByText("Say the article even when your first language skips it."),
    ).toBeTruthy();
  });

  it("computes coverage and adds a tapped book word with its sentence", async () => {
    const store = renderLibrary();
    expect(await screen.findByText("92% understood")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Open book" }));
    expect(
      screen.getByRole("heading", { name: "The First Day" }),
    ).toBeTruthy();
    const cat = screen
      .getAllByRole("button", { name: /^cat\b/ })
      .find((button) => button.classList.contains("reader-token"));
    expect(cat).toBeTruthy();
    fireEvent.click(cat as HTMLElement);
    expect(screen.getByRole("heading", { name: "cat" })).toBeTruthy();
    expect(screen.getByText("a small animal with soft fur")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Add to vocabulary" }));
    await waitFor(() => expect(store.saveCustomWord).toHaveBeenCalled());
    const call = vi.mocked(store.saveCustomWord).mock.calls[0];
    expect(call?.[1]).toMatchObject({
      lemma: "cat",
      bundleLexemeId: "en.lex.cat.animal",
      source: "book",
      context: {
        text: "Mira sees a cat in the garden.",
        bookId: "book.first-day",
        sentenceId: "sentence.first-day.1.3.1",
      },
    });
  });
});
