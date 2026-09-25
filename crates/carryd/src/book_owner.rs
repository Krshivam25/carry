use std::sync::Arc;

use carry_core::{CarryError, OrderBook};
use carry_venues::{BookEvent, VenueEvent};
use tokio::sync::{Notify, mpsc, watch};

const BATCH: usize = 256;

pub async fn run_book_owner(
    mut book: OrderBook,
    mut rx: mpsc::Receiver<VenueEvent>,
    publish: watch::Sender<Arc<OrderBook>>,
    resync: Arc<Notify>,
) {
    let mut batch = Vec::with_capacity(BATCH);
    while rx.recv_many(&mut batch, BATCH).await > 0 {
        for msg in batch.drain(..) {
            apply(&mut book, msg, &resync);
        }
        publish.send_replace(Arc::new(book.clone()));
    }
}

fn apply(book: &mut OrderBook, msg: VenueEvent, resync: &Notify) {
    let result = match msg.event {
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
    };

    match result {
        Ok(()) => {}
        Err(CarryError::NotSynced) => {}
        Err(err) => {
            eprintln!("[{}] {err}; requesting resync", msg.venue);
            resync.notify_one();
        }
    }
}

#[cfg(test)]
mod tests {
    use carry_core::{Level, Qty, Tick, Venue};
    use rust_decimal::Decimal;
    use std::time::Duration;
    use tokio::time::timeout;

    use super::*;

    const WAIT: Duration = Duration::from_secs(1);

    struct Harness {
        tx: mpsc::Sender<VenueEvent>,
        view: watch::Receiver<Arc<OrderBook>>,
        resync: Arc<Notify>,
    }

    fn start() -> Harness {
        let book = OrderBook::new(Decimal::ONE).unwrap();
        let (tx, rx) = mpsc::channel(16);
        let (publish, view) = watch::channel(Arc::new(book.clone()));
        let resync = Arc::new(Notify::new());
        tokio::spawn(run_book_owner(book, rx, publish, Arc::clone(&resync)));
        Harness { tx, view, resync }
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
        assert!(
            timeout(Duration::from_millis(50), h.resync.notified())
                .await
                .is_err()
        );
    }
}
