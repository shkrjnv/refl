//! Building a session and recomputing the steps ahead
//! (`docs/04-placement-planner.md`, «Сборка сессии в прототипе»).

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use super::values::{LearnValue, ReviewValue};
use super::{
    Budget, Done, End, History, Learn, Phase, Plan, PlannedReview, Planner, Rule, Session, Step,
    draw, second_seed,
};
use crate::bundle::KcStatus;
use crate::content::EdgeKind;
use crate::ids::{FamilyId, ItemId, KcId};
use crate::model::State;
use crate::model::derived::KcProgress;
use crate::value::{Lang, Seed, Timestamp};

const DAY_MS: i64 = 86_400_000;

/// The UTC day of a timestamp.
fn utc_day(t: Timestamp) -> i64 {
    t.0.div_euclid(DAY_MS)
}

impl Step {
    fn new(kc: &KcId, phase: Phase, production: bool, seconds: u32) -> Self {
        Self {
            kc: kc.clone(),
            phase,
            production,
            family: None,
            seconds,
        }
    }
}

/// What the session start decided: the reviews, the learning candidates and
/// the pick among them.
pub(super) struct Start {
    /// Every due KC with an eligible item, by V^rev.
    pub due: Vec<ReviewValue>,
    /// Empty under the debt rule; by V^learn.
    pub learn: Vec<LearnValue>,
    /// Index into `learn` and its propensity.
    pub pick: Option<(usize, f64)>,
}

/// A second learning KC opened by rule 6, with the draw that picked it.
pub(super) struct Opened {
    pub learn: Learn,
    /// The candidates by V^learn now; `learn` is `values[index]`.
    pub values: Vec<LearnValue>,
    pub index: usize,
    pub propensity: f64,
    pub seed: Seed,
}

/// Steps ahead as they are planned: the run rule and the unseen items of
/// each KC are checked on every push.
struct Builder<'p, 'a> {
    planner: &'p Planner<'a>,
    plan: &'p Plan,
    steps: Vec<Step>,
    /// The KC of the last step counted by the run rule, and its run.
    run: Option<(KcId, usize)>,
    /// The run rule applies: off while a learning KC goes on alone.
    runs: bool,
    /// Unseen eligible items per KC: all, and productions among them.
    capacity: BTreeMap<KcId, (u32, u32)>,
    shown: BTreeSet<&'p ItemId>,
    /// A step of the main part was left out for lack of time.
    short_of_time: bool,
}

impl<'p, 'a> Builder<'p, 'a> {
    fn new(planner: &'p Planner<'a>, plan: &'p Plan, runs: bool) -> Self {
        let mut b = Self {
            planner,
            plan,
            steps: Vec::new(),
            run: None,
            runs,
            capacity: BTreeMap::new(),
            shown: plan.shown().map(|(_, s)| &s.item.id).collect(),
            short_of_time: false,
        };
        for d in &plan.done {
            b.extend_run(&d.step);
        }
        b
    }

    fn extend_run(&mut self, step: &Step) {
        if !step.phase.counts_in_runs() {
            return;
        }
        self.run = match self.run.take() {
            Some((kc, n)) if kc == step.kc => Some((kc, n + 1)),
            _ => Some((step.kc.clone(), 1)),
        };
    }

    fn breaks_run(&self, step: &Step) -> bool {
        self.runs
            && step.phase.counts_in_runs()
            && matches!(&self.run, Some((kc, n)) if *kc == step.kc && *n >= self.planner.params.max_run)
    }

    fn capacity(&mut self, kc: &KcId) -> &mut (u32, u32) {
        if !self.capacity.contains_key(kc) {
            let mut items: BTreeMap<&ItemId, bool> = BTreeMap::new();
            for s in self.planner.slots(kc) {
                if !self.shown.contains(&s.item.id) {
                    items.insert(&s.item.id, s.projection.is_production());
                }
            }
            let total = u32::try_from(items.len()).unwrap_or(u32::MAX);
            let production = u32::try_from(items.values().filter(|p| **p).count()).unwrap_or(0);
            self.capacity.insert(kc.clone(), (total, production));
        }
        self.capacity.get_mut(kc).expect("inserted above")
    }

