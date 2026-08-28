use super::*;

impl HomeState {
    pub fn selected_task_count(&self) -> usize {
        let enabled = &self.tasks.enabledTasks;
        [
            enabled.daily_task,
            enabled.get_reward,
            enabled.buy_enkephalin,
            enabled.mirror,
        ]
        .into_iter()
        .filter(|selected| *selected)
        .count()
    }

    pub fn is_expanded(&self, task: FixedTaskId) -> bool {
        self.expanded_tasks.contains(&task)
    }

    pub fn options_tab(&self, task: FixedTaskId) -> TaskOptionsTab {
        self.task_options_tabs
            .get(&task)
            .copied()
            .unwrap_or_default()
    }

    pub fn set_options_tab(&mut self, task: FixedTaskId, tab: TaskOptionsTab) {
        self.task_options_tabs.insert(task, tab);
    }

    pub fn toggle_expanded(&mut self, task: FixedTaskId) {
        if !self.expanded_tasks.insert(task) {
            self.expanded_tasks.remove(&task);
        }
    }

    pub fn toggle_select(&mut self, select: HomeSelect) {
        self.open_select = if self.open_select == Some(select) {
            None
        } else {
            Some(select)
        };
    }

    pub fn close_select(&mut self) {
        self.open_select = None;
    }

    pub fn is_select_open(&self, select: HomeSelect) -> bool {
        self.open_select == Some(select)
    }
}
