use std::{fs::read_to_string, io};

fn main() -> Result<(), io::Error> {
    let args: Vec<String> = std::env::args().collect();
    let file = &args[1];

    let text = read_to_string(file)?;
    println!("{text}");

    Ok(())
}