    /// An unseen eligible item of `family` where `kc` is primary exists.
    fn family_open(&self, kc: &KcId, family: &FamilyId) -> bool {
        self.planner
            .slots(kc)
            .iter()
            .any(|s| &s.projection.family == family && !self.shown.contains(&s.item.id))
    }

    /// Adds the step if it keeps the run rule and has an unseen item.
    fn push(&mut self, step: Step) -> bool {
        if self.breaks_run(&step) {
            return false;
        }
        match (&step.phase, &step.family) {
            (Phase::Lesson, _) => {}
            (Phase::Final, Some(family)) => {
                if !self.family_open(&step.kc, family) {
                    return false;
                }
            }
            _ => {
                let production = step.production;
                let cap = self.capacity(&step.kc);
                if cap.0 == 0 || (production && cap.1 == 0) {
                    return false;
                }
                cap.0 -= 1;
                if production {
                    cap.1 -= 1;
                }
            }
        }
        self.extend_run(&step);
        self.steps.push(step);
        true
    }

    /// A production item was answered on `kc` in this session, or a
    /// production step on it is planned.
    fn has_production(&self, kc: &KcId) -> bool {
        answered_production(self.plan, kc) || self.steps.iter().any(|s| s.production && &s.kc == kc)
    }

    /// Drops the learning steps of a KC left without production, then the
    /// steps that would break the run rule, until neither changes anything:
    /// a dropped step can expose either again.
    fn finish(mut self) -> Vec<Step> {
        loop {
            let starved: BTreeSet<KcId> = self
                .steps
                .iter()
                .filter(|s| matches!(s.phase, Phase::Block | Phase::Interleaved))
                .map(|s| &s.kc)
                .filter(|kc| !self.has_production(kc))
                .cloned()
                .collect();
            let before = self.steps.len();
            self.steps
                .retain(|s| !(s.phase.is_learning() && starved.contains(&s.kc)));
            let steps = core::mem::take(&mut self.steps);
            self.run = None;
            for d in &self.plan.done {
                self.extend_run(&d.step);
            }
            for s in steps {
                if !self.breaks_run(&s) {
                    self.extend_run(&s);
                    self.steps.push(s);
                }
            }
            if self.steps.len() == before {
                return self.steps;
            }
        }
    }
}

/// A production item was answered on a step of `kc` in this session.
pub(super) fn answered_production(plan: &Plan, kc: &KcId) -> bool {
    plan.shown()
        .any(|(step, s)| &step.kc == kc && s.kind.is_production())
}

impl<'a> Planner<'a> {
    /// Every live KC due for review with an eligible item, by V^rev
    /// descending, then id.
    pub(super) fn due(&self, s: &State, now: Timestamp) -> Vec<ReviewValue> {
        let mut due: Vec<ReviewValue> = self
            .bundle
            .content
            .kcs
            .iter()
            .filter(|e| e.status == KcStatus::Live)
            .filter_map(|e| self.review_value(s, &e.kc.id, now))
            .filter(|r| r.seconds > 0)
            .collect();
        due.sort_by(|a, b| b.value.total_cmp(&a.value).then_with(|| a.kc.cmp(&b.kc)));
        due
    }

