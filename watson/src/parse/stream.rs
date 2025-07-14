use std::char;
use ustr::Ustr;

#[derive(Debug)]
pub struct Stream<'a> {
    text: &'a str,
    pos: usize,
    ignore_ws: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Checkpoint(pub usize);

impl<'a> Stream<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            pos: 0,
            ignore_ws: true,
        }
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

    pub fn include_ws<F, T>(&mut self, mut f: F) -> ParseResult<T>
    where
        F: FnMut(&mut Self) -> ParseResult<T>,
    {
        self.skip_ws();

        let prior_mode = self.ignore_ws;

        self.ignore_ws = false;
        let res = f(self);
        self.ignore_ws = prior_mode;

        res
    }

    pub fn skip_ws(&mut self) {
        if !self.ignore_ws {
            return;
        }

        let mut chars = self.text[self.pos..].char_indices();
        let mut off = 0;

        while let Some((char_off, char)) = chars.next() {
            off = char_off;

            if !char.is_ascii_whitespace() {
                break;
            }
        }

        if chars.next().is_none() {
            self.pos = self.text.len();
        } else {
            self.pos += off;
        }
    }

    pub fn pop(&mut self) -> Option<char> {
        self.skip_ws();

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
        let check = self.checkpoint();
        if let Some(char) = self.pop()
            && pred(char)
        {
            Ok(char)
        } else {
            self.rewind(check);
            Err(ParseError::new_backtrack(self.pos))
        }
    }

    pub fn expect_char(&mut self, char: char) -> ParseResult<char> {
        self.expect_char_is(|c| c == char).ctx_expect_char(char)
    }

    pub fn expect_str(&mut self, str: &str) -> ParseResult<()> {
        self.fallible(|s| {
            s.include_ws(|s| {
                for char in str.chars() {
                    s.expect_char(char)?;
                }

                Ok(())
            })
        })
        .ctx_clear(self.pos)
        .ctx_expect_str(Ustr::from(str).as_str())
    }

    pub fn expect_eof(&mut self) -> ParseResult<()> {
        let check = self.checkpoint();

        self.skip_ws();
        if self.pos >= self.text.len() {
            Ok(())
        } else {
            self.rewind(check);
            Err(ParseError::new_backtrack(self.pos)).ctx_expect_desc("end of input")
        }
    }

    pub fn fail<T>(&self) -> ParseResult<T> {
        Err(ParseError::new_backtrack(self.pos))
    }

    pub fn measure<F, T>(&mut self, mut f: F) -> Option<Checkpoint>
    where
        F: FnMut(&mut Self) -> ParseResult<T>,
    {
        let start = self.checkpoint();
        let parsed = f(self).ok();
        let end = parsed.map(|_| self.checkpoint());
        self.rewind(start);

        end
    }

    pub fn remaining_text(&self) -> &str {
        &self.text[self.pos..]
    }

    pub fn text(&self) -> &str {
        self.text
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
    pub fn place(&self) -> usize {
        self.place
    }

    pub fn trace(&self) -> &[ParseErrorCtxTy] {
        &self.trace
    }
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
    pub fn new_backtrack(place: usize) -> Self {
        Self::Backtrack(ParseErrorCtx::new(place))
    }
}

pub type ParseResult<T> = Result<T, ParseError>;

pub trait ParseErrorCtxHolder {
    fn ctx_expect_char(self, char: char) -> Self;
    fn ctx_expect_str(self, str: &'static str) -> Self;
    fn ctx_expect_desc(self, desc: &'static str) -> Self;
    fn ctx_label(self, label: &'static str) -> Self;

    fn ctx_clear(self, place: usize) -> Self;
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

    fn ctx_clear(mut self, place: usize) -> Self {
        if let Err(ParseError::Backtrack(e) | ParseError::Commit(e)) = &mut self {
            e.place = place;
            e.trace.clear();
        }
        self
    }
}
