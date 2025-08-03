use crate::parse::location::SourceId;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// Stores the text of all the loaded source files.
pub struct SourceCache {
    root_dir: PathBuf,
    sources: HashMap<SourceId, String>,
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

    pub fn add(&mut self, id: SourceId, text: String) {
        assert!(!self.has_source(id));
        self.sources.insert(id, text);
    }

    pub fn source_keys(&self) -> impl Iterator<Item = SourceId> {
        self.sources.keys().copied()
    }

    pub fn get_text(&self, id: SourceId) -> &str {
        &self.sources[&id]
    }
}
