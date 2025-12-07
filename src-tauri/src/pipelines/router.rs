use crate::config::models::get_model_config;
use crate::models::registry::{get_model, get_available_models};
use crate::models::base::LocalModel;
use crate::models::availability::ensure_model_loaded;
use crate::error::ZosError;
use crate::cache::{get_cached, cache_response};
use crate::state::app::AppState;
use crate::circuit_breaker::{CircuitBreaker, ExponentialBackoff};
use chrono::Utc;
use tokio::time::Instant;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub enum TaskType {
    ProofAnalysis,
    ProblemGeneration,
    General,
}

#[derive(Debug, Clone)]
pub struct RouteDecision {
    pub selected: String,
    pub fallback: Option<String>,
    pub task: TaskType,
    pub timestamp: i64,
}

#[derive(Debug, Default, Clone)]
pub struct RoutingMetrics {
    pub success_count: u64,
    pub failure_count: u64,
    pub total_latency_ms: u64,
}

/// Find an alternative model if the primary is unavailable
fn find_fallback_model(task: TaskType, primary: &str) -> Option<String> {
    let config = get_model_config();
    let available = get_available_models();
    
    // Priority list for fallback
    let fallback_candidates = match task {
        TaskType::ProofAnalysis => {
            vec![
                config.proof_model.clone(),
                config.general_model.clone(),
                config.problem_model.clone(),
            ]
        }
        TaskType::ProblemGeneration => {
            vec![
                config.problem_model.clone(),
                config.general_model.clone(),
                config.proof_model.clone(),
            ]
        }
        TaskType::General => {
            vec![
                config.general_model.clone(),
                config.proof_model.clone(),
                config.problem_model.clone(),
            ]
        }
    };
    
    // Find first available model that's not the primary (registry-based check)
    for candidate in fallback_candidates {
        if candidate != primary && available.contains(&candidate) {
            return Some(candidate);
        }
    }
    
    // Last resort: any available model from registry
    for model in available {
        if model != primary {
            return Some(model);
        }
    }
    
    None
}

/// Route a task to the appropriate model with fallback support
/// Optimized O(1) routing - no I/O, uses cached config
pub fn model_for_task(task: TaskType) -> RouteDecision {
    let config = get_model_config();
    // Use references to avoid cloning until necessary
    let primary = match task {
        TaskType::ProofAnalysis => &config.proof_model,
        TaskType::ProblemGeneration => &config.problem_model,
        TaskType::General => &config.general_model,
    };
    
    // Pre-compute fallback (actual availability checked async)
    let fallback = find_fallback_model(task, primary);
    
    RouteDecision {
        selected: primary.clone(), // Only clone when creating decision
        fallback,
        task,
        timestamp: Utc::now().timestamp(),
    }
}

/// Get the model instance for a task (with fallback)
pub fn get_model_for_task(task: TaskType) -> Option<LocalModel> {
    let decision = model_for_task(task);
    get_model(&decision.selected)
}