    /// The session decision: the reviews that fit, the debt rule and the
    /// learning KC. A KC deferred on the UTC day of `now` waits for the next.
    pub(super) fn start(
        &self,
        s: &State,
        history: &History,
        now: Timestamp,
        budget: Budget,
        seed: Seed,
    ) -> Start {
        let p = self.params;
        let limit = p.review_share * f64::from(budget.seconds());
        let due = self.due(s, now);
        let demand: u32 = due.iter().map(|r| r.seconds).sum();
        if f64::from(demand) > limit {
            return Start {
                due,
                learn: Vec::new(),
                pick: None,
            };
        }
        let unlocks = self.unlock(s, now);
        let mut learn: Vec<LearnValue> = self
            .bundle
            .content
            .kcs
            .iter()
            .filter(|e| e.status == KcStatus::Live)
            .map(|e| &e.kc.id)
            .filter(|kc| {
                !matches!(
                    self.progress(s, kc, now),
                    KcProgress::Mastered | KcProgress::LongTerm
                )
            })
            .filter(|kc| {
                history
                    .last_deferred(kc)
                    .is_none_or(|t| utc_day(t) < utc_day(now))
            })
            .filter(|kc| due.iter().all(|r| &r.kc != *kc))
            .filter(|kc| self.ready(s, kc, now) >= p.ready_min)
            .filter(|kc| self.has_production(kc))
            .map(|kc| self.learn_value(s, kc, now, &unlocks))
            .collect();
        learn.sort_by(|a, b| b.value.total_cmp(&a.value).then_with(|| a.kc.cmp(&b.kc)));
        let pick = learn.first().map(|best| {
            let bar = p.tie_share * best.value;
            let n = learn.iter().take_while(|v| v.value >= bar).count();
            draw(seed, n)
        });
        Start { due, learn, pick }
    }

    /// The KCs interleaved with `kc`: linked by `contrasts` either way, live,
    /// not blocked, started, not among the reviews, with a production item.
    fn contrasts(
        &self,
        s: &State,
        kc: &KcId,
        now: Timestamp,
        reviews: &[PlannedReview],
    ) -> Vec<KcId> {
        let mut out: Vec<KcId> = self
            .bundle
            .content
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Contrasts)
            .filter_map(|e| {
                if &e.from == kc {
                    Some(&e.to)
                } else if &e.to == kc {
                    Some(&e.from)
                } else {
                    None
                }
            })
            .filter(|c| *c != kc && self.is_live(c))
            .filter(|c| self.ready(s, c, now) >= self.params.ready_min)
            .filter(|c| self.progress(s, c, now) != KcProgress::NotStarted)
            .filter(|c| reviews.iter().all(|r| &r.kc != *c))
            .filter(|c| self.has_production(c))
            .cloned()
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// The plan of a session starting at `now` and its `plan_issued`.
    /// `history` is read from the log before the session; `lang` is the
    /// interface language, which picks the explanation a lesson shows.
    pub fn build_session(
        &self,
        s: &State,
        history: &History,
        now: Timestamp,
        budget: Budget,
        lang: Lang,
        seed: Seed,
    ) -> Session {
        let p = self.params;
        let start = self.start(s, history, now, budget, seed);
        let limit = p.review_share * f64::from(budget.seconds());
        let mut reviews = Vec::new();
        let mut used = 0u32;
        for r in &start.due {
            if f64::from(used + r.seconds) > limit {
                break;
            }
            used += r.seconds;
            reviews.push(PlannedReview {
                kc: r.kc.clone(),
                seconds: r.seconds,
            });
        }
        let learn: Vec<Learn> = start
            .pick
            .map(|(i, _)| self.new_learn(s, &start.learn[i].kc, now, &reviews, lang))
            .into_iter()
            .collect();
        let mut plan = Plan {
            budget,
            started: now,
            seed,
            learn,
            candidates: start.learn.iter().map(|v| v.kc.clone()).collect(),
            reviews,
            done: Vec::new(),
            steps: Vec::new(),
            break_suggested: false,
            lang,
            end: None,
        };
        // Rule 6 needs a closed block, so it cannot open a KC before any
        // answer.
        let opened = self.replan(s, &mut plan, now);
        debug_assert!(opened.is_none(), "rule 6 before any answer");
        let issued = self.issue_start(s, &plan, &start, now);
        Session { plan, issued }
    }

