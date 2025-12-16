use crate::context::Ctx;
use crate::parse::parse_state::ParseAtomPattern;
use crate::parse::source_cache::SourceDecl;
use crate::parse::{Location, SourceCache, SourceId, Span};
use crate::semant::parse_fragment;
use annotate_snippets::{Level, Message, Renderer, Snippet};
use itertools::Itertools;
use std::marker::PhantomData;
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
    title: &'static str,
    parts: Vec<DiagnosticPart>,
    p: PhantomData<&'ctx ()>,
}

#[derive(Debug, Clone)]
pub enum DiagnosticPart {
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
    pub fn new(title: &str) -> Self {
        let title = Ustr::from(title).as_str();
        Self {
            title,
            parts: Vec::new(),
            p: PhantomData,
        }
    }

    pub fn with_error(mut self, msg: &str, span: Span) -> Self {
        let msg = Ustr::from(msg).as_str();
        self.parts.push(DiagnosticPart::Error(msg, span));
        self
    }

    pub fn with_info(mut self, msg: &str, span: Span) -> Self {
        let msg = Ustr::from(msg).as_str();
        self.parts.push(DiagnosticPart::Info(msg, span));
        self
    }

    pub fn to_message<'a>(&self, sources: &'a SourceCache) -> Message<'a> {
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

        msg
    }
}

impl<'ctx> DiagManager<'ctx> {
    pub fn add_diag(&mut self, diag: Diagnostic<'ctx>) {
        self.diags.push(diag);
    }

    pub fn add_diags(&mut self, diags: Vec<Diagnostic<'ctx>>) {
        self.diags.extend(diags);
    }
}

impl<'ctx> Diagnostic<'ctx> {
    pub fn err_module_redeclaration<T>(
        source_id: SourceId,
        decl: Span,
        previous_decl: SourceDecl,
    ) -> WResult<'ctx, T> {
        let mut diag = Diagnostic::new(&format!("redeclaration of module `{}`", source_id.name()))
            .with_error("", decl);

        if let SourceDecl::Module(prev_span) = previous_decl {
            diag = diag.with_info("previous declaration", prev_span);
        }

        Err(vec![diag])
    }

    pub fn err_non_existent_file<T>(path: &Path, decl: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(&format!("source file `{}` does not exist", path.display()))
            .with_error("", decl);

        Err(vec![diag])
    }

    pub fn _err_elaboration_infinite_recursion<T>(span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new("infinite recursion while expanding").with_error("", span);

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

        let span = Span::new(location, location);
        let diag = Diagnostic::new("error while parsing command").with_error(&expected, span);

        Err(vec![diag])
    }

    pub fn err_duplicate_formal_syntax_cat<T>() -> WResult<'ctx, T> {
        let diag = Diagnostic::new("err_duplicate_formal_syntax_cat");

        Err(vec![diag])
    }

    pub fn err_duplicate_formal_syntax_rule<T>() -> WResult<'ctx, T> {
        let diag = Diagnostic::new("err_duplicate_formal_syntax_rule");

        Err(vec![diag])
    }

    pub fn err_unknown_formal_syntax_cat<T>(name: Ustr, span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(&format!("unknown formal syntax category `{name}`"))
            .with_error("", span);

        Err(vec![diag])
    }

    pub fn err_duplicate_tactic_cat<T>() -> WResult<'ctx, T> {
        let diag = Diagnostic::new("err_duplicate_tactic_cat");

        Err(vec![diag])
    }

    pub fn err_reserved_tactic_cat_name<T>(name: Ustr, span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(&format!(
            "tactic category name `{name}` is reserved (it conflicts with a built-in Luau type)"
        ))
        .with_error("reserved name used here", span);

        Err(vec![diag])
    }

    pub fn err_duplicate_tactic_rule<T>() -> WResult<'ctx, T> {
        let diag = Diagnostic::new("err_duplicate_tactic_rule");

        Err(vec![diag])
    }

    pub fn err_unknown_tactic_cat<T>(name: Ustr, span: Span) -> WResult<'ctx, T> {
        let diag =
            Diagnostic::new(&format!("unknown tactic category `{name}`")).with_error("", span);

        Err(vec![diag])
    }

    pub fn err_reserved_tactic_label<T>(label: Ustr, span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new(&format!("tactic label `{label}` is reserved"))
            .with_error("reserved label used here", span);

        Err(vec![diag])
    }

    pub fn err_ambiguous_parse<T>(span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new("ambiguous parse").with_error("", span);

        Err(vec![diag])
    }

    pub fn err_frag_parse_failure(span: Span, err: parse_fragment::ParseResultErr) -> Self {
        let diag = Diagnostic::new(&format!("failed to parse fragment because {err:?}"))
            .with_error("", span);

        diag
    }
}
