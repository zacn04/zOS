use crate::config::models::get_model_config;
use crate::models::registry::{get_model, get_available_models};
use crate::models::base::LocalModel;
use crate::models::availability::ensure_model_loaded;
use crate::error::ZosError;
use crate::cache::{get_cached, cache_response};
use crate::state::app::AppState;
use chrono::Utc;
use tokio::time::Instant;

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
/// DeepSeek is NOT used for JSON tasks (ProblemGeneration, JSON-structured responses)
pub fn model_for_task(task: TaskType) -> RouteDecision {
    let config = get_model_config();
    
    // For JSON tasks, prefer non-DeepSeek models
    let primary = match task {
        TaskType::ProofAnalysis => {
            // ProofAnalysis may return JSON (Step1/Step2), so prefer non-DeepSeek
            // But DeepSeek is good for free-form analysis, so we keep it as fallback
            if config.proof_model.contains("deepseek") {
                // Prefer general model for JSON-structured responses
                &config.general_model
            } else {
                &config.proof_model
            }
        }
        TaskType::ProblemGeneration => {
            // ProblemGeneration always needs JSON - never use DeepSeek
            if config.problem_model.contains("deepseek") {
                &config.general_model
            } else {
                &config.problem_model
            }
        }
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
                match try_model_with_retry::<T>(state, &fallback_model, &prompt, task, query_start).await {
                    Ok(result) => {
                        cache_response(state, &fallback_model, &prompt, &result)
                            .map_err(|e| ZosError::new(
                                format!("Failed to cache response: {}", e),
                                "cache"
                            ))?;
                        return Ok(result);
                    }
                    Err((err, _)) => return Err(err.with_retry(false)),
                }
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
        Err((e, raw_response)) => {
            // If we have a raw response and JSON extraction failed, try repair with fallback
            // BUT skip repair if truncated, timeout, or too large (regenerate instead)
            if let (Some(raw), Some(fallback_model)) = (raw_response, decision.fallback.clone()) {
                // Skip repair for truncation, timeout, or size issues
                let should_repair = matches!(e.stage.as_str(), "json_extract" | "json_parse")
                    && !matches!(e.stage.as_str(), "truncated" | "timeout_truncation" | "output_too_large");
                
                if should_repair {
                    tracing::warn!(
                        primary = %primary_model,
                        fallback = %fallback_model,
                        raw_response_length = raw.len(),
                        "Primary model JSON extraction failed, attempting repair with fallback"
                    );
                    if ensure_model_loaded(&fallback_model).await.is_ok() {
                        match repair_json_with_fallback::<T>(state, &fallback_model, &raw, &prompt).await {
                            Ok(result) => {
                                cache_response(state, &fallback_model, &prompt, &result)
                                    .map_err(|e| ZosError::new(
                                        format!("Failed to cache response: {}", e),
                                        "cache"
                                    ))?;
                                tracing::info!(
                                    primary = %primary_model,
                                    fallback = %fallback_model,
                                    "Successfully repaired JSON with fallback model"
                                );
                                return Ok(result);
                            }
                            Err(repair_err) => {
                                // Check if repair detected truncation
                                if repair_err.stage == "truncated_detected" {
                                    tracing::warn!(
                                        primary = %primary_model,
                                        fallback = %fallback_model,
                                        "Repair detected truncation, regenerating with fallback"
                                    );
                                    // Fall through to regenerate
                                } else {
                                    tracing::warn!(
                                        primary = %primary_model,
                                        fallback = %fallback_model,
                                        error = %repair_err,
                                        "JSON repair failed, trying full retry with fallback"
                                    );
                                    // Fall through to try fallback with original prompt
                                }
                            }
                        }
                    }
                } else {
                    tracing::warn!(
                        primary = %primary_model,
                        fallback = %fallback_model,
                        error_stage = %e.stage,
                        "Skipping repair (truncation/size/timeout), regenerating with fallback"
                    );
                }
            }
            
            // Standard fallback: try fallback model with original prompt
            if let Some(fallback_model) = decision.fallback.clone() {
                tracing::warn!(
                    primary = %primary_model,
                    fallback = %fallback_model,
                    "Primary model failed, trying fallback with original prompt"
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
                        Err((fallback_err, _)) => Err(fallback_err.with_retry(false)),
                    }
                } else {
                    Err(e.with_retry(false))
                }
            } else {
                Err(e.with_retry(false))
            }
        }
    }
}

