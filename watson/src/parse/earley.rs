use crate::parse::stream::{Checkpoint, Stream};
use std::{
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
) where
    NonTerm: Hash + Eq + Clone + Debug,
    Term: EarleyTerm + Hash + Eq + Clone + Debug,
{
    dbg!(str.remaining_text());

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
                        // dbg!(&(next.start, next.sym));
                        for creator in creators.clone() {
                            let item = creator.advance();
                            chart.entry(cur_pos).or_default().insert(item.clone());
                        }
                    }
                }
            }
        }
    }

    let mut vec: Vec<_> = chart.into_iter().collect();
    vec.sort_by_key(|x| x.0);
    for (check, items) in vec {
        let mut items: Vec<_> = items.into_iter().collect();
        items.sort_by_key(|i| format!("{:?}", i.sym));

        println!("{check:?}:");
        for item in items {
            print!("  ({:?}) {:?} ->", item.start, item.sym);
            for (i, p) in item.rule.iter().enumerate() {
                if i == item.pos {
                    print!(" •");
                }
                match p {
                    EarleySymbol::Terminal(t) => print!(" {t:?}"),
                    EarleySymbol::NonTerminal(nt) => print!(" {nt:?}"),
                }
            }
            if item.rule.len() == item.pos {
                print!(" •");
            }
            println!();
        }
        println!();
    }
}
