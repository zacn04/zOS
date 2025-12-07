use serde::{Serialize, Deserialize};
use std::fmt;

/// Unified error type for the entire zOS codebase.
/// All functions should return Result<T, ZosError> instead of String errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZosError {
    pub message: String,
    pub stage: String,
    pub model: Option<String>,
    pub retry_succeeded: bool,
    pub context: Option<String>,
    pub source: Option<String>,
}

impl ZosError {
    /// Create a new error with stage and message
    pub fn new<S: Into<String>>(message: S, stage: &'static str) -> Self {
        ZosError {
            message: message.into(),
            stage: stage.to_string(),
            model: None,
            retry_succeeded: false,
            context: None,
            source: None,
        }
    }

    /// Add model context to the error
    pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Mark whether a retry succeeded
    pub fn with_retry(mut self, succeeded: bool) -> Self {
        self.retry_succeeded = succeeded;
        self
    }

    /// Add additional context information
    pub fn with_context<S: Into<String>>(mut self, context: S) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Add source error information
    pub fn with_source<S: Into<String>>(mut self, source: S) -> Self {
        self.source = Some(source.into());
        self
    }
}

impl fmt::Display for ZosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.stage, self.message)?;
        if let Some(ref model) = self.model {
            write!(f, " (model: {})", model)?;
        }
        if let Some(ref context) = self.context {
            write!(f, " (context: {})", context)?;
        }
        if let Some(ref source) = self.source {
            write!(f, " (source: {})", source)?;
        }
        Ok(())
    }
}

impl std::error::Error for ZosError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<anyhow::Error> for ZosError {
    fn from(err: anyhow::Error) -> Self {
        ZosError::new(
            err.to_string(),
            "unknown"
        ).with_source("anyhow")
    }
}

impl From<std::io::Error> for ZosError {
    fn from(err: std::io::Error) -> Self {
        ZosError::new(
            format!("I/O error: {}", err),
            "io"
        ).with_source("std::io")
    }
}

impl From<serde_json::Error> for ZosError {
    fn from(err: serde_json::Error) -> Self {
        ZosError::new(
            format!("JSON error: {}", err),
            "json_parse"
        ).with_source("serde_json")
    }
}

impl From<tokio::time::error::Elapsed> for ZosError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        ZosError::new(
            "Operation timed out",
            "timeout"
        ).with_source("tokio::time")
    }
}

