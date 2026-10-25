use super::*;

impl Default for ResourcesState {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourcesState {
    pub fn new() -> Self {
        Self::with_client(MockClient::default())
    }

    pub fn with_client(client: impl Into<crate::ipc::BackendClient>) -> Self {
        #[allow(unused_mut)]
        let mut state = Self {
            rpc: RpcGateway::new(client),
            groups: Vec::new(),
            sync_progress: None,
            sync_finish_scheduled: false,
            feedback: None,
        };
        #[cfg(test)]
        {
            state.groups = state
                .rpc
                .request(method::RESOURCE_STATUS, None)
                .unwrap_or_default();
        }
        state
    }

    #[cfg(test)]
    pub(crate) fn pump_mock(&mut self) {
        for _ in 0..8 {
            let events = self.rpc.take_events();
            let completions = self.rpc.take_completions();
            if events.is_empty() && completions.is_empty() {
                break;
            }
            self.apply_events(events);
            for completion in completions {
                let result = crate::ipc::RpcGateway::decode_response(
                    &completion.method,
                    completion.response,
                );
                self.apply_rpc_result(&completion.method, result);
            }
        }
    }

    pub fn check_update(&mut self) {
        self.rpc.submit(method::RESOURCE_CHECK_UPDATE, None);
        self.feedback = Some("正在检查资源更新".to_owned());
    }

    pub fn sync_now(&mut self) {
        if self.sync_progress.is_some() {
            return;
        }
        self.sync_progress = Some(0);
        self.sync_finish_scheduled = false;
        self.rpc.submit(method::RESOURCE_SYNC_START, None);
        self.feedback = Some("正在启动资源同步".to_owned());
    }

    pub fn finish_sync(&mut self) {
        self.sync_progress = None;
        self.sync_finish_scheduled = false;
    }

    /// Consume one newly completed sync so the event pump can schedule the
    /// terminal-progress delay outside the render path.
    pub(crate) fn take_completed_sync(&mut self) -> bool {
        if self.sync_progress == Some(100) && !self.sync_finish_scheduled {
            self.sync_finish_scheduled = true;
            true
        } else {
            false
        }
    }

    pub(crate) fn apply_events(&mut self, events: Vec<EventEnvelope>) {
        for event in events {
            if event.event == event::RESOURCE_SYNC_PROGRESS {
                if let Some(error) = event.payload.get("error").and_then(|value| value.as_str()) {
                    self.sync_progress = None;
                    self.sync_finish_scheduled = false;
                    self.feedback = Some(error.to_owned());
                } else {
                    self.sync_progress = event
                        .payload
                        .get("progress")
                        .and_then(|value| value.as_u64())
                        .map(|progress| progress.min(100) as u8);
                }
            }
        }
    }

    pub(crate) fn apply_rpc_result(
        &mut self,
        method_name: &str,
        result: Result<Option<serde_json::Value>, crate::ipc::RpcError>,
    ) {
        let value = match result {
            Ok(value) => value,
            Err(error) => {
                if method_name == method::RESOURCE_SYNC_START {
                    self.sync_progress = None;
                }
                self.feedback = Some(error.message);
                return;
            }
        };
        match method_name {
            method::RESOURCE_CHECK_UPDATE | method::RESOURCE_STATUS => {
                let groups = value
                    .and_then(|value| serde_json::from_value::<Vec<ResourceGroup>>(value).ok())
                    .unwrap_or_default();
                let has_update = groups.iter().any(|group| {
                    group
                        .remoteVersion
                        .as_ref()
                        .is_some_and(|remote| remote != &group.localVersion)
                });
                self.groups = groups;
                if method_name == method::RESOURCE_CHECK_UPDATE {
                    self.feedback = Some(if has_update {
                        "发现资源更新".to_owned()
                    } else {
                        "资源已是最新版本".to_owned()
                    });
                }
            }
            method::RESOURCE_SYNC_START => {
                self.feedback = Some("资源同步已启动".to_owned());
                self.rpc.submit(method::RESOURCE_STATUS, None);
            }
            _ => {}
        }
    }
}
