use crate::context::Ctx;
use crate::parse::elaborator::BindingResolution;
use crate::parse::parse_state::ParseAtomPattern;
use crate::parse::source_cache::SourceDecl;
use crate::parse::{Location, SourceCache, SourceId, Span};
use crate::semant::notation::NotationPatternSource;
use crate::semant::parse_fragment;
use crate::semant::tactic::tactic_info::{TacticInfo, TacticInfoStep};
use crate::semant::theorems::TheoremId;
use crate::util::ansi::{ANSI_BOLD, ANSI_GRAY, ANSI_RESET, ANSI_YELLOW};
use annotate_snippets::{Level, Message, Renderer, Snippet};
use itertools::Itertools;
use std::path::Path;
use std::vec;
use ustr::Ustr;

pub type WResult<'ctx, T> = Result<T, Vec<Diagnostic<'ctx>>>;

pub struct DiagManager<'ctx> {
    diags: Vec<Diagnostic<'ctx>>,
}

impl<'ctx> DiagManager<'ctx> {
    pub fn new() -> Self {
        Self { diags: Vec::new() }
    }

    pub fn add_diag(&mut self, diag: Diagnostic<'ctx>) {
        self.diags.push(diag);
    }

    pub fn add_diags(&mut self, diags: Vec<Diagnostic<'ctx>>) {
        self.diags.extend(diags);
    }

    pub fn clear_errors(&mut self) {
        self.diags.clear();
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

#[derive(Debug, Clone)]
pub struct Diagnostic<'ctx> {
    main: DiagnosticPart,
    parts: Vec<DiagnosticPart>,
    proof: Option<DiagnosticInProof<'ctx>>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticInProof<'ctx> {
    thm: TheoremId<'ctx>,
    tactic_info: TacticInfo<'ctx>,
}

#[derive(Debug, Clone)]
pub struct DiagnosticPart {
    level: DiagnosticLevel,
    title: &'static str,
    spans: Vec<DiagnosticSpan>,
}

impl DiagnosticPart {
    pub fn new(level: DiagnosticLevel, title: &'static str, spans: Vec<DiagnosticSpan>) -> Self {
        Self {
            level,
            title,
            spans,
        }
    }

    pub fn to_message<'a>(&self, sources: &'a SourceCache) -> Message<'a> {
        let level = match self.level {
            DiagnosticLevel::Error => Level::Error,
            DiagnosticLevel::Info => Level::Info,
        };
        level
            .title(self.title)
            .snippets(self.spans.iter().map(|s| s.to_snippet(sources)))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DiagnosticSpan {
    level: DiagnosticLevel,
    span: Span,
    msg: &'static str,
}

impl DiagnosticSpan {
    pub fn new_error(msg: &'static str, span: Span) -> Self {
        Self {
            level: DiagnosticLevel::Error,
            span,
            msg,
        }
    }

    pub fn new_info(msg: &'static str, span: Span) -> Self {
        Self {
            level: DiagnosticLevel::Info,
            span,
            msg,
        }
    }

    pub fn to_snippet<'a>(&self, sources: &'a SourceCache) -> Snippet<'a> {
        let anno = self
            .level
            .to_level()
            .span(self.span.bytes())
            .label(self.msg);
        Snippet::source(sources.get_text(self.span.source()).as_str())
            .annotation(anno)
            .fold(true)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DiagnosticLevel {
    Error,
    Info,
}

impl DiagnosticLevel {
    pub fn to_level(&self) -> Level {
        match self {
            DiagnosticLevel::Error => Level::Error,
            DiagnosticLevel::Info => Level::Info,
        }
    }
}

impl<'ctx> Diagnostic<'ctx> {
    pub fn new(title: &str, spans: Vec<DiagnosticSpan>) -> Self {
        let title = Ustr::from(title).as_str();
        Self {
            main: DiagnosticPart::new(DiagnosticLevel::Error, title, spans),
            parts: Vec::new(),
            proof: None,
        }
    }

    pub fn with_error(mut self, msg: &str, spans: Vec<DiagnosticSpan>) -> Self {
        let msg = Ustr::from(msg).as_str();
        self.parts
            .push(DiagnosticPart::new(DiagnosticLevel::Error, msg, spans));
        self
    }

    pub fn with_info(mut self, msg: &str, spans: Vec<DiagnosticSpan>) -> Self {
        let msg = Ustr::from(msg).as_str();
        self.parts
            .push(DiagnosticPart::new(DiagnosticLevel::Info, msg, spans));
        self
    }

    pub fn in_proof(mut self, thm: TheoremId<'ctx>, tactic_info: TacticInfo<'ctx>) -> Self {
        self.proof = Some(DiagnosticInProof { thm, tactic_info });
        self
    }

    pub fn to_message<'a>(&self, sources: &'a SourceCache) -> Message<'a> {
        let mut msg = self.main.to_message(sources);

        for part in &self.parts {
            msg = msg.footer(part.to_message(sources))
        }

        if let Some(in_proof) = &self.proof {
            let title = format!("While checking theorem `{}`", in_proof.thm.name());
            let title = Ustr::from(&title);
            msg = msg.footer(Level::Help.title(title.as_str()));

            let title = render_tactic_info(&in_proof.tactic_info);
            let title = Ustr::from(&title);
            msg = msg.footer(Level::Help.title(title.as_str()));
        }

        msg
    }
}

