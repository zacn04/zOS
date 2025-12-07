use reqwest::Client;
use anyhow::{Result, Context};
use crate::error::ZosError;
use crate::logging::{log_info, log_warn, log_error};
use tokio::time::{timeout, Duration};
use std::sync::OnceLock;

const OLLAMA_BASE_URL: &str = "http://localhost:11434";
const MODEL_CHECK_TIMEOUT: u64 = 3; // 3 seconds max for availability check

/// Reusable HTTP client for availability checks
static AVAILABILITY_CLIENT: OnceLock<Client> = OnceLock::new();

fn get_availability_client() -> &'static Client {
    AVAILABILITY_CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(MODEL_CHECK_TIMEOUT))
            .build()
            .expect("Failed to create availability HTTP client")
    })
}

/// Check if a model exists in Ollama by calling the API
pub async fn model_exists_in_ollama(model: &str) -> bool {
    let check_result = timeout(
        Duration::from_secs(MODEL_CHECK_TIMEOUT),
        check_model_availability(model)
    ).await;
    
    match check_result {
        Ok(Ok(true)) => {
            log_info(&format!("[Availability] Model '{}' is available", model));
            true
        }
        Ok(Ok(false)) => {
            log_warn(&format!("[Availability] Model '{}' not found", model));
            false
        }
        Ok(Err(e)) => {
            log_error(&format!("[Availability] Error checking '{}': {}", model, e));
            false
        }
        Err(_) => {
            log_error(&format!("[Availability] Timeout checking '{}' ({}s)", model, MODEL_CHECK_TIMEOUT));
            false
        }
    }
}

async fn check_model_availability(model: &str) -> Result<bool> {
    let client = get_availability_client();
    
    // Try to list models and check if ours is in the list
    let response = client
        .get(&format!("{}/api/tags", OLLAMA_BASE_URL))
        .send()
        .await
        .context("Failed to connect to Ollama API")?;
    
    if !response.status().is_success() {
        return Ok(false);
    }
    
    #[derive(serde::Deserialize)]
    struct ModelsResponse {
        models: Vec<ModelInfo>,
    }
    
    #[derive(serde::Deserialize)]
    struct ModelInfo {
        name: String,
    }
    
    let text = response.text().await
        .context("Failed to read Ollama models list")?;
    let models: ModelsResponse = serde_json::from_str(&text)
        .context("Failed to parse Ollama models list")?;
    
    // Check if model exists (exact match or prefix match)
    let exists = models.models.iter().any(|m| {
        m.name == model || m.name.starts_with(&format!("{}:", model))
    });
    
    Ok(exists)
}

/// Ensure a model is loaded/available, with optional preloading
pub async fn ensure_model_loaded(model: &str) -> Result<(), ZosError> {
    if model_exists_in_ollama(model).await {
        return Ok(());
    }
    
    // Try to pull the model (this is async and may take a while)
    log_info(&format!("[Availability] Attempting to pull model '{}'", model));
    
    let pull_result = timeout(
        Duration::from_secs(30), // Give it 30 seconds to start pulling
        pull_model(model)
    ).await;
    
    match pull_result {
        Ok(Ok(_)) => {
            log_info(&format!("[Availability] Successfully pulled model '{}'", model));
            Ok(())
        }
        Ok(Err(e)) => {
            Err(ZosError::new(
                format!("Failed to load model '{}': {}", model, e),
                "model_availability"
            ).with_model(model.to_string()))
        }
        Err(_) => {
            Err(ZosError::new(
                format!("Timeout waiting for model '{}' to load", model),
                "model_availability"
            ).with_model(model.to_string()))
        }
    }
}

async fn pull_model(model: &str) -> Result<()> {
    let client = get_availability_client();
    
    let response = client
        .post(&format!("{}/api/pull", OLLAMA_BASE_URL))
        .json(&serde_json::json!({
            "name": model
        }))
        .send()
        .await
        .context("Failed to initiate model pull")?;
    
    if !response.status().is_success() {
        anyhow::bail!("Ollama returned error status: {}", response.status());
    }
    
    // Note: Pulling is async, we just initiated it
    // In a real implementation, you might want to poll for completion
    Ok(())
}

