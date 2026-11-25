# M1: why the knowledge model lags

Measurement only. Neither `core/` nor `docs/` changed. Labels: **[code]** read in the source, **[run]** measured by `m1`, **[docs]** the spec, **[hyp]** not verified.

Reproduce: `nix develop -c just m1` (about 1 minute) writes `output.txt` next to this file. Every number below is from it.

## Method

- **Data** [run]: the bench v1 cohort (frozen `content/bench/v1/bundle.json`, 200 learners × 30 days, seed 1, our sessions), with the log recorded: 61,313 answers and 248 mastered (learner, KC) pairs, as in audit H. Recording is the opt-in `Track::log` (`tools/sim/src/cohort.rs:445`, `record:507`). It changes nothing else: `just bench-v1` is still identical.
- **Replica** [code + run]: `tools/sim/src/bin/m1.rs:replay:493` replays each log through a copy of the knowledge part of model v1 (`core/src/model/v1.rs:touch:335`, `observe:368`, `answer:496`, propagation `:677-700`), changing one thing at a time. Memory, automaticity and production come from the core fold of the same log, run in lockstep. With no change, the copy equals the core fold on every answer, bit for bit: μ, σ², prediction, rapid flag and mastery (asserted in `replay`).
- **On-policy runs** [run]: changes that `KnowledgeModel::Params` can express also run with the planner reading them. The two planner-side options run through `m1.rs:session:118`, a copy of `session_ours` that only swaps the chosen item. Without a swap it equals `session_ours` on every event (asserted in `main`).
- **Ground truth** [code]: `Learner::p_correct` (`cohort.rs:251`) is the learner's answer formula without rapid guesses, fatigue or learning: wrong idea and secondary KCs included. The true correct rate of a KC is its mean over the KC's primary slots. The learner has no item memory, so every item counts as held out.
- **False mastery (FM)**: a mastered pair whose true correct rate 7 days after mastery, without reviews, is below 0.7.
- **Paired median**: answers to mastery over the 248 pairs v1 mastered; a pair the variant never masters counts as ∞.
- **Caveats**:
  - Replay is off-policy: the logs come from v1's planner. A variant slower than v1 is censored, because the planner stopped teaching the KC once v1 mastered it.
  - The synthetic learner gains 0.25–0.6 logits per answer (`cohort.rs:practise:199-201`), so the size of the lag is an assumption of the generator [hyp].
  - docs/11 forbids fitting parameters on synthetic data. The values below show direction only; they are not proposed values.

## 1. Static θ

- **[code]** There is no process noise per answer and no learning term. The update is one Newton step on a static θ (`v1.rs:observe:368`, `math.rs:posterior:92`). σ² grows only with time, by η·Δt (`v1.rs:touch:335`, `belief:253`), and η = 0.005 per day. docs/03:110 states it: "Само μ от времени не падает, растёт только неопределённость" ("μ itself does not fall with time; only the uncertainty grows") [docs]. So an error made when the learner knew nothing weighs as much as a success made today.
- **[run]** The learner's true rate on a KC's items first reaches 0.8 after a median of 12 answers (643 pairs). At that point the model's μ is −0.71 and σ is 1.03, so M = 0.01, while the learner's θ on the model's scale is 1.30. **μ lags the learner by about 2 logits.** At mastery: μ 2.01, σ 0.57, true rate 0.930 (0.919 a week later).
- **[run]** Process noise q, replayed on the same logs:

| q | per answer: paired median / FM / log loss | per day (η): paired median / FM / log loss |
| --- | --- | --- |
| 0 (v1) | 54 / 3.2% / 0.4952 | 54 / 3.2% / 0.4952 |
| 0.02 | 46 / 5.1% / 0.4721 | 53 / 2.5% / 0.4890 |
| 0.05 | 41 / 8.7% / 0.4630 | 50 / 4.3% / 0.4819 |
| 0.1 | 36 / 14.3% / 0.4572 | 49 / 5.4% / 0.4753 |
| 0.2 | 31 / 23.8% / 0.4531 | 45 / 6.5% / 0.4684 |
| 0.3 | 28 / 31.2% / 0.4516 | 43 / 7.7% / 0.4646 |

