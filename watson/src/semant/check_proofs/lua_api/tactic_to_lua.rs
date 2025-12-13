use crate::semant::{
    check_proofs::lua_api::unresolved_to_lua::{
        LuaUnresolvedAnyFrag, LuaUnresolvedFact, LuaUnresolvedFrag,
    },
    tactic::unresolved_proof::{TacticInst, TacticInstPart},
};
use mlua::{IntoLua, Lua, Value};

impl<'ctx> IntoLua for &TacticInst<'ctx> {
    fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
        let table = lua.create_table()?;

        // Add the rule name under _rule (reserved key)
        let rule = self.rule();
        table.set("_rule", rule.name().as_str())?;

        // Get the pattern parts from the rule
        let pattern = rule.pattern();
        let pattern_parts = pattern.parts();
        let children = self.children();

        // Every pattern part has a corresponding child (including Lit/Kw which are NoInstantiation)
        // We only add fields to the table for parts that:
        // 1. Have a label
        // 2. Are not NoInstantiation
        for (pattern_part, child) in pattern_parts.iter().zip(children.iter()) {
            // Skip if no label or if it's a NoInstantiation
            if let Some(label) = pattern_part.label() {
                if !matches!(child, TacticInstPart::NoInstantiation) {
                    let value = child.into_lua(lua)?;
                    table.set(label.as_str(), value)?;
                }
            }
        }

        Ok(Value::Table(table))
    }
}

impl<'ctx> IntoLua for &TacticInstPart<'ctx> {
    fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
        match self {
            TacticInstPart::NoInstantiation => {
                // This shouldn't be converted, but return nil if it happens
                Ok(Value::Nil)
            }
            TacticInstPart::Name(name) => {
                // Names are converted to Lua strings
                name.as_str().into_lua(lua)
            }
            TacticInstPart::SubInst(sub_tactic) => {
                // Recursively convert sub-tactics to tables
                sub_tactic.into_lua(lua)
            }
            TacticInstPart::Frag(frag) => {
                let lua_frag = LuaUnresolvedFrag::new(*frag);
                lua_frag.into_lua(lua)
            }
            TacticInstPart::AnyFrag(any_frag) => {
                let lua_any_frag = LuaUnresolvedAnyFrag::new(*any_frag);
                lua_any_frag.into_lua(lua)
            }
            TacticInstPart::Fact(fact) => {
                let lua_fact = LuaUnresolvedFact::new(fact);
                lua_fact.into_lua(lua)
            }
        }
    }
}
