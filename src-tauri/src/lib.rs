mod routes;
mod pipelines;
pub mod skills;
mod memory;
pub mod problems;
mod sessions;
mod brain;
mod config;
mod models;
mod error;
mod logging;
mod cache;
mod state;
mod metrics;

#[cfg(test)]
mod tests {
    // Re-export test modules
    #[path = "../tests/error_handling_test.rs"]
    mod error_handling_test;
    #[path = "../tests/json_extraction_test.rs"]
    mod json_extraction_test;
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize structured logging first
    logging::init_logging();
    tracing::info!("zOS application starting");

    // Initialize AppState
    let app_state = state::app::AppState::new();
    
    // Initialize problems directory (copy to app data if needed)
    // Note: This is still blocking, but it's a one-time setup
    problems::problem::Problem::initialize_problems_dir();
    
    // Initialize async runtime for startup tasks
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| error::ZosError::new(
            format!("Failed to create async runtime: {}", e),
            "startup"
        ))
        .expect("Failed to create async runtime");
    
    // Run async startup tasks
    rt.block_on(async {
        // Load skills from disk on startup
        let _skills = skills::store::load_skill_vector().await;
        tracing::info!("Skills loaded successfully");
        
        // Generate daily plan if it doesn't exist or is expired
        match brain::store::load().await {
            Ok(Some(plan)) => {
                if plan.is_expired() {
                    let new_plan = brain::generate_daily_plan().await;
                    if let Err(e) = brain::store::save(&new_plan).await {
                        tracing::warn!(error = %e, "Failed to save daily plan");
                    }
                }
            }
            Ok(None) => {
                // No plan exists, generate one
                let new_plan = brain::generate_daily_plan().await;
                if let Err(e) = brain::store::save(&new_plan).await {
                    tracing::warn!(error = %e, "Failed to save daily plan");
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to load daily plan");
            }
        }
        
        // Warm up models in background (non-blocking)
        tokio::spawn(async {
            models::warmup::warmup_models().await;
        });
    });
    
    // Store AppState in Tauri's managed state
    let app_state_arc = std::sync::Arc::new(app_state);
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state_arc.clone())
        .invoke_handler(tauri::generate_handler![
            routes::step1_analyze_proof,
            routes::step2_evaluate_answers,
            routes::get_recommended_problem,
            routes::precompute_next_problem,
            routes::get_problems_by_topic,
            routes::get_problem_by_id,
            routes::get_skills,
            routes::update_skills_from_issues,
            routes::save_session_record,
            routes::refresh_daily_plan,
            routes::get_daily_plan,
            routes::submit_problem_attempt
        ])
        .run(tauri::generate_context!())
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to run Tauri application");
            e
        })
        .expect("error while running tauri application");
}
