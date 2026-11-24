//! Synthetic content and learners for testing the learner model without
//! real content: a bundle shaped like the real one, a log generator driven by
//! a hidden "true" skill, and small helpers to write events by hand.
//!
//! Everything is a function of a seed through the core PRNG, so the same
//! arguments give the same bundle and log on every platform: the native and
//! WASM folds of one generated log must hash alike.

// Test support: every `expect` here guards a literal the generator builds,
// so a panic is a bug in this crate, not a condition for callers.
#![allow(clippy::missing_panics_doc)]

pub mod cohort;

use std::collections::BTreeMap;

use refl_core::answer::dfa::{Dfa, TokenTable};
use refl_core::answer::normalize::NORMALIZATION_VERSION;
use refl_core::bundle::{
    BUNDLE_FORMAT_VERSION, Bundle, BundleContent, ItemEntry, KcEntry, KcItems, KcStatus, Required,
    SlotAutomata,
};
use refl_core::content::{
    Edge, EdgeKind, Explanation, ExplanationDepth, ExplanationTarget, Kc, L1Prior, L1Set,
    LangReviews, Misconception, MisconceptionOrigin, Review, ReviewStatus,
};
use refl_core::event::{
    AnswerContext, AnswerEvent, CURRENT_SCHEMA_VERSION, Confidence, Decoded, Event, EventPayload,
    PlanAction, PlanCandidate, PlanIssuedEvent, PlanScope, ReclassifiedEvent, Response,
    UnknownEvent, Verdict,
};
use refl_core::ids::{
    ContentVersion, DomainId, EventId, ExplanationId, FamilyId, InstructionId, ItemId, ItemRef,
    ItemVersion, KcId, LearnerId, LessonId, MisconceptionId, SubjectId,
};
use refl_core::item::{
    AnswerPattern, AuthorDifficulty, ChoiceBody, ChoiceOption, EnglishCheck, ExpectedWrong,
    GapFillBody, Item, ItemBody, ItemStatus, KcRole, Slot, SlotKc, Stimulus,
};
use refl_core::kind::ItemKind;
use refl_core::lexicon::LexiconRef;
use refl_core::model::math::sigmoid;
use refl_core::projection::{ItemProjection, ItemVersionEntry};
use refl_core::rng::SplitMix64;
use refl_core::subject::{Checker, LevelKey, ScaleDef, ScaleLevel, Subject};
use refl_core::value::{Finite, Localized, Seed, Timestamp, UnitInterval};
use uuid::Uuid;

pub const SUBJECT: &str = "en";
pub const SPELLING_KC: &str = "en.orth.spelling";
/// 2025-10-01T00:00:00Z: logs start here.
pub const T0: Timestamp = Timestamp(1_759_276_800_000);
pub const DAY_MS: i64 = 86_400_000;

fn kc(s: &str) -> KcId {
    KcId::new(s).expect("valid KC id")
}

/// Id of synthetic KC `i`: `en.gram.k00`, `en.gram.k01`, …
pub fn kc_id(i: usize) -> KcId {
    kc(&format!("en.gram.k{i:02}"))
}

/// Id of item `j` of KC `i`. Even `j`: choice; odd `j`: gap fill.
pub fn item_id(i: usize, j: usize) -> ItemId {
    ItemId::new(format!("en.item.k{i:02}.{j:02}")).expect("valid item id")
}

pub fn item_ref(i: usize, j: usize) -> ItemRef {
    ItemRef {
        id: item_id(i, j),
        version: ItemVersion(1),
        seed: None,
    }
}

/// Misconception `x` (0 or 1) of KC `i`.
pub fn misconception_id(i: usize, x: usize) -> MisconceptionId {
    MisconceptionId::new(format!("en.mis.k{i:02}.{x}")).expect("valid misconception id")
}

pub fn explanation_id(i: usize) -> ExplanationId {
    ExplanationId::new(format!("en.expl.k{i:02}")).expect("valid explanation id")
}

/// Shape of a synthetic bundle.
#[derive(Debug, Clone, Copy)]
pub struct BundleSpec {
    pub kcs: usize,
    pub items_per_kc: usize,
}

