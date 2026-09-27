//! Native toolbox page backed by the shared tool IPC events.
//!
//! The card grid uses one column on narrow windows, two columns from the `md`
//! breakpoint, and three on wide windows.
//! Tool actions continue to use `ToolboxState`, so this page stays on the
//! canonical sidecar IPC boundary.

use crate::components::{Icon, IconName};
use gpui::{Context, Div, Window, div, prelude::*, px};

use crate::{
    app::{ACCENT, AhabApp, TEXT, TEXT_MUTED},
    components::style::{DANGER, GREEN, current_render_palette},
    components::{
        BadgeTone, ButtonVariant, action_button, badge, card, is_activation_key, page_root,
        palette_rgb, render_rgb as rgb, scroll_area_with_id, svg_icon,
    },
    i18n::{Localized, paired as text},
    model::{Language, ToolId},
};

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

pub fn render(app: &mut AhabApp, _window: &mut Window, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let feedback = app.toolbox.feedback.clone();
    let cards: Vec<Div> = TOOLS
        .into_iter()
        .map(|tool| tool_card(app, cx, tool, language))
        .collect();

    // The minimum native window is 800px, where the React md breakpoint
    // produces two columns. Keeping this explicit also avoids a container
    // query treating the padded 752px content width as below md.
    let grid = div()
        .w_full()
        .grid()
        .grid_cols(2)
        .gap(px(16.))
        .children(cards);

    let mut content = div().w_full().flex().flex_col().gap_3().child(grid).child(
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
    let is_resolution = tool.id == ToolId::Resolution;

    let action_area = if is_resolution {
        let set_btn = action_button(
            "tool-action-resolution-set",
            text("修改 1080P", "Set 1080P").get(language),
            ButtonVariant::Default,
            Some(brand_icon(IconName::Monitor, 14.)),
            32.,
        )
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
            "tool-action-resolution-reset",
            text("还原默认", "Restore").get(language),
            ButtonVariant::Outline,
            Some(svg_icon(IconName::RotateCcw, 14., TEXT)),
            32.,
        )
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

        div()
            .w_full()
            .flex()
            .gap_2()
            .child(set_btn)
            .child(reset_btn)
    } else {
        let action_id = format!("tool-action-{:?}", tool.id);
        let mut action = if is_screenshot {
            action_button(
                action_id.clone(),
                text("运行", "Run").get(language),
                ButtonVariant::Outline,
                Some(svg_icon(IconName::Camera, 16., TEXT)),
                32.,
            )
        } else if running {
            action_button(
                action_id.clone(),
                text("停止", "Stop").get(language),
                ButtonVariant::Outline,
                Some(svg_icon(IconName::Square, 16., TEXT)),
                32.,
            )
            .text_color(rgb(DANGER))
        } else {
            action_button(
                action_id,
                text("运行", "Run").get(language),
                ButtonVariant::Default,
                Some(brand_icon(IconName::Play, 16.)),
                32.,
            )
        }
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
                .child(svg_icon(tool_icon(tool.icon), 20., ACCENT))
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

fn brand_icon(name: impl Into<Icon>, size: f32) -> Icon {
    crate::components::svg_icon_colored(
        name,
        size,
        palette_rgb(current_render_palette().brand_foreground),
    )
}

fn tool_icon(icon: ToolIcon) -> IconName {
    match icon {
        ToolIcon::Crosshair => IconName::Crosshair,
        ToolIcon::Pill => IconName::Pill,
        ToolIcon::Camera => IconName::Camera,
        ToolIcon::Monitor => IconName::Monitor,
    }
}

fn localized_feedback(feedback: &str, language: Language) -> String {
    crate::i18n::feedback(feedback, language)
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
