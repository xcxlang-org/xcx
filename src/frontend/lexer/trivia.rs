use super::cursor::Cursor;

// Skips whitespace and comments in the source text. Inside a multi-line
// comment, double-quoted spans are opaque: a `*---` or `---` sequence that
// appears within quotes does not close or split the comment, mirroring how
// the string lexer consumes escapes.
pub fn skip(cursor: &mut Cursor) {
    loop {
        let c = cursor.peek();
        if c.is_ascii_whitespace() {
            cursor.advance();
        } else if cursor.remain().starts_with(b"---") {
            cursor.advance(); cursor.advance(); cursor.advance();

            let mut is_multi = true;
            let mut offset = 0;
            while cursor.peek_at(offset) != b'\n' && cursor.peek_at(offset) != b'\0' {
                if !cursor.peek_at(offset).is_ascii_whitespace() {
                    is_multi = false;
                    break;
                }
                offset += 1;
            }

            if is_multi {
                let mut found_close = false;
                let mut scan_offset = 0;
                let mut in_string = false;
                while cursor.peek_at(scan_offset) != b'\0' {
                    let b = cursor.peek_at(scan_offset);
                    if in_string {
                        if b == b'\\' {
                            scan_offset += 1; // skip the escaped byte
                        } else if b == b'"' {
                            in_string = false;
                        }
                        scan_offset += 1;
                        continue;
                    }
                    if b == b'"' {
                        in_string = true;
                        scan_offset += 1;
                        continue;
                    }

                    let mut match_close = true;
                    for (i, &expected) in b"*---".iter().enumerate() {
                        if cursor.peek_at(scan_offset + i) != expected {
                            match_close = false;
                            break;
                        }
                    }
                    if match_close {
                        found_close = true;
                        break;
                    }

                    let mut match_start = true;
                    for (i, &expected) in b"---".iter().enumerate() {
                        if cursor.peek_at(scan_offset + i) != expected {
                            match_start = false;
                            break;
                        }
                    }
                    if match_start {
                        let mut is_own_line = true;
                        let mut check_offset = scan_offset + 3;
                        while cursor.peek_at(check_offset) != b'\n' && cursor.peek_at(check_offset) != b'\0' {
                            if !cursor.peek_at(check_offset).is_ascii_whitespace() {
                                is_own_line = false;
                                break;
                            }
                            check_offset += 1;
                        }
                        if is_own_line {
                            break;
                        }
                    }

                    scan_offset += 1;
                }
                if !found_close {
                    is_multi = false;
                }
            }

            if !is_multi {
                while cursor.peek() != b'\n' && cursor.peek() != b'\0' {
                    cursor.advance();
                }
            } else {
                let mut in_string = false;
                while cursor.peek() != b'\0' {
                    let b = cursor.peek();
                    if in_string {
                        if b == b'\\' {
                            cursor.advance();
                            if cursor.peek() == b'\0' {
                                break;
                            }
                        } else if b == b'"' {
                            in_string = false;
                        }
                        cursor.advance();
                        continue;
                    }
                    if b == b'"' {
                        in_string = true;
                        cursor.advance();
                        continue;
                    }
                    if cursor.remain().starts_with(b"*---") {
                        cursor.advance(); cursor.advance(); cursor.advance(); cursor.advance();
                        break;
                    }
                    cursor.advance();
                }
            }
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::frontend::parser::Parser;

    /// A multi-line comment containing a quoted `*---` must not close at
    /// the in-string sequence. Regression test: 4.4 terminated the comment
    /// inside the quotes and parsed the remaining comment text as code.
    #[test]
    fn comment_string_star_dashes_is_opaque() {
        let src = "---\ncomment with \"str*---ing\" inside\n*---\ns: b = \"ok\";\n>! b;\n";
        let mut parser = Parser::new(src);
        let program = parser.parse_program();
        assert!(!parser.has_error, "comment must close at the real *---");
        assert_eq!(program.stmts.len(), 2);
    }

    /// A string in code keeps containing `*---` verbatim, and an escaped
    /// quote inside a commented string keeps the span opaque.
    #[test]
    fn star_dashes_opacity_edges() {
        let src = "s: a = \"x*---y\";\n>! a.length;\n---\n\"escaped \\\" stays *--- open\"\n*---\ns: x = 1;\n";
        let mut parser = Parser::new(src);
        let program = parser.parse_program();
        assert!(!parser.has_error, "valid program must parse");
        assert_eq!(program.stmts.len(), 3);
    }
}