/// Grammar KCs `en.gram.kNN` in chains of four (`k(i)` requires `k(i−1)`
/// unless `i % 4 == 0`) with an extra edge to `k(i−2)` when `i % 5 == 0`,
/// and `k(i)` contrasting with `k(i−1)` when `i % 4 == 2`; level rank
/// `1 + i % 3` (A1–B1), all live. Per KC: two misconceptions, a short
/// explanation, and items alternating choice (three options: two diagnosed,
/// one noise) and gap fill (two expected errors); every fifth item also
/// needs the previous KC as secondary. Plus the spelling KC. Every item is
/// session-eligible and indexed by KC.
#[allow(clippy::too_many_lines)] // one literal bundle
pub fn synthetic_bundle(spec: BundleSpec) -> Bundle {
    let subject_id = SubjectId::new(SUBJECT).expect("valid subject");
    let grammar = DomainId::new("grammar").expect("valid domain");
    let orthography = DomainId::new("orthography").expect("valid domain");
    let levels = ["pre-A1", "A1", "A2", "B1", "B2", "C1", "C2"];
    let subject = Subject {
        id: subject_id.clone(),
        title: Localized::default(),
        scale: ScaleDef {
            levels: levels
                .iter()
                .map(|k| ScaleLevel {
                    key: LevelKey::new(*k).expect("valid level"),
                    title: Localized::default(),
                })
                .collect(),
            step_logit: Finite::new(1.2).expect("finite"),
        },
        domains: vec![grammar.clone(), orthography.clone()],
        checker: Checker::EnWords,
        spelling_kc: kc(SPELLING_KC),
    };
    let kc_entry = |id: KcId, domain: &DomainId, rank: u16, importance: f64| KcEntry {
        kc: Kc {
            id,
            subject: subject_id.clone(),
            domain: domain.clone(),
            title: Localized::default(),
            review: LangReviews::default(),
            level: subject.scale.levels[usize::from(rank)].key.clone(),
            importance: UnitInterval::new(importance).expect("unit"),
            rule: String::new(),
            tags: Vec::new(),
        },
        status: KcStatus::Live,
        level_rank: rank,
    };

    let mut kcs = vec![kc_entry(kc(SPELLING_KC), &orthography, 1, 0.5)];
    let mut edges = Vec::new();
    let mut misconceptions = Vec::new();
    let mut explanations = Vec::new();
    let mut items = Vec::new();
    for i in 0..spec.kcs {
        let rank = 1 + u16::try_from(i % 3).expect("small");
        #[allow(clippy::cast_precision_loss)] // small
        let importance = 0.3 + 0.1 * (i % 7) as f64;
        kcs.push(kc_entry(kc_id(i), &grammar, rank, importance));
        let mut requires = |to: usize| {
            edges.push(Edge {
                kind: EdgeKind::Requires,
                from: kc_id(i),
                to: kc_id(to),
            });
        };
        if i % 4 != 0 {
            requires(i - 1);
        }
        if i % 5 == 0 && i >= 2 {
            requires(i - 2);
        }
        if i % 4 == 2 {
            edges.push(Edge {
                kind: EdgeKind::Contrasts,
                from: kc_id(i),
                to: kc_id(i - 1),
            });
        }
        for x in 0..2 {
            #[allow(clippy::cast_precision_loss)] // small
            let p = 0.1 + 0.05 * ((i + x) % 5) as f64;
            misconceptions.push(Misconception {
                id: misconception_id(i, x),
                title: Localized::default(),
                review: LangReviews::default(),
                affects: vec![kc_id(i)],
                origin: MisconceptionOrigin::L1Transfer,
                l1: L1Set { uz: true, ru: true },
                prior: L1Prior {
                    uz: UnitInterval::new(p).expect("unit"),
                    ru: UnitInterval::new(p / 2.0).expect("unit"),
                },
                remedy: kc_id(i.saturating_sub(1)),
            });
        }
        explanations.push(Explanation {
            id: explanation_id(i),
            target: ExplanationTarget::Kc(kc_id(i)),
            depth: ExplanationDepth::Short,
            lang: refl_core::value::Lang::En,
            body: String::new(),
            l1_contrast: None,
            examples: Vec::new(),
            review: Review {
                status: ReviewStatus::Draft,
                by: None,
                hash: None,
            },
        });
        for j in 0..spec.items_per_kc {
            let mut slot_kcs = vec![SlotKc {
                kc: kc_id(i),
                role: KcRole::Primary,
            }];
            if j % 5 == 4 && i > 0 {
                slot_kcs.push(SlotKc {
                    kc: kc_id(i - 1),
                    role: KcRole::Secondary,
                });
            }
            let slot = Slot {
                kcs: slot_kcs,
                answer: "the".into(),
                accepted: vec![AnswerPattern("the".into())],
                options: if j % 2 == 0 {
                    vec![
                        ChoiceOption {
                            text: "a".into(),
                            misconception: Some(misconception_id(i, 0)),
                            confuses_with: None,
                        },
                        ChoiceOption {
                            text: "an".into(),
                            misconception: Some(misconception_id(i, 1)),
                            confuses_with: None,
                        },
                        ChoiceOption {
                            text: "this".into(),
                            misconception: None,
                            confuses_with: None,
                        },
                    ]
                } else {
                    Vec::new()
                },
                expected_wrong: if j % 2 == 0 {
                    Vec::new()
                } else {
                    (0..2)
                        .map(|x| ExpectedWrong {
                            pattern: AnswerPattern(["a", "an"][x].into()),
                            misconception: misconception_id(i, x),
                            confuses_with: None,
                        })
                        .collect()
                },
                hint: None,
                why: Localized::default(),
            };
            let stimulus = Stimulus {
                text: "I saw ___ cat.".into(),
                audio: None,
                image: None,
            };
            let body = if j % 2 == 0 {
                ItemBody::Choice(ChoiceBody {
                    stimulus,
                    slots: vec![slot],
                })
            } else {
                ItemBody::GapFill(GapFillBody {
                    stimulus,
                    slots: vec![slot],
                    check: EnglishCheck::default(),
                })
            };
            let item = Item {
                id: item_id(i, j),
                version: ItemVersion(1),
                subject: subject_id.clone(),
                family: FamilyId::new(format!("en.fam.k{i:02}.{}", j / 4)).expect("valid family"),
                instruction: InstructionId::new("en.instr.gap").expect("valid instruction"),
                difficulty: AuthorDifficulty::new(1 + u8::try_from(j % 5).expect("small"))
                    .expect("1..=5"),
                expected_time: 8 + 2 * u32::try_from(j % 4).expect("small"),
                topic: None,
                status: ItemStatus::Live,
                source: None,
                license: None,
                breaks_calibration: false,
                review: LangReviews::default(),
                review_group: None,
                body,
            };
            let tokens = TokenTable::default();
            let automata = SlotAutomata {
                accepted: Dfa::compile(&[], &tokens).expect("empty automaton"),
                expected_wrong: Vec::new(),
            };
            items.push(ItemEntry {
                item,
                session_eligible: true,
                slots: vec![automata],
            });
        }
    }
    kcs.sort_by(|a, b| a.kc.id.cmp(&b.kc.id));
    edges.sort();
    misconceptions.sort_by(|a, b| a.id.cmp(&b.id));
    explanations.sort_by(|a, b| a.id.cmp(&b.id));
    items.sort_by(|a, b| a.item.id.cmp(&b.item.id));
    let item_versions = items
        .iter()
        .map(|e| ItemVersionEntry {
            item: e.item.id.clone(),
            version: e.item.version,
            projection: ItemProjection::of(&e.item),
        })
        .collect();
    let requires_closure = closure(&kcs, &edges);
    let mut bundle = Bundle {
        format_version: BUNDLE_FORMAT_VERSION,
        content_version: ContentVersion("synthetic".into()),
        content: BundleContent {
            normalization_version: NORMALIZATION_VERSION,
            lexicon: LexiconRef {
                version: "0".repeat(64),
                words: 0,
                bytes: 0,
            },
            subjects: vec![subject],
            kcs,
            edges,
            requires_closure,
            misconceptions,
            explanations,
            instructions: Vec::new(),
            lessons: Vec::new(),
            items,
            item_versions,
            items_by_kc: BTreeMap::new(),
            tokens: TokenTable::default(),
        },
    };
    // Synthetic texts are empty and never reviewed.
    reindex(&mut bundle, TextReview::Ignored);
    bundle
}

