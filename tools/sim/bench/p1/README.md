# P1: practice volume

Measurement of the P1 change of the session builder (docs/04, «Сборка сессии в прототипе»): our sessions before and after it, and the «только FSRS» baseline, on the same learners. It is a new versioned run next to benchmark v1; v1 is untouched (`just bench-v1`).

## Reproduce

```sh
nix develop -c just p1 1     # seed 1; the same with 2
```

The `p1` binary (`tools/sim/src/bin/p1.rs`) uses only the public cohort API, so the same file builds at an older commit. The «before» run is that binary at the commit before the P1 planner change, in a temporary git worktree (`git worktree add --detach <dir> <commit>`; run `just p1` there).

## What is fixed

| Part | Value |
| --- | --- |
| bundle | `content/bench/v2/bundle.json`, a build of `content/` at `0641540` (content version `b6ecf22add768418ebb498b910122fad0c4871f2be96a93ba1b428c3634a745e`). It has the edge «a / an requires first mention» (commit `cd422c7`), the «H2 edge» of `tools/sim/bench/v1/AUDIT.md`, so the baseline no longer stalls on its first topic |
| cohort | 200 learners × 30 daily sessions, 25-minute budget, seeds 1 and 2 (`SplitMix64(seed)` as in benchmark v1) |
| model | `KnowledgeModel::default()`: `knowledge-v1.2+…` |
| arms | ours (`session_ours`) and «только FSRS» (`session_fsrs` without our `next_item`) |

## Metrics

- **False mastery**: a topic the model counted mastered whose true correct rate 7 days later, without reviews, is below 0.7. The true rate is the learner's answer formula without rapid guesses, fatigue and learning, averaged over the topic's items, for the learner as they were at the end of the session that mastered the topic (the session, not the answer, is the resolution of this binary; `m1` has the per-answer version).
- **Calendar days to mastery**: from the day of the topic's first answer to the day of the session that mastered it.
- **Known gain per active hour**: truly known topics (true recall ≥ 0.8) at the end minus at the start, per learner, divided by the mean active hours per learner (session time).
- Session minutes and answers are per session; end reasons are shares of sessions (docs/04, «Конец сессии»).

The numbers are the simulator's, not pilot results. The simulator's learners learn 0.25–0.6 logits per answer by construction, and the baseline's weakness is how it was built (`AUDIT.md`), so no advantage of either side is claimed until simulator v2.
