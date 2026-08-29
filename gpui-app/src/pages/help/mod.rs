//! Small native Markdown renderer for the bundled help documents.
//!
//! This page intentionally renders the bundled Markdown directly with GPUI;
//! it does not embed a WebView. The document and directory have independent
//! layout regions, and directory entries address direct document children so
//! `ScrollHandle::scroll_to_top_of_item` remains deterministic.

use std::process::Command;

use gpui::{Context, Div, FontWeight, KeyDownEvent, div, point, prelude::*, px};

use crate::{
    app::{ACCENT, AhabApp, BACKGROUND, BORDER, SURFACE, TEXT, TEXT_MUTED},
    assets,
    components::style::current_render_palette,
    components::{
        ButtonVariant, button, is_activation_key, palette_rgb, render_rgb as rgb,
        scroll_area_with_handle,
    },
    i18n::{self, Key as I18nKey},
    model::Language,
};

mod parser;

use parser::{HelpBlock, InlinePart, parse_help, parse_inline};

const HELP_SCROLL_ID: &'static str = "help-document-scroll";

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
        link = link
            .on_click(cx.listener(move |view, _, _, cx| {
                view.help_scroll.scroll_to_top_of_item(index);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.help_scroll.scroll_to_top_of_item(index);
                    cx.notify();
                }
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
        control = control
            .on_click(cx.listener(move |view, _, _, cx| {
                view.set_language(candidate);
                view.help_scroll.set_offset(point(px(0.), px(0.)));
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.set_language(candidate);
                    view.help_scroll.set_offset(point(px(0.), px(0.)));
                    cx.notify();
                }
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

    let mut document = div().w_full().flex().flex_col().gap_2().px_8().py_6();
    for block in blocks {
        document = document.child(render_block(block, cx));
    }

    let scroll_handle = app.help_scroll.clone();
    let document = scroll_area_with_handle(app, HELP_SCROLL_ID, document, scroll_handle)
        .w_full()
        .max_w(px(672.))
        .mx_auto()
        .scrollbar_width(px(6.))
        .flex_1()
        .min_w_0();

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
            .child(div().flex_none().text_color(rgb(TEXT)).child("•"))
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
            .bg(palette_rgb(current_render_palette().popover))
            .font_family("Consolas")
            .text_size(px(12.))
            .line_height(px(19.))
            .text_color(palette_rgb(current_render_palette().foreground))
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
                let url_for_key = url.clone();
                let mut link = button(label, ButtonVariant::Ghost)
                    .id(format!("help-link-{}", url))
                    .px_1()
                    .py_0()
                    .text_size(px(14.))
                    .text_color(rgb(ACCENT))
                    .underline();
                link = link
                    .on_click(cx.listener(move |_, _, _, _| {
                        open_external_url(&url);
                    }))
                    .on_key_down(cx.listener(move |_, event: &KeyDownEvent, window, _| {
                        if is_activation_key(event) {
                            window.prevent_default();
                            open_external_url(&url_for_key);
                        }
                    }));
                view = view.child(link);
            }
        }
    }
    view
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
    use super::parser::{heading, inline_text, ordered_item, parse_help, parse_inline};
    use super::{HelpBlock, InlinePart};

    #[test]
    fn parser_extracts_headings_lists_and_code_without_a_webview() {
        let blocks = parse_help("# Title\n\n## Section\n\n- **item**\n1. step\n\n```\ncode\n```");
        assert!(blocks.contains(&HelpBlock::Heading {
            level: 2,
            text: "Section".into()
        }));
        assert!(blocks.contains(&HelpBlock::Bullet("**item**".into())));
        assert!(blocks.contains(&HelpBlock::Ordered("1.  step".into())));
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
        assert_eq!(ordered_item("12. item"), Some("12.  item".into()));
    }
}
