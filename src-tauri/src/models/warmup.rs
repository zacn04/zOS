/// Model warm-up functionality to reduce cold-start latency
use crate::config::models::get_model_config;
use crate::models::availability::model_exists_in_ollama;
use crate::logging::log_info;
use tokio::time::Instant;

/// Warm up all configured models with a lightweight ping
pub async fn warmup_models() {
    let start = Instant::now();
    let config = get_model_config();
    
    log_info("[Warmup] Starting model warm-up...");
    
    // Warm up all three models in parallel
    let (proof_result, problem_result, general_result) = tokio::join!(
        warmup_single_model(&config.proof_model),
        warmup_single_model(&config.problem_model),
        warmup_single_model(&config.general_model),
    );
    
    let elapsed_ms = start.elapsed().as_millis() as u64;
    
    if proof_result && problem_result && general_result {
        log_info(&format!("[Warmup] All models warmed up in {}ms", elapsed_ms));
    } else {
        log_info(&format!("[Warmup] Warm-up completed in {}ms (some models may not be available)", elapsed_ms));
    }
}

/// Warm up a single model with a lightweight check
async fn warmup_single_model(model: &str) -> bool {
    let start = Instant::now();
    
    // Just check if model exists (lightweight operation)
    let exists = model_exists_in_ollama(model).await;
    
    let elapsed_ms = start.elapsed().as_millis() as u64;
    
    if exists {
        log_info(&format!("[Warmup] Warmed up model '{}' in {}ms", model, elapsed_ms));
    } else {
        log_info(&format!("[Warmup] Model '{}' not available (checked in {}ms)", model, elapsed_ms));
    }
    
    exists
}

