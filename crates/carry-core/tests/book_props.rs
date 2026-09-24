use std::collections::BTreeMap;

use carry_core::{CarryError, Level, LevelUpdate, OrderBook, Qty, Side, Tick};
use proptest::prelude::*;
use rust_decimal::Decimal;

const TOLERANCE: Decimal = Decimal::from_parts(1, 0, 0, false, 18);

fn to_levels(map: &BTreeMap<u64, i64>) -> Result<Vec<Level>, CarryError> {
    map.iter()
        .map(|(&tick, &size)| {
            Ok(Level {
                tick: Tick::new(tick),
                qty: Qty::new(Decimal::new(size, 0))?,
            })
        })
        .collect()
}

fn to_updates(raw: &[(bool, u64, i64)]) -> Result<Vec<LevelUpdate>, CarryError> {
    raw.iter()
        .map(|&(is_bid, tick, size)| {
            Ok(LevelUpdate {
                side: if is_bid { Side::Bid } else { Side::Ask },
                tick: Tick::new(tick),
                qty: Qty::new(Decimal::new(size, 0))?,
            })
        })
        .collect()
}

fn book_with(
    bids: &BTreeMap<u64, i64>,
    asks: &BTreeMap<u64, i64>,
) -> Result<OrderBook, CarryError> {
    let mut book = OrderBook::new(Decimal::ONE)?;
    book.apply_snapshot(0, &to_levels(bids)?, &to_levels(asks)?)?;
    Ok(book)
}

fn side(ticks: std::ops::Range<u64>) -> impl Strategy<Value = BTreeMap<u64, i64>> {
    prop::collection::btree_map(ticks, 1i64..100, 1..20)
}

fn batch() -> impl Strategy<Value = Vec<(bool, u64, i64)>> {
    prop::collection::vec((any::<bool>(), 1u64..2000, 0i64..100), 0..5)
}

proptest! {
    #[test]
    fn synced_book_is_never_crossed(
        bids in side(1..1000),
        asks in side(1000..2000),
        batches in prop::collection::vec(batch(), 0..50),
    ) {
        let mut book = book_with(&bids, &asks).unwrap();
        for (seq, raw) in (1u64..).zip(&batches) {
            let result = book.apply_delta(seq, &to_updates(raw).unwrap());
            if !book.is_synced() {
                let crossed = matches!(result, Err(CarryError::CrossedBook { .. }));
                prop_assert!(crossed, "unsynced for another reason: {:?}", result);
                break;
            }
            if let (Some(bid), Some(ask)) = (book.best_bid(), book.best_ask()) {
                prop_assert!(bid.tick < ask.tick);
            }
        }
    }

    #[test]
    fn bigger_buy_never_gets_better_avg_price(
        asks in side(1..1000),
        a in 1i64..10_000,
        b in 1i64..10_000,
    ) {
        let book = book_with(&BTreeMap::new(), &asks).unwrap();
        let (small, large) = (a.min(b), a.max(b));
        let (Ok(s), Ok(l)) = (
            book.walk(Side::Ask, Decimal::new(small, 0)),
            book.walk(Side::Ask, Decimal::new(large, 0)),
        ) else {
            return Ok(()); // not enough liquidity for this case
        };
        prop_assert!(l.avg_price + TOLERANCE >= s.avg_price);
    }

    #[test]
    fn sell_avg_price_is_between_best_and_worst(
        bids in side(1..1000),
        notional in 1i64..10_000,
    ) {
        let book = book_with(&bids, &BTreeMap::new()).unwrap();
        let Ok(fill) = book.walk(Side::Bid, Decimal::new(notional, 0)) else {
            return Ok(());
        };
        prop_assert!(fill.avg_price <= fill.best_price + TOLERANCE);
        prop_assert!(fill.avg_price + TOLERANCE >= fill.worst_price);
        prop_assert!(fill.slippage_bps >= Decimal::ZERO);
        prop_assert!(fill.levels_used <= bids.len());
    }
}
