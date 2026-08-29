use super::*;

impl Default for ThemePacksState {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemePacksState {
    pub fn new() -> Self {
        Self::with_client(MockClient::default())
    }

    pub fn with_client(client: impl Into<crate::ipc::BackendClient>) -> Self {
        let data = ThemePackState::default();
        let mut state = Self {
            rpc: RpcGateway::new(client),
            confirmed_packs: data.packs.clone(),
            data,
            sort_by_weight: false,
            feedback: None,
            persist_due: None,
        };
        if !state.rpc.is_sidecar() {
            state.reload();
        }
        state
    }

    pub fn reload(&mut self) {
        if self.rpc.is_sidecar() {
            self.rpc.submit(method::THEME_PACK_LIST, None);
            return;
        }
        if let Some(value) = self.request(method::THEME_PACK_LIST, None)
            && let Ok(data) = serde_json::from_value(value)
        {
            self.data = data;
            self.confirmed_packs = self.data.packs.clone();
        }
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

    pub fn set_all_enabled(&mut self, enabled: bool) {
        let mut packs = self.data.packs.clone();
        for pack in &mut packs {
            pack.enabled = enabled;
        }
        self.persist_packs(packs);
    }

    pub fn toggle_enabled(&mut self, id: &str) {
        if let Some(pack) = self.data.packs.iter().find(|pack| pack.id == id) {
            let enabled = !pack.enabled;
            self.patch_pack(id, |pack| pack.enabled = enabled);
        }
    }

    pub fn cycle_weight(&mut self, id: &str) {
        if let Some(pack) = self.data.packs.iter().find(|pack| pack.id == id)
            && pack.enabled
        {
            let weight = if pack.weight >= 10 {
                0
            } else {
                pack.weight + 1
            };
            self.set_weight(id, weight);
        }
    }

    pub fn adjust_weight(&mut self, id: &str, delta: i8) {
        let Some(pack) = self.data.packs.iter().find(|pack| pack.id == id) else {
            return;
        };
        if !pack.enabled {
            return;
        }
        let weight = if delta < 0 {
            pack.weight.saturating_sub((-delta) as u8)
        } else {
            pack.weight.saturating_add(delta as u8).min(10)
        };
        self.set_weight(id, weight);
    }

    pub fn set_weight(&mut self, id: &str, weight: u8) {
        if self
            .data
            .packs
            .iter()
            .any(|pack| pack.id == id && pack.enabled)
        {
            self.patch_pack(id, |pack| pack.weight = weight.min(10));
        }
    }

    pub fn reset_weights(&mut self) {
        if self.rpc.is_sidecar() {
            self.persist_due = None;
            self.rpc.submit(method::THEME_PACK_RESET_WEIGHTS, None);
            self.feedback = Some("正在恢复默认权重".to_owned());
            return;
        }
        match self
            .rpc
            .request_value(method::THEME_PACK_RESET_WEIGHTS, None)
        {
            Err(error) => self.feedback = Some(error.message),
            Ok(Some(value)) => {
                if let Ok(data) = serde_json::from_value(value) {
                    self.data = data;
                    self.feedback = Some("已恢复默认权重".to_owned());
                }
            }
            Ok(None) => {}
        }
    }

    fn patch_pack(&mut self, id: &str, patch: impl FnOnce(&mut ThemePack)) {
        let mut packs = self.data.packs.clone();
        if let Some(pack) = packs.iter_mut().find(|pack| pack.id == id) {
            patch(pack);
            self.persist_packs(packs);
        }
    }

    fn persist_packs(&mut self, packs: Vec<ThemePack>) {
        if self.rpc.is_sidecar() {
            self.data.packs = packs;
            self.persist_due =
                Some(std::time::Instant::now() + std::time::Duration::from_millis(200));
            self.feedback = Some("主题包设置待保存".to_owned());
            return;
        }
        let result = self.rpc.request_value(
            method::THEME_PACK_UPDATE_ALL,
            Some(json!({ "packs": packs })),
        );
        if let Err(error) = result {
            self.feedback = Some(error.message);
            return;
        }
        self.data.packs = packs;
        self.confirmed_packs = self.data.packs.clone();
        self.feedback = Some("主题包设置已保存".to_owned());
    }

    pub(crate) fn flush_debounced(&mut self) -> bool {
        let Some(due) = self.persist_due else {
            return false;
        };
        if std::time::Instant::now() < due {
            return false;
        }
        self.persist_due = None;
        self.rpc.submit(
            method::THEME_PACK_UPDATE_ALL,
            Some(json!({ "packs": self.data.packs.clone() })),
        );
        self.feedback = Some("正在保存主题包设置".to_owned());
        true
    }

    pub(crate) fn apply_rpc_result(
        &mut self,
        method_name: &str,
        result: Result<Option<serde_json::Value>, crate::ipc::RpcError>,
    ) {
        match (method_name, result) {
            (_, Err(error)) => {
                self.data.packs = self.confirmed_packs.clone();
                self.feedback = Some(error.message);
            }
            (method::THEME_PACK_LIST | method::THEME_PACK_RESET_WEIGHTS, Ok(Some(value))) => {
                if let Ok(data) = serde_json::from_value(value) {
                    self.data = data;
                    self.confirmed_packs = self.data.packs.clone();
                    self.feedback = Some(if method_name == method::THEME_PACK_RESET_WEIGHTS {
                        "已恢复默认权重".to_owned()
                    } else {
                        "主题包设置已刷新".to_owned()
                    });
                }
            }
            (method::THEME_PACK_UPDATE_ALL, Ok(_)) => {
                self.confirmed_packs = self.data.packs.clone();
                self.feedback = Some("主题包设置已保存".to_owned());
            }
            _ => {}
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
