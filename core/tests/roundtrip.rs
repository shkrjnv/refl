//! Serialization round trips and byte-level determinism for the log and content types.

use proptest::prelude::*;
use refl_core::content::{
    Edge, EdgeKind, Kc, L1Prior, L1Set, LangReviews, Misconception, MisconceptionOrigin,
};
use refl_core::event::{
    AnswerContext, AnswerEvent, Confidence, CriterionScore, EventPayload, HalfBands, InputDevice,
    MentorOverrideEvent, OverrideKind, OverrideReason, PlanAction, PlanCandidate, PlanFactor,
    PlanIssuedEvent, PlanScope, ReclassifiedEvent, Response, Verdict, WritingScoredEvent,
};
use refl_core::ids::{
    AchievementId, ContentVersion, DomainId, EventId, ExplanationId, ItemId, ItemRef, ItemVersion,
    KcId, LearnerId, LessonId, MisconceptionId, PackageKey, SessionId, SubjectId, Tag, UserId,
};
use refl_core::item::{
    AnswerPattern, AuthorDifficulty, ChoiceBody, ChoiceOption, Contractions, DictationBody,
    EnglishCheck, ExpectedWrong, FindErrorBody, GapFillBody, Item, ItemBody, ItemStatus, KcRole,
    MatchingBody, OwnSentenceBody, SentenceAssemblyBody, ShadowingBody, Slot, SlotKc, Stimulus,
    TransformationBody, TranslationBody, TrueFalseNotGivenBody, WritingBody,
};
use refl_core::subject::{Checker, LevelKey, ScaleDef, ScaleLevel, Subject};
use refl_core::unlock::{
    AchievementTier, CosmeticSlot, Milestone, ShowcaseKeys, ShowcaseKind, UnlockKind, UnlockSource,
};
use refl_core::value::{Cefr, Finite, Localized, MAX_SAFE_INTEGER, Seed, Timestamp, UnitInterval};
use refl_core::{
    CURRENT_SCHEMA_VERSION, DecodeError, Decoded, Event, UnknownEvent, decode_event, encode_event,
};
use uuid::Uuid;

fn slug<T: std::fmt::Debug>(make: fn(String) -> Option<T>) -> impl Strategy<Value = T> {
    "[a-z][a-z0-9]{0,6}(\\.[a-z0-9]{1,6}){0,2}(-[a-z0-9]{1,4})?".prop_map(move |s| make(s).unwrap())
}

fn uuid_v7() -> impl Strategy<Value = EventId> {
    any::<u128>().prop_map(|bits| {
        // Set version 7 and RFC 4122 variant bits.
        let b = (bits & !(0xF << 76) & !(0b11 << 62)) | (0x7 << 76) | (0b10 << 62);
        EventId::new(Uuid::from_u128(b)).unwrap()
    })
}

fn text() -> impl Strategy<Value = String> {
    // Arbitrary Unicode, including quotes, escapes and non-BMP characters.
    any::<String>()
}

fn localized() -> impl Strategy<Value = Localized> {
    (
        proptest::option::of(text()),
        proptest::option::of(text()),
        proptest::option::of(text()),
    )
        .prop_map(|(uz, ru, en)| Localized { uz, ru, en })
}

fn unit() -> impl Strategy<Value = UnitInterval> {
    (0.0..=1.0f64).prop_map(|v| UnitInterval::new(v).unwrap())
}

fn verdict() -> impl Strategy<Value = Verdict> {
    prop_oneof![
        Just(Verdict::Correct),
        Just(Verdict::Typo),
        Just(Verdict::Wrong),
        Just(Verdict::Unrecognized)
    ]
}

/// Any finite float, including zeros, subnormals and the extremes.
fn finite() -> impl Strategy<Value = Finite> {
    prop_oneof![
        Just(f64::MAX),
        Just(f64::MIN),
        Just(-0.0),
        any::<f64>().prop_filter("finite", |v| v.is_finite())
    ]
    .prop_map(|v| Finite::new(v).unwrap())
}

/// The whole 53-bit range, edges included.
fn seed() -> impl Strategy<Value = Seed> {
    prop_oneof![Just(MAX_SAFE_INTEGER), 0..=MAX_SAFE_INTEGER].prop_map(|s| Seed::new(s).unwrap())
}

fn timestamp() -> impl Strategy<Value = Timestamp> {
    let max = i64::try_from(MAX_SAFE_INTEGER).unwrap();
    prop_oneof![Just(-max), Just(max), -max..=max].prop_map(Timestamp)
}

fn item_ref() -> impl Strategy<Value = ItemRef> {
    (
        slug(ItemId::new),
        any::<u32>(),
        proptest::option::of(seed()),
    )
        .prop_map(|(id, v, seed)| ItemRef {
            id,
            version: ItemVersion(v),
            seed,
        })
}

/// The timing fields of an answer, schema 2: first input, typing,
/// characters, edits, device, hidden time.
#[allow(clippy::type_complexity)]
fn timing() -> impl Strategy<
    Value = (
        Option<u32>,
        Option<u32>,
        Option<u32>,
        Option<u32>,
        Option<InputDevice>,
        Option<u32>,
    ),
> {
    (
        any::<Option<u32>>(),
        any::<Option<u32>>(),
        any::<Option<u32>>(),
        any::<Option<u32>>(),
        proptest::option::of(prop_oneof![
            Just(InputDevice::Touch),
            Just(InputDevice::Keyboard)
        ]),
        any::<Option<u32>>(),
    )
}

