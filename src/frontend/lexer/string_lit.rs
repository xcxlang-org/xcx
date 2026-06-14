use crate::intern::Interner;
use super::cursor::Cursor;
use super::token::Token;
use super::token_kind::TokenKind;

// Lexes single- and double-quoted string literals.
pub fn lex(cursor: &mut Cursor, line: usize, col: usize, interner: &mut Interner) -> Token {
    let start_char_pos = cursor.char_pos;
    let mut bytes = Vec::new();

    while cursor.peek() != b'"' && cursor.peek() != b'\0' {
        let c = cursor.advance();
        if c == b'\\' {
            match cursor.peek() {
                b'n' => { cursor.advance(); bytes.push(b'\n'); }
                b't' => { cursor.advance(); bytes.push(b'\t'); }
                b'r' => { cursor.advance(); bytes.push(b'\r'); }
                b'0'..=b'7' => {
                    let mut octal = String::new();
                    for _ in 0..3 {
                        if cursor.peek().is_ascii_digit() && cursor.peek() <= b'7' {
                            octal.push(cursor.advance() as char);
                        } else {
                            break;
                        }
                    }
                    if let Ok(val) = u32::from_str_radix(&octal, 8) {
                        if let Some(ch) = std::char::from_u32(val) {
                            let mut b = [0; 4];
                            bytes.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
                        }
                    }
                }
                b'x' => {
                    cursor.advance();
                    let mut hex = String::new();
                    for _ in 0..2 {
                        if cursor.peek().is_ascii_hexdigit() {
                            hex.push(cursor.advance() as char);
                        } else {
                            break;
                        }
                    }
                    if let Ok(val) = u32::from_str_radix(&hex, 16) {
                        if let Some(ch) = std::char::from_u32(val) {
                            let mut b = [0; 4];
                            bytes.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
                        }
                    }
                }
                b'"' => { cursor.advance(); bytes.push(b'"'); }
                b'\\' => { cursor.advance(); bytes.push(b'\\'); }
                _ => { bytes.push(b'\\'); }
            }
        } else {
            bytes.push(c);
        }
    }

    if cursor.peek() == b'"' {
        cursor.advance();
        let parsed_str = String::from_utf8(bytes).unwrap_or_default();
        let len = cursor.char_pos - start_char_pos;
        Token::new(TokenKind::StringLiteral(interner.intern(&parsed_str)), line, col, len)
    } else {
        Token::new(TokenKind::Unknown('"'), line, col, 1)
    }
}

// Lexes raw block literals (<<< ... >>>).
pub fn lex_raw(cursor: &mut Cursor, line: usize, col: usize, interner: &mut Interner) -> Token {
    let start_raw = cursor.pos;
    let start_char_pos = cursor.char_pos - 2; // assume '<<' already consumed by caller

    while !cursor.is_at_end() {
        if cursor.remain().starts_with(b">>>") {
            let source = cursor.source;
            let num_bytes = &source[start_raw..cursor.pos];
            let parsed_str = std::str::from_utf8(num_bytes).unwrap_or_default();
            let string_id = interner.intern(parsed_str);

            cursor.advance(); cursor.advance(); cursor.advance();
            let len = cursor.char_pos - start_char_pos;
            return Token::new(TokenKind::RawBlock(string_id), line, col, len);
        }
        cursor.advance();
    }

    Token::new(TokenKind::Unknown('<'), line, col, 3)
}
