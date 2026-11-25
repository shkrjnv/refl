# Item H: audit of the K5 cohort simulator

Date 2026-11-25, repo at `a2a40ac` (working tree clean before the audit). Only change made: benchmark v1 freeze (section 6).

Labels: **[code]** read in the source, **[run]** measured by running the code (the `cohort` binary or the read-only harness below), **[docs]** from the spec, **[hyp]** hypothesis, not verified.

Harness: `/tmp/claude-1000/-home-ilumix-development-projcts-refl/787c2dfa-be25-4cd0-887d-1c8bed1f3a8b/scratchpad/harness/src/main.rs`, a scratch crate outside the repo. It calls only the public API (`Learner::new`, `Track::new`, `session_ours`, `session_fsrs`, `Learner::recall`, model getters) with the same seed derivation as `cohort()`, so it reproduces the binary exactly (ours 1.24 / 3.15 / 0.8952 / 604 of 6000 frustrated; only FSRS 0.22 / 1.16 / 0.7881 / 4624; [run]). Variants only change the in-memory bundle. Raw outputs: `h-*.txt` next to it.

---

## 0. Headline

The F2 advantage of "ours" over "only FSRS" comes mostly from how the baseline is built, not from our planner:

1. The baseline orders topics by `requires` depth, then by id (`tools/sim/src/cohort.rs:session_fsrs:839-862`). `a-an-sound` has depth 0 and sorts first, so every baseline learner starts on it [code]. Every a-an item has `first-mention` as a secondary KC (`content/articles/items-a-an.ncl:5`) [code]. The synthetic learner needs every slot KC to answer (`Learner::answer:306-310`) but learns only the primary ones (`:371-379`) [code]. The baseline never teaches `first-mention`, so accuracy on a-an stays capped.
2. The baseline moves to the next topic only when *our model* sets `ever_mastered` (`session_fsrs:863-866`). It has no fallback, unlike ours, which defers a topic after 12 tasks (`plan/adapt.rs:fired:92-95`) [code].
3. Result: baseline learners touch 1.21 KCs in 30 days, and 182 of 200 never master a-an-sound [run].

Diagnostic: add one edge to the in-memory bundle (a-an-sound requires first-mention, which is what its items already encode), and the ranking flips. Only FSRS then masters 2.15 KCs per learner against ours 0.92 (ours − FSRS −1.23, 95% [−1.42, −1.05]). It also truly knows more: 2.76 vs 2.45 (−0.31 [−0.51, −0.11]). Frustration and retention differences mostly vanish (section 8) [run].

What survives in both configurations: ours uses fewer minutes and answers, and gains more truly known KCs per active minute and per answer [run]. Whether that holds at equal time is unknown: ours ends sessions after about 1.7 of the 25 minutes.

---

## 1. Path of the experiment

`just cohort` (`justfile`) → `content-build content --out target/bundle.json` → `cargo run --release -p refl-sim --bin cohort -- target/bundle.json 200 30 1` [code].

- `tools/sim/src/bin/cohort.rs:main:118-155`: reads the bundle, `live_fixture` (`tools/sim/src/lib.rs:402-408`: every KC live, `reindex(.., TextReview::Ignored)`), spec `{200, 30, seed 1, Budget::TwentyFive}` (`:143-148`), `KnowledgeModel::default()`, `Params::default()` (`:149-150`), then `cohort()` for each of the 3 policies and `print` [code].
- `cohort.rs:cohort:971-1074`: `seeds = SplitMix64(spec.seed)`; for learner `i`: `learner_seed`, `track_seed` = next two outputs (`:998-999`); `Learner::new(bundle, learner_seed)`, `Track::new(model, bundle, T0, track_seed)`, `jitter = SplitMix64(track_seed ^ 0x5EED)` (`:1000-1002`). Days 0..29: session start at 18:00 UTC plus 0–2 h jitter (`:1005-1009`); one session a day under the policy (`:1010-1018`). The learner seed is the same in every arm; the track seed (decision seeds) too [code].
- After day 30: metrics read at `end = track.now()`, i.e. right after the last session (`:1036-1071`) [code].
- `T0` = 2025-10-01 00:00 UTC (`lib.rs:50`) [code].
- Content: 5 KCs (4 A1, 1 A2), 68 items: a-an 16, countability 15, first-mention 12, second-mention 14, zero 11; expected times 10–20 s. A whole KC's bank is about 3–4 minutes [run, from `content-build report`].

