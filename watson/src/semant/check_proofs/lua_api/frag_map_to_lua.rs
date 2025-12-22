use crate::semant::check_proofs::lua_api::frag_to_lua::{LuaPresFact, LuaPresFrag};
use mlua::{FromLua, MetaMethod, UserData};

#[derive(Debug, Clone, FromLua)]
struct LuaFragMap {
    map: im::HashMap<LuaPresFrag, mlua::Value>,
}

impl LuaFragMap {
    fn new() -> Self {
        Self {
            map: im::HashMap::new(),
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

        methods.add_meta_method(MetaMethod::Iter, |_, this, _: ()| {
            Ok(LuaFragMapIter {
                iter: this.map.clone().into_iter(),
            })
        });
    }
}

pub struct LuaFragMapIter {
    iter: im::hashmap::ConsumingIter<(LuaPresFrag, mlua::Value)>,
}

impl UserData for LuaFragMapIter {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method_mut(MetaMethod::Call, |_, this, _: ()| {
            Ok(this.iter.next().unzip())
        });
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
    map: im::HashMap<LuaPresFact, mlua::Value>,
}

impl LuaFactMap {
    fn new() -> Self {
        Self {
            map: im::HashMap::new(),
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

        methods.add_meta_method(MetaMethod::Iter, |_, this, _: ()| {
            Ok(LuaFactMapIter {
                iter: this.map.clone().into_iter(),
            })
        });
    }
}

pub struct LuaFactMapIter {
    iter: im::hashmap::ConsumingIter<(LuaPresFact, mlua::Value)>,
}

impl UserData for LuaFactMapIter {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method_mut(MetaMethod::Call, |_, this, _: ()| {
            Ok(this.iter.next().unzip())
        });
    }
}

pub struct LuaFactMapMeta;

impl UserData for LuaFactMapMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("new", |_, _, _: ()| Ok(LuaFactMap::new()));
    }
}
