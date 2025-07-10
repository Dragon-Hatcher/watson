use crate::parse::{
    Stream,
    common::{Template, parse_hypotheses, parse_name, parse_templates},
    sentence::{Sentence, parse_sentence},
    utils::ws_nl,
};
use tap::Pipe;
use ustr::Ustr;
use winnow::{Parser, combinator::seq};

pub struct Axiom {
    name: Ustr,
    templates: Vec<Template>,
    hypotheses: Vec<Sentence>,
    conclusion: Sentence,
}

pub fn parse_axiom(str: &mut Stream) -> winnow::ModalResult<Axiom> {
    seq! {Axiom{
        _: "theorem".pipe(ws_nl),
        name: parse_name,
        templates: parse_templates,
        _: ":".pipe(ws_nl),
        hypotheses: parse_hypotheses,
        _: "|-".pipe(ws_nl),
        conclusion: parse_sentence,
        _: "end".pipe(ws_nl)
    }}
    .parse_next(str)
}
