use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeCardView {
    Backend,
    CurrentRun,
}

impl RuntimeCardView {
    fn animation_key(self) -> &'static str {
        match self {
            Self::Backend => "backend",
            Self::CurrentRun => "current-run",
        }
    }
}

pub(crate) fn runtime_card_view(phase: BackendPhase) -> RuntimeCardView {
    if phase == BackendPhase::Ready {
        RuntimeCardView::CurrentRun
    } else {
        RuntimeCardView::Backend
    }
}

pub(crate) fn runtime_card(
    snapshot: &StatsSnapshot,
    root: &WeakEntity<AhabApp>,
) -> impl IntoElement {
    let view = runtime_card_view(snapshot.backend_status.phase);
    let card = match view {
        RuntimeCardView::Backend => backend_status_card(snapshot, root),
        RuntimeCardView::CurrentRun => current_run_card(snapshot),
    };

    card.flex_grow(1.0)
        .flex_shrink(1.0)
        .flex_basis(relative(0.0))
        .id("runtime-status-card")
        .with_animation(
            format!("runtime-card-{}", view.animation_key()),
            Animation::new(Duration::from_millis(150)).with_easing(gpui::ease_out_quint()),
            |card, progress| card.opacity(progress),
        )
}
