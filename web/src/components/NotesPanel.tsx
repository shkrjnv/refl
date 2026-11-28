import { useMemo, useState } from "react";

import type { NoteAnchorKind } from "../generated/core/NoteAnchorKind";
import { useTranslation } from "../i18n";
import {
  activeNotes,
  type NoteView,
  type PersonalRecords,
  type RecordStore,
} from "../prototype/records";
import { RichText } from "./RichText";
import { Button } from "./ui";

export interface NotesPanelProps {
  records: PersonalRecords;
  anchor: { kind: NoteAnchorKind; id?: string | null };
  recordsWriter: RecordStore;
  stateJson: string;
  onStateChange: (stateJson: string) => void;
  title?: string;
  onClose?: () => void;
}

function noteAnchorMatches(
  note: NoteView,
  kind: NoteAnchorKind,
  id: string | null | undefined,
): boolean {
  return note.anchor.kind === kind && (note.anchor.id ?? null) === (id ?? null);
}

/** Renders the small Markdown subset notes allow, without HTML. */
export function NoteBody({ body }: { body: string }) {
  const lines = body.split("\n");
  const blocks: React.ReactNode[] = [];
  let list: string[] = [];
  const flush = (key: number) => {
    if (list.length > 0) {
      blocks.push(
        <ul key={`list-${key}`}>
          {list.map((item, index) => (
            <li key={index}>
              <RichText text={item} />
            </li>
          ))}
        </ul>,
      );
      list = [];
    }
  };
  lines.forEach((line, index) => {
    const trimmed = line.trim();
    if (trimmed.startsWith("- ") || trimmed.startsWith("* ")) {
      list.push(trimmed.slice(2));
      return;
    }
    flush(index);
    if (trimmed.startsWith("### ")) {
      blocks.push(<h4 key={index}>{trimmed.slice(4)}</h4>);
    } else if (trimmed.startsWith("## ")) {
      blocks.push(<h3 key={index}>{trimmed.slice(3)}</h3>);
    } else if (trimmed.startsWith("> ")) {
      blocks.push(<blockquote key={index}>{trimmed.slice(2)}</blockquote>);
    } else if (trimmed.length > 0) {
      blocks.push(<p key={index}><RichText text={trimmed} /></p>);
    }
  });
  flush(lines.length);
  return <div className="note-markdown">{blocks}</div>;
}

export function NotesPanel({
  records,
  anchor,
  recordsWriter,
  stateJson,
  onStateChange,
  title,
  onClose,
}: NotesPanelProps) {
  const { t } = useTranslation();
  const notes = useMemo(
    () => activeNotes(records, anchor),
    [records, anchor],
  );
  const [draft, setDraft] = useState("");
  const [editing, setEditing] = useState<NoteView | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const bytes = new TextEncoder().encode(draft).length;
  const save = async () => {
    setBusy(true);
    setError(null);
    try {
      const next = await recordsWriter.saveNote(stateJson, {
        noteId: editing?.note_id,
        anchor,
        body: draft,
      });
      onStateChange(next);
      setDraft("");
      setEditing(null);
    } catch {
      setError(t("notes-error-save"));
    } finally {
      setBusy(false);
    }
  };
  const remove = async (note: NoteView) => {
    setBusy(true);
    setError(null);
    try {
      onStateChange(await recordsWriter.deleteNote(stateJson, note.note_id));
      if (editing?.note_id === note.note_id) {
        setEditing(null);
        setDraft("");
      }
    } catch {
      setError(t("notes-error-save"));
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="notes-panel" aria-label={title ?? t("notes-panel-title")}>
      <header className="notes-panel-head">
        <h3>{title ?? t("notes-panel-title")}</h3>
        {onClose ? (
          <Button type="button" variant="ghost" size="sm" onClick={onClose}>
            {t("notes-close")}
          </Button>
        ) : null}
      </header>
      {notes.length === 0 ? (
        <p className="notes-empty">{t("notes-empty-anchor")}</p>
      ) : (
        <ul className="notes-list">
          {notes.map((note) => (
            <li key={note.note_id} className="note-item">
              <NoteBody body={note.body} />
              <div className="note-actions">
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    setEditing(note);
                    setDraft(note.body);
                  }}
                >
                  {t("notes-edit")}
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  disabled={busy}
                  onClick={() => void remove(note)}
                >
                  {t("notes-delete")}
                </Button>
              </div>
            </li>
          ))}
        </ul>
      )}
      <label className="field">
        <span>{editing ? t("notes-edit-label") : t("notes-new-label")}</span>
        <textarea
          className="answer-input note-input"
          value={draft}
          maxLength={4096}
          rows={4}
          placeholder={t("notes-placeholder")}
          onChange={(event) => setDraft(event.currentTarget.value)}
        />
        <span className={`note-count${bytes > 4096 ? " over" : ""}`}>
          {t("notes-count", { used: bytes, limit: 4096 })}
        </span>
      </label>
      {error ? <p className="inline-alert danger">{error}</p> : null}
      <div className="note-form-actions">
        {editing ? (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => {
              setEditing(null);
              setDraft("");
            }}
          >
            {t("notes-cancel")}
          </Button>
        ) : null}
        <Button
          type="button"
          variant="primary"
          size="sm"
          disabled={busy || draft.trim().length === 0 || bytes > 4096}
          onClick={() => void save()}
        >
          {t("notes-save")}
        </Button>
      </div>
    </section>
  );
}

/** A compact count used by Today's private-notes card. */
export function noteCount(records: PersonalRecords): number {
  return activeNotes(records).length;
}

export { noteAnchorMatches };
