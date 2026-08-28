use super::*;

pub(super) fn execution_toolbar(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    state: ExecutionState,
) -> Div {
    super::execution_toolbar::execution_toolbar(app, cx, busy, state)
}

pub(super) fn after_completion_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
) -> gpui::AnyElement {
    super::completion_editor::after_completion_editor(app, cx, busy)
}

pub(super) fn after_completion_summary(
    config: &crate::model::AfterCompletionConfig,
    language: Language,
) -> String {
    super::completion::after_completion_summary(config, language)
}
