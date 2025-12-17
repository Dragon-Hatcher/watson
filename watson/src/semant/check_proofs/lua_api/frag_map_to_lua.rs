use crate::semant::check_proofs::lua_api::frag_to_lua::{LuaPresFact, LuaPresFrag};
use mlua::{FromLua, UserData};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, FromLua)]
struct LuaFragMap {
    map: FxHashMap<LuaPresFrag, mlua::Value>,
}

impl LuaFragMap {
    fn new() -> Self {
        Self {
            map: FxHashMap::default(),
        }
    }
}

impl UserData for LuaFragMap {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get", |_, this, f: LuaPresFrag| {
            Ok(this.map.get(&f).cloned())
        });

        methods.add_method_mut("set", |_, this, (f, v): (LuaPresFrag, mlua::Value)| {
            Ok(this.map.insert(f, v))
        });

        methods.add_method("copy", |_, this, _: ()| Ok(this.clone()));
    }
}

pub struct LuaFragMapMeta;

impl UserData for LuaFragMapMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("new", |_, _, _: ()| Ok(LuaFragMap::new()));
    }
}

#[derive(Debug, Clone, FromLua)]
struct LuaFactMap {
    map: FxHashMap<LuaPresFact, mlua::Value>,
}

impl LuaFactMap {
    fn new() -> Self {
        Self {
            map: FxHashMap::default(),
        }
    }
}

impl UserData for LuaFactMap {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("get", |_, this, f: LuaPresFact| {
            Ok(this.map.get(&f).cloned())
        });

        methods.add_method_mut("set", |_, this, (f, v): (LuaPresFact, mlua::Value)| {
            Ok(this.map.insert(f, v))
        });

        methods.add_method("copy", |_, this, _: ()| Ok(this.clone()));
    }
}

pub struct LuaFactMapMeta;

impl UserData for LuaFactMapMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("new", |_, _, _: ()| Ok(LuaFactMap::new()));
    }
}
