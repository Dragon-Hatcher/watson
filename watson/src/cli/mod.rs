use crate::cli::{
    book_command::{BookCommand, run_book},
    check_command::{CheckCommand, run_check},
    new_command::{NewCommand, run_new},
};
use argh::FromArgs;

mod book_command;
mod check_command;
mod new_command;

/// The Watson proof assistant.
#[derive(FromArgs)]
struct Args {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Command {
    New(NewCommand),
    Check(CheckCommand),
    Book(BookCommand),
}

pub fn run_cli() {
    let args: Args = argh::from_env();

    match args.command {
        Command::New(cmd) => run_new(cmd),
        Command::Check(cmd) => run_check(cmd),
        Command::Book(cmd) => run_book(cmd),
    }
}
