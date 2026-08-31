use super::*;

impl TeamsState {
    pub fn count_for(&self, filter: TeamFilter) -> usize {
        match filter.purpose() {
            None => self.teams.len(),
            Some(purpose) => self
                .teams
                .iter()
                .filter(|team| team.purpose == purpose)
                .count(),
        }
    }

    pub fn sinner_name(&self, id: &str) -> String {
        self.sinners
            .iter()
            .find(|sinner| sinner.id == id)
            .map(|sinner| sinner.name.clone())
            .unwrap_or_else(|| id.to_owned())
    }

    pub fn open_new(&mut self) {
        self.open_new_with_slot(None, self.filter.purpose().unwrap_or(TeamPurpose::General));
    }

    pub fn open_new_for_slot(&mut self, number: u32) {
        if !(1..=TEAM_SLOT_COUNT).contains(&number) {
            return;
        }
        self.open_new_with_slot(Some(number), TeamSlot::default_purpose(number));
    }

    fn open_new_with_slot(&mut self, requested_team_number: Option<u32>, purpose: TeamPurpose) {
        self.close_select();
        let is_luxcavation = purpose == TeamPurpose::Luxcavation;
        self.editor = Some(TeamEditorState::new_with_slot(
            TeamDetail {
                schemaVersion: 1,
                id: String::new(),
                name: String::new(),
                sinners: Vec::new(),
                purpose,
                accessoryScheme: "burn".into(),
                enabled: !is_luxcavation,
                mirrorConfig: (!is_luxcavation).then(TeamMirrorConfig::default),
            },
            requested_team_number,
        ));
        self.feedback = None;
    }

    pub fn open_edit(&mut self, team: &TeamDetail) {
        self.close_select();
        self.editor = Some(TeamEditorState::new(team.clone()));
        self.feedback = None;
    }

    pub fn close_editor(&mut self) {
        self.editor = None;
        self.close_select();
    }

    pub fn set_editor_tab(&mut self, tab: TeamEditorTab) {
        if let Some(editor) = self.editor.as_mut() {
            editor.tab = if tab.is_available(editor.team.purpose) {
                tab
            } else {
                TeamEditorTab::Basic
            };
        }
        self.close_select();
    }

    pub fn toggle_select(&mut self, select: TeamSelect) {
        self.open_select = if self.open_select == Some(select) {
            None
        } else {
            Some(select)
        };
    }

    pub fn close_select(&mut self) {
        self.open_select = None;
    }

    pub fn is_select_open(&self, select: TeamSelect) -> bool {
        self.open_select == Some(select)
    }

    pub fn set_filter(&mut self, filter: TeamFilter) {
        self.filter = filter;
    }

    pub fn fixed_slots_for_filter(&self, filter: TeamFilter) -> Vec<TeamSlot> {
        let numbers: Vec<u32> = match filter {
            TeamFilter::All => (1..=TEAM_SLOT_COUNT).collect(),
            TeamFilter::Mirror => (2..=TEAM_SLOT_COUNT).collect(),
            TeamFilter::Luxcavation => vec![1],
            TeamFilter::General => Vec::new(),
        };
        let purpose = filter.purpose();
        numbers
            .into_iter()
            .filter_map(|number| {
                let team = self
                    .teams
                    .iter()
                    .find(|team| team_number_from_id(&team.id) == Some(number))
                    .cloned();
                if let Some(team) = team.as_ref()
                    && purpose.is_some()
                    && Some(team.purpose) != purpose
                {
                    return None;
                }
                Some(TeamSlot { number, team })
            })
            .collect()
    }

    pub fn extra_teams_for_filter(&self, filter: TeamFilter) -> Vec<TeamDetail> {
        let fixed_numbers: Vec<u32> = match filter {
            TeamFilter::All => (1..=TEAM_SLOT_COUNT).collect(),
            TeamFilter::Mirror => (2..=TEAM_SLOT_COUNT).collect(),
            TeamFilter::Luxcavation => vec![1],
            TeamFilter::General => Vec::new(),
        };
        self.filtered_teams_for(filter)
            .filter(|team| {
                team_number_from_id(&team.id)
                    .map(|number| !fixed_numbers.contains(&number))
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    }

    fn filtered_teams_for(&self, filter: TeamFilter) -> impl Iterator<Item = &TeamDetail> {
        let purpose = filter.purpose();
        self.teams
            .iter()
            .filter(move |team| purpose.is_none() || Some(team.purpose) == purpose)
    }

    pub fn set_editor_purpose(&mut self, purpose: TeamPurpose) {
        if let Some(editor) = self.editor.as_mut() {
            let was_luxcavation = editor.team.purpose == TeamPurpose::Luxcavation;
            editor.team.purpose = purpose;
            if purpose == TeamPurpose::Luxcavation {
                editor.team.enabled = false;
                editor.team.mirrorConfig = None;
            } else if was_luxcavation {
                editor.team.enabled = true;
                editor.team.mirrorConfig = Some(TeamMirrorConfig::default());
            }
            if !editor.tab.is_available(purpose) {
                editor.tab = TeamEditorTab::Basic;
            }
        }
        self.close_select();
    }

    pub fn set_editor_scheme(&mut self, scheme: impl Into<String>, system: u8) {
        if let Some(editor) = self.editor.as_mut() {
            if editor.team.purpose == TeamPurpose::Luxcavation {
                return;
            }
            editor.team.accessoryScheme = scheme.into();
            editor
                .team
                .mirrorConfig
                .get_or_insert_with(Default::default)
                .team_system = system;
        }
    }

    pub fn set_editor_enabled(&mut self, enabled: bool) {
        if let Some(editor) = self.editor.as_mut()
            && editor.team.purpose != TeamPurpose::Luxcavation
        {
            editor.team.enabled = enabled;
        }
    }

    pub fn toggle_sinner(&mut self, id: &str) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        if let Some(index) = editor
            .team
            .sinners
            .iter()
            .position(|selected| selected == id)
        {
            editor.team.sinners.remove(index);
        } else if editor.team.sinners.len() < 12 {
            editor.team.sinners.push(id.to_owned());
        }
    }

    pub fn clear_sinners(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            editor.team.sinners.clear();
        }
    }
}
