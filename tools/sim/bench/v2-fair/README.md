# Simulator v2, fair remediation, with the product planner

The same cohort, seed and bundles as `../v2/`, with two changes:

1. The content's micro-explanation after a wrong answer on a targeted L1
   pattern is given to **every** strategy, not only to S3. It is content: the
   client shows it whatever chose the item, so a comparison of planners must
   give it to all of them. In `../v2/` only S3 received it, and that built
   most of S3's lead into the setup. `../v2/` is unchanged and reproduces with
   `just v2` (`s3-only`).
2. **S4 is the product planner itself**: `refl-core`'s `build_session`,
   `next_item` and `adapt`, one 25-minute session after another, lessons
   folded as `explanation_viewed`. S3 ("full refl") is a scoring heuristic
   written inside the simulator, not the code the app runs.

S4 runs on the live Articles graph only. The full 32-KC fixture promotes
draft KCs with two items each; the planner's block needs four steps with an
input step (docs/04, «Ввод в каждой отрабатываемой KC»), so every session
there ends at once. The live gate requires ten items per KC, so the product
never meets such a KC.

Reproduce: `just v2-fair 1000 1`. Generated from the `refl-deepseek-s1`
content build.

## Results, live Articles (5 target KCs, 1000 learners)

| metric | steps | S1 Leitner | S2 KC-adaptive | S3 sim heuristic | **S4 product** |
|---|---|--:|--:|--:|--:|
| target reach | 200 | **0.660** | 0.623 | 0.544 | **0.659** |
| target reach | 600 | 0.912 | 0.838 | 0.730 | **0.963** |
| level MAE ↓ | 200 | 0.516 | 0.581 | **0.290** | 0.694 |
| level MAE ↓ | 600 | 0.295 | 0.495 | **0.201** | 0.287 |
| misconception reduction | 200 / 600 | **1.000** | **1.000** | 0.946 | 0.954 |

95% intervals for S4 reach: 0.636–0.681 (200), 0.951–0.975 (600); for S1:
0.629–0.691 (200), 0.891–0.930 (600).

## Reading

- The product planner is level with Leitner over the first 200 answers and
  ahead of every baseline over 600 (~30 days); the intervals at 600 do not
  overlap.
- Its weak spots: a slow start on `a-an-sound` (33% reached by step 200 vs
  67% for S1; median step 168), a level estimate that lags early (MAE 0.69 at
  200, on par by 600), and misconceptions not always fully extinguished
  (0.954 vs 1.000).
- S3, the simulator's own heuristic, is the weakest at reach. The old v2
  headline was a statement about that heuristic plus the one-sided
  remediation, not about the product.
- Synthetic learners written by the same authors (docs/11: nothing is fitted
  on them): a pointer for the pilot, not a verdict.
