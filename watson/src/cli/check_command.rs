use crate::{
    book,
    config::{WatsonConfig, find_config_file},
    context::{Arenas, Ctx},
    parse::{ParseReport, SourceCache, SourceId, parse, source_cache::SourceDecl},
    report::{ProofReport, display_report},
    semant::{check_circularity::find_circular_dependency_groups, check_proofs::check_proofs},
};
use argh::FromArgs;
use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};
use notify::Watcher;
use std::{io, path::PathBuf, sync::mpsc, thread};
use ustr::Ustr;

/// Check proofs in a Watson project.
#[derive(FromArgs)]
#[argh(subcommand, name = "check")]
pub struct CheckCommand {
    /// continually recheck on file changes.
    #[argh(switch, short = 'w')]
    watch: bool,

    /// build and serve the book after successful checks.
    #[argh(switch, short = 'b')]
    book: bool,

    /// path to watson.toml config file.
    #[argh(option, short = 'c')]
    config: Option<PathBuf>,
}

pub fn run_check(cmd: CheckCommand) {
    // Find watson.toml config file
    let config_file_path = match cmd.config {
        Some(file) => file.canonicalize().unwrap(),
        None => find_config_file().unwrap(),
    };

    if cmd.watch {
        let config = WatsonConfig::from_file(&config_file_path).unwrap();

        let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
        let mut watcher = notify::recommended_watcher(tx).unwrap();
        watcher
            .watch(config.src_dir(), notify::RecursiveMode::Recursive)
            .unwrap();

        // Build book initially and start server in background if book flag is set
        if cmd.book {
            let book_path = config.build_dir().join("book");
            let book_port = config.book().port();
            thread::spawn(move || {
                book::server::serve(&book_path, book_port);
            });
        }

        for i in 1.. {
            let _ = rx.try_iter().count();
            let arenas = Arenas::new();
            let (mut ctx, parse_report, report) = check(config.clone(), &arenas);

            // Clear the screen to print the new info
            _ = execute!(io::stdout(), Clear(ClearType::Purge), MoveTo(0, 0));

            display_report(&report, ctx.diags.has_errors(), Some(i));
            if ctx.diags.has_errors() {
                ctx.diags.print_errors(&ctx);
            } else if cmd.book {
                // Rebuild book on successful check
                println!();
                book::build_book(&mut ctx, parse_report, report, true, "/");
            }

            while let Ok(e) = rx.recv().unwrap() {
                if !matches!(e.kind, notify::EventKind::Access(_)) {
                    break;
                }
            }
        }
    } else {
        let config = WatsonConfig::from_file(&config_file_path).unwrap();

        let arenas = Arenas::new();
        let (mut ctx, parse_report, report) = check(config.clone(), &arenas);

        display_report(&report, ctx.diags.has_errors(), None);

        if ctx.diags.has_errors() {
            ctx.diags.print_errors(&ctx);
            std::process::exit(1)
        } else if cmd.book {
            // Build and serve book after successful check
            let book_path = book::build_book(&mut ctx, parse_report, report, false, "/");
            let port = config.book().port();
            println!();
            book::server::serve(&book_path, port);
        }
    }
}

pub fn check<'ctx>(
    config: WatsonConfig,
    arenas: &'ctx Arenas<'ctx>,
) -> (Ctx<'ctx>, ParseReport<'ctx>, ProofReport<'ctx>) {
    let (source_cache, root_id) = make_source_cache(&config);
    let mut ctx = Ctx::new(source_cache, config, arenas);
    let (parse_report, proof_report) = compile(root_id, &mut ctx);
    (ctx, parse_report, proof_report)
}

fn make_source_cache(config: &WatsonConfig) -> (SourceCache, SourceId) {
    let source_cache = SourceCache::new(config.project_dir().into());

    let root_path = config.src_dir().join("main.wats");
    let root_text = std::fs::read_to_string(&root_path).unwrap();
    let root_id = SourceId::new(Ustr::from("main"));
    source_cache.add(root_id, root_text, SourceDecl::Root);

    (source_cache, root_id)
}

fn compile<'ctx>(root: SourceId, ctx: &mut Ctx<'ctx>) -> (ParseReport<'ctx>, ProofReport<'ctx>) {
    let parse_report = parse(root, ctx);
    let statuses = check_proofs(&parse_report.theorems, ctx);
    let circularities = find_circular_dependency_groups(&statuses);

    let proof_report = ProofReport {
        statuses,
        circularities,
    };
    (parse_report, proof_report)
}
