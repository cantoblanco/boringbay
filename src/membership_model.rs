use serde::Deserialize;

#[derive(Deserialize)]
pub struct Membership {
    pub domain: String,
    pub name: String,
    pub icon: String,
    pub description: String,
    pub github_username: String,
}
