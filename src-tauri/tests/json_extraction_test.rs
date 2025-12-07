#[cfg(test)]
mod tests {
    use crate::pipelines::ollama_utils::extract_json;

    #[test]
    fn test_extract_json_from_code_block() {
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
    fn test_extract_json_with_trailing_comma() {
        let text = r#"{"key": "value",}"#;
        let json = extract_json(text).unwrap();
        // Should remove trailing comma
        assert!(json.contains(r#""key""#));
        assert!(!json.contains(r#","}"#));
    }

    #[test]
    fn test_extract_json_plain() {
        let text = r#"{"key": "value"}"#;
        let json = extract_json(text).unwrap();
        assert_eq!(json, r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_json_with_extra_text() {
        let text = r#"Some text before {"key": "value"} some text after"#;
        let json = extract_json(text).unwrap();
        assert_eq!(json, r#"{"key": "value"}"#);
    }

    #[test]
    fn test_extract_json_fallback_strategies() {
        // Test with multiple commas
        let text = r#"{"key": "value",,}"#;
        let result = extract_json(text);
        // Should either succeed with fixed JSON or provide helpful error
        assert!(result.is_ok() || result.unwrap_err().to_string().contains("Failed to extract"));
    }
}
