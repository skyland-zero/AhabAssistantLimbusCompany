//! Small native Markdown renderer for the bundled help documents.
//!
//! This page intentionally renders the bundled Markdown directly with GPUI;
//! it does not embed a WebView. The document and directory have independent
//! layout regions, and directory entries address direct document children so
//! `ScrollHandle::scroll_to_top_of_item` remains deterministic.

use std::process::Command;

use gpui::{Context, Div, FontWeight, div, prelude::*, px, rgb};

use crate::{
    app::{ACCENT, AhabApp, BACKGROUND, BORDER, SURFACE, TEXT, TEXT_MUTED},
    assets,
    components::{ButtonVariant, button},
    i18n::{self, Key as I18nKey},
    model::Language,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum HelpBlock {
    Heading { level: u8, text: String },
    Paragraph(String),
    Bullet(String),
    Ordered(String),
    Code(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum InlinePart {
    Text(String),
    Strong(String),
    Code(String),
    Link { label: String, url: String },
}

#[derive(Clone, Copy)]
struct Localized {
    zh: &'static str,
    en: &'static str,
}

impl Localized {
    fn get(self, language: Language) -> &'static str {
        match language {
            Language::ZhCn => self.zh,
            Language::EnUs => self.en,
        }
    }
}

const fn text(zh: &'static str, en: &'static str) -> Localized {
    Localized { zh, en }
}

pub fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let source = String::from_utf8_lossy(assets::help(language).embedded());
    // The React page hides the document's single H1. Omitting it here also
    // keeps TOC indices aligned with the visible direct children of the scroll.
    let blocks: Vec<HelpBlock> = parse_help(source.as_ref())
        .into_iter()
        .filter(|block| !matches!(block, HelpBlock::Heading { level: 1, .. }))
        .collect();
    let toc: Vec<(usize, String)> = blocks
        .iter()
        .enumerate()
        .filter_map(|(index, block)| match block {
            HelpBlock::Heading { level: 2, text } => Some((index, text.clone())),
            _ => None,
        })
        .collect();

    let mut toc_view = div().flex().flex_col().gap_0p5();
    for (index, title) in toc {
        let mut link = button(title, ButtonVariant::Ghost)
            .id(format!("help-toc-{index}"))
            .w_full()
            .justify_start()
            .px_2()
            .py_1p5()
            .text_size(px(12.))
            .text_color(rgb(TEXT_MUTED));
        link = link.on_click(cx.listener(move |view, _, _, cx| {
            view.help_scroll.scroll_to_top_of_item(index);
            cx.notify();
        }));
        toc_view = toc_view.child(link);
    }

    let mut language_switch = div().flex().items_center().gap_1();
    for (candidate, label) in [(Language::ZhCn, "简体中文"), (Language::EnUs, "English")] {
        let mut control = button(
            label,
            if candidate == language {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Ghost
            },
        )
        .id(format!("help-language-{candidate:?}"))
        .px_2()
        .py_1()
        .text_size(px(11.));
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            view.set_language(candidate);
            view.help_scroll = gpui::ScrollHandle::new();
            cx.notify();
        }));
        language_switch = language_switch.child(control);
    }

    let directory = div()
        .w(px(208.))
        .flex_none()
        .flex()
        .flex_col()
        .gap_2()
        .border_r_1()
        .border_color(rgb(BORDER))
        .p_3()
        .bg(rgb(SURFACE))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(TEXT_MUTED))
                        .child(i18n::text(language, I18nKey::Contents)),
                )
                .child(language_switch),
        )
        .child(toc_view);

    let mut document = div()
        .id("help-document-scroll")
        .w_full()
        .max_w(px(672.))
        .mx_auto()
        .overflow_y_scroll()
        .scrollbar_width(px(6.))
        .track_scroll(&app.help_scroll)
        .flex()
        .flex_col()
        .gap_2()
        .px_8()
        .py_6()
        .flex_1()
        .min_w_0();
    for block in blocks {
        document = document.child(render_block(block, cx));
    }

    div()
        .size_full()
        .flex()
        .bg(rgb(BACKGROUND))
        .child(directory)
        .child(document)
}

