use crate::{
    context::Ctx,
    diagnostics::{Diagnostic, WResult},
    parse::{
        Location, ParseEntry, ParseReport, Span,
        location::SourceOffset,
        parse_state::ParseRuleSource,
        parse_tree::{ParseAtomKind, ParseTreeId, ParseTreePart},
    },
    report::ProofReport,
    util::ansi::{ANSI_BOLD, ANSI_GREEN, ANSI_RESET},
};
use aho_corasick::AhoCorasick;
use line_span::LineSpanExt;
use std::{fs, path::PathBuf};

pub mod server;

pub fn build_book<'ctx>(
    ctx: &mut Ctx<'ctx>,
    parse_report: ParseReport<'ctx>,
    _proof_report: ProofReport<'ctx>,
) -> PathBuf {
    ctx.diags.clear_errors();

    let mut doc = DocState::new();
    doc.process_entries(&parse_report.entries, ctx);

    if ctx.diags.has_errors() {
        ctx.diags.print_errors(ctx);
        std::process::exit(1);
    }

    // Delete existing book directory to ensure clean build
    let book_dir = ctx.config.build_dir().join("book");
    if book_dir.exists() {
        fs::remove_dir_all(&book_dir).expect("Failed to remove old book directory");
    }
    fs::create_dir_all(&book_dir).unwrap();

    let css_path = book_dir.join("styles.css");
    fs::write(css_path, include_str!("templates/styles.css")).expect("TODO");

    for (i, chapter_contents) in doc.chapter_contents.iter().enumerate() {
        let chapter_num = i + 1;
        let chapter_title = &doc.chapter_titles[i];
        let page_title = match ctx.config.book().title() {
            Some(book_title) => format!("{} - {}", chapter_title, book_title),
            None => chapter_title.to_string(),
        };

        let chapter_dir = book_dir.join(format!("chapter-{chapter_num}"));
        fs::create_dir_all(&chapter_dir).expect("Failed to create chapter directory");
        let path = chapter_dir.join("index.html");
        let content = replace_patterns(
            include_str!("templates/layout.html"),
            &[
                "{{PAGE_TITLE}}",
                "{{SIDEBAR}}",
                "{{CHAPTER_CONTENT}}",
                "{{CHAPTER_NUM}}",
            ],
            &[
                &page_title,
                &doc.sidebar_content,
                chapter_contents,
                &chapter_num.to_string(),
            ],
        );
        fs::write(path, content).expect("TODO");
    }

    let full_path = book_dir.canonicalize().unwrap();
    println!(
        "{ANSI_GREEN}{ANSI_BOLD}Created book{ANSI_RESET} at {}",
        full_path.display()
    );

    full_path
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
    chapter_titles: Vec<String>,
    current_chapter_content: String,
    current_para_content: String,
    sidebar_content: String,

    chapter: Option<usize>,
    section: Option<usize>,
    subsection: Option<usize>,
}

