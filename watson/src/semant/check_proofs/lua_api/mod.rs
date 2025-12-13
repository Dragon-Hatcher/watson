use crate::{
    context::{Arenas, Ctx},
    diagnostics::{DiagManager, Diagnostic, WResult},
    semant::check_proofs::lua_api::file_loader::LuaFileRequirer,
};
use mlua::{Lua, LuaOptions, StdLib};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

mod file_loader;
mod tactic_to_lua;
mod unresolved_to_lua;

pub struct WLua<'ctx> {
    lua: Lua,
    // We don't actually need this for anything, but we are going to be using
    // some unsafe stuff to pass all of our 'ctx lifetime objects into lua, so
    // we want to make sure that our Lua runtime doesn't live longer than 'ctx.
    _arenas: &'ctx Arenas<'ctx>,
}

impl<'ctx> std::fmt::Debug for WLua<'ctx> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WLua").field(&self.lua).finish()
    }
}

impl<'ctx> Deref for WLua<'ctx> {
    type Target = Lua;

    fn deref(&self) -> &Self::Target {
        &self.lua
    }
}

impl<'ctx> DiagManager<'ctx> {
    pub fn err_lua_load_error<T>(&mut self, error: mlua::Error) -> WResult<T> {
        let diag = Diagnostic::new(&format!("lua error while loading:\n {error}."));

        self.add_diag(diag);
        Err(())
    }

    pub fn err_bad_module_ret<T>(&mut self, got: &mlua::Value) -> WResult<T> {
        let diag = Diagnostic::new(&format!(
            "bad return value from main lua module.
Expected main module to return a lua table with field `handleTactic`.
Instead it returned: 
{got:#?}"
        ));

        self.add_diag(diag);
        Err(())
    }
}

#[derive(Debug)]
pub struct LuaInfo<'ctx> {
    pub runtime: WLua<'ctx>,
    pub handle_tactic_fn: mlua::Function,
    pub logs: Rc<RefCell<Vec<String>>>,
}

impl<'ctx> LuaInfo<'ctx> {
    pub fn clear_logs(&self) {
        self.logs.borrow_mut().clear();
    }

    pub fn get_logs(&self) -> Vec<String> {
        self.logs.borrow().clone()
    }
}

pub fn setup_lua<'ctx>(ctx: &mut Ctx<'ctx>) -> WResult<LuaInfo<'ctx>> {
    // Initialize the Lua runtime.
    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::BIT | StdLib::MATH,
        LuaOptions::new(),
    )
    .or_else(|e| ctx.diags.err_lua_load_error(e))?;

    // Set up log storage
    let logs = Rc::new(RefCell::new(Vec::new()));

    // Create custom log function
    let logs_clone = Rc::clone(&logs);
    let log_fn = lua
        .create_function(move |_lua, args: mlua::Variadic<mlua::Value>| {
            let mut log_parts = Vec::new();
            for arg in args.iter() {
                let formatted = match arg {
                    mlua::Value::String(s) => match s.to_str() {
                        Ok(string) => string.to_string(),
                        Err(_) => "<invalid utf8>".to_string(),
                    },
                    _ => format!("{arg:#?}"),
                };
                log_parts.push(formatted);
            }
            logs_clone.borrow_mut().push(log_parts.join(" "));
            Ok(())
        })
        .unwrap();

    lua.globals().set("log", log_fn).unwrap();

    // Set up our custom require system.
    let src_folder = ctx.config.project_dir().join("src");
    let require = LuaFileRequirer::new(src_folder.clone());
    let require = lua.create_require_function(require).unwrap();
    lua.globals().set("require", require).unwrap();

    // Load the root file
    let lua_root = src_folder.join("main.luau");
    let chunk = lua.load(lua_root).set_name("@main");
    let result = chunk
        .call(())
        .or_else(|e| ctx.diags.err_lua_load_error(e))?;

    let wlua = WLua {
        lua,
        _arenas: ctx.arenas,
    };

    read_main_module(wlua, result, logs, ctx)
}

fn read_main_module<'ctx>(
    lua: WLua<'ctx>,
    module: mlua::Value,
    logs: Rc<RefCell<Vec<String>>>,
    ctx: &mut Ctx<'ctx>,
) -> WResult<LuaInfo<'ctx>> {
    let mut err_fn = || {
        _ = ctx.diags.err_bad_module_ret::<()>(&module);
    };

    let table = module.as_table().ok_or_else(|| err_fn())?;
    let handle_tactic_fn: mlua::Function = table.get("handleTactic").map_err(|_| err_fn())?;

    Ok(LuaInfo {
        runtime: lua,
        handle_tactic_fn,
        logs,
    })
}