/// Whether session eligibility needs the item's texts reviewed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextReview {
    /// As content-build decides: `why` and `hint` reviewed in ru and en.
    Required,
    /// Test fixtures only: unreviewed items count as reviewed.
    Ignored,
}

/// Session eligibility and the KC index of the items, as content-build
/// computes them from KC statuses (`docs/02-layer1-content.md`, «Вычитка и
/// показ»), except that `review` can be ignored for a test fixture.
pub fn reindex(bundle: &mut Bundle, review: TextReview) {
    let status: BTreeMap<KcId, KcStatus> = bundle
        .content
        .kcs
        .iter()
        .map(|e| (e.kc.id.clone(), e.status))
        .collect();
    let mut by_kc: BTreeMap<KcId, KcItems> = BTreeMap::new();
    for entry in &mut bundle.content.items {
        let item = &entry.item;
        let slots = item.body.slots();
        let primary_live = slots
            .iter()
            .flat_map(|s| &s.kcs)
            .filter(|k| k.role == KcRole::Primary)
            .all(|k| status.get(&k.kc) == Some(&KcStatus::Live));
        let reviewed = review == TextReview::Ignored || item.is_reviewed();
        entry.session_eligible = item.status == ItemStatus::Live && primary_live && reviewed;
        if entry.session_eligible {
            let r = ItemRef {
                id: item.id.clone(),
                version: item.version,
                seed: None,
            };
            for sk in slots.iter().flat_map(|s| &s.kcs) {
                let refs = by_kc.entry(sk.kc.clone()).or_default();
                match sk.role {
                    KcRole::Primary => refs.primary.push(r.clone()),
                    KcRole::Secondary => refs.secondary.push(r.clone()),
                }
            }
        }
    }
    for refs in by_kc.values_mut() {
        for list in [&mut refs.primary, &mut refs.secondary] {
            list.sort();
            list.dedup();
        }
    }
    bundle.content.items_by_kc = by_kc;
}