fn render_block(block: HelpBlock, cx: &mut Context<AhabApp>) -> Div {
    match block {
        HelpBlock::Heading { level: 1, .. } => div().hidden(),
        HelpBlock::Heading { level: 2, text } => div()
            .mt_8()
            .pb_1()
            .border_b_1()
            .border_color(rgb(BORDER))
            .text_size(px(16.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(TEXT))
            .child(render_inline(&text, cx)),
        HelpBlock::Heading { level: _, text } => div()
            .mt_5()
            .text_size(px(14.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(TEXT))
            .child(render_inline(&text, cx)),
        HelpBlock::Paragraph(text) => div()
            .my_2()
            .text_size(px(14.))
            .line_height(px(22.))
            .text_color(rgb(TEXT))
            .child(render_inline(&text, cx)),
        HelpBlock::Bullet(text) => div()
            .flex()
            .items_start()
            .gap_2()
            .pl_5()
            .my_1()
            .text_size(px(14.))
            .line_height(px(22.))
            .text_color(rgb(TEXT))
            .child(div().flex_none().text_color(rgb(ACCENT)).child("•"))
            .child(render_inline(&text, cx)),
        HelpBlock::Ordered(text) => div()
            .pl_5()
            .my_1()
            .text_size(px(14.))
            .line_height(px(22.))
            .text_color(rgb(TEXT))
            .child(render_inline(&text, cx)),
        HelpBlock::Code(text) => div()
            .w_full()
            .my_2()
            .p_3()
            .rounded_md()
            .bg(rgb(0x171e29))
            .font_family("Consolas")
            .text_size(px(12.))
            .line_height(px(19.))
            .text_color(rgb(0xb8c8dd))
            .child(text),
    }
}

fn render_inline(text: &str, cx: &mut Context<AhabApp>) -> Div {
    let mut view = div().flex().flex_wrap().items_center().gap_0();
    for part in parse_inline(text) {
        match part {
            InlinePart::Text(value) => view = view.child(value),
            InlinePart::Strong(value) => {
                view = view.child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT))
                        .child(value),
                )
            }
            InlinePart::Code(value) => {
                view = view.child(
                    div()
                        .px_1()
                        .rounded_sm()
                        .bg(rgb(SURFACE))
                        .font_family("Consolas")
                        .text_size(px(12.))
                        .child(value),
                )
            }
            InlinePart::Link { label, url } => {
                let mut link = button(label, ButtonVariant::Ghost)
                    .id(format!("help-link-{}", url))
                    .px_1()
                    .py_0()
                    .text_size(px(14.))
                    .text_color(rgb(ACCENT))
                    .underline();
                link = link.on_click(cx.listener(move |_, _, _, _| {
                    open_external_url(&url);
                }));
                view = view.child(link);
            }
        }
    }
    view
}

fn parse_help(source: &str) -> Vec<HelpBlock> {
    let mut blocks = Vec::new();
    let mut paragraph = Vec::new();
    let mut code = false;
    let mut code_lines = Vec::new();

    let flush_paragraph = |blocks: &mut Vec<HelpBlock>, paragraph: &mut Vec<String>| {
        if !paragraph.is_empty() {
            blocks.push(HelpBlock::Paragraph(paragraph.join(" ")));
            paragraph.clear();
        }
    };
    let flush_code = |blocks: &mut Vec<HelpBlock>, code_lines: &mut Vec<String>| {
        if !code_lines.is_empty() {
            blocks.push(HelpBlock::Code(code_lines.join("\n")));
            code_lines.clear();
        }
    };

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if code {
                flush_code(&mut blocks, &mut code_lines);
            } else {
                flush_paragraph(&mut blocks, &mut paragraph);
            }
            code = !code;
            continue;
        }
        if code {
            code_lines.push(line.to_owned());
            continue;
        }
        if trimmed.is_empty() {
            flush_paragraph(&mut blocks, &mut paragraph);
        } else if let Some((level, title)) = heading(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(HelpBlock::Heading {
                level,
                text: inline_text(title),
            });
        } else if let Some(text) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(HelpBlock::Bullet(text.to_owned()));
        } else if let Some(text) = ordered_item(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(HelpBlock::Ordered(text));
        } else {
            paragraph.push(trimmed.to_owned());
        }
    }
    if code {
        flush_code(&mut blocks, &mut code_lines);
    } else {
        flush_paragraph(&mut blocks, &mut paragraph);
    }
    blocks
}

