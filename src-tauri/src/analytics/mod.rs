use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::sessions::load_all_sessions;
use crate::brain::compute_weekly_trends;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnalyticsPayload {
    pub skill_history: HashMap<String, Vec<(i64, f32)>>, // timestamp → skill value
    pub session_counts: HashMap<String, usize>,
    pub avg_difficulty: HashMap<String, f32>,
    pub weekly_trends: HashMap<String, f32>,
}

pub fn compute_analytics() -> AnalyticsPayload {
    let all_sessions = load_all_sessions();
    
    let mut skill_history: HashMap<String, Vec<(i64, f32)>> = HashMap::new();
    let mut session_counts: HashMap<String, usize> = HashMap::new();
    let mut difficulty_sums: HashMap<String, f32> = HashMap::new();
    let mut difficulty_counts: HashMap<String, usize> = HashMap::new();
    
    // Process all sessions
    for session in &all_sessions {
        // Increment session count
        *session_counts.entry(session.skill.clone()).or_insert(0) += 1;
        
        // Accumulate difficulty for average
        *difficulty_sums.entry(session.skill.clone()).or_insert(0.0) += session.difficulty;
        *difficulty_counts.entry(session.skill.clone()).or_insert(0) += 1;
        
        // Add to skill history (use skill_after as the value at this timestamp)
        skill_history
            .entry(session.skill.clone())
            .or_default()
            .push((session.timestamp, session.skill_after));
    }
    
    // Sort skill history by timestamp for each skill
    for history in skill_history.values_mut() {
        history.sort_by_key(|(ts, _)| *ts);
    }
    
    // Compute average difficulties
    let avg_difficulty: HashMap<String, f32> = difficulty_sums
        .into_iter()
        .map(|(skill, sum)| {
            let count = difficulty_counts.get(&skill).copied().unwrap_or(1);
            (skill, if count > 0 { sum / count as f32 } else { 0.0 })
        })
        .collect();
    
    // Get weekly trends
    let weekly_trends = compute_weekly_trends();
    
    AnalyticsPayload {
        skill_history,
        session_counts,
        avg_difficulty,
        weekly_trends,
    }
}

