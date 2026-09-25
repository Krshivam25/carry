use carry_core::{CarryError, Level, OrderBook, Venue};
use carry_venues::{BookEvent, FeedConfig, run_feed};
use rust_decimal::Decimal;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Notify, mpsc};
use tokio::time::interval;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let tick = Decimal::new(1, 1);
    let (tx, mut rx) = mpsc::channel(1024);
    // This example never requests a resync; carryd does.
    let never = Arc::new(Notify::new());
    tokio::spawn(run_feed(
        FeedConfig::hyperliquid("BTC", tick),
        tx.clone(),
        Arc::clone(&never),
    ));
    tokio::spawn(run_feed(FeedConfig::lighter(1, tick), tx, never));

    let mut hl = OrderBook::new(tick)?;
    let mut lighter = OrderBook::new(tick)?;
    let mut print = interval(Duration::from_secs(1));
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            Some(msg) = rx.recv() => {
                let book = match msg.venue {
                    Venue::Hyperliquid => &mut hl,
                    Venue::Lighter => &mut lighter,
                };
                if let Err(err) = apply(book, msg.event) {
                    eprintln!("[{}] book error: {err}", msg.venue);
                }
            }
            _ = print.tick() => {
                println!("hyperliquid {:<28} lighter {}", top(&hl), top(&lighter));
            }
            _ = &mut ctrl_c => break,
        }
    }
    Ok(())
}

fn apply(book: &mut OrderBook, event: BookEvent) -> Result<(), CarryError> {
    match event {
        BookEvent::Snapshot { seq, bids, asks } => book.apply_snapshot(seq, &bids, &asks),
        BookEvent::Delta {
            prev_seq,
            seq,
            updates,
        } => book.apply_delta_from(prev_seq, seq, &updates),
        BookEvent::Disconnected => {
            book.mark_stale();
            Ok(())
        }
    }
}

fn top(book: &OrderBook) -> String {
    let (Some(bid), Some(ask)) = (book.best_bid(), book.best_ask()) else {
        return "empty".to_owned();
    };
    if !book.is_synced() {
        return "stale".to_owned();
    }
    format!("{} / {}", price(book, bid), price(book, ask))
}

fn price(book: &OrderBook, level: Level) -> String {
    level
        .tick
        .to_price(book.tick_size())
        .map(|p| p.value().to_string())
        .unwrap_or_else(|err| err.to_string())
}
