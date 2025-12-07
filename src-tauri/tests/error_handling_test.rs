#[cfg(test)]
mod tests {
    use crate::error::ZosError;

    #[test]
    fn test_error_creation() {
        let error = ZosError::new("Test error", "test_stage");
        assert_eq!(error.message, "Test error");
        assert_eq!(error.stage, "test_stage");
    }

    #[test]
    fn test_error_with_context() {
        let error = ZosError::new("Test error", "test_stage")
            .with_context("Additional context");
        assert!(error.context.is_some());
        assert_eq!(error.context.unwrap(), "Additional context");
    }

    #[test]
    fn test_error_with_model() {
        let error = ZosError::new("Test error", "test_stage")
            .with_model("test-model");
        assert!(error.model.is_some());
        assert_eq!(error.model.unwrap(), "test-model");
    }

    #[test]
    fn test_error_display() {
        let error = ZosError::new("Test error", "test_stage")
            .with_context("context")
            .with_model("model");
        let display = format!("{}", error);
        assert!(display.contains("test_stage"));
        assert!(display.contains("Test error"));
    }
}
