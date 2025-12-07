use crate::pipelines::ollama;
use crate::pipelines::ollama_utils;
use serde::de::DeserializeOwned;
use anyhow::{Result, Context};

#[derive(Clone)]
pub struct DeepSeekModel {
    model_name: &'static str,
}

impl DeepSeekModel {
    pub fn new(model_name: &'static str) -> Self {
        DeepSeekModel { model_name }
    }

    pub fn name(&self) -> &'static str {
        self.model_name
    }

    pub async fn call_json<T: DeserializeOwned>(&self, prompt: &str) -> Result<T> {
        let raw_response = ollama::call_ollama_model(self.model_name, prompt).await?;
        
        // Log raw response for debugging (first 500 chars)
        eprintln!("[DeepSeek] Raw response (first 500 chars): {}", 
            raw_response.chars().take(500).collect::<String>());
        
        let json_str = ollama_utils::extract_json(&raw_response)
            .with_context(|| format!("DeepSeek model '{}' failed to extract JSON. Raw response (first 500 chars): {}", 
                self.model_name, raw_response.chars().take(500).collect::<String>()))?;
        
        // Log extracted JSON for debugging
        eprintln!("[DeepSeek] Extracted JSON (first 500 chars): {}", 
            json_str.chars().take(500).collect::<String>());
        
        let parsed: T = serde_json::from_str(&json_str)
            .with_context(|| format!(
                "DeepSeek model '{}' returned invalid JSON.\nExtracted JSON: {}\nRaw response (first 1000 chars): {}", 
                self.model_name, 
                json_str,
                raw_response.chars().take(1000).collect::<String>()
            ))?;
        Ok(parsed)
    }

    pub async fn call_text(&self, prompt: &str) -> Result<String> {
        ollama::call_ollama_model(self.model_name, prompt).await
    }

    pub fn healthcheck(&self) -> bool {
        // TODO: Implement actual healthcheck by calling Ollama API
        true
    }
}