fn payload() -> impl Strategy<Value = EventPayload> {
    let answer = (
        (item_ref(), any::<u16>(), any::<u8>(), any::<bool>()),
        prop_oneof![
            Just(AnswerContext::Lesson),
            Just(AnswerContext::Review),
            Just(AnswerContext::Placement),
            Just(AnswerContext::Practice),
            Just(AnswerContext::Mock),
            Just(AnswerContext::Audit),
            Just(AnswerContext::ReturnCheck)
        ],
        prop_oneof![
            any::<u16>().prop_map(Response::Option),
            text().prop_map(Response::Text)
        ],
        verdict(),
        proptest::option::of(slug(MisconceptionId::new)),
        (any::<u32>(), timing()),
        proptest::option::of(prop_oneof![
            Just(Confidence::Sure),
            Just(Confidence::Unsure)
        ]),
        (any::<u8>(), any::<u8>()),
    )
        .prop_map(
            |(
                (item, slot, scaffold, revealed_deps),
                context,
                response,
                verdict,
                misconception,
                (
                    latency_ms,
                    (first_input_ms, typing_ms, chars_typed, edits, input_device, hidden_ms),
                ),
                confidence,
                (hints, attempt),
            )| {
                EventPayload::Answer(AnswerEvent {
                    item,
                    slot,
                    scaffold,
                    revealed_deps,
                    context,
                    response,
                    verdict,
                    misconception,
                    latency_ms,
                    first_input_ms,
                    typing_ms,
                    chars_typed,
                    edits,
                    input_device,
                    hidden_ms,
                    confidence,
                    hints,
                    attempt,
                })
            },
        );
    let reclassified = (
        uuid_v7(),
        verdict(),
        proptest::option::of(slug(MisconceptionId::new)),
        text(),
    )
        .prop_map(|(original, verdict, misconception, v)| {
            EventPayload::Reclassified(ReclassifiedEvent {
                original,
                verdict,
                misconception,
                content_version: ContentVersion(v),
            })
        });
    prop_oneof![
        answer,
        reclassified,
        slug(LessonId::new).prop_map(|lesson| EventPayload::LessonStarted { lesson }),
        slug(LessonId::new).prop_map(|lesson| EventPayload::LessonFinished { lesson }),
        (slug(ExplanationId::new), any::<u32>()).prop_map(|(explanation, seconds)| {
            EventPayload::ExplanationViewed {
                explanation,
                depth: refl_core::content::ExplanationDepth::Why,
                seconds,
            }
        }),
        reward_payload(),
        quality_payload(),
    ]
}

fn user() -> impl Strategy<Value = UserId> {
    any::<u128>().prop_map(|u| UserId(Uuid::from_u128(u)))
}

/// `plan_issued`, `mentor_override`, `writing_scored` (docs/11).
#[allow(clippy::too_many_lines)] // one strategy for each payload, in order
fn quality_payload() -> impl Strategy<Value = EventPayload> {
    let action = prop_oneof![
        slug(KcId::new).prop_map(|kc| PlanAction::Learn { kc }),
        slug(KcId::new).prop_map(|kc| PlanAction::Review { kc }),
        (item_ref(), any::<u16>()).prop_map(|(item, slot)| PlanAction::Item { item, slot }),
        Just(PlanAction::Break),
    ];
    let candidate = (
        action,
        finite(),
        proptest::collection::vec(
            (text(), finite()).prop_map(|(name, value)| PlanFactor { name, value }),
            0..4,
        ),
    )
        .prop_map(|(action, value_per_minute, factors)| PlanCandidate {
            action,
            value_per_minute,
            factors,
        });
    let plan = (
        (text(), text()),
        prop_oneof![Just(PlanScope::Session), Just(PlanScope::NextItem)],
        proptest::collection::vec(candidate, 1..5),
        (any::<u16>(), unit(), seed()),
        proptest::collection::vec(text(), 0..3),
        (
            text(),
            proptest::option::of(text()),
            proptest::option::of(text()),
        ),
    )
        .prop_map(
            |(
                (planner_version, model_version),
                scope,
                candidates,
                (chosen, propensity, seed),
                violations,
                (dominant, trigger, end),
            )| {
                // A valid plan chooses one of its candidates.
                let chosen = chosen % u16::try_from(candidates.len()).unwrap();
                EventPayload::PlanIssued(PlanIssuedEvent {
                    planner_version,
                    model_version,
                    scope,
                    candidates,
                    chosen,
                    propensity,
                    seed,
                    violations,
                    dominant,
                    trigger,
                    end,
                })
            },
        );
    let mentor = (
        user(),
        prop_oneof![
            uuid_v7().prop_map(|plan| OverrideKind::PlanChange { plan }),
            (slug(KcId::new), unit())
                .prop_map(|(kc, model_mastery)| OverrideKind::MarkKnown { kc, model_mastery }),
            uuid_v7().prop_map(|event| OverrideKind::DisputeVerdict { event }),
        ],
        prop_oneof![
            Just(OverrideReason::KnowsElsewhere),
            Just(OverrideReason::TooHard),
            Just(OverrideReason::TooEasy),
            Just(OverrideReason::WrongVerdict),
            Just(OverrideReason::Other),
        ],
        proptest::option::of(text()),
    )
        .prop_map(|(mentor, kind, reason, note)| {
            EventPayload::MentorOverride(MentorOverrideEvent {
                mentor,
                kind,
                reason,
                note,
            })
        });
    let writing = (
        item_ref(),
        any::<u16>(),
        user(),
        proptest::collection::vec(
            (text(), 0u8..=18).prop_map(|(criterion, h)| CriterionScore {
                criterion,
                half_bands: HalfBands::new(h).unwrap(),
            }),
            0..5,
        ),
        any::<bool>(),
    )
        .prop_map(|(item, slot, scorer, scores, blind)| {
            EventPayload::WritingScored(WritingScoredEvent {
                item,
                slot,
                scorer,
                scores,
                blind,
            })
        });
    prop_oneof![plan, mentor, writing]
}

