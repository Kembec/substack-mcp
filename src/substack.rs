use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use serde_json::{json, Value};
use tokio::sync::OnceCell;

use crate::auth::Credentials;

pub struct SubstackClient {
    http: reqwest::Client,
    creds: Credentials,
    pub_session: OnceCell<()>,
}

impl SubstackClient {
    pub fn new(http: reqwest::Client, creds: Credentials) -> Self {
        Self {
            http,
            creds,
            pub_session: OnceCell::new(),
        }
    }

    fn global_base() -> &'static str {
        "https://substack.com/api/v1"
    }

    fn auth_headers(&self) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        if let Some(sid) = &self.creds.sid {
            let cookie = format!("connect.sid={sid}");
            headers.insert(
                COOKIE,
                HeaderValue::from_str(&cookie)
                    .map_err(|_| anyhow!("invalid SUBSTACK_SID cookie value"))?,
            );
        }
        Ok(headers)
    }

    fn text_to_prosemirror(text: &str) -> Value {
        json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{"type": "text", "text": text}]
            }]
        })
    }

    fn require_sid(&self) -> Result<&str> {
        self.creds
            .sid
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("SUBSTACK_SID not configured"))
    }

    fn require_pub_base(&self) -> Result<&str> {
        self.creds
            .pub_base_url
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("SUBSTACK_PUBLICATION_URL not configured"))
    }

    fn require_subdomain(&self) -> Result<&str> {
        self.creds
            .publication_subdomain
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("SUBSTACK_PUBLICATION_URL must be a *.substack.com URL"))
    }

    async fn prepare_draft_session(&self) -> Result<()> {
        self.pub_session
            .get_or_try_init(|| async {
                self.require_sid()?;
                let subdomain = self.require_subdomain()?;
                let headers = self.auth_headers()?;

                let _ = self
                    .http
                    .get("https://substack.com/")
                    .headers(headers.clone())
                    .send()
                    .await?;

                let signin_url =
                    format!("https://substack.com/sign-in?redirect=%2F&for_pub={subdomain}");
                let _ = self.http.get(&signin_url).headers(headers).send().await?;

                Ok(())
            })
            .await
            .map(|_| ())
    }

    async fn get_user_profile_self(&self) -> Result<Value> {
        self.require_sid()?;
        let url = format!("{}/user/profile/self", Self::global_base());
        let response = self
            .http
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;
        self.parse_response(response).await
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<Value> {
        let status = response.status();
        let body = response.text().await?;
        if status.as_u16() == 401 || body.contains("Please sign in") || body.contains("Not authorized")
        {
            return Err(anyhow!(
                "Substack authentication failed (HTTP {}). Refresh SUBSTACK_SID from browser DevTools → Application → Cookies → substack.com → connect.sid",
                status.as_u16()
            ));
        }
        if !status.is_success() {
            return Err(anyhow!("HTTP {}: {}", status.as_u16(), body));
        }
        if body.trim().is_empty() {
            return Ok(json!({}));
        }
        serde_json::from_str(&body).map_err(|e| anyhow!("invalid JSON response: {e}"))
    }

    pub async fn get_profile(&self, slug: &str) -> Result<Value> {
        let url = format!("{}/user/{slug}/profile", Self::global_base());
        let response = self
            .http
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;
        self.parse_response(response).await
    }

    pub async fn get_posts(&self, pub_host: &str, limit: u32, offset: u32) -> Result<Value> {
        let url = format!("https://{pub_host}/api/v1/archive");
        let limit_s = limit.to_string();
        let offset_s = offset.to_string();
        let response = self
            .http
            .get(&url)
            .query(&[("sort", "new"), ("limit", &limit_s), ("offset", &offset_s)])
            .headers(self.auth_headers()?)
            .send()
            .await?;
        self.parse_response(response).await
    }

    pub async fn get_post(&self, pub_host: &str, post_slug: &str) -> Result<Value> {
        let url = format!("https://{pub_host}/api/v1/posts/{post_slug}");
        let response = self
            .http
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;
        self.parse_response(response).await
    }

    pub async fn get_comments(&self, post_id: &str, limit: u32) -> Result<Value> {
        let url = format!("{}/post/{post_id}/comments", Self::global_base());
        let limit_s = limit.to_string();
        let response = self
            .http
            .get(&url)
            .query(&[("all_comments", "true"), ("limit", &limit_s)])
            .headers(self.auth_headers()?)
            .send()
            .await?;
        self.parse_response(response).await
    }

    pub async fn get_notes(&self, user_id: &str, limit: u32, offset: u32) -> Result<Value> {
        let url = format!("{}/profile/{user_id}/notes", Self::global_base());
        let limit_s = limit.to_string();
        let offset_s = offset.to_string();
        let response = self
            .http
            .get(&url)
            .query(&[
                ("types[]", "note"),
                ("limit", &limit_s),
                ("offset", &offset_s),
            ])
            .headers(self.auth_headers()?)
            .send()
            .await?;
        self.parse_response(response).await
    }

    pub async fn create_note(&self, body: &str) -> Result<Value> {
        self.require_sid()?;
        let url = format!("{}/note", Self::global_base());
        let payload = json!({ "bodyJson": Self::text_to_prosemirror(body) });
        let response = self
            .http
            .post(&url)
            .headers(self.auth_headers()?)
            .json(&payload)
            .send()
            .await?;
        self.parse_response(response).await
    }

    pub async fn like_post(&self, post_id: &str) -> Result<Value> {
        self.require_sid()?;
        let url = format!("{}/post/{post_id}/like", Self::global_base());
        let response = self
            .http
            .post(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;
        self.parse_response(response).await
    }

    pub async fn list_drafts(&self, limit: u32, offset: u32) -> Result<Value> {
        self.prepare_draft_session().await?;
        let pub_base = self.require_pub_base()?;
        let url = format!("{pub_base}/drafts");
        let limit_s = limit.to_string();
        let offset_s = offset.to_string();
        let response = self
            .http
            .get(&url)
            .query(&[
                ("filter", "draft"),
                ("limit", &limit_s),
                ("offset", &offset_s),
            ])
            .headers(self.auth_headers()?)
            .send()
            .await?;
        self.parse_response(response).await
    }

    pub async fn create_draft(&self, title: &str, body: &str, audience: &str) -> Result<Value> {
        self.prepare_draft_session().await?;
        let pub_base = self.require_pub_base()?;
        let profile = self.get_user_profile_self().await?;
        let user_id = profile
            .get("id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow!("could not read user id from profile"))?;
        let draft_body = serde_json::to_string(&Self::text_to_prosemirror(body))?;
        let payload = json!({
            "draft_title": title,
            "draft_body": draft_body,
            "audience": audience,
            "draft_bylines": [{"id": user_id, "is_guest": false}],
        });
        let url = format!("{pub_base}/drafts");
        let response = self
            .http
            .post(&url)
            .headers(self.auth_headers()?)
            .json(&payload)
            .send()
            .await?;
        self.parse_response(response).await
    }

    pub async fn update_draft(
        &self,
        draft_id: &str,
        title: Option<&str>,
        body: Option<&str>,
    ) -> Result<Value> {
        self.prepare_draft_session().await?;
        let pub_base = self.require_pub_base()?;
        let mut payload = serde_json::Map::new();
        if let Some(t) = title {
            payload.insert("draft_title".to_string(), json!(t));
        }
        if let Some(b) = body {
            let draft_body = serde_json::to_string(&Self::text_to_prosemirror(b))?;
            payload.insert("draft_body".to_string(), json!(draft_body));
        }
        if payload.is_empty() {
            return Err(anyhow!("at least one of title or body must be provided"));
        }
        let url = format!("{pub_base}/drafts/{draft_id}");
        let response = self
            .http
            .put(&url)
            .headers(self.auth_headers()?)
            .json(&Value::Object(payload))
            .send()
            .await?;
        self.parse_response(response).await
    }

    pub async fn publish_draft(&self, draft_id: &str, send_email: bool) -> Result<Value> {
        self.prepare_draft_session().await?;
        let pub_base = self.require_pub_base()?;
        let url = format!("{pub_base}/drafts/{draft_id}/publish");
        let payload = json!({
            "send": send_email,
            "share_automatically": false,
        });
        let response = self
            .http
            .post(&url)
            .headers(self.auth_headers()?)
            .json(&payload)
            .send()
            .await?;
        self.parse_response(response).await
    }

    pub async fn upload_image(&self, image_url: &str) -> Result<String> {
        self.prepare_draft_session().await?;
        let pub_base = self.require_pub_base()?;
        let url = format!("{pub_base}/image");
        let payload = json!({ "image": image_url });
        let response = self
            .http
            .post(&url)
            .headers(self.auth_headers()?)
            .json(&payload)
            .send()
            .await?;
        let json = self.parse_response(response).await?;
        json.get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("upload_image: response missing 'url' field"))
    }

    pub async fn set_draft_cover_image(&self, draft_id: &str, image_url: &str) -> Result<Value> {
        self.prepare_draft_session().await?;
        let pub_base = self.require_pub_base()?;
        let url = format!("{pub_base}/drafts/{draft_id}");
        let payload = json!({ "cover_image": image_url });
        let response = self
            .http
            .put(&url)
            .headers(self.auth_headers()?)
            .json(&payload)
            .send()
            .await?;
        self.parse_response(response).await
    }
}
