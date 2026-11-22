//! Serialization round trips and byte-level determinism for the log and content types.

use eng_core::content::{
    Edge, EdgeKind, Kc, KcDomain, L1Prior, L1Set, Misconception, MisconceptionOrigin,
};
use eng_core::event::{
    AnswerContext, AnswerEvent, Confidence, EventPayload, ReclassifiedEvent, Response, Verdict,
};
use eng_core::ids::{
    AchievementId, ContentVersion, EventId, ExplanationId, ItemId, ItemRef, ItemVersion, KcId,
    LearnerId, LessonId, MisconceptionId, PackageKey, SessionId, Tag, UserId,
};
use eng_core::item::{
    AnswerPattern, AuthorDifficulty, ChoiceOption, Contractions, ExpectedWrong, Item, ItemStatus,
    ItemType, KcRole, Slot, SlotKc, Stimulus,
};
use eng_core::unlock::{Milestone, UnlockKind, UnlockSource};
use eng_core::value::{Cefr, Localized, Timestamp, UnitInterval};
use eng_core::{CURRENT_SCHEMA_VERSION, DecodeError, Event, decode_event, encode_event};
use proptest::prelude::*;
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

fn payload() -> impl Strategy<Value = EventPayload> {
    let answer = (
        (slug(ItemId::new), any::<u32>(), any::<u16>()),
        prop_oneof![
            Just(AnswerContext::Lesson),
            Just(AnswerContext::Review),
            Just(AnswerContext::Placement),
            Just(AnswerContext::Practice),
            Just(AnswerContext::Mock)
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
                (id, v, slot),
                context,
                response,
                verdict,
                misconception,
                (latency_ms, backgrounded),
                confidence,
                (hints, attempt),
            )| {
                EventPayload::Answer(AnswerEvent {
                    item: ItemRef {
                        id,
                        version: ItemVersion(v),
                    },
                    slot,
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
                depth: eng_core::content::ExplanationDepth::Why,
                seconds,
            }
        }),
        reward_payload(),
    ]
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
        Just(UnlockKind::Frame),
        Just(UnlockKind::Companion)
    ]
}

/// Milestones, achievements, unlocks and selections (docs/07).
fn reward_payload() -> impl Strategy<Value = EventPayload> {
    let source = prop_oneof![
        milestone().prop_map(|milestone| UnlockSource::Milestone { milestone }),
        slug(AchievementId::new).prop_map(|achievement| UnlockSource::Achievement { achievement }),
        any::<u128>().prop_map(|u| UnlockSource::Admin {
            by: UserId(Uuid::from_u128(u))
        }),
    ];
    prop_oneof![
        milestone().prop_map(|milestone| EventPayload::MilestoneReached { milestone }),
        slug(AchievementId::new)
            .prop_map(|achievement| EventPayload::AchievementEarned { achievement }),
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
    ]
}

fn event() -> impl Strategy<Value = Event> {
    (
        uuid_v7(),
        any::<u128>(),
        proptest::option::of(any::<u128>()),
        any::<i64>(),
        proptest::option::of(any::<i64>()),
        payload(),
    )
        .prop_map(|(event_id, learner, session, c, s, payload)| Event {
            schema_version: CURRENT_SCHEMA_VERSION,
            event_id,
            learner_id: LearnerId(Uuid::from_u128(learner)),
            session_id: session.map(|u| SessionId(Uuid::from_u128(u))),
            client_ts: Timestamp(c),
            server_ts: s.map(Timestamp),
            payload,
        })
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
            (text(), proptest::option::of(slug(MisconceptionId::new))).prop_map(
                |(text, misconception)| ChoiceOption {
                    text,
                    misconception,
                },
            ),
            0..4,
        ),
        proptest::collection::vec(
            (text(), slug(MisconceptionId::new)).prop_map(|(p, misconception)| ExpectedWrong {
                pattern: AnswerPattern(p),
                misconception,
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
            slug(eng_core::ids::FamilyId::new),
        ),
        prop_oneof![
            Just(ItemType::Choice),
            Just(ItemType::GapFill),
            Just(ItemType::Transformation)
        ],
        slug(eng_core::ids::InstructionId::new),
        text(),
        proptest::collection::vec(slot(), 1..4),
        (1u8..=5, any::<u32>()),
        proptest::option::of(slug(eng_core::ids::TopicId::new)),
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
        let back = decode_event(&a).unwrap();
        prop_assert_eq!(&back, &e);
        let again = encode_event(&back);
        prop_assert_eq!(a.as_bytes(), again.as_bytes());
        prop_assert_eq!(a, encode_event(&e));
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
    let e = decode_event(&json).unwrap();
    assert_eq!(encode_event(&e), json, "canonical form must be stable");
}

#[test]
fn rejects_future_unknown_and_missing_versions() {
    assert!(matches!(
        decode_event(&sample_event_json(2, V7)),
        Err(DecodeError::FutureSchemaVersion(2))
    ));
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
    assert_eq!(
        key(EventPayload::AchievementEarned { achievement: ach() }),
        Some(("achievement", "first-step".to_owned()))
    );
    for source in [
        UnlockSource::Milestone { milestone: b1 },
        UnlockSource::Achievement { achievement: ach() },
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
}
