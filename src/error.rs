//! Error type shared by every command: a structured AXI error document plus
//! the exit code it maps to.

use crate::output::Document;
use std::process::ExitCode;

/// Stable error codes. Agents branch on these, so they never change wording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Usage,
    AuthRequired,
    AuthInvalid,
    NotFound,
    Forbidden,
    Conflict,
    ApiError,
    Network,
    Internal,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::Usage => "usage",
            ErrorCode::AuthRequired => "auth_required",
            ErrorCode::AuthInvalid => "auth_invalid",
            ErrorCode::NotFound => "not_found",
            ErrorCode::Forbidden => "forbidden",
            ErrorCode::Conflict => "conflict",
            ErrorCode::ApiError => "api_error",
            ErrorCode::Network => "network",
            ErrorCode::Internal => "internal",
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("{message}")]
pub struct AxiError {
    pub code: ErrorCode,
    pub message: String,
    pub help: Vec<String>,
    pub flags: Vec<String>,
    pub request_id: Option<String>,
    /// Printed instead of the plain error document when a command has partial
    /// output worth keeping (the home view still prints the command index
    /// when its one live call fails).
    pub carried: Option<Box<Document>>,
}

impl AxiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        AxiError {
            code,
            message: message.into(),
            help: Vec::new(),
            flags: Vec::new(),
            request_id: None,
            carried: None,
        }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        AxiError::new(ErrorCode::Usage, message)
    }

    pub fn auth_required(message: impl Into<String>) -> Self {
        AxiError::new(ErrorCode::AuthRequired, message).with_help([
            "Run `chictrip-axi auth set --from-json -` and paste the JSON copied from the browser (see README)",
            "Run `chictrip-axi auth status` to check what is configured",
        ])
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        AxiError::new(ErrorCode::NotFound, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        AxiError::new(ErrorCode::Internal, message)
    }

    pub fn with_help<I, S>(mut self, lines: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.help = lines.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_flags(mut self, flags: Vec<String>) -> Self {
        self.flags = flags;
        self
    }

    pub fn with_request_id(mut self, request_id: Option<String>) -> Self {
        self.request_id = request_id;
        self
    }

    pub fn with_document(mut self, document: Document) -> Self {
        self.carried = Some(Box::new(document));
        self
    }

    /// AXI contract: 0 success (including no-ops), 1 error, 2 usage error.
    pub fn exit_code(&self) -> ExitCode {
        match self.code {
            ErrorCode::Usage => ExitCode::from(2),
            _ => ExitCode::FAILURE,
        }
    }

    pub fn document(&self) -> Document {
        if let Some(carried) = &self.carried {
            return (**carried).clone();
        }
        let mut doc = Document::new();
        doc.set("error", self.code.as_str());
        doc.set("message", self.message.as_str());
        if let Some(id) = &self.request_id {
            doc.set("request_id", id.as_str());
        }
        if !self.flags.is_empty() {
            doc.set_strings("flags", &self.flags);
        }
        if !self.help.is_empty() {
            doc.set_strings("help", &self.help);
        }
        doc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_errors_exit_with_2_and_failures_with_1() {
        assert_eq!(AxiError::usage("bad flag").exit_code(), ExitCode::from(2));
        assert_eq!(
            AxiError::new(ErrorCode::Network, "upstream down").exit_code(),
            ExitCode::FAILURE
        );
    }

    #[test]
    fn error_document_leads_with_code_and_message() {
        let err = AxiError::usage("unknown flag --bogus")
            .with_flags(vec!["--limit".into(), "--json".into()]);
        let rendered = crate::output::render(&err.document(), false);
        assert!(rendered.starts_with("error: usage\n"));
        assert!(rendered.contains("unknown flag --bogus"));
        assert!(rendered.contains("flags[2]:"));
        assert!(rendered.contains("--limit"));
    }
}
