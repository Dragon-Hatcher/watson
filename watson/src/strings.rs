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
    MACRO_RULE = "macro_rule";
    AXIOM = "axiom";
}

// Symbols:
str_const! {
    BNF_REPLACE = "::=";
    FAT_ARROW = "=>";
    AT = "@";
    DOLLAR = "$";
    COLON = ":";
    TURNSTILE = "|-";
    LEFT_PAREN = "(";
    RIGHT_PAREN = ")";
    LEFT_BRACKET = "[";
    RIGHT_BRACKET = "]";
    COMMA = ",";
}

str_const! {
    SENTENCE = "sentence";
    BINDING = "binding";
    VARIABLE = "variable";
    KW = "kw";
    NAME = "name";
    TEMPLATE = "template";
}

str_const! {
    FILE_EXTENSION = "wats";
}
