use carry_core::{CarryError, OrderBook, Venue};
use carry_venues::{BookEvent, VenueEvent};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Notify, mpsc, watch};
use tokio::time::interval;
use tracing::{info, instrument, warn};

const BATCH: usize = 256;
const STATS_EVERY: Duration = Duration::from_secs(60);

#[derive(Debug, Default)]
struct Stats {
    events: u64,
    snapshots: u64,
    resyncs: u64,
    disconnects: u64,
}

#[instrument(name = "book", skip_all, fields(%venue))]
pub async fn run_book_owner(
    venue: Venue,
    mut book: OrderBook,
    mut rx: mpsc::Receiver<VenueEvent>,
    publish: watch::Sender<Arc<OrderBook>>,
    resync: Arc<Notify>,
) {
    let mut stats = Stats::default();
    let mut report = interval(STATS_EVERY);
    report.tick().await;
    let mut batch = Vec::with_capacity(BATCH);

    loop {
        tokio::select! {
            // `recv_many` waits for at least one event, then takes whatever else is queued.
            received = rx.recv_many(&mut batch, BATCH) => {
                if received == 0 {
                    info!(?stats, "feed closed; book owner stopping");
                    return;
                }
                for msg in batch.drain(..) {
                    apply(&mut book, msg.event, &resync, &mut stats);
                }
                publish.send_replace(Arc::new(book.clone()));
            }
            _ = report.tick() => {
                info!(
                    synced = book.is_synced(),
                    seq = ?book.seq(),
                    events = stats.events,
                    snapshots = stats.snapshots,
                    resyncs = stats.resyncs,
                    disconnects = stats.disconnects,
                    "book stats"
                );
            }
        }
    }
}

fn apply(book: &mut OrderBook, event: BookEvent, resync: &Notify, stats: &mut Stats) {
    stats.events += 1;

    let result = match event {
        BookEvent::Snapshot { seq, bids, asks } => {
            stats.snapshots += 1;
            book.apply_snapshot(seq, &bids, &asks)
        }
        BookEvent::Delta {
            prev_seq,
            seq,
            updates,
        } => book.apply_delta_from(prev_seq, seq, &updates),
        BookEvent::Disconnected => {
            stats.disconnects += 1;
            book.mark_stale();
            Ok(())
        }
    };
    match result {
        Ok(()) => {}
        // Already stale and waiting for the snapshot we asked for.
        Err(CarryError::NotSynced) => {}
        Err(err) => {
            stats.resyncs += 1;
            warn!(%err, "book invalid; requesting resync");
            resync.notify_one();
        }
    }
}

#[cfg(test)]
mod tests {
    use carry_core::{Level, Qty, Tick};
    use rust_decimal::Decimal;
    use tokio::task::JoinHandle;
    use tokio::time::timeout;

    use super::*;

    const WAIT: Duration = Duration::from_secs(1);
    struct Harness {
        tx: mpsc::Sender<VenueEvent>,
        view: watch::Receiver<Arc<OrderBook>>,
        resync: Arc<Notify>,
        task: JoinHandle<()>,
    }

    fn start() -> Harness {
        let book = OrderBook::new(Decimal::ONE).unwrap();
        let (tx, rx) = mpsc::channel(16);
        let (publish, view) = watch::channel(Arc::new(book.clone()));
        let resync = Arc::new(Notify::new());
        let task = tokio::spawn(run_book_owner(
            Venue::Lighter,
            book,
            rx,
            publish,
            Arc::clone(&resync),
        ));
        Harness {
            tx,
            view,
            resync,
            task,
        }
    }

    fn level(tick: u64) -> Level {
        Level {
            tick: Tick::new(tick),
            qty: Qty::new(Decimal::ONE).unwrap(),
        }
    }

    fn event(event: BookEvent) -> VenueEvent {
        VenueEvent {
            venue: Venue::Lighter,
            event,
        }
    }

    fn snapshot(seq: u64) -> VenueEvent {
        event(BookEvent::Snapshot {
            seq,
            bids: vec![level(99)],
            asks: vec![level(101)],
        })
    }
    #[tokio::test]
    async fn snapshot_is_published() {
        let mut h = start();
        h.tx.send(snapshot(1)).await.unwrap();
        let book = timeout(WAIT, h.view.wait_for(|b| b.is_synced()))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(book.best_bid(), Some(level(99)));
    }

    #[tokio::test]
    async fn gap_marks_stale_and_requests_resync() {
        let h = start();
        h.tx.send(snapshot(1)).await.unwrap();
        let gap = BookEvent::Delta {
            prev_seq: 5,
            seq: 6,
            updates: vec![],
        };
        h.tx.send(event(gap)).await.unwrap();
        timeout(WAIT, h.resync.notified()).await.unwrap();
        let book = h.view.borrow().clone();
        assert!(!book.is_synced());
    }

    #[tokio::test]
    async fn disconnect_marks_stale_without_resync() {
        let mut h = start();
        h.tx.send(snapshot(1)).await.unwrap();
        h.tx.send(event(BookEvent::Disconnected)).await.unwrap();
        timeout(WAIT, h.view.wait_for(|b| !b.is_synced()))
            .await
            .unwrap()
            .unwrap();
        // The feed is already reconnecting; asking again would reconnect twice.
        assert!(
            timeout(Duration::from_millis(50), h.resync.notified())
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn owner_stops_when_feed_closes() {
        let h = start();
        drop(h.tx); // the feed task ending drops its sender
        timeout(WAIT, h.task).await.unwrap().unwrap();
    }
}
