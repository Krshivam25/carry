mod book_owner;

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use carry_core::{OrderBook, Side};
use carry_venues::{FeedConfig, run_feed};
use rust_decimal::Decimal;
use tokio::sync::{Notify, mpsc, watch};
use tokio::task::JoinSet;
use tokio::time::{interval, timeout};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::book_owner::run_book_owner;

const CHANNEL_CAPACITY: usize = 256;
const REPORT_EVERY: Duration = Duration::from_secs(10);
const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

#[tokio::main]
async fn main() -> Result<()> {
    // Log level from RUST_LOG (e.g. RUST_LOG=debug), default "info".
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let shutdown = CancellationToken::new();
    let mut tasks = JoinSet::new();
    let tick = Decimal::new(1, 1); // BTC on both venues
    let hl = spawn_venue(&mut tasks, FeedConfig::hyperliquid("BTC", tick), &shutdown)?;
    let lighter = spawn_venue(&mut tasks, FeedConfig::lighter(1, tick), &shutdown)?;
    info!("carryd started");

    let notional = Decimal::new(50_000, 0);
    let mut report = interval(REPORT_EVERY);
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    loop {
        tokio::select! {
            _ = report.tick() => {
                // Take the Arc and release the watch borrow at once: never hold it.
                let hl_book = Arc::clone(&hl.borrow());
                let lighter_book = Arc::clone(&lighter.borrow());
                log_executable("hyperliquid", &hl_book, notional);
                log_executable("lighter", &lighter_book, notional);
            }
            _ = &mut ctrl_c => break,
        }
    }

    // Graceful shutdown: cancel feeds -> they drop their senders -> owners see the
    // channel close and exit. Wait for all of them, but not forever.
    info!("shutting down");
    shutdown.cancel();
    let drained = timeout(SHUTDOWN_GRACE, async {
        while let Some(joined) = tasks.join_next().await {
            if let Err(err) = joined {
                warn!(%err, "task failed");
            }
        }
    })
    .await;
    if drained.is_err() {
        warn!("tasks did not stop in time; aborting");
        tasks.abort_all();
    }
    info!("stopped cleanly");
    Ok(())
}

/// Starts a feed task and a book-owner task; returns the book's watch receiver.
fn spawn_venue(
    tasks: &mut JoinSet<()>,
    cfg: FeedConfig,
    shutdown: &CancellationToken,
) -> Result<watch::Receiver<Arc<OrderBook>>> {
    let venue = cfg.venue;
    let book = OrderBook::new(cfg.tick_size)?;
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
    let (publish, view) = watch::channel(Arc::new(book.clone()));
    let resync = Arc::new(Notify::new());
    tasks.spawn(run_feed(cfg, tx, Arc::clone(&resync), shutdown.clone()));
    tasks.spawn(run_book_owner(venue, book, rx, publish, resync));
    Ok(view)
}

fn log_executable(venue: &str, book: &OrderBook, notional: Decimal) {
    if !book.is_synced() {
        warn!(venue, "book stale");
        return;
    }
    match (
        book.walk(Side::Ask, notional),
        book.walk(Side::Bid, notional),
    ) {
        (Ok(buy), Ok(sell)) => info!(
            venue,
            %notional,
            buy_avg = %buy.avg_price.round_dp(1),
            buy_bps = %buy.slippage_bps.round_dp(2),
            sell_avg = %sell.avg_price.round_dp(1),
            sell_bps = %sell.slippage_bps.round_dp(2),
            "executable"
        ),
        (Err(err), _) | (_, Err(err)) => warn!(venue, %err, "walk failed"),
    }
}