fn milestone() -> impl Strategy<Value = Milestone> {
    prop_oneof![
        prop_oneof![
            Just(Cefr::PreA1),
            Just(Cefr::A1),
            Just(Cefr::B2),
            Just(Cefr::C2)
        ]
        .prop_map(Milestone::Level),
        slug(KcId::new).prop_map(Milestone::Topic),
    ]
}

fn unlock_kind() -> impl Strategy<Value = UnlockKind> {
    prop_oneof![
        Just(UnlockKind::Theme),
        Just(UnlockKind::Companion),
        Just(UnlockKind::Avatar),
        Just(UnlockKind::Frame),
        Just(UnlockKind::Background),
        Just(UnlockKind::Title),
        Just(UnlockKind::Effect)
    ]
}

fn tier() -> impl Strategy<Value = AchievementTier> {
    (1u8..=6).prop_map(|t| AchievementTier::new(t).unwrap())
}

fn cosmetic_slot() -> impl Strategy<Value = CosmeticSlot> {
    prop_oneof![
        Just(CosmeticSlot::Avatar),
        Just(CosmeticSlot::Frame),
        Just(CosmeticSlot::Background),
        Just(CosmeticSlot::Title),
        Just(CosmeticSlot::Effect)
    ]
}

/// Milestones, achievements, unlocks and selections (docs/07).
fn reward_payload() -> impl Strategy<Value = EventPayload> {
    let source = prop_oneof![
        milestone().prop_map(|milestone| UnlockSource::Milestone { milestone }),
        (slug(AchievementId::new), tier())
            .prop_map(|(achievement, tier)| UnlockSource::Achievement { achievement, tier }),
        any::<u128>().prop_map(|u| UnlockSource::Admin {
            by: UserId(Uuid::from_u128(u))
        }),
    ];
    prop_oneof![
        milestone().prop_map(|milestone| EventPayload::MilestoneReached { milestone }),
        (slug(AchievementId::new), tier())
            .prop_map(|(achievement, tier)| EventPayload::AchievementEarned { achievement, tier }),
        (slug(AchievementId::new), any::<bool>()).prop_map(|(achievement, visible)| {
            EventPayload::AchievementVisibilitySet {
                achievement,
                visible,
            }
        }),
        (unlock_kind(), slug(PackageKey::new), source)
            .prop_map(|(kind, key, source)| EventPayload::UnlockGranted { kind, key, source }),
        (unlock_kind(), slug(PackageKey::new), any::<u128>()).prop_map(|(kind, key, u)| {
            EventPayload::UnlockRevoked {
                kind,
                key,
                by: UserId(Uuid::from_u128(u)),
            }
        }),
        slug(PackageKey::new).prop_map(|theme| EventPayload::ThemeSelected { theme }),
        slug(PackageKey::new).prop_map(|companion| EventPayload::CompanionSelected { companion }),
        (cosmetic_slot(), slug(PackageKey::new))
            .prop_map(|(slot, key)| EventPayload::CosmeticSelected { slot, key }),
        proptest::collection::vec(slug(KcId::new), 0..6).prop_map(|keys| {
            EventPayload::ShowcaseSet {
                kind: ShowcaseKind::Cards,
                keys: ShowcaseKeys::new(keys).unwrap(),
            }
        }),
    ]
}

fn event() -> impl Strategy<Value = Event> {
    (
        uuid_v7(),
        any::<u128>(),
        proptest::option::of(any::<u128>()),
        timestamp(),
        proptest::option::of(timestamp()),
        payload(),
    )
        .prop_map(
            |(event_id, learner, session, client_ts, server_ts, payload)| Event {
                schema_version: CURRENT_SCHEMA_VERSION,
                event_id,
                learner_id: LearnerId(Uuid::from_u128(learner)),
                session_id: session.map(|u| SessionId(Uuid::from_u128(u))),
                client_ts,
                server_ts,
                payload,
            },
        )
}

fn slot() -> impl Strategy<Value = Slot> {
    (
        proptest::collection::vec(
            (slug(KcId::new), any::<bool>()).prop_map(|(kc, p)| SlotKc {
                kc,
                role: if p {
                    KcRole::Primary
                } else {
                    KcRole::Secondary
                },
            }),
            1..3,
        ),
        proptest::collection::vec(text().prop_map(AnswerPattern), 0..3),
        proptest::collection::vec(
            (
                text(),
                proptest::option::of(slug(MisconceptionId::new)),
                proptest::option::of(slug(KcId::new)),
            )
                .prop_map(|(text, misconception, confuses_with)| ChoiceOption {
                    text,
                    misconception,
                    confuses_with,
                }),
            0..4,
        ),
        proptest::collection::vec(
            (
                text(),
                slug(MisconceptionId::new),
                proptest::option::of(slug(KcId::new)),
            )
                .prop_map(|(p, misconception, confuses_with)| ExpectedWrong {
                    pattern: AnswerPattern(p),
                    misconception,
                    confuses_with,
                }),
            0..3,
        ),
        proptest::option::of(localized()),
        localized(),
        text(),
    )
        .prop_map(
            |(kcs, accepted, options, expected_wrong, hint, why, answer)| Slot {
                kcs,
                answer,
                accepted,
                options,
                expected_wrong,
                hint,
                why,
            },
        )
}

fn stimulus() -> impl Strategy<Value = Stimulus> {
    (
        text(),
        proptest::option::of(text().prop_map(refl_core::content::AudioRef)),
        proptest::option::of(text().prop_map(refl_core::item::ImageRef)),
    )
        .prop_map(|(text, audio, image)| Stimulus { text, audio, image })
}

fn english_check() -> impl Strategy<Value = EnglishCheck> {
    (
        any::<bool>(),
        prop_oneof![Just(Contractions::Strict), Just(Contractions::Equivalent)],
        any::<bool>(),
    )
        .prop_map(
            |(case_sensitive, contractions, punctuation_sensitive)| EnglishCheck {
                case_sensitive,
                contractions,
                punctuation_sensitive,
            },
        )
}

