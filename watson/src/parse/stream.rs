use std::char;

pub struct Stream<'a> {
    text: &'a str,
    pos: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checkpoint(usize);

pub enum ParseError {
    Backtrack,
    Commit,
}

pub type ParseResult<T> = Result<T, ParseError>;

impl<'a> Stream<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text, pos: 0 }
    }

    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint(self.pos)
    }

    pub fn rewind(&mut self, to: Checkpoint) {
        self.pos = to.0;
    }

    pub fn fallible<F, T>(&mut self, mut f: F) -> ParseResult<T>
    where
        F: FnMut(&mut Self) -> ParseResult<T>,
    {
        let check = self.checkpoint();
        let res = f(self);

        if res.is_err() {
            self.rewind(check);
        }

        res
    }

    pub fn commit<F, T>(&mut self, mut f: F) -> ParseResult<T>
    where
        F: FnMut(&mut Self) -> ParseResult<T>,
    {
        f(self).map_err(|_| ParseError::Commit)
    }

    pub fn peek(&mut self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    pub fn pop(&mut self) -> Option<char> {
        let (width, char) = self.text[self.pos..].char_indices().next()?;
        self.pos += width;
        Some(char)
    }

    pub fn expect_char_is<F>(&mut self, pred: F) -> ParseResult<char>
    where
        F: Fn(char) -> bool,
    {
        if let Some(char) = self.peek()
            && pred(char)
        {
            self.pop();
            Ok(char)
        } else {
            Err(ParseError::Backtrack)
        }
    }

    pub fn expect_char(&mut self, char: char) -> ParseResult<char> {
        self.expect_char_is(|c| c == char)
    }

    pub fn expect_str(&mut self, str: &str) -> ParseResult<()> {
        self.fallible(|s| {
            for char in str.chars() {
                s.expect_char(char)?;
            }

            Ok(())
        })
    }

    pub fn expect_eof(&self) -> ParseResult<()> {
        if self.pos >= self.text.len() {
            Ok(())
        } else {
            Err(ParseError::Backtrack)
        }
    }
}
