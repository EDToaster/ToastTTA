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

    /// Render diagnostics with snippet and caret pointing at the span.
    pub fn render(&self, source: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let mut out = String::new();
        for diag in &self.items {
            let sev = match diag.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            out.push_str(&format!("{}: {}\n", sev, diag.message));
            out.push_str(&format!(
                "  --> {}:{}:{}\n",
                diag.span.file, diag.span.line, diag.span.col
            ));
            let line_idx = diag.span.line.saturating_sub(1) as usize;
            if let Some(line) = lines.get(line_idx) {
                let line_num = diag.span.line.to_string();
                out.push_str(&format!("{} | {}\n", line_num, line));
                let prefix_width = line_num.len() + 3; // "<n> | "
                let col = diag.span.col.saturating_sub(1) as usize;
                let len = diag.span.len.max(1) as usize;
                let mut caret_line = String::new();
                for _ in 0..(prefix_width + col) {
                    caret_line.push(' ');
                }
                for _ in 0..len {
                    caret_line.push('^');
                }
                out.push_str(&caret_line);
                out.push('\n');
            }
        }
        out
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

    #[test]
    fn renders_caret_under_span() {
        let mut d = Diagnostics::new();
        d.error(
            Span { file: Arc::from("test.tasm"), line: 1, col: 6, len: 3 },
            "oh no",
        );
        let source = "hello world\nsecond line";
        let rendered = d.render(source);
        assert!(rendered.contains("oh no"), "expected 'oh no' in:\n{rendered}");
        assert!(rendered.contains("^^^"), "expected '^^^' in:\n{rendered}");
    }
}
