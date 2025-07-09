use crate::{
    diagnostics::{AnnotationTy, Report},
    span::SourceCache,
};
use annotate_snippets::{self as snip, Snippet};

fn snip_an_level(ty: AnnotationTy) -> snip::Level {
    match ty {
        AnnotationTy::Note => snip::Level::Note,
        AnnotationTy::Info => snip::Level::Info,
    }
}

pub fn render(report: &Report, sources: &SourceCache) {
    let snip_level = match report.level {
        super::ReportLevel::Error => snip::Level::Error,
    };

    let mut snip_msg = snip_level.title(&report.msg);

    for annotations in report
        .annotations
        .chunk_by(|a, b| a.span.file() == b.span.file())
    {
        let filename = annotations[0].span.file().as_str();
        let source = sources.get_text(annotations[0].span.file());

        let snippet = Snippet::source(source)
            .origin(filename)
            .fold(true)
            .annotations(
                annotations
                    .iter()
                    .map(|a| snip_an_level(a.ty).span(a.span.range()).label(&a.msg)),
            );
        snip_msg = snip_msg.snippet(snippet);
    }

    let renderer = snip::Renderer::styled();
    println!("{}", renderer.render(snip_msg));
}
