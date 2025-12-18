use std::sync::Arc;
use parking_lot::RwLock;
use crate::skills::model::SkillVector;
use crate::state::session::ProofState;
use crate::pipelines::router::RoutingMetrics;
use crate::cache::CachedResponse;
use crate::problems::problem::Problem;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::collections::VecDeque;

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
    /// Recently selected problem IDs (to avoid immediate repeats)
    pub recently_selected_problems: Arc<RwLock<VecDeque<String>>>,
    /// Precomputed next problems (for instant loading) - stores easier, same, harder
    pub precomputed_problems: Arc<RwLock<Vec<Problem>>>,
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
            recently_selected_problems: Arc::new(RwLock::new(VecDeque::with_capacity(5))),
            precomputed_problems: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get skills, loading from disk if not cached (synchronous - returns cached value or error)
    /// For loading from disk, use memory::store::get_skills() instead
    pub fn get_skills(&self) -> Result<SkillVector, crate::error::ZosError> {
        let guard = self.skills.read();
        guard.as_ref()
            .ok_or_else(|| crate::error::ZosError::new(
                "Skills not loaded - use memory::store::get_skills() to load from disk",
                "state"
            ))
            .map(|s| s.clone())
    }

    /// Update skills with a closure (requires skills to already be loaded)
    /// For loading from disk first, use memory::store::update_skills() instead
    pub fn update_skills<F>(&self, f: F) -> Result<(), crate::error::ZosError>
    where
        F: FnOnce(&mut SkillVector),
    {
        let mut guard = self.skills.write();
        let skills = guard.as_mut()
            .ok_or_else(|| crate::error::ZosError::new(
                "Skills not loaded - use memory::store::update_skills() to load from disk first",
                "state"
            ))?;
        f(skills);
        Ok(())
    }
    
    /// Set skills directly (for initialization from async load)
    pub fn set_skills(&self, skills: SkillVector) {
        *self.skills.write() = Some(skills);
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

    /// Record that a problem was just selected (to avoid immediate repeats)
    pub fn record_problem_selected(&self, problem_id: String) {
        let mut recent = self.recently_selected_problems.write();
        // Remove if already present (to avoid duplicates)
        recent.retain(|id| id != &problem_id);
        // Add to front
        recent.push_front(problem_id);
        // Keep only last 5
        if recent.len() > 5 {
            recent.pop_back();
        }
    }

    /// Get recently selected problem IDs
    pub fn get_recently_selected_problems(&self) -> Vec<String> {
        self.recently_selected_problems.read().iter().cloned().collect()
    }

    /// Get and remove a precomputed problem (prefers same difficulty, then easier, then harder)
    pub fn take_precomputed_problem(&self, target_difficulty: Option<f32>) -> Option<Problem> {
        let mut problems = self.precomputed_problems.write();
        if problems.is_empty() {
            return None;
        }
        
        // If we have a target difficulty, try to find the closest match
        if let Some(target) = target_difficulty {
            // Sort by distance from target difficulty
            problems.sort_by(|a, b| {
                let diff_a = (a.difficulty - target).abs();
                let diff_b = (b.difficulty - target).abs();
                diff_a.partial_cmp(&diff_b).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        
        problems.pop()
    }

    /// Add a precomputed problem (keeps max 3: easier, same, harder)
    pub fn add_precomputed_problem(&self, problem: Problem) {
        let mut problems = self.precomputed_problems.write();
        problems.push(problem);
        // Keep only the 3 most recent
        if problems.len() > 3 {
            problems.remove(0);
        }
    }

    /// Clear all precomputed problems
    pub fn clear_precomputed_problems(&self) {
        self.precomputed_problems.write().clear();
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
