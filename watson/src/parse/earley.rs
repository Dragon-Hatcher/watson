use ustr::Ustr;

use crate::parse::{
    Location, SourceOffset, Span,
    parse_tree::{
        AtomPattern, ParseAtom, ParseAtomKind, ParseRule, ParseRuleId, ParseTree, SyntaxCategoryId,
    },
};
use std::collections::HashMap;

pub fn parse_category(
    text: &str,
    offset: Location,
    category: SyntaxCategoryId,
    rules: &HashMap<ParseRuleId, ParseRule>,
) -> ParseTree {
    todo!()
}

fn parse_atom_at_offset(text: &str, start: Location, atom: AtomPattern) -> Option<ParseAtom> {
    let content_offset = skip_ws_and_comments(text, start);

    match atom {
        AtomPattern::Lit(lit) => {
            if !text[content_offset.byte_offset()..].starts_with(lit.as_str()) {
                return None;
            }

            let end = content_offset.forward(lit.len());
            let full_span = Span::new(start, end);
            let content_span = Span::new(content_offset, end);

            Some(ParseAtom {
                full_span,
                content_span,
                kind: ParseAtomKind::Lit(lit),
            })
        }
        AtomPattern::Kw(kw) => {
            let (name, end) = parse_name(text, start)?;

            if name != kw {
                return None;
            }

            let full_span = Span::new(start, end);
            let content_span = Span::new(content_offset, end);
            Some(ParseAtom {
                full_span,
                content_span,
                kind: ParseAtomKind::Kw(kw),
            })
        }
        AtomPattern::Name => {
            let (name, end) = parse_name(text, start)?;

            let full_span = Span::new(start, end);
            let content_span = Span::new(content_offset, end);
            Some(ParseAtom {
                full_span,
                content_span,
                kind: ParseAtomKind::Name(name),
            })
        }
    }
}

fn skip_ws_and_comments(text: &str, mut offset: Location) -> Location {
    while let Some(next) = text[offset.byte_offset()..].chars().next() {
        if next.is_ascii_whitespace() {
            offset = offset.forward(next.len_utf8());
            continue;
        }

        if next == '-'
            && let Some('-') = text[offset.byte_offset() + 1..].chars().next()
        {
            while let Some(next) = text[offset.byte_offset()..].chars().next() {
                offset = offset.forward(next.len_utf8());
                if next == '\n' {
                    break;
                }
            }

            continue;
        }

        break;
    }

    offset
}

pub fn parse_name(text: &str, mut offset: Location) -> Option<(Ustr, Location)> {
    let mut chars = text[offset.byte_offset()..].chars();
    let mut name = String::new();

    let first = chars.next()?;

    if !char_can_start_name(first) {
        return None;
    }

    name.push(first);
    offset = offset.forward(first.len_utf8());

    while let Some(next) = chars.next()
        && char_can_continue_name(next)
    {
        name.push(next);
        offset = offset.forward(next.len_utf8());
    }

    Some((Ustr::from(&name), offset))
}

fn char_can_start_name(char: char) -> bool {
    char.is_alphabetic() || char == '.'
}

fn char_can_continue_name(char: char) -> bool {
    char_can_start_name(char) || char.is_numeric() || char == '\''
}
