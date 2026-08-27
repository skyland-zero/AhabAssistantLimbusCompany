//! Native toolbox page backed by the shared tool IPC events.
//!
//! The card grid follows `ui/src/pages/ToolboxPage.tsx`: one column on narrow
//! windows, two columns from the `md` breakpoint, and three on wide windows.
//! Tool actions continue to use `ToolboxState`, so this page stays on the
//! canonical Mock IPC boundary.

use gpui::{Context, Div, Svg, container_query, div, prelude::*, px, svg};

use crate::{
    app::{ACCENT, AhabApp, BACKGROUND, TEXT, TEXT_MUTED},
    components::style::{DANGER, GREEN},
    components::{
        BadgeTone, ButtonVariant, badge, button, card, empty_state, render_rgb as rgb,
        scroll_area_with_id,
    },
    model::{Language, ToolId},
};

const ICON_CROSSHAIR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="22" x2="18" y1="12" y2="12"/><line x1="6" x2="2" y1="12" y2="12"/><line x1="12" x2="12" y1="6" y2="2"/><line x1="12" x2="12" y1="22" y2="18"/></svg>"#;
const ICON_PILL: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="m10.5 20.5 9-9a4.95 4.95 0 0 0-7-7l-9 9a4.95 4.95 0 0 0 7 7Z"/><path d="m8.5 8.5 7 7"/><path d="M7 13h.01"/><path d="M11 17h.01"/></svg>"#;
const ICON_CAMERA: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3Z"/><circle cx="12" cy="13" r="3"/></svg>"#;
const ICON_PLAY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><polygon points="6 3 20 12 6 21 6 3"/></svg>"#;
const ICON_STOP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><rect width="14" height="14" x="5" y="5" rx="2"/></svg>"#;

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

#[derive(Clone, Copy)]
enum ToolIcon {
    Crosshair,
    Pill,
    Camera,
}

#[derive(Clone, Copy)]
struct ToolMeta {
    id: ToolId,
    icon: ToolIcon,
    title: Localized,
    description: Localized,
}

const TOOLS: [ToolMeta; 3] = [
    ToolMeta {
        id: ToolId::InfiniteBattle,
        icon: ToolIcon::Crosshair,
        title: text("自动战斗", "Auto Battle"),
        description: text(
            "循环执行战斗直至手动停止",
            "Loop battles until stopped manually",
        ),
    },
    ToolMeta {
        id: ToolId::Enkephalin,
        icon: ToolIcon::Pill,
        title: text("体力换饼", "Enkephalin Module"),
        description: text(
            "自动将狂气转换为体力并合成脑啡肽模块，防止体力溢出",
            "Convert Lunacy to Enkephalin modules automatically to prevent overflow",
        ),
    },
    ToolMeta {
        id: ToolId::Screenshot,
        icon: ToolIcon::Camera,
        title: text("辅助截图", "Screenshot Tool"),
        description: text(
            "截取当前游戏窗口画面并保存到 AALC 目录",
            "Capture the game window and save it to the AALC folder",
        ),
    },
];

pub fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let feedback = app.toolbox.feedback.clone();
    let cards: Vec<Div> = TOOLS
        .into_iter()
        .map(|tool| tool_card(app, cx, tool, language))
        .collect();
    let has_tools = !cards.is_empty();

    let grid = container_query(move |size, _, _| {
        let columns: u16 = if size.width >= px(1024.) {
            3
        } else if size.width >= px(768.) {
            2
        } else {
            1
        };
        div()
            .w_full()
            .grid()
            .grid_cols(columns)
            .gap(px(16.))
            .children(cards)
    });

    let body = if has_tools {
        div().w_full().child(grid)
    } else {
        div().w_full().child(empty_state(
            text("暂无工具", "No tools available").get(language),
            text("工具资源尚未加载。", "Tool resources are not loaded yet.").get(language),
        ))
    };

    let mut content = div().w_full().p_6().child(body).child(
        div()
            .mt_4()
            .text_size(px(12.))
            .text_color(rgb(TEXT_MUTED))
            .child(
                text(
                    "工具均为 Mock 模拟，后端接入后生效",
                    "Tools are mocked until the backend lands",
                )
                .get(language),
            ),
    );
    if let Some(feedback) = feedback {
        content = content.child(
            div()
                .mt_2()
                .text_size(px(12.))
                .text_color(rgb(GREEN))
                .child(feedback),
        );
    }

    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(rgb(BACKGROUND))
        .child(
            scroll_area_with_id("toolbox-scroll", content)
                .flex_1()
                .min_h_0(),
        )
}

