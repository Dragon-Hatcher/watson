use crate::{
    context::Ctx,
    generate_arena_handle,
    parse::{
        parse_state::{CategoryId, RulePattern, RulePatternPart},
        parse_tree::{ParseTree, ParseTreeChildren, ParseTreeId, ParseTreePart},
    },
};
use rustc_hash::FxHashMap;
use ustr::Ustr;

generate_arena_handle!(MacroId<'ctx> => Macro<'ctx>);

#[derive(Debug, PartialEq, Eq)]
pub struct Macro<'ctx> {
    name: Ustr,
    cat: CategoryId<'ctx>,
    pat: MacroPat<'ctx>,
    replacement: ParseTreeId<'ctx>,
}

impl<'ctx> Macro<'ctx> {
    pub fn new(
        name: Ustr,
        cat: CategoryId<'ctx>,
        pat: MacroPat<'ctx>,
        replacement: ParseTreeId<'ctx>,
    ) -> Self {
        Self {
            name,
            cat,
            pat,
            replacement,
        }
    }

    pub fn _name(&self) -> Ustr {
        self.name
    }

    pub fn _cat(&self) -> CategoryId<'ctx> {
        self.cat
    }

    pub fn pat(&self) -> &MacroPat<'ctx> {
        &self.pat
    }

    pub fn replacement(&self) -> ParseTreeId<'ctx> {
        self.replacement
    }

    pub fn collect_macro_bindings(
        &self,
        tree: &ParseTreeChildren<'ctx>,
    ) -> FxHashMap<Ustr, ParseTreeId<'ctx>> {
        let mut map = FxHashMap::default();
        for (&name, &idx) in self.pat.keys() {
            map.insert(name, tree.children()[idx].as_node().unwrap());
        }
        map
    }
}

pub fn do_macro_replacement<'ctx>(
    replace_in: ParseTreeId<'ctx>,
    bindings: &FxHashMap<Ustr, ParseTreeId<'ctx>>,
    ctx: &mut Ctx<'ctx>,
) -> ParseTreeId<'ctx> {
    let mut new_possibilities = Vec::new();

    for possibility in replace_in.possibilities().to_owned() {
        if let [binding] = possibility.children()
            && let Some(binding) = binding.as_macro_binding()
        {
            // Add all the possibilities from the binding.
            for new_possibility in bindings[&binding].possibilities() {
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

    let new_tree = ParseTree::new(replace_in.span(), replace_in.cat(), new_possibilities);
    ctx.arenas.parse_forest.intern(new_tree)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroPat<'ctx> {
    parts: Vec<RulePatternPart<'ctx>>,
    keys: FxHashMap<Ustr, usize>,
}

impl<'ctx> MacroPat<'ctx> {
    pub fn new(parts: Vec<RulePatternPart<'ctx>>, keys: FxHashMap<Ustr, usize>) -> Self {
        Self { parts, keys }
    }

    pub fn parts(&self) -> &[RulePatternPart<'ctx>] {
        &self.parts
    }

    pub fn keys(&self) -> &FxHashMap<Ustr, usize> {
        &self.keys
    }

    pub fn to_parse_rule(&self) -> RulePattern<'ctx> {
        RulePattern::new(self.parts.clone())
    }
}
