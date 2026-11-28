// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../i18n";
import type { PersonalRecords, RecordStore } from "../prototype/records";
import { NotesPanel } from "./NotesPanel";

afterEach(cleanup);

const emptyRecords: PersonalRecords = {
  lexicalCards: {},
  customLexemes: {},
  notes: {},
  bookMarks: {},
};

function writer(): RecordStore {
  return {
    saveNote: vi.fn(async () => "next"),
    deleteNote: vi.fn(async () => "next"),
    saveCustomWord: vi.fn(async () => "next"),
    archiveCustomWord: vi.fn(async () => "next"),
    reviewLexeme: vi.fn(async () => "next"),
    createBookMark: vi.fn(async () => ({ stateJson: "next", markId: "m" })),
    deleteBookMark: vi.fn(async () => "next"),
    selectCompanion: vi.fn(async () => "next"),
  } as unknown as RecordStore;
}

function renderPanel(records = emptyRecords, store = writer()) {
  const onStateChange = vi.fn();
  render(
    <I18nProvider initialLocale="en">
      <NotesPanel
        records={records}
        anchor={{ kind: "standalone" }}
        recordsWriter={store}
        stateJson="{}"
        onStateChange={onStateChange}
      />
    </I18nProvider>,
  );
  return { store, onStateChange };
}

describe("private notes panel", () => {
  it("saves a Markdown note under its anchor", async () => {
    const { store, onStateChange } = renderPanel();
    fireEvent.change(screen.getByLabelText(/New note/), {
      target: { value: "**First** thought\n\n- one" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save note" }));
    await waitFor(() => expect(store.saveNote).toHaveBeenCalled());
    expect(vi.mocked(store.saveNote).mock.calls[0]?.[1]).toMatchObject({
      anchor: { kind: "standalone" },
      body: "**First** thought\n\n- one",
    });
    expect(onStateChange).toHaveBeenCalledWith("next");
  });

  it("renders, edits and deletes an existing note", async () => {
    const records: PersonalRecords = {
      ...emptyRecords,
      notes: {
        n1: {
          note_id: "n1",
          anchor: { kind: "standalone", id: null },
          body: "Old note",
          deleted: false,
          event_id: "e1",
          updated_at: 1,
        },
      },
    };
    const { store } = renderPanel(records);
    expect(screen.getByText("Old note")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    expect((screen.getByLabelText(/Edit note/) as HTMLTextAreaElement).value).toBe(
      "Old note",
    );
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    await waitFor(() =>
      expect(store.deleteNote).toHaveBeenCalledWith("{}", "n1"),
    );
  });

  it("refuses a note over 4096 bytes", () => {
    renderPanel();
    const body = "a".repeat(4097);
    fireEvent.change(screen.getByLabelText(/New note/), {
      target: { value: body },
    });
    expect(screen.getByText(/4,097 \/ 4,096 bytes/)).toBeTruthy();
    expect(
      (screen.getByRole("button", { name: "Save note" }) as HTMLButtonElement)
        .disabled,
    ).toBe(true);
  });
});
