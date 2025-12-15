# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Watson is a proof assistant written in Rust with an extensible syntax system and Lua-based tactics. The project consists of:
- `watson/` - Core Rust implementation of the proof assistant
- `lib/` - Watson source files (`.wats`) containing example proofs and logic definitions
- `vscode-extension/` - VSCode language extension for Watson syntax

## Build and Development Commands

### Building the Project

```bash
# Build the Watson CLI
cd watson && cargo build

# Build in release mode
cd watson && cargo build --release

# Run the Watson CLI directly
cargo run --manifest-path watson/Cargo.toml -- <command>
```

### Running Watson

```bash
# Check proofs in a Watson project
watson/target/debug/watson check

# Check proofs with watch mode (recheck on file changes)
watson/target/debug/watson check -w

# Check with specific config file
watson/target/debug/watson check -c path/to/watson.toml

# Create a new Watson project
watson/target/debug/watson new <project-name>
```

### VSCode Extension

```bash
# Install dependencies
cd vscode-extension && npm install

# Compile TypeScript
cd vscode-extension && npm run compile

# Watch mode for development
cd vscode-extension && npm run watch

# Run linter
cd vscode-extension && npm run lint

# Run tests
cd vscode-extension && npm test
```

## Architecture

### Core Components

**Parsing Pipeline** (`watson/src/parse/`)
- **Earley parser** (`earley.rs`) - Generalized parsing algorithm
- **Elaborator** (`elaborator.rs`) - Converts parse trees into semantic structures
- **Grammar** (`grammar.rs`) - Dynamic grammar construction from syntax declarations
- **Parse State** (`parse_state.rs`) - Tracks available syntax categories and rules during parsing
- Watson has an extensible syntax system where `.wats` files can define new syntax categories and rules that immediately become available for parsing subsequent code

**Semantic Analysis** (`watson/src/semant/`)
- **Proof Kernel** (`proof_kernel.rs`) - Core proof checking with `ProofState` and `ProofCertificate`
- **Formal Syntax** (`formal_syntax.rs`) - Formal language categories and rules
- **Notation** (`notation.rs`) - User-defined notation patterns with precedence/associativity
- **Fragments** (`fragment.rs`) - Syntax fragments representing terms and sentences
- **Theorems** (`theorems.rs`) - Theorem statements and template handling
- **Tactics** (`tactic/`) - Proof tactics implemented in Lua
- **Check Proofs** (`check_proofs/`) - Lua integration for tactic execution

**Context Management** (`watson/src/context/`)
- **Ctx** - Central context object containing arenas, parse state, diagnostics, source cache, and config
- **Arenas** - Memory arenas for efficient allocation of interned AST nodes and semantic objects

**CLI** (`watson/src/cli/`)
- `check_command.rs` - Implements proof checking with optional watch mode
- `new_command.rs` - Creates new Watson projects

### Key Architectural Patterns

1. **Dynamic Syntax Extension**: Watson files can declare new syntax categories (e.g., `syntax_category term`) and syntax rules (e.g., `syntax syntax.equality`) that extend the parser's grammar during parsing. This allows domain-specific notation to be defined within Watson source files.

2. **Arena Allocation**: Heavy use of typed arenas (`typed_arena` crate) for allocating AST nodes and semantic objects with stable references using lifetimes.

3. **Proof Kernel Safety**: The proof kernel uses a `safe` inner module with `SafeFrag` and `SafeFact` types to ensure only well-formed, validated fragments can participate in proofs.

4. **Lua-Rust Bridge**: Tactics are executed via Lua (using `mlua` crate with Luau). The Rust proof state is converted to Lua, tactics manipulate it, and results are converted back to Rust for verification.

5. **Incremental Parsing**: The parser processes source files line-by-line, maintaining a stack of parsing locations and handling commands that can load new modules or extend the grammar.

6. **Project Structure**: Watson projects have a `watson.toml` config file at the root and a `src/main.wats` entry point. The config file is found by searching up the directory tree from the current directory.

## Watson Language Concepts

### Commands
Watson source files consist of commands that declare:
- `module` - Import other Watson files
- `syntax_category` - Declare new syntax categories
- `syntax` - Define syntax rules for formal languages
- `notation` - Define notation patterns (syntactic sugar)
- `definition` - Define term-level macros
- `axiom` - Declare axioms with proof obligations
- `theorem` - State and prove theorems
- `tactic_category` - Declare tactic syntax categories
- `tactic` - Define new proof tactics

### Syntax System
- Categories have names and can be either formal language categories or tactic categories
- Rules map patterns to categories with precedence and associativity
- Patterns can include literals, keywords, names, variables, bindings, and templates
- The parser is dynamically extended as syntax declarations are processed

### Proof Checking
- Theorems have hypotheses and a conclusion (separated by `|-`)
- Proofs are written using tactics in `proof ... qed` blocks
- The proof kernel maintains a `ProofState` with known facts and assumptions
- Proofs must derive the theorem's conclusion from its hypotheses to succeed
- Circular dependencies between theorems are detected and reported

## Common Patterns

### Adding New Builtin Syntax
When adding builtin syntax rules, update both:
1. `watson/src/parse/grammar.rs` - Add the rule in `add_builtin_rules()`
2. `watson/src/context/mod.rs` - Add rule ID to `BuiltinRules` struct

### Working with Fragments
Fragments represent syntax tree nodes. They have:
- A category (e.g., `sentence_cat`, or user-defined categories)
- A head (which notation pattern was used)
- Children (bound sub-fragments)
- Metadata (holes, unclosed bindings)

Use `SafeFrag::new()` to validate fragments before using them in proofs.

### Error Reporting
Use `ctx.diags` methods to report errors:
- Errors are accumulated in `DiagManager` and printed at the end
- Include source location spans for accurate error reporting
- Check `ctx.diags.has_errors()` to determine if compilation succeeded

### Working with Arenas
Objects allocated in arenas return IDs (e.g., `TheoremId<'ctx>`, `FragmentId<'ctx>`). These IDs can be used to retrieve the object later via the arena and support efficient equality checks and hashing.