    /// A learning KC with its contrasts and whether it starts with a lesson.
    fn new_learn(
        &self,
        s: &State,
        kc: &KcId,
        now: Timestamp,
        reviews: &[PlannedReview],
        lang: Lang,
    ) -> Learn {
        Learn {
            kc: kc.clone(),
            contrasts: self.contrasts(s, kc, now, reviews),
            lesson: !s.explained.contains(kc) && self.lesson_explanation(kc, lang).is_some(),
        }
    }

    /// Opens a second learning KC when rule 6 fires, then recomputes the
    /// steps ahead and, when none is left, why the session ends
    /// (`docs/04-placement-planner.md`, «Конец сессии»). Returns the opened
    /// KC with its draw.
    pub(super) fn replan(&self, s: &State, plan: &mut Plan, now: Timestamp) -> Option<Opened> {
        let opened = self.second_learn(s, plan, now);
        if let Some(o) = &opened {
            plan.learn.push(o.learn.clone());
        }
        let (steps, short_of_time) = self.remaining(s, plan, now);
        plan.end = steps.is_empty().then(|| {
            if short_of_time {
                return End::Budget;
            }
            if plan.learn.is_empty() {
                return if plan.reviews.is_empty() {
                    End::NothingDue
                } else if plan.pending_reviews().is_empty() {
                    End::ReviewsDone
                } else {
                    End::ItemsExhausted
                };
            }
            if self.active_learns(s, plan, now).is_empty() {
                let held = plan.learn.iter().any(|l| {
                    plan.deferred(&l.kc) || self.ready(s, &l.kc, now) < self.params.ready_min
                });
                return if held {
                    End::KcDeferred
                } else {
                    End::KcMastered
                };
            }
            match self.alone(s, plan, now) {
                Some(l) if plan.interleaved_done(&l.kc) >= self.params.alone_cap => End::KcCap,
                // Steps were wanted, but no unseen item was left for them.
                _ => End::ItemsExhausted,
            }
        });
        plan.steps = steps;
        opened
    }

    /// The model counts `kc` mastered now.
    fn is_mastered(&self, s: &State, kc: &KcId, now: Timestamp) -> bool {
        matches!(
            self.progress(s, kc, now),
            KcProgress::Mastered | KcProgress::LongTerm
        )
    }

