pub mod store;

use std::collections::HashMap;
use chrono::{Utc, Duration};
use serde::{Serialize, Deserialize};
use crate::skills::store::load_skill_vector;
use crate::sessions::load_all_sessions;

/// One task directive in a daily plan.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TaskDirective {
    Adaptive { skill: String, difficulty: f32 },
    Review { skill: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CurriculumPlan {
    pub tasks: Vec<TaskDirective>,
    pub generated_at: i64,
    pub expires_at: i64,
}

impl CurriculumPlan {
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.expires_at
    }
}

/// Compute 7-day skill trend (Δ skill score).
pub fn compute_weekly_trends() -> HashMap<String, f32> {
    skill_trends(7)
}

/// Compute N-day skill trend (Δ skill score).
fn skill_trends(days: i64) -> HashMap<String, f32> {
    let cutoff = Utc::now() - Duration::days(days);
    let mut hist: HashMap<String, Vec<(i64, f32)>> = HashMap::new();
    
    for s in load_all_sessions().into_iter().filter(|s| s.timestamp > cutoff.timestamp()) {
        hist.entry(s.skill.clone())
            .or_default()
            .push((s.timestamp, s.skill_after));
    }
    
    hist.into_iter()
        .map(|(k, v)| {
            let trend = if v.len() > 1 {
                v.last().map(|last| last.1).unwrap_or(0.0) - 
                v.first().map(|first| first.1).unwrap_or(0.0)
            } else { 
                0.0 
            };
            (k, trend)
        })
        .collect()
}

/// Build the plan: 2 weakest-skill drills + review any negative trend.
pub fn generate_daily_plan() -> CurriculumPlan {
    let skills = load_skill_vector();
    let trends = compute_weekly_trends();

    // Weakest two skills
    let mut weakest: Vec<_> = skills.skills.iter().collect();
    weakest.sort_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut tasks = vec![];
    for (skill, value) in weakest.iter().take(2) {
        tasks.push(TaskDirective::Adaptive {
            skill: (*skill).clone(),
            difficulty: (0.3_f32).max(1.0 - *value),
        });
    }

    // Any negative 7-day trend → review task
    for (skill, trend) in trends {
        if trend < -0.03 {
            tasks.push(TaskDirective::Review { skill });
        }
    }

    CurriculumPlan {
        tasks,
        generated_at: Utc::now().timestamp(),
        expires_at: (Utc::now() + Duration::hours(24)).timestamp(),
    }
}