fn render_tactic_info<'ctx>(tactic: &TacticInfo<'ctx>) -> String {
    let mut res = String::new();

    res += "Proof state:\n";

    for step in tactic.steps() {
        match step {
            TacticInfoStep::Hypothesis(f) => {
                res += ANSI_GRAY;
                res += "> ";
                res += ANSI_RESET;
                res += &f.print();
            }
            TacticInfoStep::Assume(f) => {
                res += ANSI_GRAY;
                res += "? ";
                res += ANSI_RESET;
                res += &f.print()
            }
            TacticInfoStep::Deduce(f) => {
                res += "  ";
                res += &f.print()
            }
            TacticInfoStep::Let(binding, replacement) => {
                res += "  ";
                res += &binding.print();
                res += ANSI_GRAY;
                res += " : ";
                res += &binding.pattern().cat().name();
                res += ANSI_RESET;
                if let Some(replacement) = replacement {
                    res += ANSI_GRAY;
                    res += " := ";
                    res += ANSI_RESET;
                    res += &replacement.print();
                }
            }
        }

        res += "\n";
    }

    res += &format!("{ANSI_YELLOW}{ANSI_BOLD}⊢{ANSI_RESET} ");
    res += &tactic.goal().print();

    res
}

impl<'ctx> Diagnostic<'ctx> {
    pub fn err_module_redeclaration<T>(
        source_id: SourceId,
        decl: Span,
        previous_decl: SourceDecl,
    ) -> WResult<'ctx, T> {
        let mut diag = Diagnostic::new(
            &format!("redeclaration of module `{}`", source_id.name()),
            vec![DiagnosticSpan::new_error("", decl)],
        );

        if let SourceDecl::Module(prev_span) = previous_decl {
            diag = diag.with_info(
                "module previously declared here",
                vec![DiagnosticSpan::new_error("", prev_span)],
            );
        }

