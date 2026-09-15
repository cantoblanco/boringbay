use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use tokio::net::lookup_host;
use url::{Host, Url};

#[derive(Clone, Debug)]
pub struct PublicHttpsUrl(Url);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkPolicyError(String);

impl fmt::Display for NetworkPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for NetworkPolicyError {}

impl PublicHttpsUrl {
    pub fn parse(input: &str) -> Result<Self, NetworkPolicyError> {
        let url = Url::parse(input).map_err(|_| error("invalid URL"))?;
        if url.scheme() != "https" {
            return Err(error("only HTTPS URLs are allowed"));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(error("URL user information is not allowed"));
        }
        match url.host().ok_or_else(|| error("URL must contain a host"))? {
            Host::Domain(host) => {
                if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
                    return Err(error("local hosts are not allowed"));
                }
            }
            Host::Ipv4(ip) => {
                if !is_public_ip(IpAddr::V4(ip)) {
                    return Err(error("non-public IP addresses are not allowed"));
                }
            }
            Host::Ipv6(ip) => {
                if !is_public_ip(IpAddr::V6(ip)) {
                    return Err(error("non-public IP addresses are not allowed"));
                }
            }
        }
        Ok(Self(url))
    }

    pub fn as_url(&self) -> &Url {
        &self.0
    }

    pub async fn validate_resolution(&self) -> Result<(), NetworkPolicyError> {
        let host = self
            .0
            .host_str()
            .ok_or_else(|| error("URL must contain a host"))?;
        let port = self.0.port_or_known_default().unwrap_or(443);
        let resolved = lookup_host((host, port))
            .await
            .map_err(|_| error("host could not be resolved"))?
            .collect::<Vec<_>>();
        if resolved.is_empty() || resolved.iter().any(|addr| !is_public_ip(addr.ip())) {
            return Err(error("host resolved to a non-public address"));
        }
        Ok(())
    }
}

fn error(message: &str) -> NetworkPolicyError {
    NetworkPolicyError(message.to_string())
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => is_public_ipv6(ip),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_broadcast()
        || ip.is_documentation()
        || octets[0] == 0
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 0 && octets[2] == 0)
        || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
        || octets[0] >= 240)
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return is_public_ipv4(mapped);
    }
    !(ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] == 0x2001 && segments[1] == 0x0db8))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_public_https_urls_are_accepted() {
        assert!(PublicHttpsUrl::parse("https://example.com/feed.xml").is_ok());
        for url in [
            "http://example.com/feed.xml",
            "https://localhost/feed.xml",
            "https://127.0.0.1/feed.xml",
            "https://10.0.0.1/feed.xml",
            "https://[::1]/feed.xml",
            "https://user:pass@example.com/feed.xml",
        ] {
            assert!(PublicHttpsUrl::parse(url).is_err(), "{url}");
        }
    }
}
