use std::{collections::HashMap, fmt::Debug};
use ustr::Ustr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Filename(Ustr);

impl Filename {
    pub fn new(str: &str) -> Self {
        Self(Ustr::from(str))
    }

    pub fn as_str(&self) -> &'static str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Span {
    filename: Filename,
    start: usize,
    end: usize,
}

impl Span {
    pub fn new(filename: Filename, start: usize, end: usize) -> Self {
        Self { filename, start, end }
    }
}

impl Span {
    pub fn file(&self) -> Filename {
        self.filename
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }
}

pub struct SourceCache {
    files: HashMap<Filename, String>
}

impl SourceCache {
    pub fn new() -> Self {
        Self { files: HashMap::new() }
    }

    pub fn add_file(&mut self, filename: Filename) {
        let file_text = std::fs::read_to_string(filename.as_str()).unwrap();
        self.files.insert(filename, file_text);
    }

    pub fn get_text(&self, filename: Filename) -> &str {
        &self.files[&filename]
    }

    pub fn files(&self) -> impl Iterator<Item=(&Filename, &String)> {
        self.files.iter()
    }
}
