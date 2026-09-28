use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use carry_core::OrderBook;
use carry_venues::{FeedConfig, run_feed};
use tokio::sync::{Notify, mpsc, watch};
use tokio::task::JoinSet;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::book_owner::run_book_owner;

const CHANNEL_CAPACITY: usize = 256;

const SHUTDOWN_GRACE: Duration = Duration::from_secs(5);

pub fn spawn_venue(
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

pub async fn stop(mut tasks: JoinSet<()>, shutdown: CancellationToken) {
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
}
