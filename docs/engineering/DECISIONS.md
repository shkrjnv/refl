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
