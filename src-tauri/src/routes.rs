use crate::pipelines::proof::{
    call_deepseek_step1, call_deepseek_step2, ProofIssue, Step1Response, Step2Response,
};
use crate::problems::{problem::Problem, selector, generator};
use crate::skills::{model::SkillVector, store as skills_store};
use crate::memory::store;
use crate::sessions::{SessionRecord, save_session, load_all_sessions, recent_success_rate};
use crate::brain::TaskDirective;
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
    
    // Get problem statement if we have a problem_id
    let problem_statement = if let Some(pid) = &problem_id {
        match get_problem_by_id(pid.clone()) {
            Ok(problem) => Some(problem.statement),
            Err(_) => None,
        }
    } else {
        None
    };
    
    match call_deepseek_step1(app_state, &proof, problem_statement.as_deref()).await {
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

    // Get problem statement for context (if problem_id is available)
    let problem_statement = if let Some(pid) = &problem_id {
        Problem::load_all()
            .ok()
            .and_then(|problems| problems.into_iter().find(|p| p.id == *pid))
            .map(|p| p.statement)
            .unwrap_or_else(|| "Problem statement not available".to_string())
    } else {
        "Problem statement not available".to_string()
    };

    // Get skill before update
    let skills_before = store::get_skills(app_state).await
        .map_err(|e| format!("Failed to get skills: {}", e))?;
    let skill_before = problem_topic.as_ref()
        .and_then(|topic| skills_before.skills.get(topic))
        .copied()
        .unwrap_or(0.5);

    match call_deepseek_step2(app_state, &problem_statement, &proof, &issues_json, &questions_json, &answers_json).await {
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

/// Internal helper function to select a problem (extracted for reuse)
async fn select_problem_internal(
    app_state: &AppState,
) -> Result<Problem, String> {
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
        sessions.iter().map(|s| s.problem_id.clone()).collect()
    };
    
    // Get recently used problem IDs (last 3 problems from sessions) to avoid immediate repeats
    let session_recently_used: std::collections::HashSet<String> = {
        let mut sessions = load_all_sessions().await.unwrap_or_default();
        // Sort by timestamp descending (most recent first)
        sessions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        // Take last 3 problems
        sessions.into_iter()
            .take(3)
            .map(|s| s.problem_id)
            .collect()
    };
    
    // Get in-memory recently selected problems (to avoid immediate repeats even without sessions)
    let in_memory_recently_selected: std::collections::HashSet<String> = {
        app_state.get_recently_selected_problems().into_iter().collect()
    };
    
    // Combine both sources for comprehensive recently used tracking
    let mut recently_used_problem_ids = session_recently_used;
    recently_used_problem_ids.extend(in_memory_recently_selected);
    
    // FIRST: Try to get a cached problem (fast, no LLM call) - exclude completed and recently used ones
    let mut cached = crate::problems::cache::ProblemCache::load_async().await;
    if let Some(pos) = cached.queue.iter()
        .position(|p| p.topic == weakest_skill 
            && !completed_problem_ids.contains(&p.id)
            && !recently_used_problem_ids.contains(&p.id)) {
        let problem = cached.queue.remove(pos);
        // Save updated cache
        let _ = cached.save_async().await;
        tracing::info!(skill = %weakest_skill, problem_id = %problem.id, "Using cached problem (not completed, not recently used)");
        app_state.record_problem_selected(problem.id.clone());
        return Ok(problem);
    }
    
    // SECOND: Try static problems (exclude completed and recently used ones) - prefer uncompleted and not recently used
    let available_problems: Vec<&Problem> = problems.iter()
        .filter(|p| !completed_problem_ids.contains(&p.id)
            && !recently_used_problem_ids.contains(&p.id))
        .collect();
    
    if let Some(static_problem) = selector::pick_problem_from_list(&skills, &available_problems) {
        tracing::info!(skill = %weakest_skill, problem_id = %static_problem.id, "Using static problem (not completed, not recently used)");
        app_state.record_problem_selected(static_problem.id.clone());
        return Ok(static_problem.clone());
    }
    
    // If all uncompleted problems are recently used, try to pick from other skills for variety
    // This adds variety when the same skill keeps getting selected
    let available_other_skill_problems: Vec<&Problem> = problems.iter()
        .filter(|p| !completed_problem_ids.contains(&p.id)
            && !recently_used_problem_ids.contains(&p.id)
            && p.topic != weakest_skill)
        .collect();
    
    if !available_other_skill_problems.is_empty() {
        use rand::seq::SliceRandom;
        use rand::thread_rng;
        let mut rng = thread_rng();
        if let Some(problem) = available_other_skill_problems.choose(&mut rng) {
            tracing::info!(skill = %weakest_skill, selected_skill = %problem.topic, problem_id = %problem.id, "Using problem from different skill for variety");
            app_state.record_problem_selected(problem.id.clone());
            return Ok((*problem).clone());
        }
    }
    
    // If all problems are completed, allow repeats but still avoid recently used
    let repeatable_problems: Vec<&Problem> = problems.iter()
        .filter(|p| !recently_used_problem_ids.contains(&p.id))
        .collect();
    
    if let Some(static_problem) = selector::pick_problem_from_list(&skills, &repeatable_problems) {
        tracing::info!(skill = %weakest_skill, problem_id = %static_problem.id, "Using static problem (all completed, avoiding recently used)");
        app_state.record_problem_selected(static_problem.id.clone());
        return Ok(static_problem.clone());
    }
    
    // Final fallback: allow any problem (including recently used) if nothing else available
    if let Some(static_problem) = selector::pick_problem(&skills, &problems) {
        tracing::info!(skill = %weakest_skill, problem_id = %static_problem.id, "Using static problem (final fallback, may be recently used)");
        app_state.record_problem_selected(static_problem.id.clone());
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
                        Ok(problem) => {
                            app_state.record_problem_selected(problem.id.clone());
                            return Ok(problem);
                        },
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
                                app_state.record_problem_selected(problem.id.clone());
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
        Ok(problem) => {
            app_state.record_problem_selected(problem.id.clone());
            Ok(problem)
        },
        Err(e) => {
            tracing::warn!(skill = %weakest_skill, error = %e, "Failed to generate problem");
            Err(format!("No problems available and generation failed: {}", e))
        }
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
    // First, check if we have a precomputed problem ready
    // Try to get one matching the expected difficulty (if we can determine it)
    let skills = store::get_skills(app_state).await
        .map_err(|e| format!("Failed to get skills: {}", e))?;
    let expected_difficulty = skills.get_weakest_skill()
        .and_then(|(skill, _)| skills.skills.get(&skill).copied())
        .map(|skill_val| (0.3_f32).max(1.0 - skill_val));
    
    if let Some(precomputed) = app_state.take_precomputed_problem(expected_difficulty) {
        tracing::info!(problem_id = %precomputed.id, difficulty = precomputed.difficulty, "Using precomputed problem");
        app_state.record_problem_selected(precomputed.id.clone());
        
        // Trigger precomputation of next problems in background (don't await)
        let app_state_clone = app_state.clone();
        tokio::spawn(async move {
            if let Err(e) = precompute_next_problems_internal(&app_state_clone, precomputed.difficulty).await {
                tracing::warn!(error = %e, "Failed to precompute next problems");
            }
        });
        
        return Ok(precomputed);
    }
    
    // No precomputed problem available, compute it now
    let problem = select_problem_internal(app_state).await?;
    let problem_difficulty = problem.difficulty;
    
    // Trigger precomputation of next problems in background (don't await)
    let app_state_clone = app_state.clone();
    tokio::spawn(async move {
        if let Err(e) = precompute_next_problems_internal(&app_state_clone, problem_difficulty).await {
            tracing::warn!(error = %e, "Failed to precompute next problems");
        }
    });
    
    Ok(problem)
}

/// Internal function to precompute the next problems (easier, same, harder) in parallel
async fn precompute_next_problems_internal(
    app_state: &AppState,
    base_difficulty: f32,
) -> Result<(), String> {
    let skills = store::get_skills(app_state).await
        .map_err(|e| format!("Failed to get skills: {}", e))?;
    let weakest_skill = match skills.get_weakest_skill() {
        Some((skill_name, _)) => skill_name,
        None => {
            // If no skill found, try to generate for first available skill
            if let Some((skill, _)) = skills.skills.iter().next() {
                skill.clone()
            } else {
                return Err("No skills available for precomputation".to_string());
            }
        }
    };
    
    // Calculate difficulty variants
    let easier_diff = (base_difficulty - 0.2).max(0.1);
    let harder_diff = (base_difficulty + 0.2).min(1.0);
    
    // Spawn 3 parallel tasks to generate problems with different difficulties
    // Each will use different models (via router fallback) to avoid bottlenecking
    let app_state_clone1 = app_state.clone();
    let app_state_clone2 = app_state.clone();
    let app_state_clone3 = app_state.clone();
    let skill_clone1 = weakest_skill.clone();
    let skill_clone2 = weakest_skill.clone();
    let skill_clone3 = weakest_skill.clone();
    
    let handle1 = tokio::spawn(async move {
        generator::generate_problem(&app_state_clone1, &skill_clone1, easier_diff).await
    });
    
    let handle2 = tokio::spawn(async move {
        generator::generate_problem(&app_state_clone2, &skill_clone2, base_difficulty).await
    });
    
    let handle3 = tokio::spawn(async move {
        generator::generate_problem(&app_state_clone3, &skill_clone3, harder_diff).await
    });
    
    // Wait for all to complete and collect results
    let (result1, result2, result3) = tokio::join!(handle1, handle2, handle3);
    
    let mut success_count = 0;
    
    // Add successful generations to precomputed problems
    match result1 {
        Ok(Ok(problem)) => {
            app_state.add_precomputed_problem(problem);
            tracing::info!(difficulty = easier_diff, "Precomputed easier problem");
            success_count += 1;
        },
        Ok(Err(e)) => tracing::warn!(error = %e, "Failed to generate easier problem"),
        Err(e) => tracing::warn!(error = %e, "Task panicked for easier problem"),
    }
    
    match result2 {
        Ok(Ok(problem)) => {
            app_state.add_precomputed_problem(problem);
            tracing::info!(difficulty = base_difficulty, "Precomputed same difficulty problem");
            success_count += 1;
        },
        Ok(Err(e)) => tracing::warn!(error = %e, "Failed to generate same difficulty problem"),
        Err(e) => tracing::warn!(error = %e, "Task panicked for same difficulty problem"),
    }
    
    match result3 {
        Ok(Ok(problem)) => {
            app_state.add_precomputed_problem(problem);
            tracing::info!(difficulty = harder_diff, "Precomputed harder problem");
            success_count += 1;
        },
        Ok(Err(e)) => tracing::warn!(error = %e, "Failed to generate harder problem"),
        Err(e) => tracing::warn!(error = %e, "Task panicked for harder problem"),
    }
    
    // Return success if at least one succeeded
    if success_count > 0 {
        Ok(())
    } else {
        Err("All precomputation attempts failed".to_string())
    }
}

/// Command to manually trigger precomputation (called from frontend when problem is loaded)
#[tauri::command]
pub async fn precompute_next_problem(
    state: State<'_, std::sync::Arc<AppState>>,
) -> Result<(), String> {
    let app_state = state.inner();
    // Get current problem difficulty if available, otherwise use default
    let skills = store::get_skills(app_state).await
        .map_err(|e| format!("Failed to get skills: {}", e))?;
    let base_difficulty = skills.get_weakest_skill()
        .and_then(|(skill, _)| skills.skills.get(&skill).copied())
        .map(|skill_val| (0.3_f32).max(1.0 - skill_val))
        .unwrap_or(0.5);

    precompute_next_problems_internal(app_state, base_difficulty).await
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

/// Submit/abandon a problem attempt (for tracking when user moves on without completing)
#[tauri::command]
pub async fn submit_problem_attempt(
    state: State<'_, std::sync::Arc<AppState>>,
    problem_id: Option<String>,
    problem_topic: Option<String>,
    problem_difficulty: Option<f32>,
    user_attempt: String,
    status: String, // "abandoned", "incomplete", "perfect", etc.
) -> Result<(), String> {
    let app_state = state.inner();
    
    // Only save if we have problem info
    if let (Some(pid), Some(topic)) = (problem_id, problem_topic) {
        let skills = store::get_skills(app_state).await
            .map_err(|e| format!("Failed to get skills: {}", e))?;
        let skill_before = skills.skills.get(&topic).copied().unwrap_or(0.5);
        let skill_after = skill_before; // No change if abandoned/incomplete
        
        let record = SessionRecord {
            session_id: format!("sess_{}", Utc::now().timestamp_millis()),
            problem_id: pid,
            skill: topic,
            user_attempt,
            issues: vec![],
            eval_summary: format!("Attempt {} - user moved on", status),
            skill_before,
            skill_after,
            difficulty: problem_difficulty.unwrap_or(0.5),
            timestamp: Utc::now().timestamp(),
        };
        
        if let Err(e) = save_session(&record).await {
            tracing::warn!(error = %e, "Failed to save problem attempt record");
        }
    }
    
    Ok(())
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