/// The bundle of `content/` for simulations: every KC live, review ignored.
/// The Articles unit is draft until a person reviews its texts (docs/15),
/// so only a test fixture may treat it as live.
pub fn live_fixture(mut bundle: Bundle) -> Bundle {
    for e in &mut bundle.content.kcs {
        e.status = KcStatus::Live;
    }
    reindex(&mut bundle, TextReview::Ignored);
    bundle
}

/// Breadth-first shortest distances over `requires`, as content-build does.
fn closure(kcs: &[KcEntry], edges: &[Edge]) -> BTreeMap<KcId, Vec<Required>> {
    let mut out = BTreeMap::new();
    for e in kcs {
        let mut distance: BTreeMap<&KcId, u16> = BTreeMap::new();
        let mut queue = std::collections::VecDeque::from([(&e.kc.id, 0u16)]);
        while let Some((k, d)) = queue.pop_front() {
            for edge in edges
                .iter()
                .filter(|x| x.kind == EdgeKind::Requires && &x.from == k)
            {
                if !distance.contains_key(&edge.to) {
                    distance.insert(&edge.to, d + 1);
                    queue.push_back((&edge.to, d + 1));
                }
            }
        }
        let required = distance
            .into_iter()
            .map(|(kc, distance)| Required {
                kc: kc.clone(),
                distance,
            })
            .collect();
        out.insert(e.kc.id.clone(), required);
    }
    out
}

/// The learner of every synthetic log.
pub fn learner() -> LearnerId {
    LearnerId(Uuid::from_u128(0x0193_0000_0000_7000_8000_0000_0000_0001))
}

/// A `UUIDv7` event id at `at` with `tail` as its random part: ids sort by time
/// first, then by `tail`.
pub fn event_id(at: Timestamp, tail: u64) -> EventId {
    let ms = u64::try_from(at.0).expect("time after the epoch");
    let mut b = [0u8; 16];
    b[..6].copy_from_slice(&ms.to_be_bytes()[2..]);
    b[8..].copy_from_slice(&tail.to_be_bytes());
    b[6] = 0x70; // version 7
    b[8] = 0x80 | (b[8] & 0x3F); // RFC 4122 variant
    EventId::new(Uuid::from_bytes(b)).expect("UUIDv7")
}

/// A known event of the synthetic learner at `at`.
pub fn event(at: Timestamp, tail: u64, payload: EventPayload) -> Decoded {
    Decoded::Known(Event {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: event_id(at, tail),
        learner_id: learner(),
        session_id: None,
        client_ts: at,
        server_ts: None,
        payload,
    })
}

/// Milliseconds of the latest known event of a log; [`T0`] for none.
pub fn event_time(log: &[Decoded]) -> i64 {
    log.iter()
        .filter_map(|e| match e {
            Decoded::Known(e) => Some(e.client_ts.0),
            Decoded::Unknown(_) => None,
        })
        .max()
        .unwrap_or(T0.0)
}

