use super::*;

impl MockState {
    pub(super) fn handle_teams(&mut self, request: &RpcRequest) -> Result<Value, RpcError> {
        match request.method.as_str() {
            method::TEAM_LIST => Ok(serde_json::to_value(&self.teams).unwrap()),
            method::TEAM_STATS_GET => {
                let values: Value =
                    Self::params(request.params.clone(), "team.stats.get requires {id}")?;
                let team_id = values.get("id").and_then(Value::as_str).ok_or_else(|| {
                    RpcError::invalid_params("team.stats.get requires a string id")
                })?;
                let team = self
                    .teams
                    .iter()
                    .find(|team| team.id == team_id)
                    .ok_or_else(|| RpcError::invalid_params("team.stats.get team not found"))?;
                let number = team_number_from_id(&team.id).unwrap_or_default();
                let stats = self
                    .team_stats
                    .entry(team.id.clone())
                    .or_insert_with(|| TeamStats::empty_for(team.id.clone(), number));
                Ok(serde_json::to_value(stats).unwrap())
            }
            method::TEAM_STATS_CLEAR => {
                let values: Value =
                    Self::params(request.params.clone(), "team.stats.clear requires {id}")?;
                let team_id = values.get("id").and_then(Value::as_str).ok_or_else(|| {
                    RpcError::invalid_params("team.stats.clear requires a string id")
                })?;
                let team = self
                    .teams
                    .iter()
                    .find(|team| team.id == team_id)
                    .ok_or_else(|| RpcError::invalid_params("team.stats.clear team not found"))?;
                let number = team_number_from_id(&team.id).unwrap_or_default();
                let stats = self
                    .team_stats
                    .entry(team.id.clone())
                    .or_insert_with(|| TeamStats::empty_for(team.id.clone(), number));
                *stats = TeamStats::empty_for(team.id.clone(), number);
                Ok(serde_json::to_value(stats).unwrap())
            }
            method::TEAM_PRESET_LIST => Ok(serde_json::to_value(builtin_team_presets()).unwrap()),
            method::TEAM_SAVE => {
                let raw: Value =
                    Self::params(request.params.clone(), "team.save requires an object")?;
                let raw_object = raw
                    .as_object()
                    .ok_or_else(|| RpcError::invalid_params("team.save requires an object"))?;
                let team_id = raw_object
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !team_id.is_empty() && raw_object.contains_key("teamNumber") {
                    Err(RpcError::invalid_params(
                        "team.save.teamNumber is only valid when creating a team",
                    ))
                } else {
                    let mut team: TeamDetail = if team_id.is_empty() {
                        Self::params(Some(raw.clone()), "team.save requires TeamDetail")?
                    } else {
                        let existing = self
                            .teams
                            .iter()
                            .find(|entry| entry.id == team_id)
                            .cloned()
                            .ok_or_else(|| RpcError::invalid_params("team.save team not found"))?;
                        let mut merged = serde_json::to_value(existing).unwrap();
                        let merged_object = merged.as_object_mut().unwrap();
                        merged_object.extend(raw_object.clone());
                        Self::params(Some(merged), "team.save requires TeamDetail")?
                    };
                    if team.name.trim().is_empty() {
                        Err(RpcError::invalid_params("team name required"))
                    } else {
                        if team.id.is_empty() {
                            let requested_number = raw_object
                                .get("teamNumber")
                                .map(|value| {
                                    value.as_u64().ok_or_else(|| {
                                        RpcError::invalid_params(
                                            "team.save.teamNumber requires a positive integer",
                                        )
                                    })
                                })
                                .transpose()?;
                            let team_number = requested_number.unwrap_or(self.next_team_id);
                            if team_number == 0 {
                                return Err(RpcError::invalid_params(
                                    "team.save.teamNumber requires a positive integer",
                                ));
                            }
                            let requested_id = format!("team-{team_number}");
                            if self.teams.iter().any(|entry| entry.id == requested_id) {
                                return Err(RpcError::invalid_params("team number already in use"));
                            }
                            team.id = requested_id;
                            self.next_team_id = self.next_team_id.max(team_number + 1);
                        }
                        if team.purpose == TeamPurpose::Luxcavation {
                            team.enabled = false;
                            team.mirrorConfig = None;
                        } else if team.mirrorConfig.is_none()
                            && let Some(existing) =
                                self.teams.iter().find(|entry| entry.id == team.id)
                        {
                            team.mirrorConfig = existing.mirrorConfig.clone();
                        }
                        if let Some(existing) =
                            self.teams.iter_mut().find(|entry| entry.id == team.id)
                        {
                            *existing = team.clone();
                        } else {
                            self.teams.push(team.clone());
                        }
                        Ok(json!(team))
                    }
                }
            }
            method::TEAM_DELETE => {
                let id_value: Value =
                    Self::params(request.params.clone(), "team.delete requires {id}")?;
                let team_id = id_value
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| RpcError::invalid_params("team.delete requires a string id"))?;
                self.teams.retain(|team| team.id != team_id);
                self.team_stats.remove(team_id);
                Ok(json!(true))
            }
            method::SINNER_LIST => Ok(serde_json::to_value(&self.sinners).unwrap()),
            _ => Err(RpcError::method_not_found(&request.method)),
        }
    }
}
