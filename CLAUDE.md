# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Watson is a proof assistant written in Rust that enables users to define formal languages, state theorems, and provide proofs. Users can define custom syntax for logical propositions, declare axioms and theorems, and write tactic-based proofs that reference previously proven results.

## Build and Run Commands

```bash
# Build the project
cargo build --manifest-path=watson/Cargo.toml

# Run Watson on a .wats file
./watson/target/debug/watson lib/bool.wats

# Or using cargo run
cargo run --manifest-path=watson/Cargo.toml -- lib/bool.wats

# Watch mode (recheck on file changes)
cargo run --manifest-path=watson/Cargo.toml -- lib/bool.wats --watch
```

The main Cargo.toml is located at `watson/Cargo.toml`, not at the repository root.

## High-Level Architecture

Watson follows a pipeline: **Parse → Semantic Analysis → Proof Verification → Reporting**

### 1. Parse Phase (`watson/src/parse/`)

Converts `.wats` source files into structured parse trees using an **Earley parser** with dynamic grammar.

- **`earley.rs`**: Core Earley parsing algorithm with predict/scan/complete steps
- **`grammar.rs`**: Manages grammar rules and converts high-level patterns to parse rules
  - `add_parse_rules_for_notation()`: User-defined syntax becomes parse rules
  - `add_parse_rules_for_tactic_rule()`: Tactic patterns become parse rules
- **`parse_state.rs`**: Tracks which rules apply to which categories, computes grammar properties (nullable, first-sets)
- **`elaborator.rs`**: Converts parse trees to semantic actions (`ElaborateAction` enum)

**Key insight**: Grammar is extended dynamically during compilation. New `syntax` and `notation` commands add rules that affect subsequent parsing.

### 2. Semantic Analysis (`watson/src/semant/`)

Builds semantic representations and manages symbols.

#### Core Data Structures

- **`fragment.rs`**: `Fragment<'ctx>` is the semantic representation of formal syntax trees
  - Contains: category, head (RuleApplication/Variable/TemplateRef/Hole), children
  - Tracks: holes, template references, unclosed variable bindings
  - `Fact<'ctx>`: Represents hypothetical judgments "assumption |- conclusion"

- **`scope.rs`**: `Scope<'ctx>` is an immutable map from notation bindings to their definitions
  - Functional/persistent: `scope.child_with()` creates new scope without mutating old
  - Tracks `binding_depth` for de Bruijn index adjustment

- **`theorems.rs`**: `TheoremStatement<'ctx>` contains templates, hypotheses, conclusion, and proof
  - `Template<'ctx>`: Template parameters with holes for instantiation
  - `PresFact<'ctx>`: Paired fragment and presentation for display

- **`presentation.rs`**: `Pres<'ctx>` and `PresTree<'ctx>` handle display layer
  - Separates semantic meaning from how it's displayed
  - `PresFrag<'ctx>`: Paired (Fragment, PresTree)

- **`parse_fragment.rs`**: Bridges parse trees to fragments
  - `parse_fragment()`: Converts parsed notation to `PresFrag`
  - Resolves notation bindings through scope
  - Instantiates template parameters with holes

#### de Bruijn Indices

Variables use de Bruijn indexing for binding:
- `FragHead::Variable(cat, db_idx)` where `db_idx=0` means bound by innermost binding
- Adjusted during substitution to prevent variable capture
- `unclosed_count` tracks open binding abstractions

### 3. Proof Verification (`watson/src/semant/proof_kernel.rs`)

**Currently mostly stubbed** - infrastructure exists but main logic not yet implemented.

- `ProofState<'ctx>`: Current proof state with known facts and assumption stack
- `SafeFrag`/`SafeFact`: Validated fragments (no holes, proper closedness, correct category)
- Intended operations: `add_assumption()`, `apply_theorem()`, `complete()`
- `instantiate_frag()`, `fill_holes()`: Template/hole substitution

