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
    AXIOM = "axiom";
    THEOREM = "theorem";
    PROOF = "proof";
    QED = "qed";
    ASSUME = "assume";
    BY = "by";
    HAVE = "have";
    TODO = "todo";
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
    SEMICOLON = ";";
}

str_const! {
    SENTENCE = "sentence";
    BINDING = "binding";
    VARIABLE = "variable";
    KW = "kw";
    TEMPLATE = "template";
}

str_const! {
    FILE_EXTENSION = "wats";
}
