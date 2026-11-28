# Engineering decisions

## W30 — Batch 4B: book importer, library UI, hybrid graph, Sendo and the consolidated map

- **Two-stage books.** `book-import` is a separate Stage 1 tool: it repairs structure and writes canonical `source.md`; `book-build` remains the deterministic compiler. Keeping import and publication apart means a bad OCR pass cannot silently publish, and a text fix does not require re-importing. `content/book-versions.lock` pins `version` plus a structural `token_index` hash; only `just book-relock` accepts a change that moves `book_mark` token offsets.
- **Books as PWA assets in Batch 4B.** Compiled books are committed under `web/src/generated/books` and bundled with the shell so the reader works on the first offline launch; the server book endpoints from docs/16 are deferred. The asset still carries `version`, and every mark stores it, so moving to HTTP delivery later does not change the event schema.
- **Stateful versus snapshot folding.** `WasmSession` remains the stateful bridge, while the web worker uses stateless JSON calls plus an IndexedDB snapshot. This is the crash-safe choice: a reload can re-open the last snapshot and re-deliver the outbox without trusting worker memory. The two paths share the same core fold and are checked by the bridge tests.
- **Lexical review is an event.** A focused vocabulary card writes `lexical_review` (sense or custom id, direction, correct) instead of inventing a synthetic item. It is a new schema-2 payload variant, keeps the KC graph untouched, and reuses `KnowledgeModel::review_lexeme`; passive credit still comes only from real grammar answers.
- **Private records are LWW by UUIDv7.** Notes, personal words and book marks are not answers and do not merge: the greatest envelope `event_id` wins whole. Tombstones are stored so arrival order cannot resurrect a deleted record. Size and integer limits are checked at decode, with stable `note_too_large` and `custom_lexeme_too_large` reasons.
- **Hybrid graph.** The layered view reads precomputed `layout_x`/`layout_y`; the desktop web view runs a deterministic 300-step force layout seeded by `content_version`. Both use the same live-KC nodes, edge types (`requires`/`supports`/`contrasts`) and status model, so selecting a topic cannot show different facts on phone and desktop.
- **Companion registry.** Iella and Sendo are data entries, not screen branches. Sendo is its own 16×16 pixel sprite with six emotional states, selected through `companion_selected` plus a local fast-start preference; the library milestone gates selection, and dev/`VITE_SENDO_UNLOCKED` enables preview without pressure copy.
- **Architecture reference.** `docs/ARCHITECTURE_STATUS.md` consolidates the running system, event/version rules, module status and deferred gaps. The numbered docs stay normative; this is the implementation map.
- **Verification.** `just ci` and `just books` pass; web tests are **45 files / 176 tests**. Native and WASM state/plan hashes match at `ba7ec46fcd9b…` / `ef8d986ebce6…` on the 20,000-event log. The commits are local and not pushed.

## W29 — Batch 4A: lexical bank, private records, graph layout and the book compiler

- **Lexical content.** `Lexeme` (lemma, POS, frequency rank 1…10 000, CEFR level, forms, source/license, reviewed uz/ru/en text) and `LexemeSense` (definition ≤ 25 words, glosses, 1–3 examples, 2–5 collocations, UK/US IPA, optional audio key) are first-class content types in `core::content` and the Nickel schema under `content/lib/schema.ncl`. The bundle carries `lexemes` plus an `item_lexemes` index from an item's stimulus and reference answers to **sense ids**; the index, not the item version lock, is hashed into `fold_version` (format 3) so old item projections stay append-only. `content/lexemes/a1-starter.ncl` seeds 13 reviewed A1–A2 words across the live Articles KCs.
- **Cards.** `LearnerState.lexical_cards` maps a lexeme (bundle sense or personal UUID) to independent recognize/produce FSRS-6 cards with `exposures`, `source`, `review_count` and `lapse_count`; nested maps keep JSON keys strings. Direct evidence starts a card through `review_lexeme`. Passive credit (correct or typo, first attempt, no hints, not rapid) adds 0.15 evidence per meeting; after three different items across two families a recognize card is born with one-day stability, but passive meetings never move the FSRS interval. Placement Stage 2 has a frequency-band prior hook (`placement_lexeme_prior`): a known claim seeds both cards with `stability = 1 + band`, an unknown claim creates nothing, and direct evidence is never overwritten.
- **Private records.** `custom_lexeme_created/updated/archived`, `note_saved/deleted` and `book_mark_created/deleted` keep schema version 2 as new payload variants. Each record is last-write-wins by envelope `event_id`; the fold stores tombstones when a delete beats a creation, so shuffled and incremental folds agree. Limits are enforced in `decode_event`: note bodies ≤ 4096 UTF-8 bytes, custom-lexeme payloads ≤ 4096 JSON bytes, token indices ≤ 2^53−1 and ordered, SHA-256 fields 64 hex, and record ids UUIDv7. The sync API names `note_too_large` / `custom_lexeme_too_large`; TS bindings use `number`, never `bigint`. Export and erase include these events, and the integration tests check both.
- **Model/state version.** New state sections mean `knowledge-v1.4+<hash>` and a refold of old snapshots. `fold_version` includes the lexeme bank and item→sense index but not `layout_x/layout_y` or books, because the map and reading do not change the model.
- **Graph layout.** content-build computes `layout_x/layout_y` (0…1000) for live KCs only: layer is the longest `requires` path from a root, order within a layer is the mean parent position then id, and coordinates are normalized per layer. Draft and observed-only KCs stay at 0,0 and never enter the map.
- **Books.** `tools/book-build` exports each `content/books/<id>/book.ncl` with Nickel, checks that the license is original/public-domain/CC0/CC-BY and recorded in both `THIRD_PARTY.md` and `content/LICENSE`, rejects copyleft, and compiles deterministic chapter/paragraph/sentence/token ids and a word→sense/kc index using the content-build tokenizer. The asset `version` is SHA-256 of the book JSON with an empty version field; `index.json` records bytes and digest. `just books` builds `target/books` after `just content`. The starter shelf contains public-domain A1/A2 retellings of Aesop and O. Henry’s B1 “The Gift of the Magi”; personal uploads stay device-local in IndexedDB with 320×480 compressed covers.
- **Coverage.** `core::book::coverage` counts only tokens with a recognize card that has a direct review and `R ≥ 0.8`; tokens without a lexeme are outside the denominator. `book_coverage(book_json, snapshot_json, now_ms)` exposes it through the WASM bridge.
- **Verification.** `just fmt-check`, `just lint`, `just test`, `just wasm`, `just wasm-hash`, `just content`, `just books` and `just web` pass; web tests are **38 files / 157 tests**. Native and WASM state/plan hashes match at `bac668ad9fc7…` / `18b3f97ef2f9…` on a 20,000-event log that includes the new record events. The commits are local and not pushed.

## W28 — seamless Iella spotlight, five-stop tour, and an assessed placement ceiling

- The tour no longer paints four fixed shade rectangles and a box-shadow highlight. It renders one fixed SVG mask with a rounded hole and a matching SVG stroke, so the cutout and border use exactly the same coordinates and cannot leave horizontal seams or a vertical mismatch. Backdrop blur was removed from the tour.
- Target bounds are tracked with `ResizeObserver` on the target, `resize`/`scroll` listeners, `transitionend`/`animationend`, and a short `requestAnimationFrame` settle loop after scrolling and entrance motion. The card size is measured the same way, so feedback and copy changes reposition the card cleanly. `prefers-reduced-motion` still switches scrolling to `auto` and disables the card/mask entrance animation.
- The first-run tour now has five stops: arrival, an interactive `a`/`an` try-step with feedback, today's route, level badge, and the start button. The try step is deliberately not logged as a learning answer.
- Added the one-time contextual `IellaTip` primitive and all four localized messages for Vocabulary, Library, Graph, and Notes. It uses `refl.tip.<section>` storage, marks itself seen on first mount, and is mounted on the Today Notes surface; the Vocabulary/Library/Graph screens are Batch 4 and will mount the same component with their section key.
- Placement now uses `KnowledgeModel::level_assessed` for its result: live KCs with at least one direct observation, not untouched priors, and never draft KCs. A clean run of at least four answers reports the highest live level in the domain as `point`/`high` with the lower band one level below; the UI adds “A2 is the top of the current content” when it reaches that ceiling. The full `KnowledgeModel::level` remains the statistics/planner view over all live KCs.
- Added `tools/sim/tests/placement_live.rs`: the real bundle's adaptive and forced 10/10 runs now assert `point = 2.0`, band `A1–A2`, and `start_level = 2.0`; a separate test proves draft KCs do not change the level.
- Verification: `just fmt-check`, `just lint`, `just test`, `just wasm-hash`, and `just web` pass; web tests are **38 files / 157 tests**. Native and WASM hashes remain `ff400f505425…` / `e078735dcd0e…`. The commit is local and not pushed.

## W27 — tight brand lockup, compact palette menu, and working Telegram auth/notifications

