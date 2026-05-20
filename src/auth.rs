use std::fmt;

fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                out.push((h * 16 + l) as u8 as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn normalize_sid(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.contains('%') {
        url_decode(trimmed)
    } else {
        trimmed.to_string()
    }
}

#[derive(Clone)]
pub struct Credentials {
    pub sid: Option<String>,
    pub pub_base_url: Option<String>,
    pub publication_subdomain: Option<String>,
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("sid", &"[REDACTED]")
            .field("pub_base_url", &self.pub_base_url)
            .finish()
    }
}

pub fn load() -> Credentials {
    let sid = std::env::var("SUBSTACK_SID")
        .ok()
        .map(|s| normalize_sid(&s))
        .filter(|s| !s.is_empty());

    let publication_url = std::env::var("SUBSTACK_PUBLICATION_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty());

    let pub_base_url = publication_url
        .as_ref()
        .map(|url| format!("{url}/api/v1"));

    let publication_subdomain = publication_url.as_ref().and_then(|url| {
        let host = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);
        let host = host.split('/').next().unwrap_or(host);
        host.strip_suffix(".substack.com").map(|s| s.to_string())
    });

    Credentials {
        sid,
        pub_base_url,
        publication_subdomain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_sid() {
        let creds = Credentials {
            sid: Some("super-secret-sid".to_string()),
            pub_base_url: Some("https://test.substack.com/api/v1".to_string()),
            publication_subdomain: Some("test".to_string()),
        };
        let output = format!("{creds:?}");
        assert!(!output.contains("super-secret-sid"));
        assert!(output.contains("[REDACTED]"));
    }

    #[test]
    fn sid_url_encoded_is_decoded() {
        let encoded = "s%3Aabc123.xyz%2Ffoo";
        assert_eq!(normalize_sid(encoded), "s:abc123.xyz/foo");
    }

    #[test]
    fn sid_plain_is_unchanged() {
        let plain = "s:abc123.xyz/foo";
        assert_eq!(normalize_sid(plain), "s:abc123.xyz/foo");
    }

    #[test]
    fn sid_trimmed_of_whitespace() {
        assert_eq!(normalize_sid("  s:abc  "), "s:abc");
        assert_eq!(normalize_sid("  s%3Aabc  "), "s:abc");
    }

    #[test]
    fn pub_base_url_appends_api_v1() {
        std::env::set_var("SUBSTACK_PUBLICATION_URL", "https://test.substack.com");
        std::env::remove_var("SUBSTACK_SID");
        let creds = load();
        assert_eq!(
            creds.pub_base_url.as_deref(),
            Some("https://test.substack.com/api/v1")
        );
        std::env::remove_var("SUBSTACK_PUBLICATION_URL");
    }
}
