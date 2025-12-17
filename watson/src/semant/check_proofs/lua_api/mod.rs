use crate::{
    context::{Arenas, Ctx},
    diagnostics::{Diagnostic, WResult},
    semant::check_proofs::{
        LuaTheoremInfo,
        lua_api::{
            ctx_to_lua::LuaCtx,
            diag_to_lua::LuaDiagnosticMeta,
            file_loader::LuaFileRequirer,
            frag_map_to_lua::{LuaFactMapMeta, LuaFragMapMeta},
            frag_to_lua::LuaPresFactMeta,
            tactic_to_lua::generate_luau_tactic_types,
            theorem_to_lua::LuaTheoremMeta,
        },
    },
    util::ansi::{ANSI_BOLD, ANSI_RESET, ANSI_YELLOW},
};
use mlua::{Lua, LuaOptions, StdLib};
use std::{fs, ops::Deref};

pub mod ctx_to_lua;
pub mod diag_to_lua;
mod file_loader;
pub mod formal_to_lua;
pub mod frag_map_to_lua;
pub mod frag_to_lua;
pub mod notation_to_lua;
pub mod proof_to_lua;
pub mod scope_to_lua;
pub mod span_to_lua;
pub mod tactic_info_to_lua;
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

impl<'ctx> Diagnostic<'ctx> {
    pub fn err_lua_load_error<T>(error: mlua::Error) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(&format!("lua error while loading:\n {error}."));

        Err(vec![diag])
    }

    pub fn err_bad_module_ret<T>(got: &mlua::Value) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(&format!(
            "bad return value from main lua module.
Expected main module to return a lua table with field `handleTactic`.
Instead it returned:
{got:#?}"
        ));

        Err(vec![diag])
    }

    pub fn err_lua_execution_error<T>(lua_ctx: &str, error: mlua::Error) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(&format!("lua error executing {lua_ctx}:\n{error}"));

        Err(vec![diag])
    }
}

#[derive(Debug)]
pub struct LuaInfo<'ctx> {
    pub runtime: WLua<'ctx>,
    pub handle_tactic_fn: mlua::Function,
}

pub fn setup_lua<'ctx>(ctx: &Ctx<'ctx>) -> WResult<'ctx, LuaInfo<'ctx>> {
    // Write out types
    write_luau_types(ctx);

    // Initialize the Lua runtime.
    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::BIT | StdLib::MATH,
        LuaOptions::new(),
    )
    .or_else(Diagnostic::err_lua_load_error)?;

    // Add the ctx as app data.
    let lua_ctx = LuaCtx::new(ctx);
    lua.set_app_data(lua_ctx);

    // Set up the custom log function
    add_log_fn(&lua);

    // Set up metatables.
    lua.globals().set("Diagnostic", LuaDiagnosticMeta).unwrap();
    lua.globals().set("Fact", LuaPresFactMeta).unwrap();
    lua.globals().set("FragMap", LuaFragMapMeta).unwrap();
    lua.globals().set("FactMap", LuaFactMapMeta).unwrap();
    lua.globals().set("Theorem", LuaTheoremMeta).unwrap();

    // Set up our custom require system.
    let src_folder = ctx.config.project_dir().join("src");
    let require = LuaFileRequirer::new(src_folder.clone());
    let require = lua.create_require_function(require).unwrap();
    lua.globals().set("require", require).unwrap();

    // Load the root file
    let lua_root = src_folder.join("main.luau");
    let chunk = lua.load(lua_root).set_name("@main");
    let result = chunk.call(()).or_else(Diagnostic::err_lua_load_error)?;

    let wlua = WLua {
        lua,
        _arenas: ctx.arenas,
    };

    read_main_module(wlua, result)
}

fn add_log_fn(lua: &Lua) {
    // Create custom log function
    let log_fn = lua
        .create_function(move |lua, args: mlua::Variadic<mlua::Value>| {
            let info = lua.app_data_ref::<LuaTheoremInfo>().unwrap();
            let mut info = info.borrow_mut();
            if !info.has_logs {
                eprintln!(
                    "{ANSI_BOLD}{ANSI_YELLOW}logs for {}{ANSI_RESET}",
                    info.thm.out().name()
                );
                info.has_logs = true;
            }
            drop(info);

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

    // Set up global functions
    lua.globals().set("log", log_fn).unwrap();
}

fn read_main_module<'ctx>(lua: WLua<'ctx>, module: mlua::Value) -> WResult<'ctx, LuaInfo<'ctx>> {
    let table = module
        .as_table()
        .ok_or_else(|| Diagnostic::err_bad_module_ret::<()>(&module).unwrap_err())?;
    let handle_tactic_fn: mlua::Function = table
        .get("handleTactic")
        .map_err(|_| Diagnostic::err_bad_module_ret::<()>(&module).unwrap_err())?;

    Ok(LuaInfo {
        runtime: lua,
        handle_tactic_fn,
    })
}

fn write_luau_types<'ctx>(ctx: &Ctx<'ctx>) {
    let definitions_file = include_str!("./definitions.d.luau");
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
        fs::write(types_path, new_def_file).expect("TODO");
    }
}
