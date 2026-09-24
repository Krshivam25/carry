use crate::{CarryError, Qty, Side, Tick};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    pub tick: Tick,
    pub qty: Qty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelUpdate {
    pub side: Side,
    pub tick: Tick,
    pub qty: Qty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fill {
    pub base_qty: Decimal,
    pub notional: Decimal,
    pub avg_price: Decimal,
    pub best_price: Decimal,
    pub worst_price: Decimal,
    pub levels_used: usize,
    pub slippage_bps: Decimal,
}
#[derive(Debug, Clone)]
pub struct OrderBook {
    tick_size: Decimal,
    bids: BTreeMap<Tick, Qty>,
    asks: BTreeMap<Tick, Qty>,
    seq: Option<u64>,
}

impl OrderBook {
    pub fn new(tick_size: Decimal) -> Result<Self, CarryError> {
        if tick_size <= Decimal::ZERO {
            return Err(CarryError::InvalidTickSize(tick_size));
        }
        Ok(Self {
            tick_size,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            seq: None,
        })
    }

    pub fn tick_size(&self) -> Decimal {
        self.tick_size
    }

    pub fn seq(&self) -> Option<u64> {
        self.seq
    }

    pub fn is_synced(&self) -> bool {
        self.seq.is_some()
    }

    pub fn mark_stale(&mut self) {
        self.seq = None;
    }

    pub fn apply_snapshot(
        &mut self,
        seq: u64,
        bids: &[Level],
        asks: &[Level],
    ) -> Result<(), CarryError> {
        self.bids.clear();
        self.asks.clear();
        for &level in bids {
            Self::set_level(&mut self.bids, level.tick, level.qty);
        }
        for &level in asks {
            Self::set_level(&mut self.asks, level.tick, level.qty);
        }
        self.seq = Some(seq);
        self.check_not_crossed()
    }

    pub fn apply_delta(&mut self, seq: u64, updates: &[LevelUpdate]) -> Result<(), CarryError> {
        let last = self.seq.ok_or(CarryError::NotSynced)?;
        let expected = last.checked_add(1).ok_or(CarryError::Overflow)?;
        if seq != expected {
            self.seq = None;
            return Err(CarryError::SequenceGap { expected, got: seq });
        }
        self.apply_levels(seq, updates)
    }

    pub fn apply_delta_from(
        &mut self,
        prev_seq: u64,
        seq: u64,
        updates: &[LevelUpdate],
    ) -> Result<(), CarryError> {
        let last = self.seq.ok_or(CarryError::NotSynced)?;
        if prev_seq != last {
            self.seq = None;
            return Err(CarryError::SequenceGap {
                expected: last,
                got: prev_seq,
            });
        }
        if seq < prev_seq {
            self.seq = None;
            return Err(CarryError::SequenceGap {
                expected: prev_seq,
                got: seq,
            });
        }
        self.apply_levels(seq, updates)
    }

    fn apply_levels(&mut self, seq: u64, updates: &[LevelUpdate]) -> Result<(), CarryError> {
        for update in updates {
            let levels = match update.side {
                Side::Bid => &mut self.bids,
                Side::Ask => &mut self.asks,
            };
            Self::set_level(levels, update.tick, update.qty);
        }
        self.seq = Some(seq);
        self.check_not_crossed()
    }
    pub fn best_bid(&self) -> Option<Level> {
        self.bids
            .last_key_value()
            .map(|(&tick, &qty)| Level { tick, qty })
    }

    pub fn best_ask(&self) -> Option<Level> {
        self.asks
            .first_key_value()
            .map(|(&tick, &qty)| Level { tick, qty })
    }

    pub fn walk(&self, side: Side, notional: Decimal) -> Result<Fill, CarryError> {
        if !self.is_synced() {
            return Err(CarryError::NotSynced);
        }
        if notional <= Decimal::ZERO {
            return Err(CarryError::InvalidNotional(notional));
        }
        match side {
            Side::Ask => walk_levels(self.asks.iter(), side, self.tick_size, notional),
            Side::Bid => walk_levels(self.bids.iter().rev(), side, self.tick_size, notional),
        }
    }

    fn set_level(levels: &mut BTreeMap<Tick, Qty>, tick: Tick, qty: Qty) {
        if qty.is_zero() {
            levels.remove(&tick);
        } else {
            levels.insert(tick, qty);
        }
    }

    fn check_not_crossed(&mut self) -> Result<(), CarryError> {
        if let (Some(bid), Some(ask)) = (self.best_bid(), self.best_ask())
            && bid.tick >= ask.tick
        {
            self.seq = None;
            return Err(CarryError::CrossedBook {
                bid: bid.tick,
                ask: ask.tick,
            });
        }
        Ok(())
    }
}

fn walk_levels<'a>(
    levels: impl Iterator<Item = (&'a Tick, &'a Qty)>,
    side: Side,
    tick_size: Decimal,
    notional: Decimal,
) -> Result<Fill, CarryError> {
    let mut remaining = notional;
    let mut base_qty = Decimal::ZERO;
    let mut best_price = None;
    let mut worst_price = Decimal::ZERO;
    let mut levels_used = 0;

    for (&tick, &qty) in levels {
        let price = tick.to_price(tick_size)?.value();
        let level_notional = price.checked_mul(qty.value()).ok_or(CarryError::Overflow)?;
        let take = remaining.min(level_notional);
        let bought = take.checked_div(price).ok_or(CarryError::Overflow)?;
        base_qty = base_qty.checked_add(bought).ok_or(CarryError::Overflow)?;
        remaining -= take;
        best_price.get_or_insert(price);
        worst_price = price;
        levels_used += 1;
        if remaining.is_zero() {
            break;
        }
    }

    let Some(best) = best_price else {
        return Err(CarryError::InsufficientLiquidity {
            requested: notional,
            available: Decimal::ZERO,
        });
    };
    if !remaining.is_zero() {
        return Err(CarryError::InsufficientLiquidity {
            requested: notional,
            available: notional - remaining,
        });
    }

    let avg_price = notional.checked_div(base_qty).ok_or(CarryError::Overflow)?;
    let worse_by = match side {
        Side::Ask => avg_price - best,
        Side::Bid => best - avg_price,
    }
    .max(Decimal::ZERO);
    let slippage_bps = worse_by
        .checked_mul(Decimal::from(10_000))
        .and_then(|x| x.checked_div(best))
        .ok_or(CarryError::Overflow)?;

    Ok(Fill {
        base_qty,
        notional,
        avg_price,
        best_price: best,
        worst_price,
        levels_used,
        slippage_bps,
    })
}
