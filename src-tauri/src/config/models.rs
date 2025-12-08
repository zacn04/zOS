use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use lazy_static::lazy_static;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub proof_model: String,
    pub problem_model: String,
    pub general_model: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        ModelConfig {
            proof_model: "deepseek-r1:7b".to_string(),
            problem_model: "qwen2-math:7b".to_string(),
            general_model: "qwen2.5:7b-instruct".to_string(),
        }
    }
}

fn get_config_path() -> PathBuf {
    // Use platform-specific app data directory
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut dir = PathBuf::from(home);
            dir.push("Library/Application Support/com.zacnwo.zos");
            dir.push("models.toml");
            return dir;
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut dir = PathBuf::from(appdata);
            dir.push("com.zacnwo.zos");
            dir.push("models.toml");
            return dir;
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut dir = PathBuf::from(home);
            dir.push(".local/share/com.zacnwo.zos");
            dir.push("models.toml");
            return dir;
        }
    }
    
    // Fallback
    PathBuf::from("models.toml")
}

fn load_model_config_internal() -> ModelConfig {
    let config_path = get_config_path();
    
    // Try to load from config file
    if let Ok(content) = fs::read_to_string(&config_path) {
        if let Ok(config) = toml::from_str::<ModelConfig>(&content) {
            eprintln!("[Config] Loaded model config from: {:?}", config_path);
            return config;
        } else {
            eprintln!("[Config] Failed to parse models.toml, using defaults");
        }
    }
    
    // Return defaults if file doesn't exist or parsing fails
    eprintln!("[Config] Using default model configuration");
    ModelConfig::default()
}

lazy_static! {
    static ref MODEL_CONFIG: ModelConfig = load_model_config_internal();
}

/// Get the cached model configuration (loaded once at startup)
pub fn get_model_config() -> &'static ModelConfig {
    &MODEL_CONFIG
}

/// Legacy function for backward compatibility
pub fn load_model_config() -> ModelConfig {
    get_model_config().clone()
}

