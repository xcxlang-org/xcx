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
            
            if is_multi {
                let mut found_close = false;
                let mut scan_offset = 0;
                while cursor.peek_at(scan_offset) != b'\0' {
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
