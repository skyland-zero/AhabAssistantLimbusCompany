//! Native toolbox page backed by the shared tool IPC events.
//!
//! The card grid uses one column on narrow windows, two columns from the `md`
//! breakpoint, and three on wide windows.
//! Tool actions continue to use `ToolboxState`, so this page stays on the
//! canonical sidecar IPC boundary.

use gpui::{Context, Div, Svg, div, prelude::*, px, svg};

use crate::{
    app::{ACCENT, AhabApp, TEXT, TEXT_MUTED},
    components::style::{DANGER, GREEN, current_render_palette},
    components::{
        BadgeTone, ButtonVariant, action_button, badge, card, empty_state, is_activation_key,
        page_root, palette_rgb, render_rgb as rgb, scroll_area_with_id, svg_icon,
    },
    i18n::{Localized, paired as text},
    model::{Language, ToolId},
};

const ICON_CROSSHAIR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="22" x2="18" y1="12" y2="12"/><line x1="6" x2="2" y1="12" y2="12"/><line x1="12" x2="12" y1="6" y2="2"/><line x1="12" x2="12" y1="22" y2="18"/></svg>"#;
const ICON_PILL: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="m10.5 20.5 9-9a4.95 4.95 0 0 0-7-7l-9 9a4.95 4.95 0 0 0 7 7Z"/><path d="m8.5 8.5 7 7"/><path d="M7 13h.01"/><path d="M11 17h.01"/></svg>"#;
const ICON_CAMERA: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M14.5 4h-5L7 7H4a2 2 0 0 0-2 2v9a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-3l-2.5-3Z"/><circle cx="12" cy="13" r="3"/></svg>"#;
const ICON_MONITOR: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><rect width="20" height="14" x="2" y="3" rx="2"/><line x1="8" x2="16" y1="21" y2="21"/><line x1="12" x2="12" y1="17" y2="21"/></svg>"#;
const ICON_ROTATE_CCW: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/></svg>"#;
const ICON_PLAY: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><polygon points="6 3 20 12 6 21 6 3"/></svg>"#;
const ICON_STOP: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><rect width="14" height="14" x="5" y="5" rx="2"/></svg>"#;

#[derive(Clone, Copy)]
enum ToolIcon {
    Crosshair,
    Pill,
    Camera,
    Monitor,
}

#[derive(Clone, Copy)]
struct ToolMeta {
    id: ToolId,
    icon: ToolIcon,
    title: Localized,
    description: Localized,
}

