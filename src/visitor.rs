use axum::http::HeaderMap;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use std::net::IpAddr;

use crate::config::TrustedProxyMode;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct VisitorHasher {
    key: [u8; 32],
}

impl VisitorHasher {
    pub fn random() -> Self {
        Self {
            key: rand::random(),
        }
    }

    pub fn from_key(key: [u8; 32]) -> Self {
        Self { key }
    }

    fn digest(&self, value: &[u8]) -> String {
        let mut mac =
            <HmacSha256 as KeyInit>::new_from_slice(&self.key).expect("HMAC accepts a 32-byte key");
        mac.update(value);
        hex::encode(mac.finalize().into_bytes())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VisitorIdentity {
    pub dedupe_key: String,
    pub country: String,
    pub masked_ip: String,
}

pub fn mask_ip(value: &str) -> Option<String> {
    match value.parse::<IpAddr>().ok()? {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            Some(format!("{}.****.{}", octets[0], octets[3]))
        }
        IpAddr::V6(ip) => {
            let segments = ip.segments();
            Some(format!(
                "{:x}:{:x}:****:{:x}:{:x}",
                segments[0], segments[1], segments[6], segments[7]
            ))
        }
    }
}

impl VisitorIdentity {
    pub fn from_headers(
        headers: &HeaderMap,
        mode: TrustedProxyMode,
        hasher: &VisitorHasher,
    ) -> Option<Self> {
        if mode != TrustedProxyMode::Cloudflare {
            return None;
        }
        let ip = headers.get("CF-Connecting-IP")?.to_str().ok()?.trim();
        let country = headers.get("CF-IPCountry")?.to_str().ok()?.trim();
        if ip.is_empty() || country.is_empty() || country.len() > 8 {
            return None;
        }
        let masked_ip = mask_ip(ip)?;
        Some(Self {
            dedupe_key: hasher.digest(ip.as_bytes()),
            country: country.to_ascii_uppercase(),
            masked_ip,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(ip: &str, country: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("CF-Connecting-IP", ip.parse().unwrap());
        headers.insert("CF-IPCountry", country.parse().unwrap());
        headers
    }

    #[test]
    fn missing_headers_do_not_count() {
        assert!(VisitorIdentity::from_headers(
            &HeaderMap::new(),
            TrustedProxyMode::Cloudflare,
            &VisitorHasher::from_key([7; 32]),
        )
        .is_none());
    }

    #[test]
    fn disabled_proxy_ignores_forged_headers() {
        assert!(VisitorIdentity::from_headers(
            &headers("203.0.113.9", "es"),
            TrustedProxyMode::Disabled,
            &VisitorHasher::from_key([7; 32]),
        )
        .is_none());
    }

    #[test]
    fn dedupe_key_never_contains_plain_ip() {
        let identity = VisitorIdentity::from_headers(
            &headers("203.0.113.9", "es"),
            TrustedProxyMode::Cloudflare,
            &VisitorHasher::from_key([7; 32]),
        )
        .unwrap();
        assert!(!identity.dedupe_key.contains("203.0.113.9"));
        assert_eq!(identity.country, "ES");
        assert_eq!(identity.masked_ip, "203.****.9");
    }

    #[test]
    fn masks_ipv4_and_ipv6_without_retaining_middle_segments() {
        assert_eq!(mask_ip("203.0.113.9").as_deref(), Some("203.****.9"));
        assert_eq!(
            mask_ip("2001:db8:1:2:3:4:5:6").as_deref(),
            Some("2001:db8:****:5:6")
        );
        assert_eq!(
            mask_ip("2001:db8::7").as_deref(),
            Some("2001:db8:****:0:7")
        );
        assert_eq!(mask_ip("not-an-ip"), None);
    }
}
