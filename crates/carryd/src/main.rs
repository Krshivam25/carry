mod book_owner;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use carry_core::{OrderBook, Side};
use carry_venues::{FeedConfig, run_feed};
use rust_decimal::Decimal;
use tokio::sync::{Notify, mpsc, watch};
use tokio::time::interval;

use crate::book_owner::run_book_owner;

const CHANNEL_CAPACITY: usize = 256;

#[tokio::main]
async fn main() -> Result<()> {
    let tick = Decimal::new(1, 1);
    let hl = spawn_venue(FeedConfig::hyperliquid("BTC", tick))?;
    let lighter = spawn_venue(FeedConfig::lighter(1, tick))?;

    let notional = Decimal::new(50_000, 0);
    let mut report = interval(Duration::from_secs(1));
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            _ = report.tick() => {
                let hl_book = Arc::clone(&hl.borrow());
                let lighter_book = Arc::clone(&lighter.borrow());
                println!("{}", describe("hl", &hl_book, notional));
                println!("{}", describe("lighter", &lighter_book, notional));
            }
            _ = &mut ctrl_c => break,
        }
    }
    Ok(())
}

fn spawn_venue(cfg: FeedConfig) -> Result<watch::Receiver<Arc<OrderBook>>> {
    let book = OrderBook::new(cfg.tick_size)?;
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
    let (publish, view) = watch::channel(Arc::new(book.clone()));
    let resync = Arc::new(Notify::new());
    tokio::spawn(run_feed(cfg, tx, Arc::clone(&resync)));
    tokio::spawn(run_book_owner(book, rx, publish, resync));
    Ok(view)
}

fn describe(name: &str, book: &OrderBook, notional: Decimal) -> String {
    if !book.is_synced() {
        return format!("{name:<8} stale");
    }
    match (
        book.walk(Side::Ask, notional),
        book.walk(Side::Bid, notional),
    ) {
        (Ok(buy), Ok(sell)) => format!(
            "{name:<8} ${notional} buy avg {:.1} ({:.2} bps) | sell avg {:.1} ({:.2} bps)",
            buy.avg_price, buy.slippage_bps, sell.avg_price, sell.slippage_bps
        ),
        (Err(err), _) | (_, Err(err)) => format!("{name:<8} {err}"),
    }
}
