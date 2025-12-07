/// Performance timing utilities for measuring latency
use std::time::Instant;

/// Performance timer that logs on drop
pub struct PerfTimer {
    label: &'static str,
    start: Instant,
}

impl PerfTimer {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            start: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

impl Drop for PerfTimer {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().as_millis() as u64;
        eprintln!("[Perf] {} duration_ms={}", self.label, elapsed);
    }
}

/// Log a performance metric
pub fn log_perf(label: &str, duration_ms: u64) {
    eprintln!("[Perf] {} duration_ms={}", label, duration_ms);
}

/// Log a performance metric with additional context
pub fn log_perf_with_context(label: &str, duration_ms: u64, context: &str) {
    eprintln!("[Perf] {} duration_ms={} context={}", label, duration_ms, context);
}