## 2. The synthetic learner (`cohort.rs:Learner`)

### Generation (`Learner::new:104-168`) [code]

| Trait | Formula | Range |
| --- | --- | --- |
| offset | 0.7·N(0,1) | per learner |
| learn_rate | 0.25 + 0.35·U | 0.25–0.60 logits per answer |
| forget_rate | 0.5 + 1.5·U | 0.5–2.0 |
| slip | 0.02 + 0.08·U | 0.02–0.10 |
| rapid | 0.05·U·U | 0–0.05, mean 0.0125 |
| fatigue | 0.002 + 0.006·U | slip added per minute |
| pace | 0.7 + 0.8·U | latency and reading multiplier |
| base skill per KC | offset − 1.2 − 0.6·(level_rank − 1) + 0.8·N | A1 mean −1.2, A2 −1.8 (`:115-116`) |
| bug per KC | 30%: 0.15 + 0.25·U, else 0 | unlisted wrong idea (`:117-121`) |
| own prerequisites | each of our direct `requires` kept with p 0.7 (`:137-143`), plus a hidden edge to every KC earlier in (level_rank, id) order with p 0.1 (`:150-156`) | |

Traits are independent of each other: no correlation between, say, learning and forgetting.

### Mechanisms [code]

- **Knowledge**: `skill = base + gain·exp(−forget_rate·days/stability)` (`skill:179-186`).
- **Learning** (`practise:199-213`): `step = weight·learn_rate·readiness·(1 − σ(skill − 2.5))`; weight 1.0 if correct, 0.6 if wrong (`answer:372`), 0.8 for an explanation (`lesson:229`). It does not depend on item difficulty or kind. Only primary KCs learn (`:371`). `readiness` = 0.35 per own prerequisite with skill < 0 (`:189-195`).
- **Forgetting**: exponential, per-learner rate; stability starts at 1, ×1.6 on every touch at least 0.5 days after the last one, +0.1 for every massed touch (`:205-210`). It never decreases, and the gain does not depend on interval length.
- **Answer probability** (`answer:300-313`): with probability `bug` (max over primary KCs) the answer comes from the wrong idea and is wrong. Otherwise it is correct with `g + (1 − g − slip')·Π σ(1.3·(skill_k − b))`, where the product runs over every slot KC including secondaries, `b = difficulty − 3 + shift(id)` with shift ∈ [−0.5, 0.5] (FNV hash, `:82-90`), `g` = the item's guessing rate, and `slip' = min(slip + fatigue·minutes, 0.4)`.
- **Rapid guesses** (`:273-298`): probability `rapid·(1 + minutes/10)`; typed answer "?" at 800 ms; choice answer uniform at 0.1·expected time.
- **Latency** (`:314-325`): `expected·pace·exp(−0.2(θ_min − b))·LogN(0, 0.25)`, clamped to [0.3, 5]·expected; at least 2.5 s for typed input.
- **Confidence** (`:326-330`): Sure if from the bug or if p_skill > 0.7, else Unsure.
- **Errors** (`:331-370`): typed errors match one of our `expected_wrong` patterns at random 70% of the time (not from the bug), otherwise Unrecognized. A choice error from the bug takes the undiagnosed option if one exists. Other choice errors take a uniform distractor, which carries our misconception id. 3% of correct typed answers are typos.
- **Reading** (`reading_seconds:222-225`): `words·0.6·pace·LogN(0, 0.3)` s, which is 100 wpm at pace 1, clamped to [10, 1800].
- **Explanation** (`lesson:228-233`): `practise(weight 0.8)` and `bug *= 0.7`. The bug also shrinks ×0.85 on each wrong answer that did not come from it (`:373-378`).

### docs/11 K5 requirements: implemented? [code + docs]

| Requirement | Status | Where |
| --- | --- | --- |
| hidden prerequisites outside the graph | yes, but only toward KCs earlier in (level, id) order: here countability can get hidden prerequisites on article KCs, and a-an-sound can get none. They slow learning only, never answering | `:150-156`, `:189-195` |
| spurious edges (in our graph, not the learner's) | yes, 30% of our direct edges dropped | `:137-143` |
| exponential forgetting, per-learner rate | yes | `:108`, `:182` |
| misconceptions not in our list | yes (`bug`). Our listed misconceptions are never persistent learner states: errors map to them at random | `:117-121`, `:300-305`, `:344-346`, `:361` |
| guessing and slips | yes; guessing uses the item's own g (shared with the model) | `:311-313`, `:273-298` |
| fatigue late in the session | yes, but inert: sessions last 1.5–3 min, so the added slip is at most about 0.02 | `:312`, `:273` |
| typing speed (not in docs/11 K5) | no; only a 2.5 s floor for typed input | `:323-325` |
| reading speed (not in docs/11 K5) | yes, 100 wpm × pace, the same pace as answer latency | `:222-225` |
| device (not in docs/11 K5) | no | |
| practice speed-up (not in docs/11 K5) | only through skill in the latency formula; no practice law | `:318-321` |
| memory of the item (not in docs/11 K5) | no: a repeated item is as hard as a new one | |

## 3. What each arm receives [code, run]

| | ours (`session_ours:595-674`) | only FSRS (`session_fsrs:777-907`, `ours=false`) | FSRS + our `next_item` (`ours=true`) |
| --- | --- | --- | --- |
| explanation | short explanation, `Lang::Ru` chain (`plan/values.rs:lesson_explanation:291-298`), once per KC, at the first session where it is the learning KC (`plan/build.rs:333`) | the first short explanation with `is_shown()` in bundle order, any language (`cohort.rs:871-881`), once per KC, before the first item | same as only FSRS |
| explanation time charged | yes: clock waits the learner's reading time (`Track::lesson:463-488`); the planner budgets its own estimate (`build.rs:427-429`) | yes: clock, plus `used +=` reading seconds (`:880`) | same |
| worked examples | only as words in the reading time (`cohort.rs:471-478`); no separate effect | same | same |
| KCs explained per learner | 3.62 | 1.21 | 1.26 |
| item pool per session | unseen-in-session items of the learning KC, its contrasts (first↔second, zero↔second) and due reviews; block, then up to 3 interleaved, final of this session's errors (`build.rs:remaining:392-505`) | due reviews (R < 0.9, lowest R first, ≤ 35%), then every unseen-in-session item of one KC (`Baseline::eligible:687-712`); the same items every day | same pool; order by `next_item` |
| order of new KCs | V^learn = imp·(1−M)·Ready·(1+m)·(1+ζU), Ready ≥ 0.5 gate, micro-randomised within 95% (`build.rs:start:213-263`, `values.rs:learn_value:180-203`) | (requires depth, id): a-an-sound first; next only after our `ever_mastered` | same |
| prerequisite checks | after 2 errors in a row if suspicion > 0.5 (`adapt.rs:97-103`); fired 8 times in 6000 sessions | none | none |
| budget used | 25 min nominal; mean 2.1 min per session, 63.3 min per learner over 30 days | mean 2.4 min, 71.3 min | 2.4 min, 71.0 min |
| how sessions end | block_closed 94% | items_exhausted 100% | items_exhausted 100% |
| answers per learner | 307 | 474 | 474 |
| attempts | 1 per item; no hints, scaffolds or retries (`Track::answer:501-515`) | same | same |

**Do baselines get explanations before practice?** Yes, once per KC, before the first item on it (`cohort.rs:871-881`) [code]. But they reach only about 1.2 KCs, so they see about 1.2 explanations [run].

**Why the next_item arm changes almost nothing** [code + run]. The pool is the unseen-in-session items of one KC, 11–16 items. Every session uses all of them (items_exhausted 100%), so the choice only reorders the same set. The learner's gain per answer does not depend on item difficulty or kind (`practise`), and fatigue is negligible at 2 minutes. Order can only matter through correctness-dependent weights (1.0 / 0.6). The candidate pool is large enough at the start of a session, but choice cannot matter when the whole pool is consumed. The step also passes `production: false` (`Baseline::pick:723-729`), so the ladder prefers recognition while M < 0.3. Differences: mastered 0.28 vs 0.22, known 1.20 vs 1.16.

## 4. Independence of ground truth

Shared between the synthetic learner and our model or planner [code]:

1. **Difficulty**: learner `b = difficulty − 3 + shift(±0.5)` (`cohort.rs:265`); model `b = difficulty − 3` (`model/v1.rs:difficulty:207-209`). Same scale, ±0.5 noise.
2. **Guessing**: both `1/choices` (`cohort.rs:311`; `projection.rs:94-98`).
3. **Conjunctive slot**: both multiply over slot KCs (`cohort.rs:306-310`; model predict, docs/03 open question 1). The model's structural assumption is built into the truth.
4. **Expected time**: it drives the learner's latency (`cohort.rs:264, 318-322`) and the model's τ, tempo grade and rapid rule (`v1.rs:524-525, 646-651`), so errors in authored times cancel.
5. **Reading speed**: learner 100 wpm × pace (`cohort.rs:223`) equals the planner prior `reading_words_per_minute = 100` (`plan/mod.rs:108`, `prior_seconds:547-557`).
6. **Starting skill**: learner A1 mean −1.2 (`cohort.rs:116`) equals model μ0 = 1.2·(0 − 1) for L = 0 (`v1.rs:240-243`).
7. **Prerequisite graph**: 70% of the learner's edges are ours; hidden edges are id-ordered (`:137-156`).
8. **Rapid guesses**: learner rapid latencies (800 ms typed, 0.1·E choice) are always below the model's thresholds (< 0.25·E and < 2 s typed, `v1.rs:524-525`), and normal latencies are clamped to ≥ 0.3·E, ≥ 2.5 s typed. Detection is perfect by construction.
9. **Confidence**: derived from the true p_skill > 0.7 (`cohort.rs:326-330`). Perfect calibration feeds the model's unsure discount and the ConfidentError rule.
10. **Misconception list**: learner errors emit our misconception ids at random (`:346, :368`). The model's misconception tracking reacts to noise.
11. **Mastery**: baselines progress by our model's `ever_mastered` (`:863-866`); the "mastered" metric is the same predicate.
12. **Slip**: learner 0.02–0.10 vs model 0.05; typo 3% mapped to the model's typo rule.
13. **Scale of "truly known"**: recall ≥ 0.8 with slope 1.3 and slip about 0.06 means skill ≥ about 1.33, close to θ* = 1.5 ("about 78% correct on a medium typed item", docs/03).

Not shared: FSRS (the learner forgets exponentially), p* (the learner has no target), slope (1.3 vs 1.0), the frustration threshold (0.5 from docs/11, not p*).

**Does the learner learn from something only our arm provides?** Not by mechanism. Every arm learns per answer on primary KCs, and from an explanation with the hand-chosen effect `0.8·learn_rate` plus `bug ×0.7` (`lesson:228-233`). Interleaving, contrasts, the final block and prerequisite checks have no special effect beyond ordinary practice. The learner has no confusion state and no remedy effect [code]. The explanation effect is concentrated in ours only through coverage (3.62 vs 1.21 KCs explained). Without explanations (bundle with `explanations` cleared, harness variant `noexpl`) [run]:

| | mastered ours / FSRS | known ours / FSRS | ours − FSRS mastered | ours − FSRS known |
| --- | --- | --- | --- | --- |
| default | 1.24 / 0.22 | 3.15 / 1.16 | +1.02 [95% +0.80, +1.26] | +1.99 [+1.80, +2.19] |
| no explanations | 0.83 / 0.16 | 2.84 / 1.12 | +0.67 [+0.47, +0.89] | +1.72 [+1.54, +1.90] |

About a third of the mastered gap and 14% of the known gap ride on the hand-set explanation effect. Confound: without an explanation the model never starts an FSRS card from `lesson`-context answers (`v1.rs:631`). All baseline items are `lesson` context, so the baselines then get no reviews at all [code].

## 5. Metrics

Definitions [code] and data [run], seed 1, 200 learners.

| Metric | Exact definition | ours | FSRS | FSRS+NI | Optimised only by ours? |
| --- | --- | --- | --- | --- | --- |
| frustrated sessions | `SessionReport::answered:567-577`: sliding window of the last 8 answers (model-correct, i.e. correct or typo, rapid guesses included); the session counts if at any point the full window has < 4 correct. Sessions with < 8 answers can never count. Threshold 0.5 per docs/11, **not p* = 0.8** | 604/6000 = 10.1% | 4624/6000 = 77.1% | 4516/6000 = 75.3% | Partly. Ours targets p = 0.8 / 0.7 and ends sessions early: 46% of ours' sessions have < 8 answers, against 0% of the baselines'. Among sessions with ≥ 8 answers: 18.5% / 77.1% / 75.3% |
| mastered | KCs with our model's `ever_mastered` (`cohort.rs:1040-1047`): P(θ_eff > 1.5) ≥ 0.8, timed > 0, τ ≥ −0.2, ≥ 2 distinct production items correct (`v1.rs:is_mastered:419-430`, set at `:712-721`). Applied to the baselines, and it also gates their progress | 1.24 | 0.22 | 0.28 | It is our model's own estimate, not truth. Ours targets it (the block needs a correct production, the ladder). The baselines are blocked by it |
| recall of mastered | mean of `learner.recall(kc, end)` over (learner, KC ever mastered) pairs (`:1061-1063`); `recall = (1 − slip)·σ(1.3·skill)` (`:237-239`); end = right after the last session, no delay | 0.895 over 248 pairs | 0.788 over 44 | 0.840 over 55 | Selection bias: each arm averages over its own set of KCs and learners. With overlapping sets (graph variant) the difference is +0.002 [95% −0.016, +0.018] |
| truly known | per learner, KCs (the 5 in `items_by_kc`) with `recall(kc, end) ≥ 0.8` (`:1064-1071`) | 3.15 | 1.16 | 1.20 | Independent of our model, and ours does not optimise it. But it is **not measured on items**, new or seen: it reads the latent skill on an imaginary medium typed item with b = 0 and one KC. It ignores the bug, secondary KCs and item difficulty, and it is taken right after the last session. The learner has no item memory, so "new vs seen" does not exist here |

Per active minute [run, seed 1]:

| | ours | FSRS | FSRS+NI |
| --- | --- | --- | --- |
| active minutes per learner (30 days) | 63.3 | 71.3 | 71.0 |
| answers per learner | 307 | 474 | 474 |
| mastered per active hour | 1.18 | 0.19 | 0.23 |
| known gain over day 0 per active hour | 2.96 | 0.95 | 0.99 |
| known gain per 100 answers | 1.02 | 0.24 | 0.25 |

Paired cluster bootstrap by learner (docs/11; B = 2000, SplitMix64 seed 0x20261125, percentiles 10–90 and 2.5–97.5), ours − FSRS, seed 1 [run]:

| Metric | point | 80% | 95% |
| --- | --- | --- | --- |
| mastered | +1.02 | [+0.87, +1.18] | [+0.80, +1.26] |
| truly known | +1.99 | [+1.86, +2.13] | [+1.80, +2.19] |
| truly known +30 d, no reviews | +1.38 | [+1.27, +1.49] | [+1.21, +1.55] |
| retention (own sets) | +0.107 | [+0.064, +0.153] | [+0.041, +0.175] |
| frustrated share | −0.670 | [−0.696, −0.646] | [−0.706, −0.630] |
| frustrated, sessions ≥ 8 answers | −0.586 | [−0.613, −0.560] | [−0.625, −0.544] |
| active minutes | −8.1 | [−9.8, −6.3] | [−10.8, −5.3] |
| known gain per active hour | +2.01 | [+1.91, +2.10] | [+1.86, +2.15] |

ours − FSRS+NI is within about 0.05 of these, except retention (+0.055 [+0.013, +0.102]).

## 6. Benchmark v1 (done)

- `tools/sim/bench/v1/README.md`: what is frozen and how to reproduce. `args` = `200 30 1`; `expected.txt` = byte-exact output of `cohort` (identical to the F2 run, and identical in debug and release); `planner-params.json`, `model-params.json` = `Params::default()` and the model's `Params::default()`. Versions `session-prototype-1+71adc6ae28da63d6` and `knowledge-v1+036bb2a606efe0a3`.
- `content/bench/v1/bundle.json`: frozen build of `content/` at `a2a40ac` (content version `10c6a974…`, file SHA-256 `4d98b0e4…`). It sits under `content/` on purpose: docs/14 says code (AGPL) is "everything except `content/`", so a content build under `tools/` would have changed its licence. content-build reads only `.ncl`, so it ignores the file.
- `just bench-v1`: release run plus `diff -u` against `expected.txt`.
- `tools/sim/tests/bench_v1.rs`: runs the `cohort` binary (`CARGO_BIN_EXE_cohort`) and fails at the first differing line (about 28 s in debug). Checked that it fails when one character of `expected.txt` changes.
- `flake.nix`: `./tools/sim/bench` added to the source fileset so the test sees its files in `nix flake check`.
- docs/11 (К5, «Версии симулятора») and README: noted. docs/11 open question 5: the test also fails when the planner or the model changes, and how to record a new reference without editing v1 is for the author to decide.

## 7. Mastery cost [run]

Ours, seed 1, 248 mastered (learner, KC) pairs:

- **Answers**: median 54 (per-KC medians 46–65) direct non-rapid answers where the KC is primary.
- **Calendar**: median 15 days from the first answer to mastery, over a median of 8 sessions touching the KC.
- **Last condition met**: knowledge, in 248 of 248 cases. At the start of the mastery session, automaticity and production were already met in 100% of cases. Among practised but never mastered pairs (476), knowledge is failing in 100%, automaticity in 2%, production in 2%.
- **The model lags the learner**: the KC was truly known (recall ≥ 0.8 at a session end) after a median of 10 answers, 14 days before the model called it mastered. True recall at mastery was 0.91–0.95 per KC, well above the 0.78 that θ* is meant to mean.
- **Bottleneck**: the model's knowledge bound, P(θ_eff > 1.5) ≥ 0.8, needs about 50 answers. Calendar time follows from ours giving each KC about 6–7 answers per session: the block closes, 3 interleaved items follow, and the session ends with about 23 of 25 minutes unused. Baselines, when not trapped, need 60–100 answers but reach them in 4–7 days, because they drill about 14 items a day (graph variant: FSRS 101 answers in 7 days, ours 53 in 14). So the answer count is the model's constraint, and calendar time is the planner's.
- **Why the knowledge bound is slow** [hyp, not verified]: slope mismatch (1.0 vs 1.3); items chosen near p* = 0.8 carry little information; the γ = 0.3 discount on "unsure" correct answers (the learner says Unsure whenever p_skill ≤ 0.7); secondary-KC failures blamed on the primary KC; bug answers on 30% of KCs.

## 8. Diagnostic runs (no repo code change) [run]

| Run | Result |
| --- | --- |
| seeds 1–5, 200 learners | ours − FSRS, mastered: +1.02, +1.00, +0.91, +0.86, +0.88; known: +1.99, +1.99, +1.87, +1.92, +1.95; retention: +0.107, +0.056, +0.053, +0.074, +0.066; frustrated: −0.67 to −0.68. Every 95% interval excludes 0. Seed 1 has the largest retention gap |
| 1000 learners, seed 1 | ours 1.154 / 3.083 / 0.895 / 9.5%; FSRS 0.206 / 1.143 / 0.821 / 77.7%. ours − FSRS: mastered +0.95 [95% +0.85, +1.05], known +1.94 [+1.85, +2.02], retention +0.074 [+0.049, +0.102] |
| delayed tests, no reviews (latent recall at end + 7 d / + 30 d) | ours known 3.15 → 2.81 → 2.51; FSRS 1.16 → 1.15 → 1.14. Retention of mastered: ours 0.895 → 0.879 → 0.833; FSRS 0.788 → 0.756 → 0.706 |
| no explanations (bundle `explanations` cleared) | section 4 table |
| graph variant: `requires_closure[a-an-sound] = {first-mention: 1, countability: 2}` in memory (learner, planner and baseline all see it) | ours 0.92 mastered / 2.45 known / 0.902 retention / 4.6% frustrated (15.8% among ≥ 8-answer sessions) / 50.0 min. FSRS: 2.15 / 2.76 / 0.900 / 10.3% (10.4%) / 71.8 min. ours − FSRS: mastered −1.23 [95% −1.42, −1.05]; known −0.31 [−0.51, −0.11]; known +30 d −0.30 [−0.45, −0.15]; retention +0.002 [−0.016, +0.018]; known gain per active hour +0.62 [+0.47, +0.78]; per 100 answers +0.26 [+0.20, +0.32] |
| "baselines with explanations" | not needed: they already get one for each KC they reach |

Runs that need a code change, described instead of made:

- **Ungated baseline**: a new KC every N answers, or K new per day, instead of waiting for `ever_mastered` (`session_fsrs:863-866`).
- **Baseline order** from requires + supports + item secondary KCs, or from the authored sequence (`:839-862`).
- **Equal time**: cap the baselines' daily seconds at ours' for the same learner and day, or let ours take a second learning KC after `block_closed`. The second option is a planner change and would need the spec first.
- **Fixed programme and random arms** of docs/11 (not implemented).
- **Oracle arm**: picks KC and item from the learner's true skill.
- **Item-level delayed tests** on held-out items, with bug and secondary KCs.

## 9. Bugs and defects (listed, not fixed)

| # | File:function:lines | Logic | Consequence |
| --- | --- | --- | --- |
| 1 | `cohort.rs:session_fsrs:839-866` | Baseline order is (requires depth, id), blind to `supports` and to item secondary KCs; a-an-sound comes first | Baseline is trapped; most of the F2 gap [run] |
| 2 | `cohort.rs:session_fsrs:863-866` | Progress only on our `ever_mastered`, with no deferral, unlike ours (`adapt.rs:92-95`, `build.rs:246-250`) | A never-mastered KC blocks the baseline forever; its progress depends on our model |
| 3 | `cohort.rs:Learner::answer:306-310` vs `:371-379` | Success needs every slot KC at full weight; only primary KCs learn | Secondary-KC skill cannot grow from such items; causes the trap; differs from the model (secondary w = 0.3) |
| 4 | `cohort.rs:SessionReport::answered:567-577` | Frustration needs a full 8-answer window | Short sessions (46% of ours') can never be frustrated; ours' rate is deflated |
| 5 | `cohort.rs:cohort:1061-1063` | Retention is taken over each arm's own mastered set, at the end, with no delay; docs/11 asks for retention 30 days later | Selection bias; not the docs/11 metric |
| 6 | `cohort.rs:recall:237-239`, `:1064-1071` | "Truly known" ignores the bug, secondary KCs and item difficulty | Overstates knowledge; a learner who fails 40% of answers from a wrong idea counts as knowing |
| 7 | `cohort.rs:session_fsrs:872-876` vs `values.rs:291-298` | Baseline picks the first shown short explanation in any language; ours picks the ru chain | Possibly a different record and reading time; minor |
| 8 | `cohort.rs:Learner::answer:274, 322-325` vs `v1.rs:524-525` | Rapid and normal latencies are separated by the model's thresholds | Rapid detection is perfect by construction; the rapid rule is never stressed |
| 9 | `cohort.rs:326-330` | Confidence comes from the true p_skill | Perfectly calibrated confidence; the unsure discount and ConfidentError (6035 firings) run on a clean signal |
| 10 | `cohort.rs:344-346, 361` | Listed misconceptions are random, not learner states | Misconception features of the planner cannot help or hurt except through noise |
| 11 | `cohort.rs:practise:205-210` | Stability ×1.6 for any gap of at least 12 h; +0.1 per massed answer | No interval effect; massed drills of about 14 items add +1.3 per day; spacing arguments cannot be tested |
| 12 | `cohort.rs:Learner::new:150-156` | Hidden edges only toward KCs earlier in (level, id) order | Id-dependent; here a-an-sound can never have a hidden prerequisite |
| 13 | `cohort.rs:Learner::answer:373-378`, `lesson:231` | The bug decays only on non-bug errors and on the explanation | Strong learners keep the bug forever: a permanent accuracy cap |
| 14 | `cohort.rs:Track::answer:520-532` | Answers to mastery count only direct non-rapid primary answers | A KC mastered through propagation shows too few answers; minor |
| 15 | benchmark validity | 68 items of 10–20 s: no arm fills 25 minutes | Budget, fatigue and rapid growth are inert; "25-min budget" claims are untestable |
| 16 | `cohort.rs:Baseline::pick:723-729` | One-step plan with `production: false` | The FSRS+NI arm can only reorder the pool it would exhaust anyway: no-effect by construction |
| 17 | `README.md`, Simulator run paragraph | "The gain comes from how the session is put together" | Not supported (section 0); the README should be corrected after the author's review |

---

## (a) Claims

Supported:

- Zero validator violations in about 140,000 checks [run].
- Determinism: the output is byte-identical in debug and release, and the harness reproduces it [run].
- Within this configuration, results are stable over seeds 1–5 and 1000 learners [run].
- Ours spends fewer minutes (−8.1, 95% [−10.8, −5.3]) and fewer answers [run].
- Ours gains more truly known KCs per active hour and per 100 answers, in both the default and graph configurations (graph: +0.62 per hour [+0.47, +0.78]; +0.26 per 100 answers [+0.20, +0.32]) [run].
- The model needs about 50 answers per KC against the 15–25 target, and the knowledge bound is the only binding condition [run].

Not supported:

- "Ours beats only FSRS" in mastered, known, retention or frustration. The gap is driven by bugs 1–3: one edge reverses mastered and known [run].
- Retention 0.90 vs 0.79: selection bias (bug 5); it vanishes with overlapping sets [run].
- Frustration 10% vs 77%: bug 4 plus the trap. With the trap removed, among sessions of ≥ 8 answers ours is worse (15.8% vs 10.4%) [run].
- "Our task choice adds little; the gain is session composition" (README): the third arm cannot show an effect by construction (bug 16), and the composition gain is the trap [code, run].
- Anything about a 25-minute session: no arm uses more than about 3 minutes [run].
- "Mastered" as an outcome: it is our model's estimate, and it lags the synthetic truth by about 40 answers [run].

## (b) Ranked fixes for simulator v2

1. **Arms**: only FSRS with an order that honours requires, supports and secondary KCs, and ungated new topics (K new per day). Then + our `next_item`; + prerequisites (our Ready gate and prerequisite checks on top of FSRS); full (ours); oracle (KC and item from the true state: an upper bound). Also the docs/11 random and fixed-programme arms. The same deferral rule in every arm.
2. **Metrics independent of the model and time-normalised**: delayed tests at +7 and +30 days without reviews, on held-out items never shown, with item b, bug and secondary KCs, at matched calendar points. Report everything per active minute and per answer, and add an equal-time condition. Frustration as the share of 8-answer windows (carried across sessions) or per minute. "Mastered" only next to true skill at mastery.
3. **Per-learner output and paired cluster bootstrap built into the report**; 5 seeds by default.
4. **Synthetic graphs of 50–200 KCs without texts** (extend `synthetic_bundle`: levels, secondary KCs, contrasts, supports) so that 25 minutes binds and choice matters.
5. **Learner realism**: secondary KCs learn with a weight; item memory (a familiarity boost on repeats that does not transfer); persistent listed misconceptions that respond to remedies; miscalibrated confidence; rapid-guess latencies that overlap the thresholds; reading speed independent of answer pace, with its own mean; typing speed; device; a power law of practice; correlated traits.
6. **Alternative forgetting law** (power, `R = (1 + t/(9S))^−1`, or Wickelgren) with interval-dependent stability gain; run every arm under both laws.
7. **Sensitivity of hand-set effects**: explanation weight {0, 0.4, 0.8} and bug reduction; report how the ranking moves.
8. **Learner profiles**: weak/strong, fast/slow forgetters, high guessers, L1 offsets; metrics per profile.
9. **Stress scenarios**: reversed and missing edges (drop 50%), extra hidden edges (p 0.3), high guessing (rapid 0.3), skipped days (3 of 7 days, gaps of 7–14 days to exercise the return check), a 10-minute budget.
