use crate::{
    diagnostics::{DiagManager, WResult},
    parse::{SourceCache, SourceId, parse},
};
use std::{fs, path::Path, process::exit};
use ustr::Ustr;

mod diagnostics;
mod parse;
mod semant;
mod strings;

fn main() {
    let root_file = std::env::args_os().nth(1).unwrap();
    let root_file = Path::new(&root_file);

    let mut diags = DiagManager::new();
    let (mut sources, root_source) = make_source_cache(root_file).unwrap();

    compile(root_source, &mut sources, &mut diags);

    if diags.has_errors() {
        diags.print_errors(&sources);
        exit(1);
    }
}

fn make_source_cache(root_file: &Path) -> WResult<(SourceCache, SourceId)> {
    let parent = root_file.parent().unwrap();
    let mut sources = SourceCache::new(parent.to_path_buf());

    let source_id = Ustr::from(&root_file.file_stem().unwrap().to_string_lossy());
    let source_id = SourceId::new(source_id);

    let Ok(text) = fs::read_to_string(root_file) else {
        // TODO
        return Err(());
    };

    sources.add(source_id, text, None);

    Ok((sources, source_id))
}

fn compile(root: SourceId, sources: &mut SourceCache, diags: &mut DiagManager) {
    parse(root, sources, diags);
}
