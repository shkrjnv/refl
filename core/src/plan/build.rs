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
/// Audit milestones in days since first mastery (`docs/11-quality.md`, «К4»).
const AUDIT_MILESTONES: [f64; 3] = [7.0, 30.0, 90.0];
/// How close to a milestone an audit probe may be.
const AUDIT_WINDOW_DAYS: f64 = 2.0;
/// A probe family must not have been shown for this many days.
const PROBE_FAMILY_DAYS: f64 = 30.0;
/// At most two audit probes in a session ...
const AUDIT_MAX: u32 = 2;
/// ... and one per this many ordinary tasks.
const AUDIT_TASKS_PER_PROBE: u32 = 20;
/// At most fifteen return-check tasks, or ten minutes of estimated work.
const RETURN_CHECK_MAX_TASKS: usize = 15;
const RETURN_CHECK_MAX_SECONDS: f64 = 600.0;

/// The UTC day of a timestamp.
fn utc_day(t: Timestamp) -> i64 {
    t.0.div_euclid(DAY_MS)
}

fn days_between(from: Timestamp, to: Timestamp) -> f64 {
    #[allow(clippy::cast_precision_loss)] // timestamps are within 2^53
    {
        (to.0 - from.0).max(0) as f64 / DAY_MS as f64
    }
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

/// Why a step was or was not added to the steps ahead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PushResult {
    Added,
    /// The step would make a run longer than the run rule allows.
    RunLimit,
    /// The step has no unseen eligible item, or its family has none.
    Unavailable,
}

/// The steps ahead and what prevented any more of them.
struct Remaining {
    steps: Vec<Step>,
    /// A step of the main part was left out for lack of time.
    short_of_time: bool,
    /// An otherwise eligible step was rejected by the run rule.
    run_limited: bool,
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
    /// An otherwise eligible step was rejected by the run rule.
    run_limited: bool,
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
            run_limited: false,
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

    /// The step could be added if the run rule allowed it. This does not
    /// consume capacity, so a `RunLimit` result only records a genuinely
    /// otherwise-eligible rejection.
    fn available(&mut self, step: &Step) -> bool {
        match (&step.phase, &step.family) {
            (Phase::Lesson, _) => true,
            (Phase::Final, Some(family)) => self.family_open(&step.kc, family),
            _ => {
                let production = step.production;
                let cap = self.capacity(&step.kc);
                cap.0 > 0 && (!production || cap.1 > 0)
            }
        }
    }

    /// Adds the step if it keeps the run rule and has an unseen item.
    fn push(&mut self, step: Step) -> PushResult {
        if !self.available(&step) {
            return PushResult::Unavailable;
        }
        if self.breaks_run(&step) {
            self.run_limited = true;
            return PushResult::RunLimit;
        }
        match (&step.phase, &step.family) {
            (Phase::Lesson, _) | (Phase::Final, Some(_)) => {}
            _ => {
                let production = step.production;
                let cap = self.capacity(&step.kc);
                cap.0 -= 1;
                if production {
                    cap.1 -= 1;
                }
            }
        }
        self.extend_run(&step);
        self.steps.push(step);
        // A real accepted step supersedes an earlier RunLimit rejection. If
        // this step is later removed by finish, only finish's own removal
        // evidence should remain.
        self.run_limited = false;
        PushResult::Added
    }

    /// A production item was answered on `kc` in this session, or a
    /// production step on it is planned.
    fn has_production(&self, kc: &KcId) -> bool {
        answered_production(self.plan, kc) || self.steps.iter().any(|s| s.production && &s.kc == kc)
    }

