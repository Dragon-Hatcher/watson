use crate::semant::formal_syntax::FormalSyntax;
use crate::semant::fragments::FragCtx;
use crate::semant::render_proof_state::render_proof_state;
use crate::semant::theorem::TheoremStatements;
use crate::util::ansi::{ANSI_BOLD, ANSI_RESET};
use crate::{
    parse::{
        Location, SourceCache, SourceId, Span,
        parse_tree::{AtomPattern, ParseTree},
    },
    semant::{check_proofs::ProofState, theorem::TheoremId},
    util::plural,
};
use annotate_snippets::{Level, Message, Renderer, Snippet};
use itertools::Itertools;
use std::{collections::HashMap, path::Path};
use ustr::Ustr;

pub type WResult<T> = Result<T, ()>;

pub struct DiagManager {
    diags: Vec<Diagnostic>,
    theorem_specific: HashMap<TheoremId, Vec<usize>>,
    has_fatal: bool,
}

impl DiagManager {
    pub fn new() -> Self {
        Self {
            diags: Vec::new(),
            theorem_specific: HashMap::new(),
            has_fatal: false,
        }
    }

    pub fn print_errors(
        &self,
        sources: &SourceCache,
        theorems: Option<&TheoremStatements>,
        frag_ctx: &FragCtx,
        formal_syntax: Option<&FormalSyntax>,
    ) {
        let renderer = Renderer::styled();
        for diag in &self.diags {
            if diag.theorem.is_some() {
                continue;
            }

            let msg = diag.to_message(sources);
            println!("{}", renderer.render(msg));
            println!();
        }

        let mut all_theorems: Vec<TheoremId> = self.theorem_specific.keys().copied().collect();
        all_theorems.sort_by_key(|t| t.name());

        for theorem_id in all_theorems {
            let indices = &self.theorem_specific[&theorem_id];

            println!(
                "{ANSI_BOLD}While checking theorem `{}`:{ANSI_RESET}",
                theorem_id.name()
            );
            println!();
            for &idx in indices {
                let diag = &self.diags[idx];
                let msg = diag.to_message(sources);
                println!("{}", renderer.render(msg));
                if let Some(proof_state) = &diag.proof_state {
                    let state_str = render_proof_state(
                        proof_state,
                        theorems.unwrap().get(theorem_id),
                        frag_ctx,
                        formal_syntax.unwrap(),
                    );
                    println!("{ANSI_BOLD}proof status:{ANSI_RESET}\n{state_str}");
                }
                println!();
            }
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.diags.is_empty()
    }

    pub fn has_fatal_errors(&self) -> bool {
        self.has_fatal
    }
}

struct Diagnostic {
    theorem: Option<TheoremId>,
    proof_state: Option<ProofState>,
    title: &'static str,
    parts: Vec<DiagnosticPart>,
}

enum DiagnosticPart {
    Error(&'static str, Span),
    Info(&'static str, Span),
}

impl DiagnosticPart {
    fn span(&self) -> Option<Span> {
        match self {
            DiagnosticPart::Error(_, span) => Some(*span),
            DiagnosticPart::Info(_, span) => Some(*span),
        }
    }
}

impl Diagnostic {
    fn new(title: &str) -> Self {
        let title = Ustr::from(title).as_str();
        Self {
            theorem: None,
            proof_state: None,
            title,
            parts: Vec::new(),
        }
    }

    fn for_theorem(mut self, theorem: TheoremId) -> Self {
        self.theorem = Some(theorem);
        self
    }

    fn with_proof_state(mut self, proof_state: Option<ProofState>) -> Self {
        self.proof_state = proof_state;
        self
    }

    fn with_error(mut self, msg: &str, span: Span) -> Self {
        let msg = Ustr::from(msg).as_str();
        self.parts.push(DiagnosticPart::Error(msg, span));
        self
    }

    fn with_info(mut self, msg: &str, span: Span) -> Self {
        let msg = Ustr::from(msg).as_str();
        self.parts.push(DiagnosticPart::Info(msg, span));
        self
    }

    fn to_message<'a>(&self, sources: &'a SourceCache) -> Message<'a> {
        let mut msg = Level::Error.title(self.title);

        for (source, parts) in &self.parts.iter().chunk_by(|p| p.span().map(|s| s.source())) {
            let path = source.map(|s| s.path()).unwrap();
            let source = source.map(|s| sources.get_text(s)).unwrap();
            let mut snippet = Snippet::source(source).origin(path.as_str()).fold(true);

            for part in parts {
                match part {
                    DiagnosticPart::Error(m, span) => {
                        snippet = snippet.annotation(Level::Error.span(span.bytes()).label(m))
                    }
                    DiagnosticPart::Info(m, span) => {
                        snippet = snippet.annotation(Level::Info.span(span.bytes()).label(m))
                    }
                }
            }

            msg = msg.snippet(snippet);
        }

        msg
    }
}

impl DiagManager {
    fn add_diag(&mut self, diag: Diagnostic) {
        if diag.theorem.is_none() {
            self.has_fatal = true;
        }

        if let Some(theorem_id) = diag.theorem {
            let idx = self.diags.len();
            self.theorem_specific
                .entry(theorem_id)
                .or_default()
                .push(idx);
        }

        self.diags.push(diag);
    }
}

impl DiagManager {
    pub fn err_module_redeclaration<T>(
        &mut self,
        source_id: SourceId,
        second_decl: Span,
        first_decl: Option<Span>,
    ) -> WResult<T> {
        let mut diag = Diagnostic::new(&format!("redeclaration of module `{}`", source_id.path()))
            .with_error("module declared again here", second_decl);

        if let Some(first_decl) = first_decl {
            diag = diag.with_info("module first declared here", first_decl);
        }

        self.add_diag(diag);
        Err(())
    }

