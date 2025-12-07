use std::path::PathBuf;
use crate::brain::CurriculumPlan;
use crate::error::ZosError;

pub fn get_plan_path() -> PathBuf {
    // Use platform-specific app data directory
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut dir = PathBuf::from(home);
            dir.push("Library/Application Support/com.zacnwo.zos");
            dir.push("data");
            dir.push("daily_plan.json");
            return dir;
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut dir = PathBuf::from(appdata);
            dir.push("com.zacnwo.zos");
            dir.push("data");
            dir.push("daily_plan.json");
            return dir;
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut dir = PathBuf::from(home);
            dir.push(".local/share/com.zacnwo.zos");
            dir.push("data");
            dir.push("daily_plan.json");
            return dir;
        }
    }
    
    // Fallback
    PathBuf::from("data/daily_plan.json")
}

/// Save curriculum plan asynchronously
pub async fn save(plan: &CurriculumPlan) -> Result<(), ZosError> {
    let path = get_plan_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ZosError::new(
                format!("Failed to create directory: {}", e),
                "io"
            ).with_context(format!("path: {:?}", parent)))?;
    }
    
    let json = serde_json::to_string_pretty(plan)
        .map_err(|e| ZosError::new(
            format!("Failed to serialize plan: {}", e),
            "json_serialize"
        ))?;
    
    tokio::fs::write(&path, json)
        .await
        .map_err(|e| ZosError::new(
            format!("Failed to write daily_plan.json: {}", e),
            "io"
        ).with_context(format!("path: {:?}", path)))?;
    
    Ok(())
}

/// Load curriculum plan asynchronously
pub async fn load() -> Result<Option<CurriculumPlan>, ZosError> {
    let path = get_plan_path();
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => {
            serde_json::from_str(&content)
                .map_err(|e| ZosError::new(
                    format!("Failed to parse daily_plan.json: {}", e),
                    "json_parse"
                ).with_context(format!("path: {:?}", path)))
                .map(Some)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok(None)
        }
        Err(e) => {
            Err(ZosError::new(
                format!("Failed to read daily_plan.json: {}", e),
                "io"
            ).with_context(format!("path: {:?}", path)))
        }
    }
}

/// Synchronous version for backward compatibility (deprecated)
#[deprecated(note = "Use load().await instead")]
pub fn load_sync() -> Option<CurriculumPlan> {
    let path = get_plan_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

