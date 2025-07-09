use crate::span::{SourceCache, Span};
use ustr::Ustr;

mod render;
pub mod specifics;

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
        self.has_error = self.has_error || report.level == ReportLevel::Error;
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

#[derive(Debug, Clone)]
pub struct Report {
    level: ReportLevel,
    msg: Ustr,
    annotations: Vec<Annotation>,
}

impl Report {
    pub fn new(level: ReportLevel, msg: Ustr) -> Self {
        Self {
            level,
            msg,
            annotations: Vec::new(),
        }
    }

    pub fn with_note(mut self, span: Span, msg: Ustr) -> Self {
        self.annotations
            .push(Annotation::new(AnnotationTy::Note, span, msg));
        self
    }

    pub fn with_info(mut self, span: Span, msg: Ustr) -> Self {
        self.annotations
            .push(Annotation::new(AnnotationTy::Info, span, msg));
        self
    }

    pub fn render(&self, sources: &SourceCache) {
        render::render(self, sources)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AnnotationTy {
    Info,
    Note,
}

#[derive(Debug, Clone, Copy)]
pub struct Annotation {
    ty: AnnotationTy,
    span: Span,
    msg: Ustr,
}

impl Annotation {
    pub fn new(ty: AnnotationTy, span: Span, msg: Ustr) -> Self {
        Self { ty, span, msg }
    }
}
