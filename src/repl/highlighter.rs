use crossterm::{
    style::{Color, ResetColor, SetForegroundColor, SetAttribute, Attribute},
    QueueableCommand,
};

pub struct Highlighter;

impl Highlighter {
    pub fn highlight(out: &mut std::io::Stdout, line: &str) -> std::io::Result<()> {
        // Quick handle for REPL special commands
        if line.trim().starts_with('!') {
            let _ = out.queue(SetForegroundColor(Color::Magenta));
            let _ = out.queue(crossterm::style::Print(line));
            let _ = out.queue(ResetColor);
            return Ok(());
        }

        let bytes = line.as_bytes();
        let mut i = 0;
        
        while i < bytes.len() {
            // 1. Comments
            if i + 2 < bytes.len() && bytes[i] == b'-' && bytes[i+1] == b'-' && bytes[i+2] == b'-' {
                let _ = out.queue(SetForegroundColor(Color::DarkGreen));
                let comment = &line[i..];
                let _ = out.queue(crossterm::style::Print(comment));
                let _ = out.queue(ResetColor);
                break;
            }

            // 2. String literal
            if bytes[i] == b'"' {
                let _ = out.queue(SetForegroundColor(Color::Yellow));
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i < bytes.len() {
                    i += 1; // include closing quote
                }
                let string_lit = &line[start..i];
                let _ = out.queue(crossterm::style::Print(string_lit));
                let _ = out.queue(ResetColor);
                continue;
            }

            // 3. Types and keywords / identifiers
            if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b':') {
                    i += 1;
                }
                let word_span = &line[start..i];
                
                // Extract inner name to check for types
                let mut first_part = word_span;
                if let Some(colon_pos) = word_span.find(':') {
                    first_part = &word_span[..colon_pos];
                }

                let is_type = word_span.ends_with(':') && match first_part {
                    "int" | "i" | "float" | "f" | "str" | "string" | "s" | "bool" | "b" |
                    "json" | "date" | "array" | "set" | "map" | "table" | "database" | "fiber" => true,
                    _ => false,
                };

                if is_type {
                    let _ = out.queue(SetForegroundColor(Color::Cyan));
                    let _ = out.queue(crossterm::style::Print(word_span));
                    let _ = out.queue(ResetColor);
                    continue;
                }

                // Check keywords
                let matches_keyword = match word_span {
                    "func" | "fiber" | "if" | "then" | "else" | "end" | "while" | "do" |
                    "for" | "in" | "to" | "return" | "const" | "yield" => true,
                    _ => false,
                };

                if matches_keyword {
                    let _ = out.queue(SetForegroundColor(Color::Blue));
                    let _ = out.queue(SetAttribute(Attribute::Bold));
                    let _ = out.queue(crossterm::style::Print(word_span));
                    let _ = out.queue(SetAttribute(Attribute::Reset));
                    let _ = out.queue(ResetColor);
                    continue;
                }

                // Check constants
                let matches_const = match word_span {
                    "true" | "false" | "null" => true,
                    _ => false,
                };

                if matches_const {
                    let _ = out.queue(SetForegroundColor(Color::Magenta));
                    let _ = out.queue(crossterm::style::Print(word_span));
                    let _ = out.queue(ResetColor);
                    continue;
                }

                // Normal word representation
                let _ = out.queue(crossterm::style::Print(word_span));
                continue;
            }

            // 4. Numbers
            if bytes[i].is_ascii_digit() {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.' ) {
                    i += 1;
                }
                let num = &line[start..i];
                let _ = out.queue(SetForegroundColor(Color::Magenta));
                let _ = out.queue(crossterm::style::Print(num));
                let _ = out.queue(ResetColor);
                continue;
            }

            // 5. Normal single symbol
            let ch = &line[i..i+1];
            let _ = out.queue(crossterm::style::Print(ch));
            i += 1;
        }
        
        Ok(())
    }
}