/// Every kind, so each `ItemBody` variant round-trips.
fn item_body() -> impl Strategy<Value = ItemBody> {
    (
        stimulus(),
        proptest::collection::vec(slot(), 1..4),
        english_check(),
        0..12u8,
    )
        .prop_map(|(stimulus, slots, check, kind)| match kind {
            0 => ItemBody::Choice(ChoiceBody { stimulus, slots }),
            1 => ItemBody::FindError(FindErrorBody { stimulus, slots }),
            2 => ItemBody::Matching(MatchingBody { stimulus, slots }),
            3 => ItemBody::TrueFalseNotGiven(TrueFalseNotGivenBody { stimulus, slots }),
            4 => ItemBody::GapFill(GapFillBody {
                stimulus,
                slots,
                check,
            }),
            5 => ItemBody::Transformation(TransformationBody {
                stimulus,
                slots,
                check,
            }),
            6 => ItemBody::SentenceAssembly(SentenceAssemblyBody {
                stimulus,
                slots,
                check,
            }),
            7 => ItemBody::Translation(TranslationBody {
                stimulus,
                slots,
                check,
            }),
            8 => ItemBody::Dictation(DictationBody {
                stimulus,
                slots,
                check,
            }),
            9 => ItemBody::OwnSentence(OwnSentenceBody { stimulus, slots }),
            10 => ItemBody::Writing(WritingBody { stimulus, slots }),
            _ => ItemBody::Shadowing(ShadowingBody { stimulus, slots }),
        })
}

fn item() -> impl Strategy<Value = Item> {
    (
        (
            slug(ItemId::new),
            any::<u32>(),
            slug(SubjectId::new),
            slug(refl_core::ids::FamilyId::new),
        ),
        slug(refl_core::ids::InstructionId::new),
        (1u8..=5, any::<u32>()),
        proptest::option::of(slug(refl_core::ids::TopicId::new)),
        prop_oneof![
            Just(ItemStatus::Draft),
            Just(ItemStatus::Review),
            Just(ItemStatus::Live),
            Just(ItemStatus::Retired)
        ],
        (
            proptest::option::of(text()),
            proptest::option::of(text()),
            any::<bool>(),
        ),
        item_body(),
    )
        .prop_map(
            |(
                (id, v, subject, family),
                instruction,
                (d, t),
                topic,
                status,
                (source, license, breaks_calibration),
                body,
            )| Item {
                id,
                version: ItemVersion(v),
                subject,
                family,
                instruction,
                difficulty: AuthorDifficulty::new(d).unwrap(),
                expected_time: t,
                topic,
                status,
                source,
                license,
                breaks_calibration,
                review: LangReviews::default(),
                review_group: None,
                body,
            },
        )
}

fn level_key() -> impl Strategy<Value = LevelKey> {
    "[A-Za-z0-9][A-Za-z0-9-]{0,6}".prop_map(|s| LevelKey::new(s).unwrap())
}

fn subject() -> impl Strategy<Value = Subject> {
    (
        slug(SubjectId::new),
        localized(),
        proptest::collection::vec(
            (level_key(), localized()).prop_map(|(key, title)| ScaleLevel { key, title }),
            1..8,
        ),
        finite(),
        proptest::collection::vec(slug(DomainId::new), 0..8),
        slug(KcId::new),
    )
        .prop_map(
            |(id, title, levels, step_logit, domains, spelling_kc)| Subject {
                id,
                title,
                scale: ScaleDef { levels, step_logit },
                domains,
                checker: Checker::EnWords,
                spelling_kc,
            },
        )
}

fn kc() -> impl Strategy<Value = Kc> {
    (
        (slug(KcId::new), slug(SubjectId::new), slug(DomainId::new)),
        localized(),
        level_key(),
        unit(),
        text(),
        (
            proptest::collection::vec(slug(Tag::new), 0..3),
            any::<bool>(),
        ),
    )
        .prop_map(
            |((id, subject, domain), title, level, importance, rule, (tags, observed_only))| Kc {
                id,
                subject,
                domain,
                title,
                review: LangReviews::default(),
                level,
                importance,
                rule,
                tags,
                observed_only,
            },
        )
}

fn misconception() -> impl Strategy<Value = Misconception> {
    (
        slug(MisconceptionId::new),
        localized(),
        proptest::collection::vec(slug(KcId::new), 1..3),
        (any::<bool>(), any::<bool>(), unit(), unit()),
        slug(KcId::new),
    )
        .prop_map(
            |(id, title, affects, (uz, ru, puz, pru), remedy)| Misconception {
                id,
                title,
                review: LangReviews::default(),
                affects,
                origin: MisconceptionOrigin::L1Transfer,
                l1: L1Set { uz, ru },
                prior: L1Prior { uz: puz, ru: pru },
                remedy,
            },
        )
}

/// Decodes an event this core must know.
fn known(json: &str) -> Event {
    match decode_event(json).unwrap() {
        Decoded::Known(e) => e,
        Decoded::Unknown(u) => panic!("known event decoded as unknown: {}", u.raw),
    }
}

/// x → bytes → x' must give x' == x, and x' must serialize to the same bytes.
fn assert_roundtrip<T>(x: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let a = serde_json::to_string(x).unwrap();
    let back: T = serde_json::from_str(&a).unwrap();
    assert_eq!(&back, x);
    let b = serde_json::to_string(&back).unwrap();
    assert_eq!(a.as_bytes(), b.as_bytes());
    // Same input, same bytes: serialization has no hidden state.
    assert_eq!(a, serde_json::to_string(x).unwrap());
}

