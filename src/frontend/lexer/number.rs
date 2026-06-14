use super::cursor::Cursor;
use super::token::Token;
use super::token_kind::TokenKind;

// Lexes integer and float literals.
pub fn lex(cursor: &mut Cursor, line: usize, col: usize) -> Token {
    let start_pos = cursor.pos();
    let start_char_pos = cursor.char_pos();
    let mut is_float = false;

    while cursor.peek().is_ascii_digit() {
        cursor.advance();
    }

    if cursor.peek() == b'.' && cursor.peek_next().is_ascii_digit() {
        is_float = true;
        cursor.advance();
        while cursor.peek().is_ascii_digit() {
            cursor.advance();
        }
    }

    let num_bytes = cursor.slice_from(start_pos);
    let num_str = std::str::from_utf8(num_bytes).unwrap_or("0");
    let len = cursor.char_pos() - start_char_pos;

    if is_float {
        Token::new(TokenKind::FloatLiteral(num_str.parse().unwrap_or(0.0)), line, col, len)
    } else {
        Token::new(TokenKind::IntLiteral(num_str.parse().unwrap_or(0)), line, col, len)
    }
}