/// Try a model with exponential backoff retry
/// Returns Ok(result) on success, or Err with raw_response context for JSON extraction failures
async fn try_model_with_retry<T: serde::de::DeserializeOwned>(
    state: &AppState,
    model_name: &str,
    prompt: &str,
    _task: TaskType,
    _query_start: Instant,
) -> Result<T, (ZosError, Option<String>)> {
    use crate::pipelines::ollama;
    use crate::pipelines::ollama_utils;
    
    // Verify model exists in registry
    let _model = get_model(model_name)
        .ok_or_else(|| (ZosError::new(
            format!("Model '{}' not found in registry", model_name),
            "routing"
        ).with_model(model_name.to_string()), None))?;

    let max_retries = 2;

    for attempt in 0..=max_retries {
        let attempt_start = Instant::now();

        // Get raw response first
        let raw_response = match ollama::call_ollama_model(model_name, prompt).await {
            Ok(resp) => resp,
            Err(e) => {
                if attempt < max_retries {
                    // Simple exponential backoff: 100ms * 2^attempt, max 5s
                    let delay_ms = (100 * 2_u64.pow(attempt)).min(5000);
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
                    continue;
                } else {
                    return Err((ZosError::new(
                        format!("Model '{}' failed to respond after {} attempts: {}", model_name, max_retries + 1, e),
                        "model_call"
                    ).with_model(model_name.to_string()).with_retry(true), None));
                }
            }
        };
        
        let latency_ms = attempt_start.elapsed().as_millis() as u64;
        
        // Max-latency watchdog: if > 60s, treat as truncation
        // Allows time for detailed proofs that may take longer to parse
        if latency_ms > 60000 {
            tracing::warn!(
                model = model_name,
                latency_ms = latency_ms,
                "Latency exceeded 60s, treating as truncation"
            );
            return Err((ZosError::new(
                format!("Model '{}' response took {}ms (truncation suspected)", model_name, latency_ms),
                "timeout_truncation"
            ).with_model(model_name.to_string()).with_retry(true), Some(raw_response)));
        }
        
        // Max-output-size check: if > 40k bytes, treat as invalid
        if raw_response.len() > 40_000 {
            tracing::warn!(
                model = model_name,
                output_size = raw_response.len(),
                "Output size exceeded 40k bytes, treating as invalid"
            );
            return Err((ZosError::new(
                format!("Model '{}' output too large ({} bytes)", model_name, raw_response.len()),
                "output_too_large"
            ).with_model(model_name.to_string()).with_retry(true), Some(raw_response)));
        }
        
        // Sanitize raw output before extraction
        let sanitized = ollama_utils::sanitize_raw_output(&raw_response);
        
        // Truncation check: if truncated, skip repair and regenerate
        if ollama_utils::is_truncated(&sanitized) {
            tracing::warn!(
                model = model_name,
                "Output appears truncated, skipping repair"
            );
            return Err((ZosError::new(
                format!("Model '{}' output appears truncated", model_name),
                "truncated"
            ).with_model(model_name.to_string()).with_retry(true), Some(raw_response)));
        }
        
        // Try to extract and parse JSON from sanitized output
        match ollama_utils::extract_json(&sanitized) {
            Ok(json_str) => {
                match serde_json::from_str::<T>(&json_str) {
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
                    Err(parse_err) => {
                        let error_msg = format!("Model '{}' returned invalid JSON: {}", model_name, parse_err);
                        if attempt < max_retries {
                            // Simple exponential backoff: 100ms * 2^attempt, max 5s
                            let delay_ms = (100 * 2_u64.pow(attempt)).min(5000);
                            tracing::warn!(
                                model = model_name,
                                error = %parse_err,
                                attempt = attempt + 1,
                                max_retries = max_retries + 1,
                                delay_ms = delay_ms,
                                "JSON parsing failed, retrying"
                            );
                            state.record_routing_failure();
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                            continue;
                        } else {
                            // Return error with raw response for repair
                            return Err((ZosError::new(error_msg, "json_parse").with_model(model_name.to_string()).with_retry(true), Some(raw_response)));
                        }
                    }
                }
            }
            Err(extract_err) => {
                let error_msg = format!("Model '{}' failed to extract JSON: {}", model_name, extract_err);
                if attempt < max_retries {
                    // Simple exponential backoff: 100ms * 2^attempt, max 5s
                    let delay_ms = (100 * 2_u64.pow(attempt)).min(5000);
                    tracing::warn!(
                        model = model_name,
                        error = %extract_err,
                        attempt = attempt + 1,
                        max_retries = max_retries + 1,
                        delay_ms = delay_ms,
                        "JSON extraction failed, retrying"
                    );
                    state.record_routing_failure();
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                    continue;
                } else {
                    // Return error with raw response for repair
                    return Err((ZosError::new(error_msg, "json_extract").with_model(model_name.to_string()).with_retry(true), Some(raw_response)));
                }
            }
        }
    }
    
    // Should never reach here, but compiler needs it
    unreachable!()
}

