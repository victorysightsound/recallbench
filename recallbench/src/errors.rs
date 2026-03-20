use std::fmt;

/// Error categories for RecallBench operations.
#[derive(Debug)]
pub enum RecallBenchError {
    /// Dataset loading, parsing, or validation errors.
    Dataset(String),
    /// Memory system adapter errors.
    System(String),
    /// LLM provider errors (API or CLI).
    Llm(String),
    /// Judge evaluation errors.
    Judge(String),
    /// File I/O errors.
    Io(std::io::Error),
    /// Configuration parsing errors.
    Config(String),
}

impl fmt::Display for RecallBenchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dataset(msg) => write!(f, "dataset error: {msg}"),
            Self::System(msg) => write!(f, "system error: {msg}"),
            Self::Llm(msg) => write!(f, "llm error: {msg}"),
            Self::Judge(msg) => write!(f, "judge error: {msg}"),
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Config(msg) => write!(f, "config error: {msg}"),
        }
    }
}

impl std::error::Error for RecallBenchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for RecallBenchError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_dataset_error() {
        let err = RecallBenchError::Dataset("missing field".to_string());
        assert_eq!(err.to_string(), "dataset error: missing field");
    }

    #[test]
    fn display_system_error() {
        let err = RecallBenchError::System("connection refused".to_string());
        assert_eq!(err.to_string(), "system error: connection refused");
    }

    #[test]
    fn display_llm_error() {
        let err = RecallBenchError::Llm("rate limited".to_string());
        assert_eq!(err.to_string(), "llm error: rate limited");
    }

    #[test]
    fn display_judge_error() {
        let err = RecallBenchError::Judge("ambiguous response".to_string());
        assert_eq!(err.to_string(), "judge error: ambiguous response");
    }

    #[test]
    fn display_config_error() {
        let err = RecallBenchError::Config("invalid toml".to_string());
        assert_eq!(err.to_string(), "config error: invalid toml");
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = RecallBenchError::from(io_err);
        assert!(err.to_string().contains("file missing"));
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RecallBenchError>();
    }
}
