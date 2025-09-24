pub fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

// pub mod ansi {
//     pub const ANSI_RESET: &str = "\x1b[0m";
//     pub const ANSI_RED: &str = "\x1b[31m";
//     pub const ANSI_GREEN: &str = "\x1b[32m";
//     pub const ANSI_YELLOW: &str = "\x1b[33m";
//     pub const ANSI_BLUE: &str = "\x1b[34m";
//     pub const ANSI_BOLD: &str = "\x1b[1m";
// }
