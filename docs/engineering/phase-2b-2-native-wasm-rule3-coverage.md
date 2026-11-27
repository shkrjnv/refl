# Phase 2B.2 — Native/WASM Rule 3 Coverage

Status: implemented and independently reviewed; accepted with conditions.
Date: 2026-11-26.

This is an engineering record, not a normative product specification. The
normative numbered documents in `docs/` remain the source of truth. The
record was placed under `docs/engineering/` because the existing `docs/`
tree contains numbered specifications and has no engineering-record category;
adding a sixteenth numbered specification would incorrectly present this
record as a new product decision.

## 7.1 Objective and scope

### Why this phase was necessary

`just wasm-hash` already compared native and WASM execution of:

- a fold of a generated 20,000-event log;
- three synthetic planner sessions;
- every emitted `PlanIssuedEvent` and each serialized `Plan`.

Matching hashes established only that the same outputs occurred for that
fixed trajectory. They did **not** establish that Rule 3 — the prerequisite
check in `docs/04-placement-planner.md`, «Адаптация», rule 3 — was ever
executed.

The `plan-hash` binary did not report `SessionReport.rules`, `Rule::`
values, or a required `prerequisite_check` outcome. A match could therefore
be obtained with no Rule 3 activation at all.

### What Phase 2B.2 proves

Phase 2B.2 adds two deterministic scenarios to the existing `plan-hash`
binary and executes them natively and under WASM through `just wasm-hash`.
Both scenarios explicitly assert:

- Rule 3 activation through `Planner::adapt`;
- the selected prerequisite KC;
- `PlanIssuedEvent.trigger == "prerequisite_check"`;
- the presence or absence of a `Phase::Prerequisite` step;
- subsequent/planned steps and Final steps;
- actual post-prerequisite planner continuation (`continued=true`);
- the resulting end code.

### Scope boundaries and exclusions

- No core planner behavior was changed.
- No content YAML, mastery, diagnostics, schema, End or Validator changes.
- No new simulator engine or separate execution architecture was introduced.
- The scenarios use synthetic content/state and a manually constructed prior
  session plan; they are not a real learner cohort.
- RunLimit/no-alternative termination was Phase 2B.3 work; it is now recorded
  in `docs/engineering/phase-2b-3-runlimit-termination.md`.
- `deferred_today` behavior was not changed in Phase 2B.2.
- No commit or push was made.

## 7.2 Architecture and implementation

### Existing architecture reused

`tools/sim` already provides:

- `synthetic_bundle` and `synthetic_log`;
- `Track`, `Learner`, and `session_ours`;
- `Planner`, `State`, `Plan`, `Next`, `Outcome`;
- `SessionReport.rules` and `PlanIssuedEvent`.

Phase 2B.2 adds a small shared scenario module in the simulator, not a new
simulator:

- `tools/sim/src/prereq.rs`
- exposed by `pub mod prereq;` in `tools/sim/src/lib.rs`.

The module uses the core `Planner` with the simulator's generated bundle and
state. The scenarios do not duplicate the core planner.

### How Rule 3 is reached

The scenario plan starts with:

- a learning KC (`k5`);
- one `Done` non-rapid incorrect answer on `k5`;
- a first `Phase::Block` step on `k5`.

`prereq::answer_wrong` then calls the real:

```rust
planner.next_item(state, plan, &History::default(), now, seed)
```

and feeds a non-rapid incorrect `Answered` into:

```rust
planner.adapt(state, plan, &Outcome::Answer(...), now)
```

Rule 3 is evaluated in `Planner::fired` when there is a second non-rapid
incorrect answer (`core/src/plan/adapt.rs:111-120`). The direct prerequisite
is selected by `Planner::suspect` (`adapt.rs:133-141`), which ranks by
`D × importance`, excludes current-session deferred KCs and requires an
unseen eligible item.

### Observed planner surfaces

The scenario outcome (`Scenario` in `prereq.rs:18-30`) records:

- `rule` from `Adapted.rule`;
- `selected` from `Rule::PrerequisiteCheck { kc }`;
- `trigger` from `Adapted.issued[*].trigger`;
- `prerequisite_steps` and `subsequent_steps` from `Adapted.plan.steps`;
- `final_steps` from `Adapted.plan.steps`;
- `end` from `Adapted.plan.end`;
- `continued`, `post_prerequisite_steps`, and
  `post_prerequisite_prereq_steps` from the separate prerequisite-completion
  `next_item` + `adapt` call.

