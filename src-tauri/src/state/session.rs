use serde::{Deserialize, Serialize};
use crate::pipelines::proof::{Step1Response, Step2Response};
use crate::state::app::AppState;

/// Represents the current state of a proof-solving session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofState {
    /// User has a problem but hasn't submitted a solution yet
    AwaitingSolution,
    /// User submitted a solution, Step 1 analysis is done, waiting for answers to clarifying questions
    AwaitingClarifyingAnswers {
        step1_response: Step1Response,
    },
    /// User answered questions, Step 2 evaluation is done, waiting for revision
    AwaitingRevision {
        step2_response: Step2Response,
    },
}

/// Get the current session state from AppState
pub fn get_state(state: &AppState) -> ProofState {
    state.get_session_state()
}

/// Set the session state in AppState
pub fn set_state(state: &AppState, new_state: ProofState) {
    state.set_session_state(new_state);
}

/// Reset state to initial (when starting a new problem)
pub fn reset_state(state: &AppState) {
    state.reset_session_state();
}

/// Log the current state (for debugging)
pub fn log_state(state: &AppState) {
    let current_state = get_state(state);
    match &current_state {
        ProofState::AwaitingSolution => {
            tracing::debug!("[Coach] State = AwaitingSolution");
        }
        ProofState::AwaitingClarifyingAnswers { .. } => {
            tracing::debug!("[Coach] State = AwaitingClarifyingAnswers");
        }
        ProofState::AwaitingRevision { .. } => {
            tracing::debug!("[Coach] State = AwaitingRevision");
        }
    }
}

