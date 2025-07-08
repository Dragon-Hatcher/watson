use crate::span::{SourceCache, Span};
use ustr::Ustr;

pub type WResult<T> = Result<T, ()>;

#[derive(Debug)]
pub struct ReportTracker {
    reports: Vec<Report>,
    has_error: bool,
}

impl ReportTracker {
    pub fn new() -> Self {
        Self {
            reports: Vec::new(),
            has_error: false,
        }
    }

    pub fn add_message(&mut self, report: Report) {
        self.has_error = self.has_error || report.level() == ReportLevel::Error;
        self.reports.push(report);
    }

    pub fn has_error(&self) -> bool {
        self.has_error
    }

    pub fn checkpoint(&self) -> WResult<()> {
        if self.has_error() { Err(()) } else { Ok(()) }
    }

    pub fn reports(&self) -> impl Iterator<Item = &Report> {
        self.reports.iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportLevel {
    Error,
}

impl ReportLevel {
    fn render(&self) -> &'static str {
        match self {
            ReportLevel::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Report {
    level: ReportLevel,
    span: Span,
    msg: Ustr,
}

impl Report {
    pub fn new(level: ReportLevel, span: Span, msg: Ustr) -> Self {
        Self { level, span, msg }
    }

    pub fn level(&self) -> ReportLevel {
        self.level
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn msg(&self) -> Ustr {
        self.msg
    }

    pub fn render(&self, sources: &SourceCache) {
        println!(
            "[{}:{}] {}: {}",
            self.span.file().as_str(),
            self.span.start(),
            self.level.render(),
            self.msg
        )
    }
}

pub mod specifics {
    use ustr::Ustr;

    macro_rules! uformat {
        ($($t:tt)*) => {
            ustr::Ustr::from(&format!($($t)*))
        };
    }

    use super::ReportLevel as RL;
    use crate::{diagnostics::Report, parser::StatementTy, span::Span};

    fn render_statement_ty(ty: StatementTy) -> &'static str {
        match ty {
            StatementTy::Prose => "prose",
            StatementTy::Syntax => "syntax",
            StatementTy::Notation => "notation",
            StatementTy::Definition => "definition",
            StatementTy::Axiom => "axiom",
            StatementTy::Theorem => "theorem",
        }
    }

    pub fn unclosed_statement(span: Span, ty: StatementTy, next_ty: StatementTy) -> Report {
        Report::new(
            RL::Error,
            span,
            uformat!(
                "{} declaration was unclosed at following {} declaration",
                render_statement_ty(ty),
                render_statement_ty(next_ty)
            ),
        )
    }

    pub fn unclosed_statement_at_eof(span: Span, ty: StatementTy) -> Report {
        Report::new(
            RL::Error,
            span,
            uformat!(
                "{} declaration was unclosed at EOF",
                render_statement_ty(ty)
            ),
        )
    }
}
