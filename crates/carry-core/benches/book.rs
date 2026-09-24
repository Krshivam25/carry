#![allow(clippy::unwrap_used)]
use std::hint::black_box;

use carry_core::{Level, LevelUpdate, OrderBook, Qty, Side, Tick};
use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use rust_decimal::Decimal;

fn size(i: u64) -> Qty {
    Qty::new(Decimal::new((i % 5 + 1) as i64, 1)).unwrap()
}

fn realistic_book(n: u64) -> OrderBook {
    let bids: Vec<Level> = (0..n)
        .map(|i| Level {
            tick: Tick::new(9_999 - i),
            qty: size(i),
        })
        .collect();
    let asks: Vec<Level> = (0..n)
        .map(|i| Level {
            tick: Tick::new(10_001 + i),
            qty: size(i),
        })
        .collect();
    let mut book = OrderBook::new(Decimal::ONE).unwrap();
    book.apply_snapshot(1, &bids, &asks).unwrap();
    book
}

fn bench_apply_delta(c: &mut Criterion) {
    let book = realistic_book(50);
    let updates = [
        LevelUpdate {
            side: Side::Bid,
            tick: Tick::new(9_999),
            qty: size(3),
        },
        LevelUpdate {
            side: Side::Ask,
            tick: Tick::new(10_000),
            qty: size(1),
        },
        LevelUpdate {
            side: Side::Ask,
            tick: Tick::new(10_005),
            qty: Qty::new(Decimal::ZERO).unwrap(),
        },
    ];
    c.bench_function("apply_delta/3_updates", |b| {
        b.iter_batched(
            || book.clone(),
            |mut book| {
                let result = book.apply_delta(black_box(2), black_box(&updates));
                black_box(result).unwrap();
                book
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_walk(c: &mut Criterion) {
    let book = realistic_book(50);
    let mut group = c.benchmark_group("walk_ask");
    for usd in [1_000i64, 10_000, 100_000] {
        group.bench_with_input(BenchmarkId::from_parameter(usd), &usd, |b, &usd| {
            let notional = Decimal::new(usd, 0);
            b.iter(|| black_box(book.walk(Side::Ask, black_box(notional))).unwrap());
        });
    }
    group.finish();
}

criterion_group!(benches, bench_apply_delta, bench_walk);
criterion_main!(benches);
