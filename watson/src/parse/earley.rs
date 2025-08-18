use crate::{
    diagnostics::{DiagManager, WResult},
    parse::{
        Location, Span,
        macros::{MacroPat, MacroPatPart},
        parse_tree::{
            AtomPattern, MacroBindingKind, MacroBindingNode, ParseAtom, ParseAtomKind, ParseNode,
            ParseRule, ParseRuleId, ParseTree, PatternPart, SyntaxCategoryId,
        },
    },
};
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet, VecDeque},
};
use ustr::Ustr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EarleyItem {
    start_offset: Location,
    rule: ParseRuleId,
    pattern_pos: usize,
    can_template: bool,
}

impl EarleyItem {
    fn new(start_offset: Location, rule: ParseRuleId) -> Self {
        Self {
            start_offset,
            rule,
            pattern_pos: 0,
            can_template: false,
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
    end_offset: Option<Location>,
    category: SyntaxCategoryId,
    rules: &HashMap<ParseRuleId, ParseRule>,
    macro_pat: Option<&MacroPat>,
    can_template: bool,
    diags: &mut DiagManager,
) -> Option<ParseTree> {
    let (chart, full_template) = build_chart(
        text,
        start_offset,
        end_offset,
        category,
        rules,
        macro_pat,
        can_template,
    );
    let (span, trimmed_chart) = trim_chart(start_offset, category, rules, &chart);

    if !(trimmed_chart.contains_key(&(start_offset, category)) || full_template) {
        // The parse failed.
        create_error(&chart, rules, text, diags);
        return None;
    }

    Some(read_chart(
        text,
        span,
        category,
        rules,
        &trimmed_chart,
        macro_pat.is_none(),
    ))
}

fn build_chart(
    text: &str,
    start_offset: Location,
    end_offset: Option<Location>,
    category: SyntaxCategoryId,
    rules: &HashMap<ParseRuleId, ParseRule>,
    macro_pat: Option<&MacroPat>,
    can_template: bool,
) -> (HashMap<Location, HashSet<EarleyItem>>, bool) {
    let by_category = group_by_category(rules);

    let mut chart: HashMap<Location, HashSet<EarleyItem>> = HashMap::new();
    let mut creators: HashMap<(Location, SyntaxCategoryId), HashSet<EarleyItem>> = HashMap::new();

    // First we initialize the chart with all rules for the starting symbol.
    for rule in by_category[&category].iter().copied() {
        let mut item = EarleyItem::new(start_offset, rule);
        item.can_template = can_template;
        chart.entry(start_offset).or_default().insert(item);
    }

    let mut full_template = false;
    if can_template && let Some((name, _span)) = parse_macro_binding_at_offset(text, start_offset) {
        let expected_part = macro_pat.map(|pat| {
            let pos = pat.keys()[&name];
            pat.parts()[pos]
        });

        if expected_part.map_or(true, |e| e.matches_pat(PatternPart::TemplateCat(category))) {
            full_template = true;
        }
    }

    // We store the last position we need to analyze (inclusive). Since we don't
    // know how long the string we are parsing will be, this increases over time.
    let mut last_position = start_offset;

    // This tracks where in the source we are.
    let mut current_position = start_offset;

    while current_position.byte_offset() <= last_position.byte_offset()
        && end_offset.map_or(true, |e| current_position.byte_offset() <= e.byte_offset())
    {
        let Some(items) = chart.get(&current_position) else {
            // There were no items starting at this position so we move on.
            current_position = current_position.forward(1);
            continue;
        };
        let mut item_queue: VecDeque<EarleyItem> = items.iter().copied().collect();

        let macro_binding = parse_macro_binding_at_offset(text, current_position);

        while let Some(item) = item_queue.pop_front() {
            let pattern_pat = rules[&item.rule].pattern.get(item.pattern_pos);

            let can_template =
                item.can_template || matches!(pattern_pat, Some(PatternPart::TemplateCat(_)));
            if can_template && let Some((name, full_span)) = macro_binding {
                // There is a macro binding at the current position. We will use
                // this to match whichever component comes next.

                let expected_part = macro_pat.map(|pat| {
                    let pos = pat.keys()[&name];
                    pat.parts()[pos]
                });

                if let Some(pat) = rules[&item.rule].pattern.get(item.pattern_pos)
                    && expected_part.map_or(true, |e| e.matches_pat(*pat))
                {
                    // Advance past this section.
                    let end_pos = full_span.end();
                    let entry = chart.entry(end_pos).or_default();
                    entry.insert(item.advance());
                    last_position = last_position.max(&end_pos);
                }
            }

            match pattern_pat {
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
                Some(PatternPart::Category(cat) | PatternPart::TemplateCat(cat)) => {
                    // Predict. We add an item for this category at the current position.
                    for prediction in by_category[cat].iter().copied() {
                        let mut new_item = EarleyItem::new(current_position, prediction);

                        // Keep the template status of the current item, or if this is marked explicitly
                        // as a template switch to being a template.
                        new_item.can_template = can_template;

                        // Track that the current item created the new item here
                        creators
                            .entry((current_position, *cat))
                            .or_default()
                            .insert(item);

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
        }

        current_position = current_position.forward(1);
    }

    (chart, full_template)
}

fn create_error(
    chart: &HashMap<Location, HashSet<EarleyItem>>,
    rules: &HashMap<ParseRuleId, ParseRule>,
    text: &str,
    diags: &mut DiagManager,
) {
    // To find the error we want to find all the atoms that could have worked
    // at the last matched position.

    let last_pos = chart.keys().max_by_key(|l| l.byte_offset()).unwrap();
    let items = &chart[last_pos];
    let mut possible_atoms = HashSet::new();

    for item in items {
        let parts = &rules[&item.rule].pattern;
        if let Some(PatternPart::Atom(atom)) = parts.get(item.pattern_pos) {
            possible_atoms.insert(*atom);
        }
    }

    // Sort the atoms so we get a consistent ordering for error messages.
    let mut possible_atoms: Vec<_> = possible_atoms.into_iter().collect();
    possible_atoms.sort();

    let post_ws_pos = skip_ws_and_comments(text, *last_pos);
    let _: WResult<()> = diags.err_parse_failure(post_ws_pos, &possible_atoms);
}

fn _debug_chart(
    chart: &HashMap<Location, HashSet<EarleyItem>>,
    rules: &HashMap<ParseRuleId, ParseRule>,
) {
    let mut locations: Vec<Location> = chart.keys().copied().collect();
    locations.sort_by_key(|l| l.byte_offset());

    for l in locations {
        println!("{l:?}:");
        let mut items: Vec<_> = chart[&l].iter().collect();
        items.sort_by_key(|i| rules[&i.rule].cat);

        for item in items {
            let rule = &rules[&item.rule];
            print!("  {:?} ({:?}) ::= ", rule.cat, item.start_offset);
            for (i, part) in rule.pattern.iter().enumerate() {
                if i == item.pattern_pos {
                    print!("· ");
                }
                match part {
                    PatternPart::Atom(AtomPattern::Kw(kw)) => print!("\"{kw}\" "),
                    PatternPart::Atom(AtomPattern::Lit(lit)) => print!("\"{lit}\" "),
                    PatternPart::Atom(AtomPattern::Name) => print!("name "),
                    PatternPart::Atom(AtomPattern::Str) => print!("str "),
                    PatternPart::Category(cat) => print!("{cat:?} "),
                    PatternPart::TemplateCat(cat) => print!("temp({cat:?}) "),
                }
            }

            if item.pattern_pos == rule.pattern.len() {
                print!("·");
            }

            println!();
        }
        println!();
    }
}

fn trim_chart(
    start_offset: Location,
    target_cat: SyntaxCategoryId,
    rules: &HashMap<ParseRuleId, ParseRule>,
    chart: &HashMap<Location, HashSet<EarleyItem>>,
) -> (
    Span,
    HashMap<(Location, SyntaxCategoryId), Vec<(ParseRuleId, Span)>>,
) {
    let mut best_span = Span::new(start_offset, start_offset);
    let mut trimmed: HashMap<(Location, SyntaxCategoryId), Vec<(ParseRuleId, Span)>> =
        HashMap::new();

    for (&end_loc, items) in chart {
        for item in items {
            if item.pattern_pos != rules[&item.rule].pattern.len() {
                // This item didn't complete so we don't include it in the chart.
                continue;
            }

            let start_loc = item.start_offset;
            let cat = rules[&item.rule].cat;
            let span = Span::new(start_loc, end_loc);
            let entry = trimmed.entry((start_loc, cat)).or_default();
            entry.push((item.rule, span));

            if cat == target_cat
                && start_loc == start_offset
                && end_loc.byte_offset() > best_span.end().byte_offset()
            {
                best_span = span;
            }
        }
    }

    for (_, spans) in trimmed.iter_mut() {
        spans.sort_by_key(|s| Reverse(s.1.end().byte_offset()));
    }

    (best_span, trimmed)
}

fn read_chart(
    text: &str,
    span: Span,
    category: SyntaxCategoryId,
    rules: &HashMap<ParseRuleId, ParseRule>,
    chart: &HashMap<(Location, SyntaxCategoryId), Vec<(ParseRuleId, Span)>>,
    bindings_unchecked: bool,
) -> ParseTree {
    // Find all the matches that span the full given range and have the right
    // category.
    let candidates = chart.get(&(span.start(), category));

    // Then take only those candidates that end in the right spot.
    let mut candidates = candidates
        .into_iter()
        .flatten()
        .filter(|c| c.1.end() == span.end());

    // TODO: check all options and choose the best one.
    let Some((best_choice_rule_id, _)) = candidates.next() else {
        // If there is no possible rule application, there must be a macro binding
        // here. Otherwise the parse wouldn't have been valid.
        let (name, macro_span) = parse_macro_binding_at_offset(text, span.start())
            .expect("Recognizer must produce a valid parse.");
        return ParseTree::MacroBinding(MacroBindingNode {
            name,
            kind: MacroBindingKind::Cat(category),
            span: macro_span,
            is_unchecked: bindings_unchecked,
        });
    };

    let best_choice_rule = &rules[&best_choice_rule_id];

    fn find_path(
        text: &str,
        full_span: Span,
        search_stack: &mut Vec<Span>,
        pattern: &[PatternPart],
        chart: &HashMap<(Location, SyntaxCategoryId), Vec<(ParseRuleId, Span)>>,
    ) -> Option<Vec<Span>> {
        if let Some(last) = search_stack.last()
            && last.end().byte_offset() > full_span.end().byte_offset()
        {
            // This match is too long.
            return None;
        }

        if search_stack.len() == pattern.len() {
            // We have found a match. We already did things in the optimal order
            // so this is the match we want.
            return Some(search_stack.clone());
        }

        let at = search_stack
            .last()
            .map(Span::end)
            .unwrap_or(full_span.start());

        match pattern[search_stack.len()] {
            PatternPart::Atom(atom_pat) => {
                if let Some(atom) = parse_atom_at_offset(text, at, atom_pat) {
                    search_stack.push(atom.full_span);
                    let res = find_path(text, full_span, search_stack, pattern, chart);
                    search_stack.pop();

                    res
                } else if let Some((_name, macro_span)) = parse_macro_binding_at_offset(text, at) {
                    search_stack.push(macro_span);
                    let res = find_path(text, full_span, search_stack, pattern, chart);
                    search_stack.pop();

                    res
                } else {
                    None
                }
            }
            PatternPart::Category(cat) | PatternPart::TemplateCat(cat) => {
                if let Some(spans) = &chart.get(&(at, cat)) {
                    for (_, span) in *spans {
                        search_stack.push(*span);
                        let res = find_path(text, full_span, search_stack, pattern, chart);
                        search_stack.pop();

                        if res.is_some() {
                            return res;
                        }
                    }
                };

                // If none of the rules worked then let's try looking for a macro binding.
                if let Some((_name, macro_span)) = parse_macro_binding_at_offset(text, at) {
                    search_stack.push(macro_span);
                    let res = find_path(text, full_span, search_stack, pattern, chart);
                    search_stack.pop();

                    return res;
                }

                None
            }
        }
    }

    let mut search_stack: Vec<Span> = Vec::new();

    let spans = find_path(
        text,
        span,
        &mut search_stack,
        &best_choice_rule.pattern,
        chart,
    )
    .expect("Recognizer must produce a valid parse.");

    let mut children = Vec::new();
    for (span, pat) in spans.into_iter().zip(best_choice_rule.pattern.iter()) {
        let child = match pat {
            PatternPart::Atom(atom_pat) => {
                if let Some(atom) = parse_atom_at_offset(text, span.start(), *atom_pat) {
                    ParseTree::Atom(atom)
                } else if let Some((name, span)) = parse_macro_binding_at_offset(text, span.start())
                {
                    let node = MacroBindingNode {
                        name,
                        kind: MacroBindingKind::Atom(*atom_pat),
                        span,
                        is_unchecked: bindings_unchecked,
                    };
                    ParseTree::MacroBinding(node)
                } else {
                    unreachable!()
                }
            }
            PatternPart::Category(cat) | PatternPart::TemplateCat(cat) => {
                let child = read_chart(text, span, *cat, rules, chart, bindings_unchecked);
                child
            }
        };
        children.push(child);
    }

    ParseTree::Node(ParseNode {
        category,
        rule: *best_choice_rule_id,
        has_unchecked_bindings: children.iter().any(|c| c.has_unchecked_bindings()),
        children,
        span,
    })
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
            let (name, end) = parse_name(text, content_offset)?;

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
            let (name, end) = parse_name(text, content_offset)?;

            let full_span = Span::new(start, end);
            let content_span = Span::new(content_offset, end);
            Some(ParseAtom {
                full_span,
                content_span,
                kind: ParseAtomKind::Name(name),
            })
        }
        AtomPattern::Str => {
            let content_text = &text[content_offset.byte_offset()..];

            let mut end = content_offset;
            let mut chars = content_text.chars();

            if let Some('"') = chars.next() {
                end = end.forward('"'.len_utf8());
            } else {
                return None;
            }

            for char in chars {
                end = end.forward(char.len_utf8());

                if char == '"' {
                    let full_span = Span::new(start, end);
                    let content_span = Span::new(content_offset, end);
                    let inner_text = &text[content_span.start().byte_offset() + 1
                        ..content_span.end().byte_offset() - 1];
                    let inner_text = Ustr::from(inner_text);
                    return Some(ParseAtom {
                        full_span,
                        content_span,
                        kind: ParseAtomKind::Str(inner_text),
                    });
                }

                if char.is_ascii_whitespace() {
                    return None;
                }
            }

            // No end quote so return false.
            None
        }
    }
}

fn parse_macro_binding_at_offset(text: &str, start: Location) -> Option<(Ustr, Span)> {
    let content_offset = skip_ws_and_comments(text, start);

    if text[content_offset.byte_offset()..].chars().next() != Some('$') {
        return None;
    }

    let (name, end) = parse_name(text, content_offset.forward(1))?;

    let full_span = Span::new(start, end);
    Some((name, full_span))
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
    char.is_alphabetic() || char == '.' || char == '_' || char == '\''
}

fn char_can_continue_name(char: char) -> bool {
    char_can_start_name(char) || char.is_numeric() || char == '\''
}

pub fn find_start_keywords(
    root: SyntaxCategoryId,
    rules: &HashMap<ParseRuleId, ParseRule>,
) -> HashSet<Ustr> {
    fn search<'a>(
        cat: SyntaxCategoryId,
        start_keywords: &'a mut HashMap<SyntaxCategoryId, HashSet<Ustr>>,
        rules: &HashMap<ParseRuleId, ParseRule>,
        by_category: &HashMap<SyntaxCategoryId, Vec<ParseRuleId>>,
    ) {
        if start_keywords.contains_key(&cat) {
            return;
        }

        start_keywords.insert(cat, HashSet::new());

        let mut set = HashSet::new();

        for rule in by_category.get(&cat).unwrap_or(&Vec::new()) {
            match &rules[rule].pattern[0] {
                // TODO
                PatternPart::Atom(atom) => match atom {
                    AtomPattern::Kw(ustr) => {
                        set.insert(*ustr);
                    }
                    AtomPattern::Name | AtomPattern::Lit(_) | AtomPattern::Str => todo!(),
                },
                PatternPart::Category(cat) | PatternPart::TemplateCat(cat) => {
                    search(*cat, start_keywords, rules, by_category);
                    set.extend(&start_keywords[&cat]);
                }
            }
        }

        start_keywords.insert(cat, set);
    }

    let by_category = group_by_category(rules);
    let mut start_keywords = HashMap::new();
    search(root, &mut start_keywords, rules, &by_category);

    start_keywords.remove(&root).unwrap()
}
