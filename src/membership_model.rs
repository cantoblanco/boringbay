use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone, Serialize)]
pub struct Membership {
    pub domain: String,
    pub name: String,
    pub icon: String,
    pub description: String,
    pub github_username: String,
}
