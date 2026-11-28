# Architecture status

**2026-11-28 · Batch 4B.** This is the single technical reference for what is
implemented, how the pieces fit, and what is deliberately deferred. The
normative product rules stay in the numbered specifications; this document
describes the running system and links each module to its spec.

The code is an event-sourced, offline-first learning system. Content is
compiled ahead of time, the browser folds a private append-only log in a
Web Worker, and the server stores the same events and derived snapshots. No
AI runs at runtime.

## 1. System map

```text
content/ (Nickel + text)                    content/books/<id>/source.md
        |                                             |
        | just content                                | just books
        v                                             v
target/bundle.json + lexicon-<version>.txt   target/books/<id>.json + index.json
        |                                             |
        | refl-server import-bundle                   | committed web/src/generated/books/
        v                                             v
PostgreSQL content tables                    PWA shell / offline JS bundle
        |                                             |
        +---------------- HTTP /v1/content -----------+
                              |
                        React PWA (web/)
                              |
                     CoreWorkerClient (worker)
                              |
                     refl-core-wasm -> refl-core
                              |
        IndexedDB: snapshot, events, outbox, content, meta
                              |
                       /v1/sync (server)
                              |
                 PostgreSQL events + snapshots
```

The learner state is never authoritative on a device without the log: every
snapshot has `core_version`, `fold_version`, `content_version`, a state hash
and the last event id. Events are UUIDv7 and append-only; derived server
events are added by the server only.

## 2. Core (`core/`)

| Module | Status | What it does |
| --- | --- | --- |
| `content`, `ids`, `value`, `subject` | implemented | Content types, slug/UUID ids, finite values, CEFR scale and subject definitions |
| `answer` | implemented | Normalization, typo distance, DFA compilation and answer checking |
| `kind`, `item`, `projection` | implemented | Item kinds, validation rules, versioned model projections |
| `bundle` | implemented | Immutable compiled content, lexeme bank, item→sense index, live-KC layout |
| `event` | implemented | Schema 2 envelope, payload variants, decode/upcast, per-record limits |
| `model::state` | implemented | Canonical learner state: KC beliefs, FSRS, fluency, misconceptions, lexical cards, private records |
| `model::v1` | implemented | `knowledge-v1.4` fold: answers, passive lexical credit, lexical review, LWW records, profile |
| `model::fold_version` | implemented | Canonical hash of everything the fold reads from a bundle (format 3) |
| `placement` | implemented | Deterministic adaptive placement over live checked items |
| `plan` | implemented | Session planner and validator used by the prototype |
| `book` | implemented | Compiled book types and personalized lexical coverage |
| `quality`, `stats`, `unlock` | implemented | Quality report, period stats, unlock/milestone types |
| `book::coverage` | implemented | A token counts when the recognize card has direct evidence and `R ≥ 0.8` |

**State version.** `knowledge-v1.4+<parameter hash>`. Adding lexemes and the
item→sense index changed `fold_version`; layout and book text do not.

**Event schema.** `schema_version = 2`. Known payloads include answers,
placement, lessons, explanations, planner decisions, mentor overrides,
writing scores, milestones, achievements, unlocks, theme/companion/cosmetic
selection, showcase, and the Batch 4 records: `custom_lexeme_created`,
`custom_lexeme_updated`, `custom_lexeme_archived`, `book_mark_created`,
`book_mark_deleted`, `note_saved`, `note_deleted`, plus the direct
`lexical_review` used by the vocabulary block.

Limits enforced in `decode_event`:

| Field | Limit |
| --- | --- |
| note `body` | ≤ 4096 UTF-8 bytes, reason `note_too_large` |
| custom-lexeme payload | ≤ 4096 JSON bytes, reason `custom_lexeme_too_large` |
| book-mark token indices | `0 ≤ start ≤ end ≤ 2^53−1` |
| hashes | 64 hex characters (`book_version`, `quote_hash`) |
| record ids | UUIDv7 (`custom_id`, `note_id`, `mark_id`) |

Private records are last-write-wins by envelope `event_id`, independent of
arrival order. Deletions leave tombstones so a late creation with a smaller
id cannot resurrect a hidden record.

## 3. WASM bridge (`core-wasm/`)

`core-wasm` exports JSON-string functions over `wasm-bindgen`:

- `core_info`, `check_bundle_compatibility`, `decode_event_json`;
- stateless `fold_state`, `plan_session`, `next_item`, `replan_session`,
  `check_answer`;
- `placement_start`, `placement_next`, `placement_answer`;
- `book_coverage(book_json, snapshot_json, now_ms)`;
- `WasmSession` stateful fold/plan API used by tests and embedders.

The web worker (`web/src/worker/`) calls these through a typed RPC layer.
The browser keeps snapshots in IndexedDB; the stateful bridge is exposed but
not required for crash recovery.

## 4. Server (`server/`)

| Area | Status |
| --- | --- |
| PostgreSQL migrations 0001–0018 | implemented |
| Content import: bundle, vocabulary index, item-version append-only lock, lexicon, fold version | implemented |
| `/v1/sync` push/pull with `pg_advisory_xact_lock`, idempotent UUIDv7, cursor pagination, unknown-event handling | implemented |
| `/v1/content/active`, `/v1/content/lexicon/{version}` | implemented |
| Auth: email code, Telegram Login Widget and Mini App `initData`, sessions, approval access | implemented |
| `/v1/export`, `/v1/erase`, profile/privacy settings, admin approvals | implemented |
| Graded-book HTTP delivery | deferred; Batch 4B ships compiled book assets in the PWA shell and records the version in each mark |

