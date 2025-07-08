use std::{fs::read_to_string, io};

use crate::parser::parse;

mod parser;
mod util;

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = std::env::args().collect();
    let file = &args[1];

    let text = read_to_string(file)?;
    parse(&text);

    Ok(())
}
