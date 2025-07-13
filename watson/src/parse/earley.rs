use crate::parse::stream::{Checkpoint, Stream};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt::Debug,
    hash::Hash,
};

pub trait EarleyRule<S> {
    fn predict(&self, pos: usize) -> Option<S>;
    fn debug(&self) -> Vec<String>;
}

pub trait EarleySymbol<R> {
    fn scan(&self, str: &mut Stream) -> Option<Checkpoint>;
    fn rules_for(&self) -> impl Iterator<Item = R>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EarleyItem<S, R> {
    sym: S,
    rule: R,
    start: Checkpoint,
    pos: usize,
}

impl<S, R> EarleyItem<S, R> {
    fn advance(self) -> Self {
        Self {
            pos: self.pos + 1,
            ..self
        }
    }
}

pub fn earley_parse<
    S: EarleySymbol<R> + Hash + Eq + Clone + Debug,
    R: EarleyRule<S> + Hash + Eq + Clone + Debug,
>(
    str: &mut Stream,
    start_sym: S,
) {
    str.skip_ws();

    let mut chart: HashMap<Checkpoint, HashSet<EarleyItem<S, R>>> = HashMap::new();
    let mut creators: HashMap<(Checkpoint, S), Vec<EarleyItem<S, R>>> = HashMap::new();

    let mut pos_queue = HashSet::new();
    let start_pos = str.checkpoint();
    pos_queue.insert(start_pos);

    for rule in start_sym.rules_for() {
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
        let mut queue = VecDeque::new();

        for item in chart.entry(cur_pos).or_default().iter() {
            queue.push_back(item.clone());
        }

        let add =
            |item: EarleyItem<S, R>,
             queue: &mut VecDeque<EarleyItem<S, R>>,
             chart: &mut HashMap<Checkpoint, HashSet<EarleyItem<S, R>>>,
             creators: &mut HashMap<(Checkpoint, S), Vec<EarleyItem<S, R>>>| {
                if chart.entry(cur_pos).or_default().insert(item.clone()) {
                    queue.push_back(item.clone());
                    creators
                        .entry((cur_pos, item.sym.clone()))
                        .or_default()
                        .push(item);
                }
            };

        while let Some(next) = queue.pop_front() {
            if let Some(sym) = next.rule.predict(next.pos) {
                if let Some(scan) = sym.scan(str) {
                    chart.entry(scan).or_default().insert(next.advance());
                    pos_queue.insert(scan);
                } else if let Some(sym) = next.rule.predict(next.pos) {
                    for prediction in sym.clone().rules_for() {
                        let item = EarleyItem {
                            sym: sym.clone(),
                            rule: prediction,
                            start: cur_pos,
                            pos: 0,
                        };

                        add(item, &mut queue, &mut chart, &mut creators);
                    }
                }
            } else {
                for creator in creators[&(next.start, next.sym)].clone() {
                    let item = creator.advance();
                    add(item, &mut queue, &mut chart, &mut creators);
                }
            }
        }
    }

    let mut vec: Vec<_> = chart.into_iter().collect();
    vec.sort_by_key(|x| x.0);
    for (check, items) in vec {
        println!("{check:?}:");
        for item in items {
            let d = item.rule.debug();
            print!("  (({:?})) ", item.start);
            for (i, p) in d.iter().enumerate() {
                if i == item.pos {
                    print!(" • ");
                }
                print!("{} ", p);
            }
            if d.len() == item.pos {
                print!(" • ");
            }
            println!();
        }
        println!();
    }
}
