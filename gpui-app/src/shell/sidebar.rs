use gpui::{Context, Div, div, prelude::*, px, rgb};

use crate::app::{ACCENT, AhabApp, BORDER, Page, SURFACE, SURFACE_HOVER, TEXT, TEXT_MUTED};

pub fn sidebar(active_page: Page, cx: &mut Context<AhabApp>) -> Div {
    let mut sidebar = div()
        .w(px(190.))
        .flex_none()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .bg(rgb(SURFACE))
        .border_r_1()
        .border_color(rgb(BORDER));

    sidebar = sidebar.child(
        div()
            .flex()
            .flex_col()
            .gap_1()
            .px_2()
            .py_3()
            .child(div().text_size(px(18.)).text_color(rgb(TEXT)).child("AHAB"))
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(ACCENT))
                    .child("ASSISTANT · GPUI"),
            ),
    );
    sidebar = sidebar.child(div().h(px(1.)).bg(rgb(BORDER)));

    for page in Page::ALL {
        sidebar = sidebar.child(nav_item(page, page == active_page, cx));
    }

    sidebar.child(div().flex_1()).child(
        div()
            .px_2()
            .pb_2()
            .text_size(px(10.))
            .text_color(rgb(TEXT_MUTED))
            .child("GPUI GitHub main"),
    )
}

fn nav_item(page: Page, active: bool, cx: &mut Context<AhabApp>) -> impl IntoElement {
    let mut item = div()
        .id(format!("nav-{:?}", page))
        .flex()
        .items_center()
        .gap_3()
        .w_full()
        .px_3()
        .py_2()
        .rounded_md()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(SURFACE_HOVER)))
        .on_click(cx.listener(move |view, _, _, cx| view.select_page(page, cx)));

    if active {
        item = item.bg(rgb(0x354052)).text_color(rgb(TEXT));
    } else {
        item = item.text_color(rgb(TEXT_MUTED));
    }

    item.child(div().w(px(4.)).h(px(18.)).rounded_md().bg(if active {
        rgb(ACCENT)
    } else {
        rgb(BORDER)
    }))
    .child(
        div().flex().flex_col().gap_1().child(page.label()).child(
            div()
                .text_size(px(9.))
                .text_color(rgb(TEXT_MUTED))
                .child(page.name()),
        ),
    )
}
