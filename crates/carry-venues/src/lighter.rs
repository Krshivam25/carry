use carry_core::{Level, LevelUpdate, Price, Qty, Side, Tick};
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::VenueError;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum LighterMessage {
    #[serde(rename = "subscribed/order_book")]
    Snapshot(LighterBookMsg),
    #[serde(rename = "update/order_book")]
    Update(LighterBookMsg),
    #[serde(rename = "pong")]
    Pong,
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub struct LighterBookMsg {
    pub channel: String,
    pub order_book: LighterBook,
}

#[derive(Debug, Deserialize)]
pub struct LighterBook {
    pub asks: Vec<LighterLevel>,
    pub bids: Vec<LighterLevel>,
    pub nonce: u64,
    #[serde(default)]
    pub begin_nonce: u64,
}

#[derive(Debug, Deserialize)]
pub struct LighterLevel {
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub size: Decimal,
}

impl LighterBook {
    pub fn to_levels(&self, tick_size: Decimal) -> Result<(Vec<Level>, Vec<Level>), VenueError> {
        let to_level = |l: &LighterLevel| -> Result<Level, VenueError> {
            Ok(Level {
                tick: Tick::from_price(Price::new(l.price)?, tick_size)?,
                qty: Qty::new(l.size)?,
            })
        };
        let bids = self.bids.iter().map(to_level).collect::<Result<_, _>>()?;
        let asks = self.asks.iter().map(to_level).collect::<Result<_, _>>()?;
        Ok((bids, asks))
    }

    pub fn to_updates(&self, tick_size: Decimal) -> Result<Vec<LevelUpdate>, VenueError> {
        let bids = self.bids.iter().map(|l| (Side::Bid, l));
        let asks = self.asks.iter().map(|l| (Side::Ask, l));
        bids.chain(asks)
            .map(|(side, l)| {
                Ok(LevelUpdate {
                    side,
                    tick: Tick::from_price(Price::new(l.price)?, tick_size)?,
                    qty: Qty::new(l.size)?,
                })
            })
            .collect()
    }
}
