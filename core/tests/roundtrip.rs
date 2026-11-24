//! Serialization round trips and byte-level determinism for the log and content types.

use proptest::prelude::*;
use refl_core::content::{
    Edge, EdgeKind, Kc, KcDomain, L1Prior, L1Set, Misconception, MisconceptionOrigin,
};
use refl_core::event::{
    AnswerContext, AnswerEvent, Confidence, CriterionScore, EventPayload, HalfBands,
    MentorOverrideEvent, OverrideKind, OverrideReason, PlanAction, PlanCandidate, PlanFactor,
    PlanIssuedEvent, PlanScope, ReclassifiedEvent, Response, Verdict, WritingScoredEvent,
};
use refl_core::ids::{
    AchievementId, ContentVersion, EventId, ExplanationId, ItemId, ItemRef, ItemVersion, KcId,
    LearnerId, LessonId, MisconceptionId, PackageKey, SessionId, Tag, UserId,
};
use refl_core::item::{
    AnswerPattern, AuthorDifficulty, ChoiceOption, Contractions, ExpectedWrong, Item, ItemStatus,
    ItemType, KcRole, Slot, SlotKc, Stimulus,
};
use refl_core::unlock::{
    AchievementTier, CosmeticSlot, Milestone, ShowcaseKind, UnlockKind, UnlockSource,
};
use refl_core::value::{Cefr, Localized, MAX_SAFE_INTEGER, Seed, Timestamp, UnitInterval};
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
        (any::<u32>(), any::<bool>()),
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
                (latency_ms, backgrounded),
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
                    backgrounded,
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
fn quality_payload() -> impl Strategy<Value = EventPayload> {
    let action = prop_oneof![
        slug(KcId::new).prop_map(|kc| PlanAction::Learn { kc }),
        slug(KcId::new).prop_map(|kc| PlanAction::Review { kc }),
        (item_ref(), any::<u16>()).prop_map(|(item, slot)| PlanAction::Item { item, slot }),
    ];
    let candidate = (
        action,
        any::<f64>(),
        proptest::collection::vec(
            (text(), any::<f64>()).prop_map(|(name, value)| PlanFactor { name, value }),
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
        proptest::collection::vec(candidate, 0..4),
        (any::<u16>(), unit(), seed()),
    )
        .prop_map(
            |((planner_version, model_version), scope, candidates, (chosen, propensity, seed))| {
                EventPayload::PlanIssued(PlanIssuedEvent {
                    planner_version,
                    model_version,
                    scope,
                    candidates,
                    chosen,
                    propensity,
                    seed,
                })
            },
        );
    let mentor = (
        user(),
        prop_oneof![
            Just(OverrideKind::PlanChange),
            slug(KcId::new).prop_map(|kc| OverrideKind::MarkKnown { kc }),
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
                keys,
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

fn item() -> impl Strategy<Value = Item> {
    (
        (
            slug(ItemId::new),
            any::<u32>(),
            slug(refl_core::ids::FamilyId::new),
        ),
        prop_oneof![
            Just(ItemType::Choice),
            Just(ItemType::GapFill),
            Just(ItemType::Transformation)
        ],
        slug(refl_core::ids::InstructionId::new),
        text(),
        proptest::collection::vec(slot(), 1..4),
        (1u8..=5, any::<u32>()),
        proptest::option::of(slug(refl_core::ids::TopicId::new)),
        prop_oneof![Just(ItemStatus::Draft), Just(ItemStatus::Live)],
        (
            proptest::option::of(text()),
            proptest::option::of(text()),
            any::<bool>(),
        ),
        (
            any::<bool>(),
            prop_oneof![Just(Contractions::Strict), Just(Contractions::Equivalent)],
            any::<bool>(),
        ),
    )
        .prop_map(
            |(
                (id, v, family),
                item_type,
                instruction,
                stim,
                slots,
                (d, t),
                topic,
                status,
                (source, license, breaks_calibration),
                (case_sensitive, contractions, punctuation_sensitive),
            )| Item {
                id,
                version: ItemVersion(v),
                family,
                item_type,
                instruction,
                stimulus: Stimulus {
                    text: stim,
                    audio: None,
                    image: None,
                },
                slots,
                difficulty: AuthorDifficulty::new(d).unwrap(),
                expected_time: t,
                topic,
                status,
                case_sensitive,
                contractions,
                punctuation_sensitive,
                source,
                license,
                breaks_calibration,
            },
        )
}

fn kc() -> impl Strategy<Value = Kc> {
    (
        slug(KcId::new),
        localized(),
        unit(),
        text(),
        proptest::collection::vec(slug(Tag::new), 0..3),
    )
        .prop_map(|(id, title, importance, rule, tags)| Kc {
            id,
            domain: KcDomain::Grammar,
            title,
            cefr: Cefr::B1,
            importance,
            rule,
            tags,
        })
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
    let json = sample_event_json(1, V7);
    let e = known(&json);
    assert_eq!(encode_event(&e), json, "canonical form must be stable");
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
    let future = sample_with_payload(2, &serde_json::json!({ "anything": [] }));
    assert_unknown(&future, 2);
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
            keys: vec![KcId::new("topic.articles").unwrap()]
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
        ],
        "chosen": 0,
        "propensity": 0.75,
        "seed": 42,
    });
    let mentor = serde_json::json!({
        "type": "mentor_override",
        "mentor": "00000000-0000-0000-0000-000000000002",
        "kind": { "kind": "dispute_verdict", "event": V7 },
        "reason": "wrong_verdict",
        "note": null,
    });
    for payload in [plan, mentor, writing_scored(13)] {
        let json = sample_with_payload(1, &payload);
        let e = known(&json);
        assert_eq!(serde_json::to_value(&e.payload).unwrap(), payload);
        // Not derived: no uniqueness key (docs/02, «События контроля качества»).
        assert!(e.payload.derived_key().is_none());
    }
    let mut out_of_range = serde_json::json!({
        "type": "plan_issued", "planner_version": "p1", "model_version": "m1",
        "scope": "session", "candidates": [], "chosen": 0, "propensity": 1.5, "seed": 1,
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
