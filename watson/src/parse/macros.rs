use crate::{
    context::Ctx,
    parse::{
        parse_state::{CategoryId, RulePattern, RulePatternPart},
        parse_tree::{ParseTree, ParseTreeChildren, ParseTreeId, ParseTreePart},
    },
};
use rustc_hash::FxHashMap;
use slotmap::{SlotMap, new_key_type};
use std::ops::Index;
use ustr::Ustr;

pub struct Macros {
    macros: SlotMap<MacroId, Macro>,
    by_name: FxHashMap<Ustr, MacroId>,
}

new_key_type! { pub struct MacroId; }

impl Macros {
    pub fn new() -> Self {
        Self {
            macros: SlotMap::default(),
            by_name: FxHashMap::default(),
        }
    }

    pub fn get_id_by_name(&self, name: Ustr) -> Option<MacroId> {
        self.by_name.get(&name).cloned()
    }

    pub fn add_macro(&mut self, mac: Macro) -> MacroId {
        let name = mac.name;
        let id = self.macros.insert(mac);
        self.by_name.insert(name, id);
        id
    }

    pub fn macros(&self) -> impl Iterator<Item = &Macro> {
        self.macros.values()
    }
}

impl Index<MacroId> for Macros {
    type Output = Macro;

    fn index(&self, index: MacroId) -> &Self::Output {
        &self.macros[index]
    }
}

pub struct Macro {
    name: Ustr,
    cat: CategoryId,
    pat: MacroPat,
    replacement: ParseTreeId,
}

impl Macro {
    pub fn new(name: Ustr, cat: CategoryId, pat: MacroPat, replacement: ParseTreeId) -> Self {
        Self {
            name,
            cat,
            pat,
            replacement,
        }
    }

    pub fn name(&self) -> Ustr {
        self.name
    }

    pub fn cat(&self) -> CategoryId {
        self.cat
    }

    pub fn pat(&self) -> &MacroPat {
        &self.pat
    }

    pub fn replacement(&self) -> ParseTreeId {
        self.replacement
    }

    pub fn collect_macro_bindings(&self, tree: &ParseTreeChildren) -> FxHashMap<Ustr, ParseTreeId> {
        let mut map = FxHashMap::default();
        for (&name, &idx) in self.pat.keys() {
            map.insert(name, tree.children()[idx].as_node().unwrap());
        }
        map
    }
}

pub fn do_macro_replacement(
    replace_in: ParseTreeId,
    bindings: &FxHashMap<Ustr, ParseTreeId>,
    ctx: &mut Ctx,
) -> ParseTreeId {
    let old_tree = &ctx.parse_forest[replace_in];
    let span = old_tree.span();
    let cat = old_tree.cat();

    let mut new_possibilities = Vec::new();

    for possibility in old_tree.possibilities().to_owned() {
        if let [binding] = possibility.children()
            && let Some(binding) = binding.as_macro_binding()
        {
            // Add all the possibilities from the binding.
            for new_possibility in ctx.parse_forest[bindings[&binding]].possibilities() {
                new_possibilities.push(new_possibility.clone());
            }
        } else {
            let mut parts = Vec::new();
            for child in possibility.children() {
                let part = match child {
                    ParseTreePart::Atom(atom) => ParseTreePart::Atom(*atom),
                    ParseTreePart::Node { id, cat, span } => {
                        let new_node = do_macro_replacement(*id, bindings, ctx);
                        ParseTreePart::Node {
                            id: new_node,
                            span: *span,
                            cat: *cat,
                        }
                    }
                };
                parts.push(part);
            }
            let new_possibility = ParseTreeChildren::new(possibility.rule(), parts);
            new_possibilities.push(new_possibility);
        }
    }

    let new_tree = ParseTree::new(span, cat, new_possibilities);
    ctx.parse_forest.get_or_insert(new_tree)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroPat {
    parts: Vec<RulePatternPart>,
    keys: FxHashMap<Ustr, usize>,
}

impl MacroPat {
    pub fn new(parts: Vec<RulePatternPart>, keys: FxHashMap<Ustr, usize>) -> Self {
        Self { parts, keys }
    }

    pub fn parts(&self) -> &[RulePatternPart] {
        &self.parts
    }

    pub fn keys(&self) -> &FxHashMap<Ustr, usize> {
        &self.keys
    }

    pub fn to_parse_rule(&self) -> RulePattern {
        RulePattern::new(self.parts.clone())
    }
}