### 4. Context & Memory Management (`watson/src/context/`)

- **`arena.rs`**: Typed arena allocation pattern
  - `PlainArena`: Simple allocation
  - `InternedArena`: Deduplicates equal values (structural sharing)
  - `NamedArena`: Maps names to allocated items
  - All use `'ctx` lifetime - memory freed when `Ctx` dropped

- **`mod.rs`**: `Ctx<'ctx>` is global compilation context containing:
  - `arenas`: All memory pools
  - `parse_state`: Current grammar state
  - `diags`: Error/warning manager
  - `builtin_cats`, `builtin_rules`: Built-in syntax

**Arena pattern benefit**: Zero-copy references with type safety, no GC needed.

## Key Design Patterns

### Tactic Pattern Syntax

When working with tactic patterns (`TacticPatPartCore`), note the distinction:
- `@fragment(category_name)`: Matches a fragment of specific formal category (e.g., `@fragment(sentence)`)
- `@any_fragment`: Matches any fragment regardless of category
- `@fact`: Matches a fact (hypothetical judgment)
- `@name`: Matches a name identifier
- `@kw"keyword"`: Matches a specific keyword

These are defined in `watson/src/semant/tactic.rs` and parsed in `watson/src/parse/elaborator.rs`.

### Elaboration Action Pattern

Parse phase returns `ElaborateAction` enum rather than directly modifying state:
- `NewSource(SourceId)`: Load new file
- `NewFormalCat`, `NewFormalRule`: Grammar additions
- `NewNotation`: User syntax
- `NewDefinition`: Scope updates
- `NewTheorem`: Collect for verification
- `NewTacticCat`, `NewTacticRule`: Tactic system

Main loop in `parse/mod.rs::parse()` applies actions, keeping parse logic side-effect-free.

### Fragment + Presentation Duality

Every semantic fragment has paired presentation:
```rust
PresFrag<'ctx> = (FragmentId<'ctx>, PresTreeId<'ctx>)
```
- Fragment: semantic meaning (for verification)
- PresTree: display form (for pretty-printing)

Allows multiple notations for same concept without changing semantics.

## File Structure

```
watson/
├── src/
│   ├── main.rs              # Entry point, compilation pipeline
│   ├── parse/               # Earley parsing, grammar, elaboration
│   ├── semant/              # Fragments, theorems, proof kernel
│   ├── context/             # Arena allocators, global state
│   ├── diagnostics.rs       # Error reporting
│   └── report.rs            # Final output formatting
├── Cargo.toml
lib/
└── bool.wats               # Boolean logic axioms and theorems
```

## Common Development Patterns

### Adding New String Constants

String constants are in `watson/src/strings.rs` using the `str_const!` macro:
```rust
str_const! {
    KEYWORD_NAME = "keyword_name";
}
```

### Adding New Grammar Rules

1. Update grammar comment in `watson/src/parse/grammar.rs`
2. Add to `builtin_cats!` macro if new category
3. Add to `builtin_rules!` macro if new rule
4. Add rule construction in `BuiltinRules` initialization
5. Update elaborator in `watson/src/parse/elaborator.rs` to handle new parse tree shapes

### Working with Fragments

Fragments are immutable and arena-allocated:
```rust
let frag = Fragment::new(cat, head, children, flags);
let frag_id = ctx.arenas.fragments.intern(frag);
```

Always use `FragmentId<'ctx>` handles, never store `Fragment` directly.

## Testing

Test with example files in `lib/`:
```bash
./watson/target/debug/watson lib/bool.wats
```

Expected output shows all theorems followed by verification status (currently all shown as checked since proof kernel is stubbed).

## Current Limitations

- Proof kernel (`check_proofs()`) returns empty statuses - verification infrastructure exists but core logic not implemented
- Tactic execution not fully integrated with proof kernel
- Type system is minimal (just category membership)
