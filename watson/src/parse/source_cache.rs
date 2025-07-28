use crate::parse::location::SourceId;
use std::{collections::HashMap, path::PathBuf};

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

    pub fn add(&mut self, id: SourceId, text: String) {
        assert!(!self.sources.contains_key(&id));
        self.sources.insert(id, text);
    }

    pub fn source_keys(&self) -> impl Iterator<Item = SourceId> {
        self.sources.keys().copied()
    }

    pub fn get_text(&self, id: SourceId) -> &str {
        &self.sources[&id]
    }
}
