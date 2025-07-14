use itertools::Itertools;
use ustr::Ustr;
use crate::parse::stream::{Checkpoint, ParseError, ParseResult, Stream};
use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    hash::Hash,
};

#[derive(Debug)]
pub enum EarleySymbol<Term, NonTerm> {
    Terminal(Term),
    NonTerminal(NonTerm),
}

pub trait EarleyTerm {
    fn scan(&self, str: &mut Stream) -> Option<Checkpoint>;
}

type Rule<Term, NonTerm> = Vec<EarleySymbol<Term, NonTerm>>;

#[derive(Debug)]
pub struct EarleyGrammar<Term, NonTerm> {
    rules: HashMap<NonTerm, Vec<Rule<Term, NonTerm>>>,
}

impl<Term, NonTerm> EarleyGrammar<Term, NonTerm>
where
    NonTerm: Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    pub fn add_rule(&mut self, nt: NonTerm, syms: Rule<Term, NonTerm>) {
        self.rules.entry(nt).or_default().push(syms);
    }

    fn rules(&self, nt: &NonTerm) -> &[Rule<Term, NonTerm>] {
        self.rules.get(nt).map(|r| r.as_slice()).unwrap_or(&[])
    }
}

#[derive(Debug, Clone)]
struct EarleyItem<'a, Term, NonTerm> {
    sym: NonTerm,
    rule: &'a [EarleySymbol<Term, NonTerm>],
    start: Checkpoint,
    pos: usize,
}

impl<'a, Term, NonTerm> PartialEq for EarleyItem<'a, Term, NonTerm>
where
    Term: Eq,
    NonTerm: Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.sym == other.sym
            && std::ptr::addr_eq(self.rule, other.rule)
            && self.start == other.start
            && self.pos == other.pos
    }
}

impl<'a, Term, NonTerm> Eq for EarleyItem<'a, Term, NonTerm>
where
    Term: Eq,
    NonTerm: Eq,
{
}

impl<'a, Term, NonTerm> Hash for EarleyItem<'a, Term, NonTerm>
where
    NonTerm: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.sym.hash(state);
        (self.rule as *const [EarleySymbol<Term, NonTerm>]).hash(state);
        self.start.hash(state);
        self.pos.hash(state);
    }
}

impl<'a, Term, NonTerm> EarleyItem<'a, Term, NonTerm> {
    fn advance(self) -> Self {
        Self {
            pos: self.pos + 1,
            ..self
        }
    }
}

