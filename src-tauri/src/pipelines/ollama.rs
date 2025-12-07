use reqwest::Client;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use anyhow::{Result, Context};
use crate::pipelines::ollama_utils;
use crate::pipelines::perf;
use tokio::time::{timeout, Duration};
use crate::logging::{log_model_call, log_timeout};
use std::sync::OnceLock;

const DEFAULT_TIMEOUT_SECS: u64 = 60; // 60 seconds default timeout

/// Reusable HTTP client singleton (created once, reused for all requests)
static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn get_http_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .tcp_keepalive(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()
            .expect("Failed to create HTTP client")
    })
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
    #[serde(default)]
    done: bool,
}

/// Call an Ollama model and return the raw response text (with timeout)
pub async fn call_ollama_model(model: &str, prompt: &str) -> Result<String> {
    call_ollama_model_with_timeout(model, prompt, Duration::from_secs(DEFAULT_TIMEOUT_SECS)).await
}

/// Call an Ollama model with a custom timeout
pub async fn call_ollama_model_with_timeout(
    model: &str, 
    prompt: &str, 
    timeout_duration: Duration
) -> Result<String> {
    let _perf = perf::PerfTimer::new("ollama_call");
    let start = std::time::Instant::now();
    
    let result = timeout(timeout_duration, async {
        let client = get_http_client();
        let request_start = std::time::Instant::now();

        let response = client
            .post("http://localhost:11434/api/generate")
            .json(&OllamaRequest {
                model: model.to_string(),
                prompt: prompt.to_string(),
                stream: true, // Enable streaming for better UX
            })
            .send()
            .await
            .with_context(|| format!("Failed to connect to Ollama API for model '{}'", model))?;

        let connect_ms = request_start.elapsed().as_millis() as u64;
        perf::log_perf_with_context("ollama_connect", connect_ms, model);

        // For now, read the full response (streaming can be enhanced later)
        let read_start = std::time::Instant::now();
        let text = response.text().await
            .with_context(|| format!("Failed to read response from model '{}'", model))?;
        let read_ms = read_start.elapsed().as_millis() as u64;
        perf::log_perf_with_context("ollama_read", read_ms, model);
        
        // Parse streaming response (one JSON object per line)
        let parse_start = std::time::Instant::now();
        let mut full_response = String::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(res) = serde_json::from_str::<OllamaResponse>(line) {
                full_response.push_str(&res.response);
                if res.done {
                    break;
                }
            }
        }
        let parse_ms = parse_start.elapsed().as_millis() as u64;
        perf::log_perf_with_context("ollama_parse_stream", parse_ms, model);
        
        if full_response.is_empty() {
            anyhow::bail!("Model '{}' returned empty response", model);
        }
        
        Ok(full_response)
    }).await;
    
    let latency_ms = start.elapsed().as_millis() as u64;
    
    match result {
        Ok(Ok(response)) => {
            perf::log_perf_with_context("ollama_call", latency_ms, model);
            log_model_call(model, "call", true, Some(latency_ms));
            Ok(response)
        }
        Ok(Err(e)) => {
            perf::log_perf_with_context("ollama_call_error", latency_ms, model);
            log_model_call(model, "call", false, Some(latency_ms));
            Err(e)
        }
        Err(_) => {
            perf::log_perf_with_context("ollama_call_timeout", latency_ms, model);
            log_timeout(model, timeout_duration.as_secs());
            anyhow::bail!("Model '{}' call timed out after {}s", model, timeout_duration.as_secs())
        }
    }
}

/// Call an Ollama model and parse the response as JSON into a typed struct (with timeout)
pub async fn call_ollama_json<T: DeserializeOwned>(model: &str, prompt: &str) -> Result<T> {
    call_ollama_json_with_timeout(model, prompt, Duration::from_secs(DEFAULT_TIMEOUT_SECS)).await
}

/// Call an Ollama model and parse JSON with custom timeout
pub async fn call_ollama_json_with_timeout<T: DeserializeOwned>(
    model: &str, 
    prompt: &str,
    timeout_duration: Duration
) -> Result<T> {
    let _perf = perf::PerfTimer::new("ollama_json");
    let raw_response = call_ollama_model_with_timeout(model, prompt, timeout_duration).await?;
    
    // Extract JSON from the response (handle markdown, code blocks, etc.)
    let extract_start = std::time::Instant::now();
    let json_str = ollama_utils::extract_json(&raw_response)
        .with_context(|| format!("Failed to extract JSON from model '{}' response", model))?;
    let extract_ms = extract_start.elapsed().as_millis() as u64;
    perf::log_perf_with_context("json_extract", extract_ms, model);
    
    // Parse the JSON
    let parse_start = std::time::Instant::now();
    let parsed: T = serde_json::from_str(&json_str)
        .with_context(|| format!("Model '{}' returned invalid JSON. Raw: {}", model, json_str))?;
    let parse_ms = parse_start.elapsed().as_millis() as u64;
    perf::log_perf_with_context("json_parse", parse_ms, model);
    
    Ok(parsed)
}

