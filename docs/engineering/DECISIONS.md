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
