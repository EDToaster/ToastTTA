//! Diagnostic types for the assembler.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub file: std::sync::Arc<str>,
    pub line: u32,   // 1-indexed
    pub col:  u32,   // 1-indexed
    pub len:  u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diag {
    pub severity: Severity,
    pub message:  String,
    pub span:     Span,
}

#[derive(Default, Clone, Debug)]
pub struct Diagnostics {
    pub items: Vec<Diag>,
}

impl Diagnostics {
    pub fn new() -> Self { Self::default() }

    pub fn push(&mut self, d: Diag) { self.items.push(d); }

    pub fn error(&mut self, span: Span, msg: impl Into<String>) {
        self.push(Diag { severity: Severity::Error, message: msg.into(), span });
    }

    pub fn warn(&mut self, span: Span, msg: impl Into<String>) {
        self.push(Diag { severity: Severity::Warning, message: msg.into(), span });
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn span() -> Span {
        Span { file: Arc::from("test.tasm"), line: 1, col: 1, len: 1 }
    }

    #[test]
    fn collect_and_query() {
        let mut d = Diagnostics::new();
        assert!(!d.has_errors());
        d.warn(span(), "watch out");
        assert!(!d.has_errors());
        d.error(span(), "oh no");
        assert!(d.has_errors());
        assert_eq!(d.items.len(), 2);
    }
}