`PlanIssuedEvent.trigger` is the persisted decision marker; `Adapted.rule`
is the direct return value of the adaptation step.

### Scenario module details

`tools/sim/src/prereq.rs` contains:

- `learning_state` — synthetic state with `k3` and `k4` at negative μ;
- `base_plan` — a learning plan for `k5` with one prior wrong answer;
- `slow_kc` — changes expected time for one KC’s current items/projections;
- `answer_wrong` — real `next_item` + `adapt` transition;
- `continue_after_prerequisite` — real `next_item` + `adapt` on the
  prerequisite step with a correct answer;
- `outcome` — extracts the observable Rule 3 fields;
- `scenario_a`, `scenario_b`;
- `expected_selected`, `expected_b_end`.

### `plan-hash` role

`tools/sim/src/bin/plan-hash.rs` runs the existing hash trajectory and then:

1. calls `prereq::scenario_a()` and `prereq::scenario_b()`;
2. prints one deterministic line for each;
3. asserts the required Rule 3 outcomes;
4. returns `ExitCode::FAILURE` if either scenario is absent or wrong.

`just wasm-hash` builds `plan-hash` for `wasm32-wasip1`, runs it natively and
through wasmtime, and compares the complete stdout strings.

## 7.3 Scenario A — successful prerequisite scheduling

### Fixture

- synthetic bundle: 6 KCs × 12 items;
- learning KCs: `k5` and `k0`;
- `k3` and `k4` are direct prerequisites of `k5`;
- synthetic state sets both `k3` and `k4` to μ = −3.0, variance 1.0;
- `k4` has higher importance than `k3` (synthetic values), so `k4` is the
  expected maximum of `D × importance`;
- `k5` has one prior non-rapid error;
- the second answer generated by `answer_wrong` is non-rapid and incorrect;
- budget: `Budget::TwentyFive`.

### Actual execution path

1. `next_item` chooses the initial `k5` block item.
2. `adapt` records the second error.
3. `fired` returns `Rule::PrerequisiteCheck { kc: k4 }`.
4. `remaining` evaluates the prerequisite’s mean duration against
   `main_left`.
5. The prerequisite fits; `Builder::push` adds `Phase::Prerequisite`.
6. The remaining plan is recomputed; `k0` supplies later planned steps.

### Assertions and observed output

`prerequisite_scenarios_expose_rule3_for_native_parity` and `plan-hash`
assert:

- rule code `prerequisite_check`;
- selected KC `k4`;
- trigger `prerequisite_check`;
- exactly one prerequisite step;
- at least one non-prerequisite step;
- at least one Final step;
- actual continuation after the prerequisite (`continued=true`);
- no pending prerequisite step after that continuation.

Native/WASM output:

```text
prereq_a rule=prerequisite_check selected=Some(KcId("en.gram.k04")) trigger=Some("prerequisite_check") prerequisite_steps=1 subsequent_steps=14 final_steps=2 end=None continued=true post_prerequisite_steps=14 post_prerequisite_prereq_steps=0
```

### What is genuinely verified about subsequent planning

After the Rule 3 replan, `prereq::scenario_a` calls the real
`Planner::next_item` on the prerequisite step, feeds a correct
`Outcome::Answer` into `Planner::adapt`, and records:
- `continued = true`;
- `post_prerequisite_steps = 14`;
- `post_prerequisite_prereq_steps = 0`.

The post-answer plan contains only one prerequisite step before the
continuation; after the continuation it contains no pending prerequisite
step and has subsequent planned steps.

Limitations:

- The continuation uses the existing **`State`** without folding the
  synthetic answer through the learner-model event path. The planner
  continuation is real, but the state change is not modelled.
- It does not continue through a Final step or a complete session.
- It uses a synthetic initial plan and manually inserted prior error.

This is a genuine planner continuation after the prerequisite, not merely a
count of non-prerequisite steps in the same replan. The remaining
state-fold limitation is explicit.

## 7.4 Scenario B — budget rejection

### Fixture differences from Scenario A