- The brand lockup gap is 7 px in both the auth header and the shell. The `refl` wordmark is Geologica 600 with `-0.02em` tracking and grayscale/antialiased font smoothing; the previous 800 variable-axis weight is gone and Pixelify Sans is no longer used for the wordmark.
- `AuthScreen` now places the brand alone on the left and groups the language and theme controls in `.auth-header-controls` on the right (`margin-left: auto`). `AppShell` follows the same brand-left / actions-right rule on mobile. The old five-swatch bar stays gone.
- Added `ThemeMenu`: a compact 44 px trigger with the current palette swatch and an out-of-flow, right-aligned popover of five swatches. It shows no internal slug text (labels are localized accessible names/titles only), closes on outside press, Escape, or focus leaving the menu, and cannot overlap the language control. `SettingsScreen` restores the full labelled five-palette picker with the original notes. The language popover remains absolute and outside the layout.
- Telegram Mini App auth is now verified correctly: the client sends the raw `initData`, and the server validates the HMAC with the `WebAppData` secret (not the Login Widget's `SHA-256(token)` secret), excludes both `hash` and `signature` from the data-check string, uses the verified user identity instead of client fields, and allows a 24-hour `auth_date` window. Login Widget returns keep the 5-minute window.
- Browser Telegram login supports the official widget via `VITE_TELEGRAM_BOT_USERNAME`; if the widget cannot load, the custom OAuth button remains as a fallback. The bot id can be derived from `TELEGRAM_BOT_TOKEN` by `just dev`. OAuth `error`/`error_description` returns now produce an actionable message that names the current origin and points to BotFather; a server 503 says Telegram is not configured rather than silently doing nothing.
- `just dev` derives `VITE_TELEGRAM_BOT_ID` from the token prefix, copies `TELEGRAM_BOT_USERNAME` to `VITE_TELEGRAM_BOT_USERNAME`, falls back from `ADMIN_TELEGRAM_ID` to `ADMIN_TELEGRAM_CHAT_ID`, and warns when Telegram is disabled or the approval chat is missing. A tracked `.env.example` and README note document the required keys and the BotFather domain requirement for tunnels such as `trycloudflare.com`.
- Notification delivery is observable: startup logs whether the bot worker is enabled, `notify_pending`/`notify_user` log send failures, and the long-polling worker logs `getUpdates` and approval-callback failures. The bot worker now processes updates through `&dyn TelegramGateway`, so the integration test drives the same callback path with `MockTelegramGateway`.
- Tests cover brand gap/weight, the five-swatch compact menu and full Settings picker, OAuth error parsing and the actionable UI alert, Mini App initData valid/tampered/stale cases, missing-token 503, the Login Widget HMAC/freshness, approval-request text/buttons, and the bot approve callback (status change, callback answer, keyboard clear, user notification). `just fmt-check`, `just lint`, `just test`, and `just web` pass; web tests are **37 files / 153 tests**. The commit is local and not pushed.

## W26 — cold-start content pair, one Light/Dark header control, and the Batch 4 spec

- `just dev` now builds `target/bundle.json` and imports it with the sibling SCOWL lexicon **before** starting `refl-server`, so a fresh database is not empty on the first visit. The new idempotent `just seed-content` recipe does the same and uses `REFL_DATABASE_URL` (default `postgres://refl@127.0.0.1:5433/refl`); it deliberately ignores a stale global `DATABASE_URL`, which in the dev shell points at a different cluster.
- `/v1/content/active` remains the source of the exact `bundle_json` string covered by `digest`; the client hashes that string and then fetches `/v1/content/lexicon/{version}` using the bundle's own lexicon reference. The browser pair is activated only after both hashes and sizes verify. The server integration test now asserts a cold database returns 404 before import and, after import, that the served bundle names the lexicon the endpoint returns; the web loader test covers a cold start, a cached second start, and an atomic failure when the dictionary cannot be downloaded.
- The prototype header no longer exposes five palettes or four skins. `AuthScreen` and `AppShell` show one 44 px Light/Dark toggle (`ThemeToggle`); Settings offers the same two schemes as cards. `AppearanceMenu`/`PaletteToggle` are removed from the header, and `burgundy`, `sand`, `midnight-cobalt`, and skin names no longer appear in the entry UI. The internal palette/skin registry remains for old stored selections and APCA coverage, but it is not a user-facing choice.
- The brand wordmark is Geologica (`--font-family`, weight 800); Pixelify Sans is reserved for pixel accents. The language switcher is now a click/focus popover positioned out of flow (`position: absolute; right: 0`), closes on outside press or `Escape`, and cannot widen or overlap the adjacent theme control. The previous hover/focus width-expansion rules are gone.
- Added `docs/16-vocabulary-library-notes-graph.md`: the Batch 4 vocabulary layer, personal words, book compiler and reader, knowledge graph, offline notes/bookmarks, Sendo asset contract, exact new event types, LWW rules, storage budget and phased delivery plan. `docs/README.md` and the root `README.md` list it.
- Verification: `just ci` passes (fmt-check, lint, tests, WASM, native = WASM hashes, content, web). Content is 0 errors / 5 live + 27 draft KCs. Web tests: **36 files / 148 tests**. WASM: **1,105,089 bytes raw / 426,419 bytes gzip** (budget 500 KB gzip). Production JS: **143,209 bytes gzip** for the main chunk plus **3,144 bytes gzip** for the worker (budget 300 KB gzip for JS without WASM). Native and WASM agree on fold hash `ff400f505425…` and plan hash `e078735dcd0e…`. `design/reference/design/` is untouched. The commit is local and not pushed.

## W25 — return-check/audit probes, quality windows, and simulator v2

- `State` now records the first mastery moment per KC (`mastered_at`, serde-defaulted for old snapshots). The planner uses it for the 7/30/90-day audit milestones; a state from before the field appears has no audit dates until it is refolded.
- The planner has two measurement phases. A session that starts after a break gets `return_check` steps first: mastered KCs ranked by importance × (1 − R) × σ, production items only, a family unseen for 30 days, at most 15 tasks or 10 estimated minutes. The session flag is fixed at build time so the first answer moving `last_answer` cannot cancel the check mid-session. Audit probes are added only near a mastery milestone, at most two per session and one per 20 ordinary tasks; candidate order is seeded from the recorded plan seed and each next-item choice records its own seed in `plan_issued`. Probe families are excluded across probes in one plan.
- `Plan.history` now carries the log-derived family recency/deferral table, so `adapt` can rebuild probes without a new argument. The core WASM bridge reads `plan.history` for `next_item` and returns the step's `AnswerContext`; `LessonRunner` writes that context (`lesson`, `practice`, `review`, `audit`, `return_check`) instead of collapsing probes into practice.
- Added `core/src/quality.rs`: Brier, log-loss, AUC, mean predicted probability and calibration deciles overall, by `AnswerContext`, and by `LadderLevel`; `timing-inconsistent` count/rate; validator event/code counts; and a gated family audit (≥30 precise answers with σ < 0.5) with unrecognized rate, dead/magnetic distractor options (≥50 option shows), empirical-vs-author difficulty delta, point-biserial discrimination and slow-correct median/ratio.
- Added `core/src/stats.rs`: `PeriodStats` folds arbitrary `[from, to)` windows — answers/correct, study seconds, active days and per-day activity, context counts, mastered start/end, mean recall over mastered KCs, active/cured misconceptions, timing-inconsistent count/rate and validator codes. `QualityReport` and `PeriodStats` are exported to TS bindings; the TS integer fields are typed as `number`, not `bigint`.
- Simulator remediation is now observable: after S3 shows a micro-explanation for a diagnosed wrong answer, the simulator folds a logged follow-up answer with the true post-remediation practice step. Earlier W24 runs deliberately left that gain off-log; the W24 files and `bench-v1` remain untouched as historical outputs.
- Simulator v2 is `just v2` / `tools/sim/src/bin/v2.rs`: 1,000 learners, six archetypes, live + full 32-KC bundles, both the 200-step stress horizon and a 600-step (~30-day) horizon. Every report names the git revision and carries SplitMix64 learner-cluster bootstrap intervals (B = 2000, 80%/95%); fewer than six learners omit intervals. `tools/sim/bench/v2/report-seed1.txt` is the versioned output; v1 and `bench-v1` are untouched.
- a-an-sound audit:
  - Bank and graph are unchanged between W24 and v2: **21 item slots in 8 families**, with `requires a-an-sound → first-mention` (a-an-sound is downstream of first-mention) and no other incident edge.
  - W24, 200 steps: live S3 reach **61%** (median 115 steps); full 32-KC S3 reach **3%** (median 195), the absolute bottleneck called out in `hetero-seed1.txt`.
  - v2, 200 steps: live **52%** (median 98), full **12%** (median 180). v2, 600 steps: live **72%** (median 120), full **62%** (median 267). The 600-step horizon substantially fixes the full-bundle reach; the remaining delay is the first-mention prerequisite and competition with 31 other KCs, not a missing slot bank.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W24 — 100k stress benchmarks, cross-target determinism, and a 1,000-learner heterogeneous cohort

- `tools/sim` now has two W24 runners: `just stress` (release, `--test-threads=1`, ignored heavy tests) and `just hetero` (`just content`, then the `hetero` binary). Raw outputs are `tools/sim/bench/w24/{stress.txt,stress-wasm.txt,hetero-seed1.txt}`. The stress log uses realistic answer latency, above the model's rapid-share and typed thresholds; before this fix every generated answer was a rapid tap, so the fold skipped all knowledge, memory and misconception updates and the numbers were not representative.
- Native release, 100,000 sequential answers on the synthetic 32-KC bundle (33 bundle entries: 32 learning KCs + the observed-only spelling KC; 384 item slots):
  - fold: **316.009 ms**, **316,447 events/s**, p50 **2 µs**, p95 **5 µs**, p99 **7 µs**, max **81 µs**; snapshot **283 → 35,843 bytes** (+35,560); peak RSS **38,812 KiB**.
  - replay: 100 **0.363 ms**, 1,000 **3.242 ms**, 10,000 **31.296 ms**, 50,000 **154.646 ms**, 100,000 **306.188 ms**; the first measured median-of-three replay above 16 ms is **n = 5,410 (16.081 ms)** and above 50 ms is **n = 16,277 (50.269 ms)**. Repeat runs vary a few hundred events around these thresholds because they are wall-clock measurements (observed 5,410–5,462 and 16,277–16,495).
  - planner `next_item` 100k: **5,116.033 ms**, **19,546 ops/s**, p50 **50 µs**, p95 **52 µs**, p99 **55 µs**, max **1,702 µs**.
  - placement `next` 100k: **4,256.696 ms**, **23,492 ops/s**, p50 **42 µs**, p95 **44 µs**, p99 **45 µs**, max **75 µs**.
- JSON bridge vs native, 10,000 valid sequential events: native **31.725 ms** (**315,204 events/s**, p50 2 µs, p95 6, p99 8, max 26); `core-wasm::fold_state` **25,620.584 ms** (**390 events/s**, p50 **2,542 µs**, p95 2,576, p99 2,608, max 3,958). The bridge result is the same full `State` as native (canonical bytes equal), not only the same hash. This is the actual Rust JSON boundary (`parse bundle + parse snapshot + fold + serialize`) and deliberately reparses the bundle and snapshot on every call; it is the dominant browser-worker cost and does **not** include the browser's JS `postMessage`/glue scheduling. Do not read 390/s as a fold limit; it is a bridge-protocol limit.
- Cross-target: native and `wasm32-wasip1` (wasmtime) produce the same `state_hash`, which is SHA-256 of `State::canonical_bytes`, and the same snapshot bytes. The full replay/selection mode log is `tools/sim/bench/w24/stress-wasm-modes.txt`.
  - fold 100k: native **310.091 ms / 322,486 events/s** (p50 2, p95 5, p99 7, max 60 µs); WASM **833.936 ms / 119,913 events/s** (p50 7, p95 12, p99 14, max 33 µs); both hash `ea95cc6b…` and 283 → 35,843 bytes.
  - replay 100k: native **301.743 ms**, WASM **833.568 ms**; hashes match at 100/1k/10k/100k. Replay thresholds: native 5,437 (16.633 ms) and 16,354 (50.165 ms); WASM 1,994 (16.009 ms) and 6,251 (50.080 ms).
  - selection 10k: native planner **20,601 ops/s**, placement **24,403 ops/s**; WASM **12,509** and **12,208 ops/s** (p50 79/81 µs vs 48/40 µs native).
  - `determinism_10k_shuffled_and_full_state` proves canonical-byte equality for incremental fold, ordered replay, and shuffled replay of 10,000 events (`state_hash cdb282b9…`, snapshot 35,676 bytes); `wasm_json_boundary_10k` proves bridge `State` canonical-byte equality with native. `just stress-wasm` compares native/WASM fold hashes and snapshot bytes.
- Heterogeneous cohort (`just hetero`): 1,000 learners × 200 steps, seed 1, six archetypes stratified across `uz`/`ru`, on the live Articles bundle (5 learning KCs) and on `target/bundle.json` promoted for the fixture (32 learning KCs). The simulated learner has hidden per-KC skill, true prerequisites, L1 misconception strengths, forgetting, slip, rapid guesses and its own clock; true mastery is the **latent slip-free success probability** ≥ 0.85, so a high-slip learner can be truly mastered while still making mistakes. Strategy 1 is linear/Leitner, Strategy 2 is KC-uncertainty only, Strategy 3 adds graph routing, ΔV/t, FSRS due reviews, L1 priors/strength and targeted micro-explanation remediation after a wrong diagnosed item. The remediation is a simulator assumption for S3 only; it is not a core event, so the core model does not see that the learner's misconception was corrected and its level estimate can lag (this is visible below).
- Overall results (reach = learner-KC targets reached at any point in 200 steps; live/full target denominators 4,164/21,537 because Fast+High-Slip learners have no learning targets at T0):

  | strategy | live reach | live median steps (IQR) | live level MAE / RMSE | full reach | full median (IQR) | full level MAE / RMSE |
  |---|---|---:|---:|---:|---:|---:|---:|
  | S1 Leitner | 27.1% | 106 (53–123) | 0.343 / 0.512 | 15.5% | 38 (17–77) | 0.426 / 0.697 |
  | S2 KC-adaptive | 24.0% | 98 (66–125) | 0.361 / 0.556 | 16.2% | 56 (19–129) | 0.475 / 0.749 |
  | S3 full refl | **65.4%** | **72 (51–111)** | 0.362 / 0.487 | **31.1%** | **52 (24–120)** | 0.683 / 0.908 |

- Per-KC median [IQR] steps and reach percentages are in `tools/sim/bench/w24/hetero-seed1.txt`; the live/full tables above and the audit use the same seed and target set.
- Misconception reduction (`@30/@60/@100/@200`): live S1 21.8/23.7/28.6/31.6%, S2 26.2/25.3/27.9/32.1%, S3 **77.0/91.8/99.9/100.0%**; full S1 5.3/10.8/15.9/24.6%, S2 8.7/14.4/21.0/26.3%, S3 **49.8/65.6/78.8/88.9%**.
- Per-archetype S3 reach vs S1/S2 (live / full): True zero 9.0/0.0% vs 0/0 and 0/0; Spiky 99.9/42.0% vs 27.2/28.1 and 21.9/27.0; L1-fossilized 18.9/7.0% vs 0/3.9 and 0/4.4; Fast+High-Slip no targets (level MAE 0.433 live / 1.394 full vs S1 0.876/1.591 and S2 0.769/1.697); Impulsive guesser 99.8/70.0% vs 54.6/27.0 and 48.9/27.4; Steady 100/60.3% vs 53.9/30.3 and 49.8/33.3.
- Honest failure / bottleneck audit (seed 1):
  - Live: no KC is below both baselines. S3 absolute reach is low only for True Zero (9.0%) and Fast+High-Slip (no targets by definition); live level MAE is essentially tied with S1 and S2.
  - Full: S3 is marginally below the best baseline on `noun.plural-irregular` (32% vs 33%), `possessive.s` (32% vs 33%) and `present-continuous.questions` (38% vs 43%) — all two-slot KCs where a small sample flips the rate and S3 spends steps on graph prerequisites/misconception work. `article.a-an-sound` is an absolute bottleneck at 3% (baselines 3/0%) despite a 21-slot bank; the article region is also low at 30%/23%/33% for first/second/zero, while `noun.countability` reaches 55%.
  - Full True Zero reaches almost nothing (1/5,344) and L1-Fossilized 7.0%; the 32-KC graph plus 200 steps is not enough for a learner with a low prior and a large misconception set, even with remediation. Fast+High-Slip has no learning targets at T0 but its true level is high; all models under-estimate it (S3 level MAE 1.394, S1 1.591, S2 1.697), a real false-demotion/level-observability limitation.
  - Placement accuracy is computed on a fresh learner per strategy, so it is identical across S1/S2/S3: live MAE 0.353/1.054/1.014 and full 0.471/0.811/0.810 at 3/6/10 items. It is **not monotone**: more items on the live Articles pool make the estimate worse, and the 6/10-item error is dominated by high-latent-skill learners whose observed slips pull the model down. This is an honest placement/level limitation, not a strategy difference.
  - S3's level MAE is worse than both baselines on the full projection (0.683 vs 0.426/0.475) even though its true mastery is much higher. The core sees many wrong answers on misconception-heavy items and has no event for the micro-explanation remediation, so its belief lags the true post-remediation skill. A future remediation event or explicit explanation evidence would be needed; do not claim S3 level calibration is better.
- `core/src/placement.rs` now computes the expensive level estimate only when the beginner exit can apply (`asked >= 3`); the first two selection calls skip it. This keeps short `placement_next` calls cheap and does not change any stop rule. The 100k placement selection benchmark uses the short-progress path.
- `just` recipes: `stress` (ignored release suite with single-threaded peak-RSS measurement), `stress-wasm` (native vs wasmtime fold hash/snapshot check), `hetero` (builds content and writes the 1,000 × 200 report). Heavy tests remain `#[ignore]`; the default smoke tests assert deterministic replays, bridge equality, and that S3 beats S1/S2 on the live smoke.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W23 — CEFR A1 graph, honest Pre-A1 entry, and adaptive placement

- Added 27 CEFR A1 grammar KCs under `content/a1-core/` (subject pronouns, object pronouns, possessive adjectives, possessive 's, regular and irregular plurals, demonstratives, be affirmative/negative/questions, there is/are, present simple affirmative/third-person/negative/questions, present continuous affirmative/questions, can ability/requests, imperatives, question words, adjective position, adverbs of frequency, prepositions of place/time, some/any, much/many). The five Articles KCs stay live; every new KC is `A1`, draft, and has one choice and one gap-fill starter item plus short explanations in uz/ru/en.
- The graph has 18 misconceptions: 15 `l1-transfer` + 2 `overgeneralization` + 1 `spelling-based`. The existing ten are kept; `the-for-new`, `the-for-general`, and `a-for-known` are re-tagged from overgeneralization to `l1-transfer` with `l1 = { uz = true, ru = true }` (titles, priors, affects and remedies unchanged, so no review hash changes), and eight new L1 entries were added (`be.drop-copula`, `be.am-agree`, `pronoun.drop-subject`, `present.third-person-s`, `present.no-do`, `question.word-order`, `preposition.time`, `quantifier.much-many`). Every new misconception is diagnosed by at least one starter item.
- New `placement_completed` event (schema 2, no upcaster): `{ start_level: BTreeMap<SubjectId, Finite>, answered: u16, source: StartFromZero | Diagnostic }`. The fold merges each level into `Profile.start_level` (clamped to the subject scale) and returns `Skipped`; the profile is already part of the state hash, so replay and incremental folding agree and no profile argument was added to `fold_state`.
- Added `core/src/placement.rs`: up to 10 live items, one per family, chosen by the sum of importance-weighted one-observation variance reductions divided by expected time (`ΔV = v − 1/(1/v + I)`), skipping recently seen items and kinds that are not automatically checked. Stop order: beginner exit (3 items, ≤ 1 correct, `KnowledgeModel::level` point < 0.6, source `StartFromZero`), 10-item cap, then every live teachable KC with σ < 0.8. No RNG; the same state and progress pick the same item, so a reload resumes. The result is the existing `KnowledgeModel::level` point and 80% interval; it is clamped to the subject scale.
- Added `placement_start`, `placement_next`, and `placement_answer` to the WASM bridge, the worker protocol and the typed client. Placement answers are ordinary `answer` events with `context: "placement"`; they fold as real evidence (including FSRS memory start outside a lesson), and rapid guesses are discarded by the existing filter. No model special-casing.
- Web entry: a new learner sees a three-state gate (`pending` until content and the first sync settle, then `needed` or `done`). `done` if a learner-scoped `refl.entry-choice` flag exists, `profile.start_level.en` is present (zero counts), or legacy evidence exists. “Start from the beginning” records a `start_from_zero` event; “Check my level” runs the placement runner and records a `diagnostic` event on finish. The tour mounts on Today after the gate closes; existing learners are never interrupted.
- Level display: `progress.ts` parses `profile.start_level.en` and exposes `levelView`. With no evidence and no placement, Today and Profile show `Pre-A1 → A1` and the existing `today-level-empty` copy; a fractional placement position uses `level-start-note`. The isometric CEFR chart is unchanged (it is curriculum coverage, not the current position).
- Activity: placement answers are excluded from the heatmap (and from the compacted `last_answer` fallback on days that already have placement events), so a diagnostic does not inflate learning activity; `noteActiveDay` is not called by placement.
- Tests: content-build pins the 33 bundle KC entries (32 learning + spelling), the 27 new KC ids, 18 misconceptions and 15 L1-transfer origins; core round-trip covers the new payload; `core-wasm` covers start/answer/done and the profile fold against replay; server sync pushes and pulls the event; web covers `levelView`, Pre-A1 empty states, entry decisions/events, the placement runner and screen, and activity exclusion. All 127 web tests pass.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## `604e0c9b` — availability-first checks and review prefix boundaries

- Availability is checked before time attribution for prerequisite, block, review prefix, contrast, and interleaved learning.
- An unavailable candidate cannot set `short_of_time`, `main_short`, or consume budget.
- Lesson `PushResult` is checked; `main_left` is debited only on `Added`.
- Review-prefix pruning considers only available Reviews, so an unavailable Review does not consume the cumulative time prefix.
- Regression tests cover unavailable Block over budget and Review-prefix availability/boundary cases.
- Public `End`, `Plan`, and `PlanIssuedEvent` serialization are unchanged.

## `14fb5f6c` — reset `run_limited` after an accepted step

- `Builder::push` clears `run_limited` only when it returns `PushResult::Added`.
- `finish` keeps its own run-reconstruction evidence unchanged.
- A RunLimit rejection followed by an accepted step that is later starved no longer terminates with `End::RunLimit`.
- The regression test `run_limit_is_cleared_by_a_later_accepted_step_removed_by_starvation` locks this in.
- No public serialization or `End` variant changes.

## `6a7ffde3` — learner-cluster bootstrap and p1c scale conversion

- `learner_cluster_bootstrap` resamples learners with replacement for `B=2000` fixed-seed replicates.
- Each replicate reports `sum(false_mastered) / sum(mastered)`; the result is the 95% percentile interval.
- A unit test checks determinism and `0 ≤ low ≤ high ≤ 1`.
- `p1c` maps model `μ`/`bound` to the simulator scale with `a_eff`, `SLOPE=1.3`, and the mastering item difficulty.
- `skill_later < 1.5` is replaced by the per-pair simulator-scale `mastery_sim` threshold.
- The run header records the short git commit SHA, falling back to `CARGO_PKG_VERSION`.

## C4 — pinned p1c v3 baseline at `6a7ffde3`

- Command: `cargo run --release --locked --quiet -p refl-sim --bin p1c -- content/bench/v3/bundle.json 200 30 1`; seed `2` likewise.
- `p1c` takes positional `[BUNDLE] [LEARNERS] [DAYS] [SEED]`; it has no `--bundle` or `--seed` flags.
- Baseline is pinned at `6a7ffde` because it combines availability-first, `run_limited` reset, and the bootstrap/scale correction.
- Bundle v3 is the frozen benchmark content; existing `tools/sim/bench/` outputs were not overwritten.
- Seed 1: `104/895` false mastery (`11.6%`), learner-cluster 95% CI `9.2%–14.3%`.
- Seed 2: `94/906` false mastery (`10.4%`), learner-cluster 95% CI `8.0%–12.8%`.
- Header model `knowledge-v1.3+68ea62decb36521e`; planner `session-prototype-4+c896293765c21793`.

## P1a — server HTTP layer

- Added an Axum router with `GET /health`, `GET /v1/content/active`, and `POST /v1/sync`.
- Push sorts by `event_id`, runs in one transaction, is idempotent, and reports per-event rejections; pull pages by `server_seq`.
- `AuthenticatedLearner` is the P1b seam; P1a accepts `Bearer <learner_uuid>` only behind `allow_header_learner_id`.
- Content versions store the serialized bundle and its full SHA-256; the active endpoint returns both with `min_core_version`.
- `just fmt-check`, `just lint`, and `just test` pass; HTTP tests cover push, pull, auth, limits, future events, and active content.

## P1b — email-code auth and rolling sessions

- Added `/v1/auth/request-code`, `/verify-code`, `/refresh`, and `/logout`; codes are SHA-256 hashed with a 10-minute TTL, five attempts, and a 60-second cooldown.
- `auth_sessions` (migration `0016`) stores only access/refresh SHA-256 digests; access lasts 15 minutes, refresh 90 days and rolls on refresh and successful sync.
- `CodeDelivery` has a dev logger that returns the code only under `DEV_AUTH=1`, plus a no-op production stub.
- `AuthenticatedLearner` validates the bearer token against the database and exposes erasure; sync returns `{ erased: true }`, and refresh rejects erased accounts.
- Tests cover the full flow, wrong/expired/reused codes, expired access and refresh rotation, non-dev secrecy, logout, and erasure.

## W1 — `refl-core-wasm` browser bridge

- Added `core-wasm` (`refl-core-wasm`) with `cdylib`/`rlib` and wasm-bindgen JSON exports for decode, fold, plan, next/replan, answer check, and bundle compatibility.
- The bridge keeps `refl-core` pure `no_std + alloc`; no `serde-wasm-bindgen` is used, so payloads stay exactly `ts-rs` JSON.
- `fold_state` uses core replay or snapshot+apply and returns state, hash, incomplete, and last event id; unknown events mark the state incomplete.
- `plan_session` maps budget milliseconds to the core `Budget` and returns the plan plus its issued decision; tests cover the v3 bundle.
- Added the Nix `core-wasm-bridge` check, `wasm-bindgen-cli` to the dev shell, and the bridge build to `just wasm`.

## P1c — Telegram auth, invites, and access approval

- Added Telegram Login HMAC verification, closed/open registration, and invite redemption with SHA-256 digests only.
- Added `access_status`, the `roles` catalogue, and `invites` via migration `0017`; existing users remain active.
- Added admin REST endpoints and `AdminUser`; pending users receive plain Approve/Reject inline buttons.
- Added the optional Telegram Bot long-polling worker for approval callbacks and `/invite`.
- Tests cover HMAC/freshness, pending approval and rejection, single-use invites, open registration, and non-admin denial.

## W2 — PWA foundation, worker, storage, and sync client

- `just wasm` now emits wasm-bindgen's web glue, strips debug sections with `wasm-opt`, and keeps only the typed wrapper tracked.
- Added the Vite/React/TypeScript shell, offline service worker, and per-learner IndexedDB stores with atomic content activation.
- Added the core worker RPC, UUIDv7/clock-corrected event stamps, the auth/sync client, and outbox-driven bidirectional sync.
- Added Living Studio light/dark tokens with tactile surfaces and Fluent `en`/`ru`/`uz` UI copy.
- Vitest covers UUIDv7/clock drift, storage isolation/compaction/erasure, batching/pagination, rejected retention, and mid-session update deferral.

## W3 — prototype screens, Iella onboarding, lesson loop, and admin access

- Added auth, pending, rejected, and erased views with URL invite preservation, Telegram handoff, email codes, and dev-code display.
- Added the pixel Iella sprite, three-stop spotlight tour, local onboarding step records, and Settings replay.
- Added Today, the core-backed three-item lesson flow, summary, statistics disclosures, profile/settings, and admin invite/approval UI.
- Added the per-learner bundle cache, between-session update application, sync diagnostics, and Vite `/v1` + `/health` proxying.
- Added Vitest flow and screen tests for invite/pending, offline outbox flush, onboarding, statistics disclosures, and language/theme switching.

## P1d — bundle import CLI and admin bootstrap

- `refl-server import-bundle <path>` runs migrations, imports a sibling lexicon when present, imports and activates the bundle, and prints its content version.
- `ADMIN_EMAIL` and `ADMIN_TELEGRAM_ID` activate matching logins and idempotently add the `admin` role even under closed registration.
- Added `just web` for the client tests and production build.
- The access API test covers email and Telegram bootstrap admins, closed registration, and repeated login.

## W4 — design skins, living Iella, clean Today, and Telegram hash return

- Added five selectable skins (`living-studio`, `neo-brutalism`, `soft-neo-brutalism`, `soft-bento`, `laboratory`) alongside light/dark; `refl.skin` stores the local choice, `data-skin` applies it, and a skin change queues a `theme_selected` event whose latest event id wins across devices.
- Today now shows compact sync cues only for offline, pending, rejected, or clock-warning states. Normal sync and storage-persistence diagnostics were removed from Today; persistence status is reported only in Settings.
- Auth uses dedicated panel/foreground tokens with a scrim, so the left copy keeps contrast on every skin and mode. Illustration and Iella motion are disabled under `prefers-reduced-motion`.
- Iella renders on a padded SVG canvas with idle breathing, blinking, looking, perches on Today, lesson, sidebar, and dock, and stays pointer-transparent except for the tip and tour affordances.
- Telegram OAuth returns accept query params, `#tgAuthResult` base64 JSON, and `postMessage` from `https://oauth.telegram.org`; the return URL is clean and the redirect is same-window via `location.assign`.
- Added Vitest coverage for Telegram hash parsing, skin persistence/switching, and Today clean-state suppression.

## W5 — studio dev workflow, signature palettes, studio auth, and living companion

- Added `just dev`: it initializes `.pgdata` on `:5433` with its socket in `/tmp`, applies migrations, and runs `refl-server` and Vite under a process-group trap. `.env.local` is optional and untracked.
- Deep Umber + Old Paper + Morpho Blue is the default dark palette; Cream Paper + Riso Teal + Fluorescent Pink is the light palette. The five skins remain compatible with both modes.
- Replaced the native skin select with an accessible appearance popover (skins and palettes); Auth has a compact palette toggle that writes `refl.theme` through the shared App state.
- Auth now uses a local Canvas 2D procedural backdrop, textured opaque card surfaces, a composed route preview, animated OTP slots, and reduced-motion fallbacks.
- Today uses a flat masthead with a local day streak, a five-column `01`–`05` route, and an animated celestial/parallax hero. Settings moved to a utility group in the sidebar.
- Added `StudioIcons` (1.75px geometric strokes) for navigation, sync, palette, streak, and lesson controls.
- Iella has layered body, wings, eyes, tail, and feet with idle, flying, perched, curious, and proud moods. `IellaCompanion` patrols between DOM anchors every 18–25 seconds with cubic Bézier motion and reacts to routes, lessons, answers, palette changes, and clicks without blocking the page.
- Tests cover palette persistence, appearance selector interaction, Today's clean state and streak, and Iella mood/perch placement.

## W6 — reversed studio auth, tactile OTP confirmation, and tabbed settings

- Auth now puts the frameless sign-in column on the left (43%) and the studio canvas on the right (57%). The square grid and outer card are gone; Canvas 2D draws slow luminous ribbon arcs and edge rings instead of blurred orbs.
- The OTP step shows a masked target, large tactile digit slots, and a crisp active outline. Successful email or Telegram auth plays a 650 ms ring/check confirmation before entering the app; reduced motion skips it.
- Palette labels are now Night and Day with moon/sun emblems. The choice stays in `refl.theme` and passes through the shared App state into the authenticated shell.
- Iella returns to the original taller pixel silhouette with wings, feet, and crest intact. The random patrol timer is removed; she settles deterministically on the route's chosen perch and only reacts on route, lesson, answer, hover, and click signals with anticipation, Bézier flight, wingbeats, and a landing settle.
- Today and Progress icons now have distinct sun-horizon and trajectory-node silhouettes. The “Articles in context” hero uses palette-derived atmosphere, contour lines, layered paper hills, ribbons, celestial glow, and five numbered stepping stones.
- Settings is split into Appearance, Account & Sync, Offline & Data, and role-gated Admin tabs. Offline adds JSON cache export and a two-step local reset; Account exposes both sign-in methods and sync state.
- Added tests for the OTP confirmation gate, settings tab switching and admin gating, and the restored Iella proportions/moods.

## W7 — palette independence, profile split, felt/thermal auth, nature hero, and calm Iella

- Color is now a separate `palette` axis with four complete palettes: Deep Umber, Cream Paper, Sand & Forest, and Midnight Cobalt. Palette CSS blocks load after the skin blocks, so the five skins keep their geometry while every palette combines with every skin; `refl.palette` stores the choice and old `refl.theme` values migrate to the nearest palette.
- Auth is frameless on a palette-tinted felt surface with a 24-second CSS-only rotating light beam, and the right showcase uses blurred amber, coral, morpho, teal, and plum thermal fields with a scrim-backed headline. The success check runs for 550 ms with transform and `stroke-dashoffset` only, and the shell enters with a short transform-only settle.
- Profile is a first-class route with identity, role/status, streak, mastered KCs, total reviews, FSRS memory, derived milestones, linked sign-in methods, and sign-out. Settings is reduced to Appearance, Language, Offline & Data, and role-gated Admin; the sidebar learner card opens Profile and the adjacent gear opens Settings.
- Navigation can dock left, right, or bottom. `refl.layout.v1` persists `{ nav, order, sizes }`; narrow screens force the bottom dock and the content adds bottom space for the floating dock.
- Today is a draggable CSS Grid dashboard. Edit Layout enables a reduced-motion-aware wiggle, grip handles, keyboard move controls, compact/wide telemetry sizes, and reset; widget order and sizes are stored with navigation in the same versioned layout key.
- The vector-wave hero is replaced by a local 2560×1280 rendered nature landscape with a dark scrim, zero external requests. Today telemetry reads the folded WASM/IndexedDB state: an FSRS memory ring and due count, weakest active KCs, and a seven-week answer heatmap with best and current-day counts; empty states remain explicit.
- Iella is redrawn from the 12×11 canonical artboard (28 exact one-unit rects, canonical paper colors) with integer scale steps, `crispEdges`, `image-rendering: pixelated`, and overflow clipping. The flying companion is removed; she is anchored in the companion card and the Today route perch with only a 1 px stepped breath and a slow opacity blink.
- Added Vitest coverage for the four palettes, palette persistence/migration, Profile/Settings separation, shell route buttons and nav positions, Today telemetry and editable widgets, layout order/size/nav persistence and reset, activity folding, and canonical Iella geometry.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W8 — tactile thermal auth, real photo hero, Grafana widgets, frosted dock, and FSRS analytics

- Auth now draws a woven felt surface from a local multi-layer `feTurbulence`/`feDiffuseLighting` tile and woven gradients; the rotating beam is removed. The right side is a local Canvas 2D infrared field on `#1B0B3B`, with procedural drifting blobs, crimson/magenta/amber ramps, and isothermal contour bands; the headline sits directly on the field with a text shadow and no translucent panel.
- Auth carries the same compact `EN / RU / UZ` language pill as the shell, so locale can be chosen before sign-in. The pill expands on hover, focus-within, or tap and remains fully keyboard accessible.
- The shell topbar now has only the hover language pill and the appearance control; the duplicate top gear is gone. Settings remains available from the learner footer gear and from the bottom dock.
- The appearance popover is a 340px two-section surface: compact rounded-square palette chips and a five-row skin list with crisp checks, replacing the oversized rows and diagonal swatch dots.
- Bottom navigation uses a translucent frosted dock with `blur(18px) saturate(165%)`, an inner specular rim, a soft ambient shadow, and `96px` scroll clearance. Narrow screens always use the bottom dock.
- Today's dashboard is now a 3-column Grafana-style grid. Edit mode supports pointer drag with live neighbor reflow and FLIP transitions, keyboard move buttons, and a corner resize handle that snaps column spans `1/2/3` and row spans `compact/tall` with a dashed live outline. `refl.layout.v1` stores `order`, `colSpan`, `rowSpan`, and `nav`; the previous `sizes` shape migrates on read.
- The hero uses a locally bundled 3000×1792 CC0 photograph (Bernard Spragg, Lake Pukaki / Mt Cook, Wikimedia Commons; attribution in `web/src/assets/ATTRIBUTION.md`) with no runtime network request. Iella's perch moved to the hero's upper rim, clear of the level readout.
- `ProgressScreen` replaces the old three-card stats view with FSRS analytics: the exact `R(t) = (1 + t / (9 · S))⁻¹` curve, average stability and difficulty, an interactive forgetting-curve SVG with a 90% target, CEFR mastery bars, domain distribution, misconception strength and cure rates, and a calibration reference chart with an explicit empty state.
- Added Vitest coverage for span persistence/migration, pointer resize, language switching, frosted-dock clearance CSS, and Progress FSRS metrics.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W9 — sequenced checkmark, constrained widget grid, grounded Iella, brand mark, and isometric analytics

- Auth confirmation now finishes its light 450ms ring/check sequence before `onAuthenticated` fires, so `AppShell` and the hero JPEG mount only after the GPU-composited confirmation. Reduced-motion users enter immediately.
- Widget layout enforces `widgetMinColSpans` (`hero` and `rules` 2, telemetry 1) during normalization and pointer resizing. The rules widget is an inline-size container: five cards at full width, `3 + 2` at two columns, and a two-column reflow on narrow cards. Finger/mouse resizing now shows a translucent target ghost, numeric target badge, and spring-eased transitions.
- Removed the floating Today hero perch. Iella lives in the sidebar companion card and the dedicated Today companion bar, with integer-scaled breathing/blink only.
- Added `BrandLogo`, a folded-ribbon/reflection emblem on a 28-unit rounded field, and redrew Today, Practice, Progress, Profile, and Settings on the shared 1.75px stroke set.
- `ProgressScreen` no longer shows the raw retrievability equation. The FSRS card keeps stability, difficulty, retention, and mean-interval metrics plus the interactive curve; CEFR mastery is now an animated isometric prism chart with hover/focus tooltips, and the curve/ring surfaces carry reduced-motion-aware particle and liquid-morph ambience.
- Added Vitest coverage for minimum-span clamping, ghost resize feedback, `BrandLogo`, and equation-free analytics.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W10 — authentic felt and thermal auth, true refl logo, precise guides, compact text, and large activity heatmap

- The showcase headline stays on the 14ch measure with `clamp(2.1rem, 3.3vw, 3.1rem)`, `1.06` leading, and `-0.03em` tracking. RU remains three balanced lines; UZ uses `-0.08em` display tracking so the approved sentence resolves into two balanced lines at both desktop and mobile widths instead of a four-line stack or an orphan. The body copy is tightened in EN/RU/UZ, with proper Uzbek `o‘`/`g‘`.
- The auth left column is a two-layer `feTurbulence` wool-felt surface (low-frequency clumps + high-frequency fiber) with a fine woven dot layer and soft lighting. The old diagonal scratch/wood lines and rotating beams are gone. A compact language selector now sits beside the brand in the top-left; the palette control stays on the right.
- The right panel is a six-source infrared plasma on `#150630`: independent drifting centres, elongation, rotation, domain warp, and flow modulation, with the `#150630 → #3B0D6B → #9D174D → #E11D48 → #EA580C → #FACC15 → #FFF3C4` ramp, fine sensor grain, and no concentric contour rings. The headline sits in a stronger bottom-left dark zone, and the field freezes on its last frame when the success overlay pauses it.
- `BrandLogo` is redrawn from `design/reference/refl-logo-ref.jpg` as a pure-SVG dimensional ribbon: Deep Umber `#1A1512`, cream `#EAE2D3`, Morpho Blue `#1E88E5/#2B9AF3`, and Riso Teal `#0D7A78`, with no raster import. The Progress icon is simplified to three ascending bars and a sharp trend arrow; Today, Practice, Profile, and Settings keep the shared 1.75px optical set.
- The tour is portalled to `document.body` and targets exact `getBoundingClientRect()` coordinates with 8px padding. It dims with four fixed shade panels (no giant `9999px` box-shadow), shows a gold highlight with a `0.2s ease-out` transition, and places a compact 320px card beside the target. The old spotlight class and offset overlay are removed. Iella remains grounded in her companion spots with no hover slide.
- Widget resize keeps the hard minimum spans (hero and rules 2), the rules container queries (5-up, 3+2, 2-up), and the translucent ghost at `150ms cubic-bezier(0.2,0.9,0.3,1)`. Mobile `<=768px` hides all edit controls and forces a full-width stack while preserving the stored desktop spans.
- The activity heatmap covers 16 full weeks (112 cells) in 16 columns with Mon/Wed/Fri cues, per-day tooltips, a localized Less/More legend, and plural today counts. Measured cells are 14.9px at 1440px and 14.2px at 390px with 4px gaps.
- Added coverage for the exact tour highlight padding/shade panels, the 112-cell heatmap and legend, the mobile edit lock from an active desktop edit state, and the ribbon SVG/nav icon geometry. All 83 web tests pass.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W11 — 30/70 studio split, Burgundy Studio, tactile light felt, top sheen, and editorial sign-in

- At `>= 1200px` the auth grid is `minmax(400px, 31%) 1fr`, giving ultrawide screens a compact ~30% sign-in rail and ~70% thermal studio. The `901–1199px` band keeps the balanced `42% / 58%` pair, and the layout falls back to a single column on mobile; the narrow rail uses tighter panel padding and a compact five-swatch top bar.
- Added the fifth palette **Burgundy Studio**: `#1B0B10` canvas, `#271118` surface, `#331720` elevated surface, `#F5ECE3` ink, `#C4ABB2` muted ink, `#E03A5D` crimson-rose primary, and `#E2A652` brass. It is registered in TypeScript tokens, the CSS palette and swatch blocks, local persistence, the settings palette list, and the five-chip appearance selector; the felt panel is tinted deep bordeaux.
- Light palettes now carry a denser two-tone woven felt layer with higher opacity and fiber contrast (Cream Paper and Sand & Forest), so the grain is visible at normal scale instead of reading as flat beige. Dark palettes retain the quieter weave, and the Burgundy rail lifts it slightly for the bordeaux wool.
- The felt column receives a top-edge studio sheen: the specified `rgba(255, 244, 228, 0.07) → transparent 26%` wash on dark palettes and a warmer sunlit-paper falloff on light palettes. `AuthBackdrop` now eases a secondary warmth plume toward pointer position or the focused email/OTP field, with damping; the interaction is not attached under `prefers-reduced-motion`.
- The auth copy is now editorial: “Step into the studio”, “One-time code by email, or instant entry with Telegram.”, with the specified RU/UZ translations. The form header uses `clamp(1.85rem, 2.2vw, 2.35rem)`, weight 700, `-0.03em` tracking and tuned vertical rhythm; the email input and buttons get a two-pixel focus ring with a soft halo.
- Added coverage for the five-palette registry, Burgundy values/darkness, local persistence and application, appearance-menu selection, and the new editorial AuthScreen copy plus all five palette chips. All 84 web tests pass.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W12 — 30/70 auth, crimson-bordeaux felt, crane mark, four-eddy plasma, and kinetic sign-in

- At `>= 1280px` the auth grid is `minmax(400px, 31%) 1fr`; `901–1279px` keeps the balanced `42% / 58%` pair, and `<= 900px` stacks. The compact rail uses the five-swatch top bar; the crane brand mark is 52px in the sidebar, 40px on auth, and 32px on the mobile shell.
- Burgundy Studio is now a saturated crimson-bordeaux surface: `#5A1219` felt/canvas, `#410C11` surface, `#68161E` elevated, `#38090E` inputs, `#8E2430` borders, `#FAF2E8` ink, `#C29297` bordeaux ink, and `#E5A55A` brass. Inputs and focus rings no longer use teal/green.
- Removed the top-edge sheen and the auth-panel `::after` wash completely. The felt uses a denser two-tone weave on Burgundy, Cream Paper, and Sand & Forest, with stronger wool fibre contrast.
- Replaced the ribbon `BrandLogo` with a pure-SVG crane-fused mark: layered umber/burgundy/cream wing feathers, Morpho body, Riso Teal tail, long neck, crest and beak; no raster asset. Nav silhouettes remain the shared 1.75px set with distinct active/idle treatment.
- The thermal canvas is now four asymmetric drifting eddies on `#180424`, using `#D91E36`, `#EA580C`, `#FBC02D`, and warm `#FFF2C7` cores, with sensor grain and the existing damped pointer/focus nudge (disabled under reduced motion). The gray scrim layers are removed so the plasma sits directly behind the headline.
- Sign-in remains editorial (“Step into the studio” / “Войти в студию” / “Studiyaga kirish”); RU uses the warmer “быстрый вход” subtitle. The form header uses the W11 compact type ramp, and the Send Code/verify/Telegram buttons get a spring-overshoot press, a busy-state elastic pulse, and a brief liquid-morph label (all disabled under reduced motion).
- The RU welcome headline stays on three lines; UZ keeps the existing two-line `14ch` treatment with `-0.08em` display tracking because the approved sentence cannot make three balanced lines at `14ch` without orphaning “tili.” or stacking four lines.
- The tour still targets exact `getBoundingClientRect()` bounds with 8px padding, four shade panels, and a portalled 320px card; Iella stays in her companion slots. Today retains minimum spans, container-query rule reflow, the resize ghost, and the 16-week heatmap; the dock keeps `blur(18px) saturate(165%)` and 96px clearance; Progress remains equation-free with isometric CEFR blocks and particle ambience.
- Added/updated coverage for the crane SVG, the five-palette registry, Burgundy’s new colour values/darkness, persistence/application, appearance selection, and the editorial auth copy/five chips. All 84 web tests pass.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W13 — prism-reflection mark, four-destination navigation, curated bento Today, plain-language analytics, liquid auth, Pixel World, and public profile preview

- Replaced the crane `BrandLogo` with a pure-SVG prism-reflection emblem on a transparent `40 × 40` artboard. Two interlocking brackets use Deep Umber (`#1A1512` family) for the left stem, Morpho Blue (`#1E88E5`) for the mirrored stem, Riso Teal (`#0D7A78`) for the lower shoulder, and a cream shoulder that ties into the `refl` wordmark. The mark has no raster import and is exercised at 24, 32, and 48px.
- The shell now exposes exactly four destinations in both the side rail and the bottom dock: Today, Practice, Progress, Profile. The footer gear and the Settings destination are gone; the learner dock is an identity/Profile shortcut only. Settings is opened from the Profile header and rendered inside the Profile surface.
- `refl.layout.v1` is now a navigation-only record (`{ nav }`); old stored widget order/span fields are ignored on read, and the Today screen has no edit mode, pointer drag, resize handle, ghost target, or widget order. Navigation position (left/right/bottom) still persists and lives under Profile → Settings → Appearance.
- Today is a fixed 12-column bento: route (6) + level (3) + due (3), mastery (5) + weekly activity (7), error notebook (6) + notes/route focus (6). The route card carries Iella, 10/15/25-minute pills, and a Start session CTA; the duration is passed as `LessonRunner.budgetMs`. Mastery rows use semantic Mastered/Fading/Learning/Not started pills; recurring misconceptions show `×N` badges; route notes are derived from weak rules and active mistakes.
- Progress labels are plain language in EN/RU/UZ: Memory retention, Memory strength, Topic complexity, Average review gap, Mastered rules, and Forecast accuracy (with Prediction check as the kicker). The interactive curve, isometric CEFR prisms, domain bars, cure progress, and particle/liquid motion remain. Residual visible FSRS wording was removed, including the empty-memory state.
- Auth drops the email-stage subtitle and its FTL key. The form title now uses an eight-second flowing `background-clip: text` gradient; it freezes to a fixed position under `prefers-reduced-motion`. The language selector highlights exactly one option: the active language when idle, or the hovered/focused option otherwise. The five palette swatches sit in one tight pill container with no per-choice border; the active choice rings only its swatch. The best-effort persistent-storage request no longer blocks sign-in.
- Added the sixth skin, Pixel World, across `tokens.ts`, `tokens.css`, and `app.css`: 2px borders, hard 3–4px offset shadows, square controls and previews, a 24px sky/grid canvas, square progress/badges, and `image-rendering: pixelated` Iella. Labels and notes are localized in all three locales.
- Profile is now a learner banner with Iella, level progress, milestones, and stat cards. A public-profile preview toggle works with Self/Team/Friends/Everyone visibility; the visitor view hides the learner id, error notebook, and quick notes while keeping public level/stats/milestones. The embedded Settings tabs are Appearance, Account & Sync, Offline & Data, and the role-gated Admin (pending approvals and invite generation via `AdminAccessSection`). Account & Sync carries language, linked methods, sync status, and sign-out.
- Added/updated coverage for the prism SVG at three sizes, exactly four shell destinations, navigation-only layout migration, the fixed bento and duration callback, semantic status pills, Pixel World selection and CSS geometry, plain-language Progress labels, the 8-second liquid title, single-focus language highlighting, and the Profile visitor preview plus embedded settings tabs. All 84 web tests pass.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W14 — full-width nature hero, reflection mark, solid auth title, and global pill cleanup

- Today restores the bundled `hero-nature.jpg` as a true `grid-column: 1 / -1` photographic hero with a directional left/bottom scrim and `min-height: 280px`. Iella, the A1→A2 level cue, the 10/15/25-minute controls, and the `Start session` CTA all live inside it. Beneath it, the remaining modules are three balanced four-column cards per row: Level Progress + Reviews & Observations + Weekly Activity, then Articles Mastery + Error Notebook + Notes & Route Focus.
- `BrandLogo` is now a 28 × 28 pure-vector reflection mark: two offset opposing arcs drawn with `currentColor` and `var(--accent-primary)`, no gradients, raster imports, or multi-colour blocks. `--ink-primary` and `--accent-primary` are aliases on the palette root.
- The skin set is streamlined to Living Studio, Soft Bento, Laboratory, and Neo-brutalism. Pixel World and soft neo-brutalism were removed from the TypeScript registry, token CSS, app CSS, labels, selector, and tests. The authenticated top bar now shows all five palette swatches directly through `PaletteToggle`; the appearance popover is skin-only, and AuthScreen keeps the same direct five-swatch control.
- Static metadata capsules were removed across the client. The `Badge` component and `.ui-badge` CSS are gone; `StatusMeta` renders inline text with semantic colour and an optional 6 px dot. Role/local pairs use `·` separators; streak, level, sync cues, mastery states, misconception states, corpus counts, method status, and privacy controls are editorial text rather than rounded chips. Activity `Best`, Summary practised rules, and the update banner were cleaned too. Duration, language, palette, visibility, and skin controls remain tactile because they are interactive.
- AuthScreen drops the animated `background-clip: text` fill entirely. `Step into the studio` / `Войти в студию` / `Studiyaga kirish` is solid `var(--ink-primary)` at `-0.03em` tracking on the felt surface.
- Coverage now includes the full-width hero DOM/photo/CSS, the reflection mark at 24/32/48 px, the four-skin registry and retired-skin rejection, all five direct palettes in the shell, the solid auth title, and the absence of badge/mastery/streak/sync capsules in Today, Progress, Profile, and Settings. All 88 web tests pass.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W15 — bottom Profile card, APCA contrast, open reflection gap, and studio interaction principles

- The desktop sidebar rail now has exactly three learning destinations: Today, Practice, Progress. Profile lives on the bottom learner identity card (`{learner} · Profile · This device`) with a strong active outline/inset accent; the mobile/bottom dock keeps Profile as the fourth item. Hick's Law is reflected in the three rail choices, three duration presets, and one practice card at a time.
- Added pure-TypeScript APCA 0.1.9 `Lc` math in `web/src/theme/apca.ts` and a contrast suite in `web/src/theme/apca.test.ts`. All five palettes are calibrated for body text on every base/tinted surface (≥75), muted metadata (≥60), primary/secondary/danger/ghost action text (≥65), input text/metadata, auth-panel text/metadata, and display headings (≥50). Text-muted was raised on Deep Umber, Burgundy, Midnight Cobalt, and Sand & Forest; auth-panel muted was raised on Burgundy and Sand & Forest. New `actionPrimary*`, `actionDanger*`, and `actionGhostText` tokens preserve accent character while meeting APCA. The test also parses each `:root[data-palette]` block to keep CSS and TypeScript values in lockstep.
- `BrandLogo` now has an explicit open diagonal gap of 2.79 units between the opposing ribbon endpoints (stroke included) in the 28 × 28 frame; the ink and accent arcs remain separate rather than closing into a loop.
- Added `CrossfadeIcon` (180 ms opacity/scale/blur(4px), instant under reduced motion) and `PulseConfirm` primitives. Crossfade is wired into invite copy/check, Settings sync status, palette selection, and disclosure plus/minus. Confirmation pulses fire for correct practice/pretest feedback, OTP success, invite copy, and the new Settings “Retry sync” action.
- Auth now measures and transitions its form-region height between email and OTP stages; `Disclosure` keeps its content mounted with animated `grid-template-rows`; Today rule rows expand their memory detail with the same grid-row technique.
- Progress's horizon slider and Practice answer cards implement damped 0.28× rubber-band overscroll with spring snap-back. Forgetting-curve paths, isometric CEFR bodies, and Today mastery fills use clip-path wipe reveals. Iella's tour dims and blurs the four non-target shade panels (2px backdrop blur), scrolls targets into view with smooth/auto reduced-motion behavior, and keeps the exact spotlight.
- Primary targets are at least 44px (durational pills, check/answer buttons; Start session 52px), while destructive settings actions are compact and separated. Pressed states respond in 70ms; local WASM interactions keep the optimistic busy state without a blocking spinner.
- Coverage now includes the 3-item rail plus bottom Profile card, APCA thresholds and CSS/TS parity, the open logo gap, CrossfadeIcon state transitions, and Iella focus staging. All 94 web tests pass.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W17 — crisp auth fields, direct entry, even palette swatches, and restrained motion

- Auth inputs and OTP slots now have exactly one 1px border. Their focus state changes that border colour and adds a single 1px inset focus ring; the previous offset outline, outer glow, bottom-offset shadow, and OTP translateY jump are gone. This also removes the ghost panel that appeared under a focused email field.
- The confirmation overlay (`Signed in`, backdrop tint, animated mark, and pulse rings) was removed from AuthScreen. Verification calls `onAuthenticated` directly, so the workspace appears once with no duplicated auth state or reflowed thermal panel during entry.
- The five palette choices are uniform 28 × 28 px hit areas with 20 × 20 px swatches, a 6 px gap, and one quiet 2 px active outline; mobile keeps all five visible at 24 × 24 px choices and 18 × 18 px swatches. The earlier double outline/box-shadow active treatment was removed.
- Motion is standardised at 140 ms for interactive controls (120 ms for auth press feedback): auth buttons no longer lift, spring, or blur their label; auth form/card entrances are 140 ms; CrossfadeIcon is a 140 ms opacity/scale change without blur; PulseConfirm is one 140 ms flash; disclosure, auth-height, rubber-band, and entry transitions are 140 ms. Decorative clip-path reveals were removed so content is visible immediately. The reduced-motion block disables the new auth and entry transitions.
- Coverage now includes all five palette buttons and their active ring, single-email-input rendering plus source checks for the one-border/one-inset-ring rules, immediate entry with no success overlay, and the updated reduced-motion and crossfade contracts. All 95 web tests pass.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W18 — geometry-led shadows, an explicit session exit, and a calmer Laboratory

- Elevation now belongs to the skin rather than every palette surface. Living Studio, Soft Bento, and Laboratory set all five shadow tokens to `none`; cards, modals, navigation, and buttons separate with their existing borders and surface shifts. Soft Bento's cards regain the hairline border instead of a blur, secondary buttons gain a 1px border, the Laboratory nav glow and mobile-dock drop shadow are gone, and the tour highlight uses its outline rather than a halo.
- Neo-brutalism is the only skin with offset elevation: buttons use `4px 4px 0`, cards `6px 6px 0`, and hero/modal surfaces `8px 8px 0`, all in `var(--text)` with no blur or spread. CSS box-shadow follows the element's own `border-radius`, so the offset silhouette matches rounded cards and pills exactly.
- The active session action is one `lesson-exit` key with the same label across the UI: `Exit session` (EN), `Выйти из занятия` (RU), and `Mashqdan chiqish` (UZ). The Russian and Uzbek wording stays aligned with the product's existing `Занятие` / `Mashq` session vocabulary.
- Laboratory's graph paper drops from 10px/100px cells to 28px/140px and its line opacity is more than halved in both light and dark themes. Cards gain 24px padding (30px on exercise cards), the bento gap widens to 18px, answer rows reach 60px, buttons and settings tabs 46px, and navigation items 48px; the mobile layout keeps 20px cards and a 14px bento gap.
- Coverage now includes the exit label in all three locales, quiet-vs-Neo shadow math and geometry, nav/button border treatment, and the Laboratory grid scale plus spacing rules. All 99 web tests pass.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W19 — restrained confirmation checkmarks and account-first settings

- Sign-in now shows a bare 18px checkmark beside the form title for 160ms after a successful verification, then enters the workspace. It has no overlay, backdrop, pill, pulse, shadow, or blur; the check fades/scales in over 140ms, and the animation is disabled under `prefers-reduced-motion`. The check is announced as a `status` with the existing `auth-success` label, so no duplicate visible “Signed in” block returns.
- Copy, sync, and practice feedback no longer use confirmation pulses. Invite copying and manual sync show the existing 140ms `CrossfadeIcon` check state, and correct/incorrect practice feedback carries a check/close icon directly in the verdict. The now-unused `PulseConfirm` component and its CSS were removed.
- Settings now opens on Account & Sync, with tabs ordered Account & Sync → Offline & Data → Appearance → Admin. Learner identity, linked methods, language, sync, and sign-out are reachable immediately; sign-out remains inside the account card and was not moved to the workspace. The embedded settings note was updated in EN/RU/UZ to match the new order.
- Coverage now includes the auth confirmation checkmark (label, 140ms animation, zero pulse), the copy action's checkmark with no pulse, and the account-first tab order/default selection in both ProfileScreen's embedded settings and the settings screen. All 100 web tests pass.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W20 — confirmation checkmarks that stay visible

- Auth confirmation now holds for 450ms before entering the workspace (220ms under `prefers-reduced-motion`). The hold is exposed through `confirmationHoldMs()` so the timing is testable; the form region is remeasured when the check appears.
- `.auth-confirm-check` is a 20px inline-flex, centre-aligned slot using `var(--text)` and a 2.4 stroke. It sits inline with the title, stays fully inside the measured form region, has no opacity-held state, and keeps only the 140ms fade/scale animation (disabled under reduced motion).
- Audited the other confirmation marks: lesson correct verdicts carry an 18px check in the verdict colour, invite copying uses the active `CrossfadeIcon` check with no pulse, and manual sync swaps its button icon to the active check. Crossfade active layers are explicitly asserted at `opacity: 1`, and no `PulseConfirm` remains.
- Coverage now includes 450ms/220ms hold selection, inline-flex/high-contrast/uncropped auth check CSS, sync and copy check states, and the lesson verdict check. All 102 web tests pass.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W21 — reload-safe sessions and a stroke-drawn sign-in check

- AuthGate now persists a small session-owner pointer in `localStorage` (`learnerId` + sign-in method) and keeps the tokens themselves in the learner's IndexedDB `auth_session` record. Successful email and Telegram sign-ins write the session before handing control to App, so a reload can reconstruct tokens from storage.
- On boot AuthGate enters a short `auth-refreshing` state, reopens the last learner's storage, and calls `onAuthenticated` with the stored session. Expired access tokens are deliberately left expired for the first authenticated request to refresh. Sign out and rejected sessions clear the owner; a missing session falls back to AuthScreen.
- Refresh handling now clears persisted credentials only on a genuine `401`/`403`; network and `5xx` refresh failures retain the session so the app can stay signed in and retry later. App clears identity and the owner when the API reports an invalid session, while normal offline failures keep the workspace.
- The 18px auth checkmark is now a real SVG path with `pathLength=1`, `stroke-dasharray=1`, and `stroke-dashoffset=1`, drawn over 320ms (linear), followed by a 550ms total hold before entering the workspace. Reduced motion skips the draw and shows the static check for 220ms. The same confirmation path runs for email OTP and Telegram returns.
- Coverage now includes AuthGate session ownership/restore across remounts, an actual sign-in followed by a remount, storage reopen persistence, transient-vs-401 refresh behavior, and SVG dash-draw/visibility assertions. All 107 web tests pass.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.

## W22 — exclusive Apple-Pay-style sign-in confirmation

- Removed the inline checkmark from `.auth-form-title`; the heading no longer carries any confirmation chrome.
- On successful email or Telegram authentication, `.auth-form-inner` is replaced by a dedicated confirmation state inside the same fixed-height `.auth-form-region`: a 68px circular outline (2px `var(--accent-primary)`) around a 32px `ConfirmationCheck`, plus the single `auth-signed-in` label. The confirmation fills the existing measured height and is centred; the right auth visual keeps exactly the same geometry and the previous form/backdrop are never layered together.
- The check path uses `pathLength=1`, `stroke-dasharray=1`, and `stroke-dashoffset=1`, drawing over 280ms with `cubic-bezier(0.22, 1, 0.36, 1)`. Reduced motion renders the path immediately. Confirmation holds for 650ms (250ms reduced) before `onAuthenticated`.
- Added `auth-signed-in` to the synchronized locales as `Signed in` / `Вход выполнен` / `Kirildi`; it is the sole visible confirmation label.
- Coverage now checks that the heading has no inline check, the form fields disappear in favour of the centred ring/path/label, the right visual geometry and form-region height remain unchanged, the path draw/hold timings, and clean entry after the hold. All 108 web tests pass.
- `just fmt-check`, `just lint`, `just test`, and `just web` pass; the commit is local and not pushed.
