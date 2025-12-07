use anyhow::Result;
use sha2::{Sha256, Digest};
use std::fs;
use std::path::PathBuf;
use chrono::Utc;
use crate::problems::problem::Problem;
use crate::pipelines::router::TaskType;
use crate::pipelines::perf;

pub fn hash_statement(statement: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(statement.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn get_all_existing_statements() -> Vec<String> {
    let mut hashes = Vec::new();
    
    // Check problems directory
    let possible_paths = vec![
        std::path::Path::new("problems"),
        std::path::Path::new("../problems"),
        std::path::Path::new("./problems"),
    ];
    
    for problems_dir in possible_paths {
        if let Ok(entries) = fs::read_dir(problems_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(problem) = serde_json::from_str::<Problem>(&content) {
                            hashes.push(hash_statement(&problem.statement));
                        }
                    }
                }
            }
        }
    }
    
    // Check autogen directory
    let autogen_paths = vec![
        std::path::Path::new("problems/autogen"),
        std::path::Path::new("../problems/autogen"),
        std::path::Path::new("./problems/autogen"),
    ];
    
    for autogen_dir in autogen_paths {
        if let Ok(entries) = fs::read_dir(autogen_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(problem) = serde_json::from_str::<Problem>(&content) {
                            hashes.push(hash_statement(&problem.statement));
                        }
                    }
                }
            }
        }
    }
    
    hashes
}

fn get_autogen_dir() -> PathBuf {
    // FIRST: Try app data directory (production - same logic as Problem::load_all)
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut dir = PathBuf::from(home);
            dir.push("Library/Application Support/com.zacnwo.zos");
            dir.push("problems");
            dir.push("autogen");
            // Try to create if it doesn't exist
            if let Ok(_) = fs::create_dir_all(&dir) {
                return dir;
            }
            // If it already exists, use it
            if dir.exists() {
                return dir;
            }
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let mut dir = PathBuf::from(appdata);
            dir.push("com.zacnwo.zos");
            dir.push("problems");
            dir.push("autogen");
            if let Ok(_) = fs::create_dir_all(&dir) {
                return dir;
            }
            if dir.exists() {
                return dir;
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            let mut dir = PathBuf::from(home);
            dir.push(".local/share/com.zacnwo.zos");
            dir.push("problems");
            dir.push("autogen");
            if let Ok(_) = fs::create_dir_all(&dir) {
                return dir;
            }
            if dir.exists() {
                return dir;
            }
        }
    }
    
    // FALLBACK: Development paths (for dev mode)
    let possible_paths = vec![
        std::path::Path::new("problems/autogen"),
        std::path::Path::new("../problems/autogen"),
        std::path::Path::new("./problems/autogen"),
    ];
    
    for path in possible_paths {
        if path.exists() {
            return path.to_path_buf();
        }
    }
    
    // Last resort: create in current directory (dev mode)
    PathBuf::from("problems/autogen")
}

pub async fn generate_problem(skill: &str, diff: f32) -> Result<Problem> {
    use crate::pipelines::perf;
    let _perf = perf::PerfTimer::new("problem_generation_total");
    let difficulty_str = if diff < 0.3 {
        "easy"
    } else if diff < 0.6 {
        "medium"
    } else {
        "hard"
    };
    
    let prompt = format!(
        r#"Generate a new {difficulty_str} problem for the skill domain: {skill}.

Return ONLY valid JSON in the following schema:

{{
  "id": "autogen_<unique_id>",
  "topic": "{skill}",
  "difficulty": {diff},
  "statement": "the problem statement or question",
  "solution_sketch": "a brief outline of the solution approach as a single string (NOT an array or object)"
}}

Requirements:
- The problem should be appropriate for {difficulty_str} difficulty level
- The problem should be clearly stated and solvable
- The solution_sketch MUST be a single string (not an array or object) that provides guidance without giving away the full answer
- Make the problem unique and interesting
- For coding problems, include code snippets if relevant
- For math/proof problems, be precise and mathematical
- Output only valid JSON, no markdown or extra text
- IMPORTANT: solution_sketch must be a string, not an array or object

Generate the problem now:"#
    );
    
    // Use unified query system with caching, retry, and fallback
    use crate::pipelines::router::zos_query;
    use crate::error::ZosError;
    
    let mut problem: Problem = zos_query::<Problem>(TaskType::ProblemGeneration, prompt.clone())
        .await
        .map_err(|e: ZosError| anyhow::anyhow!("Failed to generate problem: {}", e.message))?;
    
    // Generate unique ID if missing or invalid
    if problem.id.is_empty() || !problem.id.starts_with("autogen_") {
        let timestamp = Utc::now().timestamp_millis();
        problem.id = format!("autogen_{}_{}", timestamp, skill);
    }
    
    // Ensure topic matches
    problem.topic = skill.to_string();
    problem.difficulty = diff;
    
    // Check for duplicates
    let dup_check_start = std::time::Instant::now();
    let statement_hash = hash_statement(&problem.statement);
    let existing_hashes = get_all_existing_statements();
    
    if existing_hashes.contains(&statement_hash) {
        anyhow::bail!("Generated problem is a duplicate of an existing problem");
    }
    let dup_check_ms = dup_check_start.elapsed().as_millis() as u64;
    perf::log_perf("problem_generation_dup_check", dup_check_ms);
    
    // Save to autogen directory
    let save_start = std::time::Instant::now();
    let autogen_dir = get_autogen_dir();
    fs::create_dir_all(&autogen_dir)?;
    
    let timestamp = Utc::now().timestamp();
    let filename = format!("{}_{}.json", timestamp, skill);
    let file_path = autogen_dir.join(&filename);
    
    fs::write(&file_path, serde_json::to_string_pretty(&problem)?)?;
    let save_ms = save_start.elapsed().as_millis() as u64;
    perf::log_perf("problem_generation_save", save_ms);
    
    Ok(problem)
}

