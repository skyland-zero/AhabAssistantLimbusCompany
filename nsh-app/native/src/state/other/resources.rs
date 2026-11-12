//! Synchronised resource groups and their sync progress.

use serde_json::json;

use crate::{
    ipc::{BackendClient, RpcGateway, contract::method},
    model::{ResourceGroup, SyncProgressPayload},
};

pub struct ResourcesState {
    rpc: RpcGateway,
    pub groups: Vec<ResourceGroup>,
    pub sync_progress: Option<u8>,
    pub feedback: Option<String>,
    pub error: Option<String>,
}

impl ResourcesState {
    pub fn new(client: impl Into<BackendClient>) -> Self {
        let mut state = Self {
            rpc: RpcGateway::new(client),
            groups: Vec::new(),
            sync_progress: None,
            feedback: None,
            error: None,
        };
        if let Err(error) = state.refresh() {
            state.error = Some(error);
        }
        state
    }

    pub fn from_shared() -> Self {
        Self::new(crate::ipc::bridge::state_client())
    }

    pub fn refresh(&mut self) -> Result<(), String> {
        self.groups = self
            .rpc
            .request(method::RESOURCE_STATUS, None)
            .map_err(|error| error.message)?;
        self.error = None;
        Ok(())
    }

    pub fn check_update(&mut self) -> Result<(), String> {
        self.groups = self
            .rpc
            .request(method::RESOURCE_CHECK_UPDATE, None)
            .map_err(|error| error.message)?;
        let has_update = self.groups.iter().any(|group| {
            group
                .remoteVersion
                .as_ref()
                .is_some_and(|remote| remote != &group.localVersion)
        });
        self.feedback = Some(if has_update {
            "发现资源更新".to_owned()
        } else {
            "资源已是最新版本".to_owned()
        });
        Ok(())
    }

    /// Start a sync and return the progress the transport reported. A mock
    /// completes on the first event; the UI may keep 100% visible briefly.
    pub fn sync_now(&mut self) -> Result<u8, String> {
        if self.sync_progress.is_some() {
            return Ok(self.sync_progress.unwrap_or(0));
        }
        self.sync_progress = Some(0);
        self.rpc
            .request_value(method::RESOURCE_SYNC_START, Some(json!({ "scope": "all" })))
            .map_err(|error| {
                self.sync_progress = None;
                error.message
            })?;
        for event in self.rpc.take_events() {
            if event.event == crate::ipc::contract::event::RESOURCE_SYNC_PROGRESS
                && let Ok(progress) = serde_json::from_value::<SyncProgressPayload>(event.payload)
            {
                self.sync_progress = Some(progress.progress.min(100));
            }
        }
        self.refresh()?;
        let progress = self.sync_progress.unwrap_or(100);
        self.feedback = Some("资源同步已启动".to_owned());
        Ok(progress)
    }

    pub fn finish_sync(&mut self) {
        if self.sync_progress == Some(100) {
            self.sync_progress = None;
            self.feedback = Some("资源同步完成".to_owned());
        }
    }
}