fn heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if (1..=3).contains(&hashes) && line.as_bytes().get(hashes) == Some(&b' ') {
        Some((hashes as u8, line[hashes + 1..].trim()))
    } else {
        None
    }
}

fn ordered_item(line: &str) -> Option<String> {
    let split = line.find(". ")?;
    if split > 0
        && line[..split]
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        Some(format!("{}  {}", &line[..split], &line[split + 2..]))
    } else {
        None
    }
}

fn parse_inline(text: &str) -> Vec<InlinePart> {
    let mut parts = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        let next = [
            remaining.find("**"),
            remaining.find('`'),
            remaining.find('['),
        ]
        .into_iter()
        .flatten()
        .min();
        let Some(index) = next else {
            parts.push(InlinePart::Text(remaining.to_owned()));
            break;
        };
        if index > 0 {
            parts.push(InlinePart::Text(remaining[..index].to_owned()));
            remaining = &remaining[index..];
            continue;
        }

        if remaining.starts_with("**") {
            if let Some(end) = remaining[2..].find("**") {
                let value_end = 2 + end;
                parts.push(InlinePart::Strong(remaining[2..value_end].to_owned()));
                remaining = &remaining[value_end + 2..];
                continue;
            }
            parts.push(InlinePart::Text("**".to_owned()));
            remaining = &remaining[2..];
            continue;
        }

        if remaining.starts_with('`') {
            if let Some(end) = remaining[1..].find('`') {
                let value_end = 1 + end;
                parts.push(InlinePart::Code(remaining[1..value_end].to_owned()));
                remaining = &remaining[value_end + 1..];
                continue;
            }
            parts.push(InlinePart::Text("`".to_owned()));
            remaining = &remaining[1..];
            continue;
        }

        if remaining.starts_with('[') {
            if let Some(label_end) = remaining[1..].find("](") {
                let label_end = label_end + 1;
                let url_start = label_end + 2;
                if let Some(url_end) = remaining[url_start..].find(')') {
                    let url_end = url_start + url_end;
                    parts.push(InlinePart::Link {
                        label: remaining[1..label_end].to_owned(),
                        url: remaining[url_start..url_end].to_owned(),
                    });
                    remaining = &remaining[url_end + 1..];
                    continue;
                }
            }
            parts.push(InlinePart::Text("[".to_owned()));
            remaining = &remaining[1..];
            continue;
        }
    }
    parts
}

fn inline_text(text: impl AsRef<str>) -> String {
    parse_inline(text.as_ref())
        .into_iter()
        .map(|part| match part {
            InlinePart::Text(value) | InlinePart::Strong(value) | InlinePart::Code(value) => value,
            InlinePart::Link { label, .. } => label,
        })
        .collect()
}

fn open_external_url(url: &str) {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return;
    }
    #[cfg(windows)]
    {
        let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(url).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HelpBlock, InlinePart, heading, inline_text, ordered_item, parse_help, parse_inline,
    };

    #[test]
    fn parser_extracts_headings_lists_and_code_without_a_webview() {
        let blocks = parse_help("# Title\n\n## Section\n\n- **item**\n1. step\n\n```\ncode\n```");
        assert!(blocks.contains(&HelpBlock::Heading {
            level: 2,
            text: "Section".into()
        }));
        assert!(blocks.contains(&HelpBlock::Bullet("**item**".into())));
        assert!(blocks.contains(&HelpBlock::Ordered("1  step".into())));
        assert!(blocks.contains(&HelpBlock::Code("code".into())));
    }

    #[test]
    fn inline_markdown_preserves_bold_code_and_link_parts() {
        assert_eq!(
            parse_inline("**bold** and [link](https://example.test) `code`"),
            vec![
                InlinePart::Strong("bold".into()),
                InlinePart::Text(" and ".into()),
                InlinePart::Link {
                    label: "link".into(),
                    url: "https://example.test".into(),
                },
                InlinePart::Text(" ".into()),
                InlinePart::Code("code".into()),
            ]
        );
        assert_eq!(
            inline_text("**bold** and [link](https://example.test) `code`"),
            "bold and link code"
        );
        assert_eq!(heading("### Details"), Some((3, "Details")));
        assert_eq!(ordered_item("12. item"), Some("12  item".into()));
    }
}
