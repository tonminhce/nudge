#[cfg(test)]
use std::sync::atomic::AtomicU64;
#[cfg(test)]
use std::sync::atomic::Ordering;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

pub trait EventStreamClock: Send + Sync {
    fn now_ms(&self) -> u64;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl EventStreamClock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
#[derive(Debug, Default)]
pub struct ManualClock {
    now: AtomicU64,
}

#[cfg(test)]
impl ManualClock {
    pub fn new(now: u64) -> Self {
        Self {
            now: AtomicU64::new(now),
        }
    }

    pub fn advance(&self, elapsed_ms: u64) {
        self.now.fetch_add(elapsed_ms, Ordering::SeqCst);
    }
}

#[cfg(test)]
impl EventStreamClock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.now.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_injected_clock_advances_without_a_system_dependency() {
        let clock = ManualClock::new(10);
        assert_eq!(clock.now_ms(), 10);
        clock.advance(5);
        assert_eq!(clock.now_ms(), 15);
    }

    #[test]
    fn the_system_clock_is_monotonic_enough_for_ttl_bookkeeping() {
        let clock = SystemClock;
        let first = clock.now_ms();
        assert!(first > 0);
        assert!(clock.now_ms() >= first);
    }
}
