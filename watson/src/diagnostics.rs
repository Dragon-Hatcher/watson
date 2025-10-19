use crate::context::Ctx;
use crate::parse::parse_state::ParseAtomPattern;
use crate::parse::source_cache::SourceDecl;
use crate::parse::{Location, SourceCache, SourceId, Span};
use crate::semant::check_proof::{ProofStateKey, ReasoningStep};
use crate::semant::presentation::PresentationTreeId;
use crate::semant::theorems::TheoremId;
use crate::util::plural;
use annotate_snippets::{Level, Message, Renderer, Snippet};
use itertools::Itertools;
use std::path::Path;
use ustr::Ustr;

pub type WResult<T> = Result<T, ()>;

pub struct DiagManager<'ctx> {
    diags: Vec<Diagnostic<'ctx>>,
}

impl<'ctx> DiagManager<'ctx> {
    pub fn new() -> Self {
        Self { diags: Vec::new() }
    }

    pub fn print_errors(&self, ctx: &Ctx) {
        let renderer = Renderer::styled();
        for diag in &self.diags {
            let msg = diag.to_message(&ctx.sources);
            println!();
            println!("{}", renderer.render(msg));
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.diags.is_empty()
    }
}

struct Diagnostic<'ctx> {
    title: &'static str,
    parts: Vec<DiagnosticPart>,
    in_proof: Option<InProof<'ctx>>,
}

pub struct InProof<'ctx> {
    theorem: TheoremId<'ctx>,
    proof_state: ProofStateKey<'ctx>,
}

impl<'ctx> From<(TheoremId<'ctx>, ProofStateKey<'ctx>)> for InProof<'ctx> {
    fn from(value: (TheoremId<'ctx>, ProofStateKey<'ctx>)) -> Self {
        Self {
            theorem: value.0,
            proof_state: value.1,
        }
    }
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

impl<'ctx> Diagnostic<'ctx> {
    fn new(title: &str) -> Self {
        let title = Ustr::from(title).as_str();
        Self {
            title,
            parts: Vec::new(),
            in_proof: None,
        }
    }

    fn in_proof(mut self, in_proof: InProof<'ctx>) -> Self {
        self.in_proof = Some(in_proof);
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
            let path = source.map(|s| s.name()).unwrap();
            let source = source.map(|s| sources.get_text(s).as_str()).unwrap();
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

        if let Some(in_proof) = &self.in_proof {
            let title = format!("While checking theorem `{}`", in_proof.theorem.name());
            let title = Ustr::from(&title);
            msg = msg.footer(Level::Help.title(title.as_str()));

            let title = render_proof_state(in_proof.proof_state);
            let title = Ustr::from(&title);
            msg = msg.footer(Level::Help.title(title.as_str()));
        }

        msg
    }
}

fn render_proof_state<'ctx>(state: ProofStateKey<'ctx>) -> String {
    let mut res = String::new();

    res += "Proof state:\n";

    for &step in state.reasoning_chain() {
        res += "  ";

        match step {
            ReasoningStep::Hypothesis((_, pres)) => res += &pres.render_str(),
            ReasoningStep::Deduce((_, pres)) => res += &pres.render_str(),
            ReasoningStep::Assume((_, pres)) => res += &pres.render_str(),
            ReasoningStep::_Shorthand(name, (_, pres)) => {
                res += &name;
                res += " := ";
                res += &pres.render_str();
            }
        }

        res += "\n";
    }

    res += "⊢ ";
    res += &state.goal().1.render_str();

    res
}

impl<'ctx> DiagManager<'ctx> {
    fn add_diag(&mut self, diag: Diagnostic<'ctx>) {
        self.diags.push(diag);
    }
}

impl<'ctx> DiagManager<'ctx> {
    pub fn err_module_redeclaration<T>(
        &mut self,
        source_id: SourceId,
        decl: Span,
        previous_decl: SourceDecl,
    ) -> WResult<T> {
        let mut diag = Diagnostic::new(&format!("redeclaration of module `{}`", source_id.name()))
            .with_error("", decl);

        if let SourceDecl::Module(prev_span) = previous_decl {
            diag = diag.with_info("previous declaration", prev_span);
        }

        self.add_diag(diag);
        Err(())
    }

    pub fn err_non_existent_file<T>(&mut self, path: &Path, decl: Span) -> WResult<T> {
        let diag = Diagnostic::new(&format!("source file `{}` does not exist", path.display()))
            .with_error("", decl);

        self.add_diag(diag);
        Err(())
    }

    pub fn _err_elaboration_infinite_recursion<T>(&mut self, span: Span) -> WResult<T> {
        let diag = Diagnostic::new("infinite recursion while expanding").with_error("", span);

        self.add_diag(diag);
        Err(())
    }

