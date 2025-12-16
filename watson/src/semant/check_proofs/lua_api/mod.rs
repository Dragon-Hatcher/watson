use crate::{
    context::{Arenas, Ctx},
    diagnostics::{DiagManager, Diagnostic, WResult},
    semant::check_proofs::lua_api::{
        ctx_to_lua::LuaCtx, file_loader::LuaFileRequirer, tactic_to_lua::generate_luau_tactic_types,
    },
};
use mlua::{Lua, LuaOptions, StdLib};
use std::{fs, ops::Deref};

pub mod ctx_to_lua;
mod file_loader;
pub mod frag_to_lua;
pub mod proof_to_lua;
pub mod scope_to_lua;
pub mod tactic_to_lua;
pub mod theorem_to_lua;
pub mod unresolved_to_lua;

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

    pub fn err_lua_execution_error<T>(&mut self, lua_ctx: &str, error: mlua::Error) -> WResult<T> {
        let diag = Diagnostic::new(&format!("lua error executing {lua_ctx}:\n{error}"));

        self.add_diag(diag);
        Err(())
    }
}

#[derive(Debug)]
pub struct LuaInfo<'ctx> {
    pub runtime: WLua<'ctx>,
    pub handle_tactic_fn: mlua::Function,
}

pub fn setup_lua<'ctx>(ctx: &mut Ctx<'ctx>) -> WResult<LuaInfo<'ctx>> {
    // Write out types
    write_luau_types(ctx);

    // Initialize the Lua runtime.
    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::BIT | StdLib::MATH,
        LuaOptions::new(),
    )
    .or_else(|e| ctx.diags.err_lua_load_error(e))?;

    // Add the ctx as app data.
    let lua_ctx = LuaCtx::new(ctx);
    lua.set_app_data(lua_ctx);

    // Create custom log function
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
            let str = log_parts.join(" ");
            eprintln!("{str}");
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

    read_main_module(wlua, result, ctx)
}

fn read_main_module<'ctx>(
    lua: WLua<'ctx>,
    module: mlua::Value,
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
    })
}

fn write_luau_types<'ctx>(ctx: &Ctx<'ctx>) {
    let definitions_file = include_str!("../../../static/definitions.d.luau");
    let types_content = generate_luau_tactic_types(&ctx.tactic_manager);
    let types_path = ctx
        .config
        .build_dir()
        .join("luau")
        .join("definitions.d.luau");

    let current_def_file = fs::read_to_string(&types_path).unwrap_or_default();
    let new_def_file = format!("{definitions_file}\n{types_content}");

    // Only write if it has actually changed as it confused the LSP.
    if current_def_file != new_def_file {
        println!("writing!");
        fs::write(types_path, new_def_file).expect("TODO");
    }
}
