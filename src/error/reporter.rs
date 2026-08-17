use crate::error::diagnostic::{Diagnostic, Severity};

pub struct Reporter<'a> {
    lines: Vec<&'a str>,
    diagnostics: Vec<Diagnostic>,
}

const ANSI_RED: &str = "\x1b[31;1m";
const ANSI_YELLOW: &str = "\x1b[33;1m";
const ANSI_CYAN: &str = "\x1b[36m";
const ANSI_BOLD: &str = "\x1b[1m";
const ANSI_RESET: &str = "\x1b[0m";

impl<'a> Reporter<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            lines: source.lines().collect(),
            diagnostics: Vec::new(),
        }
    }

    pub fn emit(&mut self, diagnostic: Diagnostic) {
        self.print_diagnostic(&diagnostic);
        self.diagnostics.push(diagnostic);
    }

    fn print_diagnostic(&self, d: &Diagnostic) {
        let level_color = match d.severity {
            Severity::Error => ANSI_RED,
            Severity::Warning => ANSI_YELLOW,
            Severity::Note => ANSI_CYAN,
        };
        
        let level_str = match d.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "WARNING",
            Severity::Note => "NOTE",
        };

        println!("{}{}{}: {}{}{}", level_color, ANSI_BOLD, level_str, ANSI_RESET, ANSI_BOLD, d.message);
        
        let line = d.span.line;
        let col = d.span.col;
        let len = d.span.len;

        if line > 0 && line <= self.lines.len() {
            let line_content = self.lines[line - 1];
            println!("{} {:>3} |{} {}", ANSI_CYAN, line, ANSI_RESET, line_content);
            
            let padding = " ".repeat(col + 6);
            let highlight = if len > 0 { "~".repeat(len) } else { "^".to_string() };
            println!("{}{}{}{}", padding, ANSI_YELLOW, highlight, ANSI_RESET);
        }
        println!();
    }

    // Compatibility method for old code
    pub fn error(&mut self, line: usize, col: usize, len: usize, message: &str) {
        use crate::error::span::Span;
        self.emit(Diagnostic {
            message: message.to_string(),
            span: Span { line, col, len },
            severity: Severity::Error,
            code: None,
        });
    }
}
