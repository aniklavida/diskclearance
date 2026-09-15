use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Default)]
pub struct CancellationRegistry {
    tokens: Mutex<HashMap<String, CancellationToken>>,
}

impl CancellationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, session_id: &str) -> CancellationToken {
        let mut tokens = self.tokens.lock().expect("mutex poisoned");
        let token = CancellationToken::new();
        tokens.insert(session_id.to_string(), token.clone());
        token
    }

    pub fn cancel(&self, session_id: &str) -> bool {
        let tokens = self.tokens.lock().expect("mutex poisoned");
        if let Some(token) = tokens.get(session_id) {
            token.cancel();
            true
        } else {
            false
        }
    }

    pub fn unregister(&self, session_id: &str) {
        let mut tokens = self.tokens.lock().expect("mutex poisoned");
        tokens.remove(session_id);
    }
}

#[cfg(test)]
pub mod fixtures {
    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::thread;
    use std::time::Duration;

    /// Synthetic fixture representing a long-running work loop.
    /// It checks the cancellation token on every iteration and halts as soon
    /// as cancellation is signaled, proving cancellation stops the actual work.
    pub fn synthetic_long_running_worker(
        token: CancellationToken,
        target_iterations: u64,
        work_counter: Arc<AtomicU64>,
    ) {
        for _ in 0..target_iterations {
            if token.is_cancelled() {
                break;
            }
            work_counter.fetch_add(1, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::synthetic_long_running_worker;
    use super::*;
    use std::sync::atomic::AtomicU64;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_cancellation_stops_work_on_synthetic_producer() {
        let registry = CancellationRegistry::new();
        let session_id = "test-session-cancel";
        let token = registry.register(session_id);

        let target_iterations = 1_000_000u64;
        let work_counter = Arc::new(AtomicU64::new(0));

        let worker_token = token.clone();
        let worker_counter = Arc::clone(&work_counter);
        let handle = thread::spawn(move || {
            synthetic_long_running_worker(worker_token, target_iterations, worker_counter);
        });

        // Allow synthetic worker to start
        while work_counter.load(Ordering::SeqCst) < 5 {
            thread::sleep(Duration::from_millis(1));
        }

        // Cancel via registry
        let was_registered = registry.cancel(session_id);
        assert!(
            was_registered,
            "Cancellation token must be registered and found"
        );

        handle.join().expect("worker thread join");

        let performed = work_counter.load(Ordering::SeqCst);
        assert!(
            performed < target_iterations,
            "Work must stop well before target iterations ({target_iterations}), but performed {performed}"
        );
        assert!(token.is_cancelled(), "Token must be marked cancelled");
    }
}
