use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[allow(non_snake_case)]
pub struct TeamStatsBucket {
    pub count: u32,
    pub averageSeconds: f64,
    pub last5AverageSeconds: f64,
    pub last10AverageSeconds: f64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct TeamStats {
    #[serde(default)]
    pub schemaVersion: u32,
    #[serde(default)]
    pub teamId: String,
    #[serde(default)]
    pub teamNumber: u32,
    #[serde(default)]
    pub totalCount: u32,
    #[serde(default)]
    pub hard: TeamStatsBucket,
    #[serde(default)]
    pub normal: TeamStatsBucket,
}

impl TeamStats {
    pub fn empty_for(team_id: impl Into<String>, team_number: u32) -> Self {
        Self {
            schemaVersion: 1,
            teamId: team_id.into(),
            teamNumber: team_number,
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_stats_round_trip_uses_rpc_field_names() {
        let stats = TeamStats {
            schemaVersion: 1,
            teamId: "team-2".into(),
            teamNumber: 2,
            totalCount: 5,
            hard: TeamStatsBucket {
                count: 3,
                averageSeconds: 120.5,
                last5AverageSeconds: 118.2,
                last10AverageSeconds: 121.4,
            },
            normal: TeamStatsBucket::default(),
        };
        let value = serde_json::to_value(&stats).unwrap();
        assert_eq!(value["teamId"], "team-2");
        assert_eq!(value["hard"]["last5AverageSeconds"], 118.2);
        assert_eq!(serde_json::from_value::<TeamStats>(value).unwrap(), stats);
    }
}
