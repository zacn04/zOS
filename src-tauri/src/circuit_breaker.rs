use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

/// Circuit breaker for model availability tracking
/// Implements a simple three-state circuit breaker pattern
#[derive(Clone)]
pub struct CircuitBreaker {
    /// Number of consecutive failures
    failures: Arc<AtomicU64>,
    /// Last failure time
    last_failure: Arc<RwLock<Option<Instant>>>,
    /// Circuit state (true = open/failed, false = closed/healthy)
    is_open: Arc<AtomicBool>,
    /// Timeout before allowing retry (in seconds)
    timeout_secs: u64,
    /// Failure threshold before opening circuit
    failure_threshold: u64,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(timeout_secs: u64, failure_threshold: u64) -> Self {
        CircuitBreaker {
            failures: Arc::new(AtomicU64::new(0)),
            last_failure: Arc::new(RwLock::new(None)),
            is_open: Arc::new(AtomicBool::new(false)),
            timeout_secs,
            failure_threshold,
        }
    }

    /// Check if circuit is open (should not attempt call)
    pub fn is_open(&self) -> bool {
        if !self.is_open.load(Ordering::Relaxed) {
            return false;
        }

        // Check if timeout has passed
        let last_failure = self.last_failure.read();
        if let Some(time) = *last_failure {
            if time.elapsed().as_secs() >= self.timeout_secs {
                // Timeout passed, allow retry (half-open state)
                self.is_open.store(false, Ordering::Relaxed);
                self.failures.store(0, Ordering::Relaxed);
                return false;
            }
        }
        true
    }

    /// Record a successful call
    pub fn record_success(&self) {
        self.failures.store(0, Ordering::Relaxed);
        self.is_open.store(false, Ordering::Relaxed);
        *self.last_failure.write() = None;
    }

    /// Record a failed call
    pub fn record_failure(&self) {
        let failures = self.failures.fetch_add(1, Ordering::Relaxed) + 1;
        *self.last_failure.write() = Some(Instant::now());

        if failures >= self.failure_threshold {
            self.is_open.store(true, Ordering::Relaxed);
        }
    }

    /// Get current failure count
    pub fn failure_count(&self) -> u64 {
        self.failures.load(Ordering::Relaxed)
    }
}

/// Exponential backoff calculator
pub struct ExponentialBackoff {
    initial_delay_ms: u64,
    max_delay_ms: u64,
    multiplier: f64,
}

impl ExponentialBackoff {
    pub fn new(initial_delay_ms: u64, max_delay_ms: u64) -> Self {
        ExponentialBackoff {
            initial_delay_ms,
            max_delay_ms,
            multiplier: 2.0,
        }
    }

    /// Calculate delay for attempt number (0-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        let delay = (self.initial_delay_ms as f64 * self.multiplier.powi(attempt as i32)) as u64;
        delay.min(self.max_delay_ms)
    }
}

impl Default for ExponentialBackoff {
    fn default() -> Self {
        Self::new(100, 5000) // 100ms initial, 5s max
    }
}
