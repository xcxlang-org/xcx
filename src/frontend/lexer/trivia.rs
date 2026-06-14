use super::cursor::Cursor;

// Skips whitespace and comments in the source text.
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
            
            if !is_multi {
                while cursor.peek() != b'\n' && cursor.peek() != b'\0' {
                    cursor.advance();
                }
            } else {
                while cursor.peek() != b'\0' {
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
