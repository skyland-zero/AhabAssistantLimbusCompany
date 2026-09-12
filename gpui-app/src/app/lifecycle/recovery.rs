use super::*;

impl AhabApp {
    pub(crate) fn log_backend(&mut self, level: LogLevel, message: impl Into<String>) {
        self.home.append_local_log(level, message);
    }

    pub(crate) fn log_backend_localized(
        &mut self,
        level: LogLevel,
        zh: &'static str,
        en: &'static str,
    ) {
        let message = localized(self.state.settings.language, zh, en);
        self.log_backend(level, message);
    }

    pub(crate) fn maybe_recover_backend(&mut self, cx: &mut Context<Self>) -> bool {
        if self.exit_requested || !self.backend_operation.is_idle() || !self.home.rpc.is_sidecar() {
            return false;
        }

        if self.backend_status.is_ready() && !self.home.rpc.is_connected() {
            self.log_backend_localized(
                LogLevel::Warn,
                "Python 后端连接已断开，开始自动恢复",
                "Python backend connection lost; starting automatic recovery",
            );
            self.start_backend_connection(cx, BackendStartReason::Reconnect);
            return true;
        }

        false
    }
}
