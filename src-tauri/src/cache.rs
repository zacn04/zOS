use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use serde::{Serialize, Deserialize};
use crate::state::app::AppState;
use crate::error::ZosError;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedResponse {
    pub data: String,
    pub timestamp: i64,
}

/// Generate a hash key from model name and prompt
fn cache_key(model: &str, prompt: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    model.hash(&mut hasher);
    prompt.hash(&mut hasher);
    hasher.finish()
}

/// Check cache and return if found
pub fn get_cached<T: for<'de> Deserialize<'de>>(
    state: &AppState,
    model: &str,
    prompt: &str,
) -> Option<T> {
    let key = cache_key(model, prompt);
    let cache = state.response_cache.read();
    
    if let Some(cached) = cache.peek(&key) {
        tracing::debug!(
            model = model,
            prompt_preview = &prompt[..prompt.len().min(50)],
            "Cache hit"
        );
        match serde_json::from_str::<T>(&cached.data) {
            Ok(parsed) => return Some(parsed),
            Err(e) => {
                tracing::warn!(
                    model = model,
                    error = %e,
                    "Failed to parse cached response"
                );
            }
        }
    }
    
    tracing::debug!(
        model = model,
        prompt_preview = &prompt[..prompt.len().min(50)],
        "Cache miss"
    );
    None
}

/// Store response in cache
pub fn cache_response<T: Serialize>(
    state: &AppState,
    model: &str,
    prompt: &str,
    response: &T,
) -> Result<(), ZosError> {
    let key = cache_key(model, prompt);
    let data = serde_json::to_string(response)
        .map_err(|e| ZosError::new(
            format!("Failed to serialize response for cache: {}", e),
            "json_serialize"
        ))?;
    
    let cached = CachedResponse {
        data,
        timestamp: chrono::Utc::now().timestamp(),
    };
    
    let mut cache = state.response_cache.write();
    cache.put(key, cached);
    Ok(())
}