    /// The learning KCs that may still be given: not deferred, not blocked,
    /// not mastered in this session.
    pub(super) fn active_learns<'p>(
        &self,
        s: &State,
        plan: &'p Plan,
        now: Timestamp,
    ) -> Vec<&'p Learn> {
        plan.learn
            .iter()
            .filter(|l| !plan.deferred(&l.kc))
            .filter(|l| self.ready(s, &l.kc, now) >= self.params.ready_min)
            .filter(|l| !self.is_mastered(s, &l.kc, now))
            .collect()
    }

    /// An eligible item of `kc` not shown in this session; a production
    /// one if `production`.
    fn has_unseen(&self, plan: &Plan, kc: &KcId, production: bool) -> bool {
        let shown: BTreeSet<&ItemId> = plan.shown().map(|(_, sh)| &sh.item.id).collect();
        self.slots(kc)
            .iter()
            .any(|c| !shown.contains(&c.item.id) && (!production || c.projection.is_production()))
    }

    /// What the learning KCs interleave with besides reviews: their
    /// contrasts with Ready above the threshold, then the other started,
    /// not mastered candidates of the session start, in V^learn order.
    fn partners(&self, s: &State, plan: &Plan, now: Timestamp) -> Vec<KcId> {
        let p = self.params;
        let mut out: Vec<KcId> = Vec::new();
        let contrasts = plan.learn.iter().flat_map(|l| &l.contrasts);
        let started = plan.candidates.iter().filter(|kc| {
            self.progress(s, kc, now) != KcProgress::NotStarted && !self.is_mastered(s, kc, now)
        });
        for kc in contrasts.chain(started) {
            if !plan.is_learn(kc) && !out.contains(kc) && self.ready(s, kc, now) >= p.ready_min {
                out.push(kc.clone());
            }
        }
        out
    }

    /// Nothing besides the learning KCs can get a step: no review left and
    /// no partner with an unseen item.
    fn nothing_else(&self, s: &State, plan: &Plan, now: Timestamp) -> bool {
        plan.pending_reviews().is_empty()
            && !self
                .partners(s, plan, now)
                .iter()
                .any(|kc| self.has_unseen(plan, kc, false))
    }

    /// The learning KC that goes on alone: the only one that may still be
    /// given, with nothing else that can get a step. The run rule is off
    /// for it, and it gets at most `alone_cap` interleaved steps in the
    /// session.
    fn alone<'p>(&self, s: &State, plan: &'p Plan, now: Timestamp) -> Option<&'p Learn> {
        let [only] = self.active_learns(s, plan, now)[..] else {
            return None;
        };
        self.nothing_else(s, plan, now).then_some(only)
    }

    /// Rule 6: with one learning KC whose block is closed (or which is
    /// mastered), no review left and no partner that can get a step, a
    /// second one opens: the best start candidate by V^learn now, drawn
    /// like the first with the second seed.
    fn second_learn(&self, s: &State, plan: &Plan, now: Timestamp) -> Option<Opened> {
        let p = self.params;
        let [first] = plan.learn.as_slice() else {
            return None;
        };
        if p.learn_max < 2
            || plan.deferred(&first.kc)
            || !(plan.block_closed(&first.kc) || self.is_mastered(s, &first.kc, now))
            || !self.nothing_else(s, plan, now)
        {
            return None;
        }
        let unlocks = self.unlock(s, now);
        let mut values: Vec<LearnValue> = plan
            .candidates
            .iter()
            .filter(|kc| !plan.is_learn(kc))
            .filter(|kc| !self.is_mastered(s, kc, now))
            .filter(|kc| self.ready(s, kc, now) >= p.ready_min)
            .filter(|kc| self.has_unseen(plan, kc, true))
            .map(|kc| self.learn_value(s, kc, now, &unlocks))
            .collect();
        values.sort_by(|a, b| b.value.total_cmp(&a.value).then_with(|| a.kc.cmp(&b.kc)));
        let best = values.first()?;
        let bar = p.tie_share * best.value;
        let n = values.iter().take_while(|v| v.value >= bar).count();
        let seed = second_seed(plan.seed);
        let (index, propensity) = draw(seed, n);
        let learn = self.new_learn(s, &values[index].kc, now, &plan.reviews, plan.lang);
        Some(Opened {
            learn,
            values,
            index,
            propensity,
            seed,
        })
    }

    /// The steps ahead: a pending prerequisite check, the lesson and block of
    /// the learning KC, reviews interleaved with it and its contrasts, the
    /// final. A pure function of the plan's fixed part, its answers, the
    /// state and `now`; also whether a step of the main part was left out
    /// for lack of time.
    #[allow(clippy::too_many_lines)] // one ordered recipe
    pub(super) fn remaining(&self, s: &State, plan: &Plan, now: Timestamp) -> (Vec<Step>, bool) {
        let p = self.params;
        let budget = f64::from(plan.budget.seconds());
        let (mut used_main, mut used_final) = (0.0, 0.0);
        for d in &plan.done {
            // An item by its expected time, the lesson by its estimate.
            let seconds = f64::from(d.shown.as_ref().map_or(d.step.seconds, |sh| sh.seconds));
            if d.step.phase == Phase::Final {
                used_final += seconds;
            } else {
                used_main += seconds;
            }
        }
        let mut main_left = budget * (1.0 - p.final_share) - used_main;
        let learns = self.active_learns(s, plan, now);
        let alone = self.alone(s, plan, now);
        let mut out = Builder::new(self, plan, alone.is_none());

        if let Some(Done {
            rule: Some(Rule::PrerequisiteCheck { kc }),
            ..
        }) = plan.done.last()
        {
            let seconds = self.mean_seconds(kc);
            if out.push(Step::new(kc, Phase::Prerequisite, false, seconds)) {
                main_left -= f64::from(seconds);
            }
        }

        // Blocked first: the lesson and block of a learning KC whose block
        // is open (one at a time: rule 6 opens the second after the first
        // closed).
        for learn in &learns {
            let kc = &learn.kc;
            if plan.block_closed(kc) {
                continue;
            }
            let explanation = self.lesson_explanation(kc, plan.lang);
            if let Some(e) = explanation
                && learn.lesson
                && plan
                    .done
                    .iter()
                    .all(|d| !(d.step.phase == Phase::Lesson && &d.step.kc == kc))
            {
                let seconds = self.explanation_seconds(e);
                out.push(Step::new(kc, Phase::Lesson, false, seconds));
                main_left -= f64::from(seconds);
            }
            let block_done = u32::try_from(
                plan.done
                    .iter()
                    .filter(|d| d.step.phase == Phase::Block && &d.step.kc == kc)
                    .count(),
            )
            .unwrap_or(u32::MAX);
            let on_kc = u32::try_from(plan.shown().filter(|(st, _)| &st.kc == kc).count())
                .unwrap_or(u32::MAX);
            let block_left = p
                .block_items
                .saturating_sub(block_done)
                .max(1)
                .min(p.kc_cap.saturating_sub(on_kc));
            let needs = !plan
                .shown()
                .any(|(st, sh)| &st.kc == kc && sh.correct && !sh.rapid && sh.kind.is_production());
            let seconds = self.mean_seconds(kc);
            for i in block_done + 1..=block_done + block_left {
                let production = needs && i >= p.block_items && i % 2 == 0;
                if !out.push(Step::new(kc, Phase::Block, production, seconds)) {
                    break;
                }
                main_left -= f64::from(seconds);
            }
        }

        // The block comes before reviews: those that no longer fit go,
        // lowest V^rev first.
        let mut reviews = plan.pending_reviews();
        while f64::from(reviews.iter().map(|r| r.seconds).sum::<u32>()) > main_left {
            reviews.pop();
            out.short_of_time = true;
        }
        let mut left = main_left - f64::from(reviews.iter().map(|r| r.seconds).sum::<u32>());

        // Interleaved after: the learning KCs, in turn, with reviews,
        // contrasts and other started KCs; a KC alone, up to its cap.
        if learns.is_empty() {
            for r in reviews {
                out.push(Step::new(&r.kc, Phase::Review, false, r.seconds));
            }
        } else {
            let learning: Vec<&KcId> = learns.iter().map(|l| &l.kc).collect();
            let partners = self.partners(s, plan, now);
            let partners: Vec<&KcId> = partners.iter().collect();
            let cap = alone.map(|l| p.alone_cap.saturating_sub(plan.interleaved_done(&l.kc)));
            self.interleave(&mut out, &learning, &partners, &reviews, cap, &mut left);
        }

        let mut final_left = budget * p.final_share - used_final;
        for (family, kc) in plan.final_families() {
            let seconds = self
                .slots(kc)
                .iter()
                .filter(|sl| &sl.projection.family == family)
                .map(|sl| sl.projection.expected_time)
                .max()
                .unwrap_or(0);
            if seconds == 0 || f64::from(seconds) > final_left {
                continue;
            }
            let mut step = Step::new(kc, Phase::Final, false, seconds);
            step.family = Some(family.clone());
            if out.push(step) {
                final_left -= f64::from(seconds);
            }
        }
        let short_of_time = out.short_of_time;
        (out.finish(), short_of_time)
    }

    /// Other, learning KC, other, …; the learning KCs take their turns in
    /// order, «other» alternates a review and a partner (a contrast or
    /// another started KC), reviews once each, partners round. The
    /// learning KCs get at most `cap` steps (a KC going on alone). Stops
    /// when neither turn can add a step.
    fn interleave(
        &self,
        b: &mut Builder<'_, 'a>,
        learning: &[&KcId],
        contrasts: &[&KcId],
        reviews: &[&PlannedReview],
        cap: Option<u32>,
        left: &mut f64,
    ) {
        let (mut next_review, mut next_contrast, mut next_learn) = (0usize, 0usize, 0usize);
        let mut learn_left = cap.unwrap_or(u32::MAX);
        let mut review_turn = true;
        let mut other_turn = true;
        let mut idle = 0;
        while idle < 2 {
            let pushed = if other_turn {
                let mut try_review = |b: &mut Builder<'_, 'a>| {
                    while let Some(r) = reviews.get(next_review) {
                        next_review += 1;
                        if b.push(Step::new(&r.kc, Phase::Review, false, r.seconds)) {
                            return true;
                        }
                    }
                    false
                };
                let mut try_contrast = |b: &mut Builder<'_, 'a>, left: &mut f64| {
                    for k in 0..contrasts.len() {
                        let c = contrasts[(next_contrast + k) % contrasts.len()];
                        let seconds = self.mean_seconds(c);
                        if f64::from(seconds) > *left {
                            b.short_of_time = true;
                            continue;
                        }
                        let production = !b.has_production(c);
                        if b.push(Step::new(c, Phase::Interleaved, production, seconds)) {
                            *left -= f64::from(seconds);
                            next_contrast += k + 1;
                            return true;
                        }
                    }
                    false
                };
                if review_turn {
                    if try_review(b) {
                        review_turn = false;
                        true
                    } else {
                        try_contrast(b, left)
                    }
                } else if try_contrast(b, left) {
                    review_turn = true;
                    true
                } else {
                    try_review(b)
                }
            } else if learn_left == 0 {
                false
            } else {
                let mut pushed = false;
                for k in 0..learning.len() {
                    let kc = learning[(next_learn + k) % learning.len()];
                    let seconds = self.mean_seconds(kc);
                    if f64::from(seconds) > *left {
                        b.short_of_time = true;
                        continue;
                    }
                    if b.push(Step::new(kc, Phase::Interleaved, false, seconds)) {
                        *left -= f64::from(seconds);
                        next_learn += k + 1;
                        learn_left -= 1;
                        pushed = true;
                        break;
                    }
                }
                pushed
            };
            idle = if pushed { 0 } else { idle + 1 };
            other_turn = !other_turn;
        }
    }
}