fn tool_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    tool: ToolMeta,
    language: Language,
) -> Div {
    let running = app.toolbox.is_running(tool.id);
    let is_screenshot = tool.id == ToolId::Screenshot;

    let mut action = if is_screenshot {
        action_button(
            text("运行", "Run").get(language),
            ButtonVariant::Outline,
            icon(ICON_CAMERA, 16., TEXT),
        )
    } else if running {
        action_button(
            text("停止", "Stop").get(language),
            ButtonVariant::Outline,
            icon(ICON_STOP, 16., TEXT),
        )
        .text_color(rgb(DANGER))
    } else {
        action_button(
            text("运行", "Run").get(language),
            ButtonVariant::Default,
            icon(ICON_PLAY, 16., TEXT),
        )
    }
    .id(format!("tool-action-{:?}", tool.id))
    .w_full();
    if is_screenshot {
        action = action.on_click(cx.listener(|view, _, _, cx| {
            view.toolbox.screenshot();
            cx.notify();
        }));
    } else {
        let tool_id = tool.id;
        action = action.on_click(cx.listener(move |view, _, _, cx| {
            view.toolbox.toggle(tool_id);
            cx.notify();
        }));
    }

    let status = if running {
        let mut running_badge = badge("", BadgeTone::Success);
        running_badge = running_badge
            .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(rgb(GREEN)))
            .child(text("运行中", "Running").get(language));
        running_badge
    } else {
        badge(
            if is_screenshot {
                "—"
            } else {
                text("待机", "Idle").get(language)
            },
            BadgeTone::Neutral,
        )
    };

    let body = div()
        .flex()
        .flex_col()
        .items_start()
        .gap_3()
        .px_4()
        .py_4()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .w_full()
                .child(icon(tool_icon_path(tool.icon), 20., ACCENT))
                .child(status),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(14.))
                        .text_color(rgb(TEXT))
                        .child(tool.title.get(language)),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_MUTED))
                        .child(tool.description.get(language)),
                ),
        )
        .child(action);

    card(body).p_0().w_full()
}

fn action_button(label: &'static str, variant: ButtonVariant, icon: Svg) -> Div {
    button("", variant)
        .h(px(32.))
        .px_3()
        .py_0()
        .text_size(px(12.))
        .child(icon)
        .child(label)
}

fn icon(data: &'static str, size: f32, color: u32) -> Svg {
    svg()
        .data(data.as_bytes())
        .size(px(size))
        .text_color(rgb(color))
}

fn tool_icon_path(icon: ToolIcon) -> &'static str {
    match icon {
        ToolIcon::Crosshair => ICON_CROSSHAIR,
        ToolIcon::Pill => ICON_PILL,
        ToolIcon::Camera => ICON_CAMERA,
    }
}

#[cfg(test)]
mod tests {
    use crate::{model::ToolId, state::ToolboxState};

    #[test]
    fn every_tool_has_a_mock_state_boundary() {
        let mut state = ToolboxState::default();
        state.toggle(ToolId::InfiniteBattle);
        assert!(state.is_running(ToolId::InfiniteBattle));
        state.screenshot();
        assert!(state.feedback.as_deref().unwrap().contains("截图完成"));
    }
}
