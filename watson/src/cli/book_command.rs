use crate::{
    book,
    cli::check_command,
    config::{WatsonConfig, find_config_file},
    context::Arenas,
    util::ansi::{ANSI_BOLD, ANSI_RED, ANSI_RESET},
};
use argh::FromArgs;
use std::path::PathBuf;

/// Build the book for a Watson project.
#[derive(FromArgs)]
#[argh(subcommand, name = "book")]
pub struct BookCommand {
    /// path to watson.toml config file.
    #[argh(option, short = 'c')]
    config: Option<PathBuf>,
}

pub fn run_book(cmd: BookCommand) {
    // Find watson.toml config file
    let config_file_path = match cmd.config {
        Some(file) => file.canonicalize().unwrap(),
        None => find_config_file().unwrap(),
    };

    let config = WatsonConfig::from_file(&config_file_path).unwrap();

    let arenas = Arenas::new();
    let (mut ctx, parse_report, proof_report) = check_command::check(config, &arenas);

    if ctx.diags.has_errors() {
        println!("{ANSI_RED}{ANSI_BOLD}Errors reported.{ANSI_RESET} Building book anyway.")
    }

    let book_path = book::build_book(&mut ctx, parse_report, proof_report, false);
    let port = ctx.config.book().port();
    book::server::serve(&book_path, port);
}
