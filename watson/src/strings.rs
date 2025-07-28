use std::sync::LazyLock;
use ustr::Ustr;

macro_rules! str_const {
    ($($name:ident = $str:literal);*; ) => {
        $(pub static $name: LazyLock<Ustr> = LazyLock::new(|| Ustr::from($str));)*
    };
}

str_const! {
    MODULE = "module";
}