    /// Drops the learning steps of a KC left without production, then the
    /// steps that would break the run rule, until neither changes anything:
    /// a dropped step can expose either again.
    fn finish(mut self) -> (Vec<Step>, bool) {
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
                if self.breaks_run(&s) {
                    self.run_limited = true;
                } else {
                    self.extend_run(&s);
                    self.steps.push(s);
                }
            }
            if self.steps.len() == before {
                return (self.steps, self.run_limited);
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
    /// Whether the first session of this log starts with a return check.
    fn return_check_due(&self, s: &State, now: Timestamp) -> bool {
        s.kcs
            .keys()
            .find_map(|kc| self.entry(kc).map(|e| &e.kc.subject))
            .and_then(|subject| self.model.return_check(s, self.bundle, subject, now))
            .is_some_and(|check| check.due)
    }

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
            history: history.clone(),
            return_check: self.return_check_due(s, now),
            learn,
            candidates: start.learn.iter().map(|v| v.kc.clone()).collect(),
            deferred_today: self
                .bundle
                .content
                .kcs
                .iter()
                .map(|e| &e.kc.id)
                .filter(|kc| {
                    history
                        .last_deferred(kc)
                        .is_some_and(|t| utc_day(t) == utc_day(now))
                })
                .cloned()
                .collect(),
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
        // A due review whose estimate does not fit even its share is still a
        // main-part step that cannot be given. The session ends for lack of
        // budget, not because nothing was due.
        if plan.end == Some(End::NothingDue) && !start.due.is_empty() && plan.reviews.is_empty() {
            plan.end = Some(End::Budget);
        }
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
        let Remaining {
            steps,
            short_of_time,
            run_limited,
        } = self.remaining(s, plan, now);
        plan.end = steps.is_empty().then(|| {
            // A real time failure is the established Budget cause. RunLimit
            // rejects only otherwise eligible steps; it is never a time
            // failure itself, but it does not hide a genuine budget failure.
            if short_of_time {
                return End::Budget;
            }
            if run_limited {
                return End::RunLimit;
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

    /// A production family on `kc` the learner has not seen for 30 days,
    /// not shown in this session and not already chosen for another probe.
    /// The family with the largest expected time wins, then the family id.
    fn probe_family(
        &self,
        plan: &Plan,
        kc: &KcId,
        now: Timestamp,
        excluded: &BTreeSet<FamilyId>,
    ) -> Option<(FamilyId, u32)> {
        let shown: BTreeSet<&ItemId> = plan.shown().map(|(_, sh)| &sh.item.id).collect();
        let session_families: BTreeSet<&FamilyId> =
            plan.shown().map(|(_, sh)| &sh.family).collect();
        let mut families: BTreeMap<&FamilyId, u32> = BTreeMap::new();
        for s in self.slots(kc) {
            let family = &s.projection.family;
            if s.projection.is_production()
                && !shown.contains(&s.item.id)
                && !session_families.contains(family)
                && !excluded.contains(family)
                && plan
                    .history
                    .last_shown(family)
                    .is_none_or(|t| days_between(t, now) >= PROBE_FAMILY_DAYS)
            {
                let entry = families.entry(family).or_insert(0);
                *entry = (*entry).max(s.projection.expected_time);
            }
        }
        families
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)))
            .map(|(family, seconds)| (family.clone(), seconds))
    }

    /// Return-check steps: before the normal plan, production tasks on
    /// mastered KCs, highest risk first, at most fifteen tasks or ten
    /// minutes (`docs/04-placement-planner.md`, «Возвращение после
    /// перерыва»).
    fn return_probe_steps(
        &self,
        s: &State,
        plan: &Plan,
        now: Timestamp,
        initial_excluded: &BTreeSet<FamilyId>,
    ) -> Vec<Step> {
        if !plan.return_check {
            return Vec::new();
        }
        let done = plan
            .done
            .iter()
            .filter(|d| d.step.phase == Phase::ReturnCheck)
            .count();
        let used: BTreeSet<&KcId> = plan
            .done
            .iter()
            .filter(|d| d.step.phase == Phase::ReturnCheck)
            .map(|d| &d.step.kc)
            .collect();
        let mut excluded = initial_excluded.clone();
        let mut candidates: Vec<(f64, Timestamp, KcId)> = s
            .kcs
            .iter()
            .filter(|(kc, k)| k.ever_mastered && self.is_live(kc) && !used.contains(*kc))
            .filter_map(|(kc, _)| {
                let belief = self.model.belief(s, self.bundle, kc, now)?;
                let risk = self.importance(kc) * (1.0 - belief.r) * libm::sqrt(belief.var);
                self.probe_family(plan, kc, now, &excluded)?;
                Some((
                    risk,
                    s.mastered_at.get(kc).copied().unwrap_or(now),
                    kc.clone(),
                ))
            })
            .collect();
        candidates.sort_by(|a, b| {
            b.0.total_cmp(&a.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
        });
        let mut out = Vec::new();
        let mut seconds = 0.0;
        for (_, _, kc) in candidates.into_iter().take(RETURN_CHECK_MAX_TASKS - done) {
            let Some((family, estimated)) = self.probe_family(plan, &kc, now, &excluded) else {
                continue;
            };
            if seconds + f64::from(estimated) > RETURN_CHECK_MAX_SECONDS {
                break;
            }
            seconds += f64::from(estimated);
            excluded.insert(family.clone());
            let mut step = Step::new(&kc, Phase::ReturnCheck, true, estimated);
            step.family = Some(family);
            out.push(step);
        }
        out
    }

    /// Audit probes: mastered KCs near their 7/30/90-day milestone, measured
    /// at most twice per session and once per twenty ordinary tasks. The
    /// candidate order is seeded by the plan's recorded session seed; the
    /// next-item choice records its own seed in `plan_issued`.
    fn audit_probe_steps(
        &self,
        s: &State,
        plan: &Plan,
        normal_steps: &[Step],
        now: Timestamp,
        initial_excluded: &BTreeSet<FamilyId>,
    ) -> Vec<Step> {
        let done_probes = plan
            .done
            .iter()
            .filter(|d| d.step.phase == Phase::Audit)
            .count();
        let ordinary = plan
            .done
            .iter()
            .filter(|d| d.shown.is_some() && !d.step.phase.is_probe())
            .count()
            + normal_steps
                .iter()
                .filter(|st| st.phase != Phase::Lesson)
                .count();
        let allowed = AUDIT_MAX
            .min(u32::try_from(ordinary / AUDIT_TASKS_PER_PROBE as usize).unwrap_or(u32::MAX));
        let add = usize::try_from(allowed)
            .unwrap_or(usize::MAX)
            .saturating_sub(done_probes);
        if add == 0 {
            return Vec::new();
        }
        let used: BTreeSet<&KcId> = plan
            .done
            .iter()
            .filter(|d| d.step.phase == Phase::Audit)
            .map(|d| &d.step.kc)
            .collect();
        let mut excluded = initial_excluded.clone();
        let mut candidates: Vec<(f64, KcId)> = s
            .kcs
            .iter()
            .filter(|(kc, _)| {
                !used.contains(*kc)
                    && self.is_live(kc)
                    && matches!(
                        self.progress(s, kc, now),
                        KcProgress::Mastered | KcProgress::LongTerm
                    )
            })
            .filter_map(|(kc, _)| {
                let mastered = s.mastered_at.get(kc)?;
                let age = days_between(*mastered, now);
                let distance = AUDIT_MILESTONES
                    .iter()
                    .map(|m| libm::fabs(age - m))
                    .fold(f64::INFINITY, f64::min);
                (distance <= AUDIT_WINDOW_DAYS
                    && self.probe_family(plan, kc, now, &excluded).is_some())
                .then_some((distance, kc.clone()))
            })
            .collect();
        candidates.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let mut rng = crate::rng::SplitMix64::new(plan.seed.get() ^ 0xA0D1_7E55);
        let mut out = Vec::new();
        for _ in 0..add {
            if candidates.is_empty() {
                break;
            }
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_precision_loss,
                clippy::cast_sign_loss
            )]
            let index = (rng.next_f64() * candidates.len() as f64) as usize;
            let (_, kc) = candidates.remove(index.min(candidates.len() - 1));
            let Some((family, estimated)) = self.probe_family(plan, &kc, now, &excluded) else {
                continue;
            };
            excluded.insert(family.clone());
            let mut step = Step::new(&kc, Phase::Audit, true, estimated);
            step.family = Some(family);
            out.push(step);
        }
        out
    }

