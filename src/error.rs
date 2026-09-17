//! Error type shared by every command, mapped onto the AXI exit codes.

use std::process::ExitCode;

#[derive(Debug, thiserror::Error)]
pub enum AxiError {
    /// The invocation itself was malformed (unknown flag, missing value).
    #[error("{0}")]
    Usage(String),
    /// The command was well-formed but could not complete.
    #[error("{0}")]
    Failed(String),
}

impl AxiError {
    /// AXI contract: 0 success (including no-ops), 1 error, 2 usage error.
    pub fn exit_code(&self) -> ExitCode {
        match self {
            AxiError::Usage(_) => ExitCode::from(2),
            AxiError::Failed(_) => ExitCode::FAILURE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_errors_exit_with_2_and_failures_with_1() {
        assert_eq!(
            AxiError::Usage("bad flag".into()).exit_code(),
            ExitCode::from(2)
        );
        assert_eq!(
            AxiError::Failed("upstream down".into()).exit_code(),
            ExitCode::FAILURE
        );
    }
}
