use crate::{
    context::Ctx,
    diagnostics::{Diagnostic, WResult},
    parse::{
        Location, SourceId, Span,
        location::SourceOffset,
        parse_state::{Associativity, CategoryId, ParseAtomPattern, RuleId, RulePatternPart},
        parse_tree::{
            ParseAtom, ParseAtomKind, ParseTree, ParseTreeChildren, ParseTreeId, ParseTreePart,
        },
    },
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{char, cmp::Reverse, collections::VecDeque};

pub fn parse<'ctx>(
    start: Location,
    category: CategoryId<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, ParseTreeId<'ctx>> {
    let chart = build_chart(start, category, ctx);
    let trimmed = trim_chart(&chart);

    if !trimmed.contains_key(&(start.offset(), category)) {
        return make_parse_error(&chart, start.source(), ctx);
    }

    read_chart(start, category, &trimmed, ctx)
}

fn build_chart<'ctx>(start: Location, category: CategoryId<'ctx>, ctx: &Ctx<'ctx>) -> Chart<'ctx> {
    let text = ctx.sources.get_text(start.source()).as_str();
    let mut chart = Chart::new(start.offset());

    // Add all the start rules for the category we are parsing.
    for &rule in ctx.parse_state.rules_for_cat(category) {
        let item = Item::new(rule, start.offset());
        chart.add_item(item, start.offset());
    }

    // This tracks where in the source we are.
    let mut current_position = start.offset();

    while let Some(items) = chart.get_items(current_position) {
        let mut items: VecDeque<Item> = items.iter().copied().collect();

        while let Some(item) = items.pop_front() {
            let next_part = item.rule.0.pattern().parts().get(item.dot);

            match next_part {
                Some(RulePatternPart::Atom(atom)) => {
                    // Scan. Look for the atom at the current position.
                    let Some(atom_end) = parse_atom(*atom, text, current_position) else {
                        // We didn't find the atom we were looking for so this
                        // item can't match. We drop it and move on.
                        continue;
                    };

                    // Add the item wherever the atom ended.
                    chart.add_item(item.advance(), atom_end);
                }
                Some(RulePatternPart::Cat(cat)) => {
                    // Predict. Add all the rules for the category at the current position.
                    for &prediction in ctx.parse_state.rules_for_cat(*cat) {
                        let new_item = Item::new(prediction, current_position);
                        if chart.add_item(new_item, current_position) {
                            // This is a new item, so we need to process it.
                            items.push_back(new_item);
                        }
                    }

                    // If this item is nullable add the completion for it right away
                    if ctx.parse_state.can_be_empty(*cat) {
                        let new_item = item.advance();
                        if chart.add_item(new_item, current_position) {
                            // This is a new item, so we need to process it.
                            items.push_back(new_item);
                        }
                    }

                    // We also need to note that this item is waiting for the category
                    // to complete.
                    chart.wait_for_completion(current_position, *cat, item);
                }
                None => {
                    // Complete. We have reached the end of this rule, so we need to
                    // find all the items that were waiting for this rule to complete.
                    let Some(waiters) = chart.get_waiters(item.origin, item.rule.cat()) else {
                        // No one was waiting for this rule to complete, so we can
                        // just move on.
                        continue;
                    };

                    for waiter in waiters.clone() {
                        let new_item = waiter.advance();
                        if chart.add_item(new_item, current_position) {
                            // This is a new item, so we need to process it.
                            items.push_back(new_item);
                        }
                    }
                }
            }
        }

        current_position = current_position.forward(1);
    }

    chart
}

