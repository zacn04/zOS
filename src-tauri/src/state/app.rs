use std::sync::Arc;
use parking_lot::RwLock;
use crate::skills::model::SkillVector;
use crate::state::session::ProofState;
use crate::pipelines::router::RoutingMetrics;
use crate::cache::CachedResponse;
use lru::LruCache;
use std::num::NonZeroUsize;

/// Application-wide state container.
/// All mutable state is centralized here and passed explicitly to functions.
/// This eliminates global mutable state and lock-ordering hazards.
#[derive(Clone)]
pub struct AppState {
    /// In-memory skill vector cache
    pub skills: Arc<RwLock<Option<SkillVector>>>,
    /// Current proof-solving session state
    pub session_state: Arc<RwLock<ProofState>>,
    /// Routing performance metrics
    pub routing_metrics: Arc<RwLock<RoutingMetrics>>,
    /// Response cache (LRU with bounded size)
    pub response_cache: Arc<RwLock<LruCache<u64, CachedResponse>>>,
}

impl AppState {
    /// Create a new AppState with default values
    pub fn new() -> Self {
        AppState {
            skills: Arc::new(RwLock::new(None)),
            session_state: Arc::new(RwLock::new(ProofState::AwaitingSolution)),
            routing_metrics: Arc::new(RwLock::new(RoutingMetrics::default())),
            response_cache: Arc::new(RwLock::new(
                LruCache::new(NonZeroUsize::new(200).expect("200 > 0"))
            )),
        }
    }

    /// Get skills, loading from disk if not cached
    pub fn get_skills(&self) -> Result<SkillVector, crate::error::ZosError> {
        let mut guard = self.skills.write();
        if guard.is_none() {
            // Load from disk (async will be handled by caller)
            *guard = Some(crate::skills::store::load_skill_vector());
        }
        guard.as_ref()
            .ok_or_else(|| crate::error::ZosError::new(
                "Failed to load skills",
                "state"
            ))
            .map(|s| s.clone())
    }

    /// Update skills with a closure
    pub fn update_skills<F>(&self, f: F) -> Result<(), crate::error::ZosError>
    where
        F: FnOnce(&mut SkillVector),
    {
        let mut guard = self.skills.write();
        if guard.is_none() {
            *guard = Some(crate::skills::store::load_skill_vector());
        }
        let skills = guard.as_mut()
            .ok_or_else(|| crate::error::ZosError::new(
                "Failed to access skills",
                "state"
            ))?;
        f(skills);
        Ok(())
    }

    /// Get current session state
    pub fn get_session_state(&self) -> ProofState {
        self.session_state.read().clone()
    }

    /// Set session state
    pub fn set_session_state(&self, state: ProofState) {
        *self.session_state.write() = state;
    }

    /// Reset session state
    pub fn reset_session_state(&self) {
        *self.session_state.write() = ProofState::AwaitingSolution;
    }

    /// Get routing metrics
    pub fn get_routing_metrics(&self) -> RoutingMetrics {
        self.routing_metrics.read().clone()
    }

    /// Record a successful routing
    pub fn record_routing_success(&self, latency_ms: u64) {
        let mut metrics = self.routing_metrics.write();
        metrics.success_count += 1;
        metrics.total_latency_ms += latency_ms;
    }

    /// Record a routing failure
    pub fn record_routing_failure(&self) {
        let mut metrics = self.routing_metrics.write();
        metrics.failure_count += 1;
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