proptest! {
    #[test]
    fn event_roundtrip_and_determinism(e in event()) {
        let a = encode_event(&e);
        let back = known(&a);
        prop_assert_eq!(&back, &e);
        let again = encode_event(&back);
        prop_assert_eq!(a.as_bytes(), again.as_bytes());
        prop_assert_eq!(a, encode_event(&e));
    }

    #[test]
    fn known_events_never_decode_as_unknown(e in event()) {
        let decoded = decode_event(&encode_event(&e));
        prop_assert!(matches!(decoded, Ok(Decoded::Known(_))), "{:?}", decoded);
    }

    #[test]
    fn item_roundtrip(i in item()) { assert_roundtrip(&i); }

    #[test]
    fn kc_roundtrip(k in kc()) { assert_roundtrip(&k); }

    #[test]
    fn subject_roundtrip(s in subject()) { assert_roundtrip(&s); }

    #[test]
    fn misconception_roundtrip(m in misconception()) { assert_roundtrip(&m); }

    #[test]
    fn event_type_matches_the_serialized_tag(p in payload()) {
        let v = serde_json::to_value(&p).unwrap();
        prop_assert_eq!(v["type"].as_str(), Some(p.event_type()));
    }
}

#[test]
fn edge_roundtrip() {
    let e = Edge {
        kind: EdgeKind::Requires,
        from: KcId::new("gram.article.first-mention").unwrap(),
        to: KcId::new("gram.noun.countability").unwrap(),
    };
    assert_roundtrip(&e);
}

fn sample_event_json(schema_version: u64, event_id: &str) -> String {
    format!(
        r#"{{"schema_version":{schema_version},"event_id":"{event_id}","learner_id":"00000000-0000-0000-0000-000000000001","session_id":null,"client_ts":1795300000000,"server_ts":null,"payload":{{"type":"lesson_started","lesson":"articles.intro"}}}}"#
    )
}

const V7: &str = "019aa5b4-0000-7000-8000-000000000000";
const V4: &str = "019aa5b4-0000-4000-8000-000000000000";

#[test]
fn decodes_current_version_byte_exact() {
    let json = sample_event_json(u64::from(CURRENT_SCHEMA_VERSION), V7);
    let e = known(&json);
    assert_eq!(encode_event(&e), json, "canonical form must be stable");
}

#[test]
fn version_1_is_lifted_to_2() {
    // Any payload: only the version changes.
    let e = known(&sample_event_json(1, V7));
    assert_eq!(e.schema_version, 2);
    assert_eq!(encode_event(&e), sample_event_json(2, V7));
    // plan_issued gains an empty `end`.
    let plan = serde_json::json!({
        "type": "plan_issued", "planner_version": "p1", "model_version": "m1",
        "scope": "session",
        "candidates": [{ "action": { "kind": "break" }, "value_per_minute": 0.0, "factors": [] }],
        "chosen": 0, "propensity": 1.0, "seed": 1, "violations": [],
        "dominant": "rapid_guesses", "trigger": "rapid_guesses",
    });
    let e = known(&sample_with_payload(1, &plan));
    let mut lifted = plan.clone();
    lifted["end"] = serde_json::Value::Null;
    assert_eq!(serde_json::to_value(&e.payload).unwrap(), lifted);
    // An answer loses `backgrounded` (true or not) and gains empty timing
    // fields: fluency never counts it.
    for backgrounded in [false, true] {
        let mut answer = answer_payload(&serde_json::json!({ "kind": "option", "value": 1 }));
        answer["backgrounded"] = backgrounded.into();
        let e = known(&sample_with_payload(1, &answer));
        let EventPayload::Answer(a) = &e.payload else {
            panic!("not an answer");
        };
        let timing = (
            a.first_input_ms,
            a.typing_ms,
            a.chars_typed,
            a.edits,
            a.input_device,
            a.hidden_ms,
        );
        assert_eq!(timing, (None, None, None, None, None, None));
        let written = serde_json::to_value(&e.payload).unwrap();
        assert!(written.get("backgrounded").is_none());
        for field in [
            "first_input_ms",
            "typing_ms",
            "chars_typed",
            "edits",
            "input_device",
            "hidden_ms",
        ] {
            assert_eq!(written[field], serde_json::Value::Null, "{field}");
        }
    }
}

#[test]
fn answer_timing_fields_of_schema_2_round_trip() {
    let mut answer = answer_payload(&serde_json::json!({ "kind": "text", "value": "an apple" }));
    let fields = answer.as_object_mut().unwrap();
    fields.remove("backgrounded");
    fields.insert("first_input_ms".into(), 2_400.into());
    fields.insert("typing_ms".into(), 3_100.into());
    fields.insert("chars_typed".into(), 9.into());
    fields.insert("edits".into(), 1.into());
    fields.insert("input_device".into(), "keyboard".into());
    fields.insert("hidden_ms".into(), 0.into());
    let e = known(&sample_with_payload(2, &answer));
    let EventPayload::Answer(a) = &e.payload else {
        panic!("not an answer");
    };
    assert_eq!(
        (a.first_input_ms, a.typing_ms, a.chars_typed, a.edits),
        (Some(2_400), Some(3_100), Some(9), Some(1))
    );
    assert_eq!(
        (a.input_device, a.hidden_ms),
        (Some(InputDevice::Keyboard), Some(0))
    );
    let written = serde_json::to_value(&e.payload).unwrap();
    for field in [
        "first_input_ms",
        "typing_ms",
        "chars_typed",
        "edits",
        "input_device",
        "hidden_ms",
    ] {
        assert_eq!(written[field], answer[field], "{field}");
    }
}

fn sample_with_payload(schema_version: u64, payload: &serde_json::Value) -> String {
    let mut v: serde_json::Value = serde_json::from_str(&sample_event_json(1, V7)).unwrap();
    v["schema_version"] = schema_version.into();
    v["payload"] = payload.clone();
    v.to_string()
}

