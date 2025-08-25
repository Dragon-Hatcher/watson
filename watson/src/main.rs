use crate::{
    diagnostics::{DiagManager, WResult},
    parse::{SourceCache, SourceId, parse},
    semant::{
        ProofReport, check_proofs, formal_syntax::FormalSyntax, fragments::FragCtx,
        theorem::TheoremStatements,
    },
};
use std::{fs, path::Path, process::exit};
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

    let mut diags = DiagManager::new();
    let mut frag_ctx = FragCtx::new();
    let mut formal_syntax = None;
    let (mut sources, root_source) = make_source_cache(root_file).unwrap();

    let report = compile(
        root_source,
        &mut sources,
        &mut diags,
        &mut frag_ctx,
        &mut formal_syntax,
    );

    if diags.has_fatal_errors() {
        diags.print_errors(&sources, None, &frag_ctx, formal_syntax.as_ref());
        exit(1);
    }

    let (all_ok, statements) = match report {
        Ok((report, statements)) => (report::display_report(&report), Some(statements)),
        Err(_) => (false, None),
    };

    if diags.has_errors() {
        println!();
        diags.print_errors(
            &sources,
            statements.as_ref(),
            &frag_ctx,
            formal_syntax.as_ref(),
        );
    }

    if !all_ok {
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

fn compile(
    root: SourceId,
    sources: &mut SourceCache,
    diags: &mut DiagManager,
    frag_ctx: &mut FragCtx,
    formal: &mut Option<FormalSyntax>,
) -> WResult<(ProofReport, TheoremStatements)> {
    let (theorems, formal_syntax, macros) = parse(root, sources, diags);
    *formal = Some(formal_syntax);

    if diags.has_errors() {
        return Err(());
    }

    check_proofs(theorems, formal.as_ref().unwrap(), &macros, diags, frag_ctx)
}
