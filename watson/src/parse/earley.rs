use crate::{
    context::Ctx,
    diagnostics::WResult,
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

pub fn parse(start: Location, category: CategoryId, ctx: &mut Ctx) -> WResult<ParseTreeId> {
    let chart = build_chart(start, category, ctx);
    let trimmed = trim_chart(&chart, ctx);

    if !trimmed.contains_key(&(start.offset(), category)) {
        return make_parse_error(&chart, start.source(), ctx);
    }

    read_chart(start, category, &trimmed, ctx)
}

fn build_chart(start: Location, category: CategoryId, ctx: &mut Ctx) -> Chart {
    let text = ctx.sources.get_text(start.source());
    let mut chart = Chart::new(start.offset());

    // Add all the start rules for the category we are parsing.
    for &rule in ctx.parse_state.rules_for_cat(category) {
        let item = Item::new(rule, start.offset(), false);
        chart.add_item(item, start.offset());
    }

    // This tracks where in the source we are.
    let mut current_position = start.offset();

    while let Some(items) = chart.get_items(current_position) {
        let mut items: VecDeque<Item> = items.iter().copied().collect();

        while let Some(item) = items.pop_front() {
            let rule = &ctx.parse_state[item.rule];
            let next_part = rule.pattern().parts().get(item.dot);

            match next_part {
                Some(RulePatternPart::Atom(atom)) => {
                    // Scan. Look for the atom at the current position.
                    let Some(atom_end) = parse_atom(*atom, text, current_position) else {
                        // We didn't find the atom we were looking for so this
                        // item can't match. We drop it and move on.
                        continue;
                    };

                    if !item.template && atom == &ParseAtomPattern::MacroBinding {
                        // We don't allow macro bindings outside of templates.
                        continue;
                    }

                    // Add the item wherever the atom ended.
                    chart.add_item(item.advance(), atom_end);
                }
                Some(RulePatternPart::Cat {
                    id: cat,
                    template: new_template,
                }) => {
                    // Predict. Add all the rules for the category at the current position.
                    for &prediction in ctx.parse_state.rules_for_cat(*cat) {
                        let new_item =
                            Item::new(prediction, current_position, item.template || *new_template);
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
                    let Some(waiters) = chart.get_waiters(item.origin, rule.cat()) else {
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

fn _debug_chart(chart: &Chart, ctx: &Ctx) {
    for (i, items) in chart.items_at_offset.iter().enumerate() {
        if items.is_empty() {
            continue;
        }

        let pos = chart.start_offset.forward(i);
        println!("At {pos:?}:");
        for item in items {
            let rule = &ctx.parse_state[item.rule];
            let mut pattern = String::new();
            for (j, part) in rule.pattern().parts().iter().enumerate() {
                if j == item.dot {
                    pattern.push_str("• ");
                }
                match part {
                    RulePatternPart::Atom(atom) => match atom {
                        ParseAtomPattern::Kw(kw) => pattern.push_str(&format!("\"{kw}\" ")),
                        ParseAtomPattern::Name => pattern.push_str("name "),
                        ParseAtomPattern::Lit(lit) => pattern.push_str(&format!("'{lit}' ")),
                        ParseAtomPattern::Str => pattern.push_str("str "),
                        ParseAtomPattern::MacroBinding => pattern.push_str("macro_binding "),
                    },
                    RulePatternPart::Cat { id, template } => {
                        let cat_name = ctx.parse_state[*id].name();
                        if *template {
                            pattern.push_str(&format!("{{{cat_name}}} "));
                        } else {
                            pattern.push_str(&format!("<{cat_name}> "));
                        }
                    }
                }
            }
            if item.dot == rule.pattern().parts().len() {
                pattern.push('•');
            }
            let origin = item.origin;
            println!(
                "  {} -> {} (from {:?})",
                ctx.parse_state[rule.cat()].name(),
                pattern,
                origin
            );
        }

        println!();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Item {
    rule: RuleId,
    dot: usize,
    origin: SourceOffset,
    template: bool,
}

impl Item {
    fn new(rule: RuleId, origin: SourceOffset, template: bool) -> Self {
        Self {
            rule,
            dot: 0,
            origin,
            template,
        }
    }

    fn advance(self) -> Self {
        Self {
            dot: self.dot + 1,
            ..self
        }
    }
}

struct Chart {
    start_offset: SourceOffset,
    items_at_offset: Vec<FxHashSet<Item>>,
    waiting: FxHashMap<(SourceOffset, CategoryId), FxHashSet<Item>>,
}

impl Chart {
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

    fn add_item(&mut self, item: Item, pos: SourceOffset) -> bool {
        self.ensure_offset(pos);
        let index = self.idx_for_offset(pos);
        self.items_at_offset[index].insert(item)
    }

    fn wait_for_completion(&mut self, at: SourceOffset, cat: CategoryId, item: Item) {
        self.waiting.entry((at, cat)).or_default().insert(item);
    }

    fn get_items(&self, at: SourceOffset) -> Option<&FxHashSet<Item>> {
        let index = self.idx_for_offset(at);
        self.items_at_offset.get(index)
    }

    fn get_waiters(&self, at: SourceOffset, cat: CategoryId) -> Option<&FxHashSet<Item>> {
        self.waiting.get(&(at, cat))
    }
}

fn make_parse_error<T>(chart: &Chart, source: SourceId, ctx: &mut Ctx) -> WResult<T> {
    let latest_pos = chart.start_offset.forward(chart.items_at_offset.len() - 1);
    let latest_items = chart.items_at_offset.last().unwrap();

    let mut possible_next_atoms = FxHashSet::default();

    for item in latest_items {
        let rule = &ctx.parse_state[item.rule];
        let pat = rule.pattern();
        if let Some(RulePatternPart::Atom(atom)) = pat.parts().get(item.dot) {
            possible_next_atoms.insert(*atom);
        }
    }

    let location = skip_ws_and_comments(ctx.sources.get_text(source), latest_pos);
    let location = Location::new(source, location);

    let mut possible_atoms = possible_next_atoms.into_iter().collect::<Vec<_>>();
    possible_atoms.sort();

    ctx.diags.err_parse_failure(location, &possible_atoms)
}

fn read_chart(
    start: Location,
    cat: CategoryId,
    chart: &TrimmedChart,
    ctx: &mut Ctx,
) -> WResult<ParseTreeId> {
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
    fn search(
        span: Span,
        cat: CategoryId,
        chart: &TrimmedChart,
        ctx: &mut Ctx,
    ) -> WResult<ParseTreeId> {
        // The idea here is to check which rules we have for the given span and
        // category. We then choose which among those rules is best. If there
        // is still a tie the parse is ambiguous.
        let rules = chart[&(span.start().offset(), cat)]
            .iter()
            .filter(|(_, end)| *end == span.end().offset())
            .map(|(rule, _end)| *rule);
        let best_rules = choose_best_rule(rules, ctx);

        // Each rule gives us a pattern which we can use to split the span
        // into parts. We then recursively search for each part.
        let mut possibilities = Vec::new();
        for rule_id in &best_rules {
            let rule = &ctx.parse_state[*rule_id];
            let text = ctx.sources.get_text(span.source());
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
                        &rule.pattern().parts().to_owned(),
                        &split,
                        span.start(),
                        chart,
                        ctx,
                    )?;
                    possibilities.push(ParseTreeChildren::new(*rule_id, children));
                }
                // We don't allow any ambiguity within a single rule, only between
                // rules. So this is an immediate error.
                Err(SplitError::Ambiguous) => return ctx.diags.err_ambiguous_parse(span),
                // The chart guaranteed that there is at least one match so
                // this should never happen.
                Err(SplitError::NoMatch) => unreachable!(),
            }
        }

        let tree = ParseTree::new(span, cat, possibilities);
        Ok(ctx.parse_forest.get_or_insert(tree))
    }

    fn split_to_children(
        pattern: &[RulePatternPart],
        offsets: &[SourceOffset],
        start: Location,
        chart: &TrimmedChart,
        ctx: &mut Ctx,
    ) -> WResult<Vec<ParseTreePart>> {
        debug_assert_eq!(pattern.len(), offsets.len());

        let mut start = start;
        let mut parts = Vec::new();
        for (pat, offset) in pattern.iter().zip(offsets.iter()) {
            let text = ctx.sources.get_text(start.source());
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
                        ParseAtomPattern::MacroBinding => {
                            let start = skip_ws_and_comments(text, span.start().offset());
                            let name = &text[start.byte_offset() + 1..span.end().byte_offset()];
                            ParseAtomKind::MacroBinding(name.into())
                        }
                    };
                    let atom = ParseAtom::new(span, kind);
                    parts.push(ParseTreePart::Atom(atom));
                }
                RulePatternPart::Cat { id, .. } => {
                    let tree_id = search(span, *id, chart, ctx)?;
                    parts.push(ParseTreePart::Node {
                        id: tree_id,
                        span,
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

fn choose_best_rule(rules: impl Iterator<Item = RuleId>, ctx: &Ctx) -> Vec<RuleId> {
    // We pick the rules with the lowest precedence value.
    let mut best_rules = Vec::new();
    let mut best_precedence = None;

    for rule in rules {
        let precedence = ctx.parse_state[rule].pattern().precedence();
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
        RulePatternPart::Cat { id: cat, .. } => {
            let continuations = chart.get(&(at, cat)).ok_or(SplitError::NoMatch)?;
            let mut continuations = continuations.clone();

            // Now we need to decide in which order to try the continuations. This
            // depends on the associativity of the rule. If it is left associative
            // we try the longest continuations first. If it is right associative we
            // try the shortest continuations first. If it is non-associative we
            // try them in the order they appear.
            match associativity {
                Associativity::Left => continuations.sort_by_key(|c| c.1),
                Associativity::Right => continuations.sort_by_key(|c| Reverse(c.1)),
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

type TrimmedChart = FxHashMap<(SourceOffset, CategoryId), Vec<(RuleId, SourceOffset)>>;

fn trim_chart(chart: &Chart, ctx: &Ctx) -> TrimmedChart {
    let mut trimmed: TrimmedChart = FxHashMap::default();

    for (i, items) in chart.items_at_offset.iter().enumerate() {
        let pos = chart.start_offset.forward(i);
        for item in items {
            if item.dot != ctx.parse_state[item.rule].pattern().parts().len() {
                // We only care about completed items.
                continue;
            }

            let rule = item.rule;
            let cat = ctx.parse_state[rule].cat();
            let origin = item.origin;
            trimmed.entry((origin, cat)).or_default().push((rule, pos));
        }
    }

    trimmed
}

fn _debug_trimmed_chart(trimmed: &TrimmedChart, ctx: &Ctx) {
    let mut trimmed: Vec<_> = trimmed.iter().collect();
    trimmed.sort_by_key(|(key, _)| *key);

    for ((offset, cat), completions) in trimmed {
        println!(
            "At {offset:?}, completed <{}>:",
            ctx.parse_state[*cat].name()
        );
        for (rule, end) in completions {
            let rule = &ctx.parse_state[*rule];
            let mut pattern = String::new();
            for part in rule.pattern().parts() {
                match part {
                    RulePatternPart::Atom(atom) => match atom {
                        ParseAtomPattern::Kw(kw) => pattern.push_str(&format!("\"{kw}\" ")),
                        ParseAtomPattern::Name => pattern.push_str("name "),
                        ParseAtomPattern::Lit(lit) => pattern.push_str(&format!("'{lit}' ")),
                        ParseAtomPattern::Str => pattern.push_str("str "),
                        ParseAtomPattern::MacroBinding => pattern.push_str("macro_binding "),
                    },
                    RulePatternPart::Cat { id, template } => {
                        let cat_name = ctx.parse_state[*id].name();
                        if *template {
                            pattern.push_str(&format!("{{{cat_name}}} "));
                        } else {
                            pattern.push_str(&format!("<{cat_name}> "));
                        }
                    }
                }
            }
            println!(
                "  {} -> {} (to {:?})",
                ctx.parse_state[rule.cat()].name(),
                pattern,
                end
            );
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
        ParseAtomPattern::MacroBinding => {
            let (end, _name) = parse_macro_binding(text, content)?;
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

fn parse_macro_binding(text: &str, from: SourceOffset) -> Option<(SourceOffset, &str)> {
    let content = skip_ws_and_comments(text, from);

    let mut chars = text[content.byte_offset()..].chars();

    let first_char = chars.next()?;
    if first_char != '$' {
        return None;
    }
    let mut at = content.forward(first_char.len_utf8());

    let second_char = chars.next()?;
    if !char_can_start_name(second_char) {
        return None;
    }
    at = at.forward(second_char.len_utf8());

    for next_char in chars {
        if !char_can_continue_name(next_char) {
            break;
        }
        at = at.forward(next_char.len_utf8());
    }

    Some((at, &text[content.byte_offset() + 1..at.byte_offset()]))
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
            at = at.forward(2);
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
