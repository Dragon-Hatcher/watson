use std::{collections::HashMap, fmt::Debug};

use ustr::Ustr;

#[derive(Debug, Clone, Copy)]
pub struct Span {
    filename: Ustr,
    start: usize,
    end: usize,
}

impl Span {
    pub fn new(filename: Ustr, start: usize, end: usize) -> Self {
        Self { filename, start, end }
    }
}

impl ariadne::Span for Span {
    type SourceId = Ustr;

    fn source(&self) -> &Self::SourceId {
        &self.filename
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}


pub struct SourceCache {
    files: HashMap<Ustr, ariadne::Source<String>>
}

impl SourceCache {
    pub fn new() -> Self {
        Self { files: HashMap::new() }
    }

    pub fn add_file(&mut self, path: Ustr) {
        let file_text = std::fs::read_to_string(path).unwrap();
        self.files.insert(path, ariadne::Source::from(file_text));
    }

    pub fn get_text(&self, path: Ustr) -> &str {
        self.files[&path].text()
    }
}

impl ariadne::Cache<Ustr> for &SourceCache {
    type Storage = String;

    fn fetch(&mut self, id: &Ustr) -> Result<&ariadne::Source<Self::Storage>, impl Debug> {
        let source = &self.files[id];
        Ok(source) as Result<_, ()>
    }

    fn display<'a>(&self, id: &'a Ustr) -> Option<impl std::fmt::Display + 'a> {
        Some(id.as_str())
    }
}