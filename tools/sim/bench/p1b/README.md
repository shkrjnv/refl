# P1b: why false mastery rose, and practice without a partner

Measurements for P1b, next to P1 ([`../p1/README.md`](../p1/README.md)), on the same frozen bundle `content/bench/v2/bundle.json` and cohort (200 learners × 30 daily sessions, 25-minute budget, seeds 1 and 2).

## Reproduce

```sh
nix develop -c just p1b 1     # seed 1; the same with 2
```

The `p1b` binary (`tools/sim/src/bin/p1b.rs`) is `p1` plus three things: the share of masteries reached on the UTC day of the topic's first answer, false mastery inside and outside that group, and a per-learner table (`p1b BUNDLE 200 30 SEED PREFIX` writes `PREFIX-ours.tsv` and `PREFIX-fsrs.tsv`; `p1b --paired A.tsv B.tsv` prints per-learner differences B − A). Like `p1`, it uses only the public cohort API, so the same file builds at older commits. Metrics are p1's (false mastery, calendar days, known gain per active hour: see p1's README).

The m1 column is the per-answer on-policy false mastery of `m1` (`tools/sim/src/bin/m1.rs` of the cell's commit, run on bundle v2): the row `v1.1 (q = 0.02)` of «On-policy», which is the model the commit is built with (the label dates from M2). It scores the learner at the answer that mastered the topic, not at the end of the session.

## Part 1: attribution

Each cell ran in a temporary worktree (`git worktree add --detach`), with `p1b.rs` copied in; the worktrees are removed.

| cell | model | planner | commit |
| --- | --- | --- | --- |
| `v11p1` | v1.1 | session-prototype-1 | `a4dc3a5` |
| `v12p1` | v1.2 | session-prototype-1 | `7273f77` |
| `v11p2` | v1.1 | session-prototype-2 | `a4dc3a5` + `b33f355` cherry-picked, not committed |
| `v12p2` | v1.2 | session-prototype-2 | `b33f355` |

The cherry-pick conflicted only in `README.md`, `tools/sim/m1/output.txt`, `tools/sim/m2/output-seed2.txt` and `tools/sim/bench/p1/README.md` (absent at `a4dc3a5`); they were resolved to the `a4dc3a5` side. After it, `core/src/plan/`, `core/src/event.rs` and `tools/sim/src/cohort.rs` are byte-identical to `b33f355`, and `core/src/model/` to `a4dc3a5`.

Ours, seed 1 / seed 2 (outputs: `attribution/<cell>-seed{1,2}.txt`, `attribution/<cell>-m1-seed{1,2}.txt`):

| | v1.1 × proto-1 | v1.2 × proto-1 | v1.1 × proto-2 | v1.2 × proto-2 |
| --- | --: | --: | --: | --: |
| answers per session, days 1–5: median (mean) | 7 (7.4) / 7 (7.3) | 7 (7.4) / 7 (7.3) | 7 (8.9) / 7 (8.5) | 7 (8.9) / 7 (8.5) |
| minutes per session, days 1–5: median (mean) | 1.7 (2.0) / 1.7 (2.0) | 1.7 (2.0) / 1.7 (2.0) | 1.8 (2.4) / 1.8 (2.3) | 1.8 (2.4) / 1.7 (2.3) |
| answers per session, days 6–30: median (mean) | 7 (11.6) / 7 (11.7) | 7 (11.6) / 7 (11.9) | 9 (15.4) / 8 (14.7) | 8 (15.2) / 8 (14.8) |
| minutes per session, days 6–30: median (mean) | 1.6 (2.4) / 1.6 (2.5) | 1.6 (2.4) / 1.7 (2.5) | 1.8 (3.1) / 1.5 (3.0) | 1.7 (3.0) / 1.6 (3.0) |
| main end reason, days 1–5 | block_closed 93% / 91% | block_closed 93% / 91% | no_partner 92% / 90% | no_partner 92% / 90% |
| topics mastered per learner (ever) | 2.72 / 2.78 | 2.71 / 2.80 | 4.46 / 4.45 | 4.50 / 4.41 |
| answers to mastery, median | 55 / 52 | 55 / 52 | 56 / 55 | 56 / 55 |
| calendar days to mastery, median | 12 / 11 | 12 / 11 | 6 / 5 | 6 / 5 |
| false mastery (session end) | 9.0% / 7.6% | 10.0% / 7.5% | 10.5% / 9.8% | 11.3% / 10.1% |
| false mastery, m1 per answer on-policy | 10.3% / 8.3% | 10.9% / 8.4% | 12.5% / 11.2% | 13.0% / 11.7% |
| mastered on the UTC day of the first answer | 0 of 544 / 0 of 556 | 0 of 541 / 0 of 560 | 2 of 893 / 2 of 890 | 3 of 899 / 2 of 881 |
| false mastery: that day / later days | – / – | – / – | 1 of 2, 10.4% / 2 of 2, 9.6% | 1 of 3, 11.3% / 2 of 2, 9.9% |
| known gain per active hour | 3.37 / 3.30 | 3.35 / 3.29 | 2.91 / 2.95 | 2.91 / 2.91 |

Control on the old bundle (`content/bench/v1/bundle.json`, cell v1.1 × proto-1, outputs `attribution/v11p1-bundle-v1-*`): false mastery 2.1% / 2.9% at session end, 3.3% / 3.8% per answer in m1 (these are M2's numbers), 3.73 / 3.45 topics mastered per learner.

- **The rise from M2 (3.3–3.8%) to about 10% is the bundle.** The same code, v1.1 with the planner before P1, gives 3.3% / 3.8% on bundle v1 and 10.3% / 8.3% on bundle v2 (m1, per answer). The content of the two bundles differs in one edge of `content/articles/graph.ncl`: «a / an requires first mention» (`cd422c7`) in place of «first mention supports a / an» (`0641540`). Which topics carry the false masteries, and how this edge raises them, was not measured.
- **Model v1.1 → v1.2** moves false mastery by −0.1 to +1.0 points at session end and +0.1 to +0.6 in m1: within seed noise (the two seeds of one cell differ by 0.7–2.5 points).
- **Planner prototype-1 → prototype-2 (P1)** adds 1.3–2.6 points at session end and 2.1–3.3 in m1, with both models, while mastering about 1.7 more topics per learner.
- **Masteries in one sitting are almost absent:** at most 3 of about 900 pairs are mastered on the UTC day of the first answer; the median is 5–12 calendar days. False mastery does not come from same-day mastery in this cohort.

The numbers are the simulator's, not pilot results.
