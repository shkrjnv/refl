// @vitest-environment jsdom

import "fake-indexeddb/auto";

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { LearnerStorage } from "../db/storage";
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

function renderLibrary(storage: LearnerStorage | null = null) {
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
        storage={storage}
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
    expect((await screen.findAllByText("92% understood")).length).toBeGreaterThan(0);
    const gift = screen
      .getByRole("heading", { name: "The Gift of the Magi" })
      .closest(".book-card");
    expect(gift).not.toBeNull();
    fireEvent.click(
      within(gift as HTMLElement).getByRole("button", { name: "Open book" }),
    );
    expect(
      screen.getByRole("heading", { level: 1, name: "The Gift of the Magi" }),
    ).toBeTruthy();
    const prose = document.querySelector(".reader-paper")?.textContent ?? "";
    expect(prose).toContain("ONE dollar and eighty-seven cents.");
    expect(prose).toContain("“Mr. James Dillingham Young.”");
    expect(document.querySelector(".reader-sentence-action")).toBeNull();
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
        text: expect.stringContaining("grey cat"),
        bookId: "book.ohenry-gift-magi",
        sentenceId: expect.stringContaining("sentence.ohenry-gift-magi."),
      },
    });
  });

  it("shows a personal book with its cover and removes it from the shelf", async () => {
    const storage = await LearnerStorage.open(`library-${crypto.randomUUID()}`);
    const book = {
      id: "book.personal-test",
      version: "1".repeat(64),
      title: { en: "My Upload" },
      level: "A2",
      license: "personal",
      source: "Uploaded by you",
      chapters: [],
    };
    await storage.putPersonalBook({
      learner_id: storage.learnerId,
      book,
      imported_at: 1,
      bytes: JSON.stringify(book).length,
    });
    await storage.putBookCover({
      learner_id: storage.learnerId,
      book_id: book.id,
      data_url: "data:image/webp;base64,AAAA",
      updated_at: 1,
    });
    try {
      renderLibrary(storage);
      expect(
        await screen.findByRole("heading", { name: "My Upload" }),
      ).toBeTruthy();
      expect(document.querySelector(".book-cover img")).not.toBeNull();
      fireEvent.click(
        screen.getByRole("button", {
          name: "Remove My Upload from the shelf",
        }),
      );
      await waitFor(() =>
        expect(screen.queryByRole("heading", { name: "My Upload" })).toBeNull(),
      );
    } finally {
      await storage.wipe();
    }
  });
});
