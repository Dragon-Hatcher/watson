use std::collections::HashMap;

use crate::{
    diagnostics::{ReportTracker, WResult, specifics},
    parse::{
        common::parse_name,
        pattern::{Pattern, PatternId, PatternTy, parse_pattern},
        stream::{ParseError, ParseResult, Stream},
    },
    statements::{Statement, StatementId, StatementTy, StatementsSet},
};

#[derive(Debug, Default)]
pub struct PatternArena {
    in_stmt: HashMap<StatementId, Vec<PatternId>>,
    with_ty: HashMap<PatternTy, Vec<PatternId>>,
    all: HashMap<PatternId, Pattern>,
}

impl PatternArena {
    fn add(&mut self, pat: Pattern, in_stmt: StatementId) {
        self.in_stmt.entry(in_stmt).or_default().push(pat.id);
        self.with_ty.entry(pat.ty).or_default().push(pat.id);
        self.all.insert(pat.id, pat);
    }

    pub fn get(&self, id: &PatternId) -> &Pattern {
        &self.all[id]
    }

    pub fn patterns_for(&self, stmt_id: StatementId) -> &[PatternId] {
        self.in_stmt
            .get(&stmt_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn patterns_with_ty(&self, ty: PatternTy) -> &[PatternId] {
        self.with_ty.get(&ty).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

pub fn find_patterns(ss: &StatementsSet, tracker: &mut ReportTracker) -> WResult<PatternArena> {
    let mut arena = PatternArena::default();

    for s in ss.statements() {
        find_pattern_in_stmt(s, &mut arena, tracker);
    }

    tracker.checkpoint()?;
    Ok(arena)
}

fn find_pattern_in_stmt(s: &Statement, arena: &mut PatternArena, tracker: &mut ReportTracker) {
    let text = s.text().as_str();
    let mut str = Stream::new(text);

    let res = match s.ty() {
        StatementTy::Syntax => find_pattern_in_syntax(&mut str, s.id(), arena),
        StatementTy::Notation => find_pattern_in_notation(&mut str, s.id(), arena),
        StatementTy::Definition => find_pattern_in_definition(&mut str, s.id(), arena),

        StatementTy::Axiom | StatementTy::Theorem | StatementTy::Prose => return,
    };

    if let Err(e) = res {
        tracker.add_message(specifics::parse_error(e, s.span()));
    }
}

fn find_pattern_in_syntax(
    str: &mut Stream,
    stmt_id: StatementId,
    arena: &mut PatternArena,
) -> ParseResult<()> {
    str.commit(|str| {
        str.expect_str("syntax")?;

        let mut patterns = Vec::new();

        loop {
            match parse_pattern(str, PatternTy::Sentence) {
                Ok(p) => patterns.push(p),
                Err(ParseError::Backtrack(_)) => break,
                Err(ParseError::Commit(e)) => return Err(ParseError::Commit(e)),
            }
        }

        str.expect_str("end")?;

        for pattern in patterns {
            arena.add(pattern, stmt_id);
        }

        Ok(())
    })
}

fn find_pattern_in_notation(
    str: &mut Stream,
    stmt_id: StatementId,
    arena: &mut PatternArena,
) -> ParseResult<()> {
    str.commit(|str| {
        str.expect_str("notation")?;

        let _name = parse_name(str)?;
        let pattern = parse_pattern(str, PatternTy::Sentence)?;
        str.expect_str("=>")?;

        // Just go to the end. We can catch other syntax errors latter.
        while let Some(_) = str.pop() {}

        arena.add(pattern, stmt_id);

        Ok(())
    })
}

fn find_pattern_in_definition(
    str: &mut Stream,
    stmt_id: StatementId,
    arena: &mut PatternArena,
) -> ParseResult<()> {
    str.commit(|str| {
        str.expect_str("definition")?;

        let _name = parse_name(str)?;
        let pattern = parse_pattern(str, PatternTy::Value)?;
        str.expect_str("=>").or_else(|_| str.expect_str("where"))?;

        arena.add(pattern, stmt_id);

        Ok(())
    })
}
