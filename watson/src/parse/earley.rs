use crate::parse::{
    Location, Span,
    parse_tree::{
        AtomPattern, ParseAtom, ParseAtomKind, ParseRule, ParseRuleId, ParseTree, PatternPart,
        SyntaxCategoryId,
    },
};
use std::collections::{HashMap, HashSet, VecDeque};
use ustr::Ustr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EarleyItem {
    start_offset: Location,
    rule: ParseRuleId,
    pattern_pos: usize,
}

impl EarleyItem {
    fn new(start_offset: Location, rule: ParseRuleId) -> Self {
        Self {
            start_offset,
            rule,
            pattern_pos: 0,
        }
    }

    fn advance(self) -> Self {
        Self {
            pattern_pos: self.pattern_pos + 1,
            ..self
        }
    }
}

pub fn parse_category(
    text: &str,
    start_offset: Location,
    category: SyntaxCategoryId,
    rules: &HashMap<ParseRuleId, ParseRule>,
) -> ParseTree {
    let by_category = group_by_category(rules);

    let mut chart: HashMap<Location, HashSet<EarleyItem>> = HashMap::new();
    let mut creators: HashMap<(Location, SyntaxCategoryId), HashSet<EarleyItem>> = HashMap::new();

    // First we initialize the chart with all rules for the starting symbol.
    for rule in by_category[&category].iter().copied() {
        let item = EarleyItem::new(start_offset, rule);
        chart.entry(start_offset).or_default().insert(item);
    }

    // We store the last position we need to analyze (inclusive). Since we don't
    // know how long the string we are parsing will be, this increases over time.
    let mut last_position = start_offset;

    // This tracks where in the source we are.
    let mut current_position = start_offset;

    while current_position.byte_offset() <= last_position.byte_offset() {
        let Some(items) = chart.get(&current_position) else {
            // There were no items starting at this position so we move on.
            continue;
        };
        let mut item_queue: VecDeque<EarleyItem> = items.iter().copied().collect();

        while let Some(item) = item_queue.pop_front() {
            match rules[&item.rule].pattern.get(item.pattern_pos) {
                Some(PatternPart::Atom(atom_pat)) => {
                    // Scan. We look for this atom at our current position.
                    let Some(atom) = parse_atom_at_offset(text, current_position, *atom_pat) else {
                        // We didn't find the atom we were locking for so this
                        // item can't match. We drop it and move on.
                        continue;
                    };

                    let end_pos = atom.full_span.end();
                    let entry = chart.entry(end_pos).or_default();
                    entry.insert(item.advance());
                    last_position = last_position.max(&end_pos);
                }
                Some(PatternPart::Category(cat)) => {
                    // Predict. We add an item for this category at the current position.
                    for prediction in by_category[cat].iter().copied() {
                        let new_item = EarleyItem::new(current_position, prediction);

                        // Track that the current item created the new item here
                        creators
                            .entry((current_position, *cat))
                            .or_default()
                            .insert(new_item);

                        // If this is a new item we need to consider it at our current position.
                        if chart.entry(current_position).or_default().insert(new_item) {
                            item_queue.push_back(new_item);
                        }
                    }
                }
                None => {
                    // Complete. Notify whoever created this item that it has matched.
                    let category = rules[&item.rule].cat;
                    let Some(creators) = creators.get(&(item.start_offset, category)) else {
                        continue;
                    };

                    for creator in creators {
                        let new_item = creator.advance();

                        // If this is a new item we need to consider it at our current position.
                        if chart.entry(current_position).or_default().insert(new_item) {
                            item_queue.push_back(new_item);
                        }
                    }
                }
            }

            current_position = current_position.forward(1);
        }
    }

    todo!()
}

fn group_by_category(
    rules: &HashMap<ParseRuleId, ParseRule>,
) -> HashMap<SyntaxCategoryId, Vec<ParseRuleId>> {
    let mut map: HashMap<SyntaxCategoryId, Vec<ParseRuleId>> = HashMap::new();

    for (rule_id, rule) in rules {
        map.entry(rule.cat).or_default().push(*rule_id);
    }

    map
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
