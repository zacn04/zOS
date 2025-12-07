use serde::{Serialize, Deserialize, Clone};
use std::sync::Arc;
use parking_lot::Mutex;
use tokio::time::{sleep, Duration};
use crate::skills::store::load_skill_vector;
use crate::problems::{problem::Problem, generator};
use crate::error::ZosError;

const CACHE_PATH: &str = "data/problems_cache.json";
const MIN_CACHE: usize = 12;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct ProblemCache {
    pub queue: Vec<Problem>,
}

impl ProblemCache {
    /// Load cache asynchronously
    pub async fn load_async() -> Self {
        // Try platform-specific paths
        let possible_paths = vec![
            std::path::Path::new(CACHE_PATH),
            std::path::Path::new("../data/problems_cache.json"),
            std::path::Path::new("./data/problems_cache.json"),
        ];

        for path in possible_paths {
            match tokio::fs::read_to_string(path).await {
                Ok(content) => {
                    match serde_json::from_str::<ProblemCache>(&content) {
                        Ok(cache) => return cache,
                        Err(e) => {
                            tracing::warn!(
                                path = ?path,
                                error = %e,
                                "Failed to parse cache file"
                            );
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    continue;
                }
                Err(e) => {
                    tracing::debug!(
                        path = ?path,
                        error = %e,
                        "Failed to read cache file"
                    );
                }
            }
        }

        Self::default()
    }

    /// Save cache asynchronously
    pub async fn save_async(&self) -> Result<(), ZosError> {
        // Try to save to data directory
        let possible_paths = vec![
            std::path::Path::new("data"),
            std::path::Path::new("../data"),
            std::path::Path::new("./data"),
        ];

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ZosError::new(
                format!("Failed to serialize cache: {}", e),
                "json_serialize"
            ))?;

        for base_path in possible_paths {
            if tokio::fs::create_dir_all(base_path).await.is_err() {
                continue;
            }
            let file_path = base_path.join("problems_cache.json");
            if tokio::fs::write(&file_path, &json).await.is_ok() {
                return Ok(());
            }
        }

        // Fallback: try current directory
        tokio::fs::write(CACHE_PATH, json)
            .await
            .map_err(|e| ZosError::new(
                format!("Failed to write cache file: {}", e),
                "io"
            ).with_context(format!("path: {}", CACHE_PATH)))?;
        Ok(())
    }

    /// Synchronous load for backward compatibility (deprecated)
    #[deprecated(note = "Use load_async().await instead")]
    pub fn load() -> Self {
        let possible_paths = vec![
            std::path::Path::new(CACHE_PATH),
            std::path::Path::new("../data/problems_cache.json"),
            std::path::Path::new("./data/problems_cache.json"),
        ];

        for path in possible_paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(cache) = serde_json::from_str::<ProblemCache>(&content) {
                    return cache;
                }
            }
        }

        Self::default()
    }

    /// Synchronous save for backward compatibility (deprecated)
    #[deprecated(note = "Use save_async().await instead")]
    pub fn save(&self) -> Result<(), ZosError> {
        let possible_paths = vec![
            std::path::Path::new("data"),
            std::path::Path::new("../data"),
            std::path::Path::new("./data"),
        ];

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| ZosError::new(
                format!("Failed to serialize cache: {}", e),
                "json_serialize"
            ))?;

        for base_path in possible_paths {
            if std::fs::create_dir_all(base_path).is_err() {
                continue;
            }
            let file_path = base_path.join("problems_cache.json");
            if std::fs::write(&file_path, &json).is_ok() {
                return Ok(());
            }
        }

        std::fs::write(CACHE_PATH, json)
            .map_err(|e| ZosError::new(
                format!("Failed to write cache file: {}", e),
                "io"
            ).with_context(format!("path: {}", CACHE_PATH)))
    }
}

pub async fn start_problem_prefetch(cache: Arc<Mutex<ProblemCache>>) {
    tokio::spawn(async move {
        loop {
            let needs_more = {
                let guard = cache.lock();
                guard.queue.len() < MIN_CACHE
            };
            
            if needs_more {
                let skills = load_skill_vector().await;
                let weakest = skills.weakest_n(2);
                
                for (skill, value) in weakest {
                    let diff = (0.3_f32).max(1.0 - value);
                    
                    // Generate new problem for this skill (outside mutex)
                    let generated = generator::generate_problem(&skill, diff).await;
                    
                    // Now lock and add to cache
                    let mut guard = cache.lock();
                    if guard.queue.len() >= MIN_CACHE {
                        break;
                    }
                    
                    match generated {
                        Ok(problem) => {
                            guard.queue.push(problem);
                        }
                        Err(e) => {
                            tracing::warn!(
                                skill = %skill,
                                error = %e,
                                "Failed to generate problem, trying fallback"
                            );
                            // Fallback to loading existing problems if generation fails
                            match Problem::load_all() {
                                Ok(all_problems) => {
                                    let matching: Vec<Problem> = all_problems.iter()
                                        .filter(|p| p.topic == skill)
                                        .cloned()
                                        .collect();
                                    
                                    for problem in matching.into_iter().take(2) {
                                        if guard.queue.len() >= MIN_CACHE {
                                            break;
                                        }
                                        guard.queue.push(problem);
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        skill = %skill,
                                        error = %e,
                                        "Failed to load problems for fallback"
                                    );
                                }
                            }
                        }
                    }
                    
                    // Save cache asynchronously (outside lock)
                    let cache_clone = guard.clone();
                    drop(guard);
                    if let Err(e) = cache_clone.save_async().await {
                        tracing::warn!(error = %e, "Failed to save problem cache");
                    }
                }
            }
            
            sleep(Duration::from_secs(60)).await;
        }
    });
}

