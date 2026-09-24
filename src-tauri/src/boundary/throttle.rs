use std::time::{Duration, Instant};

use crate::boundary::events::ScanProgressPayload;

pub trait ProgressSink {
    fn emit_progress(&mut self, payload: ScanProgressPayload);
}

pub struct ProgressThrottler<S: ProgressSink> {
    min_interval: Duration,
    last_emitted: Option<Instant>,
    coalesced: Option<ScanProgressPayload>,
    sink: S,
}

impl<S: ProgressSink> ProgressThrottler<S> {
    pub fn new(min_interval: Duration, sink: S) -> Self {
        Self {
            min_interval,
            last_emitted: None,
            coalesced: None,
            sink,
        }
    }

    /// Record a progress update at the given timestamp.
    /// If the time budget has elapsed since the last emission, emit immediately.
    /// Otherwise, coalesce the payload in-memory.
    pub fn record(&mut self, payload: ScanProgressPayload, now: Instant) {
        let should_emit = match self.last_emitted {
            None => true,
            Some(last) => now.saturating_duration_since(last) >= self.min_interval,
        };

        if should_emit {
            self.last_emitted = Some(now);
            self.coalesced = None;
            self.sink.emit_progress(payload);
        } else {
            self.coalesced = Some(payload);
        }
    }

    /// Poll if pending coalesced progress is due for emission based on the current timestamp.
    pub fn poll_emit_due(&mut self, now: Instant) {
        if let Some(last) = self.last_emitted
            && now.saturating_duration_since(last) >= self.min_interval
            && let Some(payload) = self.coalesced.take()
        {
            self.last_emitted = Some(now);
            self.sink.emit_progress(payload);
        }
    }

    /// Flush any pending coalesced progress update.
    pub fn flush(&mut self) {
        if let Some(payload) = self.coalesced.take() {
            self.sink.emit_progress(payload);
        }
    }

    pub fn sink(&self) -> &S {
        &self.sink
    }

    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }
}

#[cfg(test)]
pub mod fixtures {
    use super::*;
    use crate::boundary::events::ScanPhase;

    #[derive(Default, Debug)]
    pub struct MockProgressSink {
        pub emissions: Vec<ScanProgressPayload>,
    }

    impl ProgressSink for MockProgressSink {
        fn emit_progress(&mut self, payload: ScanProgressPayload) {
            self.emissions.push(payload);
        }
    }

    /// Synthetic fixture that generates rapid item progress updates.
    /// This proves emitter-side throttling without depending on real filesystem traversal.
    pub fn synthetic_fast_progress_producer(
        item_count: u64,
        throttler: &mut ProgressThrottler<MockProgressSink>,
        simulated_step: Duration,
    ) {
        let start = Instant::now();
        for i in 1..=item_count {
            let current_time = start + simulated_step.saturating_mul(i as u32);
            let payload = ScanProgressPayload {
                session_id: "synthetic-session-1".into(),
                phase: ScanPhase::Scanning,
                current_scope: format!("/synthetic/dir/item_{i}"),
                items_visited: i,
                bytes_visited: i * 512,
            };
            throttler.record(payload, current_time);
        }
        throttler.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{MockProgressSink, synthetic_fast_progress_producer};
    use super::*;

    #[test]
    fn test_progress_throttler_coalesces_emissions_under_load() {
        let throttle_interval = Duration::from_millis(50);
        let sink = MockProgressSink::default();
        let mut throttler = ProgressThrottler::new(throttle_interval, sink);

        // Producer generates 10,000 items in rapid bursts (1 microsecond per item).
        // Total elapsed virtual time = 10ms (< 50ms interval).
        let total_items = 10_000u64;
        let step = Duration::from_micros(1);
        synthetic_fast_progress_producer(total_items, &mut throttler, step);

        let emissions = &throttler.sink().emissions;
        // Without throttling, 10,000 events would have flooded the listener.
        // With throttling, only the first and flushed final events are emitted.
        assert!(
            emissions.len() <= 3,
            "Expected <= 3 emissions for rapid burst of 10,000 items, got {}",
            emissions.len()
        );
        assert!(
            emissions.len() < total_items as usize,
            "Emissions ({}) must be far lower than total items ({})",
            emissions.len(),
            total_items
        );

        // Verify that the final emission reflects all 10,000 items visited.
        let final_emission = emissions.last().expect("must have final emission");
        assert_eq!(final_emission.items_visited, total_items);
        assert_eq!(final_emission.bytes_visited, total_items * 512);
    }
}
