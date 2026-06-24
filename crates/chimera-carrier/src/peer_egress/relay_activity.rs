use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[derive(Clone)]
pub(crate) struct RelayActivity {
    counter: Arc<AtomicU64>,
    finished_directions: Arc<AtomicU64>,
}

impl RelayActivity {
    pub(crate) fn new() -> Self {
        Self {
            counter: Arc::new(AtomicU64::new(0)),
            finished_directions: Arc::new(AtomicU64::new(0)),
        }
    }

    pub(crate) fn snapshot(&self) -> u64 {
        self.counter.load(Ordering::Relaxed)
    }

    pub(crate) fn record(&self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn unchanged_since(&self, snapshot: u64) -> bool {
        self.snapshot() == snapshot
    }

    pub(crate) fn record_finished_direction(&self) {
        self.finished_directions.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn has_finished_direction(&self) -> bool {
        self.finished_directions.load(Ordering::Relaxed) > 0
    }
}
