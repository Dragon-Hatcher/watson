use crate::{
    context::Ctx,
    diagnostics::{Diagnostic, WResult},
    parse::{Location, ParseEntry, ParseReport, Span, location::SourceOffset},
    report::ProofReport,
    util::ansi::{ANSI_BOLD, ANSI_GREEN, ANSI_RESET},
};
use aho_corasick::AhoCorasick;
use line_span::LineSpanExt;
use std::fs;

pub fn build_book<'ctx>(
    ctx: &mut Ctx<'ctx>,
    parse_report: ParseReport<'ctx>,
    _proof_report: ProofReport<'ctx>,
) {
    ctx.diags.clear_errors();

    let book_dir = ctx.config.build_dir().join("book");
    fs::create_dir_all(&book_dir).unwrap();

    let mut doc = DocState::new();
    doc.process_entries(&parse_report.entries, ctx);

    if ctx.diags.has_errors() {
        ctx.diags.print_errors(ctx);
        std::process::exit(1);
    }

    let css_path = book_dir.join("styles.css");
    fs::write(css_path, include_str!("templates/styles.css")).expect("TODO");

    for (i, chapter_contents) in doc.chapter_contents.iter().enumerate() {
        let chapter_num = i + 1;
        let path = book_dir.join(format!("chapter_{chapter_num}.html"));
        let content = replace_patterns(
            include_str!("templates/layout.html"),
            &["{{SIDEBAR}}", "{{CHAPTER_CONTENT}}"],
            &["", chapter_contents],
        );
        fs::write(path, content).expect("TODO");
    }

    let full_path = book_dir.canonicalize().unwrap();
    println!(
        "{ANSI_GREEN}{ANSI_BOLD}Created book{ANSI_RESET} at {}",
        full_path.display()
    )
}

impl<'ctx> Diagnostic<'ctx> {
    pub fn err_content_outside_chapter<T>(span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new("content must be inside a chapter").with_error("", span);

        Err(vec![diag])
    }

    pub fn err_section_outside_chapter<T>(span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new("section must be inside a chapter").with_error("", span);

        Err(vec![diag])
    }

    pub fn err_subsection_outside_section<T>(span: Span) -> WResult<'ctx, T> {
        let diag = Diagnostic::new("subsection must be inside a section").with_error("", span);

        Err(vec![diag])
    }
}

#[derive(Debug)]
struct DocState {
    chapter_contents: Vec<String>,
    current_chapter_content: String,
    current_para_content: String,

    chapter: Option<usize>,
    section: Option<usize>,
    subsection: Option<usize>,
}

impl DocState {
    fn new() -> Self {
        Self {
            chapter_contents: Vec::new(),
            current_chapter_content: String::new(),
            current_para_content: String::new(),
            chapter: None,
            section: None,
            subsection: None,
        }
    }

    fn commit_paragraph(&mut self) {
        if self.current_para_content.is_empty() {
            return;
        }

        self.current_chapter_content += "<p>\n";
        self.current_chapter_content += &self.current_para_content;
        self.current_chapter_content += "</p>\n\n";

        self.current_para_content = String::new();
    }

    fn commit_chapter(&mut self) {
        if self.current_chapter_content.is_empty() {
            return;
        }

        let current = std::mem::take(&mut self.current_chapter_content);
        self.chapter_contents.push(current);
    }

    fn next_chapter<'ctx>(&mut self, title: &str) -> WResult<'ctx, ()> {
        self.commit_paragraph();
        self.commit_chapter();

        let next_chapter_num = self.chapter.unwrap_or(0) + 1;
        self.current_chapter_content += &replace_patterns(
            include_str!("templates/chapter_header.html"),
            &["{{CHAPTER_TITLE}}", "{{CHAPTER_NUM}}"],
            &[title, &next_chapter_num.to_string()],
        );

        self.chapter = Some(next_chapter_num);
        self.section = None;
        self.subsection = None;

        Ok(())
    }

    fn next_section<'ctx>(&mut self, title: &str, span: Span) -> WResult<'ctx, ()> {
        self.commit_paragraph();

        let Some(chapter_num) = self.chapter else {
            return Diagnostic::err_section_outside_chapter(span);
        };
        let next_section_num = self.section.unwrap_or(0) + 1;
        self.current_chapter_content += &replace_patterns(
            include_str!("templates/section_header.html"),
            &["{{SECTION_TITLE}}", "{{CHAPTER_NUM}}", "{{SECTION_NUM}}"],
            &[
                title,
                &chapter_num.to_string(),
                &next_section_num.to_string(),
            ],
        );
        self.section = Some(next_section_num);
        self.subsection = None;

        Ok(())
    }

