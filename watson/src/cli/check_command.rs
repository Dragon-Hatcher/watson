use crate::{
    config::{WatsonConfig, find_config_file},
    context::{Arenas, Ctx},
    parse::{SourceCache, SourceId, parse, source_cache::SourceDecl},
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
use std::{io, path::PathBuf, sync::mpsc};
use ustr::Ustr;

/// Check proofs in a Watson project.
#[derive(FromArgs)]
#[argh(subcommand, name = "check")]
pub struct CheckCommand {
    /// continually recheck on file changes.
    #[argh(switch, short = 'w')]
    watch: bool,

    /// path to watson.toml config file (if not provided, searches up from current directory).
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
            .watch(config.project_dir(), notify::RecursiveMode::Recursive)
            .unwrap();

        for i in 1.. {
            let _ = rx.try_iter().count();
            let arenas = Arenas::new();
            let (ctx, report) = run(config.clone(), &arenas);

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
        let config = WatsonConfig::from_file(&config_file_path).unwrap();

        let arenas = Arenas::new();
        let (ctx, report) = run(config, &arenas);

        display_report(&report, ctx.diags.has_errors(), None);

        if ctx.diags.has_errors() {
            ctx.diags.print_errors(&ctx);
            std::process::exit(1)
        }
    }
}

fn run<'ctx>(config: WatsonConfig, arenas: &'ctx Arenas<'ctx>) -> (Ctx<'ctx>, ProofReport<'ctx>) {
    let (source_cache, root_id) = make_source_cache(&config);
    let mut ctx = Ctx::new(source_cache, config, arenas);
    let report = compile(root_id, &mut ctx);
    (ctx, report)
}

fn make_source_cache(config: &WatsonConfig) -> (SourceCache, SourceId) {
    let source_cache = SourceCache::new(config.project_dir().into());

    let root_path = config.project_dir().join("src").join("main.wats");
    let root_text = std::fs::read_to_string(&root_path).unwrap();
    let root_id = SourceId::new(Ustr::from("main"));
    source_cache.add(root_id, root_text, SourceDecl::Root);

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