**How much lag comes from weighting old errors like new successes:** at FM ≤ 10%, process noise saves 13 answers of 54 (24%; q = 0.05 per answer). Log loss falls in both families, and keeps falling up to q = 0.3: the static-θ assumption fits this learner worse than a drifting θ [run]. The FM constraint, not log loss, is what caps q. Since the optimum depends on how fast the learner learns, q must come from real logs [hyp].

## 2. Information per answer

- **[docs, code]** I = p′²/(p(1 − p)), p′ = (1 − g − s)·a_eff·P*(1 − P*)·Π P*_j (`math.rs:evidence:61-88`), with a_eff = a/√(1 + πa²σ²/8) (`math.rs:24`).
- **[run]** At p = 0.8 and s = 0.05, I is 0.043–0.077 for g = 1/3 and 0.053–0.095 for typed input (σ from 1.5 to 0.35). Your formula gives 0.1225 for g = 1/3: it is the 3PL information with s = 0 and a = 1. The slip forces P* higher, where P*(1 − P*) is smaller, which costs about a third; a_eff costs another 5–47% (I ∝ a_eff², σ from 0.35 to 1.5).
- **[run]** Measured w·I per direct answer on a primary KC before mastery: 0.055 overall, 0.072 typed, 0.042 choice.
- **[run]** From σ₀ = 1.5 down to the measured σ at mastery (0.57) at Ī = 0.055 takes (1/0.57² − 1/1.5²)/0.055 ≈ 48 answers. That is close to the measured 54.
- **Verdict on your estimate**: the count (~60) is about right, but both inputs are off. I is about half of 0.12, and the bound binds at σ ≈ 0.57, not 0.35. σ is not the root cause. The bound needs σ ≤ (μ − 1.5)/0.84, and μ is held down by static θ (section 1). If μ tracked the learner (θ ≈ 3 by mastery), σ* ≈ 1.8 and the bound would hold almost at once. Table "Answers to shrink σ" in `output.txt` [run].

## 3. Hypotheses, each isolated (replay)

| Hypothesis | Change | Paired median (v1 54) | FM (v1 3.2%) | Log loss (v1 0.4952) | Verdict |
| --- | --- | --: | --: | --: | --- |
| slope 1.0 vs the learner's 1.3 | a = 1.3 | ∞ (129 never) | 1.7% | 0.4967 | not a cause: a = 1.3 masters fewer pairs and predicts worse [run] |
| γ on unsure correct answers | γ = 0 | 53 (5 never) | 9.5% | 0.4718 | small effect on answers. The log-loss gain is an artifact: the learner says "unsure" when p_skill ≤ 0.7 (`cohort.rs:360`) and never guesses outside rapid answers. γ needs real data [run, code] |
| secondary failures blamed on the primary | secondaries left out | ∞ (159) | 3.4% | 0.4911 | refuted [run] |
| | oracle true P* for secondaries | ∞ (132) | 7.2% | 0.4743 | refuted: with the true secondary the primary masters *later*. The model's low belief in an unlearned secondary makes a correct answer more surprising, and that inflates the primary's credit (`math.rs:evidence`: score ∝ (y − p)) [run] |
| | w_secondary = 0 | 91 (104) | 3.8% | 0.5040 | slower [run] |
| the learner's own wrong idea | split | with idea 59 answers (27 pairs) vs without 53 (221) | 3.7% vs 3.2% | — | +6 answers where the idea exists; no FM difference [run] |
| typed vs choice | only typed answers update | 59 (35) | 3.4% | 0.5012 | choice answers help despite I = 0.042 vs 0.072 [run] |
| threshold 0.8 | 0.7 / 0.6 / 0.5 | 49 / 44 / 39 | 4.6 / 9.3 / 14.9% | unchanged | pure trade of FM against answers [run] |
| θ* = 1.5 | 1.2 / 1.0 | 45 / 39 | 6.0 / 8.4% | unchanged | same [run] |
| propagation shrinks prerequisite σ | ρ = 0 | 60 (36) | 1.4% | 0.4968 | propagation speeds up prerequisite mastery and adds 1.8 points of FM; it is not a lag cause [run] |
| double break penalty δ(1 − R) + ηΔt | δ = 0 / η = 0 / both | 56 / 56 / 57 | 3.3 / 3.2 / 3.6% | 0.4941 / 0.4983 / 0.4972 | refuted: no lag from it [run] |
| a vs a_eff in p′ | raw a in p′ (as docs/03:63 writes) | 63 (53) | 2.6% | 0.5053 | the code (a_eff, `math.rs:81`) beats the spec's formula; docs/03 and code disagree [code, docs, run] |

