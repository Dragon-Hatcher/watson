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
    NOTATION = "notation";
    DEFINITION = "definition";
    AXIOM = "axiom";
    THEOREM = "theorem";
    NAME = "name";
    KW = "kw";
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
    ASSIGN = ":=";
    AT = "@";
    COLON = ":";
    TURNSTILE = "|-";
    LEFT_PAREN = "(";
    RIGHT_PAREN = ")";
    LEFT_BRACKET = "[";
    RIGHT_BRACKET = "]";
    LEFT_ARROW = "<";
    RIGHT_ARROW = ">";
    COMMA = ",";
    SEMICOLON = ";";
}

str_const! {
    SENTENCE = "sentence";
    BINDING = "binding";
    VARIABLE = "variable";
}

str_const! {
    FILE_EXTENSION = "wats";
}