    fn probe_steps(
        &self,
        s: &State,
        plan: &Plan,
        normal_steps: &[Step],
        now: Timestamp,
    ) -> (Vec<Step>, Vec<Step>) {
        let mut excluded: BTreeSet<FamilyId> = normal_steps
            .iter()
            .filter_map(|step| step.family.clone())
            .collect();
        let return_steps = self.return_probe_steps(s, plan, now, &excluded);
        excluded.extend(return_steps.iter().filter_map(|step| step.family.clone()));
        let audit_steps = self.audit_probe_steps(s, plan, normal_steps, now, &excluded);
        (return_steps, audit_steps)
    }

    /// An eligible item of `kc` not shown in this session; a production
    /// one if `production`.
    pub(super) fn has_unseen(&self, plan: &Plan, kc: &KcId, production: bool) -> bool {
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

    /// Nothing besides the learning KCs can get a step: no pending review
    /// with an unseen item and no partner with an unseen item.
    fn nothing_else(&self, s: &State, plan: &Plan, now: Timestamp) -> bool {
        plan.pending_reviews()
            .iter()
            .all(|r| !self.has_unseen(plan, &r.kc, false))
            && !self
                .partners(s, plan, now)
                .iter()
                .any(|kc| self.has_unseen(plan, kc, false))
    }

    /// The pool of rule 6 now: live KCs not mastered, with Ready above the
    /// threshold, not deferred by rule 2 on the session's day or in it, not
    /// a learning KC or review of the session, with an unseen eligible
    /// production item; in bundle order.
    fn second_candidates<'p>(&'p self, s: &State, plan: &Plan, now: Timestamp) -> Vec<&'p KcId> {
        let shown: BTreeSet<&ItemId> = plan.shown().map(|(_, sh)| &sh.item.id).collect();
        self.bundle
            .content
            .kcs
            .iter()
            .filter(|e| e.status == KcStatus::Live)
            .map(|e| &e.kc.id)
            .filter(|kc| !plan.is_learn(kc) && !plan.deferred_today.contains(kc))
            .filter(|kc| plan.reviews.iter().all(|r| &r.kc != *kc))
            .filter(|kc| !self.is_mastered(s, kc, now))
            .filter(|kc| self.ready(s, kc, now) >= self.params.ready_min)
            .filter(|kc| {
                self.slots(kc)
                    .iter()
                    .any(|c| c.projection.is_production() && !shown.contains(&c.item.id))
            })
            .collect()
    }

    /// The learning KC that goes on alone: the only one that may still be
    /// given, with nothing else that can get a step and no candidate for
    /// rule 6 while it may still open one. The run rule is off for it, and
    /// it gets at most `alone_cap` interleaved steps in the session.
    fn alone<'p>(&self, s: &State, plan: &'p Plan, now: Timestamp) -> Option<&'p Learn> {
        let [only] = self.active_learns(s, plan, now)[..] else {
            return None;
        };
        let may_open = plan.learn.len() < self.params.learn_max
            && !self.second_candidates(s, plan, now).is_empty();
        (self.nothing_else(s, plan, now) && !may_open).then_some(only)
    }

    /// Rule 6: with one learning KC whose block is closed (or which is
    /// mastered), no review left and no partner that can get a step, a
    /// second one opens: the best of [`Self::second_candidates`] by V^learn
    /// now, drawn like the first with the second seed.
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
        let mut values: Vec<LearnValue> = self
            .second_candidates(s, plan, now)
            .into_iter()
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
    /// state and `now`; also what prevented any further step.
    #[allow(clippy::too_many_lines)] // one ordered recipe
    fn remaining(&self, s: &State, plan: &Plan, now: Timestamp) -> Remaining {
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
        let mut main_short = false;

        if let Some(Done {
            rule: Some(Rule::PrerequisiteCheck { kc }),
            ..
        }) = plan.done.last()
        {
            let seconds = self.mean_seconds(kc);
            let step = Step::new(kc, Phase::Prerequisite, false, seconds);
            if !out.available(&step) {
                // Availability is not a time failure; normal scheduling continues.
            } else if f64::from(seconds) > main_left {
                out.short_of_time = true;
                main_short = true;
            } else {
                match out.push(step) {
                    PushResult::Added => main_left -= f64::from(seconds),
                    // The run rule is not a time failure. Normal scheduling
                    // may continue with another step.
                    PushResult::RunLimit | PushResult::Unavailable => {}
                }
            }
        }

        if !main_short {
            // Blocked first: the lesson and block of a learning KC whose
            // block is open (one at a time: rule 6 opens the second after
            // the first closed).
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
                    let step = Step::new(kc, Phase::Lesson, false, seconds);
                    debug_assert!(out.available(&step), "a lesson is always available");
                    if f64::from(seconds) > main_left {
                        out.short_of_time = true;
                        main_short = true;
                    } else if out.push(step) == PushResult::Added {
                        main_left -= f64::from(seconds);
                    }
                }
                if main_short {
                    break;
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
                let needs = !plan.shown().any(|(st, sh)| {
                    &st.kc == kc && sh.correct && !sh.rapid && sh.kind.is_production()
                });
                let seconds = self.mean_seconds(kc);
                for i in block_done + 1..=block_done + block_left {
                    let production = needs && i >= p.block_items && i % 2 == 0;
                    let step = Step::new(kc, Phase::Block, production, seconds);
                    if !out.available(&step) {
                        break;
                    }
                    if f64::from(seconds) > main_left {
                        out.short_of_time = true;
                        main_short = true;
                        break;
                    }
                    if out.push(step) != PushResult::Added {
                        break;
                    }
                    main_left -= f64::from(seconds);
                }
                if main_short {
                    break;
                }
            }

            // The block comes before reviews: those that no longer fit go,
            // lowest V^rev first. A pending review whose items are all shown
            // cannot be given and is not an alternative.
            let mut reviews: Vec<&PlannedReview> = plan
                .pending_reviews()
                .into_iter()
                .filter(|r| self.has_unseen(plan, &r.kc, false))
                .filter(|r| out.available(&Step::new(&r.kc, Phase::Review, false, r.seconds)))
                .collect();
            while !reviews.is_empty()
                && f64::from(reviews.iter().map(|r| r.seconds).sum::<u32>()) > main_left
            {
                reviews.pop();
                out.short_of_time = true;
            }
            let mut left = main_left - f64::from(reviews.iter().map(|r| r.seconds).sum::<u32>());

            // Interleaved after: the learning KCs, in turn, with reviews,
            // contrasts and other started KCs; a KC alone, up to its cap.
            if learns.is_empty() {
                for r in reviews {
                    let _ = out.push(Step::new(&r.kc, Phase::Review, false, r.seconds));
                }
            } else {
                let learning: Vec<&KcId> = learns.iter().map(|l| &l.kc).collect();
                let partners = self.partners(s, plan, now);
                let partners: Vec<&KcId> = partners.iter().collect();
                let cap = alone.map(|l| p.alone_cap.saturating_sub(plan.interleaved_done(&l.kc)));
                self.interleave(&mut out, &learning, &partners, &reviews, cap, &mut left);
            }
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
            if out.push(step) == PushResult::Added {
                final_left -= f64::from(seconds);
            }
        }
        let short_of_time = out.short_of_time;
        let (steps, run_limited) = out.finish();
        let (return_steps, audit_steps) = self.probe_steps(s, plan, &steps, now);
        let mut combined = return_steps;
        combined.extend(steps);
        combined.extend(audit_steps);
        Remaining {
            steps: combined,
            short_of_time,
            run_limited,
        }
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
                        if b.push(Step::new(&r.kc, Phase::Review, false, r.seconds))
                            == PushResult::Added
                        {
                            return true;
                        }
                    }
                    false
                };
                let mut try_contrast = |b: &mut Builder<'_, 'a>, left: &mut f64| {
                    for k in 0..contrasts.len() {
                        let c = contrasts[(next_contrast + k) % contrasts.len()];
                        let seconds = self.mean_seconds(c);
                        let production = !b.has_production(c);
                        let step = Step::new(c, Phase::Interleaved, production, seconds);
                        if !b.available(&step) {
                            continue;
                        }
                        if f64::from(seconds) > *left {
                            b.short_of_time = true;
                            continue;
                        }
                        if b.push(step) == PushResult::Added {
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
                    let step = Step::new(kc, Phase::Interleaved, false, seconds);
                    if !b.available(&step) {
                        continue;
                    }
                    if f64::from(seconds) > *left {
                        b.short_of_time = true;
                        continue;
                    }
                    if b.push(step) == PushResult::Added {
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
            .filter(|d| d.step.phase != Phase::Final && !d.step.phase.is_probe())
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
