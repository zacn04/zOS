use std::collections::HashMap;
use lazy_static::lazy_static;
use crate::models::base::LocalModel;
use crate::models::deepseek::DeepSeekModel;
use crate::models::qwen_math::QwenMathModel;
use crate::models::qwen_instruct::QwenInstructModel;
use crate::config::models::get_model_config;

lazy_static! {
    pub static ref MODEL_REGISTRY: HashMap<String, LocalModel> = {
        let mut m = HashMap::new();
        let config = get_model_config();
        
        // Register models based on config
        m.insert(
            config.proof_model.clone(),
            LocalModel::DeepSeek(DeepSeekModel::new("deepseek-r1:7b"))
        );
        m.insert(
            config.problem_model.clone(),
            LocalModel::QwenMath(QwenMathModel::new("qwen2-math:7b"))
        );
        m.insert(
            config.general_model.clone(),
            LocalModel::QwenInstruct(QwenInstructModel::new("qwen2.5:7b-instruct"))
        );
        
        // Also register common aliases
        m.insert("deepseek-r1:7b".to_string(), LocalModel::DeepSeek(DeepSeekModel::new("deepseek-r1:7b")));
        m.insert("qwen2-math:7b".to_string(), LocalModel::QwenMath(QwenMathModel::new("qwen2-math:7bh")));
        m.insert("qwen2.5:7b-instruct".to_string(), LocalModel::QwenInstruct(QwenInstructModel::new("qwen2.5:7b-instruct")));
        
        m
    };
}

pub fn get_model(name: &str) -> Option<LocalModel> {
    MODEL_REGISTRY.get(name).cloned()
}

/// Check if a model exists in Ollama by calling the API
pub fn model_exists_in_ollama(model: &str) -> bool {
    // Try a simple healthcheck by making a minimal request
    // For now, we'll assume models exist if they're in the registry
    // TODO: Implement actual Ollama API check
    MODEL_REGISTRY.contains_key(model)
}

/// Get all available model names
pub fn get_available_models() -> Vec<String> {
    MODEL_REGISTRY.keys().cloned().collect()
}

