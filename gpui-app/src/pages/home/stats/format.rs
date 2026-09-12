use super::*;

pub(crate) fn live_elapsed_secs(
    current: &CurrentRunStats,
    fallback_updated_at: i64,
    state: ExecutionState,
) -> f64 {
    let Some(started) = current.startedAt else {
        return 0.0;
    };
    let elapsed_ms = if state == ExecutionState::Running && current.runId.is_some() {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(fallback_updated_at);
        (now_ms - started).max(0)
    } else if let Some(updated) = current.updatedAt {
        (updated - started).max(0)
    } else {
        (fallback_updated_at - started).max(0)
    };
    elapsed_ms as f64 / 1000.0
}

pub(crate) fn format_duration(seconds: f64) -> String {
    let total_seconds = if seconds.is_finite() {
        seconds.max(0.0).floor() as u64
    } else {
        0
    };
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes:02}:{seconds:02}")
}

#[allow(dead_code)]
pub(crate) fn _current_run_for_tests(current: &CurrentRunStats) -> &CurrentRunStats {
    current
}