    pub fn err_parse_failure<T>(
        &mut self,
        location: Location,
        possible_atoms: &[ParseAtomPattern],
    ) -> WResult<T> {
        fn format_atom(atom: &ParseAtomPattern) -> String {
            match atom {
                ParseAtomPattern::Lit(lit) => format!("\"{lit}\""),
                ParseAtomPattern::Kw(kw) => format!("\"{kw}\""),
                ParseAtomPattern::Name => "a name".to_string(),
                ParseAtomPattern::Str => "a string literal".to_string(),
                ParseAtomPattern::MacroBinding => "a macro binding".to_string(),
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

    pub fn err_unknown_formal_syntax_cat<T>(&mut self, name: Ustr, span: Span) -> WResult<T> {
        let diag = Diagnostic::new(&format!("unknown formal syntax category `{name}`"))
            .with_error("", span);

        self.add_diag(diag);
        Err(())
    }

    pub fn err_duplicate_macro_binding<T>(&mut self) -> WResult<T> {
        let diag = Diagnostic::new("duplicate macro binding");

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

    pub fn err_ambiguous_parse<T>(&mut self, span: Span) -> WResult<T> {
        let diag = Diagnostic::new("ambiguous parse").with_error("", span);

        self.add_diag(diag);
        Err(())
    }
}

// Below are errors relating specifically to proofs.

impl<'ctx> DiagManager<'ctx> {
    pub fn err_non_existent_theorem(&mut self, name: Ustr, span: Span) {
        let diag = Diagnostic::new(&format!("unknown theorem `{name}`")).with_error("", span);

        self.add_diag(diag);
    }

    pub fn err_missing_tactic_templates(&mut self, last_template: Span, missing: usize) {
        let diag = Diagnostic::new(&format!(
            "missing {missing} tactic template{}",
            plural(missing)
        ))
        .with_error(
            &format!("expected {missing} more tactic template{}", plural(missing)),
            last_template,
        );

        self.add_diag(diag);
    }

    pub fn err_extra_tactic_templates(&mut self, extra_template: Span, extra: usize) {
        let diag = Diagnostic::new(&format!("extra tactic template{}", plural(extra))).with_error(
            &format!("found {extra} extra tactic template{}", plural(extra)),
            extra_template,
        );

        self.add_diag(diag);
    }

    pub fn err_missing_goal(&mut self, in_proof: impl Into<InProof<'ctx>>, at: Span) {
        let in_proof = in_proof.into();
        let goal_txt = in_proof.proof_state.goal().1.render_str();

        let diag = Diagnostic::new(&format!("missing goal `{goal_txt}`"))
            .with_error("goal unproved at end of section", at)
            .in_proof(in_proof);

        self.add_diag(diag);
    }

    pub fn err_goal_conclusion_mismatch(
        &mut self,
        in_proof: impl Into<InProof<'ctx>>,
        at: Span,
        conclusion: PresentationTreeId<'ctx>,
    ) {
        let in_proof = in_proof.into();
        let goal_txt = in_proof.proof_state.goal().1.render_str();
        let conclusion_txt = conclusion.render_str();

        let diag = Diagnostic::new(&format!(
            "mismatch between goal `{goal_txt}` and conclusion `{conclusion_txt}`"
        ))
        .with_error("", at)
        .in_proof(in_proof);

        self.add_diag(diag);
    }
}

// impl DiagManager {
//     pub fn err_unknown_theorem(&mut self, theorem: TheoremId, name: Ustr, span: Span) {
//         let diag = Diagnostic::new(&format!("unknown theorem `{name}`"))
//             .with_error("", span)
//             .for_theorem(theorem);

//         self.add_diag(diag);
//     }

//     pub fn err_missing_tactic_templates(
//         &mut self,
//         theorem: TheoremId,
//         last_template: Span,
//         missing: usize,
//     ) {
//         let diag = Diagnostic::new(&format!(
//             "missing {missing} tactic template{}",
//             plural(missing)
//         ))
//         .with_error(
//             &format!("expected {missing} more tactic template{}", plural(missing)),
//             last_template,
//         )
//         .for_theorem(theorem);

//         self.add_diag(diag);
//     }

//     pub fn err_extra_tactic_templates(
//         &mut self,
//         theorem: TheoremId,
//         extra_template: Span,
//         extra: usize,
//     ) {
//         let diag = Diagnostic::new(&format!("extra tactic template{}", plural(extra)))
//             .with_error(
//                 &format!("found {extra} extra tactic template{}", plural(extra)),
//                 extra_template,
//             )
//             .for_theorem(theorem);

//         self.add_diag(diag);
//     }

//     pub fn err_unknown_name(
//         &mut self,
//         theorem: TheoremId,
//         proof_state: Option<ProofState>,
//         name: Ustr,
//         span: Span,
//     ) {
//         let diag = Diagnostic::new(&format!("unknown name `{name}`"))
//             .with_error("", span)
//             .for_theorem(theorem)
//             .with_proof_state(proof_state);

//         self.add_diag(diag);
//     }

//     pub fn err_incomplete_proof(&mut self, theorem: TheoremId, at: Span, proof_state: ProofState) {
//         let diag = Diagnostic::new("unsolved goal")
//             .with_error("", at)
//             .for_theorem(theorem)
//             .with_proof_state(Some(proof_state));

//         self.add_diag(diag);
//     }
// }
