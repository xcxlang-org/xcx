// Low-level character cursor for navigating UTF-8 source text.
pub struct Cursor<'a> {
    pub(crate) source: &'a [u8],
    pub(crate) pos: usize,
    pub(crate) char_pos: usize,
    pub(crate) line: usize,
    pub(crate) col: usize,
}

impl<'a> Cursor<'a> {
    // Creates a new cursor for the given source text.
    pub fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            pos: 0,
            char_pos: 0,
            line: 1,
            col: 1,
        }
    }

    // Returns the current line number (1-indexed).
    pub fn line(&self) -> usize { self.line }

    // Returns the current column number (1-indexed).
    pub fn col(&self) -> usize { self.col }

    // Returns the character at the given offset from the current position.
    pub fn peek_at(&self, offset: usize) -> u8 {
        if self.pos + offset >= self.source.len() {
            b'\0'
        } else {
            self.source[self.pos + offset]
        }
    }

    // Returns the character at the current position.
    pub fn peek(&self) -> u8 { self.peek_at(0) }

    // Returns the character after the current position.
    pub fn peek_next(&self) -> u8 { self.peek_at(1) }

    // Returns the raw source bytes beyond the current position.
    pub fn remain(&self) -> &'a [u8] {
        &self.source[self.pos..]
    }

    // Returns a slice from the given start position to the current position.
    pub fn slice_from(&self, start_pos: usize) -> &'a [u8] {
        &self.source[start_pos..self.pos]
    }

    // Returns the current byte position.
    pub fn pos(&self) -> usize { self.pos }

    // Returns the current character position (handling multi-byte UTF-8).
    pub fn char_pos(&self) -> usize { self.char_pos }

    // Returns true if the cursor has reached the end of the source.
    pub fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }

    // Advances the cursor by one character and returns the byte consumed.
    pub fn advance(&mut self) -> u8 {
        let c = self.source[self.pos];
        self.pos += 1;
        if c < 128 || (c & 0b1100_0000) != 0b1000_0000 {
            self.char_pos += 1;
            if c == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        c
    }

    // Moves the cursor back by the given number of bytes.
    // Dangerous if used to cross line boundaries or middle of UTF-8 characters
    // without careful management of line/col/char_pos.
    pub fn backtrack(&mut self, n: usize) {
        self.pos -= n;
        // Simple backtrack doesn't fix char_pos/line/col perfectly, 
        // but for small symbol peeks it's usually okay.
        self.char_pos -= n;
        self.col -= n;
    }
}
