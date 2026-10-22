//! Prints the gate G0 comparison of Loro and Yrs as a Markdown table.
//!
//! Run in release mode: `cargo run --release -p altais-crdt-bench --bin crdt-report`
//! Add `--quick` for a smaller run (used by the tests).

use std::hint::black_box;
use std::time::{Duration, Instant};

use altais_crdt_bench::paragraphs::{LoroParagraphs, ParagraphEngine, YrsParagraphs, type_note};
use altais_crdt_bench::trace::{Rng, editing, note_text, typing};
use altais_crdt_bench::{Engine, LoroEngine, Op, YrsEngine};

struct Sizes {
    long_words: usize,
    session_edits: usize,
    offline_edits: usize,
    archive_notes: usize,
    runs: usize,
}

const FULL: Sizes = Sizes {
    long_words: 10_000,
    session_edits: 20_000,
    offline_edits: 2_000,
    archive_notes: 10_000,
    runs: 5,
};

const QUICK: Sizes = Sizes {
    long_words: 500,
    session_edits: 1_000,
    offline_edits: 200,
    archive_notes: 200,
    runs: 1,
};

/// Median duration of `runs` executions; `setup` is not timed.
fn median<S, T>(
    runs: usize,
    mut setup: impl FnMut() -> S,
    mut work: impl FnMut(S) -> T,
) -> Duration {
    let mut times: Vec<Duration> = (0..runs)
        .map(|_| {
            let input = setup();
            let start = Instant::now();
            black_box(work(input));
            start.elapsed()
        })
        .collect();
    times.sort();
    times[times.len() / 2]
}

fn ms(d: Duration) -> String {
    let ms = d.as_secs_f64() * 1000.0;
    if ms >= 100.0 {
        format!("{ms:.0} ms")
    } else if ms >= 1.0 {
        format!("{ms:.1} ms")
    } else {
        format!("{:.0} µs", d.as_secs_f64() * 1e6)
    }
}

fn kb(bytes: usize) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} kB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn apply_all<E: Engine>(doc: &mut E::Doc, trace: &[Vec<Op>]) {
    for tx in trace {
        E::apply(doc, tx);
    }
}

fn measure<E: Engine>(s: &Sizes) -> Vec<String> {
    let mut v = Vec::new();
    let typed = typing(s.long_words, 1);
    let session = editing(typed.len(), s.session_edits, 2);

    // 1. Typing a long note keystroke by keystroke.
    let t = median(
        s.runs,
        || E::new(1),
        |mut d| {
            apply_all::<E>(&mut d, &typed);
            d
        },
    );
    v.push(ms(t));
    v.push(format!(
        "{:.0} k/s",
        typed.len() as f64 / t.as_secs_f64() / 1000.0
    ));

    // 2. Editing session on it.
    let mut long = E::new(1);
    apply_all::<E>(&mut long, &typed);
    let base = E::snapshot(&long);
    let t = median(
        s.runs,
        || E::load(&base, 1),
        |mut d| {
            apply_all::<E>(&mut d, &session);
            d
        },
    );
    v.push(ms(t));
    apply_all::<E>(&mut long, &session);

    // 3. Size on disk.
    let text = E::text(&long);
    let snapshot = E::snapshot(&long);
    let compact = E::compact_snapshot(&long);
    v.push(kb(text.len()));
    v.push(kb(snapshot.len()));
    v.push(kb(compact.len()));

    // 4. Opening the note: load the snapshot and read the text.
    let t = median(s.runs * 3, || (), |_| E::text(&E::load(&snapshot, 3)));
    v.push(ms(t));

    // 5. Saving: export the snapshot.
    let t = median(s.runs * 3, || (), |_| E::snapshot(&long));
    v.push(ms(t));

    // 6. One keystroke as a sync message.
    let mut d = E::load(&snapshot, 4);
    let mut total = 0;
    let len = E::text(&d).chars().count();
    for i in 0..100 {
        total += E::apply_and_encode(
            &mut d,
            &[Op::Insert {
                pos: (len / 2) + i,
                text: "a".into(),
            }],
        )
        .len();
    }
    v.push(format!("{} B", total / 100));

    // 7. Two devices edit offline, then exchange and merge.
    let edits_a = editing(text.chars().count(), s.offline_edits, 3);
    let edits_b = editing(text.chars().count(), s.offline_edits, 4);
    let mut a = E::load(&snapshot, 5);
    let mut b = E::load(&snapshot, 6);
    let (va, vb) = (E::version(&a), E::version(&b));
    apply_all::<E>(&mut a, &edits_a);
    apply_all::<E>(&mut b, &edits_b);
    let from_a = E::updates_since(&a, &vb);
    let from_b = E::updates_since(&b, &va);
    v.push(kb(from_a.len()));
    let t = median(
        s.runs,
        || E::load(&E::snapshot(&b), 6),
        |mut target| {
            E::import(&mut target, &from_a);
            target
        },
    );
    v.push(ms(t));
    E::import(&mut a, &from_b);
    E::import(&mut b, &from_a);
    assert_eq!(
        E::text(&a),
        E::text(&b),
        "{}: devices differ after merging",
        E::NAME
    );

    // 8. An imported archive: notes of 30–500 words, each a single insert plus a few edits.
    let mut rng = Rng::new(9);
    let texts: Vec<String> = (0..s.archive_notes)
        .map(|_| note_text(30 + rng.below(470), &mut rng))
        .collect();
    let start = Instant::now();
    let snapshots: Vec<Vec<u8>> = texts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let mut d = E::new(7);
            E::apply(
                &mut d,
                &[Op::Insert {
                    pos: 0,
                    text: t.clone(),
                }],
            );
            let n = t.chars().count();
            apply_all::<E>(&mut d, &editing(n, 5, i as u64 + 100));
            E::snapshot(&d)
        })
        .collect();
    let import_time = start.elapsed();
    let total: usize = snapshots.iter().map(Vec::len).sum();
    let text_total: usize = texts.iter().map(String::len).sum();
    v.push(ms(import_time));
    v.push(format!(
        "{} ({:.2}× text)",
        kb(total),
        total as f64 / text_total as f64
    ));
    let t = median(
        s.runs.min(3),
        || (),
        |_| {
            snapshots
                .iter()
                .map(|b| E::text(&E::load(b, 8)).len())
                .sum::<usize>()
        },
    );
    v.push(ms(t));
    v
}

