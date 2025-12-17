use crate::{
    context::Ctx,
    parse::ParseReport,
    report::ProofReport,
    util::ansi::{ANSI_BOLD, ANSI_GREEN, ANSI_RESET},
};
use std::fs;

pub fn build_book<'ctx>(
    ctx: &Ctx<'ctx>,
    _parse_report: ParseReport<'ctx>,
    _proof_report: ProofReport<'ctx>,
) {
    let book_dir = ctx.config.build_dir().join("book");
    fs::create_dir_all(&book_dir).unwrap();

    let index_html = book_dir.join("index.html");
    fs::write(&index_html, "<h1>Hello, World!</h1>").unwrap();

    let full_path = index_html.canonicalize().unwrap();
    println!(
        "{ANSI_GREEN}{ANSI_BOLD}Created book{ANSI_RESET} at {}",
        full_path.display()
    )
}
