mod book;
mod cli;
mod config;
mod context;
mod diagnostics;
mod parse;
mod report;
mod semant;
mod strings;
mod util;

fn main() {
    cli::run_cli();
}
