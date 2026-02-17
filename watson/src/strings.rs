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
    GRAMMAR_CATEGORY = "grammar_category";
    TACTIC = "tactic";
    NAME = "name";
    KW = "kw";
    PROOF = "proof";
    QED = "qed";
    ASSUME = "assume";
    FRAGMENT = "fragment";
    ANY_FRAGMENT = "any_fragment";
    FACT = "fact";
}

// Symbols:
str_const! {
    BNF_REPLACE = "::=";
    ASSIGN = ":=";
    AT = "@";
    COLON = ":";
    COMMA = ",";
    TURNSTILE = "|-";
    LEFT_PAREN = "(";
    RIGHT_PAREN = ")";
    LEFT_BRACKET = "[";
    RIGHT_BRACKET = "]";
    LEFT_ARROW = "<";
    RIGHT_ARROW = ">";
}

// Lua names:
str_const! {
    RESERVED_RULE = "_rule";
    RESERVED_SPAN = "_span";
    SPANNED_STRING = "SpannedString";
    UN_FRAG = "UnResFrag";
    UN_ANY_FRAG = "UnResAnyFrag";
    UN_FACT = "UnResFact";
}

str_const! {
    SENTENCE = "sentence";
    BINDING = "binding";
    ATTRIBUTE = "attribute";
}

str_const! {
    FILE_EXTENSION = "wats";
}
