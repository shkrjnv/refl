# G: fluency by retrieval time, model v1.2 against v1.3

The K5 cohort of P1b and P1c (200 learners × 30 daily sessions, 25-minute budget, planner `session-prototype-4`) on the frozen bundle v3 (`content/bench/v3/bundle.json`), seeds 1 and 2. v1.2 is the committed output of `../p1c/bundle-v3-*` (commit `c7cc4a8`; rerun on `9a6a4ea` in a clean worktree, identical byte for byte apart from a `nix` warning line at the top of those files). v1.3 is `v13-p1c-seed{1,2}.txt` and `v13-p1b-seed{1,2}.txt` on commit `745f92a` plus the coverage sections of `p1c`.

**These numbers are not evidence of how good G is.** The simulator's answer times come from the item's expected time, the same number the βᶠ prior is made of, so its learners are never slow in the way fluency looks for. What the run does show is how often the counting rules let an answer count with this content and this planner.

## Reproduce

```sh
nix develop -c cargo run --locked --release --quiet -p refl-sim --bin p1c -- content/bench/v3/bundle.json 200 30 1 > tools/sim/bench/g/v13-p1c-seed1.txt
nix develop -c cargo run --locked --release --quiet -p refl-sim --bin p1b -- content/bench/v3/bundle.json 200 30 1 target/g-p1b-seed1 > tools/sim/bench/g/v13-p1b-seed1.txt
```

The same with seed 2.

## Results

Seed 1 / seed 2.

| | v1.2 | v1.3 |
| --- | --: | --: |
| mastered (learner, topic) pairs | 985 / 969 | 276 / 317 |
| topics mastered per learner (p1b, ever) | 4.92 / 4.84 | 1.38 / 1.58 |
| false mastery per answer (p1c) | 11.2% / 11.1% | 13.4% / 15.5% |
| false mastery at session end (p1b) | 10.4% / 10.1% | 12.3% / 14.5% |
| answers to mastery, median (p1b) | 58 / 54 | 43 / 44 |
| calendar days to mastery, median (p1b) | 4 / 4 | 3 / 3 |
| answers per learner in 30 days (p1b) | 455 / 460 | 1226 / 1157 |
| pairs where fluency (v1.2: τ) was among the last conditions | 0% / 0% | 1.8% / 0.6% |
| learners with a topic fluent only by branch 2 | – | 0 / 0 |

## Why mastery falls

Correct, non-rapid answers by the first rule that leaves them out of fluency (from the `p1c` outputs):

| rule | seed 1 | seed 2 |
| --- | --: | --: |
| counted | 2.2% | 2.4% |
| first after a change of instruction | 45.3% | 45.3% |
| item seen within 14 days | 50.5% | 50.4% |
| page hidden | 2.0% | 2.0% |

Of the live (learner, topic) pairs never mastered by day 30, 655 / 608 fail on fluency alone, with 3.5 counted answers on average against the 6 required. Fluency is rarely the *last* condition of a mastery because almost no topic gets there: the pairs that are mastered are the few that reached 6 counted answers early, so their medians (43–44 answers, 3 days) are a selected subset, not a speed-up. Every topic has 11–21 slots, a learner gives about 450 answers in a month (1,200 once topics stop being mastered and sessions fill their budget), and the planner changes the item kind, and so the instruction, between most consecutive answers. Under these two rules a topic gets a counted answer only when a learner meets one of its unseen items right after another item with the same instruction.

The second branch never decided a mastery. At mastery τ averages +0.63 to +0.78 (synthetic learners are faster than βᶠ, so branch 1 holds), bⱼ stays at 0.00 (few counted answers on known topics, shrunk with n₀ = 20) and the spread averages 0.50–0.57, above 0.35.

The numbers are the simulator's, not pilot results.

## G2: the two rules relaxed (docs/03, commit `a679b1a`; code `f313528`)

The first answer under an instruction is excluded once per session instead of after every change of instruction, and the item window is 3 days instead of 14. Outputs: `v13b-p1c-seed{1,2}.txt`, `v13b-p1b-seed{1,2}.txt` (the files `v13-*` above are the first rules). The simulator now gives each planned session an id; no inconsistent timing occurs in it. Again **not evidence of G's quality**, and no acceptance threshold is applied: Hara decides. Seed 1 / seed 2.

| | v1.2 | v1.3, first rules | v1.3, G2 rules |
| --- | --: | --: | --: |
| correct non-rapid answers counted | – | 2.2% / 2.4% | 13.4% / 14.8% |
| left out: first under its instruction in the session | – | 45.3% / 45.3% (after a change) | 12.1% / 12.3% |
| left out: item answered within the window | – | 50.5% / 50.4% (14 days) | 72.5% / 70.9% (3 days) |
| left out: page hidden | – | 2.0% / 2.0% | 2.0% / 2.0% |
| mastered (learner, topic) pairs | 985 / 969 | 276 / 317 | 887 / 906 |
| topics mastered per learner, ever | 4.92 / 4.84 | 1.38 / 1.58 | 4.43 / 4.53 |
| false mastery per answer (p1c) | 11.2% / 11.1% | 13.4% / 15.5% | 10.4% / 11.5% |
| false mastery at session end (p1b) | 10.4% / 10.1% | 12.3% / 14.5% | 9.1% / 10.7% |
| answers to mastery, median | 58 / 54 | 43 / 44 | 55 / 55 |
| calendar days to mastery, median | 4 / 4 | 3 / 3 | 4 / 4 |
| fluency among the last conditions | 0% / 0% (τ) | 1.8% / 0.6% | 5.3% / 3.1% |
| learners with a topic fluent only by branch 2 | – | 0 / 0 | 1 / 0 |

The item window is now the main exclusion: a session repeats a KC's few items within days. The reasons are listed in the order the rules apply, so a later rule's share is read after the earlier ones.
