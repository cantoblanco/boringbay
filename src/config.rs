use std::env;

use anyhow::{anyhow, Context as _};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrustedProxyMode {
    Disabled,
    Cloudflare,
}

impl TrustedProxyMode {
    fn parse(value: &str) -> anyhow::Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "disabled" | "" => Ok(Self::Disabled),
            "cloudflare" => Ok(Self::Cloudflare),
            other => Err(anyhow!(
                "TRUSTED_PROXY_MODE must be disabled or cloudflare, got {}",
                other
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub system_domain: String,
    pub database_url: String,
    pub v2_enabled: bool,
    pub trusted_proxy_mode: TrustedProxyMode,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let system_domain = env::var("SYSTEM_DOMAIN").context("SYSTEM_DOMAIN is required")?;
        let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
        let v2_enabled = parse_v2_enabled(env::var("BORINGBAY_V2_ENABLED").ok())?;
        let trusted_proxy_mode = TrustedProxyMode::parse(
            &env::var("TRUSTED_PROXY_MODE").unwrap_or_else(|_| "disabled".to_string()),
        )?;
        Ok(Self {
            system_domain,
            database_url,
            v2_enabled,
            trusted_proxy_mode,
        })
    }

    pub fn for_test(database_url: String) -> Self {
        Self {
            system_domain: "boringbay.test".to_string(),
            database_url,
            v2_enabled: true,
            trusted_proxy_mode: TrustedProxyMode::Cloudflare,
        }
    }
}

fn parse_bool(value: &str) -> anyhow::Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        other => Err(anyhow!("invalid boolean value {}", other)),
    }
}

fn parse_v2_enabled(value: Option<String>) -> anyhow::Result<bool> {
    value.as_deref().map(parse_bool).unwrap_or(Ok(true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_mode_rejects_unknown_values() {
        assert!(TrustedProxyMode::parse("direct").is_err());
    }

    #[test]
    fn booleans_are_strict_but_human_friendly() {
        assert!(parse_bool("yes").unwrap());
        assert!(!parse_bool("OFF").unwrap());
        assert!(parse_bool("maybe").is_err());
    }

    #[test]
    fn v2_is_enabled_by_default_but_can_be_disabled() {
        assert!(parse_v2_enabled(None).unwrap());
        assert!(!parse_v2_enabled(Some("false".to_string())).unwrap());
    }
}
