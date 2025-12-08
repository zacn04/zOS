use crate::pipelines::ollama;
use crate::pipelines::ollama_utils;
use serde::de::DeserializeOwned;
use anyhow::{Result, Context};

#[derive(Clone)]
pub struct QwenInstructModel {
    model_name: &'static str,
}

impl QwenInstructModel {
    pub fn new(model_name: &'static str) -> Self {
        QwenInstructModel { model_name }
    }

    pub fn name(&self) -> &'static str {
        self.model_name
    }

    pub async fn call_json<T: DeserializeOwned>(&self, prompt: &str) -> Result<T> {
        let raw_response = ollama::call_ollama_model(self.model_name, prompt).await?;
        
        // Log raw response for debugging (first 1000 chars)
        tracing::debug!(
            model = self.model_name,
            raw_response_preview = %raw_response.chars().take(1000).collect::<String>(),
            "Qwen Instruct raw response"
        );
        
        let json_str = ollama_utils::extract_json(&raw_response)
            .with_context(|| format!(
                "Qwen Instruct model '{}' failed to extract JSON. Raw response (first 500 chars): {}", 
                self.model_name, 
                raw_response.chars().take(500).collect::<String>()
            ))?;
        
        // Log extracted JSON for debugging
        tracing::debug!(
            model = self.model_name,
            extracted_json_preview = %json_str.chars().take(500).collect::<String>(),
            "Qwen Instruct extracted JSON"
        );
        
        let parsed: T = serde_json::from_str(&json_str)
            .map_err(|e| {
                // Provide detailed error information
                anyhow::anyhow!(
                    "Qwen Instruct model '{}' returned invalid JSON.\n\
                    Parse error: {}\n\
                    Extracted JSON length: {}\n\
                    Extracted JSON (first 1000 chars): {}\n\
                    Extracted JSON (last 500 chars): {}\n\
                    Raw response (first 1000 chars): {}\n\
                    Raw response (last 500 chars): {}",
                    self.model_name,
                    e,
                    json_str.len(),
                    json_str.chars().take(1000).collect::<String>(),
                    json_str.chars().rev().take(500).collect::<String>().chars().rev().collect::<String>(),
                    raw_response.chars().take(1000).collect::<String>(),
                    raw_response.chars().rev().take(500).collect::<String>().chars().rev().collect::<String>()
                )
            })?;
        Ok(parsed)
    }

    pub async fn call_text(&self, prompt: &str) -> Result<String> {
        ollama::call_ollama_model(self.model_name, prompt).await
    }

    pub fn healthcheck(&self) -> bool {
        // TODO: Implement actual healthcheck
        true
    }
}

