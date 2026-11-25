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

## Results

Outputs: `before-seed{1,2}.txt` (planner `session-prototype-1`, the binary at commit `c8fcc16` in a temporary worktree) and `after-seed{1,2}.txt` (`session-prototype-2`, P1). The «only FSRS» arm is byte-identical before and after. Seed 1 / seed 2:

| | ours before P1 | ours after P1 | only FSRS |
| --- | --: | --: | --: |
| minutes per session, days 1–5: median (mean) | 1.7 (2.0) / 1.7 (2.0) | 1.8 (2.4) / 1.7 (2.3) | 3.0 (3.3) / 3.0 (3.3) |
| minutes per session, days 6–30: median (mean) | 1.6 (2.4) / 1.7 (2.5) | 1.7 (3.0) / 1.6 (3.0) | 2.2 (2.4) / 2.2 (2.4) |
| answers per session, days 1–5: median (mean) | 7 (7.4) / 7 (7.3) | 7 (8.9) / 7 (8.5) | 15 (14.7) / 15 (14.6) |
| answers per session, days 6–30: median (mean) | 7 (11.6) / 7 (11.9) | 8 (15.2) / 8 (14.8) | 12 (12.1) / 12 (12.0) |
| main end reason, days 1–5 | block_closed 93% / 91% | no_partner 92% / 90% | items_exhausted 100% |
| main end reasons, days 6–30 | block_closed 91% / 91% | no_partner 73% / 71%, nothing_due 17% / 17% | items_exhausted 95% / 94% |
| topics mastered per learner (ever) | 2.71 / 2.80 | 4.50 / 4.41 | 3.56 / 3.73 |
| answers to mastery, median | 55 / 52 | 56 / 55 | 66 / 64 |
| calendar days to mastery, median | 12 / 11 | 6 / 5 | 5 / 4 |
| false mastery | 10.0% / 7.5% | 11.3% / 10.1% | 14.5% / 13.3% |
| known gain per active hour | 3.35 / 3.29 | 2.91 / 2.91 | 2.73 / 2.76 |

- **Why days 1–5 barely changed.** The bundle's `requires` graph has one root (countability). A fresh learner has Ready ≥ 0.5 only on it, so rule 6 finds no second KC and the session ends `no_partner` after the first block. The baseline ignores Ready and goes on through the next topic's items.
- **Where «only FSRS» is still ahead**: practice volume in days 1–5 (15 answers and 3.0 minutes per session against 7 and 1.8), median session length in days 6–30 (2.2 against 1.7 minutes; our mean is higher, 3.0 against 2.4, with a long tail), and calendar days to mastery (5 / 4 against 6 / 5).
- **What P1 cost**: false mastery rose above the 10% cap on seed 1 (11.3%) and to it on seed 2 (10.1%), and known gain per active hour fell from 3.3 to 2.9: the extra minutes go to topics already near mastery and to nothing_due.

The numbers are the simulator's, not pilot results. The simulator's learners learn 0.25–0.6 logits per answer by construction, and the baseline's weakness is how it was built (`AUDIT.md`), so no advantage of either side is claimed until simulator v2.
