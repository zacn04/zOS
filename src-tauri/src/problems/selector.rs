use crate::problems::problem::Problem;
use crate::skills::model::SkillVector;

pub fn pick_problem(skills: &SkillVector, problems: &Vec<Problem>) -> Option<Problem> {
    if problems.is_empty() {
        return None;
    }

    // Find the weakest skill
    let weakest = match skills.get_weakest_skill() {
        Some((skill_name, _)) => skill_name,
        None => return problems.first().cloned(),
    };

    // Filter problems for the weakest skill and randomly pick from easiest ones
    use rand::seq::SliceRandom;
    use rand::thread_rng;
    
    let matching_problems: Vec<&Problem> = problems
        .iter()
        .filter(|p| p.topic == weakest)
        .collect();
    
    if !matching_problems.is_empty() {
        // Find minimum difficulty
        let min_diff = matching_problems.iter()
            .map(|p| p.difficulty)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
        
        // Filter to easiest problems and randomly pick one
        let easiest: Vec<&Problem> = matching_problems.iter()
            .filter(|p| (p.difficulty - min_diff).abs() < f32::EPSILON)
            .copied()
            .collect();
        
        let mut rng = thread_rng();
        easiest.choose(&mut rng).cloned().cloned()
    } else {
        // If no problems for weakest skill, randomly pick from easiest overall
        let min_diff = problems.iter()
            .map(|p| p.difficulty)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);
        
        let easiest: Vec<&Problem> = problems.iter()
            .filter(|p| (p.difficulty - min_diff).abs() < f32::EPSILON)
            .collect();
        
        let mut rng = thread_rng();
        easiest.choose(&mut rng).cloned().cloned()
    }
}

pub fn get_problems_by_topic(problems: &Vec<Problem>, topic: &str) -> Vec<Problem> {
    // Filter by exact topic match (case-sensitive, no whitespace)
    let filtered: Vec<Problem> = problems
        .iter()
        .filter(|p| {
            // Trim and compare topics exactly
            let p_topic = p.topic.trim();
            let expected_topic = topic.trim();
            p_topic == expected_topic
        })
        .cloned()
        .collect();
    
    // Debug logging
    if filtered.len() != problems.len() {
        tracing::debug!(
            topic = %topic,
            filtered_count = filtered.len(),
            total_count = problems.len(),
            "Filtered problems by topic"
        );
        if filtered.is_empty() && !problems.is_empty() {
            let available_topics: Vec<_> = problems.iter()
                .map(|p| p.topic.as_str())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            tracing::debug!(
                topic = %topic,
                available_topics = ?available_topics,
                "No problems found for topic"
            );
        }
    }
    
    filtered
}