- `k4` expected time is set to 1,000 seconds by `slow_kc`;
- budget is `Budget::Ten`;
- `k5` and `k0` remain learning KCs so the rule event has a plan candidate;
- `k5` still has one prior non-rapid error;
- a second non-rapid incorrect answer activates Rule 3.

### Production branch exercised

`remaining` (`core/src/plan/build.rs:617-634`) checks:

```rust
let seconds = self.mean_seconds(kc);
if f64::from(seconds) > main_left {
    out.short_of_time = true;
    main_short = true;
} else {
    match out.push(Step::new(kc, Phase::Prerequisite, false, seconds)) {
        PushResult::Added => main_left -= f64::from(seconds),
        PushResult::RunLimit | PushResult::Unavailable => {}
    }
}
```

With `mean_seconds(k4) = 1_000` and a ten-minute main budget below that,
the first branch executes: no prerequisite is pushed and `main_short` is
set.

### Why RunLimit, Unavailable and no-unseen do not explain the rejection

- `suspect` requires an unseen eligible item, so `has_unseen(k4)` is true.
- The budget comparison happens **before** `Builder::push`, so
  `PushResult::RunLimit` and `PushResult::Unavailable` are not reached for
  the prerequisite.
- The scenario has one active learning KC `k0` with no second candidate or
  partner; the run rule is off, so RunLimit cannot be the cause.
- The 1,000-second expected time is directly above the remaining main
  budget.

### Final scheduling

The prior errors produce Final families. Final scheduling is outside the
`main_short` block and independently checks its own 10% share. The observed
plan contains Final steps and has no end at this decision.

### Observed output

```text
prereq_b rule=prerequisite_check selected=Some(KcId("en.gram.k04")) trigger=Some("prerequisite_check") prerequisite_steps=0 subsequent_steps=2 final_steps=2 end=None continued=false post_prerequisite_steps=0 post_prerequisite_prereq_steps=0
```

### What is and is not directly observable

- Directly observed: Rule 3, selected KC, trigger, no prerequisite step, Final
  steps, no end.
- Inferred from source and fixture: `short_of_time = true` and
  `main_short = true`. `Plan` does not expose either internal flag.
- Not directly asserted: the exact numeric `main_left`. It is established
  by `Budget::Ten`, the prior item seconds, and the 1,000-second k4 estimate.
- Not tested here: RunLimit and no-unseen rejection; those are separate
  paths.

## 7.5 Native/WASM parity

### Deterministic inputs

Both scenarios use:

- `synthetic_bundle(BundleSpec { kcs: 6, items_per_kc: 12 })`;
- the fixed `T0` timestamp;
- fixed seeds `61` (plan seed) and `62` (next-item seed);
- fixed synthetic KcState values;
- `Params::default()` and `ReadingSpeed::default()`;
- deterministic `slow_kc` changes in Scenario B.

### Fields compared

`plan-hash` prints and compares:

- rule;
- selected KC;
- trigger;
- prerequisite step count;
- subsequent step count;
- Final step count;
- end;
- actual continuation flag and post-prerequisite step counts.

The full line is part of the native/WASM stdout comparison performed by
`just wasm-hash`.

### Complete scenario output

Native:

```text
prereq_a rule=prerequisite_check selected=Some(KcId("en.gram.k04")) trigger=Some("prerequisite_check") prerequisite_steps=1 subsequent_steps=14 final_steps=2 end=None continued=true post_prerequisite_steps=14 post_prerequisite_prereq_steps=0
prereq_b rule=prerequisite_check selected=Some(KcId("en.gram.k04")) trigger=Some("prerequisite_check") prerequisite_steps=0 subsequent_steps=2 final_steps=2 end=None continued=false post_prerequisite_steps=0 post_prerequisite_prereq_steps=0
```

WASM:

```text
prereq_a rule=prerequisite_check selected=Some(KcId("en.gram.k04")) trigger=Some("prerequisite_check") prerequisite_steps=1 subsequent_steps=14 final_steps=2 end=None continued=true post_prerequisite_steps=14 post_prerequisite_prereq_steps=0
prereq_b rule=prerequisite_check selected=Some(KcId("en.gram.k04")) trigger=Some("prerequisite_check") prerequisite_steps=0 subsequent_steps=2 final_steps=2 end=None continued=false post_prerequisite_steps=0 post_prerequisite_prereq_steps=0
```

