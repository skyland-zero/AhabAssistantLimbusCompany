use gpui::{AppContext, Context, Entity};

use crate::{
    app::{AhabApp, HomeInvalidation},
    model::FixedTaskId,
};

use super::{
    cards::RunningSweepView,
    log_panel::LogPanelView,
    panel::PreviewView,
    stats::{StatsSnapshot, StatsView},
};

/// Stable child views mounted by the Home page. Keeping these entities alive
/// across root renders is what lets GPUI invalidate only the affected subtree.
#[derive(Clone)]
pub(crate) struct HomeViewRefs {
    stats: Entity<StatsView>,
    preview: Entity<PreviewView>,
    logs: Entity<LogPanelView>,
    running_sweeps: [Entity<RunningSweepView>; 6],
}

impl HomeViewRefs {
    pub(crate) fn new(cx: &mut Context<AhabApp>) -> Self {
        let root = cx.weak_entity();
        let stats = cx.new(|_| StatsView::new(root.clone()));
        let preview = cx.new(|_| PreviewView::new());
        let logs = cx.new(|_| LogPanelView::new(root.clone()));
        let running_sweeps = [
            FixedTaskId::SetWindows,
            FixedTaskId::DailyTask,
            FixedTaskId::GetReward,
            FixedTaskId::BuyEnkephalin,
            FixedTaskId::Mirror,
            FixedTaskId::ResonateWithAhab,
        ]
        .map(|task| cx.new(|_| RunningSweepView::new(task)));

        Self {
            stats,
            preview,
            logs,
            running_sweeps,
        }
    }

    pub(crate) fn sync_from_app(&self, app: &AhabApp, cx: &mut Context<AhabApp>) {
        let stats = StatsSnapshot::from_app(app);
        let logs = app.home.logs.clone();
        let log_revision = app.home.log_revision;
        let latest_screenshot = app.home.latest_screenshot.clone();
        let screenshot_revision = app.home.screenshot_revision;
        let preview_status = app.home.preview_status;
        let preview_error = app.home.preview_error.clone();
        let language = app.state.settings.language;

        self.stats.update(cx, |view, _| view.sync_snapshot(stats));
        self.logs.update(cx, |view, _| {
            view.sync_snapshot(logs, log_revision, language)
        });
        self.preview.update(cx, |view, _| {
            view.sync_snapshot(
                latest_screenshot,
                screenshot_revision,
                preview_status,
                preview_error,
                language,
            )
        });
    }

    pub(crate) fn apply_invalidation(
        &self,
        app: &AhabApp,
        cx: &mut Context<AhabApp>,
        invalidation: HomeInvalidation,
    ) {
        let stats = invalidation.stats.then(|| StatsSnapshot::from_app(app));
        let logs = invalidation
            .logs
            .then(|| (app.home.logs.clone(), app.home.log_revision));
        let preview = invalidation.preview.then(|| {
            (
                app.home.latest_screenshot.clone(),
                app.home.screenshot_revision,
                app.home.preview_status,
                app.home.preview_error.clone(),
                app.state.settings.language,
            )
        });

        if let Some(snapshot) = stats {
            self.stats.update(cx, |view, cx| {
                view.sync_snapshot(snapshot);
                cx.notify();
            });
        }
        if let Some((logs, revision)) = logs {
            let language = app.state.settings.language;
            self.logs.update(cx, |view, cx| {
                view.sync_snapshot(logs, revision, language);
                cx.notify();
            });
        }
        if let Some((frame, revision, status, error, language)) = preview {
            self.preview.update(cx, |view, cx| {
                view.sync_snapshot(frame, revision, status, error, language);
                cx.notify();
            });
        }
    }

    pub(super) fn stats_view(&self) -> Entity<StatsView> {
        self.stats.clone()
    }

    pub(super) fn preview_view(&self) -> Entity<PreviewView> {
        self.preview.clone()
    }

    pub(crate) fn clear_preview_render_resources(
        &self,
        window: &mut gpui::Window,
        cx: &mut Context<AhabApp>,
    ) {
        self.preview.update(cx, |view, cx| {
            view.clear_render_resources(window);
            cx.notify();
        });
    }

    pub(super) fn logs_view(&self) -> Entity<LogPanelView> {
        self.logs.clone()
    }

    pub(super) fn running_sweep(&self, task: FixedTaskId) -> Entity<RunningSweepView> {
        let index = match task {
            FixedTaskId::SetWindows => 0,
            FixedTaskId::DailyTask => 1,
            FixedTaskId::GetReward => 2,
            FixedTaskId::BuyEnkephalin => 3,
            FixedTaskId::Mirror => 4,
            FixedTaskId::ResonateWithAhab => 5,
        };
        self.running_sweeps[index].clone()
    }
}