const TOOLS: [ToolMeta; 4] = [
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
    ToolMeta {
        id: ToolId::Resolution,
        icon: ToolIcon::Monitor,
        title: text("分辨率修改", "Device Resolution"),
        description: text(
            "进入游戏后点击，通过 ADB 将 Android 设备修改为 1080P 横屏 (1920x1080 240DPI)，并支持一键还原",
            "Click after entering the game to change Android device to 1080P landscape (1920x1080 240DPI) via ADB with one-click restore",
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

    // The minimum native window is 800px, where the React md breakpoint
    // produces two columns. Keeping this explicit also avoids a container
    // query treating the padded 752px content width as below md.
    let grid = div()
        .w_full()
        .grid()
        .grid_cols(2)
        .gap(px(16.))
        .children(cards);

    let body = if has_tools {
        div().w_full().child(grid)
    } else {
        div().w_full().child(empty_state(
            text("暂无工具", "No tools available").get(language),
            text("工具资源尚未加载。", "Tool resources are not loaded yet.").get(language),
        ))
    };

    let mut content = div().w_full().flex().flex_col().gap_3().child(body).child(
        card(
            div().text_size(px(12.)).text_color(rgb(TEXT_MUTED)).child(
                text(
                    "工具请求通过 Python sidecar 执行",
                    "Tool requests are executed by the Python sidecar",
                )
                .get(language),
            ),
        )
        .w_full()
        .p_3(),
    );
    if let Some(feedback) = feedback {
        content = content.child(
            card(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(GREEN))
                    .child(localized_feedback(&feedback, language)),
            )
            .w_full()
            .p_3(),
        );
    }

    page_root().child(
        scroll_area_with_id(app, "toolbox-scroll", content)
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
    let is_resolution = tool.id == ToolId::Resolution;

    let action_area = if is_resolution {
        let set_btn = action_button(
            text("修改 1080P", "Set 1080P").get(language),
            ButtonVariant::Default,
            Some(brand_icon(ICON_MONITOR, 14.)),
            32.,
        )
        .id("tool-action-resolution-set")
        .flex_1()
        .on_click(cx.listener(|view, _, _, cx| {
            view.toolbox.apply_resolution();
            cx.notify();
        }))
        .on_key_down(cx.listener(|view, event: &gpui::KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                view.toolbox.apply_resolution();
                cx.notify();
            }
        }));

        let reset_btn = action_button(
            text("还原默认", "Restore").get(language),
            ButtonVariant::Outline,
            Some(svg_icon(ICON_ROTATE_CCW, 14., TEXT)),
            32.,
        )
        .id("tool-action-resolution-reset")
        .flex_1()
        .on_click(cx.listener(|view, _, _, cx| {
            view.toolbox.reset_resolution();
            cx.notify();
        }))
        .on_key_down(cx.listener(|view, event: &gpui::KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                view.toolbox.reset_resolution();
                cx.notify();
            }
        }));

        div().w_full().flex().gap_2().child(set_btn).child(reset_btn)
    } else {
        let mut action = if is_screenshot {
            action_button(
                text("运行", "Run").get(language),
                ButtonVariant::Outline,
                Some(svg_icon(ICON_CAMERA, 16., TEXT)),
                32.,
            )
        } else if running {
            action_button(
                text("停止", "Stop").get(language),
                ButtonVariant::Outline,
                Some(svg_icon(ICON_STOP, 16., TEXT)),
                32.,
            )
            .text_color(rgb(DANGER))
        } else {
            action_button(
                text("运行", "Run").get(language),
                ButtonVariant::Default,
                Some(brand_icon(ICON_PLAY, 16.)),
                32.,
            )
        }
        .id(format!("tool-action-{:?}", tool.id))
        .w_full();

        if is_screenshot {
            action = action
                .on_click(cx.listener(|view, _, _, cx| {
                    view.toolbox.screenshot();
                    cx.notify();
                }))
                .on_key_down(cx.listener(|view, event: &gpui::KeyDownEvent, window, cx| {
                    if is_activation_key(event) {
                        window.prevent_default();
                        view.toolbox.screenshot();
                        cx.notify();
                    }
                }));
        } else {
            let tool_id = tool.id;
            let tool_id_for_key = tool_id;
            action = action
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.toolbox.toggle(tool_id);
                    cx.notify();
                }))
                .on_key_down(
                    cx.listener(move |view, event: &gpui::KeyDownEvent, window, cx| {
                        if is_activation_key(event) {
                            window.prevent_default();
                            view.toolbox.toggle(tool_id_for_key);
                            cx.notify();
                        }
                    }),
                );
        }
        div().w_full().child(action)
    };

    let status = if running {
        let mut running_badge = badge("", BadgeTone::Success);
        running_badge = running_badge
            .child(div().w(px(6.)).h(px(6.)).rounded_full().bg(rgb(GREEN)))
            .child(text("运行中", "Running").get(language));
        running_badge
    } else if is_resolution {
        badge("ADB", BadgeTone::Neutral)
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
                .child(svg_icon(tool_icon_path(tool.icon), 20., ACCENT))
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
        .child(action_area);

    card(body).p_0().w_full()
}

fn brand_icon(data: &'static str, size: f32) -> Svg {
    svg()
        .data(data.as_bytes())
        .size(px(size))
        .text_color(palette_rgb(current_render_palette().brand_foreground))
}

fn tool_icon_path(icon: ToolIcon) -> &'static str {
    match icon {
        ToolIcon::Crosshair => ICON_CROSSHAIR,
        ToolIcon::Pill => ICON_PILL,
        ToolIcon::Camera => ICON_CAMERA,
        ToolIcon::Monitor => ICON_MONITOR,
    }
}

fn localized_feedback(feedback: &str, language: Language) -> String {
    if matches!(language, Language::ZhCn) {
        return feedback.to_owned();
    }
    if let Some(path) = feedback.strip_prefix("截图完成：") {
        return format!("Screenshot saved: {path}");
    }
    if feedback == "已修改分辨率为 1080P (240 DPI)" {
        return "Resolution changed to 1080P (240 DPI)".to_owned();
    }
    if feedback == "已修改分辨率为 1080P (240 DPI)，Scrcpy 已重连" {
        return "Resolution changed to 1080P (240 DPI), Scrcpy reconnected".to_owned();
    }
    if feedback == "已还原设备分辨率与 DPI" {
        return "Device resolution and DPI restored".to_owned();
    }
    if feedback == "已还原设备分辨率与 DPI，Scrcpy 已重连" {
        return "Device resolution and DPI restored, Scrcpy reconnected".to_owned();
    }
    feedback.to_owned()
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
        state.apply_resolution();
        assert!(state.feedback.as_deref().unwrap().contains("1080P"));
        state.reset_resolution();
        assert!(state.feedback.as_deref().unwrap().contains("还原"));
    }
}
