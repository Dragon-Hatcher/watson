use crate::parse::{sentence::Sentence, utils::ws_nl, Stream};
use tap::Pipe;
use ustr::Ustr;
use winnow::{
    Parser,
    stream::AsChar,
    token::{one_of, take_while},
};

pub enum Template {
    Var { fresh: bool, name: Ustr },
    Schema { args: u32, name: Ustr },
}

pub fn parse_templates(str: &mut Stream) -> winnow::ModalResult<Vec<Template>> {
    todo!()
}

pub fn parse_hypotheses(str: &mut Stream) -> winnow::ModalResult<Vec<Sentence>> {
    todo!()
}

pub fn parse_name(str: &mut Stream) -> winnow::ModalResult<Ustr> {
    (
        one_of(|c: char| c.is_alpha() || c == '_'),
        take_while(0.., |c: char| c.is_alphanum() || c == '_'),
    )
        .take()
        .map(Ustr::from)
        .pipe(ws_nl)
        .parse_next(str)
}
