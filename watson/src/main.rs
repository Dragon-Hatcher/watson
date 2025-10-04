use crate::{
    context::{Arenas, Ctx},
    parse::{SourceCache, SourceId, parse, source_cache::SourceDecl},
    report::{ProofReport, display_report},
    semant::{check_circularity::find_circular_dependency_groups, check_proof::check_proofs},
};
use std::path::Path;
use ustr::Ustr;

mod context;
mod diagnostics;
mod parse;
mod report;
mod semant;
mod strings;
mod util;

fn main() {
    let root_file = std::env::args_os().nth(1).unwrap();
    let root_file = Path::new(&root_file);

    let (source_cache, root_id) = make_source_cache(root_file);
    let arenas = Arenas::new();
    let mut ctx = Ctx::new(source_cache, &arenas);

    let report = compile(root_id, &mut ctx);
    display_report(&report);

    if ctx.diags.has_errors() {
        ctx.diags.print_errors(&ctx);
        std::process::exit(1);
    }
}

fn make_source_cache(root_file: &Path) -> (SourceCache, SourceId) {
    let root_dir = root_file.parent().unwrap().to_path_buf();
    let root_dir = root_dir.canonicalize().unwrap();
    let source_cache = SourceCache::new(root_dir);

    let text = std::fs::read_to_string(root_file).unwrap();
    let root_id = Ustr::from(&root_file.file_stem().unwrap().to_string_lossy());
    let root_id = SourceId::new(root_id);
    source_cache.add(root_id, text, SourceDecl::Root);

    (source_cache, root_id)
}

fn compile<'ctx>(root: SourceId, ctx: &mut Ctx<'ctx>) -> ProofReport<'ctx> {
    let theorems = parse(root, ctx);
    let statuses = check_proofs(&theorems, ctx);
    let circularities = find_circular_dependency_groups(&statuses);

    ProofReport {
        statuses,
        circularities,
    }
}
