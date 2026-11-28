import { useEffect, useRef, useState } from "react";

import { Button, Modal } from "./ui";
import type { PersonalBookRecord } from "../db/storage";
import type { LearnerStorage } from "../db/storage";
import { useTranslation } from "../i18n";
import {
  BookImportError,
  MAX_BOOK_FILE_BYTES,
  parseBookFile,
  type ImportErrorCode,
} from "../prototype/bookImport";
import type { PrototypeContent } from "../prototype/content";
import { isCoverFile, prepareCover } from "../prototype/cover";

export interface BookUploadDialogProps {
  open: boolean;
  content: PrototypeContent;
  storage: LearnerStorage | null;
  onClose: () => void;
  onImported: (record: PersonalBookRecord, cover: string | null) => void;
}

const LEVELS = ["A1", "A2", "B1", "B2", "C1"] as const;

function fileName(value: string): string {
  return value.replace(/\.[^.]+$/, "").replace(/[_-]+/g, " ").trim();
}

export function BookUploadDialog({
  open,
  content,
  storage,
  onClose,
  onImported,
}: BookUploadDialogProps) {
  const { t } = useTranslation();
  const [file, setFile] = useState<File | null>(null);
  const [title, setTitle] = useState("");
  const [level, setLevel] = useState<string>("A2");
  const [author, setAuthor] = useState("");
  const [coverFile, setCoverFile] = useState<File | null>(null);
  const [coverPreview, setCoverPreview] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const coverInput = useRef<HTMLInputElement | null>(null);
  const fileInput = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    if (!open) {
      setFile(null);
      setTitle("");
      setLevel("A2");
      setAuthor("");
      setCoverFile(null);
      setCoverPreview(null);
      setBusy(false);
      setError(null);
    }
  }, [open]);

  const chooseCover = async (next: File | null) => {
    setError(null);
    setCoverFile(next);
    setCoverPreview(null);
    if (!next) {
      return;
    }
    if (!isCoverFile(next)) {
      setError(t("upload-error-cover_type"));
      setCoverFile(null);
      return;
    }
    try {
      setCoverPreview((await prepareCover(next)).dataUrl);
    } catch (reason) {
      setError(
        reason instanceof Error && reason.message === "cover_source_too_large"
          ? t("upload-error-cover_too_large")
          : t("upload-error-cover_decode"),
      );
      setCoverFile(null);
    }
  };

  const submit = async () => {
    setError(null);
    if (!file) {
      setError(t("upload-error-no_file"));
      return;
    }
    if (!storage) {
      setError(t("upload-error-storage"));
      return;
    }
    setBusy(true);
    try {
      const imported = await parseBookFile(file, content, {
        level,
        ...(title.trim() ? { title: title.trim() } : {}),
        ...(author.trim() ? { author: author.trim() } : {}),
      });
      let cover = coverPreview;
      if (!cover && imported.cover) {
        try {
          cover = (await prepareCover(imported.cover)).dataUrl;
        } catch {
          cover = null;
        }
      }
      const existing = await storage.listPersonalBooks();
      if (
        existing.length >= 24 &&
        !existing.some((entry) => entry.book.id === imported.book.id)
      ) {
        setError(t("upload-error-library_full"));
        return;
      }
      const record: PersonalBookRecord = {
        learner_id: storage.learnerId,
        book: imported.book,
        imported_at: Date.now(),
        bytes: new TextEncoder().encode(JSON.stringify(imported.book)).length,
      };
      const estimate = await navigator.storage?.estimate?.();
      const nextBytes =
        record.bytes + (cover ? Math.ceil(cover.length * 0.75) : 0);
      if (
        estimate?.usage !== undefined &&
        estimate.usage + nextBytes > 45 * 1024 * 1024
      ) {
        setError(t("upload-error-library_full"));
        return;
      }
      await storage.putPersonalBook(record);
      let storedCover = cover;
      if (cover) {
        try {
          await storage.putBookCover({
            learner_id: storage.learnerId,
            book_id: imported.book.id,
            data_url: cover,
            updated_at: Date.now(),
          });
        } catch {
          storedCover = null;
        }
      }
      onImported(record, storedCover);
      onClose();
    } catch (reason) {
      if (reason instanceof BookImportError) {
        setError(t(`upload-error-${reason.code as ImportErrorCode}`));
      } else {
        setError(t("upload-error-generic"));
      }
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal open={open} title={t("upload-title")} onClose={onClose} className="book-upload-modal">
      <div className="book-upload-body">
        <div className="book-upload-preview" aria-hidden="true">
          {coverPreview ? (
            <img src={coverPreview} alt="" />
          ) : (
            <div className="book-cover-fallback book-upload-placeholder">
              <span>{t("book-cover-library")}</span>
              <strong>{title.trim() || t("upload-preview-title")}</strong>
              <span>{level}</span>
            </div>
          )}
        </div>
        <div className="book-upload-fields">
          <p className="bento-note">{t("upload-note")}</p>
          <label className="field">
            <span>{t("upload-file")}</span>
            <input
              ref={fileInput}
              className="answer-input"
              type="file"
              accept=".epub,.txt,.md,.pdf,application/epub+zip,text/plain,text/markdown,application/pdf"
              onChange={(event) => {
                const next = event.currentTarget.files?.[0] ?? null;
                setFile(next);
                setError(null);
                if (next && !title.trim()) {
                  setTitle(fileName(next.name));
                }
              }}
            />
          </label>
          {file ? (
            <p className="book-upload-file">
              {file.name} · {Math.ceil(file.size / 1024)} KB
            </p>
          ) : null}
          <div className="book-upload-row">
            <label className="field">
              <span>{t("upload-book-title")}</span>
              <input
                className="answer-input"
                value={title}
                maxLength={120}
                onChange={(event) => setTitle(event.currentTarget.value)}
              />
            </label>
            <label className="field">
              <span>{t("upload-level")}</span>
              <select
                className="answer-input"
                value={level}
                onChange={(event) => setLevel(event.currentTarget.value)}
              >
                {LEVELS.map((entry) => (
                  <option key={entry} value={entry}>
                    {entry}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <label className="field">
            <span>{t("upload-author")}</span>
            <input
              className="answer-input"
              value={author}
              maxLength={120}
              onChange={(event) => setAuthor(event.currentTarget.value)}
            />
          </label>
          <div className="book-upload-cover-row">
            <input
              ref={coverInput}
              className="vocab-file-input"
              type="file"
              accept="image/jpeg,image/png,image/webp"
              onChange={(event) => void chooseCover(event.currentTarget.files?.[0] ?? null)}
            />
            <Button
              type="button"
              variant="secondary"
              size="sm"
              onClick={() => coverInput.current?.click()}
            >
              {coverFile ? t("upload-change-cover") : t("upload-choose-cover")}
            </Button>
            {coverFile ? (
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => void chooseCover(null)}
              >
                {t("upload-remove-cover")}
              </Button>
            ) : null}
          </div>
          <p className="bento-note">{t("upload-cover-note")}</p>
          <p className="bento-note">
            {t("upload-limit", {
              size: Math.round(MAX_BOOK_FILE_BYTES / (1024 * 1024)),
            })}
          </p>
          {error ? (
            <p className="inline-alert danger" role="alert">
              {error}
            </p>
          ) : null}
          <div className="book-upload-actions">
            <Button type="button" variant="ghost" onClick={onClose}>
              {t("notes-cancel")}
            </Button>
            <Button
              type="button"
              variant="primary"
              busy={busy}
              disabled={!file || !storage}
              onClick={() => void submit()}
            >
              {t("upload-action")}
            </Button>
          </div>
        </div>
      </div>
    </Modal>
  );
}
