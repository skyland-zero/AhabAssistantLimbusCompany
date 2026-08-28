use super::*;

pub(super) fn after_completion_summary(
    config: &crate::model::AfterCompletionConfig,
    language: Language,
) -> String {
    let power = match config.powerAction {
        crate::model::AfterPowerAction::None => None,
        action => Some(after_power_label(action, language)),
    };
    let mode = if config.keepAfterCompletion {
        text("默认", "Default").get(language)
    } else {
        text("本次", "This run").get(language)
    };

    let summary = if config.actions.is_empty() && power.is_none() {
        text("什么也不干", "Do nothing").get(language).to_owned()
    } else if config.actions.is_empty() {
        power.unwrap_or_default().to_owned()
    } else {
        let exits = config
            .actions
            .iter()
            .map(|action| after_exit_short_label(*action, language))
            .collect::<Vec<_>>()
            .join(if matches!(language, Language::ZhCn) {
                "与"
            } else {
                ", "
            });
        let prefix = text("退出", "Exit ").get(language);
        match power {
            Some(power) => format!(
                "{prefix}{exits}{}{}",
                text("后", " then ").get(language),
                power
            ),
            None => format!("{prefix}{exits}"),
        }
    };

    format!("{summary} ({mode})")
}

pub(super) fn after_exit_label(action: AfterExitAction, language: Language) -> &'static str {
    match action {
        AfterExitAction::ExitGame => text("退出游戏", "Exit Game").get(language),
        AfterExitAction::ExitEmulator => text("退出模拟器", "Exit Emulator").get(language),
        AfterExitAction::ExitAalc => text("退出 AALC", "Exit AALC").get(language),
    }
}

pub(super) fn after_exit_short_label(action: AfterExitAction, language: Language) -> &'static str {
    match action {
        AfterExitAction::ExitGame => text("游戏", "Game").get(language),
        AfterExitAction::ExitEmulator => text("模拟器", "Emulator").get(language),
        AfterExitAction::ExitAalc => text("AALC", "AALC").get(language),
    }
}

pub(super) fn after_power_label(action: AfterPowerAction, language: Language) -> &'static str {
    match action {
        AfterPowerAction::None => text("无动作", "Do Nothing").get(language),
        AfterPowerAction::Sleep => text("睡眠", "Sleep").get(language),
        AfterPowerAction::Hibernate => text("休眠", "Hibernate").get(language),
        AfterPowerAction::Lock => text("锁屏", "Lock Screen").get(language),
        AfterPowerAction::Shutdown => text("关机", "Shut Down").get(language),
    }
}

pub(super) fn after_power_key(action: AfterPowerAction) -> &'static str {
    match action {
        AfterPowerAction::None => "none",
        AfterPowerAction::Sleep => "sleep",
        AfterPowerAction::Hibernate => "hibernate",
        AfterPowerAction::Lock => "lock",
        AfterPowerAction::Shutdown => "shutdown",
    }
}

pub(super) fn parse_after_power_action(value: &str) -> Option<AfterPowerAction> {
    Some(match value {
        "none" => AfterPowerAction::None,
        "sleep" => AfterPowerAction::Sleep,
        "hibernate" => AfterPowerAction::Hibernate,
        "lock" => AfterPowerAction::Lock,
        "shutdown" => AfterPowerAction::Shutdown,
        _ => return None,
    })
}
