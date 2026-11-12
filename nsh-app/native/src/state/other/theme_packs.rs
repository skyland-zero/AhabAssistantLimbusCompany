//! Mirror Dungeon theme packs: the enabled set and each pack's weight.

use serde_json::json;

use crate::{
    ipc::{BackendClient, RpcGateway, contract::method},
    model::{ThemePack, ThemePackState},
};

pub struct ThemePacksState {
    rpc: RpcGateway,
    pub data: ThemePackState,
    pub sort_by_weight: bool,
    pub feedback: Option<String>,
    pub error: Option<String>,
}

impl ThemePacksState {
    pub fn new(client: impl Into<BackendClient>) -> Self {
        let mut state = Self {
            rpc: RpcGateway::new(client),
            data: ThemePackState::default(),
            sort_by_weight: false,
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
        self.data = self
            .rpc
            .request(method::THEME_PACK_LIST, None)
            .map_err(|error| error.message)?;
        self.error = None;
        Ok(())
    }

    pub fn sorted_packs(&self) -> Vec<ThemePack> {
        let mut packs = self.data.packs.clone();
        if self.sort_by_weight {
            packs
                .sort_by(|left, right| right.weight.cmp(&left.weight).then(left.id.cmp(&right.id)));
        }
        packs
    }

    pub fn total_weight(&self) -> u32 {
        self.data
            .packs
            .iter()
            .filter(|pack| pack.enabled)
            .map(|pack| u32::from(pack.weight))
            .sum()
    }

    pub fn set_sort_by_weight(&mut self, enabled: bool) {
        self.sort_by_weight = enabled;
    }

    pub fn toggle_enabled(&mut self, id: &str) -> Result<(), String> {
        let mut packs = self.data.packs.clone();
        let Some(pack) = packs.iter_mut().find(|pack| pack.id == id) else {
            return Ok(());
        };
        pack.enabled = !pack.enabled;
        self.persist(packs)
    }

    pub fn set_weight(&mut self, id: &str, weight: u8) -> Result<(), String> {
        let mut packs = self.data.packs.clone();
        let Some(pack) = packs.iter_mut().find(|pack| pack.id == id && pack.enabled) else {
            return Ok(());
        };
        pack.weight = weight.min(10);
        self.persist(packs)
    }

    pub fn set_all_enabled(&mut self, enabled: bool) -> Result<(), String> {
        let mut packs = self.data.packs.clone();
        for pack in &mut packs {
            pack.enabled = enabled;
        }
        self.persist(packs)
    }

    pub fn reset_weights(&mut self) -> Result<(), String> {
        self.data = self
            .rpc
            .request(method::THEME_PACK_RESET_WEIGHTS, None)
            .map_err(|error| error.message)?;
        self.feedback = Some("已恢复默认权重".to_owned());
        Ok(())
    }

    fn persist(&mut self, packs: Vec<ThemePack>) -> Result<(), String> {
        self.rpc
            .request_value(
                method::THEME_PACK_UPDATE_ALL,
                Some(json!({ "packs": packs })),
            )
            .map_err(|error| error.message)?;
        self.data.packs = packs;
        self.feedback = Some("主题包设置已保存".to_owned());
        Ok(())
    }
}
