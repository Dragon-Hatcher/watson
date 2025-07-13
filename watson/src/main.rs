use crate::{
    diagnostics::{ReportTracker, WResult},
    span::SourceCache,
};
use std::{ffi::OsStr, path::Path};
use walkdir::WalkDir;

mod diagnostics;
mod parse;
mod span;
mod statements;
mod util;

fn main() {
    let base_dir = std::env::args_os().nth(1).unwrap();
    let base_dir = Path::new(&base_dir);

    let mut sources = SourceCache::new(base_dir.to_owned());
    let mut tracker = ReportTracker::new();

    for entry in WalkDir::new(base_dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() || entry.path().extension() != Some(OsStr::new("wats")) {
            continue;
        }
        sources.add_path(entry.path());
    }

    match compile(&sources, &mut tracker) {
        Ok(_) => {
            println!("Compiled successfully!");
        }
        Err(_) => {
            for report in tracker.reports() {
                report.render(&sources);
                println!();
            }
        }
    }
}

fn compile(sources: &SourceCache, tracker: &mut ReportTracker) -> WResult<()> {
    let statements = statements::get_all_statements(sources, tracker)?;
    let parsed = parse::parse(statements, tracker)?;

    dbg!(parsed);

    Ok(())
}
