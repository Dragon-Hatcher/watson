use crate::{
    context::{Arenas, Ctx},
    parse::{SourceCache, SourceId, parse, source_cache::SourceDecl},
    report::{ProofReport, display_report},
    semant::{check_circularity::find_circular_dependency_groups, check_proof::check_proofs},
};
use argh::FromArgs;
use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};
use notify::Watcher;
use std::{
    io,
    path::{Path, PathBuf},
    sync::mpsc,
};
use ustr::Ustr;

mod context;
mod diagnostics;
mod parse;
mod report;
mod semant;
mod strings;
mod util;

/// The Watson proof assistant.
#[derive(FromArgs)]
struct Args {
    /// continually recheck on file changes.
    #[argh(switch, short = 'w')]
    watch: bool,

    /// the root file of the project.
    #[argh(positional)]
    root: PathBuf,
}

fn main() {
    let args: Args = argh::from_env();

    if args.watch {
        let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
        let mut watcher = notify::recommended_watcher(tx).unwrap();
        watcher
            .watch(&args.root, notify::RecursiveMode::Recursive)
            .unwrap();

        for i in 1.. {
            let _ = rx.try_iter().count();
            let arenas = Arenas::new();
            let (ctx, report) = run(&args, &arenas);

            // Clear the screen to print the new info
            _ = execute!(io::stdout(), Clear(ClearType::Purge), MoveTo(0, 0));

            display_report(&report, ctx.diags.has_errors(), Some(i));
            if ctx.diags.has_errors() {
                ctx.diags.print_errors(&ctx);
            }

            while let Ok(e) = rx.recv().unwrap() {
                if !matches!(e.kind, notify::EventKind::Access(_)) {
                    break;
                }
            }
        }
    } else {
        let arenas = Arenas::new();
        let (ctx, report) = run(&args, &arenas);

        // display_report(&report, ctx.diags.has_errors(), None);

        // if ctx.diags.has_errors() {
        //     ctx.diags.print_errors(&ctx);
        //     std::process::exit(1)
        // }
    }
}

fn run<'ctx>(args: &Args, arenas: &'ctx Arenas<'ctx>) -> (Ctx<'ctx>, ProofReport<'ctx>) {
    let (source_cache, root_id) = make_source_cache(&args.root);
    let mut ctx = Ctx::new(source_cache, arenas);
    let report = compile(root_id, &mut ctx);
    (ctx, report)
}

fn make_source_cache(root_file: &Path) -> (SourceCache, SourceId) {
    let root_file = root_file.canonicalize().unwrap(); // TODO.
    let root_dir = root_file.parent().unwrap().to_path_buf();
    let source_cache = SourceCache::new(root_dir);

    let text = std::fs::read_to_string(&root_file).unwrap();
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