impl DocState {
    fn new() -> Self {
        Self {
            chapter_contents: Vec::new(),
            chapter_titles: Vec::new(),
            current_chapter_content: String::new(),
            current_para_content: String::new(),
            sidebar_content: String::new(),
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

        self.close_section();

        let current = std::mem::take(&mut self.current_chapter_content);
        self.chapter_contents.push(current);

        self.sidebar_content += "</ol>\n";
        self.sidebar_content += "</li>\n";
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
        self.chapter_titles.push(title.to_string());
        self.section = None;
        self.subsection = None;

        self.sidebar_content += "<li>\n";
        self.sidebar_content += &format!(
            "<a href=\"/chapter-{}/\" class=\"chapter\" data-chapter=\"{}\"><span class=\"num\">{}</span> {}</a>\n",
            next_chapter_num, next_chapter_num, next_chapter_num, title
        );
        self.sidebar_content += "<ol class=\"section-list\">\n";

        Ok(())
    }

    fn close_section(&mut self) {
        if self.section.is_some() {
            self.current_chapter_content += "</section>\n";
        }
    }

    fn next_section<'ctx>(&mut self, title: &str, span: Span) -> WResult<'ctx, ()> {
        self.commit_paragraph();
        self.close_section();

        let Some(chapter_num) = self.chapter else {
            return Diagnostic::err_section_outside_chapter(span);
        };
        let next_section_num = self.section.unwrap_or(0) + 1;
        self.current_chapter_content += &format!("<section id=\"section-{}\">\n", next_section_num);
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

        self.sidebar_content += &format!(
            "<li class=\"section\"><a href=\"/chapter-{}/#section-{}\" data-chapter=\"{}\" data-section=\"{}\"><span class=\"num\">{}.{}</span> {}</a></li>\n",
            chapter_num,
            next_section_num,
            chapter_num,
            next_section_num,
            chapter_num,
            next_section_num,
            title
        );

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
        self.sidebar_content += r#"<ol class="chapter-list">"#;
        self.sidebar_content += "\n";

        for &entry in entries {
            match self.process_entry(entry, ctx) {
                Ok(_) => {}
                Err(err) => ctx.diags.add_diags(err),
            }
        }

        self.sidebar_content += r#"</ol>"#;
        self.sidebar_content += "\n";
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
                if self.chapter.is_none() {
                    return Diagnostic::err_content_outside_chapter(parse_tree.0.span());
                }

                let span = parse_tree.0.span();
                let source_text = ctx.sources.get_text(span.source());
                let command_text = &source_text[span.bytes()];

                // Get starting line number using the optimized method
                let start_line = ctx.sources.get_line_number(span.start());

                // Collect syntax highlighting information
                let highlights = collect_highlights(
                    parse_tree,
                    span.start().byte_offset(),
                    source_text.as_str(),
                );

                // Commit any existing paragraph before adding the code block
                self.commit_paragraph();

                // Add code block with line numbers and syntax highlighting
                self.current_chapter_content += "<pre><code>";
                let mut byte_offset = 0;
                for (i, line) in command_text.lines().enumerate() {
                    let line_num = start_line + i;
                    self.current_chapter_content += r#"<span class="line">"#;
                    self.current_chapter_content += &line_num.to_string();
                    self.current_chapter_content += r#"</span>"#;

                    // Calculate byte offsets for this line within the command
                    let line_start = byte_offset;
                    let line_end = byte_offset + line.len();

                    self.current_chapter_content += r#"<span>"#;
                    self.current_chapter_content +=
                        &render_highlighted_line(line, line_start, line_end, &highlights);
                    self.current_chapter_content += r#"</span>"#;
                    self.current_chapter_content += "\n";

                    // Move to next line (line length + newline)
                    byte_offset = line_end + 1;
                }
                self.current_chapter_content += "</code></pre>\n";

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

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HighlightKind {
    Keyword,
    Name,
    StrLit,
    Num,
    Lit,
    Comment,
}

#[derive(Debug, Clone, Copy)]
struct Highlight {
    start: usize,
    end: usize,
    kind: HighlightKind,
}

/// Collect syntax highlighting information from the parse tree
fn collect_highlights<'ctx>(
    parse_tree: ParseTreeId<'ctx>,
    offset: usize,
    source_text: &str,
) -> Vec<Highlight> {
    let mut highlights = Vec::new();

    fn visit_tree<'ctx>(
        tree: ParseTreeId<'ctx>,
        highlights: &mut Vec<Highlight>,
        offset: usize,
        source_text: &str,
        prev_was_keyword: bool,
    ) {
        // Just use the first possibility for highlighting
        if let Some(possibility) = tree.0.possibilities().first() {
            // Check if this rule is from a notation (fragment) - if so, skip highlighting
            if matches!(possibility.rule().0.source(), ParseRuleSource::Notation(_)) {
                return;
            }

            let mut last_atom_was_keyword = prev_was_keyword;
            for child in possibility.children() {
                match child {
                    ParseTreePart::Atom(atom) => {
                        let full_span = atom.full_span();
                        let span = atom.span();

                        // Check for comments in the whitespace before this atom
                        if full_span.start().byte_offset() < span.start().byte_offset() {
                            let ignored_text = &source_text
                                [full_span.start().byte_offset()..span.start().byte_offset()];
                            if !ignored_text.trim().is_empty() {
                                // The ignored text contains non-whitespace, which
                                // must be a comment.
                                highlights.push(Highlight {
                                    start: full_span.start().byte_offset() - offset,
                                    end: span.start().byte_offset() - offset,
                                    kind: HighlightKind::Comment,
                                });
                            }
                        }

                        let atom_kind = atom._kind();
                        let is_keyword = matches!(atom_kind, ParseAtomKind::Kw(_));

                        let kind = match atom_kind {
                            ParseAtomKind::Kw(_) => Some(HighlightKind::Keyword),
                            ParseAtomKind::Name(_) => {
                                // Only highlight names that immediately follow a keyword
                                if last_atom_was_keyword {
                                    Some(HighlightKind::Name)
                                } else {
                                    None
                                }
                            }
                            ParseAtomKind::StrLit(_) => Some(HighlightKind::StrLit),
                            ParseAtomKind::Num(_) => Some(HighlightKind::Num),
                            ParseAtomKind::Lit(text) => {
                                // Only highlight literals that aren't common punctuation
                                let common = matches!(
                                    text.as_str(),
                                    "(" | ")" | "[" | "]" | "{" | "}" | ":" | ";"
                                );
                                if common {
                                    None
                                } else {
                                    Some(HighlightKind::Lit)
                                }
                            }
                        };

                        if let Some(kind) = kind {
                            highlights.push(Highlight {
                                start: span.start().byte_offset() - offset,
                                end: span.end().byte_offset() - offset,
                                kind,
                            });
                        }

                        // Update flag for next atom
                        last_atom_was_keyword = is_keyword;
                    }
                    ParseTreePart::Node { id, .. } => {
                        visit_tree(*id, highlights, offset, source_text, last_atom_was_keyword);
                        // After visiting a node, reset the flag since nodes break the immediate sequence
                        last_atom_was_keyword = false;
                    }
                }
            }
        }
    }

    visit_tree(parse_tree, &mut highlights, offset, source_text, false);
    highlights.sort_by_key(|h| h.start);
    highlights
}

/// Render a line with syntax highlighting
fn render_highlighted_line(
    line: &str,
    line_start: usize,
    line_end: usize,
    highlights: &[Highlight],
) -> String {
    let mut result = String::new();
    let mut pos = line_start;

    // Find highlights that overlap with this line
    for highlight in highlights {
        // Skip highlights that end before this line
        if highlight.end <= line_start {
            continue;
        }
        // Stop processing highlights that start after this line
        if highlight.start >= line_end {
            break;
        }

        // Calculate the portion of the highlight within this line
        let hl_start = highlight.start.max(line_start);
        let hl_end = highlight.end.min(line_end);

        // Add any unhighlighted text before this highlight
        if pos < hl_start {
            let text = &line[(pos - line_start)..(hl_start - line_start)];
            result.push_str(&html_escape(text));
        }

        // Add the highlighted text
        let class = match highlight.kind {
            HighlightKind::Keyword => "kw",
            HighlightKind::Name => "name",
            HighlightKind::StrLit => "str",
            HighlightKind::Num => "number",
            HighlightKind::Lit => "lit",
            HighlightKind::Comment => "comment",
        };

        result.push_str(&format!(r#"<span class="{}">"#, class));
        let text = &line[(hl_start - line_start)..(hl_end - line_start)];
        result.push_str(&html_escape(text));
        result.push_str("</span>");

        pos = hl_end;
    }

    // Add any remaining unhighlighted text
    if pos < line_end {
        let text = &line[(pos - line_start)..];
        result.push_str(&html_escape(text));
    }

    result
}
