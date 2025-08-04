use std::sync::LazyLock;
use ustr::Ustr;

macro_rules! str_const {
    ($($name:ident = $str:literal);*; ) => {
        $(pub static $name: LazyLock<Ustr> = LazyLock::new(|| Ustr::from($str));)*
    };
}

// Keywords:
str_const! {
    END = "end";
    MODULE = "module";
    SYNTAX_CAT = "syntax_category";
    SYNTAX = "syntax";
    MACRO = "macro";
}

// Symbols:
str_const! {
    BNF_REPLACE = "::=";
    FAT_ARROW = "=>";
}

str_const! {
    SENTENCE = "sentence";
    BINDING = "binding";
    VARIABLE = "variable";
}

str_const! {
    FILE_EXTENSION = "wats";
}