### What matching output proves and does not prove

It proves:

- the scenario code runs under both native and wasm32-wasip1;
- both use the same deterministic inputs;
- the explicit Rule 3 fields and assertions execute in both builds;
- the complete scenario lines are identical.

It does not prove:

- all possible Rule 3 branches or all model states;
- continuation after answering the prerequisite;
- RunLimit behavior;
- equivalence for real content or real learners.

## 7.6 Verification record

All checks were executed on the current worktree.

| Command | Exit status | Observed result |
|---|---:|---|
| `cargo test --locked -p refl-sim --test plan --no-fail-fast` | 0 | 41 passed, 0 failed |
| `cargo test --locked -p refl-core --all-features --quiet` | 0 | all core tests passed |
| `cargo test --release --locked --workspace --all-features --quiet` | 0 | all workspace tests passed |
| `cargo fmt --all --check` | 0 | no formatting diff |
| `cargo clippy --locked -p refl-core -p refl-sim --all-targets --all-features -- -D warnings` | 0 | pass |
| `just wasm-hash` | 0 | native/WASM fold, planner hash and scenario lines match |
| `git diff --check` | 0 | no whitespace errors |

The `just wasm-hash` run printed:

```text
native: events=20000 model=knowledge-v1.3+68ea62decb36521e fold=f0a6585e046f620eb2cd7a75efe90758f7d594a544a73f61334b73fc2859d264 hash=ff400f505425ebe977fc0ea5335a52e040f26685836c39755078a0aa6d9a1e37
wasm:   events=20000 model=knowledge-v1.3+68ea62decb36521e fold=f0a6585e046f620eb2cd7a75efe90758f7d594a544a73f61334b73fc2859d264 hash=ff400f505425ebe977fc0ea5335a52e040f26685836c39755078a0aa6d9a1e37
native: sessions=3 decisions=187 violations=0 planner=session-prototype-4+c896293765c21793 state=5ce9c82cb6dbbe5743d35efd666f88b2e3df0eb82a045aee1f3712846856d9a6 hash=10c2b5b4105f51ee8e75a9142e22d26dab0520f7f04c3b8e4cb313f69a70eb37
prereq_a rule=prerequisite_check selected=Some(KcId("en.gram.k04")) trigger=Some("prerequisite_check") prerequisite_steps=1 subsequent_steps=14 final_steps=2 end=None continued=true post_prerequisite_steps=14 post_prerequisite_prereq_steps=0
prereq_b rule=prerequisite_check selected=Some(KcId("en.gram.k04")) trigger=Some("prerequisite_check") prerequisite_steps=0 subsequent_steps=2 final_steps=2 end=None continued=false post_prerequisite_steps=0 post_prerequisite_prereq_steps=0
wasm:   sessions=3 decisions=187 violations=0 planner=session-prototype-4+c896293765c21793 state=5ce9c82cb6dbbe5743d35efd666f88b2e3df0eb82a045aee1f3712846856d9a6 hash=10c2b5b4105f51ee8e75a9142e22d26dab0520f7f04c3b8e4cb313f69a70eb37
prereq_a rule=prerequisite_check selected=Some(KcId("en.gram.k04")) trigger=Some("prerequisite_check") prerequisite_steps=1 subsequent_steps=14 final_steps=2 end=None continued=true post_prerequisite_steps=14 post_prerequisite_prereq_steps=0
prereq_b rule=prerequisite_check selected=Some(KcId("en.gram.k04")) trigger=Some("prerequisite_check") prerequisite_steps=0 subsequent_steps=2 final_steps=2 end=None continued=false post_prerequisite_steps=0 post_prerequisite_prereq_steps=0
```

## 7.7 Files and changes

### New

- `tools/sim/src/prereq.rs` — deterministic Rule 3 scenarios and observable
  outcome extraction.
- `docs/engineering/phase-2b-2-native-wasm-rule3-coverage.md` — this record.

### Modified in Phase 2B.2

- `tools/sim/src/lib.rs` — exposes the `prereq` module.
- `tools/sim/src/bin/plan-hash.rs` — runs scenario A/B, asserts outcomes,
  prints scenario lines for native/WASM comparison.
- `tools/sim/tests/plan.rs` — adds
  `prerequisite_scenarios_expose_rule3_for_native_parity`.

