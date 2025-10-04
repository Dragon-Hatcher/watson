use rustc_hash::FxHashMap;
use ustr::Ustr;

use crate::{
    parse::{Span, location::SourceId},
    strings,
};
use std::{
    path::{Path, PathBuf},
    sync::RwLock,
};

/// Stores the text of all the loaded source files.
pub struct SourceCache {
    root_dir: PathBuf,
    sources: RwLock<FxHashMap<SourceId, SourceInfo>>,
}

struct SourceInfo {
    text: Ustr,
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
            sources: RwLock::default(),
        }
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn has_source(&self, id: SourceId) -> bool {
        self.sources.read().unwrap().contains_key(&id)
    }

    pub fn add(&self, id: SourceId, text: String, decl: SourceDecl) {
        assert!(!self.has_source(id));
        let text = Ustr::from(&text);
        self.sources
            .write()
            .unwrap()
            .insert(id, SourceInfo { text, decl });
    }

    pub fn get_text(&self, id: SourceId) -> Ustr {
        self.sources.read().unwrap()[&id].text
    }

    pub fn get_decl(&self, id: SourceId) -> SourceDecl {
        self.sources.read().unwrap()[&id].decl
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
