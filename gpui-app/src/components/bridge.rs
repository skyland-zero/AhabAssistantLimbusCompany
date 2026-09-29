//! Bridge from a child view back to the app entity.

use gpui::{App, Context, WeakEntity, Window};

use crate::app::AhabApp;

/// `cx.listener`, for a view that only holds a handle on the app.
///
/// A `Root` dialog or sheet body is a child view: GPUI lays it out after
/// `AhabApp::render` released its borrow, so it has no `Context<AhabApp>` to
/// build a listener from. The handler body is written exactly as it would be for
/// `cx.listener` - the only difference is where the `Context` comes from - which
/// is what makes one control serve both entry points.
///
/// The two traps are handled here so callers do not have to know about them:
/// the reborrowed `window` keeps the returned closure `Fn`, and the handler is
/// called through a shared reference so nothing is moved out of it.
pub fn app_listener<E, F>(
    root: &WeakEntity<AhabApp>,
    handler: F,
) -> impl Fn(&E, &mut Window, &mut App) + 'static
where
    E: 'static,
    F: Fn(&mut AhabApp, &E, &mut Window, &mut Context<AhabApp>) + 'static,
{
    let host = root.clone();
    move |event, window, cx| {
        if let Some(root) = host.upgrade() {
            let window = &mut *window;
            root.update(cx, |view, cx| handler(view, event, window, cx));
        }
    }
}
