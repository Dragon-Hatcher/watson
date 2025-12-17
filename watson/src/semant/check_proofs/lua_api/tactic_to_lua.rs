use crate::{
    semant::{
        check_proofs::lua_api::{
            span_to_lua::LuaSpan,
            unresolved_to_lua::{LuaUnresolvedAnyFrag, LuaUnresolvedFact, LuaUnresolvedFrag},
        },
        tactic::{
            syntax::TacticPatPartCore,
            tactic_manager::TacticManager,
            unresolved_proof::{SpannedStr, TacticInst, TacticInstPart},
        },
    },
    strings,
};
use mlua::{IntoLua, Lua, UserData, Value};

impl<'ctx> IntoLua for &TacticInst<'ctx> {
    fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
        let table = lua.create_table()?;

        let rule = self.rule();
        table.set("_rule", rule.name().as_str())?;
        table.set("_span", LuaSpan::new(self.span()))?;

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
                let value = child.into_lua(lua)?;
                table.set(label.as_str(), value)?;
            }
        }

        Ok(Value::Table(table))
    }
}

impl<'ctx> IntoLua for &TacticInstPart<'ctx> {
    fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
        match self {
            TacticInstPart::Kw(s) | TacticInstPart::Lit(s) | TacticInstPart::Name(s) => {
                s.into_lua(lua)
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

impl UserData for SpannedStr {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("str", |_, this| Ok(this.str().to_string()));

        fields.add_field_method_get("span", |_, this| Ok(LuaSpan::new(this.span())));
    }
}

pub fn generate_luau_tactic_types<'ctx>(tactics: &TacticManager<'ctx>) -> String {
    let mut out = String::new();

    for &cat in tactics.cats() {
        let name = cat.lua_name();
        let rules = tactics.rules_for_cat(cat);

        if rules.is_empty() {
            out.push_str(&format!("export type {name} = never\n\n"));
            continue;
        }

        out.push_str(&format!("export type {name} =\n"));
        for rule in rules {
            let rule_name = rule.name();
            out.push_str(&format!("  | {{ _rule: \"{rule_name}\", _span: Span"));

            for part in rule.pattern().parts() {
                use TacticPatPartCore as C;

                let Some(label) = part.label() else {
                    continue;
                };

                let luau_type = match part.part() {
                    C::Lit(_) | C::Kw(_) | C::Name => *strings::SPANNED_STRING,
                    C::Cat(cat) => cat.lua_name(),
                    C::Frag(_) => *strings::UN_FRAG,
                    C::AnyFrag => *strings::UN_ANY_FRAG,
                    C::Fact => *strings::UN_FACT,
                };

                out.push_str(&format!(", {label}: {luau_type}"));
            }

            out.push_str(" }\n");
        }
        out.push('\n');
    }

    out
}
