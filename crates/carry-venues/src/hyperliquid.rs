use carry_core::{Level, Price, Qty, Tick};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::value::RawValue;

use crate::{BookEvent, VenueError};

#[derive(Debug)]
pub enum HlMessage {
    L2Book(HlBook),
    SubscriptionResponse,
    Pong,
    Other(String),
}

#[derive(Deserialize)]
struct Envelope<'a> {
    channel: &'a str,
    #[serde(borrow)]
    data: Option<&'a RawValue>,
}

impl HlMessage {
    pub fn parse(text: &str) -> Result<Self, VenueError> {
        let envelope: Envelope<'_> = serde_json::from_str(text)?;
        let data = envelope.data.map_or("null", RawValue::get);
        Ok(match envelope.channel {
            "l2Book" => Self::L2Book(serde_json::from_str(data)?),
            "subscriptionResponse" => Self::SubscriptionResponse,
            "pong" => Self::Pong,
            other => Self::Other(other.to_owned()),
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct HlBook {
    pub coin: String,
    pub time: u64,
    pub levels: [Vec<HlLevel>; 2],
}

#[derive(Debug, Deserialize)]
pub struct HlLevel {
    #[serde(with = "rust_decimal::serde::str")]
    pub px: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub sz: Decimal,
    pub n: u32,
}

impl HlBook {
    pub fn to_levels(&self, tick_size: Decimal) -> Result<(Vec<Level>, Vec<Level>), VenueError> {
        let [bids, asks] = &self.levels;
        Ok((convert(bids, tick_size)?, convert(asks, tick_size)?))
    }
}

fn convert(levels: &[HlLevel], tick_size: Decimal) -> Result<Vec<Level>, VenueError> {
    levels
        .iter()
        .map(|level| {
            Ok(Level {
                tick: Tick::from_price(Price::new(level.px)?, tick_size)?,
                qty: Qty::new(level.sz)?,
            })
        })
        .collect()
}

/// Turns one text frame into a book event, or `None` for non-book messages.
pub fn to_event(text: &str, tick_size: Decimal) -> Result<Option<BookEvent>, VenueError> {
    match HlMessage::parse(text)? {
        HlMessage::L2Book(book) => {
            let (bids, asks) = book.to_levels(tick_size)?;
            Ok(Some(BookEvent::Snapshot {
                seq: book.time,
                bids,
                asks,
            }))
        }
        HlMessage::SubscriptionResponse | HlMessage::Pong | HlMessage::Other(_) => Ok(None),
    }
}
