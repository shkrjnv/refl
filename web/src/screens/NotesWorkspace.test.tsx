// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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

records.notes["n3"] = {
  note_id: "n3",
  anchor: { kind: "lexeme", id: "en.lex.cat.animal" },
  body: "# cat\n\nA small animal with soft fur.",
  deleted: false,
  event_id: "n3",
  updated_at: 0,
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
  const view = render(
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
  return { store, onStateChange, unmount: view.unmount };
}

describe("notes workspace", () => {
  it("reads a note with an outline, backlinks, properties, and wiki navigation", () => {
    renderWorkspace();
    expect(screen.getByRole("heading", { level: 2, name: "Workspace note" })).toBeTruthy();
    expect(screen.getByRole("tab", { name: "Workspace note" }).getAttribute("aria-selected")).toBe(
      "true",
    );
    expect(screen.getByRole("button", { name: /Section/ })).toBeTruthy();
    fireEvent.click(screen.getByRole("tab", { name: "Backlinks & properties" }));
    expect(screen.getByText("Linked from")).toBeTruthy();
    expect(screen.getByText("Links to")).toBeTruthy();
    const properties = screen.getByRole("table");
    expect(within(properties).getByText("Anchored to")).toBeTruthy();
    expect(within(properties).getByText("Links in")).toBeTruthy();
    // The collapsible block under the text lists both directions.
    expect(screen.getByText("Linked notes")).toBeTruthy();
    fireEvent.click(screen.getAllByRole("button", { name: "the other note" })[0] as HTMLElement);
    expect(screen.getByRole("heading", { level: 2, name: "The other note" })).toBeTruthy();
    // Following a link opens a second tab and keeps the first.
    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
      "Workspace note",
      "The other note",
      "Outline",
      "Backlinks & properties",
      "Local graph",
    ]);
  });

  it("filters the explorer by anchor and search", () => {
    renderWorkspace();
    const explorer = screen.getByRole("complementary", { name: "All notes" });
    expect(within(explorer).getByText("3 notes")).toBeTruthy();
    fireEvent.click(within(explorer).getByRole("button", { name: /^Vocabulary/ }));
    expect(within(explorer).getByRole("button", { name: /cat/ })).toBeTruthy();
    expect(within(explorer).queryByRole("button", { name: /Workspace note/ })).toBeNull();
    fireEvent.click(within(explorer).getByRole("button", { name: /^Books/ }));
    expect(within(explorer).getByText("No notes in this group yet.")).toBeTruthy();
    fireEvent.click(within(explorer).getByRole("button", { name: /^All/ }));
    fireEvent.change(within(explorer).getByRole("searchbox"), {
      target: { value: "points back" },
    });
    expect(within(explorer).getByRole("button", { name: /The other note/ })).toBeTruthy();
    expect(within(explorer).queryByRole("button", { name: /Workspace note/ })).toBeNull();
  });

  it("edits with the toolbar and saves through the event writer", async () => {
    const { store, onStateChange } = renderWorkspace();
    const toolbar = screen.getByRole("toolbar", { name: "Formatting" });
    expect(within(toolbar).getByRole("button", { name: "Bold" })).toHaveProperty("disabled", true);
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    const editor = screen.getByPlaceholderText(
      "Write a thought in Markdown. It stays on this device and in your log.",
    ) as HTMLTextAreaElement;
    fireEvent.change(editor, { target: { value: "# Changed\n\nNew body." } });
    expect(screen.getByText(/3 words/)).toBeTruthy();
    expect(screen.getByText("Not saved")).toBeTruthy();
    editor.setSelectionRange(11, 14);
    fireEvent.click(within(toolbar).getByRole("button", { name: "Bold" }));
    expect(editor.value).toBe("# Changed\n\n**New** body.");
    fireEvent.keyDown(editor, { key: "s", ctrlKey: true });
    await waitFor(() => expect(store.saveNoteRecord).toHaveBeenCalled());
    expect(vi.mocked(store.saveNoteRecord).mock.calls[0]?.[1]).toMatchObject({
      noteId: "n1",
      anchor: { kind: "standalone", id: null },
      body: "# Changed\n\n**New** body.",
    });
    expect(onStateChange).toHaveBeenCalledWith("next");
  });

  it("refuses a note over 4 KB and counts bytes, not characters", () => {
    renderWorkspace();
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    const editor = screen.getByPlaceholderText(
      "Write a thought in Markdown. It stays on this device and in your log.",
    );
    fireEvent.change(editor, { target: { value: `# Big\n\n${"ж".repeat(2100)}` } });
    expect(screen.getByText("4,207 / 4,096 bytes")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Save note" })).toHaveProperty("disabled", true);
  });

  it("creates a note in a new tab and saves it when the tab closes", async () => {
    const store = writer();
    vi.mocked(store.saveNoteRecord).mockResolvedValue({ stateJson: "next", noteId: "n9" });
    renderWorkspace(store);
    fireEvent.click(screen.getAllByRole("button", { name: "New note" })[0] as HTMLElement);
    const editor = screen.getByPlaceholderText(
      "Write a thought in Markdown. It stays on this device and in your log.",
    );
    fireEvent.change(editor, { target: { value: "# Fresh idea\n\nAn hour, a uniform." } });
    expect(screen.getByRole("tab", { name: /Fresh idea/ })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Close Fresh idea" }));
    await waitFor(() => expect(store.saveNoteRecord).toHaveBeenCalled());
    expect(vi.mocked(store.saveNoteRecord).mock.calls[0]?.[1]).toEqual({
      anchor: { kind: "standalone" },
      body: "# Fresh idea\n\nAn hour, a uniform.",
    });
    await waitFor(() => expect(screen.queryByRole("tab", { name: /Fresh idea/ })).toBeNull());
  });

  it("asks before deleting and writes note_deleted", async () => {
    const { store } = renderWorkspace();
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(screen.getByText("Delete this note on every device?")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Keep" }));
    expect(store.deleteNote).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Delete" }));
    const confirm = screen.getByRole("alertdialog");
    fireEvent.click(within(confirm).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(store.deleteNote).toHaveBeenCalledWith("{}", "n1"));
  });

  it("navigates from the local graph to a linked note", () => {
    renderWorkspace();
    fireEvent.click(screen.getByRole("tab", { name: "Local graph" }));
    const graph = screen.getByRole("group", { name: "Notes and topics linked to this note" });
    const center = within(graph).getByRole("button", { name: "Note: Workspace note" });
    expect(center.getAttribute("aria-current")).toBe("true");
    fireEvent.click(within(graph).getByRole("button", { name: "Note: The other note" }));
    expect(screen.getByRole("heading", { level: 2, name: "The other note" })).toBeTruthy();
  });

  it("saves unsaved drafts when the learner leaves the workspace", async () => {
    const { store, unmount } = renderWorkspace();
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    fireEvent.change(
      screen.getByPlaceholderText(
        "Write a thought in Markdown. It stays on this device and in your log.",
      ),
      { target: { value: "# Workspace note\n\nLeft mid-thought." } },
    );
    unmount();
    await waitFor(() => expect(store.saveNoteRecord).toHaveBeenCalled());
    expect(vi.mocked(store.saveNoteRecord).mock.calls[0]?.[1]).toMatchObject({
      noteId: "n1",
      body: "# Workspace note\n\nLeft mid-thought.",
    });
  });
});
