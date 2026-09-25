use carry_core::Venue;
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{Notify, mpsc};
use tokio::time::{MissedTickBehavior, interval, sleep};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::backoff::backoff_delay;
use crate::{BookEvent, VenueError, VenueEvent, hyperliquid, lighter};

const PING_EVERY: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct FeedConfig {
    pub venue: Venue,
    pub url: String,
    pub subscribe: String,
    pub ping: String,
    pub tick_size: Decimal,
}

impl FeedConfig {
    pub fn hyperliquid(coin: &str, tick_size: Decimal) -> Self {
        Self {
            venue: Venue::Hyperliquid,
            url: "wss://api.hyperliquid.xyz/ws".to_owned(),
            subscribe: json!({
                "method": "subscribe",
                "subscription": { "type": "l2Book", "coin": coin }
            })
            .to_string(),
            ping: json!({ "method": "ping" }).to_string(),
            tick_size,
        }
    }

    pub fn lighter(market_id: u32, tick_size: Decimal) -> Self {
        Self {
            venue: Venue::Lighter,
            url: "wss://mainnet.zklighter.elliot.ai/stream".to_owned(),
            subscribe: json!({
                "type": "subscribe",
                "channel": format!("order_book/{market_id}")
            })
            .to_string(),
            ping: json!({ "type": "ping" }).to_string(),
            tick_size,
        }
    }

    fn to_event(&self, text: &str) -> Result<Option<BookEvent>, VenueError> {
        match self.venue {
            Venue::Hyperliquid => hyperliquid::to_event(text, self.tick_size),
            Venue::Lighter => lighter::to_event(text, self.tick_size),
        }
    }
}

pub async fn run_feed(cfg: FeedConfig, tx: mpsc::Sender<VenueEvent>, resync: Arc<Notify>) {
    let mut attempt = 0u32;
    loop {
        match session(&cfg, &tx, &resync, &mut attempt).await {
            Ok(()) => return,
            Err(err) => eprintln!("[{}] feed error: {err}", cfg.venue),
        }
        let disconnected = VenueEvent {
            venue: cfg.venue,
            event: BookEvent::Disconnected,
        };
        if tx.send(disconnected).await.is_err() {
            return;
        }
        let delay = backoff_delay(attempt, rand::random());
        attempt = attempt.saturating_add(1);
        eprintln!("[{}] reconnecting in {delay:?}", cfg.venue);
        sleep(delay).await;
    }
}

async fn session(
    cfg: &FeedConfig,
    tx: &mpsc::Sender<VenueEvent>,
    resync: &Notify,
    attempt: &mut u32,
) -> Result<(), VenueError> {
    let (ws, _response) = connect_async(cfg.url.as_str()).await?;
    let (mut write, mut read) = ws.split();
    write.send(Message::text(cfg.subscribe.as_str())).await?;

    let mut ping = interval(PING_EVERY);

    ping.set_missed_tick_behavior(MissedTickBehavior::Delay);
    ping.tick().await;

    loop {
        tokio::select! {
                frame = read.next() => {
                    let Some(frame) = frame else {
                        return Err(VenueError::Closed);
                    };
                     match frame? {
                        Message::Text(text) => {
                            if let Some(event) = cfg.to_event(&text)? {
                                *attempt = 0;
                                let msg = VenueEvent { venue: cfg.venue, event };
                                match tx.try_send(msg) {
                                    Ok(()) => {}
                                    Err(TrySendError::Full(_)) => return Err(VenueError::Overflow),
                                    Err(TrySendError::Closed(_)) => return Ok(()),
                                }
                            }
                        }
                        Message::Close(_) => return Err(VenueError::Closed),
                        _ => {}
                }
            }
            _ = ping.tick() => {
                    write.send(Message::text(cfg.ping.as_str())).await?;
            }

            () = resync.notified() => return Err(VenueError::ResyncRequested),
        }
    }
}
