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
