use crate::pipelines::ollama;
use crate::pipelines::ollama_utils;
use serde::de::DeserializeOwned;
use anyhow::{Result, Context};

#[derive(Clone)]
pub struct QwenMathModel {
    model_name: &'static str,
}

impl QwenMathModel {
    pub fn new(model_name: &'static str) -> Self {
        QwenMathModel { model_name }
    }

    pub fn name(&self) -> &'static str {
        self.model_name
    }

    pub async fn call_json<T: DeserializeOwned>(&self, prompt: &str) -> Result<T> {
        let raw_response = ollama::call_ollama_model(self.model_name, prompt).await?;
        let json_str = ollama_utils::extract_json(&raw_response)
            .with_context(|| format!("Qwen Math model '{}' failed to extract JSON", self.model_name))?;
        let parsed: T = serde_json::from_str(&json_str)
            .with_context(|| format!("Qwen Math model '{}' returned invalid JSON", self.model_name))?;
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

