# Engineering decisions

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
