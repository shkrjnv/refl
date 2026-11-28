//! Batch 4: lexical cards, passive credit, custom words, notes, book marks
//! and coverage (`docs/16-vocabulary-library-notes-graph.md`).

use refl_core::book::{
    Book, BookChapter, BookParagraph, BookSentence, BookToken, Coverage, coverage,
};
use refl_core::bundle::Bundle;
use refl_core::event::{
    BookMarkCreatedEvent, BookMarkDeletedEvent, BookMarkKind, CustomLexemeArchivedEvent,
    CustomLexemeEvent, CustomLexemeSource, Decoded, EventPayload, NoteAnchor, NoteAnchorKind,
    NoteDeletedEvent, NoteSavedEvent,
};
use refl_core::ids::{BookId, BookMarkId, CustomLexemeId, LexemeId, NoteId};
use refl_core::model::{
    KnowledgeModel, LearnerModel, LexicalDirection, LexicalSource, Profile, State, replay,
};
use refl_core::subject::LevelKey;
use refl_core::value::{Localized, Timestamp};
use refl_sim::{
    BundleSpec, DAY_MS, LogSpec, T0, answer, event, event_id, item_ref, synthetic_bundle,
    synthetic_log,
};
use serde_json::json;

fn bundle() -> Bundle {
    synthetic_bundle(BundleSpec {
        kcs: 3,
        items_per_kc: 6,
    })
}

fn at(day: i64, ms: i64) -> Timestamp {
    Timestamp(T0.0 + day * DAY_MS + ms)
}

fn fold(bundle: &Bundle, events: &[Decoded]) -> State {
    replay(
        &KnowledgeModel::default(),
        &Profile::default(),
        bundle,
        events,
    )
}

fn custom(id: CustomLexemeId, lemma: &str, archived: bool) -> CustomLexemeEvent {
    CustomLexemeEvent {
        custom_id: id,
        source: CustomLexemeSource::Own,
        bundle_lexeme_id: None,
        lemma: lemma.into(),
        pos: Some("noun".into()),
        definition_en: "a small animal".into(),
        gloss_uz: "mushuk".into(),
        gloss_ru: "кошка".into(),
        examples: vec!["A cat naps.".into()],
        collocations: Vec::new(),
        ipa: None,
        audio_key: None,
        context: None,
        archived,
    }
}

fn custom_event(t: Timestamp, tail: u64, lemma: &str, archived: bool) -> Decoded {
    let id = CustomLexemeId(*event_id(t, tail).as_uuid());
    event(
        t,
        tail,
        EventPayload::CustomLexemeCreated(custom(id, lemma, archived)),
    )
}

#[test]
fn custom_lexeme_lww_is_by_event_id_and_creates_two_cards() {
    let b = bundle();
    let t0 = at(0, 0);
    let t1 = at(1, 0);
    let t2 = at(2, 0);
    let created = custom_event(t0, 1, "cat", false);
    let custom_id = match &created {
        Decoded::Known(ev) => match &ev.payload {
            EventPayload::CustomLexemeCreated(c) => c.custom_id,
            _ => unreachable!(),
        },
        Decoded::Unknown(_) => unreachable!(),
    };
    let updated = event(
        t1,
        2,
        EventPayload::CustomLexemeUpdated(custom(custom_id, "kitty", false)),
    );
    let archived = event(
        t2,
        3,
        EventPayload::CustomLexemeArchived(CustomLexemeArchivedEvent {
            custom_id,
            archived: true,
        }),
    );
    let state = fold(&b, &[archived.clone(), created.clone(), updated.clone()]);
    let record = &state.custom_lexemes[&custom_id];
    assert_eq!(record.lemma, "kitty");
    assert!(record.archived);
    let lexeme = LexemeId::new(custom_id.0.to_string()).unwrap();
    for direction in [LexicalDirection::Recognize, LexicalDirection::Produce] {
        let card = state.lexical_card(&lexeme, direction).expect("card");
        assert_eq!(card.review_count, 0);
        assert_eq!(card.source, LexicalSource::Custom);
    }
    // The same events in another arrival order fold to the same state.
    assert_eq!(state.hash(), fold(&b, &[created, archived, updated]).hash());
}

#[test]
fn a_custom_word_matching_the_bank_creates_no_second_card() {
    let b = bundle();
    let sense = LexemeId::new("en.lex.cat.animal").unwrap();
    let t = at(0, 0);
    let id = CustomLexemeId(*event_id(t, 1).as_uuid());
    let mut payload = custom(id, "cat", false);
    payload.bundle_lexeme_id = Some(sense.clone());
    let state = fold(
        &b,
        &[event(t, 1, EventPayload::CustomLexemeCreated(payload))],
    );
    assert!(
        state.lexical_cards.is_empty(),
        "no personal card for a bank word"
    );
    assert_eq!(state.custom_lexemes[&id].bundle_lexeme_id, Some(sense));
}

