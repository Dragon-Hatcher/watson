use crate::parse::{Span, location::SourceId};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// Stores the text of all the loaded source files.
pub struct SourceCache {
    root_dir: PathBuf,
    sources: HashMap<SourceId, SourceInfo>,
}

struct SourceInfo {
    text: String,
    decl: Option<Span>,
}

impl SourceCache {
    pub fn new(root_dir: PathBuf) -> Self {
        Self {
            root_dir,
            sources: HashMap::new(),
        }
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn has_source(&mut self, id: SourceId) -> bool {
        self.sources.contains_key(&id)
    }

    pub fn add(&mut self, id: SourceId, text: String, decl: Option<Span>) {
        assert!(!self.has_source(id));
        self.sources.insert(id, SourceInfo { text, decl });
    }

    pub fn get_text(&self, id: SourceId) -> &str {
        &self.sources[&id].text
    }

    pub fn get_decl(&self, id: SourceId) -> Option<Span> {
        self.sources[&id].decl
    }
}