pub fn earley_parse<Term, NonTerm>(
    str: &mut Stream,
    grammar: EarleyGrammar<Term, NonTerm>,
    start_sym: NonTerm,
) -> ParseResult<EarleyParseRes<Term, NonTerm>>
where
    NonTerm: Hash + Eq + Clone + Debug,
    Term: EarleyTerm + Hash + Eq + Clone + Debug,
{
    str.skip_ws();

    let mut chart: HashMap<Checkpoint, HashSet<EarleyItem<Term, NonTerm>>> = HashMap::new();
    let mut creators: HashMap<(Checkpoint, NonTerm), HashSet<EarleyItem<Term, NonTerm>>> =
        HashMap::new();

    let mut pos_queue = HashSet::new();
    let start_pos = str.checkpoint();
    pos_queue.insert(start_pos);

    for rule in grammar.rules(&start_sym) {
        let item = EarleyItem {
            sym: start_sym.clone(),
            rule,
            start: start_pos,
            pos: 0,
        };
        chart.entry(start_pos).or_default().insert(item);
    }

    while let Some(cur_pos) = pos_queue.iter().min().copied() {
        pos_queue.remove(&cur_pos);
        str.rewind(cur_pos);
        let mut queue = VecDeque::new();

        for item in chart.entry(cur_pos).or_default().iter() {
            queue.push_back(item.clone());
        }

        while let Some(next) = queue.pop_front() {
            match next.rule.get(next.pos) {
                Some(EarleySymbol::Terminal(term)) => {
                    // Scan
                    if let Some(scan) = term.scan(str) {
                        chart
                            .entry(scan)
                            .or_default()
                            .insert(next.clone().advance());
                        pos_queue.insert(scan);
                    }
                }
                Some(EarleySymbol::NonTerminal(non_term)) => {
                    // Predict
                    for prediction in grammar.rules(non_term) {
                        let item = EarleyItem {
                            sym: non_term.clone(),
                            rule: prediction,
                            start: cur_pos,
                            pos: 0,
                        };

                        creators
                            .entry((cur_pos, non_term.clone()))
                            .or_default()
                            .insert(next.clone());

                        if chart.entry(cur_pos).or_default().insert(item.clone()) {
                            queue.push_back(item.clone());
                        }
                    }
                }
                None => {
                    // Completion
                    if let Some(creators) = creators.get(&(next.start, next.sym.clone())) {
                        for creator in creators.clone() {
                            let item = creator.advance();
                            if chart.entry(cur_pos).or_default().insert(item.clone()) {
                                queue.push_back(item.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // We have now recognized. Let's filter down to completed items.
    let mut completion_chart = chart.clone();

    for (_, items) in completion_chart.iter_mut() {
        items.retain(|i| i.pos == i.rule.len());
    }

    let mut rev_chart: HashMap<Checkpoint, HashMap<NonTerm, Vec<SearchItem<Term, NonTerm>>>> =
        HashMap::new();

    for (end, items) in completion_chart {
        for item in items {
            let map = rev_chart.entry(item.start).or_default();
            let list = map.entry(item.sym).or_default();
            list.push(SearchItem {
                end,
                rule: item.rule,
            });
        }
    }

    // let mut vec: Vec<_> = rev_chart.clone().into_iter().collect();
    // vec.sort_by_key(|x| x.0);
    // for (check, items) in vec {
    //     println!("{check:?}:");
    //     for (sym, items) in items {
    //         println!("  {sym:?} ->");
    //         for item in items {
    //             print!("    {:?}", item.end);
    //             for part in item.rule {
    //                 match part {
    //                     EarleySymbol::Terminal(t) => print!(" {t:?}"),
    //                     EarleySymbol::NonTerminal(nt) => print!(" {nt:?}"),
    //                 }
    //             }
    //             println!();
    //         }
    //     }
    //     println!();
    // }

    if let Some(matches) = rev_chart
        .get(&start_pos)
        .map(|p| p.get(&start_sym))
        .flatten()
    {
        let end = matches.iter().map(|m| m.end).max().unwrap();
        let res = Ok(find_parse(start_pos, end, start_sym, &rev_chart, str));
        str.rewind(end);
        return res;
    }

    // The parse failed. Let's try to diagnose why:

    let last_pos = chart.keys().max().unwrap();
    Err(ParseError::new_backtrack(last_pos.0))
}

#[derive(Debug, Clone)]
struct SearchItem<'a, Term, NonTerm> {
    end: Checkpoint,
    rule: &'a [EarleySymbol<Term, NonTerm>],
}

#[derive(Debug, Clone)]
struct InProgressPath {
    rule_pos: usize,
    taken: Vec<Checkpoint>,
}

#[derive(Debug, Clone)]
pub enum EarleyParseRes<Term, NonTerm> {
    NonTerminal {
        symbol: NonTerm,
        children: Vec<EarleyParseRes<Term, NonTerm>>,
    },
    Terminal {
        term: Term,
        text: Ustr,
    },
}

impl InProgressPath {
    fn advance_to(mut self, pos: Checkpoint) -> Self {
        self.rule_pos += 1;
        self.taken.push(pos);
        self
    }
}

fn find_parse<Term, NonTerm>(
    start: Checkpoint,
    end: Checkpoint,
    ty: NonTerm,
    chart: &HashMap<Checkpoint, HashMap<NonTerm, Vec<SearchItem<Term, NonTerm>>>>,
    str: &mut Stream,
) -> EarleyParseRes<Term, NonTerm>
where
    NonTerm: Hash + Eq + Clone + Debug,
    Term: EarleyTerm + Hash + Eq + Clone + Debug,
{
    let candidates = &chart[&start][&ty];
    let mut candidates = candidates.iter().filter(|c| c.end == end);

    // TODO: choose based on correct factors.
    let top_choice = candidates.next().unwrap();

    let mut search_stack = Vec::new();
    search_stack.push(InProgressPath {
        rule_pos: 0,
        taken: vec![start],
    });

    while let Some(next) = search_stack.pop() {
        if next.rule_pos == top_choice.rule.len() {
            if *next.taken.last().unwrap() != end {
                continue;
            }

            let children = next
                .taken
                .into_iter()
                .tuple_windows()
                .zip(top_choice.rule)
                .map(|((start, end), sym)| match sym {
                    EarleySymbol::Terminal(term) => {
                        let text = str.text();
                        EarleyParseRes::Terminal {
                            term: term.clone(),
                            text: Ustr::from(&text[start.0..end.0]),
                        }
                    }
                    EarleySymbol::NonTerminal(nt) => find_parse(start, end, nt.clone(), chart, str),
                })
                .collect();

            return EarleyParseRes::NonTerminal {
                symbol: ty,
                children,
            };
        }

        match &top_choice.rule[next.rule_pos] {
            EarleySymbol::Terminal(term) => {
                str.rewind(*next.taken.last().unwrap());
                if let Some(end) = term.scan(str) {
                    search_stack.push(next.advance_to(end));
                }
            }
            EarleySymbol::NonTerminal(nt) => {
                let at = next.taken.last().unwrap();
                if let Some(map) = chart.get(&at)
                    && let Some(choices) = map.get(nt)
                {
                    let mut choices = choices.clone();
                    choices.sort_by_key(|c| Reverse(c.end));

                    for choice in choices {
                        if std::ptr::addr_eq(choice.rule, top_choice.rule) && *at == start {
                            continue;
                        }

                        search_stack.push(next.clone().advance_to(choice.end));
                    }
                }
            }
        }
    }

    unreachable!("There must be a valid path.");
}