fn _debug_chart(chart: &Chart) {
    for (i, items) in chart.items_at_offset.iter().enumerate() {
        if items.is_empty() {
            continue;
        }

        let pos = chart.start_offset.forward(i);
        println!("At {pos:?}:");
        for item in items {
            let mut pattern = String::new();
            for (j, part) in item.rule.pattern().parts().iter().enumerate() {
                if j == item.dot {
                    pattern.push_str("• ");
                }
                match part {
                    RulePatternPart::Atom(atom) => match atom {
                        ParseAtomPattern::Kw(kw) => pattern.push_str(&format!("\"{kw}\" ")),
                        ParseAtomPattern::Name => pattern.push_str("name "),
                        ParseAtomPattern::Lit(lit) => pattern.push_str(&format!("'{lit}' ")),
                        ParseAtomPattern::Str => pattern.push_str("str "),
                        ParseAtomPattern::Num => pattern.push_str("num "),
                    },
                    RulePatternPart::Cat(id) => {
                        let cat_name = id._name();
                        pattern.push_str(&format!("<{cat_name}> "));
                    }
                }
            }
            if item.dot == item.rule.pattern().parts().len() {
                pattern.push('•');
            }
            let origin = item.origin;
            println!(
                "  {} -> {} (from {:?})",
                item.rule.cat()._name(),
                pattern,
                origin
            );
        }

        println!();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Item<'ctx> {
    rule: RuleId<'ctx>,
    dot: usize,
    origin: SourceOffset,
}

impl<'ctx> Item<'ctx> {
    fn new(rule: RuleId<'ctx>, origin: SourceOffset) -> Self {
        Self {
            rule,
            dot: 0,
            origin,
        }
    }

    fn advance(self) -> Self {
        Self {
            dot: self.dot + 1,
            ..self
        }
    }
}

struct Chart<'ctx> {
    start_offset: SourceOffset,
    items_at_offset: Vec<FxHashSet<Item<'ctx>>>,
    waiting: FxHashMap<(SourceOffset, CategoryId<'ctx>), FxHashSet<Item<'ctx>>>,
}

impl<'ctx> Chart<'ctx> {
    fn new(start_offset: SourceOffset) -> Self {
        Self {
            start_offset,
            items_at_offset: vec![],
            waiting: FxHashMap::default(),
        }
    }

    fn idx_for_offset(&self, offset: SourceOffset) -> usize {
        offset.byte_offset() - self.start_offset.byte_offset()
    }

    fn ensure_offset(&mut self, offset: SourceOffset) {
        let index = self.idx_for_offset(offset);
        while self.items_at_offset.len() <= index {
            self.items_at_offset.push(FxHashSet::default());
        }
    }

    fn add_item(&mut self, item: Item<'ctx>, pos: SourceOffset) -> bool {
        self.ensure_offset(pos);
        let index = self.idx_for_offset(pos);
        self.items_at_offset[index].insert(item)
    }

    fn wait_for_completion(&mut self, at: SourceOffset, cat: CategoryId<'ctx>, item: Item<'ctx>) {
        self.waiting.entry((at, cat)).or_default().insert(item);
    }

    fn get_items(&self, at: SourceOffset) -> Option<&FxHashSet<Item<'ctx>>> {
        let index = self.idx_for_offset(at);
        self.items_at_offset.get(index)
    }

    fn get_waiters(
        &self,
        at: SourceOffset,
        cat: CategoryId<'ctx>,
    ) -> Option<&FxHashSet<Item<'ctx>>> {
        self.waiting.get(&(at, cat))
    }
}

fn make_parse_error<'ctx, T>(chart: &Chart, source: SourceId, ctx: &Ctx<'ctx>) -> WResult<'ctx, T> {
    let latest_pos = chart.start_offset.forward(chart.items_at_offset.len() - 1);
    let latest_items = chart.items_at_offset.last().unwrap();

    let mut possible_next_atoms = FxHashSet::default();

    for item in latest_items {
        let pat = item.rule.pattern();
        if let Some(RulePatternPart::Atom(atom)) = pat.parts().get(item.dot) {
            possible_next_atoms.insert(*atom);
        }
    }

    let text = ctx.sources.get_text(source).as_str();
    let location = skip_ws_and_comments(text, latest_pos);
    let location = Location::new(source, location);

    let mut possible_atoms = possible_next_atoms.into_iter().collect::<Vec<_>>();
    possible_atoms.sort();

    Diagnostic::err_parse_failure(
        Location::new(location.source(), latest_pos),
        &possible_atoms,
    )
}

