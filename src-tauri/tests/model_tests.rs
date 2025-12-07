use zos_lib::config::models::get_model_config;
use zos_lib::pipelines::router::{TaskType, model_for_task, get_model_for_task, get_routing_metrics};
use zos_lib::models::registry::{get_model, get_available_models};

#[test]
fn test_model_config_loads() {
    let config = get_model_config();
    assert!(!config.proof_model.is_empty());
    assert!(!config.problem_model.is_empty());
    assert!(!config.general_model.is_empty());
}

#[test]
fn test_routing_returns_decision() {
    let decision = model_for_task(TaskType::ProofAnalysis);
    assert!(!decision.selected.is_empty());
    assert_eq!(decision.task, TaskType::ProofAnalysis);
}

#[test]
fn test_get_model_for_task() {
    // This will return None if models aren't registered, which is fine for testing
    let _model = get_model_for_task(TaskType::ProofAnalysis);
    // Just verify it doesn't panic
}

#[test]
fn test_get_available_models() {
    let models = get_available_models();
    // Should have at least the default models registered
    assert!(!models.is_empty());
}

#[test]
fn test_routing_metrics_exist() {
    let metrics = get_routing_metrics();
    // Just verify we can access metrics without panicking
    assert!(metrics.success_count >= 0);
    assert!(metrics.failure_count >= 0);
}

#[test]
fn test_all_task_types() {
    let _proof = model_for_task(TaskType::ProofAnalysis);
    let _problem = model_for_task(TaskType::ProblemGeneration);
    let _general = model_for_task(TaskType::General);
    // Just verify all task types work
}

