/// Initialize structured logging with tracing
/// This should be called once at application startup
pub fn init_logging() {
    use tracing_subscriber::fmt;
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .json() // JSON output for structured logging
        );

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber");
    
    tracing::info!("Structured logging initialized");
}

/// Legacy logging functions for backward compatibility
/// These now delegate to tracing
#[deprecated(note = "Use tracing::debug! macro instead")]
pub fn log_debug(msg: &str) {
    tracing::debug!("{}", msg);
}

#[deprecated(note = "Use tracing::info! macro instead")]
pub fn log_info(msg: &str) {
    tracing::info!("{}", msg);
}

#[deprecated(note = "Use tracing::warn! macro instead")]
pub fn log_warn(msg: &str) {
    tracing::warn!("{}", msg);
}

#[deprecated(note = "Use tracing::error! macro instead")]
pub fn log_error(msg: &str) {
    tracing::error!("{}", msg);
}

#[deprecated(note = "Use tracing macros with structured fields instead")]
pub fn log_routing(task: &str, model: &str, latency_ms: Option<u64>) {
    if let Some(latency) = latency_ms {
        tracing::info!(task = task, model = model, latency_ms = latency, "Routing decision");
    } else {
        tracing::info!(task = task, model = model, "Routing decision");
    }
}

#[deprecated(note = "Use tracing macros with structured fields instead")]
pub fn log_model_call(model: &str, stage: &str, success: bool, latency_ms: Option<u64>) {
    if let Some(latency) = latency_ms {
        tracing::info!(
            model = model,
            stage = stage,
            success = success,
            latency_ms = latency,
            "Model call"
        );
    } else {
        tracing::info!(
            model = model,
            stage = stage,
            success = success,
            "Model call"
        );
    }
}

#[deprecated(note = "Use tracing macros with structured fields instead")]
pub fn log_fallback(from: &str, to: &str) {
    tracing::warn!(from = from, to = to, "Fallback triggered");
}

#[deprecated(note = "Use tracing macros with structured fields instead")]
pub fn log_timeout(model: &str, duration_secs: u64) {
    tracing::error!(model = model, duration_secs = duration_secs, "Timeout exceeded");
}

#[deprecated(note = "Use tracing macros with structured fields instead")]
pub fn log_cache_hit(key: &str) {
    tracing::debug!(key = key, "Cache hit");
}

#[deprecated(note = "Use tracing macros with structured fields instead")]
pub fn log_cache_miss(key: &str) {
    tracing::debug!(key = key, "Cache miss");
}

