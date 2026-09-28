use gpui::{AppContext, Context, ParentElement as _, Window};
use gpui_component::Root;

use super::AhabApp;

impl AhabApp {
    /// Opens the after-completion editor as a `Root` dialog.
    ///
    /// It replaces an inline overlay that the page drew itself, so `Root` now
    /// owns the centering, the close button, the focus trap and Esc.
    ///
    /// The body is `AfterCompletionView` rather than a plain builder: the
    /// builder below runs *inside* `AhabApp::render` and therefore cannot read
    /// the app, while this editor has to show the draft changing as the
    /// switches are toggled. A child view is rendered after that borrow ends.
    pub fn open_after_completion(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.home.set_after_completion_open(true);
        let language = self.state.settings.language;
        let app = cx.entity();
        let view = cx.new(|cx| crate::pages::AfterCompletionView::new(app.clone(), cx));
        let close_app = app.downgrade();
        Root::update(window, cx, move |root, window, cx| {
            root.open_dialog(
                // `Fn`, not `FnOnce`: the builder runs again on every render of
                // the dialog layer, so captures are cloned per call.
                move |dialog, _window, _cx| {
                    let close_app = close_app.clone();
                    dialog
                        .title(
                            crate::i18n::paired("结束后操作", "After Completion Actions")
                                .get(language),
                        )
                        .w(gpui::px(512.))
                        // Closing through `Root` - the X, Esc or the overlay -
                        // still has to drop the draft, which is what the state
                        // setter does. The dialog is already going away here, so
                        // this must not ask `Root` to close it again.
                        .on_close(move |_, _, cx| {
                            let _ = close_app.update(cx, |view, cx| {
                                view.dismiss_after_completion(cx);
                            });
                        })
                        .content({
                            let view = view.clone();
                            move |content, _window, _cx| content.child(view.clone())
                        })
                },
                window,
                cx,
            )
        });
        cx.notify();
    }

    /// Clears the flag without touching the dialog layer.
    ///
    /// This is the `on_close` path, where `Root` is already tearing the dialog
    /// down; asking it to close again would pop whatever dialog is beneath.
    pub fn dismiss_after_completion(&mut self, cx: &mut Context<Self>) {
        self.home.set_after_completion_open(false);
        cx.notify();
    }

    /// Closes the editor from outside itself - a page switch, or applying the
    /// draft.
    pub fn close_after_completion(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss_after_completion(cx);
        Root::update(window, cx, |root, window, cx| root.close_dialog(window, cx));
    }

    /// Commits the draft (as the default config when `keep`, otherwise for this
    /// run only) and closes the editor.
    pub fn apply_after_completion(
        &mut self,
        keep: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.home.apply_after_completion(keep);
        self.close_after_completion(window, cx);
    }
}
