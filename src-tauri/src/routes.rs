use crate::pipelines::proof::{
    call_deepseek_step1, call_deepseek_step2, ProofIssue, Step1Response, Step2Response,
};
use crate::problems::{problem::Problem, selector, generator};
use crate::skills::{model::SkillVector, store as skills_store};
use crate::memory::store;
use crate::sessions::{SessionRecord, save_session, load_all_sessions, recent_success_rate};
use crate::brain::TaskDirective;
use crate::analytics::{AnalyticsPayload, compute_analytics};
use crate::state::session::{get_state, set_state, reset_state, log_state, ProofState};
use crate::state::app::AppState;
use chrono::Utc;
use tauri::State;

/// Anneal difficulty based on success rate
/// - If success > 0.7 → increase difficulty by +0.1
/// - If success < 0.4 → decrease difficulty by -0.1
/// - Else → leave unchanged
/// Always clamp to [0.1, 1.0]
fn anneal_difficulty(base: f32, success: f32) -> f32 {
    let new_diff = if success > 0.7 {
        base + 0.1
    } else if success < 0.4 {
        base - 0.1
    } else {
        base
    };
    
    new_diff.max(0.1).min(1.0)
}

#[tauri::command]
pub async fn run_proof_pipeline(
    state: State<'_, std::sync::Arc<AppState>>,
    proof: String,
) -> Result<Step1Response, String> {
    call_deepseek_step1(state.inner(), &proof)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_proof_followup(
    state: State<'_, std::sync::Arc<AppState>>,
    original_proof: String,
    issues_json: String,
    questions: String,
    user_answers: String,
) -> Result<Step2Response, String> {
    let app_state = state.inner();
    match call_deepseek_step2(app_state, &original_proof, &issues_json, &questions, &user_answers).await {
        Ok(response) => Ok(response),
        Err(e) => Err(format!("Model error: {}", e)),
    }
}

#[tauri::command]
pub async fn step1_analyze_proof(
    state: State<'_, std::sync::Arc<AppState>>,
    proof: String,
    problem_id: Option<String>,
    problem_topic: Option<String>,
    problem_difficulty: Option<f32>,
) -> Result<Step1Response, String> {
    let app_state = state.inner();
    
    // Check state - Step 1 should only run when AwaitingSolution or AwaitingRevision
    let current_state = get_state(app_state);
    log_state(app_state);
    
    match &current_state {
        ProofState::AwaitingSolution | ProofState::AwaitingRevision { .. } => {
            // Valid state, proceed with Step 1
        }
        ProofState::AwaitingClarifyingAnswers { .. } => {
            return Err("Please answer the clarifying questions first (Step 2)".to_string());
        }
    }
    
    // Get skill before update
    let skills_before = store::get_skills(app_state).await
        .map_err(|e| format!("Failed to get skills: {}", e))?;
    let skill_before = problem_topic.as_ref()
        .and_then(|topic| skills_before.skills.get(topic))
        .copied()
        .unwrap_or(0.5);
    
    match call_deepseek_step1(app_state, &proof).await {
        Ok(response) => {
            // Update state to AwaitingClarifyingAnswers
            set_state(app_state, ProofState::AwaitingClarifyingAnswers {
                step1_response: response.clone(),
            });
            log_state(app_state);
            
            // Update skills based on issues found
            store::update_skills(app_state, |skills| {
                skills.update_from_issues(&response.issues);
            })
            .await
            .map_err(|e| format!("Failed to update skills: {}", e))?;
            
            // Check if proof is perfect (no issues and no questions)
            if response.issues.is_empty() && response.questions.is_empty() {
                if let Some(topic) = &problem_topic {
                    store::update_skills(app_state, |skills| {
                        skills.update_for_perfect_proof(topic);
                    })
                    .await
                    .map_err(|e| format!("Failed to update skills for perfect proof: {}", e))?;
                }
                
                // Save session record for perfect proof
                if let (Some(pid), Some(topic)) = (problem_id, problem_topic) {
                    let skills_after = store::get_skills(app_state).await
                        .map_err(|e| format!("Failed to get skills: {}", e))?;
                    let skill_after = skills_after.skills.get(&topic)
                        .copied()
                        .unwrap_or(0.5);
                    
                    let issues_list: Vec<String> = response.issues.iter()
                        .map(|i| format!("{}: {}", i.step_id, i.explanation))
                        .collect();
                    
                    let record = SessionRecord {
                        session_id: format!("sess_{}", Utc::now().timestamp_millis()),
                        problem_id: pid,
                        skill: topic,
                        user_attempt: proof,
                        issues: issues_list,
                        eval_summary: "Perfect solution - no issues, no questions".to_string(),
                        skill_before,
                        skill_after,
                        difficulty: problem_difficulty.unwrap_or(0.5),
                        timestamp: Utc::now().timestamp(),
                    };
                    
                    if let Err(e) = save_session(&record).await {
                        tracing::warn!(error = %e, "Failed to save session record for perfect proof");
                    }
                }
            }
            
            Ok(response)
        }
        Err(e) => Err(format!("Model error: {}", e)),
    }
}

#[tauri::command]
pub async fn step2_evaluate_answers(
    state: State<'_, std::sync::Arc<AppState>>,
    proof: String,
    issues: Vec<ProofIssue>,
    questions: Vec<String>,
    answers: Vec<String>,
    problem_id: Option<String>,
    problem_topic: Option<String>,
    problem_difficulty: Option<f32>,
) -> Result<Step2Response, String> {
    let app_state = state.inner();
    
    // Check state - Step 2 should only run when AwaitingClarifyingAnswers
    let current_state = get_state(app_state);
    log_state(app_state);
    
    match &current_state {
        ProofState::AwaitingClarifyingAnswers { .. } => {
            // Valid state, proceed with Step 2
        }
        ProofState::AwaitingSolution => {
            return Err("Please submit a solution first (Step 1)".to_string());
        }
        ProofState::AwaitingRevision { .. } => {
            return Err("Please revise your solution and resubmit (Step 1)".to_string());
        }
    }
    
    // Convert structured data to JSON strings for the prompt
    let issues_json = serde_json::to_string(&issues)
        .map_err(|e| format!("Failed to serialize issues: {}", e))?;
    let questions_json = serde_json::to_string(&questions)
        .map_err(|e| format!("Failed to serialize questions: {}", e))?;
    let answers_json = serde_json::to_string(&answers)
        .map_err(|e| format!("Failed to serialize answers: {}", e))?;

    // Get skill before update
    let skills_before = store::get_skills(app_state).await
        .map_err(|e| format!("Failed to get skills: {}", e))?;
    let skill_before = problem_topic.as_ref()
        .and_then(|topic| skills_before.skills.get(topic))
        .copied()
        .unwrap_or(0.5);

    match call_deepseek_step2(app_state, &proof, &issues_json, &questions_json, &answers_json).await {
        Ok(response) => {
            // Update state to AwaitingRevision
            set_state(app_state, ProofState::AwaitingRevision {
                step2_response: response.clone(),
            });
            log_state(app_state);
            
            // Update skills based on evaluation
            store::update_skills(app_state, |skills| {
                skills.update_from_evaluation(&response.evaluation);
            })
            .await
            .map_err(|e| format!("Failed to update skills: {}", e))?;
            
            // Get skill after update
            let skills_after = store::get_skills(app_state).await
                .map_err(|e| format!("Failed to get skills: {}", e))?;
            let skill_after = problem_topic.as_ref()
                .and_then(|topic| skills_after.skills.get(topic))
                .copied()
                .unwrap_or(0.5);

            // Save session record if we have problem info
            if let (Some(pid), Some(topic)) = (problem_id, problem_topic) {
                let issues_list: Vec<String> = issues.iter()
                    .map(|i| format!("{}: {}", i.step_id, i.explanation))
                    .collect();
                
                let eval_summary = format!("{} evaluations", response.evaluation.len());
                
                let record = SessionRecord {
                    session_id: format!("sess_{}", Utc::now().timestamp_millis()),
                    problem_id: pid,
                    skill: topic,
                    user_attempt: proof.clone(),
                    issues: issues_list,
                    eval_summary,
                    skill_before,
                    skill_after,
                    difficulty: problem_difficulty.unwrap_or(0.5),
                    timestamp: Utc::now().timestamp(),
                };

                if let Err(e) = save_session(&record).await {
                    tracing::warn!(error = %e, "Failed to save session record");
                }
            }

            // Save skills to persistent store
            let skills_final = store::get_skills(app_state).await
                .map_err(|e| format!("Failed to get skills: {}", e))?;
            if let Err(e) = skills_store::save_skill_vector(&skills_final).await {
                tracing::warn!(error = %e, "Failed to save skills");
            }

            Ok(response)
        }
        Err(e) => Err(format!("Model error: {}", e)),
    }
}

#[tauri::command]
pub async fn get_recommended_problem(
    state: State<'_, std::sync::Arc<AppState>>,
) -> Result<Problem, String> {
    let app_state = state.inner();
    // Reset state when getting a new problem (user explicitly requested a new problem)
    reset_state(app_state);
    log_state(app_state);
    
    let skills = store::get_skills(app_state).await
        .map_err(|e| format!("Failed to get skills: {}", e))?;
    let problems = Problem::load_all()
        .map_err(|e| format!("Failed to load problems: {}", e))?;
    
    // Find weakest skill (now with random selection for ties)
    let weakest_skill = match skills.get_weakest_skill() {
        Some((skill_name, _)) => skill_name,
        None => {
            // If no skill found, try to generate for first available skill
            if let Some((skill, _)) = skills.skills.iter().next() {
                skill.clone()
            } else {
                return Err("No skills available".to_string());
            }
        }
    };
    
    // Get list of completed problem IDs to exclude
    let completed_problem_ids: std::collections::HashSet<String> = {
        let sessions = load_all_sessions().await.unwrap_or_default();
        sessions.into_iter().map(|s| s.problem_id).collect()
    };
    
    // FIRST: Try to get a cached problem (fast, no LLM call) - exclude completed ones
    let mut cached = crate::problems::cache::ProblemCache::load_async().await;
    if let Some(pos) = cached.queue.iter()
        .position(|p| p.topic == weakest_skill && !completed_problem_ids.contains(&p.id)) {
        let problem = cached.queue.remove(pos);
        // Save updated cache
        let _ = cached.save_async().await;
        tracing::info!(skill = %weakest_skill, problem_id = %problem.id, "Using cached problem (not completed)");
        return Ok(problem);
    }
    
    // SECOND: Try static problems (exclude completed ones) - prefer uncompleted
    let available_problems: Vec<&Problem> = problems.iter()
        .filter(|p| !completed_problem_ids.contains(&p.id))
        .collect();
    
    if let Some(static_problem) = selector::pick_problem_from_list(&skills, &available_problems) {
        tracing::info!(skill = %weakest_skill, problem_id = %static_problem.id, "Using static problem (not completed)");
        return Ok(static_problem.clone());
    }
    
    // If all problems are completed, allow repeats (fallback)
    if let Some(static_problem) = selector::pick_problem(&skills, &problems) {
        tracing::info!(skill = %weakest_skill, problem_id = %static_problem.id, "Using static problem (all completed, allowing repeat)");
        return Ok(static_problem);
    }
    
    // THIRD: Try to get a task from the daily plan (may generate, but only if needed)
    if let Some(mut plan) = crate::brain::store::load().await
        .map_err(|e| format!("Failed to load plan: {}", e))? {
        if !plan.is_expired() && !plan.tasks.is_empty() {
            // Pop first directive
            let directive = plan.tasks.remove(0);
            
            // Save back reduced plan
            if let Err(e) = crate::brain::store::save(&plan).await {
                eprintln!("Failed to save updated plan: {}", e);
            }
            
            match directive {
                TaskDirective::Adaptive { skill, difficulty: base_difficulty } => {
                    // Apply difficulty annealing based on recent performance
                    let success_rate = recent_success_rate(&skill, 5).await
                        .unwrap_or(0.5);
                    let annealed_difficulty = anneal_difficulty(base_difficulty, success_rate);
                    
                tracing::info!(
                    skill = %skill,
                    success_rate = success_rate,
                    base_difficulty = base_difficulty,
                    annealed_difficulty = annealed_difficulty,
                    "Plan task with difficulty annealing"
                );
                    
                    // Generate a new problem for this skill with annealed difficulty
                    match generator::generate_problem(app_state, &skill, annealed_difficulty).await {
                        Ok(problem) => return Ok(problem),
                        Err(e) => {
                            tracing::warn!(skill = %skill, error = %e, "Failed to generate problem");
                            // Fall through to final fallback
                        }
                    }
                }
                TaskDirective::Review { skill } => {
                    // Find a failed problem for this skill to review
                    let fails = load_all_sessions().await
                        .map_err(|e| format!("Failed to load sessions: {}", e))?;
                    if let Some(fail) = fails.into_iter()
                        .rev()
                        .find(|s| s.skill == skill && 
                             (s.eval_summary.contains("incorrect") || 
                              s.eval_summary.contains("fail") ||
                              s.skill_after < s.skill_before)) {
                        if let Ok(all_problems) = Problem::load_all() {
                            if let Some(problem) = all_problems.into_iter()
                                .find(|p| p.id == fail.problem_id) {
                                return Ok(problem);
                            }
                        }
                    }
                    // Fall through to final fallback
                }
            }
        }
    }
    
    // FINAL FALLBACK: Generate a problem with difficulty annealing (slow, LLM call)
    // Only if no uncompleted problems exist
    let success_rate = recent_success_rate(&weakest_skill, 5).await
        .unwrap_or(0.5);
    
    // Get last difficulty used for this skill, or default based on skill level
    let skill_value = skills.skills.get(&weakest_skill).copied().unwrap_or(0.5);
    let base_difficulty = (0.3_f32).max(1.0 - skill_value);
    
    // Anneal difficulty
    let annealed_difficulty = anneal_difficulty(base_difficulty, success_rate);
    
    tracing::info!(
        skill = %weakest_skill,
        success_rate = success_rate,
        base_difficulty = base_difficulty,
        annealed_difficulty = annealed_difficulty,
        "Generating new problem with difficulty annealing (all static problems completed)"
    );
    
    // Try to generate a problem with annealed difficulty
    match generator::generate_problem(app_state, &weakest_skill, annealed_difficulty).await {
        Ok(problem) => Ok(problem),
        Err(e) => {
            tracing::warn!(skill = %weakest_skill, error = %e, "Failed to generate problem");
            Err(format!("No problems available and generation failed: {}", e))
        }
    }
}

#[tauri::command]
pub fn get_problems_by_topic(topic: String) -> Result<Vec<Problem>, String> {
    let all_problems = Problem::load_all()
        .map_err(|e| format!("Failed to load problems: {}", e))?;
    
    // Filter by topic (exact match)
    let filtered = selector::get_problems_by_topic(&all_problems, &topic);
    
    // Validate all returned problems have the correct topic
    for problem in &filtered {
        if problem.topic != topic {
            tracing::error!(
                problem_id = %problem.id,
                actual_topic = %problem.topic,
                expected_topic = %topic,
                "Problem has incorrect topic"
            );
        }
    }
    
    Ok(filtered)
}

#[tauri::command]
pub fn get_problem_by_id(problem_id: String) -> Result<Problem, String> {
    tracing::info!(problem_id = %problem_id, "Loading problem by ID (no LLM call)");
    let all_problems = Problem::load_all()
        .map_err(|e| format!("Failed to load problems: {}", e))?;
    
    let problem = all_problems
        .into_iter()
        .find(|p| p.id == problem_id)
        .ok_or_else(|| {
            tracing::warn!(problem_id = %problem_id, "Problem not found by ID");
            format!("Problem with ID '{}' not found", problem_id)
        })?;
    
    tracing::info!(problem_id = %problem.id, topic = %problem.topic, "Successfully loaded problem by ID");
    Ok(problem)
}

#[tauri::command]
pub async fn get_skills(
    state: State<'_, std::sync::Arc<AppState>>,
) -> Result<SkillVector, String> {
    store::get_skills(state.inner()).await
        .map_err(|e| format!("Failed to get skills: {}", e))
}

#[tauri::command]
pub async fn update_skills_from_issues(
    state: State<'_, std::sync::Arc<AppState>>,
    issues: Vec<ProofIssue>,
) -> Result<SkillVector, String> {
    let app_state = state.inner();
    store::update_skills(app_state, |skills| {
        skills.update_from_issues(&issues);
    }).await
        .map_err(|e| format!("Failed to update skills: {}", e))?;
    let skills = store::get_skills(app_state).await
        .map_err(|e| format!("Failed to get skills: {}", e))?;
    if let Err(e) = skills_store::save_skill_vector(&skills).await {
        eprintln!("Failed to save skills: {}", e);
    }
    Ok(skills)
}

#[tauri::command]
pub async fn save_session_record(record: SessionRecord) -> Result<(), String> {
    save_session(&record).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_session_history() -> Result<Vec<SessionRecord>, String> {
    load_all_sessions().await
        .map_err(|e| format!("Failed to load sessions: {}", e))
}

#[tauri::command]
pub async fn get_recent_failures(limit: usize) -> Result<Vec<SessionRecord>, String> {
    let mut all = load_all_sessions().await
        .map_err(|e| format!("Failed to load sessions: {}", e))?;
    all.retain(|s| s.eval_summary.contains("incorrect") || s.eval_summary.contains("fail"));
    all.reverse();
    Ok(all.into_iter().take(limit).collect())
}

#[tauri::command]
pub async fn get_skill_drift(skill: String) -> Result<f32, String> {
    let all = load_all_sessions().await
        .map_err(|e| format!("Failed to load sessions: {}", e))?;
    let mut relevant: Vec<_> = all.into_iter().filter(|s| s.skill == skill).collect();
    if relevant.len() < 2 {
        return Ok(0.0);
    }
    relevant.sort_by_key(|s| s.timestamp);
    let last_skill = relevant.last()
        .map(|s| s.skill_after)
        .unwrap_or(0.0);
    let first_skill = relevant.first()
        .map(|s| s.skill_after)
        .unwrap_or(0.0);
    Ok(last_skill - first_skill)
}

#[tauri::command]
pub async fn fetch_cached_problem() -> Result<Problem, String> {
    let mut cache = crate::problems::cache::ProblemCache::load_async().await;
    cache.queue.pop().ok_or("Cache empty".to_string())
}

#[tauri::command]
pub async fn refresh_daily_plan() -> Result<(), String> {
    let plan = crate::brain::generate_daily_plan().await;
    crate::brain::store::save(&plan).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_daily_plan() -> Result<crate::brain::CurriculumPlan, String> {
    crate::brain::store::load().await
        .map_err(|e| format!("Failed to load plan: {}", e))?
        .ok_or("No plan".into())
}

#[tauri::command]
pub async fn get_analytics_data() -> Result<AnalyticsPayload, String> {
    Ok(compute_analytics().await)
}

#[tauri::command]
pub async fn reset_all_progress(
    state: State<'_, std::sync::Arc<AppState>>,
) -> Result<(), String> {
    use std::fs;
    use crate::skills::store as skills_store;
    use crate::sessions;
    use crate::problems::cache::ProblemCache;
    
    let app_state = state.inner();
    
    // Reset skills to defaults
    let default_skills = crate::skills::model::SkillVector::new();
    skills_store::save_skill_vector(&default_skills).await
        .map_err(|e| format!("Failed to reset skills: {}", e))?;
    
    // Clear in-memory skills store
    crate::memory::store::update_skills(app_state, |skills| {
        *skills = default_skills.clone();
    }).await
        .map_err(|e| format!("Failed to update skills: {}", e))?;
    
    // Delete all session files
    let sessions_dir = sessions::sessions_dir();
    if sessions_dir.exists() {
        if let Ok(entries) = fs::read_dir(&sessions_dir) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
    
    // Delete daily plan
    let plan_path = crate::brain::store::get_plan_path();
    if plan_path.exists() {
        let _ = fs::remove_file(&plan_path);
    }
    
    // Clear problem cache
    let cache = ProblemCache::default();
    let _ = cache.save_async().await;
    
    Ok(())
}
