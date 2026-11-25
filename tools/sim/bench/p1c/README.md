# P1c: the second topic, and where false mastery comes from

Two measurements on the cohort of P1 and P1b (200 learners × 30 daily sessions, 25-minute budget, seeds 1 and 2), model v1.2, planner `session-prototype-4`. Nothing here changes the model.

## Reproduce

```sh
nix develop -c just p1b 1     # part A, seed 1; the same with 2
nix develop -c just p1c 1     # part B, both bundles, seed 1; the same with 2
```

## Part A: rule 6 picks from every topic

Since P1c the second learning topic of a session comes from all topics at the moment of the replan (docs/04, rule 6), not only from those ready at the start of the session. Outputs: `proto4-seed{1,2}.txt`, per-learner tables `proto4-seed{1,2}-{ours,fsrs}.tsv`. Seed 1 / seed 2; P1b is part 2 of `../p1b/`.

| | P1b (`session-prototype-3`) | P1c (`session-prototype-4`) | only FSRS |
| --- | --: | --: | --: |
| answers per session, days 1–5: median (mean) | 15 (17.6) / 15 (17.5) | 15 (20.8) / 15 (20.6) | 15 (14.7) / 15 (14.6) |
| minutes per session, days 1–5: median (mean) | 3.7 (4.4) / 3.7 (4.4) | 4.8 (5.2) / 4.9 (5.1) | 3.0 (3.3) / 3.0 (3.3) |
| end reasons, days 1–5 | items_exhausted 80% / 78%, kc_cap 12% / 12%, kc_deferred 4% / 3%, reviews_done 3% / 5%, kc_mastered 1% / 2% | items_exhausted 88% / 88%, kc_cap 4% / 4%, reviews_done 3% / 5%, kc_deferred 2% / 2%, kc_mastered 2% / 1% | items_exhausted 100% |
| end reasons, days 6–30 | items_exhausted 45% / 43%, nothing_due 23% / 25%, kc_mastered 16% / 16%, reviews_done 12% / 11%, kc_cap 4% / 4% | items_exhausted 40% / 39%, nothing_due 28% / 28%, kc_mastered 17% / 17%, reviews_done 12% / 13%, kc_cap 3% / 3% | items_exhausted 95% / 94% |
| topics mastered per learner (ever) | 4.68 / 4.79 | 4.87 / 4.83 | 3.56 / 3.73 |
| answers to mastery, median | 60 / 58 | 57 / 56 | 66 / 64 |
| calendar days to mastery, median | 5 / 4 | 5 / 4 | 5 / 4 |
| false mastery (session end) | 13.3% / 12.8% | **14.5% / 13.8%** | 14.5% / 13.3% |
| truly known topics at the end | 4.34 / 4.20 | 4.18 / 4.14 | 3.51 / 3.58 |
| known gain per active hour | 2.68 / 2.62 | 2.57 / 2.62 | 2.73 / 2.76 |

More early practice (about 3 more answers and 1 more minute a session in days 1–5) and fewer sessions capped. False mastery is higher still and over the 10% cap. The «only FSRS» arm has no mastery detector of its own (our model labels its logs).

## Part B: false mastery broken down

`p1c` (`tools/sim/src/bin/p1c.rs`) runs our sessions with the log recorded, folds each learner's log again through the core model (checked equal to the session's state), and at each event attributes every topic's change of μ to its source by comparing the state before and after it. False mastery here is per answer, as in m1: the learner right after the answer that mastered the topic, 7 days later without reviews, has a true correct rate below 0.7 on the topic's tasks (secondary topics and wrong ideas included). It is higher than the session-end figure of part A, which judges the learner at the end of the session.

Cells: bundle v2 (`content/bench/v2/bundle.json`) and bundle v1, the frozen file `content/bench/v1/bundle.json` run by the current code (no worktree). Outputs: `bundle-v{1,2}-seed{1,2}.txt`.

### Per topic

Seed 1 / seed 2. The last two columns split the false pairs by the learner's own skill on the topic 7 days later: below 1.5 (the topic itself was forgotten) or not (its tasks fail on something else).

| topic | v2: false / mastered | v2: forgotten / other | v1: false / mastered | v1: forgotten / other |
| --- | --: | --: | --: | --: |
| countability | 5.5% / 4.5% | 11 / 0, 9 / 0 | 4.5% / 4.0% | 9 / 0, 8 / 0 |
| first mention | **35.4% / 32.1%** | 16 / **54**, 25 / **37** | 26.5% / 25.6% | **46** / 7, **47** / 4 |
| a / an by sound | **26.7% / 31.9%** | **51** / 1, **60** / 1 | 21.6% / 25.4% | 17 / **25**, 10 / **39** |
| second mention | 5.8% / 4.3% | 3 / 8, 4 / 4 | 6.6% / 7.1% | 5 / 8, 8 / 6 |
| zero article (general) | 7.3% / 2.6% | 2 / 12, 2 / 3 | 5.2% / 6.7% | 1 / 9, 4 / 9 |
| all | 16.2% / 15.0% | 83 / 75, 100 / 45 | 12.9% / 13.7% | 78 / 49, 77 / 58 |