## 4. Calibration of M = P(θ_eff > θ*)

Truth: the learner's true rate on a medium typed item is ≥ (1 − s)·σ(θ*) = 0.777, which is what θ* = 1.5 stands for (docs/03, «Когда тема считается освоенной»). Computed after each non-rapid answer, on that answer's primary KCs; v1 [run].

| M bin | answers | mean M | true share above θ* | wrong idea included |
| --- | --: | --: | --: | --: |
| 0.0–0.1 | 34,022 | 0.023 | 0.735 | 0.688 |
| 0.1–0.2 | 7,149 | 0.145 | 0.955 | 0.898 |
| 0.2–0.3 | 4,287 | 0.247 | 0.965 | 0.941 |
| 0.3–0.4 | 2,249 | 0.346 | 0.982 | 0.973 |
| 0.4–0.5 | 2,156 | 0.451 | 0.982 | 0.976 |
| 0.5–0.6 | 2,085 | 0.549 | 0.984 | 0.981 |
| 0.6–0.7 | 2,030 | 0.651 | 0.992 | 0.991 |
| 0.7–0.8 | 1,920 | 0.749 | 0.989 | 0.987 |
| 0.8–0.9 | 1,577 | 0.848 | 0.998 | 0.997 |
| 0.9–1.0 | 3,031 | 0.966 | 1.000 | 1.000 |

M is underconfident everywhere: where it says 2%, the learner is above θ* 74% of the time. That is the static-θ lag of section 1, seen as calibration [run].

## 5. Objective: fewest answers with FM ≤ 10%

Ranked by answers saved on the paired median (replay, the same data for every variant). On-policy runs are shown where the planner can read the change.

| # | Change | Kind | Saved (replay) | FM | Log loss | On-policy: mastered per learner (v1 1.24), own median answers (v1 54), FM |
| --: | --- | --- | --: | --: | --: | --- |
| 1 | θ* = 1.0 | threshold | 15 | 8.4% | = | — |
| 2 | q 0.02 per answer + θ* 1.2 | noise + threshold | 14 | 9.0% | 0.4721 | — |
| 3 | q 0.05 per answer (± raw a) | noise | 13 | 8.7% (8.2%) | 0.4630 (0.4647) | needs a model change |
| 4 | η 0.3 per day | noise | 11 | 7.7% | 0.4646 | — |
| 5 | η 0.05 + θ* 1.2 | noise + threshold | 11 | 6.6% | 0.4819 | 3.53, 48, 6.1% |
| 6 | confidence 0.6 | threshold | 10 | 9.3% | = | 1.97, 47, **10.9%** (fails) |
| 7 | η 0.2 per day | noise | 9 | 6.5% | 0.4684 | **3.87, 48, 4.7%** |
| 8 | θ* = 1.2 | threshold | 9 | 6.0% | = | 2.09, 47, 8.6% |
| 9 | η 0.05 + confidence 0.7 | noise + threshold | 9 | 7.4% | 0.4819 | 2.86, 49, 7.3% |
| 10 | q 0.02 per answer | noise | 8 | 5.1% | 0.4721 | needs a model change |
| 11 | confidence 0.7 | threshold | 5 | 4.6% | = | 1.62, 51, 4.6% |
| 12 | η 0.05 per day | noise | 4 | 4.3% | 0.4819 | 2.45, 54, 2.9% |
| 13 | γ = 0 | input | 1 | 9.5% | 0.4718 | 1.55, 50, 4.2% |
| P1 | planner: 1–2 hardest items while 0.5 ≤ M < 0.8 | planner | — | 3.0% | — | 1.34, 53, 3.0% (1,952 swaps): small |
| P2 | planner: typed items near mastery | planner | — | — | — | **0 swaps.** Already done: the ladder prefers guided production once M ≥ 0.3 (`plan/next.rs:90-95`), and reviews prefer typed (`next.rs:88`) [code, run] |