#[test]
fn notes_are_lww_restore_and_delete() {
    let b = bundle();
    let t = [at(0, 0), at(1, 0), at(2, 0), at(3, 0)];
    let id = NoteId(*event_id(t[0], 1).as_uuid());
    let save = |when: Timestamp, tail: u64, body: &str| {
        event(
            when,
            tail,
            EventPayload::NoteSaved(NoteSavedEvent {
                note_id: id,
                anchor: NoteAnchor {
                    kind: NoteAnchorKind::Standalone,
                    id: None,
                },
                body: body.into(),
            }),
        )
    };
    let delete = event(
        t[2],
        3,
        EventPayload::NoteDeleted(NoteDeletedEvent { note_id: id }),
    );
    let restored = save(t[3], 4, "restored");
    let state = fold(
        &b,
        &[
            delete.clone(),
            save(t[1], 2, "second"),
            save(t[0], 1, "first"),
            restored.clone(),
        ],
    );
    assert!(!state.notes[&id].deleted);
    assert_eq!(state.notes[&id].body, "restored");
    let deleted = fold(
        &b,
        &[
            save(t[0], 1, "first"),
            save(t[1], 2, "second"),
            delete.clone(),
        ],
    );
    assert!(deleted.notes[&id].deleted);
    assert_eq!(
        state.hash(),
        fold(
            &b,
            &[
                save(t[0], 1, "first"),
                save(t[1], 2, "second"),
                delete.clone(),
                restored
            ]
        )
        .hash()
    );
    assert_eq!(deleted.notes[&id].body, "second");
}

#[test]
fn book_marks_are_lww_and_deletion_leaves_a_tombstone() {
    let b = bundle();
    let t = [at(0, 0), at(1, 0)];
    let id = BookMarkId(*event_id(t[0], 1).as_uuid());
    let create = event(
        t[0],
        1,
        EventPayload::BookMarkCreated(BookMarkCreatedEvent {
            mark_id: id,
            book_id: BookId::new("book.first-day").unwrap(),
            book_version: "a".repeat(64),
            kind: BookMarkKind::Bookmark,
            chapter_id: "chapter.first-day.1".into(),
            paragraph_id: "para.first-day.1.1".into(),
            sentence_id: "sentence.first-day.1.1.1".into(),
            start_token: 0,
            end_token: 2,
            quote_hash: "b".repeat(64),
            color: None,
        }),
    );
    let delete = event(
        t[1],
        2,
        EventPayload::BookMarkDeleted(BookMarkDeletedEvent { mark_id: id }),
    );
    let state = fold(&b, &[create.clone(), delete.clone()]);
    assert!(state.book_marks[&id].deleted);
    // A deletion with no matching creation still suppresses an earlier one.
    let deleted_first = fold(&b, &[delete, create]);
    assert!(deleted_first.book_marks[&id].deleted);
    assert_eq!(state.hash(), deleted_first.hash());
}

#[test]
fn passive_credit_needs_three_items_and_two_families() {
    let mut b = bundle();
    let sense = LexemeId::new("en.lex.cat.animal").unwrap();
    // Items 0 and 1 share family `...k00.0`; item 4 is family `...k00.1`.
    for j in [0usize, 1, 4] {
        b.content
            .item_lexemes
            .insert(item_ref(0, j).id, vec![sense.clone()]);
    }
    let t = at(0, 0);
    let answers: Vec<Decoded> = [0usize, 1, 4]
        .iter()
        .enumerate()
        .map(|(n, j)| {
            let mut a = answer(
                item_ref(0, *j),
                refl_core::event::Verdict::Correct,
                refl_core::event::Response::Option(0),
                8_000,
            );
            a.attempt = 1;
            event(
                Timestamp(t.0 + i64::try_from(n).unwrap() * 1_000),
                10 + n as u64,
                EventPayload::Answer(a),
            )
        })
        .collect();
    let state = fold(&b, &answers);
    let card = state
        .lexical_card(&sense, LexicalDirection::Recognize)
        .expect("card born on the third meeting");
    assert_eq!(card.review_count, 0);
    assert!((card.exposures - 0.45).abs() < f64::EPSILON);
    let exposure = &state.lexical_exposures[&sense];
    assert_eq!(exposure.count, 3);
    assert_eq!(exposure.items.len(), 3);
    assert_eq!(exposure.families.len(), 2);
}

#[test]
fn a_wrong_rapid_or_hinted_answer_does_not_credit() {
    let mut b = bundle();
    let sense = LexemeId::new("en.lex.cat.animal").unwrap();
    for j in [0usize, 1, 4] {
        b.content
            .item_lexemes
            .insert(item_ref(0, j).id, vec![sense.clone()]);
    }
    let t = at(0, 0);
    let mut answers = Vec::new();
    for (n, j) in [0usize, 1, 4].into_iter().enumerate() {
        let mut a = answer(
            item_ref(0, j),
            refl_core::event::Verdict::Correct,
            refl_core::event::Response::Option(0),
            100,
        );
        match n {
            0 => a.hints = 1,
            1 => a.attempt = 2,
            _ => a.verdict = refl_core::event::Verdict::Wrong,
        }
        answers.push(event(
            Timestamp(t.0 + i64::try_from(n).unwrap() * 1_000),
            20 + n as u64,
            EventPayload::Answer(a),
        ));
    }
    let state = fold(&b, &answers);
    assert!(
        state
            .lexical_card(&sense, LexicalDirection::Recognize)
            .is_none(),
        "no card without clean first-try correct answers"
    );
}