/// An event from a newer core: a payload type this core does not know.
pub fn unknown_event(at: Timestamp, tail: u64) -> Decoded {
    let id = event_id(at, tail);
    Decoded::Unknown(UnknownEvent {
        schema_version: u64::from(CURRENT_SCHEMA_VERSION),
        event_id: id,
        learner_id: learner(),
        raw: format!(
            r#"{{"schema_version":{CURRENT_SCHEMA_VERSION},"event_id":"{}","learner_id":"{}","session_id":null,"client_ts":{},"server_ts":null,"payload":{{"type":"from_the_future"}}}}"#,
            id.as_uuid(),
            learner().0,
            at.0
        ),
    })
}

/// An answer on slot 0 of `item`: first attempt, no hints, no confidence,
/// in the foreground.
pub fn answer(item: ItemRef, verdict: Verdict, response: Response, latency_ms: u32) -> AnswerEvent {
    AnswerEvent {
        item,
        slot: 0,
        scaffold: 0,
        revealed_deps: false,
        context: AnswerContext::Practice,
        response,
        verdict,
        misconception: None,
        latency_ms,
        backgrounded: false,
        confidence: None,
        hints: 0,
        attempt: 1,
    }
}

/// Shape of a synthetic log.
#[derive(Debug, Clone, Copy)]
pub struct LogSpec {
    pub events: usize,
    pub seed: u64,
}

