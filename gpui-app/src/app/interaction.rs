use gpui::{Context, Window};

use super::{AhabApp, Page};
use crate::shell;

impl AhabApp {
    pub fn select_page(&mut self, page: Page, window: &mut Window, cx: &mut Context<Self>) {
        self.home.close_select();
        self.teams.close_select();
        self.settings_page.close_select();
        // The editor is a `Root` dialog now, so leaving it out of the state
        // alone would leave it on screen over the next page. Guarded because
        // `close_dialog` pops whatever `Root` has on top.
        if self.home.after_completion_open {
            self.close_after_completion(window, cx);
        }
        self.current_page = page;
        self.reconcile_preview(Some(window), cx);
        cx.notify();
    }

    pub fn show_toast(
        &mut self,
        kind: shell::ToastKind,
        message: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.toast_generation = self.toast_generation.wrapping_add(1);
        let generation = self.toast_generation;
        self.toast = Some(shell::Toast {
            id: generation,
            kind,
            message: message.into(),
        });
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(2400))
                .await;
            let _ = this.update(cx, |view, cx| {
                if view
                    .toast
                    .as_ref()
                    .is_some_and(|toast| toast.id == generation)
                {
                    view.toast = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }
}