    pub fn err_non_existent_file<T>(&mut self, path: &Path, decl: &ParseTree) -> WResult<T> {
        let diag = Diagnostic::new(&format!("source file `{}` does not exist", path.display()))
            .with_error("", decl.span());

        self.add_diag(diag);
        Err(())
    }

    pub fn err_elaboration_infinite_recursion<T>(&mut self, span: Span) -> WResult<T> {
        let diag = Diagnostic::new("infinite recursion while expanding").with_error("", span);

        self.add_diag(diag);
        Err(())
    }

    pub fn err_parse_failure<T>(
        &mut self,
        location: Location,
        possible_atoms: &[AtomPattern],
    ) -> WResult<T> {
        fn format_atom(atom: &AtomPattern) -> String {
            match atom {
                AtomPattern::Lit(lit) => format!("\"{lit}\""),
                AtomPattern::Kw(kw) => format!("\"{kw}\""),
                AtomPattern::Name => "a name".to_string(),
                AtomPattern::Str => "a string literal".to_string(),
            }
        }

        let expected = if possible_atoms.is_empty() {
            "what".to_string()
        } else if let [atom] = possible_atoms {
            format!("expected {}", format_atom(atom))
        } else if let [atom1, atom2] = possible_atoms {
            format!("expected {} or {}", format_atom(atom1), format_atom(atom2))
        } else {
            format!(
                "expected {}, or {}",
                possible_atoms[..possible_atoms.len() - 1]
                    .iter()
                    .map(format_atom)
                    .join(", "),
                format_atom(possible_atoms.last().unwrap())
            )
        };

        let diag = Diagnostic::new("error while parsing command")
            .with_error(&expected, Span::new(location, location));

        self.add_diag(diag);
        Err(())
    }

    pub fn err_duplicate_formal_syntax_cat<T>(&mut self) -> WResult<T> {
        let diag = Diagnostic::new("err_duplicate_formal_syntax_cat");

        self.add_diag(diag);
        Err(())
    }

    pub fn err_duplicate_formal_syntax_rule<T>(&mut self) -> WResult<T> {
        let diag = Diagnostic::new("err_duplicate_formal_syntax_rule");

        self.add_diag(diag);
        Err(())
    }

    pub fn err_duplicate_theorem<T>(&mut self, name: Ustr, span: Span) -> WResult<T> {
        let diag = Diagnostic::new("err_duplicate_theorem")
            .with_error(&format!("theorem `{name}` declared again here"), span);

        self.add_diag(diag);
        Err(())
    }

    pub fn err_unknown_formal_syntax_cat<T>(&mut self) -> WResult<T> {
        let diag = Diagnostic::new("err_unknown_formal_syntax_cat");

        self.add_diag(diag);
        Err(())
    }

    pub fn err_undefined_macro_binding<T>(&mut self, name: Ustr, span: Span) -> WResult<T> {
        let diag =
            Diagnostic::new(&format!("undefined macro binding `${name}`")).with_error("", span);

        self.add_diag(diag);
        Err(())
    }

    pub fn err_non_existent_syntax_category<T>(&mut self, name: Ustr, span: Span) -> WResult<T> {
        let diag =
            Diagnostic::new(&format!("unknown syntax category `{name}`")).with_error("", span);

        self.add_diag(diag);
        Err(())
    }
}

// Below are errors relating specifically to proofs.

impl DiagManager {
    pub fn err_unknown_theorem(&mut self, theorem: TheoremId, name: Ustr, span: Span) {
        let diag = Diagnostic::new(&format!("unknown theorem `{name}`"))
            .with_error("", span)
            .for_theorem(theorem);

        self.add_diag(diag);
    }

    pub fn err_missing_tactic_templates(
        &mut self,
        theorem: TheoremId,
        last_template: Span,
        missing: usize,
    ) {
        let diag = Diagnostic::new(&format!(
            "missing {missing} tactic template{}",
            plural(missing)
        ))
        .with_error(
            &format!("expected {missing} more tactic template{}", plural(missing)),
            last_template,
        )
        .for_theorem(theorem);

        self.add_diag(diag);
    }

    pub fn err_extra_tactic_templates(
        &mut self,
        theorem: TheoremId,
        extra_template: Span,
        extra: usize,
    ) {
        let diag = Diagnostic::new(&format!("extra tactic template{}", plural(extra)))
            .with_error(
                &format!("found {extra} extra tactic template{}", plural(extra)),
                extra_template,
            )
            .for_theorem(theorem);

        self.add_diag(diag);
    }

    pub fn err_unknown_name(
        &mut self,
        theorem: TheoremId,
        proof_state: Option<ProofState>,
        name: Ustr,
        span: Span,
    ) {
        let diag = Diagnostic::new(&format!("unknown name `{name}`"))
            .with_error("", span)
            .for_theorem(theorem)
            .with_proof_state(proof_state);

        self.add_diag(diag);
    }

    pub fn err_incomplete_proof(&mut self, theorem: TheoremId, at: Span, proof_state: ProofState) {
        let diag = Diagnostic::new("unsolved goal")
            .with_error("", at)
            .for_theorem(theorem)
            .with_proof_state(Some(proof_state));

        self.add_diag(diag);
    }
}
