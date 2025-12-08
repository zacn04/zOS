use serde::de::DeserializeOwned;
use crate::models::deepseek::DeepSeekModel;
use crate::models::qwen_math::QwenMathModel;
use crate::models::qwen_instruct::QwenInstructModel;

/// Unified model wrapper enum
#[derive(Clone)]
pub enum LocalModel {
    DeepSeek(DeepSeekModel),
    QwenMath(QwenMathModel),
    QwenInstruct(QwenInstructModel),
}

impl LocalModel {
    pub fn name(&self) -> &'static str {
        match self {
            LocalModel::DeepSeek(m) => m.name(),
            LocalModel::QwenMath(m) => m.name(),
            LocalModel::QwenInstruct(m) => m.name(),
        }
    }

    pub async fn call_json<T: DeserializeOwned>(&self, prompt: &str) -> anyhow::Result<T> {
        match self {
            LocalModel::DeepSeek(m) => m.call_json(prompt).await,
            LocalModel::QwenMath(m) => m.call_json(prompt).await,
            LocalModel::QwenInstruct(m) => m.call_json(prompt).await,
        }
    }
    

    pub async fn call_text(&self, prompt: &str) -> anyhow::Result<String> {
        match self {
            LocalModel::DeepSeek(m) => m.call_text(prompt).await,
            LocalModel::QwenMath(m) => m.call_text(prompt).await,
            LocalModel::QwenInstruct(m) => m.call_text(prompt).await,
        }
    }

    pub fn healthcheck(&self) -> bool {
        match self {
            LocalModel::DeepSeek(m) => m.healthcheck(),
            LocalModel::QwenMath(m) => m.healthcheck(),
            LocalModel::QwenInstruct(m) => m.healthcheck(),
        }
    }
}