First mention and a / an carry 74–85% of all false pairs. Each is secondary in the other's tasks: all 16 a / an slots have first mention as secondary, and 5 of the 12 first-mention slots have a / an.

### Evidence at mastery

All topics, bundle v2, seed 1 (the other cells are in the outputs and look the same). Means over pairs; μ = μ₀ + the sum of increments by source.

| | pairs | μ₀ | own primary answers | secondary credit | propagation | μ | μ − 0.84σ | own answers | own correct inputs |
| --- | --: | --: | --: | --: | --: | --: | --: | --: | --: |
| false | 158 | −1.31 | +3.43 | +0.07 | +0.15 | 2.34 | 1.55 | 51.1 | 19.9 |
| true | 816 | −1.46 | +3.64 | +0.06 | +0.05 | 2.29 | 1.54 | 69.0 | 30.3 |

Misconceptions have no path to μ in model v1; no other source changed μ. On bundle v1, secondary credit is larger (+0.36 false, +0.31 true): there a / an and first mention are learned side by side.

| | true skill at mastery | skill +7 d | weakest secondary +7 d | true rate +7 d | model S, days | model R +7 d | days practised | span, days |
| --- | --: | --: | --: | --: | --: | --: | --: | --: |
| false | 3.89 | 1.83 | 0.35 | 0.62 | 18.5 | 0.91 | 5.6 | 4.7 |
| true | 4.44 | 3.24 | 2.35 | 0.86 | 20.3 | 0.93 | 6.9 | 6.1 |
| false, a / an only | 3.36 | 0.09 | 3.41 | 0.60 | 10.7 | 0.89 | 2.8 | 1.8 |
| false, first mention only | 4.21 | 3.02 | −1.75 | 0.63 | 25.8 | 0.91 | 7.8 | 7.0 |

**The last condition to become true** was confidence in every mastered pair of every cell: τ and production were already met.

### What the data say

- **μ is not wrong at the moment of mastery.** False and true pairs reach the bar the same way (μ ≈ 2.3, μ − 0.84σ ≈ 1.55, almost all of it from their own primary answers), and the learner's true skill is above the bar in both (by 2.3 and 2.9 logits). The model's lag of M1 is still there, and it is on the safe side.
- **Mechanism 1, forgetting** (53–69% of false pairs): a topic learned within 1–2 days on 2–3 days of practice (a / an on v2, first mention on v1, countability) falls from skill ≈ 3.4 to ≈ 0 within a week, while the model's FSRS card (default weights, S ≈ 10 days) predicts R ≈ 0.9 at +7 days. Nothing in the criterion looks past the moment of mastery, and the model's memory does not see this learner forgetting faster than its default weights. How realistic the simulator's forgetting is (−3.3 logits a week) is a question for simulator v2.
- **Mechanism 2, an unlearned secondary topic** (31–47%): the learner still knows the topic, but its tasks also need a secondary topic they do not know. On v2, a / an requires first mention, so first mention is mastered first while 5 of its 12 slots need a / an (weakest secondary skill −1.75 at +7 days). On v1, the same happens the other way round: a / an is mastered while first mention, secondary in all its slots, is weak. This is why the H2 edge moved false mastery from a / an to first mention and raised the total by 1.3–3.3 points under the current planner. In part 1 of P1b the gap was about 7 points, with the old planner.

### Structural options (for Hara to decide)

1. **Secondary topics only from upstream.** A content-build check: a slot's secondary topic is a prerequisite of its primary topic (in the `requires` closure) or unrelated to it, never one that requires it. On v2 it flags the 5 first-mention slots with a / an as secondary; the content fix is to make a / an primary in those slots too, or to drop the a / an contrast from them. Spec: docs/02 «Проверки сборки» gets the check. The model, the planner and the false-mastery definition (docs/03) stay as they are. It addresses mechanism 2 on v2 (54 / 37 pairs, about 4–5 points), not mechanism 1. The v1 direction (a / an with first mention secondary) is allowed, since first mention is upstream there.
2. **Mastery tested after forgetting had a chance.** A fourth condition in docs/03 «Когда тема считается освоенной»: a correct production answer on the topic given while its FSRS recall was below r\* = 0.9, that is, a passed due review (r\* is the existing review threshold; no new number). It addresses mechanism 1, with a cost: with the model's S of 10–25 days at mastery, mastery moves to the first due review, about 1–3 weeks later, which the 30-day cohort will show as fewer topics mastered. It also relies on the model's FSRS memory, whose weights are defaults until they are fitted on pilot logs (docs/03).

The numbers are the simulator's, not pilot results.
