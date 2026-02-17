use crate::semant::{
    attributes::AttributeTracker, check_proofs::lua_api::command_to_lua::LuaCommandId,
};
use mlua::{FromLua, IntoLua, UserData, Value};

#[derive(Debug, Clone, FromLua)]
pub struct LuaAttributeTracker {
    tracker: AttributeTracker<'static>,
}

impl LuaAttributeTracker {
    pub fn new<'ctx>(tracker: AttributeTracker<'ctx>) -> Self {
        // SAFETY: This isn't actually safe the way we have set this up. But!
        // as long as we only use these objects inside lua, since the lua
        // runtime doesn't live for as long as context, this is safe.
        let tracker: AttributeTracker<'static> = unsafe { std::mem::transmute(tracker) };
        Self { tracker }
    }

    pub fn out_ref<'ctx>(&self) -> &AttributeTracker<'ctx> {
        // SAFETY: see above.
        unsafe { std::mem::transmute(&self.tracker) }
    }
}

impl UserData for LuaAttributeTracker {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getAttributes", |lua, this, cmd: LuaCommandId| {
            let attrs = this.out_ref().get(cmd.out());

            let mut lua_attrs: Vec<Value> = Vec::new();
            for attr in attrs {
                lua_attrs.push(attr.0.into_lua(lua)?);
            }

            Ok(lua_attrs)
        });
    }
}

pub struct LuaAttributeTrackerMeta;

impl UserData for LuaAttributeTrackerMeta {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("atEnd", |lua, _, _: ()| {
            let tracker = lua.app_data_ref::<LuaAttributeTracker>().unwrap();
            Ok(tracker.clone())
        });
    }
}
