use super::*;

impl HomeState {
    pub(crate) fn suspend_preview(&mut self) -> bool {
        let changed = self.latest_screenshot.is_some()
            || self.preview_status != PreviewStatus::Stopped
            || self.preview_error.is_some();
        self.latest_screenshot = None;
        self.preview_status = PreviewStatus::Stopped;
        self.preview_error = None;
        if changed {
            self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
        }
        changed
    }

    pub(crate) fn start_preview(&mut self) -> bool {
        let changed =
            self.preview_status != PreviewStatus::Starting || self.preview_error.is_some();
        self.preview_status = PreviewStatus::Starting;
        self.preview_error = None;
        changed
    }

    pub(crate) fn reset_after_sidecar_restart(&mut self) {
        self.execution = ExecutionStatusPayload::default();
        self.state_before_stopping = None;
        self.last_event_sequence = 0;
        self.execution_event_sequences.clear();
        self.next_client_request_id = 0;
        self.selected_device = None;
        self.device_status = ConnectionStatus::Disconnected;
        self.latest_screenshot = None;
        self.preview_status = PreviewStatus::Stopped;
        self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
        self.preview_identity = None;
        self.preview_generation_floor.clear();
        self.preview_recent_run_id = None;
    }

    pub(crate) fn current_execution_run_id(&self) -> Option<&str> {
        self.execution
            .runId
            .as_deref()
            .or(self.stats.currentRun.runId.as_deref())
    }

    /// Runner-derived events must never be allowed to resurrect a completed
    /// run or overwrite a newer run.  A run id is optional only for legacy
    /// schema-2 events; once schema-3 state has identified a run, an explicit
    /// mismatching id is rejected.
    pub(crate) fn accepts_derived_run(&self, run_id: Option<&str>) -> bool {
        let Some(run_id) = run_id else {
            return true;
        };
        match self.current_execution_run_id() {
            Some(current) if current == run_id => self.execution.state != ExecutionState::Idle,
            Some(_) => {
                self.execution.state == ExecutionState::Idle && self.execution.stateRevision == 0
            }
            None => true,
        }
    }

    pub(crate) fn preview_execution_is_active(&self) -> bool {
        self.execution.state != ExecutionState::Idle
            || self.execution.deviceLease != DeviceLeaseState::None
    }

    /// Extract the identity fields that Python emits on every preview event.
    /// Missing fields are deliberately rejected rather than defaulted: a
    /// partial handshake must never overwrite the last known frame.
    pub(crate) fn preview_metadata(
        payload: &serde_json::Value,
    ) -> Option<(String, Option<String>, u64)> {
        let device_id = payload.get("deviceId")?.as_str()?.to_owned();
        if device_id.is_empty() {
            return None;
        }
        let run_id = match payload.get("runId")? {
            serde_json::Value::Null => None,
            serde_json::Value::String(value) if !value.is_empty() => Some(value.clone()),
            _ => return None,
        };
        let generation = payload.get("generation")?.as_u64()?;
        Some((device_id, run_id, generation))
    }

    /// Accept only the currently leased run while execution is active.  Once
    /// idle, the completed run remains a short-lived compatibility allowance,
    /// while a null-run sidecar preview may establish a fresh baseline.
    pub(crate) fn preview_run_is_allowed(&self, run_id: Option<&str>) -> bool {
        let current = self.current_execution_run_id();
        if self.preview_execution_is_active() {
            return run_id.is_some_and(|run_id| current == Some(run_id));
        }
        match run_id {
            Some(run_id) => {
                current == Some(run_id) || self.preview_recent_run_id.as_deref() == Some(run_id)
            }
            None => true,
        }
    }

    pub(crate) fn accepts_preview_identity(
        &self,
        device_id: &str,
        run_id: Option<&str>,
        generation: u64,
    ) -> bool {
        if self.selected_device.as_deref() != Some(device_id)
            || !self.preview_run_is_allowed(run_id)
        {
            return false;
        }

        // After a null-run sidecar preview has established a baseline, a late
        // event from the completed Runner run must not reclaim the surface.
        if !self.preview_execution_is_active()
            && self
                .preview_identity
                .as_ref()
                .is_some_and(|identity| identity.run_id.is_none() && run_id.is_some())
        {
            return false;
        }

        let key = (device_id.to_owned(), run_id.map(str::to_owned));
        self.preview_generation_floor
            .get(&key)
            .is_none_or(|floor| generation >= *floor)
    }

    pub(crate) fn record_preview_identity(
        &mut self,
        device_id: &str,
        run_id: Option<&str>,
        generation: u64,
    ) {
        let run_id = run_id.map(str::to_owned);
        let key = (device_id.to_owned(), run_id.clone());
        self.preview_generation_floor
            .entry(key)
            .and_modify(|floor| *floor = (*floor).max(generation))
            .or_insert(generation);
        self.preview_identity = Some(PreviewEventIdentity {
            device_id: device_id.to_owned(),
            run_id,
            generation,
        });
    }

    pub(crate) fn reconcile_preview_execution_boundary(
        &mut self,
        previous: &ExecutionStatusPayload,
        current: &ExecutionStatusPayload,
    ) {
        let was_active = previous.state != ExecutionState::Idle
            || previous.deviceLease != DeviceLeaseState::None;
        let is_active =
            current.state != ExecutionState::Idle || current.deviceLease != DeviceLeaseState::None;
        if !is_active {
            if let Some(run_id) = current.runId.clone().or_else(|| previous.runId.clone()) {
                self.preview_recent_run_id = Some(run_id);
            }
            // The next null-run sidecar event is allowed to establish its own
            // generation baseline after lease restoration reaches idle.
            self.preview_identity = None;
            if self.latest_screenshot.take().is_some() {
                self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
            }
        } else if !was_active || previous.runId != current.runId {
            // An active execution boundary invalidates any sidecar preview
            // identity left over from the previous idle period.
            self.preview_identity = None;
            if self.latest_screenshot.take().is_some() {
                self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
            }
        }
    }

    /// `app.exitRequested` is emitted after the sidecar has finalized a run,
    /// so the normal derived-event gate (which intentionally rejects events
    /// for a run already in `idle`) cannot be used for it.  Keep the explicit
    /// run association, however, so a delayed completion from an older run
    /// cannot close the current application.
    pub(crate) fn accepts_exit_request(&self, run_id: Option<&str>) -> bool {
        run_id.is_none_or(|run_id| self.current_execution_run_id() == Some(run_id))
    }
}
