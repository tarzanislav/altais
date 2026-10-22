//! Statistical timing of the key operations (`cargo bench -p altais-crdt-bench`).
//! The full comparison with sizes is the `crdt-report` binary.

use altais_crdt_bench::paragraphs::{LoroParagraphs, ParagraphEngine, YrsParagraphs, type_note};
use altais_crdt_bench::trace::{editing, typing};
use altais_crdt_bench::{Engine, LoroEngine, YrsEngine};
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

const WORDS: usize = 2_000;

fn engine<E: Engine>(c: &mut Criterion) {
    let typed = typing(WORDS, 1);
    let session = editing(typed.len(), 2_000, 2);
    let mut group = c.benchmark_group(E::NAME);

    group.bench_function("type 2000 words", |b| {
        b.iter_batched(
            || E::new(1),
            |mut d| {
                for tx in &typed {
                    E::apply(&mut d, tx);
                }
                d
            },
            BatchSize::LargeInput,
        )
    });

    let mut doc = E::new(1);
    for tx in typed.iter().chain(&session) {
        E::apply(&mut doc, tx);
    }
    let snapshot = E::snapshot(&doc);
    group.bench_function("open note", |b| b.iter(|| E::text(&E::load(&snapshot, 2))));
    group.bench_function("save note", |b| b.iter(|| E::snapshot(&doc)));
    group.finish();
}

fn paragraphs<E: ParagraphEngine>(c: &mut Criterion) {
    c.bench_function(
        &format!("{} / type 2000 words as paragraphs", E::NAME),
        |b| {
            b.iter_batched(
                || E::new(1),
                |mut d| {
                    type_note::<E>(&mut d, WORDS, 1);
                    d
                },
                BatchSize::LargeInput,
            )
        },
    );
}

fn all(c: &mut Criterion) {
    engine::<LoroEngine>(c);
    engine::<YrsEngine>(c);
    paragraphs::<LoroParagraphs>(c);
    paragraphs::<YrsParagraphs>(c);
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(20);
    targets = all
}
criterion_main!(benches);