/// Unified query function with retry, fallback, caching, and timeouts
pub async fn zos_query<T: serde::de::DeserializeOwned + serde::Serialize>(
    state: &AppState,
    task: TaskType,
    prompt: String,
) -> Result<T, ZosError> {
    use crate::pipelines::perf;
    let _perf = perf::PerfTimer::new("zos_query_total");
    let query_start = Instant::now();
    let routing_start = Instant::now();
    let decision = model_for_task(task);
    let routing_ms = routing_start.elapsed().as_millis() as u64;
    perf::log_perf("routing", routing_ms);
    
    let primary_model = decision.selected.clone();
    
    tracing::debug!(
        task = ?task,
        model = %primary_model,
        "Routing decision"
    );
    
    // Check cache first
    if let Some(cached) = get_cached::<T>(state, &primary_model, &prompt) {
        let latency_ms = query_start.elapsed().as_millis() as u64;
        tracing::info!(
            task = ?task,
            model = %primary_model,
            latency_ms = latency_ms,
            "Cache hit"
        );
        return Ok(cached);
    }
    
    // Ensure model is available
    if let Err(e) = ensure_model_loaded(&primary_model).await {
        // Try fallback
        if let Some(fallback_model) = decision.fallback.clone() {
            tracing::warn!(
                primary = %primary_model,
                fallback = %fallback_model,
                "Primary model unavailable, trying fallback"
            );
            if ensure_model_loaded(&fallback_model).await.is_ok() {
                return try_model_with_retry::<T>(state, &fallback_model, &prompt, task, query_start).await;
            }
        }
        return Err(e);
    }
    
    // Try primary model with retry
    match try_model_with_retry::<T>(state, &primary_model, &prompt, task, query_start).await {
        Ok(result) => {
            // Cache the result
            cache_response(state, &primary_model, &prompt, &result)
                .map_err(|e| ZosError::new(
                    format!("Failed to cache response: {}", e),
                    "cache"
                ))?;
            Ok(result)
        }
        Err(_e) => {
            // Try fallback if available
            if let Some(fallback_model) = decision.fallback.clone() {
                tracing::warn!(
                    primary = %primary_model,
                    fallback = %fallback_model,
                    "Primary model failed, trying fallback"
                );
                if ensure_model_loaded(&fallback_model).await.is_ok() {
                    match try_model_with_retry::<T>(state, &fallback_model, &prompt, task, query_start).await {
                        Ok(result) => {
                            cache_response(state, &fallback_model, &prompt, &result)
                                .map_err(|e| ZosError::new(
                                    format!("Failed to cache response: {}", e),
                                    "cache"
                                ))?;
                            Ok(result)
                        }
                        Err(fallback_err) => Err(fallback_err.with_retry(false)),
                    }
                } else {
                    Err(_e.with_retry(false))
                }
            } else {
                Err(_e.with_retry(false))
            }
        }
    }
}

/// Try a model with exponential backoff retry
async fn try_model_with_retry<T: serde::de::DeserializeOwned>(
    state: &AppState,
    model_name: &str,
    prompt: &str,
    _task: TaskType,
    _query_start: Instant,
) -> Result<T, ZosError> {
    let model = get_model(model_name)
        .ok_or_else(|| ZosError::new(
            format!("Model '{}' not found in registry", model_name),
            "routing"
        ).with_model(model_name.to_string()))?;
    
    // Check circuit breaker (would be stored per-model in production)
    // For now, we'll use a simple retry with exponential backoff
    
    let backoff = ExponentialBackoff::default();
    let max_retries = 2;
    
    for attempt in 0..=max_retries {
        let attempt_start = Instant::now();
        match model.call_json::<T>(prompt).await {
            Ok(result) => {
                let latency_ms = attempt_start.elapsed().as_millis() as u64;
                if attempt > 0 {
                    tracing::info!(
                        model = model_name,
                        latency_ms = latency_ms,
                        attempt = attempt,
                        "Model call succeeded after retry"
                    );
                } else {
                    tracing::info!(
                        model = model_name,
                        latency_ms = latency_ms,
                        "Model call succeeded"
                    );
                }
                state.record_routing_success(latency_ms);
                return Ok(result);
            }
            Err(e) => {
                if attempt < max_retries {
                    let delay_ms = backoff.delay_for_attempt(attempt);
                    tracing::warn!(
                        model = model_name,
                        error = %e,
                        attempt = attempt + 1,
                        max_retries = max_retries + 1,
                        delay_ms = delay_ms,
                        "Model call failed, retrying with backoff"
                    );
                    state.record_routing_failure();
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                } else {
                    tracing::error!(
                        model = model_name,
                        error = %e,
                        attempts = max_retries + 1,
                        "Model call failed after all retries"
                    );
                    state.record_routing_failure();
                    return Err(ZosError::new(
                        format!("Model '{}' failed after {} attempts: {}", model_name, max_retries + 1, e),
                        "model_call"
                    ).with_model(model_name.to_string()).with_retry(true));
                }
            }
        }
    }
    
    // Should never reach here, but compiler needs it
    unreachable!()
}
