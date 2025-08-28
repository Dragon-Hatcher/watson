use ustr::Ustr;

use crate::{
    parse::{Span, location::SourceId},
    strings,
};
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
    decl: SourceDecl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceDecl {
    Root,
    Module(Span),
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

    pub fn add(&mut self, id: SourceId, text: String, decl: SourceDecl) {
        assert!(!self.has_source(id));
        self.sources.insert(id, SourceInfo { text, decl });
    }

    pub fn get_text(&self, id: SourceId) -> &str {
        &self.sources[&id].text
    }

    pub fn get_decl(&self, id: SourceId) -> SourceDecl {
        self.sources[&id].decl
    }
}

pub fn source_id_to_path(source: SourceId, root_dir: &Path) -> PathBuf {
    let mut path = root_dir.to_path_buf();
    for part in source.name().as_str().split('.') {
        path.push(part);
    }
    path.set_extension(*strings::FILE_EXTENSION);
    path
}
