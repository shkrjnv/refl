# Phase 2B.3 — RunLimit Termination

Status: implemented; awaiting independent acceptance review.
Date: 2026-11-26.

This is an engineering record. The normative planner contract remains
`docs/04-placement-planner.md`.

## 1. Objective and scope

Phase 2B.2 established that Rule 3 is executed and represented in native and
WASM builds. Its review found that RunLimit was enforced as a step-level
rejection but had no terminal `End` reason.

Phase 2B.3 resolves that gap. It does not change ordinary scheduling, the
solitary-KC exception, prerequisite scheduling, the maximum run, the final
block, content, mastery, diagnostics, or the Validator contract beyond the
run-rule alternative check described below.

## 2. Previous behavior

Before this phase:

- `Builder::push` returned `PushResult::RunLimit` before insertion
  (`core/src/plan/build.rs:143-169` before the change).
- `remaining()` returned only `(Vec<Step>, bool)`; the bool was
  `short_of_time`.
- `replan()` had no RunLimit branch. An empty plan with no time failure fell
  through to the existing `ItemsExhausted` fallback or another plan-state
  reason.
- `End` had no run-limit variant and `End::code()` had no matching code.
- The existing `a_run_limited_prerequisite_is_not_a_budget_failure` test
  asserted only `end != Budget`; it did not assert an exact terminal reason.

That could report `items_exhausted` even when the rejected candidate had an
unseen item and was rejected specifically by the run rule.

## 3. Implementation

### 3.1 New terminal reason

`End::RunLimit` was added with stable code `run_limit`
(`core/src/plan/mod.rs`). It is distinct from `ItemsExhausted`.

### 3.2 Private stop-cause propagation

`remaining()` now returns a private `Remaining` value:

```text
Remaining {
    steps: Vec<Step>,
    short_of_time: bool,
    run_limited: bool,
}
```

`Builder` records `run_limited = true` only when a candidate passes the
availability check and is then rejected by the run rule. `available()` checks
for an unseen eligible item/family without consuming capacity, so a candidate
that is both run-limited and unavailable is reported as unavailable rather
than as a RunLimit rejection.

`Builder::finish()` also records `run_limited` when a previously added step is
dropped because it would break the run rule. This preserves the terminal
cause when the post-processing pass makes the plan empty.

No public planner API was expanded.

### 3.3 End assignment

`replan()` uses the following deterministic priority when `steps.is_empty()`:

1. `short_of_time` → `End::Budget`.
2. `run_limited` → `End::RunLimit`.
3. Existing plan-state reasons: `NothingDue`, `ReviewsDone`, `KcDeferred`,
   `KcMastered`, `KcCap`, `ItemsExhausted`.

A run-limit rejection never sets `short_of_time`. A genuine time failure
independently sets it, so Budget remains the reason for real budget
exhaustion. If both occur, Budget wins because a time rejection is the
established higher-priority cause.

If any valid step remains, `end` stays `None`; alternatives and final steps
therefore keep the session alive.

### 3.4 Alternative availability

`nothing_else()` now counts a pending review as “other” only when that review
KC has an unseen eligible item. A pending review whose items are exhausted no
longer switches the run rule on. `remaining()` also filters such reviews out
before review time/interleave scheduling, so they cannot turn a no-unseen
state into a false Budget or keep a run-limited plan alive.

The validator's independent `alone_kc` reading was aligned with the same
condition. This avoids builder/validator disagreement without changing
review scheduling for reviews that still have an unseen eligible item.

### 3.5 Preserved behavior

- the run boundary remains `run >= max_run` for the next counted step;
- `Lesson`, `Block`, and `Final` remain outside the run rule;
- the solitary-KC exception still disables the run rule and uses `alone_cap`;
- prerequisite scheduling and `PushResult` behavior remain unchanged;
- final steps still have their own 10% share and remain scheduled when main
  steps are run-limited;
- planning remains deterministic.

## 4. Budget interaction

The deterministic rule is defined in `replan()`:

- A RunLimit rejection is never converted into `Budget`.
- If a genuine main-part time failure independently exists, the end is
  `Budget`.
- If a RunLimit rejection is the only remaining cause, the end is
  `RunLimit`.
- If neither applies, the existing end-reason rules apply.

The regression test
`a_genuine_budget_failure_takes_precedence_over_a_run_limit` covers the
overlap.

## 5. Compatibility analysis

### `End::code()`

`End::RunLimit.code()` returns `run_limit`. No existing code value changed.

### `Plan` serialization

`Plan` derives `Serialize`/`Deserialize`; `End::RunLimit` serializes as
`"run_limit"`. A round-trip test was added.

### `PlanIssuedEvent.end`

`PlanIssuedEvent.end` is `Option<String>` (`core/src/event.rs:411-413`), not
an enum. The planner writes the stable code through `issue.rs:109`. The
terminal scenario asserts the actual `end` on the last session event.

### `SessionReport.end`

