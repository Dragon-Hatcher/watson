use std::char;
use ustr::Ustr;

#[derive(Debug)]
pub struct Stream<'a> {
    text: &'a str,
    pos: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checkpoint(usize);

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
        f(self).map_err(|e| match e {
            ParseError::Backtrack(e) => ParseError::Commit(e),
            ParseError::Commit(e) => ParseError::Commit(e),
        })
    }

    pub fn peek(&mut self) -> Option<char> {
        self.text[self.pos..].chars().next()
    }

    pub fn pop(&mut self) -> Option<char> {
        let mut chars = self.text[self.pos..].char_indices();
        let (_, char) = chars.next()?;

        if let Some((width, _)) = chars.next() {
            self.pos += width;
        } else {
            self.pos = self.text.len();
        }
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
            Err(ParseError::new(self.pos))
        }
    }

    pub fn expect_char(&mut self, char: char) -> ParseResult<char> {
        self.expect_char_is(|c| c == char).ctx_expect_char(char)
    }

    pub fn expect_str(&mut self, str: &str) -> ParseResult<()> {
        self.fallible(|s| {
            for char in str.chars() {
                s.expect_char(char)?;
            }

            Ok(())
        })
        .ctx_clear()
        .ctx_expect_str(Ustr::from(str).as_str())
    }

    pub fn expect_eof(&self) -> ParseResult<()> {
        if self.pos >= self.text.len() {
            Ok(())
        } else {
            Err(ParseError::new(self.pos)).ctx_expect_desc("end of input")
        }
    }
}

#[derive(Debug)]
pub enum ParseErrorCtxTy {
    ExpectChar(char),
    ExpectStr(&'static str),
    ExpectDescription(&'static str),
    Label(&'static str),
}

#[derive(Debug)]
pub struct ParseErrorCtx {
    place: usize,
    trace: Vec<ParseErrorCtxTy>,
}

impl ParseErrorCtx {
    fn new(place: usize) -> Self {
        Self {
            place,
            trace: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum ParseError {
    Backtrack(ParseErrorCtx),
    Commit(ParseErrorCtx),
}

impl ParseError {
    fn new(place: usize) -> Self {
        Self::Backtrack(ParseErrorCtx::new(place))
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

trait ParseErrorCtxHolder {
    fn ctx_expect_char(self, char: char) -> Self;
    fn ctx_expect_str(self, str: &'static str) -> Self;
    fn ctx_expect_desc(self, desc: &'static str) -> Self;
    fn ctx_label(self, label: &'static str) -> Self;

    fn ctx_clear(self) -> Self;
}

impl<T> ParseErrorCtxHolder for ParseResult<T> {
    fn ctx_expect_char(mut self, char: char) -> Self {
        if let Err(ParseError::Backtrack(e) | ParseError::Commit(e)) = &mut self {
            e.trace.push(ParseErrorCtxTy::ExpectChar(char));
        }
        self
    }

    fn ctx_expect_str(mut self, str: &'static str) -> Self {
        if let Err(ParseError::Backtrack(e) | ParseError::Commit(e)) = &mut self {
            e.trace.push(ParseErrorCtxTy::ExpectStr(str));
        }
        self
    }

    fn ctx_expect_desc(mut self, desc: &'static str) -> Self {
        if let Err(ParseError::Backtrack(e) | ParseError::Commit(e)) = &mut self {
            e.trace.push(ParseErrorCtxTy::ExpectDescription(desc));
        }
        self
    }

    fn ctx_label(mut self, label: &'static str) -> Self {
        if let Err(ParseError::Backtrack(e) | ParseError::Commit(e)) = &mut self {
            e.trace.push(ParseErrorCtxTy::Label(label));
        }
        self
    }

    fn ctx_clear(mut self) -> Self {
        if let Err(ParseError::Backtrack(e) | ParseError::Commit(e)) = &mut self {
            e.trace.clear();
        }
        self
    }
}
