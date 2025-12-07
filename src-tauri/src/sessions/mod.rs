use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use crate::error::ZosError;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionRecord {
    pub session_id: String,
    pub problem_id: String,
    pub skill: String,
    pub user_attempt: String,
    pub issues: Vec<String>,
    pub eval_summary: String,
    pub skill_before: f32,
    pub skill_after: f32,
    #[serde(default = "default_difficulty")]
    pub difficulty: f32,
    pub timestamp: i64,
}

fn default_difficulty() -> f32 {
    0.5
}

pub fn sessions_dir() -> PathBuf {
    // Use platform-specific app data directory
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut dir = PathBuf::from(home);
            dir.push("Library/Application Support/com.zacnwo.zos");
            dir.push("data");
            dir.push("sessions");
            return dir;
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut dir = PathBuf::from(appdata);
            dir.push("com.zacnwo.zos");
            dir.push("data");
            dir.push("sessions");
            return dir;
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut dir = PathBuf::from(home);
            dir.push(".local/share/com.zacnwo.zos");
            dir.push("data");
            dir.push("sessions");
            return dir;
        }
    }
    
    // Fallback
    PathBuf::from("data/sessions")
}

/// Save a session record asynchronously
pub async fn save_session(record: &SessionRecord) -> Result<(), ZosError> {
    let dir = sessions_dir();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| ZosError::new(
            format!("Failed to create sessions directory: {}", e),
            "io"
        ).with_context(format!("path: {:?}", dir)))?;
    
    let fname = dir.join(format!("{}.json", record.session_id));
    let json = serde_json::to_string_pretty(record)
        .map_err(|e| ZosError::new(
            format!("Failed to serialize session record: {}", e),
            "json_serialize"
        ))?;
    
    tokio::fs::write(&fname, json)
        .await
        .map_err(|e| ZosError::new(
            format!("Failed to write session file: {}", e),
            "io"
        ).with_context(format!("path: {:?}", fname)))?;
    
    Ok(())
}

/// Load all session records asynchronously
pub async fn load_all_sessions() -> Result<Vec<SessionRecord>, ZosError> {
    let mut records = Vec::new();
    let dir = sessions_dir();

    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Directory doesn't exist yet, return empty vec
            return Ok(records);
        }
        Err(e) => {
            return Err(ZosError::new(
                format!("Failed to read sessions directory: {}", e),
                "io"
            ).with_context(format!("path: {:?}", dir)));
        }
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        match tokio::fs::read_to_string(&path).await {
            Ok(text) => {
                match serde_json::from_str::<SessionRecord>(&text) {
                    Ok(rec) => records.push(rec),
                    Err(e) => {
                        tracing::warn!(
                            path = ?path,
                            error = %e,
                            "Failed to parse session file"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    path = ?path,
                    error = %e,
                    "Failed to read session file"
                );
            }
        }
    }

    records.sort_by_key(|r| r.timestamp);
    Ok(records)
}

/// Synchronous version for backward compatibility (deprecated)
#[deprecated(note = "Use load_all_sessions().await instead")]
pub fn load_all_sessions_sync() -> Vec<SessionRecord> {
    let mut records = vec![];
    let dir = sessions_dir();

    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(text) = std::fs::read_to_string(entry.path()) {
                if let Ok(rec) = serde_json::from_str::<SessionRecord>(&text) {
                    records.push(rec);
                }
            }
        }
    }

    records.sort_by_key(|r| r.timestamp);
    records
}

/// Compute recent success rate for a skill
/// Returns the fraction of correct sessions in the last n attempts
/// If fewer than 3 attempts exist, returns 0.5 (neutral)
pub async fn recent_success_rate(skill: &str, n: usize) -> Result<f32, ZosError> {
    let all_sessions = load_all_sessions().await?;
    
    // Filter by skill and get last n sessions
    let mut relevant: Vec<_> = all_sessions
        .into_iter()
        .filter(|s| s.skill == skill)
        .collect();
    
    // Sort by timestamp (most recent last)
    relevant.sort_by_key(|s| s.timestamp);
    
    // Take last n sessions
    let recent: Vec<_> = relevant.iter().rev().take(n).collect();
    
    // Need at least 3 attempts for meaningful data
    if recent.len() < 3 {
        return 0.5;
    }
    
    // Count correct sessions
    // A session is considered correct if:
    // - eval_summary doesn't contain "incorrect" or "fail"
    // - skill_after >= skill_before (or close to it)
    let correct_count = recent.iter()
        .filter(|s| {
            let eval_lower = s.eval_summary.to_lowercase();
            !eval_lower.contains("incorrect") && 
            !eval_lower.contains("fail") &&
            s.skill_after >= s.skill_before - 0.01 // Allow tiny rounding errors
        })
        .count();
    
    Ok(correct_count as f32 / recent.len() as f32)
}

/// Synchronous version for backward compatibility (deprecated)
#[deprecated(note = "Use recent_success_rate().await instead")]
pub fn recent_success_rate_sync(skill: &str, n: usize) -> f32 {
    let all_sessions = load_all_sessions_sync();
    
    // Filter by skill and get last n sessions
    let mut relevant: Vec<_> = all_sessions
        .into_iter()
        .filter(|s| s.skill == skill)
        .collect();
    
    // Sort by timestamp (most recent last)
    relevant.sort_by_key(|s| s.timestamp);
    
    // Take last n sessions
    let recent: Vec<_> = relevant.iter().rev().take(n).collect();
    
    // Need at least 3 attempts for meaningful data
    if recent.len() < 3 {
        return 0.5;
    }
    
    // Count correct sessions
    let correct_count = recent.iter()
        .filter(|s| {
            let eval_lower = s.eval_summary.to_lowercase();
            !eval_lower.contains("incorrect") && 
            !eval_lower.contains("fail") &&
            s.skill_after >= s.skill_before - 0.01
        })
        .count();
    
    correct_count as f32 / recent.len() as f32
}

