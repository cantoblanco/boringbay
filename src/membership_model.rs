use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Membership {
    pub domain: String,
    pub name: String,
    pub icon: String,
    pub description: String,
    pub github_username: String,
}