The simulator copies the `Next::Finished` end code into
`SessionReport.end` (`tools/sim/src/cohort.rs:835-840`). No type or control-flow
change was needed.

### Simulator statistics and benchmarks

Benchmark/statistics code groups by end-code strings. A later simulator run may
now show `run_limit`; frozen benchmark files were not edited. No result was
fabricated or relabeled.

### UI, API, recovery, and replay

- Repository search found no UI consumer of a concrete `End` enum; the event
  surface uses the string code.
- The plan is device-local and not synchronized (`docs/04:410`).
- `plan_issued` is skipped by the learner-model fold; `end` is decision
  metadata, not model state.
- Replay/recovery therefore needs no model migration.
- No new event field or new event type was introduced.

### Schema

`docs/02-layer1-content.md:346` defines `end` as a string code from the
planner contract. `docs/04:341` states that end codes may change within
`schema_version` 2 while it is unreleased. This phase adds a value, not a
field or enum in the serialized event shape. No upcaster or migration was
added. If schema 2 were already released, the change would require the
versioning decision in `docs/13-sync-versions.md`.

The authoritative planner table in `docs/04-placement-planner.md` now lists
`run_limit` and its alternative, budget, and final conditions. `README.md` was
updated to include the new end reason.

## 6. Tests

Added deterministic tests in `tools/sim/tests/plan.rs`:

1. `run_limit_is_exactly_the_max_run_boundary`
2. `run_limit_is_a_terminal_reason_without_an_alternative`
3. `a_review_with_an_unseen_item_keeps_a_run_limited_plan_alive`
4. `another_learning_kc_is_a_valid_alternative_to_a_run_limit`
5. `a_pending_review_without_an_unseen_item_is_not_an_alternative`
6. `a_run_limit_keeps_final_steps_scheduled`
7. `a_genuine_budget_failure_takes_precedence_over_a_run_limit`
8. `the_solitary_kc_exception_does_not_create_a_run_limit`
9. `no_unseen_item_without_a_run_limit_stays_items_exhausted`
10. `a_terminal_run_limit_is_stable_across_repeated_adaptation`
11. `run_limit_end_serializes_as_its_stable_code`

The terminal test also checks the last `PlanIssuedEvent.end`. The review test
checks the independent Validator condition.

Existing tests were adjusted only where the new approved semantics require it:
the old cap test's artificial unavailable review no longer counts as an
alternative, so its review-run sub-check moved to the new explicit tests.

`cargo test --locked -p refl-sim --test plan --no-fail-fast` passed with
52 tests.

## 7. Native/WASM

A new module `tools/sim/src/termination.rs` uses the existing `plan-hash`
harness. It covers:

- terminal `run_limit` with no alternative;
- a run-limited learning KC with an eligible review alternative.

`plan-hash` prints and asserts exact end/steps fields. `just wasm-hash`
compares complete native and wasm output.

Representative output:

```text
runlimit_terminal end=Some("run_limit") steps=0 review_steps=0 final_steps=0
runlimit_alternative end=None steps=1 review_steps=1 final_steps=0
```

Both native and WASM produce the same lines.

## 8. Limitations

- The regression fixtures use a manually constructed synthetic `Plan` with a
  consumed `Lesson`; they do not model a full learner session.
- The terminal fixture uses a review on the same KC as the learning KC to
  construct a deterministic run-limit-only state. The production path under
  test is still the real `Planner::adapt`/`remaining`/`Builder` path.
- The synthetic answer is not folded through the learner-model event path.
- Benchmark result files were not regenerated; existing frozen results remain
  historical.
- The new end code is not a new event field or schema version.
- No UI consumer exists in this repository to update.

## 9. Verification

The final verification was run on the current worktree:

| Command | Exit | Result |
|---|---:|---|
| `cargo test --locked -p refl-sim --test plan --no-fail-fast` | 0 | 52 passed, 0 failed |
| `cargo test --locked -p refl-core --all-features --quiet` | 0 | all core tests passed |
| `cargo test --release --locked --workspace --all-features --quiet` | 0 | all workspace tests passed |
| `cargo fmt --all --check` | 0 | no formatting diff |
| `cargo clippy --locked -p refl-core -p refl-sim --all-targets --all-features -- -D warnings` | 0 | pass |
| `just wasm-hash` | 0 | native/WASM hashes and terminal scenario output match |
| `git diff --check` | 0 | no whitespace errors |

Native/WASM terminal output:

```text
runlimit_terminal end=Some("run_limit") steps=0 review_steps=0 final_steps=0
runlimit_alternative end=None steps=1 review_steps=1 final_steps=0
```

Both executions also produced identical prerequisite scenario lines and
matching fold/planner hashes.


## 10. Worktree

The implementation preserves the pre-existing uncommitted MVP-01B, Phase 2A,
Phase 2B.1, and Phase 2B.2 work. No reset, stash, checkout, clean, commit, or
push was used.
