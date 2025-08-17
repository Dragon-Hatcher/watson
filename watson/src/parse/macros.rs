use crate::parse::parse_tree::{AtomPattern, ParseTree, SyntaxCategoryId};
use std::collections::HashMap;
use ustr::Ustr;

pub struct Macros {
    macros: HashMap<MacroId, MacroInfo>,
}

impl Macros {
    pub fn new() -> Self {
        Self {
            macros: HashMap::new(),
        }
    }

    pub fn has_id(&self, id: MacroId) -> bool {
        self.macros.contains_key(&id)
    }

    pub fn add_macro(&mut self, info: MacroInfo) {
        self.macros.insert(info.id, info);
    }

    pub fn macros(&self) -> impl Iterator<Item = &MacroInfo> {
        self.macros.values()
    }

    pub fn get(&self, id: MacroId) -> Option<&MacroInfo> {
        self.macros.get(&id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MacroId(Ustr);

impl MacroId {
    pub fn new(name: Ustr) -> Self {
        Self(name)
    }
}

pub struct MacroInfo {
    id: MacroId,
    cat: SyntaxCategoryId,
    pat: MacroPat,
    replacement: ParseTree,
}

impl MacroInfo {
    pub fn new(id: MacroId, cat: SyntaxCategoryId, pat: MacroPat, replacement: ParseTree) -> Self {
        Self {
            id,
            cat,
            pat,
            replacement,
        }
    }

    pub fn id(&self) -> MacroId {
        self.id
    }

    pub fn cat(&self) -> SyntaxCategoryId {
        self.cat
    }

    pub fn pat(&self) -> &MacroPat {
        &self.pat
    }

    pub fn replacement(&self) -> &ParseTree {
        &self.replacement
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroPat {
    parts: Vec<MacroPatPart>,
    keys: HashMap<Ustr, usize>,
}

impl MacroPat {
    pub fn new(parts: Vec<MacroPatPart>, keys: HashMap<Ustr, usize>) -> Self {
        Self { parts, keys }
    }

    pub fn parts(&self) -> &[MacroPatPart] {
        &self.parts
    }

    pub fn keys(&self) -> &HashMap<Ustr, usize> {
        &self.keys
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MacroPatPart {
    Cat(SyntaxCategoryId),
    Lit(Ustr),
    Kw(Ustr),
    Name,
}

impl MacroPatPart {
    pub fn matches_atom_pat(self, atom_pat: AtomPattern) -> bool {
        match (self, atom_pat) {
            (MacroPatPart::Lit(lit), AtomPattern::Lit(atom_lit)) => lit == atom_lit,
            (MacroPatPart::Kw(kw), AtomPattern::Kw(atom_kw)) => kw == atom_kw,
            (MacroPatPart::Name, AtomPattern::Name) => true,
            _ => false,
        }
    }
}