### Pre-existing changes preserved

The following pre-existing uncommitted work remains:

- `core/src/plan/adapt.rs`;
- `core/src/plan/build.rs`;
- `core/src/plan/mod.rs`;
- existing additions in `tools/sim/tests/plan.rs`.

No core planner behavior was changed in Phase 2B.2. No content YAML,
mastery, diagnostics, schema, End, or Validator contract was changed.

No commit or push was made.

## 7.8 Decisions and rationale

- **Reuse `tools/sim` and `plan-hash`** instead of creating another simulator.
  The existing bundle generator, state representation and core planner are
  sufficient; a parallel harness would duplicate behavior and risk drift.
- **Expose explicit Rule 3 outcomes** rather than relying on hashes. Matching
  hashes cannot prove branch coverage; `Adapted.rule`, `PlanIssuedEvent.trigger`
  and `Plan.steps` are directly inspectable.
- **Use synthetic deterministic fixtures.** They make Rule 3 reachable
  without probabilistic learner simulation and keep native/WASM inputs
  identical. The cost is reduced realism.
- **Keep Scenario B in the budget branch and avoid RunLimit.** The fixture
  uses a single active learning KC and a very slow prerequisite; the
  budget check happens before `Builder::push`.
- **Continue Scenario A after the prerequisite.** The follow-up review asked
  for more than same-plan step counting, so the scenario now calls
  `next_item` and then `adapt` with a correct prerequisite answer. The
  answer is not folded through the model event path; that limitation is
  explicit.
- **Avoid core production changes** because no planner defect was
  demonstrated by this phase.
- **Defer RunLimit/no-alternative End semantics** to Phase 2B.3. `ItemsExhausted`
  remains the existing fallback and is not redefined here.

Rejected alternatives:

- Adding a separate WASM test binary: unnecessary duplication of
  `plan-hash` and `just wasm-hash`.
- Relying only on the existing hash trajectory: cannot establish Rule 3
  coverage.
- Adding a public `short_of_time` field to `Plan` solely for tests: would
  change production state/serialization and was not necessary.
- Changing the core planner to make the test fixture pass: no defect was
  demonstrated.

## 7.9 Outstanding issues and next steps

### Verified

- Rule 3 activates through real `Planner::adapt` in both scenarios.
- The selected prerequisite is `k4`.
- `Adapted.rule` and `PlanIssuedEvent.trigger` are asserted.
- Scenario A emits one `Phase::Prerequisite`; Scenario B emits none.
- Scenario A performs a real `next_item` + `adapt` continuation after the
  prerequisite and then has no pending prerequisite step.
- Scenario B preserves Final scheduling.
- Native and WASM produce identical complete scenario output and hashes.

### Partially verified

- Scenario A’s continuation is planner-level only; the synthetic answer is
  not folded through the learner-model event path.
- Scenario B’s `short_of_time`/`main_short`: inferred from source and fixture,
  not directly exposed by the public result.
- The exact `main_left` numeric value is not printed; the budget rejection is
  established by the 1,000-second estimate and ten-minute budget.

### Known limitations

- Synthetic plan/state and prior error, not a real learner trajectory.
- Matching output covers only the fixed fixture.
- No RunLimit/no-unseen rejection in scenario B.
- No `SessionReport.rules` use; direct `Adapted` fields are stronger for this
  scenario.

### Deferred to Phase 2B.3

- RunLimit rejection with no alternative step. Resolved in
  `docs/engineering/phase-2b-3-runlimit-termination.md`.
- End semantics for that case. Resolved in the same Phase 2B.3 record.
- Any decision on `deferred_today` affecting Rule 3.

## 7.10 Review outcome

The independent source review confirms the implementation described above
and finds no core production defect in the Rule 3 paths exercised by the
fixtures.

The proper acceptance status is **accepted with conditions**, not
unconditional:

1. Scenario A performs a real post-prerequisite `next_item` + `adapt`
   continuation, but the synthetic answer is not folded through the model
   event path.
2. Scenario B’s budget flags are inferred rather than directly exposed.
3. Native/WASM parity is established for these two fixed scenarios only.

No files were modified during the read-only review except the creation of
this engineering record. No commit or push was made. Phase 2B.3 was not
started.