/// A learner with a hidden skill per KC that grows with practice and fades
/// over breaks, studying in daily sessions with a long break now and then.
/// Besides answers of every verdict, confidence, hint, attempt, rapid and
/// background variant, the log has explanation views, lesson markers,
/// planner decisions, reclassifications of earlier answers and events from a
/// newer core.
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)] // one generator; small counts
pub fn synthetic_log(bundle: &Bundle, spec: LogSpec) -> Vec<Decoded> {
    let mut rng = SplitMix64::new(spec.seed);
    let grammar: Vec<&KcEntry> = bundle
        .content
        .kcs
        .iter()
        .filter(|e| e.kc.domain.as_str() == "grammar")
        .collect();
    let mut skill: Vec<f64> = grammar.iter().map(|_| rng.next_normal()).collect();
    let mut log = Vec::with_capacity(spec.events);
    let mut answers: Vec<EventId> = Vec::new();
    let mut now = T0.0;
    let mut in_session = 0u32;
    let mut focus = 0usize;
    while log.len() < spec.events {
        // Sessions of ~30 events a day; every ~40 sessions a 20-day break.
        if in_session == 0 {
            let gap_days = if one_in(&mut rng, 40) { 20 } else { 1 };
            now += gap_days * DAY_MS + i64::try_from(rng.next_u64() % 3_600_000).expect("small");
            for s in &mut skill {
                *s -= 0.02 * gap_days as f64;
            }
            in_session = 20 + u32::try_from(rng.next_u64() % 20).expect("small");
            focus = pick(&mut rng, grammar.len());
        }
        in_session -= 1;
        now += 5_000 + i64::try_from(rng.next_u64() % 25_000).expect("small");
        let at = Timestamp(now);
        let tail = rng.next_u64();
        let roll = rng.next_u64() % 100;
        if roll < 4 {
            let i = grammar[focus].kc.id.as_str()[9..].parse().expect("kNN");
            log.push(event(
                at,
                tail,
                EventPayload::ExplanationViewed {
                    explanation: explanation_id(i),
                    depth: ExplanationDepth::Short,
                    seconds: 40,
                },
            ));
            continue;
        }
        if roll < 6 {
            log.push(event(
                at,
                tail,
                EventPayload::LessonStarted {
                    lesson: LessonId::new("en.lesson.x").expect("valid lesson"),
                },
            ));
            continue;
        }
        if roll < 8 {
            log.push(event(at, tail, plan_issued(&grammar[focus].kc.id, tail)));
            continue;
        }
        if roll < 9 {
            log.push(unknown_event(at, tail));
            continue;
        }
        if roll < 11 && !answers.is_empty() {
            let original = answers[pick(&mut rng, answers.len())];
            log.push(event(
                at,
                tail,
                EventPayload::Reclassified(ReclassifiedEvent {
                    original,
                    verdict: Verdict::Correct,
                    misconception: None,
                    content_version: ContentVersion("synthetic".into()),
                }),
            ));
            continue;
        }
        if one_in(&mut rng, 5) {
            focus = pick(&mut rng, grammar.len());
        }
        let kc = &grammar[focus].kc.id;
        let candidates: Vec<&ItemEntry> = bundle
            .content
            .items
            .iter()
            .filter(|e| e.item.body.slots()[0].kcs[0].kc == *kc)
            .collect();
        let entry = candidates[pick(&mut rng, candidates.len())];
        let item = &entry.item;
        let slot = &item.body.slots()[0];
        let b = f64::from(item.difficulty.get()) - 3.0;
        let p_star: f64 = slot
            .kcs
            .iter()
            .map(|k| {
                let ix = grammar
                    .iter()
                    .position(|e| e.kc.id == k.kc)
                    .expect("grammar KC");
                sigmoid(skill[ix] - b)
            })
            .product();
        let g = item.body.guessing(0);
        let correct = rng.next_f64() < g + (1.0 - g - 0.05) * p_star;
        skill[focus] += 0.03;
        let typed = item.body.typed_input();
        let (verdict, misconception, response) = if correct {
            let text = if one_in(&mut rng, 10) { "it's" } else { "the" };
            let response = if typed {
                Response::Text(text.into())
            } else {
                Response::Option(0)
            };
            (Verdict::Correct, None, response)
        } else {
            let x = usize::try_from(rng.next_u64() % 3).expect("small");
            let mis = (x < 2).then(|| slot_misconception(slot, x)).flatten();
            match (typed, rng.next_u64() % 10) {
                (true, 0) => (Verdict::Typo, None, Response::Text("teh".into())),
                (true, 1) => (Verdict::Unrecognized, None, Response::Text("x1".into())),
                (true, _) => (Verdict::Wrong, mis, Response::Text("a".into())),
                (false, _) => (
                    Verdict::Wrong,
                    mis,
                    Response::Option(1 + u16::try_from(x).expect("small")),
                ),
            }
        };
        let expected = f64::from(item.expected_time) * 1000.0;
        let latency = if one_in(&mut rng, 20) {
            0.1 * expected
        } else {
            expected * libm::exp(0.5 * rng.next_normal())
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // positive, bounded
        let latency_ms = latency.min(600_000.0) as u32;
        let mut a = answer(
            ItemRef {
                id: item.id.clone(),
                version: item.version,
                seed: None,
            },
            verdict,
            response,
            latency_ms,
        );
        a.misconception = misconception;
        a.context = match rng.next_u64() % 4 {
            0 | 1 => AnswerContext::Lesson,
            2 => AnswerContext::Review,
            _ => AnswerContext::Practice,
        };
        a.backgrounded = one_in(&mut rng, 30);
        a.confidence = match rng.next_u64() % 3 {
            0 => None,
            1 => Some(Confidence::Sure),
            _ => Some(Confidence::Unsure),
        };
        a.hints = u8::from(one_in(&mut rng, 10));
        a.attempt = if one_in(&mut rng, 10) { 2 } else { 1 };
        let ev = event(at, tail, EventPayload::Answer(a));
        if let Decoded::Known(e) = &ev {
            answers.push(e.event_id);
        }
        log.push(ev);
    }
    log
}

fn slot_misconception(slot: &Slot, x: usize) -> Option<MisconceptionId> {
    slot.options
        .get(x)
        .and_then(|o| o.misconception.clone())
        .or_else(|| slot.expected_wrong.get(x).map(|w| w.misconception.clone()))
}

fn plan_issued(kc: &KcId, tail: u64) -> EventPayload {
    EventPayload::PlanIssued(PlanIssuedEvent {
        planner_version: "synthetic".into(),
        model_version: "synthetic".into(),
        scope: PlanScope::NextItem,
        candidates: vec![PlanCandidate {
            action: PlanAction::Learn { kc: kc.clone() },
            value_per_minute: Finite::new(0.5).expect("finite"),
            factors: Vec::new(),
        }],
        chosen: 0,
        propensity: UnitInterval::new(1.0).expect("unit"),
        seed: Seed::new(tail >> 11).expect("53 bits"),
        violations: Vec::new(),
        dominant: "gap".into(),
        trigger: None,
    })
}

/// True with probability `1/n`.
fn one_in(rng: &mut SplitMix64, n: u64) -> bool {
    rng.next_u64().is_multiple_of(n)
}

/// Uniform index below `n`, reduced in 64 bits: `usize` is 32 bits on wasm32.
fn pick(rng: &mut SplitMix64, n: usize) -> usize {
    let n = u64::try_from(n).expect("usize fits u64");
    usize::try_from(rng.next_u64() % n).expect("below n")
}
