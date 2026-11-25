# K5 cohort benchmark v1

The cohort simulator of `docs/11-quality.md` (К5, «Симулятор с эталонами»), frozen on 2026-11-25 at commit `a2a40ac`. It is the configuration behind the F2 numbers: 200 learners × 30 daily sessions, 25-minute budget, seed 1, three arms (ours, only FSRS, only FSRS + our `next_item`).

v1 is never edited. A change to the simulator, its arms or its metrics goes into a v2 configuration next to this one; the v1 code path stays as it is.

## Reproduce

```sh
nix develop -c just bench-v1
```

It runs `cohort` (release) on the frozen bundle with the arguments in `args` and diffs its output against `expected.txt`; any difference fails. v1 is frozen by the local git tag `bench-v1` (commit `af51ab3`): the diff is identical at the tag, and on a later commit it differs by the measured effect of whatever planner, model or simulator change came since. The test `tools/sim/tests/bench_v1.rs` checks only the simulator's own determinism. The output is the same in debug and release builds.

`AUDIT.md` is the audit of this run (item H): the F2 gain over «only FSRS» comes mostly from how the baseline was built.

## What is frozen

| Part | Value |
| --- | --- |
| bundle | `content/bench/v1/bundle.json`, a build of `content/` at `a2a40ac` (content version `10c6a97473a363dff316b3a0c6ed22bcea301b33e6e49c14faf801774782f965`, SHA-256 of the file `4d98b0e472484d9aac9458c4f377694531013bd70d38e1607930a1b949a3c558`). It lives under `content/`, so `content/LICENSE` covers it |
| fixture | `refl_sim::live_fixture`: every KC live, text review ignored |
| arguments | `args`: learners 200, days 30, seed 1 (`cohort BUNDLE LEARNERS DAYS SEED`) |
| budget | `Budget::TwentyFive`, set in `tools/sim/src/bin/cohort.rs` |
| planner | `Params::default()`, version `session-prototype-1+71adc6ae28da63d6`; values in `planner-params.json` |
| model | `KnowledgeModel::default()`, version `knowledge-v1+036bb2a606efe0a3`; values in `model-params.json` |
| learners, arms, metrics | `tools/sim/src/cohort.rs` at `a2a40ac` |
| seeds | learner `i` takes the `2i`-th and `2i+1`-th outputs of `SplitMix64(1)` as its learner and track seeds; session start jitter is `SplitMix64(track_seed ^ 0x5EED)` |
| toolchain | Rust 1.100.0 (`rust-toolchain.toml`) |
| output | `expected.txt` |

## When the diff is not empty

- The simulator changed: restore v1 behaviour and put the change into v2.
- The planner or the model changed: the new output of v1 is the measured effect of that change. The planner and model versions above say which code `expected.txt` belongs to. A new reference result is a new versioned output that names its commit; v1 files are never edited (docs/11, «Версии симулятора»).
- The bundle format changed and the frozen bundle no longer reads: v1 cannot run on the new format; use the tag.