/// Attempt to repair/extract JSON from a raw model response using a fallback model
async fn repair_json_with_fallback<T: serde::de::DeserializeOwned>(
    _state: &AppState,
    fallback_model_name: &str,
    raw_response: &str,
    _original_prompt: &str,
) -> Result<T, ZosError> {
    use crate::pipelines::ollama;
    use crate::pipelines::ollama_utils;
    
    // Sanitize and extract JSON-like substring
    let sanitized = ollama_utils::sanitize_raw_output(raw_response);
    
    // Try to find JSON boundaries in sanitized output
    let json_substring = if let Some(start) = sanitized.find('{') {
        // Find matching closing brace
        let mut brace_count = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let mut end_pos = None;
        
        for (i, ch) in sanitized[start..].char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }
            
            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => brace_count += 1,
                '}' if !in_string => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        end_pos = Some(start + i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        
        if let Some(end) = end_pos {
            &sanitized[start..end]
        } else {
            // Incomplete JSON, use what we have
            &sanitized[start..]
        }
    } else {
        // No JSON found, use sanitized output
        &sanitized
    };
    
    // Create strict repair prompt - NO hallucination, NO continuation
    let repair_prompt = format!(
        r#"You are a JSON repair model.

Input: A text that contains an incomplete or malformed JSON object.

Recover the JSON object exactly as written, fixing ONLY formatting errors.

Rules:
- Do NOT add any new fields.
- Do NOT guess missing content.
- Do NOT complete truncated arrays or objects.
- Do NOT change values or reorder keys.
- Only fix formatting: missing commas, missing quotes, bracket mismatches.

If the JSON is truncated (content ends mid-key, mid-value, mid-array, or brace_count != 0), return exactly:

"__TRUNCATED__"

Return only the corrected JSON with no commentary and no code fences.

Malformed JSON:
{}"#,
        json_substring
    );
    
    tracing::info!(
        fallback_model = fallback_model_name,
        json_substring_length = json_substring.len(),
        "Attempting to repair JSON with fallback model"
    );
    
    let repaired_raw = ollama::call_ollama_model(fallback_model_name, &repair_prompt).await
        .map_err(|e| ZosError::new(
            format!("Fallback model '{}' failed to repair JSON: {}", fallback_model_name, e),
            "json_repair"
        ))?;
    
    let sanitized_repaired = ollama_utils::sanitize_raw_output(&repaired_raw);
    
    // Check for "__TRUNCATED__" response
    if sanitized_repaired.trim() == "\"__TRUNCATED__\"" || sanitized_repaired.trim() == "__TRUNCATED__" {
        return Err(ZosError::new(
            "Fallback model detected truncation",
            "truncated_detected"
        ));
    }
    
    // Try to extract JSON from the repair attempt
    let json_str = ollama_utils::extract_json(&sanitized_repaired)
        .map_err(|e| ZosError::new(
            format!("Failed to extract JSON from repair attempt: {}", e),
            "json_repair_extract"
        ))?;
    
    serde_json::from_str::<T>(&json_str)
        .map_err(|e| ZosError::new(
            format!("Repaired JSON is invalid: {}", e),
            "json_repair_parse"
        ))
}
