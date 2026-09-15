use serde::{Deserialize, Serialize};

use crate::statistics_model::Statistics;

#[derive(Deserialize, Clone, Debug, Serialize)]
pub struct Membership {
    #[serde(skip_deserializing)]
    pub id: i64,
    pub domain: String,
    pub name: String,
    pub icon: String,
    pub description: String,
    pub github_username: String,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub feed_url: Option<String>,
    pub hidden: Option<bool>,
}

impl Membership {
    pub fn tag_csv(&self) -> String {
        self.tags.clone().unwrap_or_default().join(",")
    }
}

#[derive(Deserialize, Clone, Serialize)]
pub struct RankAndMembership {
    pub rank: Statistics,
    pub membership: Membership,
}
