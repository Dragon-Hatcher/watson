use crate::parse::{SourceCache, SourceId, parse};
use std::{ffi::OsStr, path::Path};
use ustr::Ustr;
use walkdir::WalkDir;

mod parse;
mod diagnostics;
mod strings;

fn main() {
    let base_dir = std::env::args_os().nth(1).unwrap();
    let base_dir = Path::new(&base_dir);

    let sources = collect_sources(base_dir);

    compile(&sources);
}

fn collect_sources(base_dir: &Path) -> SourceCache {
    fn get_source_key(base_dir: &Path, file: &Path) -> SourceId {
        let relative = file.strip_prefix(base_dir).unwrap();
        let relative = Ustr::from(&relative.to_string_lossy());
        SourceId::new(relative)
    }

    let extension = OsStr::new("wats");

    let mut source_cache = SourceCache::new();

    for entry in WalkDir::new(base_dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() || entry.path().extension() != Some(extension) {
            continue;
        }

        let key = get_source_key(base_dir, entry.path());
        let text = std::fs::read_to_string(entry.path()).unwrap();
        source_cache.add(key, text);
    }

    source_cache
}

fn compile(sources: &SourceCache) {
    parse(sources);
}
