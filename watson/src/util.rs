use convert_case::ccase;

pub fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

pub fn name_to_lua(name: &str) -> String {
    ccase!(snake -> pascal, name.replace('.', "_"))
}

pub mod ansi {
    pub const ANSI_RESET: &str = "\x1b[0m";
    pub const ANSI_RED: &str = "\x1b[31m";
    pub const ANSI_GREEN: &str = "\x1b[32m";
    pub const ANSI_YELLOW: &str = "\x1b[33m";
    pub const ANSI_GRAY: &str = "\x1b[90m";
    pub const ANSI_BOLD: &str = "\x1b[1m";
    pub const ANSI_UNDERLINE: &str = "\x1b[4m";
}