        Err(vec![diag])
    }

    pub fn err_non_existent_file<T>(path: &Path, decl: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(
            &format!("source file `{}` does not exist", path.display()),
            vec![DiagnosticSpan::new_error("", decl)],
        );

        Err(vec![diag])
    }

    pub fn err_parse_failure<T>(
        location: Location,
        possible_atoms: &[ParseAtomPattern],
    ) -> WResult<'ctx, T> {
        fn format_atom(atom: &ParseAtomPattern) -> String {
            match atom {
                ParseAtomPattern::Lit(lit) => format!("\"{lit}\""),
                ParseAtomPattern::Kw(kw) => format!("\"{kw}\""),
                ParseAtomPattern::Name => "a name".to_string(),
                ParseAtomPattern::Str => "a string literal".to_string(),
                ParseAtomPattern::Num => "a number".to_string(),
            }
        }

        let expected = if possible_atoms.is_empty() {
            "impossible".to_string()
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
        let expected = Ustr::from(&expected);

        let span = Span::new(location, location);
        let diag = Diagnostic::new(
            "error while parsing command",
            vec![DiagnosticSpan::new_error(expected.as_str(), span)],
        );

        Err(vec![diag])
    }

    pub fn err_duplicate_formal_syntax_cat<T>() -> WResult<'ctx, T> {
        let diag = Diagnostic::new("err_duplicate_formal_syntax_cat", vec![]);

        Err(vec![diag])
    }

    pub fn err_duplicate_formal_syntax_rule<T>() -> WResult<'ctx, T> {
        let diag = Diagnostic::new("err_duplicate_formal_syntax_rule", vec![]);

        Err(vec![diag])
    }

    pub fn err_unknown_formal_syntax_cat<T>(name: Ustr, span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(
            &format!("unknown formal syntax category `{name}`"),
            vec![DiagnosticSpan::new_error("", span)],
        );

        Err(vec![diag])
    }

    pub fn err_duplicate_pattern_binding<T>(name: Ustr, span: Span) -> WResult<'ctx, T> {
        let diag: Diagnostic<'_> = Diagnostic::new(
            &format!("duplicate pattern binding `{name}`"),
            vec![DiagnosticSpan::new_error("", span)],
        );

        Err(vec![diag])
    }

    pub fn err_unknown_pattern_binding<T>(name: Ustr, span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(
            &format!("unknown pattern binding `{name}`"),
            vec![DiagnosticSpan::new_error("", span)],
        );

        Err(vec![diag])
    }

    pub fn err_duplicate_tactic_cat<T>() -> WResult<'ctx, T> {
        let diag = Diagnostic::new("err_duplicate_tactic_cat", vec![]);

        Err(vec![diag])
    }

    pub fn err_reserved_tactic_cat_name<T>(name: Ustr, span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(
            &format!(
                "tactic category name `{name}` is reserved (it conflicts with a built-in Luau type)"
            ),
            vec![DiagnosticSpan::new_error("", span)],
        );

        Err(vec![diag])
    }

    pub fn err_duplicate_tactic_rule<T>() -> WResult<'ctx, T> {
        let diag = Diagnostic::new("err_duplicate_tactic_rule", vec![]);

        Err(vec![diag])
    }

    pub fn err_unknown_tactic_cat<T>(name: Ustr, span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(
            &format!("unknown tactic category `{name}`"),
            vec![DiagnosticSpan::new_error("", span)],
        );

        Err(vec![diag])
    }

    pub fn err_reserved_tactic_label<T>(label: Ustr, span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(
            &format!("tactic label `{label}` is reserved"),
            vec![DiagnosticSpan::new_error("", span)],
        );

        Err(vec![diag])
    }

    pub fn err_ambiguous_parse<T>(span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new("ambiguous parse", vec![DiagnosticSpan::new_error("", span)]);

        Err(vec![diag])
    }

    pub fn err_no_matching_notation_binding<T>(
        cat_name: ustr::Ustr,
        span: Span,
    ) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(
            &format!("no matching notation binding for category `{cat_name}`"),
            vec![DiagnosticSpan::new_error("", span)],
        );

        Err(vec![diag])
    }

    pub fn err_ambiguous_notation_binding<T>(
        cat_name: ustr::Ustr,
        matching_notations: &[BindingResolution<'ctx>],
        span: Span,
    ) -> WResult<'ctx, T> {
        let count = matching_notations.len();
        let mut diag = Diagnostic::new(
            &format!(
                "ambiguous notation binding: {count} different notations match category `{cat_name}`"
            ),
            vec![DiagnosticSpan::new_error("", span)],
        );

        for resolution in matching_notations {
            let pattern = resolution.binding.pattern();
            match pattern.source() {
                NotationPatternSource::UserDeclared(decl_span) => {
                    diag = diag.with_info(
                        &format!("binding `{}` matches", pattern.name()),
                        vec![DiagnosticSpan::new_info("declared here", decl_span)],
                    );
                }
                NotationPatternSource::Builtin => {
                    diag = diag.with_info(
                        &format!("builtin binding `{}` matches", pattern.name()),
                        vec![],
                    );
                }
            }
        }

        Err(vec![diag])
    }

    pub fn err_frag_parse_failure(span: Span, err: parse_fragment::ParseResultErr) -> Self {
        Diagnostic::new(
            &format!("failed to parse fragment because {err:?}"),
            vec![DiagnosticSpan::new_error("", span)],
        )
    }

    pub fn _err_TODO_real_error_later<T>(span: Span, msg: &str) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(msg, vec![DiagnosticSpan::new_error("", span)]);

        Err(vec![diag])
    }
}
