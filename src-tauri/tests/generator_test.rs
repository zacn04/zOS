use zos_lib::problems::{generator, problem::Problem};
use std::fs;
use std::path::Path;

#[tokio::test]
async fn test_generate_problem() {
    // Generate a problem
    let result = generator::generate_problem("rl_theory", 0.3).await;
    
    assert!(result.is_ok(), "generate_problem should succeed");
    
    let problem = result.unwrap();
    
    // Verify problem structure
    assert_eq!(problem.topic, "rl_theory", "Topic should match");
    assert_eq!(problem.difficulty, 0.3, "Difficulty should match");
    assert!(!problem.statement.is_empty(), "Statement should not be empty");
    assert!(!problem.solution_sketch.is_empty(), "Solution sketch should not be empty");
    assert!(problem.id.starts_with("autogen_"), "ID should start with autogen_");
    
    // Verify file was created
    let autogen_paths = vec![
        Path::new("problems/autogen"),
        Path::new("../problems/autogen"),
        Path::new("./problems/autogen"),
    ];
    
    let mut file_exists = false;
    for autogen_dir in autogen_paths {
        if autogen_dir.exists() {
            if let Ok(entries) = fs::read_dir(autogen_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(saved_problem) = serde_json::from_str::<Problem>(&content) {
                                if saved_problem.id == problem.id {
                                    file_exists = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    assert!(file_exists, "Generated problem file should exist in autogen directory");
}