impl Plan {
    /// The planned reviews not yet given: a KC answered in this session in
    /// any phase is no longer due.
    pub(super) fn pending_reviews(&self) -> Vec<&PlannedReview> {
        self.reviews
            .iter()
            .filter(|r| self.shown().all(|(step, _)| step.kc != r.kc))
            .collect()
    }

    /// This session's errors for the final, one per family: confident ones
    /// first, then by time; families already given in the final are out.
    fn final_families(&self) -> Vec<(&FamilyId, &KcId)> {
        let errors: Vec<(&Done, &FamilyId)> = self
            .done
            .iter()
            .filter(|d| d.step.phase != Phase::Final)
            .filter_map(|d| {
                let s = d.shown.as_ref()?;
                (!s.correct && !s.rapid).then_some((d, &s.family))
            })
            .collect();
        let given: BTreeSet<&FamilyId> = self
            .done
            .iter()
            .filter(|d| d.step.phase == Phase::Final)
            .filter_map(|d| d.step.family.as_ref())
            .collect();
        let confident = errors
            .iter()
            .filter(|(d, _)| d.rule == Some(Rule::ConfidentError));
        let other = errors
            .iter()
            .filter(|(d, _)| d.rule != Some(Rule::ConfidentError));
        let mut seen: BTreeSet<&FamilyId> = BTreeSet::new();
        let mut out = Vec::new();
        for (d, family) in confident.chain(other) {
            if !given.contains(family) && seen.insert(family) {
                out.push((*family, &d.step.kc));
            }
        }
        out
    }
}
