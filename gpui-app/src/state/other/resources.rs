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
        let mut state = Self {
            rpc: RpcGateway::new(client),
            groups: Vec::new(),
            sync_progress: None,
            sync_finish_scheduled: false,
            feedback: None,
        };
        if !state.rpc.is_sidecar() {
            state.reload();
        }
        state
    }

    pub fn reload(&mut self) {
        if self.rpc.is_sidecar() {
            return;
        }
        if let Some(value) = self.request(method::RESOURCE_STATUS, None)
            && let Ok(groups) = serde_json::from_value(value)
        {
            self.groups = groups;
        }
    }

    pub fn check_update(&mut self) {
        if self.rpc.is_sidecar() {
            let _ = self.rpc.request_async(method::RESOURCE_CHECK_UPDATE, None);
            self.feedback = Some("正在检查资源更新".to_owned());
            return;
        }
        let groups = match self.rpc.request_value(method::RESOURCE_CHECK_UPDATE, None) {
            Err(error) => {
                self.feedback = Some(error.message);
                return;
            }
            Ok(value) => value
                .and_then(|value| serde_json::from_value::<Vec<ResourceGroup>>(value).ok())
                .unwrap_or_default(),
        };
        let has_update = groups.iter().any(|group| {
            group
                .remoteVersion
                .as_ref()
                .is_some_and(|remote| remote != &group.localVersion)
        });
        self.groups = groups;
        self.feedback = Some(if has_update {
            "发现资源更新".to_owned()
        } else {
            "资源已是最新版本".to_owned()
        });
    }

    pub fn sync_now(&mut self) {
        if self.sync_progress.is_some() {
            return;
        }
        self.sync_progress = Some(0);
        self.sync_finish_scheduled = false;
        if self.rpc.is_sidecar() {
            let _ = self.rpc.request_async(method::RESOURCE_SYNC_START, None);
            self.feedback = Some("资源同步请求已提交".to_owned());
            return;
        }
        let result = self.rpc.request_value(method::RESOURCE_SYNC_START, None);
        let events = self.rpc.take_events();
        self.apply_events(events);
        if let Err(error) = result {
            self.sync_progress = None;
            self.feedback = Some(error.message);
            return;
        }
        self.reload();
        self.feedback = Some("资源同步完成".to_owned());
    }

    pub fn finish_sync(&mut self) {
        self.sync_progress = None;
        self.sync_finish_scheduled = false;
    }

    pub(crate) fn apply_events(&mut self, events: Vec<EventEnvelope>) {
        for event in events {
            if event.event == event::RESOURCE_SYNC_PROGRESS {
                self.sync_progress = event
                    .payload
                    .get("progress")
                    .and_then(|value| value.as_u64())
                    .map(|progress| progress.min(100) as u8);
            }
        }
    }

    fn request(
        &mut self,
        method_name: &str,
        params: Option<serde_json::Value>,
    ) -> Option<serde_json::Value> {
        match self.rpc.request_value(method_name, params) {
            Ok(value) => value,
            Err(error) => {
                self.feedback = Some(error.message);
                None
            }
        }
    }
}
