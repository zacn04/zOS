use std::path::PathBuf;
use crate::skills::model::SkillVector;
use crate::error::ZosError;

fn skills_path() -> PathBuf {
    // Use platform-specific app data directory
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut dir = PathBuf::from(home);
            dir.push("Library/Application Support/com.zacnwo.zos");
            dir.push("skills.json");
            return dir;
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut dir = PathBuf::from(appdata);
            dir.push("com.zacnwo.zos");
            dir.push("skills.json");
            return dir;
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut dir = PathBuf::from(home);
            dir.push(".local/share/com.zacnwo.zos");
            dir.push("skills.json");
            return dir;
        }
    }
    
    // Fallback
    PathBuf::from("skills.json")
}

/// Load skill vector from disk asynchronously
pub async fn load_skill_vector() -> SkillVector {
    let path = skills_path();
    match tokio::fs::read_to_string(&path).await {
        Ok(data) => {
            match serde_json::from_str::<SkillVector>(&data) {
                Ok(vec) => vec,
                Err(e) => {
                    tracing::warn!(
                        path = ?path,
                        error = %e,
                        "Failed to parse skills.json, using defaults"
                    );
                    SkillVector::new()
                }
            }
        }
        Err(e) => {
            tracing::debug!(
                path = ?path,
                error = %e,
                "Failed to read skills.json, using defaults"
            );
            SkillVector::new()
        }
    }
}

/// Save skill vector to disk asynchronously
pub async fn save_skill_vector(v: &SkillVector) -> Result<(), ZosError> {
    let path = skills_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ZosError::new(
                format!("Failed to create directory: {}", e),
                "io"
            ).with_context(format!("path: {:?}", parent)))?;
    }
    
    let json = serde_json::to_string_pretty(v)
        .map_err(|e| ZosError::new(
            format!("Failed to serialize skills: {}", e),
            "json_serialize"
        ))?;
    
    tokio::fs::write(&path, json)
        .await
        .map_err(|e| ZosError::new(
            format!("Failed to write skills.json: {}", e),
            "io"
        ).with_context(format!("path: {:?}", path)))?;
    
    Ok(())
}

/// Synchronous version for backward compatibility (deprecated, use async version)
#[deprecated(note = "Use load_skill_vector().await instead")]
pub fn load_skill_vector_sync() -> SkillVector {
    let path = skills_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(vec) = serde_json::from_str::<SkillVector>(&data) {
            return vec;
        }
    }
    SkillVector::new()
}

