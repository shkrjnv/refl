// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { I18nProvider } from "../i18n";
import type { PersonalRecords, RecordStore } from "../prototype/records";
import { NotesWorkspace } from "./NotesWorkspace";
import { testContent } from "./testContent";

afterEach(cleanup);

const records: PersonalRecords = {
  lexicalCards: {},
  customLexemes: {},
  notes: {
    n1: {
      note_id: "n1",
      anchor: { kind: "standalone", id: null },
      body: "# Workspace note\n\nA sentence links to [[note:n2|the other note]].\n\n## Section\n\nDetails.",
      deleted: false,
      event_id: "n1",
      updated_at: 2,
    },
    n2: {
      note_id: "n2",
      anchor: { kind: "standalone", id: null },
      body: "# The other note\n\nIt points back to [[note:n1]].\n\n## Other section\n\nText.",
      deleted: false,
      event_id: "n2",
      updated_at: 1,
    },
  },
  bookMarks: {},
};

function writer(): RecordStore {
  return {
    saveNoteRecord: vi.fn(async () => ({ stateJson: "next", noteId: "n1" })),
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

function renderWorkspace(store = writer()) {
  const onStateChange = vi.fn();
  render(
    <I18nProvider initialLocale="en">
      <NotesWorkspace
        records={records}
        content={testContent()}
        stateJson="{}"
        locale="en"
        recordsWriter={store}
        onStateChange={onStateChange}
        initialNoteId="n1"
      />
    </I18nProvider>,
  );
  return { store, onStateChange };
}

describe("notes workspace", () => {
  it("reads a note with an outline, incoming links, and wiki navigation", () => {
    renderWorkspace();
    expect(
      screen.getAllByRole("heading", { name: "Workspace note" }).length,
    ).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Section" })).toBeTruthy();
    fireEvent.click(screen.getByRole("tab", { name: "Backlinks" }));
    expect(screen.getByText("Linked from")).toBeTruthy();
    expect(screen.getByText("Links to")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "the other note" }));
    expect(
      screen.getAllByRole("heading", { level: 2, name: "The other note" }).length,
    ).toBeGreaterThan(0);
  });

  it("edits and saves a note through the event writer", async () => {
    const { store, onStateChange } = renderWorkspace();
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    const editor = screen.getByPlaceholderText(
      "Write a thought in Markdown. It stays on this device and in your log.",
    );
    fireEvent.change(editor, { target: { value: "# Changed\n\nNew body." } });
    expect(screen.getByText(/3 words/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Save note" }));
    await waitFor(() => expect(store.saveNoteRecord).toHaveBeenCalled());
    expect(vi.mocked(store.saveNoteRecord).mock.calls[0]?.[1]).toMatchObject({
      noteId: "n1",
      anchor: { kind: "standalone", id: null },
      body: "# Changed\n\nNew body.",
    });
    expect(onStateChange).toHaveBeenCalledWith("next");
  });
});