fn read_chart<'ctx>(
    start: Location,
    cat: CategoryId<'ctx>,
    chart: &TrimmedChart<'ctx>,
    ctx: &Ctx<'ctx>,
) -> WResult<'ctx, ParseTreeId<'ctx>> {
    // First we are going to find the length of the longest parse. Our recursive
    // function needs to know the full span so we assume the longest span is the
    // correct one.
    let end = chart[&(start.offset(), cat)]
        .iter()
        .map(|(_, end)| end.byte_offset())
        .max()
        .unwrap();
    let span = Span::new(start, Location::new(start.source(), SourceOffset::new(end)));

    // Now we can recursively read the parse tree.
    fn search<'ctx>(
        span: Span,
        cat: CategoryId<'ctx>,
        chart: &TrimmedChart<'ctx>,
        ctx: &Ctx<'ctx>,
    ) -> WResult<'ctx, ParseTreeId<'ctx>> {
        let text = ctx.sources.get_text(span.source()).as_str();

        // The idea here is to check which rules we have for the given span and
        // category. We then choose which among those rules is best. If there
        // is still a tie the parse is ambiguous.
        let rules = chart[&(span.start().offset(), cat)]
            .iter()
            .filter(|(_, end)| *end == span.end().offset())
            .map(|(rule, _end)| *rule);
        let best_rules = choose_best_rule(rules);

        // Each rule gives us a pattern which we can use to split the span
        // into parts. We then recursively search for each part.
        let mut possibilities = Vec::new();
        for &rule in &best_rules {
            let split = split_with_pattern(
                text,
                span,
                rule.pattern().parts(),
                rule.pattern().associativity(),
                &mut vec![],
                chart,
            );

            match split {
                Ok(split) => {
                    let children = split_to_children(
                        rule.0.pattern().parts(),
                        &split,
                        span.start(),
                        chart,
                        ctx,
                    )?;
                    possibilities.push(ParseTreeChildren::new(rule, children));
                }
                // We don't allow any ambiguity within a single rule, only between
                // rules. So this is an immediate error.
                Err(SplitError::Ambiguous) => return Diagnostic::err_ambiguous_parse(span),
                // The chart guaranteed that there is at least one match so
                // this should never happen.
                Err(SplitError::NoMatch) => unreachable!(),
            }
        }

        // For the parse tree don't include any whitespace in the span.
        let start = skip_ws_and_comments(text, span.start().offset());
        let start = Location::new(span.source(), start);
        let span = Span::new(start, span.end());

        let tree = ParseTree::new(span, cat, possibilities);
        Ok(ctx.arenas.parse_forest.intern(tree))
    }

    fn split_to_children<'ctx>(
        pattern: &[RulePatternPart<'ctx>],
        offsets: &[SourceOffset],
        start: Location,
        chart: &TrimmedChart<'ctx>,
        ctx: &Ctx<'ctx>,
    ) -> WResult<'ctx, Vec<ParseTreePart<'ctx>>> {
        debug_assert_eq!(pattern.len(), offsets.len());

        let text = ctx.sources.get_text(start.source()).as_str();

        let mut start = start;
        let mut parts = Vec::new();
        for (pat, offset) in pattern.iter().zip(offsets.iter()) {
            let span = Span::new(start, Location::new(start.source(), *offset));

            match pat {
                RulePatternPart::Atom(atom_pat) => {
                    let kind = match atom_pat {
                        ParseAtomPattern::Kw(kw) => ParseAtomKind::Kw(*kw),
                        ParseAtomPattern::Name => {
                            let start = skip_ws_and_comments(text, span.start().offset());
                            let name = &text[start.byte_offset()..span.end().byte_offset()];
                            ParseAtomKind::Name(name.into())
                        }
                        ParseAtomPattern::Lit(lit) => ParseAtomKind::Lit(*lit),
                        ParseAtomPattern::Str => {
                            let start = skip_ws_and_comments(text, span.start().offset());
                            let name = &text[start.byte_offset() + 1..span.end().byte_offset() - 1];
                            ParseAtomKind::StrLit(name.into())
                        }
                        ParseAtomPattern::Num => {
                            let start = skip_ws_and_comments(text, span.start().offset());
                            let num = &text[start.byte_offset()..span.end().byte_offset()];
                            ParseAtomKind::Num(num.parse().unwrap())
                        }
                    };
                    let no_ws_start = skip_ws_and_comments(text, span.start().offset());
                    let no_ws_span =
                        Span::new(Location::new(span.source(), no_ws_start), span.end());
                    let atom = ParseAtom::new(span, no_ws_span, kind);
                    parts.push(ParseTreePart::Atom(atom));
                }
                RulePatternPart::Cat(id) => {
                    let tree_id = search(span, *id, chart, ctx)?;
                    parts.push(ParseTreePart::Node {
                        id: tree_id,
                        span: tree_id.span(),
                        cat: *id,
                    });
                }
            }

            start = Location::new(start.source(), *offset);
        }

        Ok(parts)
    }

    search(span, cat, chart, ctx)
}

