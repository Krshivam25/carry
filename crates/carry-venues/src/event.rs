use carry_core::{Level, LevelUpdate, Venue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookEvent {
    Snapshot {
        seq: u64,
        bids: Vec<Level>,
        asks: Vec<Level>,
    },
    Delta {
        prev_seq: u64,
        seq: u64,
        updates: Vec<LevelUpdate>,
    },
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueEvent {
    pub venue: Venue,
    pub event: BookEvent,
}
