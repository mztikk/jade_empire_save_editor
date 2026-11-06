//! Frame-time instrumentation.
//!
//! The editor measures the time between renders that change state and records
//! it here, so a slow layout can be spotted without a profiler. The values are
//! process wide and informational: nothing in the UI depends on them.

use std::sync::atomic::{AtomicU64, Ordering};

/// Budget of one frame at 60 Hz in microseconds.
pub const FRAME_BUDGET_MICROS: u64 = 16_667;

static LAST_FRAME_MICROS: AtomicU64 = AtomicU64::new(0);
static OVER_BUDGET_FRAMES: AtomicU64 = AtomicU64::new(0);
static FRAMES: AtomicU64 = AtomicU64::new(0);

/// Records the duration of one render.
pub fn record_frame(duration: std::time::Duration) {
    let micros = u64::try_from(duration.as_micros()).unwrap_or(u64::MAX);
    LAST_FRAME_MICROS.store(micros, Ordering::Relaxed);
    FRAMES.fetch_add(1, Ordering::Relaxed);
    if micros > FRAME_BUDGET_MICROS {
        OVER_BUDGET_FRAMES.fetch_add(1, Ordering::Relaxed);
    }
}

/// Duration of the last recorded render in microseconds.
#[must_use]
pub fn last_frame_micros() -> u64 {
    LAST_FRAME_MICROS.load(Ordering::Relaxed)
}

/// Number of renders recorded so far.
#[must_use]
pub fn frames() -> u64 {
    FRAMES.load(Ordering::Relaxed)
}

/// Number of recorded renders that exceeded the 60 Hz budget.
#[must_use]
pub fn over_budget_frames() -> u64 {
    OVER_BUDGET_FRAMES.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_over_the_budget_are_counted() {
        let before = over_budget_frames();
        record_frame(std::time::Duration::from_micros(1_000));
        assert_eq!(last_frame_micros(), 1_000);
        assert_eq!(over_budget_frames(), before);
        record_frame(std::time::Duration::from_micros(FRAME_BUDGET_MICROS + 1));
        assert_eq!(over_budget_frames(), before + 1);
    }
}
