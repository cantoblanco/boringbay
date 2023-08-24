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
}

#[derive(Deserialize, Clone, Serialize)]
pub struct RankAndMembership {
    pub rank: Statistics,
    pub membership: Membership,
}