fn answer_payload(response: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "type": "answer",
        "item": { "id": "articles.a-an.1", "version": 1 },
        "slot": 0,
        "context": "lesson",
        "response": response,
        "verdict": "correct",
        "misconception": null,
        "latency_ms": 1200,
        "backgrounded": false,
        "confidence": null,
        "hints": 0,
        "attempt": 1,
    })
}

fn assert_unknown(json: &str, schema_version: u64) {
    match decode_event(json) {
        Ok(Decoded::Unknown(u)) => assert_eq!(
            u,
            UnknownEvent {
                schema_version,
                event_id: EventId::new(V7.parse().unwrap()).unwrap(),
                learner_id: LearnerId(Uuid::from_u128(1)),
                raw: json.to_owned(),
            }
        ),
        other => panic!("expected Unknown, got {other:?}"),
    }
}

#[test]
fn newer_events_decode_as_unknown_with_raw_preserved() {
    // Unusual spacing and key order: raw is kept byte for byte.
    let new_type = sample_with_payload(1, &serde_json::json!({ "type": "sticker_sent", "n": 1 }))
        .replace(',', " ,  ");
    assert_unknown(&new_type, 1);
    let new_response = sample_with_payload(
        1,
        &answer_payload(&serde_json::json!({ "kind": "point", "value": [1, 2] })),
    );
    assert_unknown(&new_response, 1);
    // A future schema version is unknown whatever its payload looks like.
    let next = u64::from(CURRENT_SCHEMA_VERSION) + 1;
    let future = sample_with_payload(next, &serde_json::json!({ "anything": [] }));
    assert_unknown(&future, next);
    assert_unknown(&sample_event_json(MAX_SAFE_INTEGER, V7), MAX_SAFE_INTEGER);
}

#[test]
fn malformed_events_stay_errors() {
    let json_err = |json: &str| matches!(decode_event(json), Err(DecodeError::Json(_)));
    // Known type, invalid field values.
    assert!(json_err(&sample_with_payload(
        1,
        &serde_json::json!({ "type": "lesson_started", "lesson": "Not A Slug" })
    )));
    assert!(json_err(&sample_with_payload(
        1,
        &serde_json::json!({ "type": "lesson_started" })
    )));
    // Known response kind, wrong value.
    assert!(json_err(&sample_with_payload(
        1,
        &answer_payload(&serde_json::json!({ "kind": "option", "value": "a" }))
    )));
    // Answer without a response kind is malformed, not unknown.
    assert!(json_err(&sample_with_payload(
        1,
        &answer_payload(&serde_json::json!({ "value": 1 }))
    )));
    // Payload without a type tag, or no payload at all.
    assert!(json_err(&sample_with_payload(
        1,
        &serde_json::json!({ "lesson": "a" })
    )));
    let mut no_payload: serde_json::Value =
        serde_json::from_str(&sample_event_json(1, V7)).unwrap();
    no_payload.as_object_mut().unwrap().remove("payload");
    assert!(json_err(&no_payload.to_string()));
    // The envelope must parse even for unknown and future events.
    for v in [1, 2] {
        let bad_envelope =
            sample_with_payload(v, &serde_json::json!({ "type": "sticker_sent" })).replace(V7, V4);
        assert!(json_err(&bad_envelope), "schema {v}");
        let mut no_learner: serde_json::Value = serde_json::from_str(&sample_with_payload(
            v,
            &serde_json::json!({ "type": "sticker_sent" }),
        ))
        .unwrap();
        no_learner.as_object_mut().unwrap().remove("learner_id");
        assert!(json_err(&no_learner.to_string()), "schema {v}");
    }
    assert!(json_err("{not json"));
}

