//! Both engines must produce the reference text for the same traces, keep it
//! through snapshots and converge after concurrent edits.

use altais_crdt_bench::trace::{apply_to_string, editing, typing};
use altais_crdt_bench::{Engine, LoroEngine, YrsEngine};

fn reference(traces: &[&[Vec<altais_crdt_bench::Op>]]) -> String {
    let mut text = Vec::new();
    for trace in traces {
        for tx in trace.iter() {
            apply_to_string(&mut text, tx);
        }
    }
    text.into_iter().collect()
}

fn same_text_as_reference<E: Engine>() {
    let typed = typing(300, 11);
    let session = editing(typed.len(), 1_500, 12);
    let mut doc = E::new(1);
    for tx in typed.iter().chain(&session) {
        E::apply(&mut doc, tx);
    }
    assert_eq!(E::text(&doc), reference(&[&typed, &session]), "{}", E::NAME);
    assert!(
        E::bold_runs(&doc) > 0,
        "{}: bold formatting missing",
        E::NAME
    );

    // Snapshots restore the same note, and the compact form is not larger.
    let snapshot = E::snapshot(&doc);
    assert_eq!(
        E::text(&E::load(&snapshot, 2)),
        E::text(&doc),
        "{}",
        E::NAME
    );
    assert!(!E::compact_snapshot(&doc).is_empty());
}

fn converges_after_offline_edits<E: Engine>() {
    let typed = typing(300, 21);
    let mut a = E::new(1);
    for tx in &typed {
        E::apply(&mut a, tx);
    }
    let mut b = E::load(&E::snapshot(&a), 2);
    let (va, vb) = (E::version(&a), E::version(&b));

    for tx in &editing(typed.len(), 500, 22) {
        E::apply(&mut a, tx);
    }
    for tx in &editing(typed.len(), 500, 23) {
        E::apply(&mut b, tx);
    }
    let from_a = E::updates_since(&a, &vb);
    let from_b = E::updates_since(&b, &va);
    E::import(&mut a, &from_b);
    E::import(&mut b, &from_a);
    assert_eq!(
        E::text(&a),
        E::text(&b),
        "{}: devices differ after merging",
        E::NAME
    );
    assert_eq!(E::bold_runs(&a), E::bold_runs(&b), "{}", E::NAME);
}

#[test]
fn loro_matches_reference() {
    same_text_as_reference::<LoroEngine>();
}

#[test]
fn yrs_matches_reference() {
    same_text_as_reference::<YrsEngine>();
}

#[test]
fn loro_converges() {
    converges_after_offline_edits::<LoroEngine>();
}

#[test]
fn yrs_converges() {
    converges_after_offline_edits::<YrsEngine>();
}

fn paragraphs_round_trip<E: altais_crdt_bench::paragraphs::ParagraphEngine>() {
    let mut doc = E::new(1);
    altais_crdt_bench::paragraphs::type_note::<E>(&mut doc, 200, 5);
    let text = E::text(&doc);
    assert_eq!(text.lines().count(), 5, "{}", E::NAME);
    assert_eq!(
        E::text(&E::load(&E::snapshot(&doc), 2)),
        text,
        "{}",
        E::NAME
    );
}

#[test]
fn paragraph_structures_round_trip_and_agree() {
    use altais_crdt_bench::paragraphs::{
        LoroParagraphs, ParagraphEngine, YrsParagraphs, type_note,
    };
    paragraphs_round_trip::<LoroParagraphs>();
    paragraphs_round_trip::<YrsParagraphs>();
    let mut a = LoroParagraphs::new(1);
    let mut b = YrsParagraphs::new(1);
    type_note::<LoroParagraphs>(&mut a, 300, 9);
    type_note::<YrsParagraphs>(&mut b, 300, 9);
    assert_eq!(LoroParagraphs::text(&a), YrsParagraphs::text(&b));
}
