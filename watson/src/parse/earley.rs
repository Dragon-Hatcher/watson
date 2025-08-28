use crate::{
    context::Ctx,
    diagnostics::WResult,
    parse::{
        Location, Span,
        location::SourceOffset,
        parse_state::{CategoryId, ParseAtomPattern, RuleId, RulePatternPart},
        parse_tree::ParseTreePromise,
    },
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

pub fn parse(start: Location, category: CategoryId, ctx: &mut Ctx) -> WResult<ParseTreePromise> {
    let chart = build_chart(start, category, ctx);
    let (promises, top_promise) = build_promises(start, category, &chart, ctx);

    match top_promise {
        Ok(top_promise) => {
            add_promises(&promises, ctx);
            Ok(top_promise)
        }
        Err(_) => {
            todo!("parse error: no parse found");
        }
    }
}

fn build_chart(start: Location, category: CategoryId, ctx: &mut Ctx) -> Chart {
    let text = ctx.sources.get_text(start.source());
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

                    // Add the item wherever the atom ended.
                    chart.add_item(item.advance(), atom_end);
                }
                Some(RulePatternPart::Cat(cat) | RulePatternPart::TempCat(cat)) => {
                    // Predict. Add all the rules for the category at the current position.
                    for &prediction in ctx.parse_state.rules_for_cat(*cat) {
                        let new_item = Item::new(prediction, current_position);
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

fn build_promises(
    start: Location,
    top_cat: CategoryId,
    chart: &Chart,
    ctx: &mut Ctx,
) -> (
    FxHashMap<ParseTreePromise, Vec<RuleId>>,
    WResult<ParseTreePromise>,
) {
    let mut promises: FxHashMap<ParseTreePromise, Vec<RuleId>> = FxHashMap::default();
    let mut top_promise = Err(());

    for (offset, items) in chart.items_at_offset.iter().enumerate() {
        for item in items {
            let rule = &ctx.parse_state[item.rule];
            if item.dot < rule.pattern().parts().len() {
                continue;
            }

            let span = Span::new(
                Location::new(start.source(), item.origin),
                start.forward(offset),
            );
            let promise = ParseTreePromise::new(rule.cat(), span);

            if span.start() == start && rule.cat() == top_cat {
                top_promise = Ok(promise);
            }

            promises.entry(promise).or_default().push(item.rule);
        }
    }

    (promises, top_promise)
}

fn add_promises(promises: &FxHashMap<ParseTreePromise, Vec<RuleId>>, ctx: &mut Ctx) {
    for (promise, rules) in promises {
        for &rule in rules {
            ctx.parse_forest.add_promise(*promise, rule);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Item {
    rule: RuleId,
    dot: usize,
    origin: SourceOffset,
}

impl Item {
    fn new(rule: RuleId, origin: SourceOffset) -> Self {
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
        ParseAtomPattern::Str => todo!(),
    }
}

fn parse_name(text: &str, from: SourceOffset) -> Option<(SourceOffset, &str)> {
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
