use super::*;

impl HomeState {
    pub fn apply_stats_summary(&mut self, value: serde_json::Value) -> bool {
        let Ok(stats) = serde_json::from_value(value) else {
            return false;
        };
        self.stats = stats;
        true
    }

    pub fn apply_daily_stats(&mut self, value: serde_json::Value) -> bool {
        let Ok(stats) = serde_json::from_value(value) else {
            self.stats_details_error = Some("每日统计加载失败".to_owned());
            self.stats_details_loading = false;
            return false;
        };
        self.daily_stats = Some(stats);
        if self.stats_selected_date.is_none() {
            self.stats_selected_date = self
                .daily_stats
                .as_ref()
                .and_then(|value| value.days.first())
                .map(|day| day.date.clone());
        }
        self.stats_details_error = None;
        self.stats_details_loading = false;
        true
    }

    pub fn set_stats_details_open(&mut self, open: bool) {
        self.stats_details_open = open;
        if !open {
            self.stats_details_loading = false;
            self.stats_details_error = None;
        }
    }

    pub fn select_stats_date(&mut self, date: String) {
        self.stats_selected_date = Some(date);
    }
}
