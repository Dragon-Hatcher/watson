use std::{
    collections::VecDeque,
    env, fs,
    path::{Component, Path, PathBuf},
};

use mlua::{Lua, LuaOptions, NavigateError, Require, StdLib};

use crate::{
    context::Ctx,
    diagnostics::WResult,
    semant::{
        proof_status::{ProofStatus, ProofStatuses},
        tactic::unresolved_proof::{TacticInst, UnresolvedProof},
        theorems::TheoremId,
    },
};

pub fn check_proofs<'ctx>(
    theorems: &[(TheoremId<'ctx>, UnresolvedProof<'ctx>)],
    ctx: &mut Ctx<'ctx>,
) -> ProofStatuses<'ctx> {
    let mut statuses = ProofStatuses::new();

    _ = match setup_lua(ctx) {
        Ok(_) => {}
        Err(_) => return statuses,
    };

    for (theorem, proof) in theorems {
        let status = match proof {
            UnresolvedProof::Axiom => ProofStatus::new_axiom(),
            UnresolvedProof::Theorem(proof) => check_theorem(*theorem, proof, ctx),
        };
        statuses.add(*theorem, status);
    }

    statuses
}

fn check_theorem<'ctx>(
    thm: TheoremId<'ctx>,
    proof: &TacticInst<'ctx>,
    ctx: &mut Ctx<'ctx>,
) -> ProofStatus<'ctx> {
    todo!()
}

fn setup_lua<'ctx>(ctx: &mut Ctx<'ctx>) -> WResult<()> {
    // Initialize the Lua runtime.
    let lua = Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::UTF8 | StdLib::BIT | StdLib::MATH,
        LuaOptions::new(),
    )
    .or_else(|e| ctx.diags.err_lua_load_error(e))?;

    // Set up our custom require system.
    let src_folder = ctx.config.project_dir().join("src");
    let require = LuaFileRequirer::new(src_folder.clone());
    let require = lua.create_require_function(require).unwrap();
    lua.globals().set("require", require).unwrap();

    // Load the root file
    let lua_root = src_folder.join("main.luau");
    let chunk = lua.load(lua_root).set_name("@main");
    let result: mlua::Value = chunk
        .call(())
        .or_else(|e| ctx.diags.err_lua_load_error(e))?;

    let tactic_fn: mlua::Function = result.as_table().unwrap().get("handleTactic").unwrap();
    let ret: mlua::Value = tactic_fn.call(()).unwrap();
    dbg!(ret);

    Ok(())
}

#[derive(Debug)]
pub struct LuaFileRequirer {
    /// The parent folder containing the code.
    src_folder: PathBuf,
    /// A relative path to the current Luau module (not mapped to a physical file)
    rel_path: PathBuf,
    /// An absolute path to the current Luau module if it exists. This is guaranteed
    /// to be a luau file.
    resolved: Option<PathBuf>,
}

impl LuaFileRequirer {
    /// The file extensions that are considered valid for Luau modules.
    const FILE_EXTENSIONS: &[&str] = &["luau", "lua"];

    /// Creates a new `TextRequirer` instance.
    pub fn new(src_folder: PathBuf) -> Self {
        Self {
            src_folder,
            rel_path: PathBuf::default(),
            resolved: None,
        }
    }

    /// Normalizes the path by removing unnecessary components
    fn normalize_path(path: &Path) -> PathBuf {
        let mut components = VecDeque::new();

        for comp in path.components() {
            match comp {
                Component::Prefix(..) | Component::RootDir => {
                    components.push_back(comp);
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    if matches!(components.back(), None | Some(Component::ParentDir)) {
                        components.push_back(Component::ParentDir);
                    } else if matches!(components.back(), Some(Component::Normal(..))) {
                        components.pop_back();
                    }
                }
                Component::Normal(..) => components.push_back(comp),
            }
        }

        if matches!(components.front(), None | Some(Component::Normal(..))) {
            components.push_front(Component::CurDir);
        }

        // Join the components back together
        components.into_iter().collect()
    }

    /// Resolve a Luau module path to a physical file or directory.
    ///
    /// Empty directories without init files are considered valid as "intermediate" directories.
    fn resolve_module(path: &Path) -> Result<Option<PathBuf>, NavigateError> {
        if path.is_dir() {
            return Ok(None);
        }

        for ext in Self::FILE_EXTENSIONS {
            let with_extension = path.with_extension(ext);
            if with_extension.is_file() {
                return Ok(Some(with_extension));
            }
        }

        Err(NavigateError::NotFound)
    }
}

impl Require for LuaFileRequirer {
    fn is_require_allowed(&self, _chunk_name: &str) -> bool {
        true
    }

    fn reset(&mut self, chunk_name: &str) -> Result<(), NavigateError> {
        // Slice to remove the @ from the start.
        let chunk_path = Self::normalize_path(chunk_name[1..].as_ref());

        let abs_path = self.src_folder.join(chunk_path.clone());
        let resolved = Self::resolve_module(&abs_path)?;

        self.rel_path = chunk_path;
        self.resolved = resolved;

        Ok(())
    }

    fn jump_to_alias(&mut self, _path: &str) -> Result<(), NavigateError> {
        unreachable!()
    }

    fn to_parent(&mut self) -> Result<(), NavigateError> {
        let mut rel_path = self.rel_path.clone();
        if !rel_path.pop() {
            // It's important to return `NotFound` if we reached the root, as it's a "recoverable" error if we
            // cannot go beyond the root directory.
            // Luau "require-by-string` has a special logic to search for config file to resolve aliases.
            return Err(NavigateError::NotFound);
        }

        let abs = self.src_folder.join(rel_path.clone());
        let resolved = Self::resolve_module(&abs)?;
        self.rel_path = rel_path;
        self.resolved = resolved;

        Ok(())
    }

    fn to_child(&mut self, name: &str) -> Result<(), NavigateError> {
        let rel_path = self.rel_path.join(name);
        let abs = self.src_folder.join(rel_path.clone());
        let resolved = Self::resolve_module(&abs)?;

        self.rel_path = rel_path;
        self.resolved = resolved;

        Ok(())
    }

    fn has_module(&self) -> bool {
        self.resolved.is_some()
    }

    fn cache_key(&self) -> String {
        self.rel_path.display().to_string()
    }

    fn has_config(&self) -> bool {
        false
    }

    fn config(&self) -> std::io::Result<Vec<u8>> {
        unreachable!()
    }

    fn loader(&self, lua: &Lua) -> mlua::Result<mlua::Function> {
        let name = format!("@{}", self.rel_path.display());
        lua.load(self.resolved.as_deref().unwrap())
            .set_name(name)
            .into_function()
    }
}
