use crate::{
    context::Ctx,
    diagnostics::{Diagnostic, DiagnosticSpan, WResult},
    parse::{
        ParseEntry, ParseReport, Span,
        parse_state::ParseRuleSource,
        parse_tree::{ParseAtomKind, ParseTreeId, ParseTreePart},
    },
    report::ProofReport,
    util::ansi::{ANSI_BOLD, ANSI_GREEN, ANSI_RESET},
};
use aho_corasick::AhoCorasick;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::{fs, path::PathBuf};

pub mod server;

pub fn build_book<'ctx>(
    ctx: &mut Ctx<'ctx>,
    parse_report: ParseReport<'ctx>,
    _proof_report: ProofReport<'ctx>,
    watch: bool,
    base_path: &str,
) -> PathBuf {
    ctx.diags.clear_errors();

    let mut doc = DocState::new(base_path.to_string());
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

    // Include auto-reload script only for watch mode
    let auto_reload_script = if watch {
        include_str!("templates/auto_reload.js")
    } else {
        ""
    };

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
                "{{AUTO_RELOAD_SCRIPT}}",
                "{{BASE_PATH}}",
            ],
            &[
                &page_title,
                &doc.sidebar_content,
                chapter_contents,
                &chapter_num.to_string(),
                auto_reload_script,
                base_path,
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
        let diag = Diagnostic::new(
            "content must be inside a chapter",
            vec![DiagnosticSpan::new_error("", span)],
        );
        Err(vec![diag])
    }
}

#[derive(Debug)]
struct DocState {
    chapter_contents: Vec<String>,
    chapter_titles: Vec<String>,
    current_chapter_content: String,
    sidebar_content: String,
    base_path: String,

    chapter: Option<usize>,
    section: Option<usize>,
}

impl DocState {
    fn new(base_path: String) -> Self {
        Self {
            chapter_contents: Vec::new(),
            chapter_titles: Vec::new(),
            current_chapter_content: String::new(),
            sidebar_content: String::new(),
            base_path,
            chapter: None,
            section: None,
        }
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

        self.sidebar_content += "<li>\n";
        self.sidebar_content += &format!(
            "<a href=\"{}chapter-{}/\" class=\"chapter\" data-chapter=\"{}\"><span class=\"num\">{}</span> {}</a>\n",
            self.base_path, next_chapter_num, next_chapter_num, next_chapter_num, title
        );
        self.sidebar_content += "<ol class=\"section-list\">\n";

        Ok(())
    }

    fn close_section(&mut self) {
        if self.section.is_some() {
            self.current_chapter_content += "</section>\n";
        }
    }

