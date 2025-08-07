use std::path::Path;

use crate::parse::{SourceCache, SourceId, Span, parse_tree::ParseTree};
use annotate_snippets::{Level, Message, Renderer, Snippet};
use itertools::Itertools;
use ustr::Ustr;

pub type WResult<T> = Result<T, ()>;

pub struct DiagManager {
    diags: Vec<Diagnostic>,
}

impl DiagManager {
    pub fn new() -> Self {
        Self { diags: Vec::new() }
    }

    pub fn print_errors(&self, sources: &SourceCache) {
        let renderer = Renderer::styled();
        for diag in &self.diags {
            let msg = diag.to_message(sources);
            println!("{}", renderer.render(msg));
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.diags.is_empty()
    }
}

struct Diagnostic {
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
            title,
            parts: Vec::new(),
        }
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

        // let source = sources.get_text(self.span.source());

        // Level::Error.title(self.title).snippet(
        //     Snippet::source(source)
        //         .origin(self.span.source().path().as_str())
        //         .fold(true)
        //         .annotation(Level::Error.span(self.span.bytes())),
        // )
    }
}

impl DiagManager {
    fn add_diag(&mut self, diag: Diagnostic) {
        self.diags.push(diag);
    }
}

impl DiagManager {
    pub fn err_module_redeclaration<T>(
        &mut self,
        source_id: SourceId,
        second_decl: &ParseTree,
        first_decl: Option<Span>,
    ) -> WResult<T> {
        let mut diag = Diagnostic::new(&format!("redeclaration of module `{}`", source_id.path()))
            .with_error("module declared again here", second_decl.span());

        if let Some(first_decl) = first_decl {
            diag = diag.with_info("module first declared here", first_decl);
        }

        self.add_diag(diag);
        Err(())
    }

    pub fn err_non_existent_file<T>(
        &mut self,
        path: &Path,
        decl: &ParseTree,
    ) -> WResult<T> {
        let diag = Diagnostic::new(&format!(
            "source file `{:?}` does not exist",
            path
        ))
        .with_error("", decl.span());

        self.add_diag(diag);
        Err(())
    }

    pub fn err_elaboration_infinite_recursion<T>(&mut self) -> WResult<T> {
        let diag = Diagnostic::new("err_elaboration_infinite_recursion");

        self.add_diag(diag);
        Err(())
    }

    pub fn err_parse_failure<T>(&mut self) -> WResult<T> {
        let diag = Diagnostic::new("err_parse_failure");

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

    pub fn err_unknown_formal_syntax_cat<T>(&mut self) -> WResult<T> {
        let diag = Diagnostic::new("err_unknown_formal_syntax_cat");

        self.add_diag(diag);
        Err(())
    }
}
