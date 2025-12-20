use crate::{
    parse::{Location, Span, location::SourceId},
    strings,
};
use rustc_hash::FxHashMap;
use std::{
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
};
use ustr::Ustr;

/// Stores the text of all the loaded source files.
pub struct SourceCache {
    sources: RwLock<FxHashMap<SourceId, SourceInfo>>,
}

struct SourceInfo {
    text: Ustr,
    decl: SourceDecl,
    /// Lazily-computed line start offsets (byte offsets where each line begins).
    /// line_starts[0] = 0 (start of line 1)
    /// line_starts[1] = byte offset of line 2, etc.
    line_starts: OnceLock<Vec<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceDecl {
    Root,
    LuaSnippet,
    Module(Span),
}

impl SourceCache {
    pub fn new() -> Self {
        Self {
            sources: RwLock::default(),
        }
    }

    pub fn has_source(&self, id: SourceId) -> bool {
        self.sources.read().unwrap().contains_key(&id)
    }

    pub fn add(&self, id: SourceId, text: String, decl: SourceDecl) {
        assert!(!self.has_source(id));
        let text = Ustr::from(&text);
        self.sources.write().unwrap().insert(
            id,
            SourceInfo {
                text,
                decl,
                line_starts: OnceLock::new(),
            },
        );
    }

    pub fn get_text(&self, id: SourceId) -> Ustr {
        self.sources.read().unwrap()[&id].text
    }

    pub fn get_decl(&self, id: SourceId) -> SourceDecl {
        self.sources.read().unwrap()[&id].decl
    }

    /// Get the 1-indexed line number for a location in the source.
    /// This method lazily builds a line start index on first use.
    pub fn get_line_number(&self, location: Location) -> usize {
        let sources = self.sources.read().unwrap();
        let source_info = &sources[&location.source()];

        // Lazily compute line starts if not already done
        let line_starts = source_info
            .line_starts
            .get_or_init(|| compute_line_starts(source_info.text.as_str()));

        // Binary search to find which line the byte offset falls into
        let byte_offset = location.byte_offset();
        match line_starts.binary_search(&byte_offset) {
            // Exact match - this byte offset is the start of a line
            Ok(line_index) => line_index + 1,
            // Not an exact match - the byte offset is within a line
            Err(line_index) => line_index,
        }
    }
}

/// Compute the byte offset of the start of each line.
/// Returns a vector where line_starts[i] is the byte offset where line i+1 begins.
fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut line_starts = vec![0]; // Line 1 starts at byte 0

    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            // Next line starts after this newline
            line_starts.push(i + 1);
        }
    }

    line_starts
}

pub fn source_id_to_path(source: SourceId, root_dir: &Path) -> PathBuf {
    let mut path = root_dir.to_path_buf();
    for part in source.name().as_str().split('.') {
        path.push(part);
    }
    path.set_extension(*strings::FILE_EXTENSION);
    path
}
