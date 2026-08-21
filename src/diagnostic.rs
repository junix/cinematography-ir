use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub path: String,
    pub message: String,
    pub hint: Option<String>,
}

impl Diagnostic {
    pub fn error(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            path: path.into(),
            message: message.into(),
            hint: None,
        }
    }

    pub fn warning(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Warning,
            path: path.into(),
            message: message.into(),
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn extend(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|item| item.severity == Severity::Error)
    }

    pub fn has_warnings(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|item| item.severity == Severity::Warning)
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|item| item.severity == Severity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|item| item.severity == Severity::Warning)
            .count()
    }
}
