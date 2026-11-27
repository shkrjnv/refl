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