fn choose_best_rule<'ctx>(rules: impl Iterator<Item = RuleId<'ctx>>) -> Vec<RuleId<'ctx>> {
    // We pick the rules with the lowest precedence value.
    let mut best_rules = Vec::new();
    let mut best_precedence = None;

    for rule in rules {
        let precedence = rule.pattern().precedence();
        if best_precedence.is_none_or(|bp| precedence < bp) {
            best_precedence = Some(precedence);
            best_rules = vec![rule];
        } else if best_precedence == Some(precedence) {
            best_rules.push(rule);
        }
    }

    best_rules
}

enum SplitError {
    NoMatch,
    Ambiguous,
}

fn split_with_pattern(
    text: &str,
    span: Span,
    pattern: &[RulePatternPart],
    associativity: Associativity,
    stack: &mut Vec<SourceOffset>,
    chart: &TrimmedChart,
) -> Result<Vec<SourceOffset>, SplitError> {
    let at = stack.last().copied().unwrap_or(span.start().offset());

    if stack.len() == pattern.len() && at == span.end().offset() {
        // We have successfully matched the entire pattern.
        return Ok(stack.clone());
    }

    if at.byte_offset() > span.end().byte_offset() {
        // We have reached the end of the span and not matched so this path is
        // a failure.
        return Err(SplitError::NoMatch);
    }

    if stack.len() == pattern.len() {
        // We have matched the entire pattern but not reached the end of the
        // span so this path is a failure.
        return Err(SplitError::NoMatch);
    }

    match pattern[stack.len()] {
        RulePatternPart::Atom(atom) => {
            // Check if the text has the atom at the current position.
            let Some(atom_end) = parse_atom(atom, text, at) else {
                return Err(SplitError::NoMatch);
            };
            stack.push(atom_end);
            let result = split_with_pattern(text, span, pattern, associativity, stack, chart);
            stack.pop();

            result
        }
        RulePatternPart::Cat(cat) => {
            let continuations = chart.get(&(at, cat)).ok_or(SplitError::NoMatch)?;
            let mut continuations = continuations.clone();

            // Now we need to decide in which order to try the continuations. This
            // depends on the associativity of the rule. If it is left associative
            // we try the longest continuations first. If it is right associative we
            // try the shortest continuations first. If it is non-associative we
            // try them in the order they appear.
            match associativity {
                Associativity::Left => continuations.sort_by_key(|c| Reverse(c.1)),
                Associativity::Right => continuations.sort_by_key(|c| c.1),
                Associativity::NonAssoc => {}
            }

            // Now we search through the continuations. If the pattern is
            // associative we take the first match. If it is non-associative
            // we need to ensure there is only one match.
            let mut solution = None;
            for continuation in continuations {
                stack.push(continuation.1);
                let result = split_with_pattern(text, span, pattern, associativity, stack, chart);
                stack.pop();

                match result {
                    Ok(split) => {
                        if matches!(associativity, Associativity::NonAssoc) {
                            if solution.is_some() && solution.as_ref() != Some(&split) {
                                // We have found more than one match so this
                                // is ambiguous.
                                return Err(SplitError::Ambiguous);
                            }
                            solution = Some(split);
                        } else {
                            return Ok(split);
                        }
                    }
                    Err(SplitError::NoMatch) => continue,
                    Err(SplitError::Ambiguous) => return Err(SplitError::Ambiguous),
                }
            }

            solution.ok_or(SplitError::NoMatch)
        }
    }
}

type TrimmedChart<'ctx> =
    FxHashMap<(SourceOffset, CategoryId<'ctx>), Vec<(RuleId<'ctx>, SourceOffset)>>;

fn trim_chart<'ctx>(chart: &Chart<'ctx>) -> TrimmedChart<'ctx> {
    let mut trimmed: TrimmedChart<'ctx> = FxHashMap::default();

    for (i, items) in chart.items_at_offset.iter().enumerate() {
        let pos = chart.start_offset.forward(i);
        for item in items {
            if item.dot != item.rule.pattern().parts().len() {
                // We only care about completed items.
                continue;
            }

            trimmed
                .entry((item.origin, item.rule.cat()))
                .or_default()
                .push((item.rule, pos));
        }
    }

    trimmed
}

