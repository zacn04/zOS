# Testing Strategy for zOS

## Overview

zOS aims for 60%+ test coverage with comprehensive unit, integration, and fuzz tests.

## Test Structure

```
src-tauri/
├── src/
│   └── ...
└── tests/
    ├── unit/
    │   ├── error_handling.rs
    │   ├── state_management.rs
    │   └── json_parsing.rs
    ├── integration/
    │   ├── routing.rs
    │   ├── fallback.rs
    │   └── error_paths.rs
    └── fuzz/
        └── json_extraction.rs
```

## Unit Tests

### Error Handling Tests

Test all error paths:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_io_error_handling() {
        let result = tokio::fs::read_to_string("/nonexistent/file.txt").await;
        assert!(result.is_err());
        // Verify error is ZosError with proper context
    }
    
    #[test]
    fn test_error_context_chaining() {
        let error = ZosError::new("Base error", "io")
            .with_context("Additional context");
        assert!(error.context.is_some());
    }
}
```

### State Management Tests

Test AppState operations:

```rust
#[tokio::test]
async fn test_app_state_skills() {
    let state = AppState::new();
    
    // Test loading skills
    let skills = state.get_skills().await.unwrap();
    assert!(skills.skills.is_empty());
    
    // Test updating skills
    state.update_skills(|s| {
        s.skills.insert("test".to_string(), 0.5);
    }).await.unwrap();
}
```

### JSON Parsing Tests

Test JSON extraction and parsing:

```rust
#[test]
fn test_json_extraction() {
    let text = r#"
    Here's some text.
    ```json
    {"key": "value"}
    ```
    More text.
    "#;
    
    let json = extract_json(text).unwrap();
    assert_eq!(json, r#"{"key": "value"}"#);
}

#[test]
fn test_json_extraction_with_trailing_comma() {
    let text = r#"{"key": "value",}"#;
    let json = extract_json(text).unwrap();
    assert_eq!(json, r#"{"key": "value"}"#);
}
```

## Integration Tests

### Routing Tests

Test model routing with fallback:

```rust
#[tokio::test]
async fn test_routing_with_fallback() {
    let state = Arc::new(AppState::new());
    
    // Test primary model routing
    let result = zos_query::<Response>(
        &state,
        TaskType::ProofAnalysis,
        "test prompt".to_string(),
    ).await;
    
    // Verify routing decision
    // Verify fallback if primary fails
}
```

### Fallback Logic Tests

Test graceful degradation:

```rust
#[tokio::test]
async fn test_fallback_on_primary_failure() {
    // Mock primary model to fail
    // Verify fallback is attempted
    // Verify error if fallback also fails
}
```

### Error Path Tests

Test all error scenarios:

```rust
#[tokio::test]
async fn test_error_propagation() {
    // Test that errors are properly propagated
    // Test that context is preserved
    // Test that errors are logged
}
```

## Fuzz Tests

### JSON Extraction Fuzzing

Fuzz the JSON extraction logic:

```rust
#[cfg(fuzzing)]
mod fuzz {
    use libfuzzer_sys::fuzz_target;
    
    fuzz_target!(|data: &[u8]| {
        if let Ok(text) = std::str::from_utf8(data) {
            let _ = extract_json(text);
            // Should never panic
        }
    });
}
```

## Mocking

### Model Trait

Create a mockable trait for models:

```rust
#[async_trait]
pub trait ModelTrait {
    async fn call_json<T: DeserializeOwned>(&self, prompt: &str) -> Result<T, ZosError>;
}

// Real implementation
impl ModelTrait for LocalModel { ... }

// Mock implementation
pub struct MockModel {
    responses: Vec<String>,
}

impl ModelTrait for MockModel {
    async fn call_json<T: DeserializeOwned>(&self, prompt: &str) -> Result<T, ZosError> {
        // Return mocked response
    }
}
```

## Test Utilities

### Test Helpers

Create reusable test utilities:

```rust
pub mod test_utils {
    use crate::state::app::AppState;
    
    pub fn create_test_state() -> AppState {
        AppState::new()
    }
    
    pub fn create_mock_model() -> MockModel {
        MockModel::new()
    }
}
```

## Coverage Goals

### Target Coverage

- **Overall**: 60%+ line coverage
- **Error paths**: 80%+ coverage
- **Critical paths**: 90%+ coverage
- **Edge cases**: 70%+ coverage

### Coverage Tools

Use `cargo-tarpaulin` for coverage:

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## Continuous Integration

### CI Pipeline

```yaml
test:
  - cargo test --all-features
  - cargo tarpaulin --out Xml
  - cargo clippy -- -D warnings
  - cargo fmt --check
```

## Best Practices

1. **Test all error paths** - Every `Result` should have error tests
2. **Test concurrent access** - Verify no race conditions
3. **Test edge cases** - Empty inputs, large inputs, invalid data
4. **Use property-based testing** - For complex logic
5. **Mock external dependencies** - Don't call real APIs in tests
6. **Test async code properly** - Use `#[tokio::test]` for async tests

## Example Test Suite

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_happy_path() {
        // Test normal operation
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        // Test error scenarios
    }
    
    #[tokio::test]
    async fn test_concurrent_access() {
        // Test thread safety
    }
    
    #[test]
    fn test_edge_cases() {
        // Test boundary conditions
    }
}
```

## Future Improvements

1. Property-based testing with `proptest`
2. Mutation testing with `cargo-mutants`
3. Performance benchmarks
4. Load testing for concurrent scenarios
