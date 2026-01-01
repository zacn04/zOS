use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Prometheus-style metrics for observability
/// All metrics are atomic counters for thread-safety
#[derive(Clone, Default)]
pub struct Metrics {
    /// Model call latency in milliseconds (sum)
    pub model_latency_ms: Arc<AtomicU64>,
    /// Routing decision time in milliseconds (sum)
    pub routing_time_ms: Arc<AtomicU64>,
    /// Cache hit count
    pub cache_hit_count: Arc<AtomicU64>,
    /// Cache miss count
    pub cache_miss_count: Arc<AtomicU64>,
    /// Fallback count (when primary model fails)
    pub fallback_count: Arc<AtomicU64>,
    /// Total errors by stage
    pub errors_total: Arc<AtomicU64>,
    /// Session state transitions
    pub session_state_transitions: Arc<AtomicU64>,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record model latency
    pub fn record_model_latency(&self, ms: u64) {
        self.model_latency_ms.fetch_add(ms, Ordering::Relaxed);
    }

    /// Record routing time
    pub fn record_routing_time(&self, ms: u64) {
        self.routing_time_ms.fetch_add(ms, Ordering::Relaxed);
    }

    /// Record cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hit_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record cache miss
    pub fn record_cache_miss(&self) {
        self.cache_miss_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record fallback
    pub fn record_fallback(&self) {
        self.fallback_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Record error
    pub fn record_error(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record session state transition
    pub fn record_state_transition(&self) {
        self.session_state_transitions.fetch_add(1, Ordering::Relaxed);
    }
}