fn _debug_trimmed_chart<'ctx>(trimmed: &TrimmedChart<'ctx>) {
    let mut trimmed: Vec<_> = trimmed.iter().collect();
    trimmed.sort_by_key(|(key, _)| *key);

    for ((offset, cat), completions) in trimmed {
        println!("At {offset:?}, completed <{}>:", cat._name());
        for (rule, end) in completions {
            let mut pattern = String::new();
            for part in rule.pattern().parts() {
                match part {
                    RulePatternPart::Atom(atom) => match atom {
                        ParseAtomPattern::Kw(kw) => pattern.push_str(&format!("\"{kw}\" ")),
                        ParseAtomPattern::Name => pattern.push_str("name "),
                        ParseAtomPattern::Lit(lit) => pattern.push_str(&format!("'{lit}' ")),
                        ParseAtomPattern::Str => pattern.push_str("str "),
                        ParseAtomPattern::Num => pattern.push_str("num "),
                    },
                    RulePatternPart::Cat(id) => {
                        let cat_name = id._name();
                        pattern.push_str(&format!("<{cat_name}> "));
                    }
                }
            }
            println!("  {} -> {} (to {:?})", rule.cat()._name(), pattern, end);
        }
        println!();
    }
}

fn parse_atom(atom: ParseAtomPattern, text: &str, at: SourceOffset) -> Option<SourceOffset> {
    let content = skip_ws_and_comments(text, at);

    match atom {
        ParseAtomPattern::Kw(kw) => {
            let (end, name) = parse_name(text, content)?;
            if name != kw.as_str() {
                return None;
            }
            Some(end)
        }
        ParseAtomPattern::Name => {
            let (end, _name) = parse_name(text, content)?;
            Some(end)
        }
        ParseAtomPattern::Lit(lit) => text[content.byte_offset()..]
            .starts_with(lit.as_str())
            .then(|| content.forward(lit.len())),
        ParseAtomPattern::Str => {
            let (end, _str) = parse_str(text, content)?;
            Some(end)
        }
        ParseAtomPattern::Num => {
            let end = parse_num(text, content)?;
            Some(end)
        }
    }
}

pub fn parse_name(text: &str, from: SourceOffset) -> Option<(SourceOffset, &str)> {
    let mut chars = text[from.byte_offset()..].chars();

    let first_char = chars.next()?;
    if !char_can_start_name(first_char) {
        return None;
    }
    let mut at = from.forward(first_char.len_utf8());

    for next_char in chars {
        if !char_can_continue_name(next_char) {
            break;
        }
        at = at.forward(next_char.len_utf8());
    }

    Some((at, &text[from.byte_offset()..at.byte_offset()]))
}

fn parse_str(text: &str, from: SourceOffset) -> Option<(SourceOffset, &str)> {
    let mut chars = text[from.byte_offset()..].chars();

    let first_char = chars.next()?;
    if first_char != '"' {
        return None;
    }
    let mut at = from.forward(first_char.len_utf8());

    for next_char in chars {
        if next_char == '"' {
            // We have reached the end of the string.
            return Some((
                at.forward(next_char.len_utf8()),
                &text[from.byte_offset() + 1..at.byte_offset()],
            ));
        }
        at = at.forward(next_char.len_utf8());
    }

    // We reached the end of the input without finding a closing quote.
    None
}

fn parse_num(text: &str, from: SourceOffset) -> Option<SourceOffset> {
    let mut chars = text[from.byte_offset()..].chars();
    let mut at = from;

    while let Some(next) = chars.next()
        && next.is_ascii_digit()
    {
        at = at.forward(next.len_utf8());
    }

    if at == from { None } else { Some(at) }
}

fn char_can_start_name(char: char) -> bool {
    char.is_alphabetic() || char == '_' || char == '\''
}

fn char_can_continue_name(char: char) -> bool {
    char_can_start_name(char) || char.is_numeric() || char == '.'
}

fn skip_ws_and_comments(text: &str, mut at: SourceOffset) -> SourceOffset {
    let mut chars = text[at.byte_offset()..].chars().peekable();

    while let Some(next_char) = chars.next() {
        if next_char.is_whitespace() {
            at = at.forward(next_char.len_utf8());
            continue;
        }

        if next_char == '-' && chars.peek() == Some(&'-') {
            at = at.forward(1);
            for next_char in chars.by_ref() {
                at = at.forward(next_char.len_utf8());
                if next_char == '\n' {
                    break;
                }
            }

            continue;
        }

        break;
    }

    at
}
