use crate::parse::{parse_state::CategoryId, parse_tree::ParseTree};
use rustc_hash::FxHashMap;
use slotmap::{SlotMap, new_key_type};
use std::{collections::HashMap, ops::Index};
use ustr::Ustr;

pub struct Macros {
    macros: SlotMap<MacroId, MacroInfo>,
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

    pub fn add_macro(&mut self, info: MacroInfo) -> MacroId {
        let name = info.name;
        let id = self.macros.insert(info);
        self.by_name.insert(name, id);
        id
    }

    pub fn macros(&self) -> impl Iterator<Item = &MacroInfo> {
        self.macros.values()
    }
}

impl Index<MacroId> for Macros {
    type Output = MacroInfo;

    fn index(&self, index: MacroId) -> &Self::Output {
        &self.macros[index]
    }
}

pub struct MacroInfo {
    name: Ustr,
    cat: CategoryId,
    pat: MacroPat,
    replacement: ParseTree,
}

impl MacroInfo {
    pub fn new(name: Ustr, cat: CategoryId, pat: MacroPat, replacement: ParseTree) -> Self {
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

// TODO
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MacroPatPart {
    Cat(CategoryId),
    TempCat(CategoryId),
    Lit(Ustr),
    Kw(Ustr),
    Name,
}
