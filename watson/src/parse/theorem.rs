use crate::parse::{
    common::{parse_hypotheses, parse_name, parse_templates, Template},
    sentence::{parse_sentence, Sentence},
    tactic::{parse_tactics, Tactic},
    utils::ws_nl, Stream,
};
use tap::Pipe;
use ustr::Ustr;
use winnow::{Parser, combinator::seq};

pub struct Theorem {
    name: Ustr,
    templates: Vec<Template>,
    hypotheses: Vec<Sentence>,
    conclusion: Sentence,
    proof: Vec<Tactic>,
}

pub fn parse_theorem(str: &mut Stream) -> winnow::ModalResult<Theorem> {
    seq! {Theorem{
        _: "theorem".pipe(ws_nl),
        name: parse_name,
        templates: parse_templates,
        _: ":".pipe(ws_nl),
        hypotheses: parse_hypotheses,
        _: "|-".pipe(ws_nl),
        conclusion: parse_sentence,
        _: "proof".pipe(ws_nl),
        proof: parse_tactics,
        _: "qed".pipe(ws_nl)
    }}
    .parse_next(str)
}
