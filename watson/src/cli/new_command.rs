use crate::util::ansi::{ANSI_BOLD, ANSI_GREEN, ANSI_RED, ANSI_RESET};
use argh::FromArgs;
use std::{fs, path::PathBuf};

/// Create a new Watson project.
#[derive(FromArgs)]
#[argh(subcommand, name = "new")]
pub struct NewCommand {
    /// the name of the project to create
    #[argh(positional)]
    name: String,
}

pub fn run_new(cmd: NewCommand) {
    match run(cmd) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("{ANSI_RED}{ANSI_BOLD}error:{ANSI_RESET} {err}");
            std::process::exit(1);
        }
    }
}

fn run(cmd: NewCommand) -> Result<(), std::io::Error> {
    let project_path = PathBuf::from(&cmd.name);

    // Create project directory
    if project_path.exists() {
        eprintln!(
            "{ANSI_RED}{ANSI_BOLD}error:{ANSI_RESET} directory '{}' already exists",
            cmd.name
        );
        std::process::exit(1);
    }

    fs::create_dir(&project_path).unwrap_or_else(|e| {
        eprintln!(
            "{ANSI_RED}{ANSI_BOLD}error{ANSI_RESET} creating directory '{}': {}",
            cmd.name, e
        );
        std::process::exit(1);
    });

    // Create src directory
    let src_path = project_path.join("src");
    fs::create_dir(&src_path)?;

    // Create watson.toml (empty file)
    let toml_path = project_path.join("watson.toml");
    fs::write(&toml_path, "")?;

    // Create src/main.luau with default content
    let main_luau_path = src_path.join("main.luau");
    let main_luau_content = r#"local M = {}
    
function M.handleTactic(tactic: Tactic, proofState: ProofState)
    local thm = proofState.theorem
    return proofState:applyTodo(thm.conclusion)
end

return M
"#;
    fs::write(&main_luau_path, main_luau_content)?;

    // Create src/main.wats with default content
    let main_wats_path = src_path.join("main.wats");
    let main_wats_content = r#"
"#;
    fs::write(&main_wats_path, main_wats_content)?;

    // Create .vscode directory
    let vscode_path = project_path.join(".vscode");
    fs::create_dir(&vscode_path)?;

    let vscode_settings_path = vscode_path.join("settings.json");
    let settings_content = r#"{
    "luau-lsp.platform.type": "standard",
    "luau-lsp.sourcemap.enabled": false,
    "luau-lsp.types.definitionFiles": {
        "watson": "build/luau/definitions.d.luau"
    },
    "luau-lsp.fflags.enableNewSolver": true,
    "luau-lsp.server.baseLuaurc": "build/luau/.luaurc"
}
"#;
    fs::write(&vscode_settings_path, settings_content)?;

    let build_path = project_path.join("build");
    fs::create_dir(&build_path)?;

    let luau_path = build_path.join("luau");
    fs::create_dir(&luau_path)?;

    let definitions_path = luau_path.join("definitions.d.luau");
    let definitions_content = r#"declare function log(...: any): number

declare class UnResFrag
end

declare class UnResAnyFrag
end

declare class UnResFact
    assumption: UnResFrag?
    conclusion: UnResFrag
end

declare class Frag
    formal: Frag
end

declare class Fact
end

declare class Scope
end

declare class Theorem
    name: string
    hypotheses: {Fact}
    conclusion: Frag
end

declare class ProofState
    theorem: Theorem
    function addAssumption(self, assumption: Frag): ProofState
    function popAssumption(self, justifying: Frag): ProofState
    function applyTheorem(self, thm: Theorem, templates: {Frag}): ProofState
    function applyTodo(self, justifying: Frag): ProofState
end
"#;
    fs::write(&definitions_path, definitions_content)?;

    let luau_rc_path = luau_path.join(".luaurc");
    let luau_rc_content = r#"{
    "languageMode": "strict"
}
"#;
    fs::write(&luau_rc_path, luau_rc_content)?;

    let gitignore_path = project_path.join(".gitignore");
    let gitignore_content = "build/\n";
    fs::write(&gitignore_path, gitignore_content)?;

    println!(
        "{ANSI_GREEN}{ANSI_BOLD}Created{ANSI_RESET} Watson project '{}'",
        cmd.name
    );

    Ok(())
}
