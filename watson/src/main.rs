use std::{io};
use ustr::Ustr;

use crate::{parser::parse, span::SourceCache};

mod parser;
mod span;
mod util;

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = std::env::args().collect();
    let file = &args[1];

    let mut sources = SourceCache::new();
    sources.add_file(Ustr::from(file));

    parse(&sources, Ustr::from(file));

    Ok(())
}
