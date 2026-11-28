# Simulator v2, fair remediation

The same cohort, seed and bundles as `../v2/`, with one change: the content's
micro-explanation after a wrong answer on a targeted L1 pattern is given to
**every** strategy, not only to S3.

Why: the micro-explanation is content. The client shows it after such an
answer whatever chose the item, so a comparison of planners must give it to
all of them. In `../v2/` only S3 received it, and the simulator lowered the
misconception's strength and added a practice gain for S3 alone. That built
most of S3's lead into the setup. `../v2/` is kept unchanged and reproduces
with `just v2` (`s3-only`); this report is `just v2-fair` (`all`).

Generated at `c220d3a-dirty` (the change in this directory), from the
`refl-deepseek-s1` content build. Reproduce: `just v2-fair 1000 1`.

## Results, live Articles sub-graph (5 target KCs)

| metric | horizon | S1 Leitner | S2 KC-adaptive | S3 refl |
|---|---|--:|--:|--:|
| target reach | 200 | **0.660** | 0.623 | 0.544 |
| target reach | 600 | **0.912** | 0.838 | 0.730 |
| level MAE (lower is better) | 200 | 0.516 | 0.581 | **0.290** |
| level MAE | 600 | 0.295 | 0.495 | **0.201** |
| misconception reduction | 200 | **1.000** | **1.000** | 0.946 |
| misconception reduction | 600 | **1.000** | **1.000** | 0.946 |

For comparison, `../v2/` (S3 only) reported S3 reach 0.544 vs 0.271 / 0.241
and misconception reduction 0.946 vs 0.435 / 0.445 at 200 steps.

## Reading

- With the remediation shared, S3 no longer leads on reach or on
  misconception reduction; the simple baselines reach more target KCs.
- S3 keeps a clear lead on **level MAE**: its model of the learner is the most
  accurate. That is the same picture as `../v1/AUDIT.md`.
- These are synthetic learners written by the same authors as the planner
  (docs/11: no parameters are fitted on them). The numbers say where to look
  in a real pilot, not that one planner is better.