#[test]
fn direct_review_beats_passive_and_placement_seeds_a_prior() {
    let model = KnowledgeModel::default();
    let sense = LexemeId::new("en.lex.cat.animal").unwrap();
    let mut state = model.init(&Profile::default());
    let at0 = at(0, 0);
    let card = model.review_lexeme(
        &mut state,
        &sense,
        LexicalDirection::Recognize,
        true,
        at0,
        LexicalSource::Bundle,
    );
    assert_eq!(card.review_count, 1);
    assert_eq!(card.exposures, 0.0);
    assert!(model.lexical_recall(&state, &sense, LexicalDirection::Recognize, at0) > 0.99);
    // An unknown placement claim creates nothing; a known one seeds both
    // directions, and never overwrites the direct card.
    let other = LexemeId::new("en.lex.dog.animal").unwrap();
    model.placement_lexeme_prior(&mut state, &other, 2, false, at0);
    assert!(
        state
            .lexical_card(&other, LexicalDirection::Recognize)
            .is_none()
    );
    model.placement_lexeme_prior(&mut state, &other, 2, true, at0);
    let recognize = state
        .lexical_card(&other, LexicalDirection::Recognize)
        .unwrap();
    assert_eq!(recognize.review_count, 1);
    assert!((recognize.stability - 3.0).abs() < f64::EPSILON);
    assert_eq!(
        state
            .lexical_card(&other, LexicalDirection::Produce)
            .unwrap()
            .direction,
        LexicalDirection::Produce
    );
    model.placement_lexeme_prior(&mut state, &sense, 9, true, at0);
    assert_eq!(
        state
            .lexical_card(&sense, LexicalDirection::Recognize)
            .unwrap()
            .review_count,
        1,
        "a prior does not replace direct evidence"
    );
}

#[test]
fn coverage_counts_only_directly_reviewed_recognize_cards() {
    let model = KnowledgeModel::default();
    let mut state = model.init(&Profile::default());
    let known = LexemeId::new("en.lex.cat.animal").unwrap();
    let passive = LexemeId::new("en.lex.dog.animal").unwrap();
    model.review_lexeme(
        &mut state,
        &known,
        LexicalDirection::Recognize,
        true,
        at(0, 0),
        LexicalSource::Bundle,
    );
    model.review_lexeme(
        &mut state,
        &passive,
        LexicalDirection::Recognize,
        true,
        at(0, 0),
        LexicalSource::Bundle,
    );
    // A passive-only card has review_count 0 even if it exists.
    state
        .lexical_cards
        .get_mut(&passive)
        .unwrap()
        .get_mut(&LexicalDirection::Recognize)
        .unwrap()
        .review_count = 0;
    let token = |id: &str, lexeme: Option<&LexemeId>| BookToken {
        id: id.into(),
        text: "x".into(),
        lemma: "x".into(),
        lexeme_id: lexeme.cloned(),
        sense_ids: Vec::new(),
        kc_ids: Vec::new(),
    };
    let book = Book {
        id: BookId::new("book.first-day").unwrap(),
        version: "a".repeat(64),
        title: Localized::default(),
        level: LevelKey::new("A1").unwrap(),
        license: "original".into(),
        source: "test".into(),
        chapters: vec![BookChapter {
            id: "chapter.first-day.1".into(),
            title: Localized::default(),
            paragraphs: vec![BookParagraph {
                id: "para.first-day.1.1".into(),
                sentences: vec![BookSentence {
                    id: "sentence.first-day.1.1.1".into(),
                    text: "x x x x".into(),
                    tokens: vec![
                        token("t1", Some(&known)),
                        token("t2", Some(&passive)),
                        token("t3", None),
                        token("t4", Some(&known)),
                    ],
                }],
            }],
        }],
    };
    let c = coverage(&book, &state, &model, at(0, 0));
    assert_eq!(
        c,
        Coverage {
            known_tokens: 2,
            total_tokens: 3,
            percent: 2.0 / 3.0,
        }
    );
}

#[test]
fn the_synthetic_log_still_folds_reproducibly_with_batch4_events() {
    let b = bundle();
    let log = synthetic_log(
        &b,
        LogSpec {
            events: 300,
            seed: 42,
        },
    );
    let has_records = log.iter().any(|e| match e {
        Decoded::Known(ev) => matches!(
            ev.payload,
            EventPayload::NoteSaved(_)
                | EventPayload::CustomLexemeCreated(_)
                | EventPayload::BookMarkCreated(_)
        ),
        Decoded::Unknown(_) => false,
    });
    assert!(has_records, "the generator must exercise Batch 4 events");
    let state = fold(&b, &log);
    assert_eq!(state.hash(), fold(&b, &log).hash());
    assert!(serde_json::to_string(&json!({ "hash": state.hash() })).is_ok());
}
