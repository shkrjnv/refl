// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../i18n";
import type { RecordStore } from "../prototype/records";
import { testContent } from "./testContent";
import { VocabularyScreen } from "./VocabularyScreen";

afterEach(cleanup);

const now = Date.now();
const dueState = JSON.stringify({
  lexical_cards: {
    "en.lex.cat.animal": {
      recognize: {
        stability: 2,
        difficulty: 4,
        last_review: now - 3 * 86_400_000,
        due: now - 1000,
        direction: "recognize",
        source: "bundle",
        exposures: 0,
        review_count: 1,
        lapse_count: 0,
      },
    },
  },
});

function fakeStore(overrides: Partial<RecordStore> = {}): RecordStore {
  return {
    saveNote: vi.fn(async (_state: string) => dueState),
    deleteNote: vi.fn(async (_state: string) => dueState),
    saveCustomWord: vi.fn(async (_state: string) => dueState),
    archiveCustomWord: vi.fn(async (_state: string) => dueState),
    reviewLexeme: vi.fn(async (_state: string) => dueState),
    createBookMark: vi.fn(async (_state: string) => ({
      stateJson: dueState,
      markId: "01930000-0000-7000-8000-000000000001",
    })),
    deleteBookMark: vi.fn(async (_state: string) => dueState),
    selectCompanion: vi.fn(async (_state: string) => dueState),
    ...overrides,
  } as unknown as RecordStore;
}

function renderScreen(state = dueState, store = fakeStore()) {
  const onStateChange = vi.fn();
  render(
    <I18nProvider initialLocale="en">
      <VocabularyScreen
        content={testContent()}
        stateJson={state}
        locale="en"
        recordsWriter={store}
        onStateChange={onStateChange}
      />
    </I18nProvider>,
  );
  return { store, onStateChange };
}

describe("vocabulary screen", () => {
  it("lists bank senses and filters by meaning", () => {
    renderScreen();
    expect(screen.getByRole("heading", { name: "cat" })).toBeTruthy();
    expect(screen.getByText("a small animal with soft fur")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("Find a word"), {
      target: { value: "zzz" },
    });
    expect(screen.queryByRole("heading", { name: "cat" })).toBeNull();
    expect(screen.getByText("No word matches this filter.")).toBeTruthy();
  });

  it("runs a focused review and logs lexical evidence", async () => {
    const { store, onStateChange } = renderScreen();
    fireEvent.click(screen.getByRole("button", { name: /Review 1 due/ }));
    expect(screen.getByText("Say the meaning before you turn the card.")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Show meaning" }));
    expect(screen.getByRole("heading", { name: "a small animal with soft fur" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "I knew it" }));
    await waitFor(() => {
      expect(store.reviewLexeme).toHaveBeenCalledWith(
        dueState,
        "en.lex.cat.animal",
        true,
      );
    });
    expect(onStateChange).toHaveBeenCalledWith(dueState);
  });

  it("creates a personal word and reuses the bank pronunciation", async () => {
    const { store } = renderScreen("{}");
    fireEvent.click(screen.getByRole("button", { name: "Add word" }));
    fireEvent.change(screen.getByLabelText("Word"), {
      target: { value: "cat" },
    });
    fireEvent.change(screen.getByLabelText("English definition"), {
      target: { value: "my own cat definition" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save word" }));
    await waitFor(() => {
      expect(store.saveCustomWord).toHaveBeenCalled();
    });
    const call = vi.mocked(store.saveCustomWord).mock.calls[0];
    expect(call?.[1]).toMatchObject({
      lemma: "cat",
      definitionEn: "my own cat definition",
      bundleLexemeId: "en.lex.cat.animal",
      ipa: "kæt",
    });
  });
});