## 5. Authoring tools (`tools/`)

| Tool | Status | Output |
| --- | --- | --- |
| `content-build` | implemented | Validated bundle, SCOWL lexicon, review/missing reports, live layout coordinates |
| `book-import` | implemented | EPUB/text-layer PDF/TXT/Markdown → canonical `source.md` with YAML frontmatter |
| `book-build` | implemented | Deterministic indexed book assets, license checks, `book-versions.lock`, web assets |
| `sim` | implemented | Synthetic logs, cohort/benchmark utilities, native/WASM hash parity harness |

### Two-stage book pipeline

1. `just book-import <file> --license <license>` repairs running headers,
   page numbers, end-of-line hyphenation and smart punctuation, detects or
   synthesizes `##` chapters, and writes
   `content/books/<slug>/source.md`. Scanned PDF pages are OCR'd through
   `tesseract` + `pdftoppm` when both are on `PATH`; otherwise the importer
   stops with an actionable diagnostic and never writes an empty chapter.
2. `just books` compiles every `source.md` (legacy `book.ncl` is still read),
   checks `THIRD_PARTY.md` and `content/LICENSE`, writes versioned JSON and
   `index.json` to `target/books/` and `web/src/generated/books/`, and
   verifies `content/book-versions.lock`.

The lock pins each published book's `version` and a structural
`token_index` hash (chapter/paragraph/sentence ids and token counts). A text
fix keeps the hash; a re-index that would move `book_mark` offsets fails
unless `just book-relock` is run explicitly.

## 6. Content (`content/`)

- Five live Articles KCs, 27 draft A1 KCs, 18 misconceptions.
- 13 reviewed A1–A2 starter lexemes with senses, definitions, glosses,
  examples, collocations and IPA.
- Three public-domain graded readers at A1, A2 and B1 (`book.aesop-lion-mouse`,
  `book.aesop-north-wind-sun`, `book.ohenry-gift-magi`), compiled with stable
  chapter/paragraph/sentence/token ids and attributed in `THIRD_PARTY.md`.
- Every learner-facing text remains review-gated: draft KCs do not enter a
  session, and only reviewed explanations appear in the library.

## 7. Web client (`web/`)

| Route | Screen |
| --- | --- |
| Today | first-run entry, placement, Iella tour, level, session, activity, error notebook, private notes |
| Vocabulary | search, FSRS status filters, senses, glosses, IPA, examples, focused review, personal words |
| Library | live grammar reference (short/why/deep + L1 contrast), L1 traps & refutations, cover-first bookshelf with personal EPUB/PDF/TXT/Markdown uploads and compressed covers, clean reader |
| Notes | full Markdown workspace with reader/editor, wiki links, outline, backlinks and a local graph; opened from Today, Library or an anchored drawer |
| Graph | layered mobile map and web explorer, neighbourhood focus, model numbers, notes, practice |
| Practice | lesson runner |
| Progress | isometric CEFR chart, stats |
| Profile / Settings | level, milestones, companion picker, appearance, sync, export/erase |

All screens use semantic `var(--...)` tokens from Live Studio and the other
palettes, self-hosted fonts and no external requests. Iella and Sendo share a
whole-number pixel scaling contract; Sendo supplies idle, happy, celebrate,
thinking, sleep and surprised states. Companion choice is a
`companion_selected` event plus a local preference; Sendo is unlocked by a
library milestone and previewable in development without shaming copy.

Offline path: the shell worker caches the app, fonts and books; the content
pair is verified and cached in IndexedDB; answers and private records are
folded locally and queued in the outbox; sync reconciles when the network
returns. Notes are owner-private and never enter a public profile.

## 8. Quality gates

Run inside the dev shell:

```text
just fmt-check
just lint
just test
just wasm
just wasm-hash
just content
just books
just web
just ci
```

- Rust tests cover the core, wasm bridge, server (throwaway PostgreSQL),
  content-build, book-import, book-build and the simulator.
- `just wasm-hash` folds a 20,000-event log (including Batch 4 records)
  natively and in wasm32-wasip1; state and plan hashes must match.
- Web tests cover every screen, the worker protocol, storage isolation,
  i18n key parity and offline contracts.
- `just books` fails on a stale version lock or changed token index, and the
  book-build integration test fails when `web/src/generated/books` is stale.

## 9. Deliberate gaps

- Server-side delivery of book bytes (`/v1/content/books/...`) is deferred;
  Batch 4B keeps books as versioned PWA assets.
- The lexical bank is a reviewed seed, not the 1,500–2,000-word starter
  target from docs/16.
- OCR needs `tesseract` and `pdftoppm`; PDFs without a text layer cannot be
  imported on a machine with neither.
- Sendo has the asset/registry/picker contract; story episodes and the full
  library milestone server derivation remain later work.
- Full placement stage 2 (vocabulary size estimation and pseudo-words) is
  represented by the placement prior hook, not by a complete UI flow.
