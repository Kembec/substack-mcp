use anyhow::{anyhow, Result};
use serde_json::Value;

pub fn validate_slug(s: &str) -> Result<()> {
    if s.is_empty() || s.len() > 100 {
        return Err(anyhow!("slug must be 1-100 characters"));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow!("slug contains invalid characters"));
    }
    Ok(())
}

pub fn validate_post_slug(s: &str) -> Result<()> {
    if s.is_empty() || s.len() > 200 {
        return Err(anyhow!("post_slug must be 1-200 characters"));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow!("post_slug contains invalid characters"));
    }
    Ok(())
}

pub fn validate_pub_url(url: &str) -> Result<String> {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(anyhow!("publication_url is empty"));
    }
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let host = without_scheme
        .split('/')
        .next()
        .unwrap_or(without_scheme)
        .split(':')
        .next()
        .unwrap_or(without_scheme);
    if !host.ends_with(".substack.com") && host != "substack.com" {
        return Err(anyhow!("publication_url must be a *.substack.com host"));
    }
    Ok(host.to_string())
}

pub fn validate_numeric_id(id: &str, field: &str) -> Result<()> {
    if id.is_empty() || id.len() > 20 {
        return Err(anyhow!("{field} must be 1-20 digits"));
    }
    if !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("{field} must contain only digits"));
    }
    Ok(())
}

pub fn validate_limit(v: u64) -> Result<u32> {
    if !(1..=50).contains(&v) {
        return Err(anyhow!("limit must be between 1 and 50"));
    }
    Ok(v as u32)
}

pub fn validate_offset(v: u64) -> Result<u32> {
    u32::try_from(v).map_err(|_| anyhow!("offset out of range"))
}

pub fn validate_note_body(s: &str) -> Result<()> {
    if s.is_empty() || s.len() > 500 {
        return Err(anyhow!("body must be 1-500 characters"));
    }
    if s.contains('\0') {
        return Err(anyhow!("body must not contain null bytes"));
    }
    Ok(())
}

pub fn validate_post_title(s: &str) -> Result<()> {
    if s.is_empty() || s.len() > 280 {
        return Err(anyhow!("title must be 1-280 characters"));
    }
    if s.contains('\0') {
        return Err(anyhow!("title must not contain null bytes"));
    }
    Ok(())
}

pub fn validate_post_body(s: &str) -> Result<()> {
    if s.is_empty() || s.len() > 50_000 {
        return Err(anyhow!("body must be 1-50000 characters"));
    }
    if s.contains('\0') {
        return Err(anyhow!("body must not contain null bytes"));
    }
    Ok(())
}

pub fn validate_audience(s: &str) -> Result<()> {
    match s {
        "everyone" | "paid" => Ok(()),
        _ => Err(anyhow!("audience must be 'everyone' or 'paid'")),
    }
}

pub fn validate_image_path(path: &str) -> Result<()> {
    if path.is_empty() || path.len() > 4096 {
        return Err(anyhow!("file_path must be 1-4096 characters"));
    }
    let lower = path.to_lowercase();
    if !lower.ends_with(".jpg")
        && !lower.ends_with(".jpeg")
        && !lower.ends_with(".png")
        && !lower.ends_with(".gif")
        && !lower.ends_with(".webp")
    {
        return Err(anyhow!(
            "file_path must be a .jpg, .jpeg, .png, .gif, or .webp image"
        ));
    }
    Ok(())
}

pub fn validate_image_url(url: &str) -> Result<()> {
    if url.is_empty() || url.len() > 4096 {
        return Err(anyhow!("image_url must be 1-4096 characters"));
    }
    if !url.starts_with("https://") {
        return Err(anyhow!("image_url must start with https://"));
    }
    Ok(())
}

pub fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| anyhow!("missing required parameter: {key}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validate_slug_accepts_valid() {
        assert!(validate_slug("kembec").is_ok());
    }

    #[test]
    fn validate_slug_rejects_empty() {
        assert!(validate_slug("").is_err());
    }

    #[test]
    fn validate_slug_rejects_invalid_chars() {
        assert!(validate_slug("bad slug!").is_err());
    }

    #[test]
    fn validate_pub_url_extracts_host() {
        let host = validate_pub_url("https://kembec.substack.com").unwrap();
        assert_eq!(host, "kembec.substack.com");
    }

    #[test]
    fn validate_pub_url_rejects_non_substack() {
        assert!(validate_pub_url("https://evil.com").is_err());
    }

    #[test]
    fn validate_numeric_id_rejects_letters() {
        assert!(validate_numeric_id("abc", "post_id").is_err());
    }

    #[test]
    fn validate_limit_bounds() {
        assert!(validate_limit(0).is_err());
        assert!(validate_limit(51).is_err());
        assert_eq!(validate_limit(10).unwrap(), 10);
    }

    #[test]
    fn validate_audience_allowlist() {
        assert!(validate_audience("everyone").is_ok());
        assert!(validate_audience("paid").is_ok());
        assert!(validate_audience("free").is_err());
    }

    #[test]
    fn require_str_rejects_missing() {
        assert!(require_str(&json!({}), "x").is_err());
    }
}