#[test]
fn rejects_zero_and_missing_versions() {
    assert!(matches!(
        decode_event(&sample_event_json(0, V7)),
        Err(DecodeError::UnknownSchemaVersion(0))
    ));
    assert!(matches!(
        decode_event(r#"{"event_id":"x"}"#),
        Err(DecodeError::MissingSchemaVersion)
    ));
}

#[test]
fn rejects_non_v7_event_id() {
    assert!(matches!(
        decode_event(&sample_event_json(1, V4)),
        Err(DecodeError::Json(_))
    ));
}

#[test]
fn rejects_invalid_field_values() {
    assert!(serde_json::from_str::<UnitInterval>("1.5").is_err());
    assert!(serde_json::from_str::<UnitInterval>("-0.1").is_err());
    assert!(serde_json::from_str::<AuthorDifficulty>("0").is_err());
    assert!(serde_json::from_str::<AuthorDifficulty>("6").is_err());
    assert!(serde_json::from_str::<KcId>(r#""Gram.Article""#).is_err());
    assert!(serde_json::from_str::<KcId>(r#""gram..article""#).is_err());
}

#[test]
fn cefr_scale() {
    assert_eq!(Cefr::PreA1.level(), 0);
    assert_eq!(Cefr::C2.level(), 6);
    assert_eq!(serde_json::to_string(&Cefr::PreA1).unwrap(), r#""pre-A1""#);
}

#[test]
fn derived_keys_cover_only_server_derived_events() {
    let key = |p: EventPayload| p.derived_key().map(|k| (k.kind, k.key));
    let b1 = Milestone::Level(Cefr::B1);
    let pkg = || PackageKey::new("blocks").unwrap();
    let ach = || AchievementId::new("first-step").unwrap();
    assert_eq!(
        key(EventPayload::MilestoneReached {
            milestone: b1.clone()
        }),
        Some(("level", "B1".to_owned()))
    );
    assert_eq!(
        key(EventPayload::MilestoneReached {
            milestone: Milestone::Topic(KcId::new("topic.articles").unwrap())
        }),
        Some(("topic", "topic.articles".to_owned()))
    );
    // Each tier is its own derived event.
    for (t, k) in [(1, "first-step:1"), (4, "first-step:4")] {
        assert_eq!(
            key(EventPayload::AchievementEarned {
                achievement: ach(),
                tier: AchievementTier::new(t).unwrap(),
            }),
            Some(("achievement", k.to_owned()))
        );
    }
    for source in [
        UnlockSource::Milestone { milestone: b1 },
        UnlockSource::Achievement {
            achievement: ach(),
            tier: AchievementTier::I,
        },
    ] {
        assert_eq!(
            key(EventPayload::UnlockGranted {
                kind: UnlockKind::Theme,
                key: pkg(),
                source
            }),
            Some(("theme", "blocks".to_owned()))
        );
    }
    let admin = UserId(Uuid::from_u128(1));
    assert_eq!(
        key(EventPayload::UnlockGranted {
            kind: UnlockKind::Theme,
            key: pkg(),
            source: UnlockSource::Admin { by: admin }
        }),
        None
    );
    assert_eq!(
        key(EventPayload::UnlockRevoked {
            kind: UnlockKind::Theme,
            key: pkg(),
            by: admin
        }),
        None
    );
    assert_eq!(key(EventPayload::ThemeSelected { theme: pkg() }), None);
    assert_eq!(
        key(EventPayload::CosmeticSelected {
            slot: CosmeticSlot::Frame,
            key: pkg()
        }),
        None
    );
    assert_eq!(
        key(EventPayload::ShowcaseSet {
            kind: ShowcaseKind::Cards,
            keys: ShowcaseKeys::new(vec![KcId::new("topic.articles").unwrap()]).unwrap()
        }),
        None
    );
}

#[test]
fn new_answer_fields_default_on_input() {
    let json = sample_with_payload(
        1,
        &answer_payload(&serde_json::json!({ "kind": "text", "value": "an apple" })),
    );
    let EventPayload::Answer(a) = known(&json).payload else {
        panic!("not an answer");
    };
    assert_eq!((a.item.seed, a.scaffold, a.revealed_deps), (None, 0, false));
    let written = serde_json::to_value(&a).unwrap();
    assert_eq!(
        written["item"],
        serde_json::json!({ "id": "articles.a-an.1", "version": 1 })
    );
    assert_eq!(written["scaffold"], 0);
    assert_eq!(written["revealed_deps"], false);
    for (context, tag) in [
        (AnswerContext::Audit, "audit"),
        (AnswerContext::ReturnCheck, "return_check"),
    ] {
        assert_eq!(serde_json::to_value(context).unwrap(), tag);
    }
}

fn writing_scored(half_bands: u64) -> serde_json::Value {
    serde_json::json!({
        "type": "writing_scored",
        "item": { "id": "ielts.task2.1", "version": 1 },
        "slot": 0,
        "scorer": "00000000-0000-0000-0000-000000000002",
        "scores": [{ "criterion": "coherence", "half_bands": half_bands }],
        "blind": true,
    })
}

#[test]
fn half_bands_are_validated() {
    for ok in [0, 13, 18] {
        let EventPayload::WritingScored(w) =
            known(&sample_with_payload(1, &writing_scored(ok))).payload
        else {
            panic!("not writing_scored");
        };
        assert_eq!(u64::from(w.scores[0].half_bands.get()), ok);
    }
    for bad in [19, 255, 256] {
        assert!(
            matches!(
                decode_event(&sample_with_payload(1, &writing_scored(bad))),
                Err(DecodeError::Json(_))
            ),
            "{bad}"
        );
    }
    assert!(HalfBands::new(19).is_none());
}

#[test]
fn quality_events_wire_shape() {
    let plan = serde_json::json!({
        "type": "plan_issued",
        "planner_version": "p1",
        "model_version": "m1",
        "scope": "next_item",
        "candidates": [
            {
                "action": { "kind": "item", "item": { "id": "articles.a-an.1", "version": 2, "seed": 7 }, "slot": 1 },
                "value_per_minute": 0.5,
                "factors": [{ "name": "forgetting", "value": 0.25 }],
            },
            { "action": { "kind": "learn", "kc": "gram.article.first-mention" }, "value_per_minute": 0.25, "factors": [] },
            { "action": { "kind": "break" }, "value_per_minute": 0.0, "factors": [] },
        ],
        "chosen": 0,
        "propensity": 0.75,
        "seed": 42,
        "violations": ["too-many-new-kcs"],
        "dominant": "memory",
        "trigger": "block_closed",
        "end": "kc_cap",
    });
    let mentor = |kind: serde_json::Value| {
        serde_json::json!({
            "type": "mentor_override",
            "mentor": "00000000-0000-0000-0000-000000000002",
            "kind": kind,
            "reason": "other",
            "note": "knows it from school",
        })
    };
    for payload in [
        plan,
        mentor(serde_json::json!({ "kind": "plan_change", "plan": V7 })),
        mentor(serde_json::json!({
            "kind": "mark_known", "kc": "gram.article.first-mention", "model_mastery": 0.25,
        })),
        mentor(serde_json::json!({ "kind": "dispute_verdict", "event": V7 })),
        writing_scored(13),
    ] {
        let json = sample_with_payload(u64::from(CURRENT_SCHEMA_VERSION), &payload);
        let e = known(&json);
        assert_eq!(serde_json::to_value(&e.payload).unwrap(), payload);
        // Not derived: no uniqueness key (docs/02, «События контроля качества»).
        assert!(e.payload.derived_key().is_none());
    }
    let mut out_of_range = serde_json::json!({
        "type": "plan_issued", "planner_version": "p1", "model_version": "m1",
        "scope": "session", "chosen": 0, "propensity": 1.5, "seed": 1, "violations": [],
        "dominant": "memory", "trigger": null,
        "candidates": [{ "action": { "kind": "review", "kc": "a" }, "value_per_minute": 1.0, "factors": [] }],
    });
    assert!(decode_event(&sample_with_payload(1, &out_of_range)).is_err());
    out_of_range["propensity"] = 1.0.into();
    out_of_range["scope"] = "lesson".into();
    assert!(decode_event(&sample_with_payload(1, &out_of_range)).is_err());
}

#[test]
fn reward_events_wire_shape_and_tier_default() {
    // Without a tier, an achievement and an achievement source are tier I.
    let earned = known(&sample_with_payload(
        1,
        &serde_json::json!({ "type": "achievement_earned", "achievement": "first-step" }),
    ));
    assert_eq!(
        earned.payload,
        EventPayload::AchievementEarned {
            achievement: AchievementId::new("first-step").unwrap(),
            tier: AchievementTier::I,
        }
    );
    let granted = known(&sample_with_payload(
        1,
        &serde_json::json!({
            "type": "unlock_granted", "kind": "title", "key": "scholar",
            "source": { "kind": "achievement", "achievement": "first-step" },
        }),
    ));
    let EventPayload::UnlockGranted {
        source: UnlockSource::Achievement { tier, .. },
        ..
    } = granted.payload
    else {
        panic!("not an achievement unlock");
    };
    assert_eq!(tier, AchievementTier::I);
    for payload in [
        serde_json::json!({ "type": "achievement_earned", "achievement": "streak", "tier": 6 }),
        serde_json::json!({ "type": "cosmetic_selected", "slot": "background", "key": "aurora" }),
        serde_json::json!({ "type": "showcase_set", "kind": "cards", "keys": ["topic.articles"] }),
        serde_json::json!({
            "type": "unlock_granted", "kind": "effect", "key": "sparkle",
            "source": { "kind": "achievement", "achievement": "streak", "tier": 2 },
        }),
    ] {
        let e = known(&sample_with_payload(1, &payload));
        assert_eq!(serde_json::to_value(&e.payload).unwrap(), payload);
    }
    for bad_tier in [0, 7] {
        let payload = serde_json::json!({ "type": "achievement_earned", "achievement": "streak", "tier": bad_tier });
        assert!(
            matches!(
                decode_event(&sample_with_payload(1, &payload)),
                Err(DecodeError::Json(_))
            ),
            "{bad_tier}"
        );
    }
}

#[test]
fn integers_beyond_2_pow_53_are_rejected() {
    let limit = MAX_SAFE_INTEGER;
    assert_eq!(Seed::new(limit).map(Seed::get), Some(limit));
    assert!(Seed::new(limit + 1).is_none());
    let with_seed = |s: u64| {
        let mut p = answer_payload(&serde_json::json!({ "kind": "option", "value": 1 }));
        p["item"]["seed"] = s.into();
        sample_with_payload(1, &p)
    };
    let EventPayload::Answer(a) = known(&with_seed(limit)).payload else {
        panic!("not an answer");
    };
    assert_eq!(a.item.seed, Seed::new(limit));
    let json_err = |json: &str| matches!(decode_event(json), Err(DecodeError::Json(_)));
    assert!(json_err(&with_seed(limit + 1)));
    assert!(json_err(&with_seed(u64::MAX)));
    for ts in [i128::from(limit) + 1, -i128::from(limit) - 1] {
        let mut v: serde_json::Value = serde_json::from_str(&sample_event_json(1, V7)).unwrap();
        v["client_ts"] = serde_json::Value::Number(ts.to_string().parse().unwrap());
        assert!(json_err(&v.to_string()), "{ts}");
    }
    for version in [limit + 1, u64::MAX] {
        assert!(matches!(
            decode_event(&sample_event_json(version, V7)),
            Err(DecodeError::UnknownSchemaVersion(v)) if v == version
        ));
    }
}

fn plan_with(chosen: u64, value_per_minute: &serde_json::Value) -> String {
    sample_with_payload(
        1,
        &serde_json::json!({
            "type": "plan_issued", "planner_version": "p1", "model_version": "m1",
            "scope": "session", "chosen": chosen, "propensity": 1.0, "seed": 1,
            "violations": [], "dominant": "memory", "trigger": null,
            "candidates": [
                { "action": { "kind": "review", "kc": "a" }, "value_per_minute": value_per_minute,
                  "factors": [{ "name": "f", "value": 2.0 }] },
                { "action": { "kind": "learn", "kc": "b" }, "value_per_minute": 1.0, "factors": [] },
            ],
        }),
    )
}

#[test]
fn plan_issued_is_validated_on_decode() {
    let json_err = |json: &str| matches!(decode_event(json), Err(DecodeError::Json(_)));
    let v = serde_json::json!(0.5);
    for ok in [0, 1] {
        known(&plan_with(ok, &v));
    }
    // chosen must index a candidate; an empty list has none.
    assert!(json_err(&plan_with(2, &v)));
    let mut empty: serde_json::Value = serde_json::from_str(&plan_with(0, &v)).unwrap();
    empty["payload"]["candidates"] = serde_json::json!([]);
    assert!(json_err(&empty.to_string()));
    // NaN and infinities serialize as null or overflow; neither decodes.
    assert!(json_err(&plan_with(0, &serde_json::Value::Null)));
    assert!(json_err(&plan_with(0, &v).replace("0.5", "1e400")));
    assert!(json_err(&plan_with(0, &v).replace("2.0", "null")));
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(Finite::new(bad).is_none());
        assert!(serde_json::from_str::<Finite>(&serde_json::to_string(&bad).unwrap()).is_err());
    }
}

#[test]
fn showcase_holds_at_most_five_cards() {
    let showcase = |n: usize| {
        let keys: Vec<String> = (0..n).map(|i| format!("topic.t{i}")).collect();
        sample_with_payload(
            1,
            &serde_json::json!({ "type": "showcase_set", "kind": "cards", "keys": keys }),
        )
    };
    for n in [0, 5] {
        known(&showcase(n));
    }
    assert!(matches!(
        decode_event(&showcase(6)),
        Err(DecodeError::Json(_))
    ));
}