    fn next_subsection<'ctx>(&mut self, title: &str, span: Span) -> WResult<'ctx, ()> {
        self.commit_paragraph();

        let Some(chapter_num) = self.chapter else {
            return Diagnostic::err_section_outside_chapter(span);
        };
        let Some(section_num) = self.section else {
            return Diagnostic::err_subsection_outside_section(span);
        };
        let next_subsection_num = self.subsection.unwrap_or(0) + 1;
        self.current_chapter_content += &replace_patterns(
            include_str!("templates/subsection_header.html"),
            &[
                "{{SUBSECTION_TITLE}}",
                "{{CHAPTER_NUM}}",
                "{{SECTION_NUM}}",
                "{{SUBSECTION_NUM}}",
            ],
            &[
                title,
                &chapter_num.to_string(),
                &section_num.to_string(),
                &next_subsection_num.to_string(),
            ],
        );

        self.subsection = Some(next_subsection_num);

        Ok(())
    }

    fn process_entries<'ctx>(&mut self, entries: &[ParseEntry<'ctx>], ctx: &mut Ctx<'ctx>) {
        for &entry in entries {
            match self.process_entry(entry, ctx) {
                Ok(_) => {}
                Err(err) => ctx.diags.add_diags(err),
            }
        }
        self.commit_paragraph();
        self.commit_chapter();
    }

    fn process_entry<'ctx>(
        &mut self,
        entry: ParseEntry<'ctx>,
        ctx: &Ctx<'ctx>,
    ) -> WResult<'ctx, ()> {
        match entry {
            ParseEntry::Text(span) => {
                let text = ctx.sources.get_text(span.source());
                let text = &text[span.bytes()];

                for line in text.line_spans() {
                    let start = SourceOffset::new(line.start());
                    let end = SourceOffset::new(line.end());
                    let line_span = Span::new(
                        Location::new(span.source(), start),
                        Location::new(span.source(), end),
                    );
                    self.process_text_line(line.as_str(), line_span)?;
                }

                Ok(())
            }
            ParseEntry::Command(parse_tree) => {
                // TODO
                Ok(())
            }
        }
    }

    fn process_text_line<'ctx>(&mut self, line: &str, span: Span) -> WResult<'ctx, ()> {
        let line = line.trim_start();

        if let Some(subsection_title) = line.strip_prefix("===") {
            self.next_subsection(subsection_title, span)
        } else if let Some(section_title) = line.strip_prefix("==") {
            self.next_section(section_title, span)
        } else if let Some(chapter_title) = line.strip_prefix("=") {
            self.next_chapter(chapter_title)
        } else if line.is_empty() {
            self.commit_paragraph();
            Ok(())
        } else {
            self.process_para_text(line, span)
        }
    }

    fn process_para_text<'ctx>(&mut self, text: &str, span: Span) -> WResult<'ctx, ()> {
        if self.chapter.is_none() {
            return Diagnostic::err_content_outside_chapter(span);
        }

        self.current_para_content += &process_inline_formatting(text);
        self.current_para_content += "\n";

        Ok(())
    }
}

fn replace_patterns(template: &str, patterns: &[&str], replacements: &[&str]) -> String {
    AhoCorasick::new(patterns)
        .unwrap()
        .replace_all(template, replacements)
}

fn process_inline_formatting(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '$' && chars[i + 1] == '$' {
            // Check for display math $$...$$
            if let Some(end) = find_double_dollar_end(&chars, i + 2) {
                let latex = chars[i + 2..end].iter().collect::<String>();
                let rendered = render_latex(&latex, true);
                result.push_str(&rendered);
                i = end + 2;
                continue;
            }
        } else if chars[i] == '$' {
            // Check for inline math $...$
            if let Some(end) = find_closing_delimiter(&chars, i + 1, '$') {
                let latex = chars[i + 1..end].iter().collect::<String>();
                let rendered = render_latex(&latex, false);
                result.push_str(&rendered);
                i = end + 1;
                continue;
            }
        } else if chars[i] == '*' {
            // Check for bold *...*
            if let Some(end) = find_closing_delimiter(&chars, i + 1, '*') {
                result.push_str("<strong>");
                result.push_str(&chars[i + 1..end].iter().collect::<String>());
                result.push_str("</strong>");
                i = end + 1;
                continue;
            }
        } else if chars[i] == '_' {
            // Check for italic _..._
            if let Some(end) = find_closing_delimiter(&chars, i + 1, '_') {
                result.push_str("<em>");
                result.push_str(&chars[i + 1..end].iter().collect::<String>());
                result.push_str("</em>");
                i = end + 1;
                continue;
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

fn find_closing_delimiter(chars: &[char], start: usize, delimiter: char) -> Option<usize> {
    (start..chars.len()).find(|&i| chars[i] == delimiter)
}

fn find_double_dollar_end(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start;
    while i + 1 < chars.len() {
        if chars[i] == '$' && chars[i + 1] == '$' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn render_latex(latex: &str, display_mode: bool) -> String {
    let ctx = katex::KatexContext::default();
    let settings = katex::Settings::builder()
        .display_mode(display_mode)
        .throw_on_error(false)
        .build();

    katex::render_to_string(&ctx, latex, &settings).unwrap()
}
