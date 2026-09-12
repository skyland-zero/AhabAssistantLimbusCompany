use std::time::Duration;

use gpui::Context;

use super::{AhabApp, BackendOperation, BackendPhase, BackendStatus, Page, VisualState};
use crate::{
    app_inputs::{SettingsInputs, TeamInputs},
    ipc::{BackendAttach, BackendClient, RpcGateway, contract::method},
    model::{Language, LogLevel},
    state::{
        AppState, HomeState, ResourcesState, SettingsPageState, TeamsState, ThemePacksState,
        ToolboxState,
    },
};

mod bootstrap;
mod construct;
mod recovery;

const MAX_AUTO_RETRIES: u8 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BackendStartReason {
    Initial,
    ManualRetry,
    Reconnect,
}

fn retry_delay(retry_no: u8) -> Duration {
    Duration::from_secs(match retry_no {
        1 => 1,
        2 => 2,
        _ => 4,
    })
}

fn localized(language: Language, zh: &'static str, en: &'static str) -> String {
    match language {
        Language::ZhCn => zh.to_owned(),
        Language::EnUs => en.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_retry_delays_use_exponential_backoff() {
        assert_eq!(retry_delay(1), Duration::from_secs(1));
        assert_eq!(retry_delay(2), Duration::from_secs(2));
        assert_eq!(retry_delay(3), Duration::from_secs(4));
        assert_eq!(retry_delay(0), Duration::from_secs(4));
    }

    #[test]
    fn retry_budget_allows_three_retries_after_the_initial_attempt() {
        assert_eq!(MAX_AUTO_RETRIES, 3);
        assert_eq!(usize::from(MAX_AUTO_RETRIES) + 1, 4);
    }
}
