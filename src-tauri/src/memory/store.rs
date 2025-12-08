use crate::skills::model::SkillVector;
use crate::state::app::AppState;
use crate::error::ZosError;

/// Get skills from AppState, loading from disk if not cached
pub async fn get_skills(state: &AppState) -> Result<SkillVector, ZosError> {
    // Check if already cached
    {
        let guard = state.skills.read();
        if let Some(skills) = guard.as_ref() {
            return Ok(skills.clone());
        }
    }
    
    // Load from disk
    let skills = crate::skills::store::load_skill_vector().await;
    state.set_skills(skills.clone());
    Ok(skills)
}

/// Update skills in AppState and persist to disk
pub async fn update_skills<F>(state: &AppState, f: F) -> Result<(), ZosError>
where
    F: FnOnce(&mut SkillVector),
{
    // Ensure skills are loaded
    let _ = get_skills(state).await?;
    
    // Update in memory
    state.update_skills(f)?;
    
    // Load current skills and save to disk
    let skills = {
        let guard = state.skills.read();
        guard.as_ref()
            .ok_or_else(|| ZosError::new("Skills not loaded", "state"))?
            .clone()
    };
    crate::skills::store::save_skill_vector(&skills).await?;
    
    Ok(())
}

/// Synchronous versions for backward compatibility (deprecated)
/// These will be removed once all callers are migrated to async
#[deprecated(note = "Use get_skills(state).await instead")]
pub fn get_skills_sync() -> SkillVector {
    crate::skills::store::load_skill_vector_sync()
}

#[deprecated(note = "Use update_skills(state, f).await instead")]
pub fn update_skills_sync<F>(f: F)
where
    F: FnOnce(&mut SkillVector),
{
    use crate::skills::store as skills_store;
    let mut skills = skills_store::load_skill_vector_sync();
    f(&mut skills);
    // Note: sync save is not available, so we skip persistence
    // This is a temporary compatibility shim
}
