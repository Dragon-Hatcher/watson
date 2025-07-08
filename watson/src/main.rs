use crate::{
    diagnostics::{ReportTracker, WResult},
    parser::parse,
    span::{Filename, SourceCache},
};

mod diagnostics;
mod parser;
mod span;
mod util;

fn main() {
    let mut sources = SourceCache::new();
    let mut tracker = ReportTracker::new();

    for filename in std::env::args().skip(1) {
        let filename = Filename::new(&filename);
        sources.add_file(filename);
    }

    match compile(&sources, &mut tracker) {
        Ok(_) => {
            println!("Compiled successfully!");
        },
        Err(_) => {
            for report in tracker.reports() {
                report.render(&sources);
            }
        },
    }
}

fn compile(sources: &SourceCache, tracker: &mut ReportTracker) -> WResult<()> {
    parse(sources, tracker)?;

    Ok(())
}
