use headers::HeaderMap;
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use sha2::Sha256;

use crate::config::TrustedProxyMode;

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone)]
pub struct VisitorHasher {
    key: [u8; 32],
}

impl VisitorHasher {
    pub fn random() -> Self {
        let mut key = [0_u8; 32];
        OsRng.fill_bytes(&mut key);
        Self { key }
    }

    pub fn from_key(key: [u8; 32]) -> Self {
        Self { key }
    }

    fn digest(&self, value: &[u8]) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("HMAC accepts a 32-byte key");
        mac.update(value);
        hex::encode(mac.finalize().into_bytes())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VisitorIdentity {
    pub dedupe_key: String,
    pub country: String,
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
        Some(Self {
            dedupe_key: hasher.digest(ip.as_bytes()),
            country: country.to_ascii_uppercase(),
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
    }
}