/// The long note stored as paragraphs, as the editor stores it.
fn measure_paragraphs<E: ParagraphEngine>(s: &Sizes) -> Vec<String> {
    let t = median(
        s.runs,
        || E::new(1),
        |mut d| {
            type_note::<E>(&mut d, s.long_words, 1);
            d
        },
    );
    let mut doc = E::new(1);
    type_note::<E>(&mut doc, s.long_words, 1);
    let snapshot = E::snapshot(&doc);
    let open = median(s.runs * 3, || (), |_| E::text(&E::load(&snapshot, 2)));
    vec![ms(t), kb(snapshot.len()), ms(open)]
}

const PARAGRAPH_LABELS: &[&str] = &[
    "Typing the long note (one text per paragraph)",
    "Stored note with history (snapshot)",
    "Opening the note (load + read text)",
];

const LABELS: &[&str] = &[
    "Typing a long note, keystroke by keystroke",
    "… keystrokes per second",
    "Editing session on it",
    "Text of the note (UTF-8)",
    "Stored note with history (snapshot)",
    "Compact form (Loro: shallow snapshot; Yrs: v2)",
    "Opening the note (load + read text)",
    "Saving the note (export snapshot)",
    "One keystroke as a sync message",
    "Offline edits of one device as a sync message",
    "Merging those edits",
    "Importing an archive (create + save each note)",
    "Archive on disk",
    "Opening every note of the archive",
];

fn main() {
    let quick = std::env::args().any(|a| a == "--quick");
    let s = if quick { QUICK } else { FULL };
    let loro = measure::<LoroEngine>(&s);
    let yrs = measure::<YrsEngine>(&s);

    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| l.starts_with("model name"))
                .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
        })
        .unwrap_or_else(|| "unknown CPU".into());
    println!(
        "Workload: long note of {} words typed keystroke by keystroke ({} keystrokes), then {} edits",
        s.long_words,
        typing(s.long_words, 1).len(),
        s.session_edits
    );
    println!(
        "(70 % typing a word, 20 % deleting, 10 % bold); offline: {} edits per device;",
        s.offline_edits
    );
    println!(
        "archive: {} notes of 30–500 words. Median of {} runs. CPU: {cpu}.",
        s.archive_notes, s.runs
    );
    println!();
    println!(
        "| Measurement | {} | {} |",
        LoroEngine::NAME,
        YrsEngine::NAME
    );
    println!("| --- | ---: | ---: |");
    println!("| **The note as one long text** | | |");
    for (i, label) in LABELS.iter().enumerate() {
        println!("| {label} | {} | {} |", loro[i], yrs[i]);
    }
    let loro_p = measure_paragraphs::<LoroParagraphs>(&s);
    let yrs_p = measure_paragraphs::<YrsParagraphs>(&s);
    println!("| **The same note as paragraphs (how the editor stores it)** | | |");
    for (i, label) in PARAGRAPH_LABELS.iter().enumerate() {
        println!("| {label} | {} | {} |", loro_p[i], yrs_p[i]);
    }
}
