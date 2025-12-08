use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkillVector {
    pub skills: HashMap<String, f32>,
}

impl SkillVector {
    pub fn new() -> Self {
        let mut skills = HashMap::new();
        skills.insert("rl_theory".into(), 0.5);
        skills.insert("ml_theory".into(), 0.5);
        skills.insert("ai_research".into(), 0.5);
        skills.insert("coding_debugging".into(), 0.5);
        skills.insert("algorithms".into(), 0.5);
        skills.insert("production_engineering".into(), 0.5);
        skills.insert("analysis_math".into(), 0.5);
        skills.insert("putnam_competition".into(), 0.5);
        skills.insert("proof_strategy".into(), 0.5);
        skills.insert("logical_reasoning".into(), 0.5);
        Self { skills }
    }

    pub fn update_from_issues(&mut self, issues: &Vec<crate::pipelines::proof::ProofIssue>) {
        for issue in issues {
            match issue.issue_type.as_str() {
                "missing_justification" => {
                    if let Some(skill) = self.skills.get_mut("proof_strategy") {
                        *skill = (*skill - 0.02).max(0.0);
                    }
                }
                "incorrect_logic" => {
                    if let Some(skill) = self.skills.get_mut("logical_reasoning") {
                        *skill = (*skill - 0.03).max(0.0);
                    }
                }
                "wrong_definition" => {
                    if let Some(skill) = self.skills.get_mut("analysis_math") {
                        *skill = (*skill - 0.02).max(0.0);
                    }
                }
                "math_gaps" => {
                    if let Some(skill) = self.skills.get_mut("analysis_math") {
                        *skill = (*skill - 0.03).max(0.0);
                    }
                    if let Some(skill) = self.skills.get_mut("putnam_competition") {
                        *skill = (*skill - 0.02).max(0.0);
                    }
                }
                "rl_math_error" => {
                    if let Some(skill) = self.skills.get_mut("rl_theory") {
                        *skill = (*skill - 0.03).max(0.0);
                    }
                }
                "ml_derivation_error" => {
                    if let Some(skill) = self.skills.get_mut("ml_theory") {
                        *skill = (*skill - 0.03).max(0.0);
                    }
                }
                "code_bug" => {
                    if let Some(skill) = self.skills.get_mut("coding_debugging") {
                        *skill = (*skill - 0.03).max(0.0);
                    }
                }
                "faulty_logic" => {
                    if let Some(skill) = self.skills.get_mut("logical_reasoning") {
                        *skill = (*skill - 0.02).max(0.0);
                    }
                }
                "misuse_of_theorem" => {
                    if let Some(skill) = self.skills.get_mut("proof_strategy") {
                        *skill = (*skill - 0.02).max(0.0);
                    }
                }
                "undefined_term" => {
                    if let Some(skill) = self.skills.get_mut("analysis_math") {
                        *skill = (*skill - 0.02).max(0.0);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn update_from_evaluation(&mut self, evaluation: &Vec<crate::pipelines::proof::QuestionEvaluation>) {
        for eval in evaluation {
            match eval.assessment.as_str() {
                "correct" => {
                    // Small positive XP for correct answers
                    if let Some(skill) = self.skills.get_mut("logical_reasoning") {
                        *skill = (*skill + 0.01).min(1.0);
                    }
                }
                "partially_correct" => {
                    if let Some(skill) = self.skills.get_mut("proof_strategy") {
                        *skill = (*skill + 0.005).min(1.0);
                    }
                }
                _ => {}
            }
        }
    }

    /// Reward skills for a perfect proof (no issues, no questions needed)
    pub fn update_for_perfect_proof(&mut self, skill_topic: &str) {
        // Reward the specific skill domain for a perfect proof
        if let Some(skill) = self.skills.get_mut(skill_topic) {
            *skill = (*skill + 0.02).min(1.0);
        }
        // Also reward proof strategy and logical reasoning as secondary skills
        if let Some(skill) = self.skills.get_mut("proof_strategy") {
            *skill = (*skill + 0.01).min(1.0);
        }
        if let Some(skill) = self.skills.get_mut("logical_reasoning") {
            *skill = (*skill + 0.01).min(1.0);
        }
    }

    pub fn get_weakest_skill(&self) -> Option<(String, f32)> {
        use rand::seq::SliceRandom;
        use rand::thread_rng;
        
        if self.skills.is_empty() {
            return None;
        }
        
        // Find the minimum skill value
        let min_value = self.skills.values()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .copied()?;
        
        // Collect all skills with the minimum value
        let tied_skills: Vec<(String, f32)> = self.skills.iter()
            .filter(|(_, &v)| (v - min_value).abs() < f32::EPSILON)
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        
        // Randomly pick one from the tied skills
        let mut rng = thread_rng();
        tied_skills.choose(&mut rng).cloned()
    }

    pub fn weakest_n(&self, n: usize) -> Vec<(String, f32)> {
        use rand::seq::SliceRandom;
        use rand::thread_rng;
        
        let mut skills_vec: Vec<(String, f32)> = self.skills.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        
        // Sort by skill value
        skills_vec.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        if skills_vec.is_empty() {
            return vec![];
        }
        
        // Group by skill value and randomly pick from ties
        let mut result = Vec::new();
        let mut i = 0;
        let mut rng = thread_rng();
        
        while result.len() < n && i < skills_vec.len() {
            let current_value = skills_vec[i].1;
            // Find all skills with the same value
            let mut tied_group = Vec::new();
            while i < skills_vec.len() && (skills_vec[i].1 - current_value).abs() < f32::EPSILON {
                tied_group.push(skills_vec[i].clone());
                i += 1;
            }
            
            // Randomly shuffle tied group and add to result
            tied_group.shuffle(&mut rng);
            for skill in tied_group {
                if result.len() >= n {
                    break;
                }
                result.push(skill);
            }
        }
        
        result
    }
}

impl Default for SkillVector {
    fn default() -> Self {
        Self::new()
    }
}