Excluded because FM > 10%: q ≥ 0.1 per answer, confidence 0.5, q 0.05 + γ 0, and combinations with q 0.1.

Threshold changes save answers only by moving the bar. They leave log loss and calibration untouched. Process noise is the only change that both saves answers and predicts better, which is the CLAUDE.md test. On-policy, η = 0.1–0.2 per day masters 2.5–3.1× as many KCs at FM ≤ 4.7%: noise lets μ follow the learner, and the planner then moves on. It also uses 82–88 minutes per learner against v1's 63 [run].

## 6. Proposed spec changes for docs/03

These are proposals only; nothing is applied. Each one needs the author's decision, then the regression on the simulator and the log-loss comparison on real logs.

1. **«Знание: вера о навыке» — a time update before every answer.** Add σₖ² ← σₖ² + q for every KC of the slot before the prediction and the Newton step, with q starting at 0.02. The skill changes with practice, so a static θ makes old errors weigh like new successes.
   - Measured: paired median 54 → 46, FM 3.2% → 5.1%, log loss 0.4952 → 0.4721.
   - q = 0.05: 54 → 41, FM 8.7%, log loss 0.4630.
   - q is fitted on pilot logs by «Общее правило для всех параметров», and accepted only if log loss improves on the last 20% by time and with each learner left out.
2. **«Память и забывание» — η is a fitted parameter, not just a break term.** Raise the starting value from 0.005 to 0.05 per day, or fit it together with q.
   - Replay: 54 → 50, FM 4.3%, log loss 0.4819.
   - On-policy: mastered per learner 1.24 → 2.45, FM 2.9%.
   - With q 0.02 per answer: 46, FM 5.7%, log loss 0.4689.
3. **Formula correction: a_eff in p′**, matching the code (`math.rs:evidence:81`). docs/03:63 writes a. With raw a, log loss is worse (0.5053 vs 0.4952) and mastery slower (63 vs 54). This is a spec fix, not a model change.
4. **Open question 2 (thresholds): record the measured trade-off; no change to θ* now.**
   - Lowering θ* or the confidence trades FM for answers: 0.7 saves 5 answers (FM 4.6%; on-policy 1.62 mastered per learner); θ* 1.2 saves 9 (FM 6.0%).
   - It leaves calibration as it is. Decide only after 1–2, which address the cause.
5. **γ stays 0.3 until real data.** γ = 0 improves log loss on the simulator only because the synthetic learner's "unsure" is not guessing (`cohort.rs:360`). It is no evidence about real learners.
6. **«Распространение по графу»: keep ρ = 0.3.** It shortens prerequisite mastery (ρ = 0: paired 60, 36 pairs never) at a cost of +1.8 points of FM. Watch it in the pilot's audit items (К4).
7. **The 15–25 answer target (docs/03:258)** is not reachable by v1 on this simulator. The best accepted combination reaches 40–46 answers (rows 2, 3, 10). Under the 10% FM cap, I expect the remaining gap to come from the information per answer (section 2) [hyp, not isolated].
8. **«Самопроверка» / docs/11 К1: add the reliability table of section 4 as a simulator check.** It needs a new item in docs/11; I note it here because docs/11 is the place.