    fn next_section<'ctx>(&mut self, title: &str) -> WResult<'ctx, ()> {
        self.close_section();

        let Some(chapter_num) = self.chapter else {
            // TODO
            return Ok(());
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

        self.sidebar_content += &format!(
            "<li class=\"section\"><a href=\"{}chapter-{}/#section-{}\" data-chapter=\"{}\" data-section=\"{}\"><span class=\"num\">{}.{}</span> {}</a></li>\n",
            self.base_path,
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
                self.process_markdown_text(text)?;
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

                // Add code block with line numbers and syntax highlighting
                self.current_chapter_content += r#"<pre><code class="code-block">"#;
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

    fn process_markdown_text<'ctx>(&mut self, text: &str) -> WResult<'ctx, ()> {
        // Remove Watson-style -- comments before processing markdown
        let text_without_comments = strip_watson_comments(text);

        // Enable math support in pulldown-cmark
        let mut options = Options::empty();
        options.insert(Options::ENABLE_MATH);
        let parser = Parser::new_ext(&text_without_comments, options);

        let mut in_heading: Option<HeadingLevel> = None;
        let mut heading_text = String::new();

        for event in parser {
            match event {
                Event::Start(tag) => match tag {
                    Tag::Heading { level, .. } => {
                        in_heading = Some(level);
                        heading_text.clear();
                    }
                    _ if in_heading.is_none() => match tag {
                        Tag::Paragraph => self.current_chapter_content += "<p>",
                        Tag::BlockQuote(_) => self.current_chapter_content += "<blockquote>",
                        Tag::CodeBlock(_) => self.current_chapter_content += "<pre><code>",
                        Tag::List(None) => self.current_chapter_content += "<ul>",
                        Tag::List(Some(_)) => self.current_chapter_content += "<ol>",
                        Tag::Item => self.current_chapter_content += "<li>",
                        Tag::Emphasis => self.current_chapter_content += "<em>",
                        Tag::Strong => self.current_chapter_content += "<strong>",
                        Tag::Link { dest_url, .. } => {
                            self.current_chapter_content +=
                                &format!(r#"<a href="{}">"#, html_escape(&dest_url));
                        }
                        _ => {}
                    },
                    _ => {}
                },
                Event::End(tag_end) => {
                    match tag_end {
                        TagEnd::Heading(level) => {
                            // Handle the heading based on its level
                            match level {
                                HeadingLevel::H1 => {
                                    self.next_chapter(&heading_text)?;
                                }
                                HeadingLevel::H2 => {
                                    self.next_section(&heading_text)?;
                                }
                                _ => {
                                    // Regular headings
                                    self.current_chapter_content +=
                                        &format!("<{}>", heading_tag(level));
                                    self.current_chapter_content += &heading_text;
                                    self.current_chapter_content +=
                                        &format!("</{}>", heading_tag(level));
                                }
                            }
                            in_heading = None;
                            heading_text.clear();
                        }
                        _ if in_heading.is_none() => match tag_end {
                            TagEnd::Paragraph => self.current_chapter_content += "</p>\n",
                            TagEnd::BlockQuote(_) => {
                                self.current_chapter_content += "</blockquote>\n"
                            }
                            TagEnd::CodeBlock => self.current_chapter_content += "</code></pre>\n",
                            TagEnd::List(false) => self.current_chapter_content += "</ul>\n",
                            TagEnd::List(true) => self.current_chapter_content += "</ol>\n",
                            TagEnd::Item => self.current_chapter_content += "</li>\n",
                            TagEnd::Emphasis => self.current_chapter_content += "</em>",
                            TagEnd::Strong => self.current_chapter_content += "</strong>",
                            TagEnd::Link => self.current_chapter_content += "</a>",
                            _ => {}
                        },
                        _ => {}
                    }
                }
                Event::Text(text) => {
                    if in_heading.is_some() {
                        // Accumulate heading text
                        heading_text.push_str(&text);
                    } else {
                        self.current_chapter_content += &html_escape(&text);
                    }
                }
                Event::Code(code) => {
                    if in_heading.is_some() {
                        heading_text.push_str(&code);
                    } else {
                        self.current_chapter_content += "<code>";
                        self.current_chapter_content += &html_escape(&code);
                        self.current_chapter_content += "</code>";
                    }
                }
                Event::Html(html) => {
                    if in_heading.is_some() {
                        // Strip HTML tags from headings
                        heading_text.push_str(&html);
                    } else {
                        // Preserve HTML (including rendered math from KaTeX)
                        self.current_chapter_content += &html;
                    }
                }
                Event::InlineMath(latex) => {
                    if in_heading.is_some() {
                        // Include math in heading text (rendered)
                        heading_text.push_str(&render_latex(&latex, false));
                    } else {
                        let rendered = render_latex(&latex, false);
                        self.current_chapter_content += &rendered;
                    }
                }
                Event::DisplayMath(latex) => {
                    if in_heading.is_some() {
                        // Include math in heading text (rendered)
                        heading_text.push_str(&render_latex(&latex, false));
                    } else {
                        let rendered = render_latex(&latex, true);
                        self.current_chapter_content += &rendered;
                    }
                }
                Event::SoftBreak if in_heading.is_none() => {
                    self.current_chapter_content += "\n";
                }
                Event::HardBreak if in_heading.is_none() => {
                    self.current_chapter_content += "<br/>\n";
                }
                _ => {}
            }
        }

        Ok(())
    }
}

/// Strip Watson-style -- comments from text
/// Comments start with -- and continue to the end of the line
fn strip_watson_comments(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for line in text.lines() {
        // Find the position of -- comment starter
        if let Some(comment_pos) = line.find("--") {
            // Keep everything before the comment
            result.push_str(&line[..comment_pos]);
            result.push(' ');
        } else {
            // No comment, keep the whole line
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

fn replace_patterns(template: &str, patterns: &[&str], replacements: &[&str]) -> String {
    AhoCorasick::new(patterns)
        .unwrap()
        .replace_all(template, replacements)
}

fn heading_tag(level: HeadingLevel) -> &'static str {
    match level {
        HeadingLevel::H1 => "h1",
        HeadingLevel::H2 => "h2",
        HeadingLevel::H3 => "h3",
        HeadingLevel::H4 => "h4",
        HeadingLevel::H5 => "h5",
        HeadingLevel::H6 => "h6",
    }
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
