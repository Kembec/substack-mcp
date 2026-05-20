use std::sync::Arc;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::mcp::ServerState;
use crate::tools_validation::{
    self, validate_audience, validate_image_url, validate_limit, validate_note_body,
    validate_numeric_id, validate_offset, validate_post_body, validate_post_slug,
    validate_post_title, validate_pub_url, validate_slug,
};

pub fn tools_list() -> Value {
    json!({
        "tools": [
            {
                "name": "get_profile",
                "description": "Get a Substack user profile by slug.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "slug": { "type": "string", "description": "User handle without @" }
                    },
                    "required": ["slug"],
                    "additionalProperties": false
                }
            },
            {
                "name": "get_posts",
                "description": "List posts from a Substack publication archive.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "publication_url": { "type": "string", "description": "Publication URL or hostname" },
                        "limit": { "type": "integer", "description": "Max posts (1-50, default 10)" },
                        "offset": { "type": "integer", "description": "Pagination offset (default 0)" }
                    },
                    "required": ["publication_url"],
                    "additionalProperties": false
                }
            },
            {
                "name": "get_post",
                "description": "Get a single post by slug from a publication.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "publication_url": { "type": "string" },
                        "post_slug": { "type": "string" }
                    },
                    "required": ["publication_url", "post_slug"],
                    "additionalProperties": false
                }
            },
            {
                "name": "get_comments",
                "description": "List comments on a post by numeric post ID.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "post_id": { "type": "string" },
                        "limit": { "type": "integer", "description": "1-50, default 10" }
                    },
                    "required": ["post_id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "get_notes",
                "description": "List Notes for a user by numeric user ID.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "user_id": { "type": "string" },
                        "limit": { "type": "integer" },
                        "offset": { "type": "integer" }
                    },
                    "required": ["user_id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "create_note",
                "description": "Publish a short Note to your Substack feed.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "body": { "type": "string", "description": "Note text (1-500 chars)" }
                    },
                    "required": ["body"],
                    "additionalProperties": false
                }
            },
            {
                "name": "like_post",
                "description": "Like a post by numeric post ID.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "post_id": { "type": "string" }
                    },
                    "required": ["post_id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "list_drafts",
                "description": "List draft posts for the publication in SUBSTACK_PUBLICATION_URL.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "limit": { "type": "integer" },
                        "offset": { "type": "integer" }
                    },
                    "additionalProperties": false
                }
            },
            {
                "name": "create_draft",
                "description": "Create a newsletter draft on the publication in SUBSTACK_PUBLICATION_URL.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string" },
                        "body": { "type": "string" },
                        "audience": { "type": "string", "description": "everyone or paid" }
                    },
                    "required": ["title", "body"],
                    "additionalProperties": false
                }
            },
            {
                "name": "update_draft",
                "description": "Update an existing draft by ID.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "draft_id": { "type": "string" },
                        "title": { "type": "string" },
                        "body": { "type": "string" }
                    },
                    "required": ["draft_id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "publish_post",
                "description": "IRREVERSIBLE: sends email to all subscribers. Publishes a draft.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "draft_id": { "type": "string" },
                        "send_email": { "type": "boolean", "description": "Default true" }
                    },
                    "required": ["draft_id"],
                    "additionalProperties": false
                }
            },
            {
                "name": "upload_image",
                "description": "Submit a publicly accessible image URL to Substack. Substack fetches the image and returns its permanent CDN URL.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "image_url": { "type": "string", "description": "Publicly accessible https:// URL of the image to upload" }
                    },
                    "required": ["image_url"],
                    "additionalProperties": false
                }
            },
            {
                "name": "set_cover_image",
                "description": "Set the cover image of an existing draft using a URL (e.g. returned by upload_image).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "draft_id": { "type": "string" },
                        "image_url": { "type": "string", "description": "https:// URL of the image" }
                    },
                    "required": ["draft_id", "image_url"],
                    "additionalProperties": false
                }
            }
        ]
    })
}

pub fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    tools_validation::require_str(args, key)
}

pub fn optional_u64(args: &Value, key: &str, default: u64) -> u64 {
    args.get(key)
        .and_then(|v| v.as_u64())
        .unwrap_or(default)
}

pub fn optional_str<'a>(args: &'a Value, key: &str, default: &'a str) -> &'a str {
    args.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(default)
}

pub fn optional_bool(args: &Value, key: &str, default: bool) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
}

pub async fn call(state: Arc<ServerState>, name: &str, arguments: Value) -> Result<Value> {
    match name {
        "get_profile" => tool_get_profile(&state, &arguments).await,
        "get_posts" => tool_get_posts(&state, &arguments).await,
        "get_post" => tool_get_post(&state, &arguments).await,
        "get_comments" => tool_get_comments(&state, &arguments).await,
        "get_notes" => tool_get_notes(&state, &arguments).await,
        "create_note" => tool_create_note(&state, &arguments).await,
        "like_post" => tool_like_post(&state, &arguments).await,
        "list_drafts" => tool_list_drafts(&state, &arguments).await,
        "create_draft" => tool_create_draft(&state, &arguments).await,
        "update_draft" => tool_update_draft(&state, &arguments).await,
        "publish_post" => tool_publish_post(&state, &arguments).await,
        "upload_image" => tool_upload_image(&state, &arguments).await,
        "set_cover_image" => tool_set_cover_image(&state, &arguments).await,
        other => Err(anyhow!("unknown tool: {other}")),
    }
}

async fn tool_get_profile(state: &ServerState, args: &Value) -> Result<Value> {
    let slug = require_str(args, "slug")?;
    validate_slug(slug)?;
    state.client.get_profile(slug).await
}

async fn tool_get_posts(state: &ServerState, args: &Value) -> Result<Value> {
    let url = require_str(args, "publication_url")?;
    let host = validate_pub_url(url)?;
    let limit = validate_limit(optional_u64(args, "limit", 10))?;
    let offset = validate_offset(optional_u64(args, "offset", 0))?;
    state.client.get_posts(&host, limit, offset).await
}

async fn tool_get_post(state: &ServerState, args: &Value) -> Result<Value> {
    let url = require_str(args, "publication_url")?;
    let host = validate_pub_url(url)?;
    let post_slug = require_str(args, "post_slug")?;
    validate_post_slug(post_slug)?;
    state.client.get_post(&host, post_slug).await
}

async fn tool_get_comments(state: &ServerState, args: &Value) -> Result<Value> {
    let post_id = require_str(args, "post_id")?;
    validate_numeric_id(post_id, "post_id")?;
    let limit = validate_limit(optional_u64(args, "limit", 10))?;
    state.client.get_comments(post_id, limit).await
}

async fn tool_get_notes(state: &ServerState, args: &Value) -> Result<Value> {
    let user_id = require_str(args, "user_id")?;
    validate_numeric_id(user_id, "user_id")?;
    let limit = validate_limit(optional_u64(args, "limit", 10))?;
    let offset = validate_offset(optional_u64(args, "offset", 0))?;
    state.client.get_notes(user_id, limit, offset).await
}

async fn tool_create_note(state: &ServerState, args: &Value) -> Result<Value> {
    let body = require_str(args, "body")?;
    validate_note_body(body)?;
    state.client.create_note(body).await
}

async fn tool_like_post(state: &ServerState, args: &Value) -> Result<Value> {
    let post_id = require_str(args, "post_id")?;
    validate_numeric_id(post_id, "post_id")?;
    state.client.like_post(post_id).await
}

async fn tool_list_drafts(state: &ServerState, args: &Value) -> Result<Value> {
    let limit = validate_limit(optional_u64(args, "limit", 10))?;
    let offset = validate_offset(optional_u64(args, "offset", 0))?;
    state.client.list_drafts(limit, offset).await
}

async fn tool_create_draft(state: &ServerState, args: &Value) -> Result<Value> {
    let title = require_str(args, "title")?;
    let body = require_str(args, "body")?;
    validate_post_title(title)?;
    validate_post_body(body)?;
    let audience = optional_str(args, "audience", "everyone");
    validate_audience(audience)?;
    state.client.create_draft(title, body, audience).await
}

async fn tool_update_draft(state: &ServerState, args: &Value) -> Result<Value> {
    let draft_id = require_str(args, "draft_id")?;
    validate_numeric_id(draft_id, "draft_id")?;
    let title = args.get("title").and_then(|v| v.as_str());
    let body = args.get("body").and_then(|v| v.as_str());
    if title.is_none() && body.is_none() {
        return Err(anyhow!("at least one of title or body must be provided"));
    }
    if let Some(t) = title {
        validate_post_title(t)?;
    }
    if let Some(b) = body {
        validate_post_body(b)?;
    }
    state.client.update_draft(draft_id, title, body).await
}

async fn tool_publish_post(state: &ServerState, args: &Value) -> Result<Value> {
    let draft_id = require_str(args, "draft_id")?;
    validate_numeric_id(draft_id, "draft_id")?;
    let send_email = optional_bool(args, "send_email", true);
    state.client.publish_draft(draft_id, send_email).await
}

async fn tool_upload_image(state: &ServerState, args: &Value) -> Result<Value> {
    let image_url = require_str(args, "image_url")?;
    validate_image_url(image_url)?;
    let url = state.client.upload_image(image_url).await?;
    Ok(serde_json::json!({ "url": url }))
}

async fn tool_set_cover_image(state: &ServerState, args: &Value) -> Result<Value> {
    let draft_id = require_str(args, "draft_id")?;
    validate_numeric_id(draft_id, "draft_id")?;
    let image_url = require_str(args, "image_url")?;
    validate_image_url(image_url)?;
    state.client.set_draft_cover_image(draft_id, image_url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_count_is_13() {
        let list = tools_list();
        let tools = list.get("tools").and_then(|v| v.as_array()).unwrap();
        assert_eq!(tools.len(), 13);
    }

    #[test]
    fn tools_list_all_have_additional_properties_false() {
        let list = tools_list();
        let tools = list.get("tools").and_then(|v| v.as_array()).unwrap();
        for tool in tools {
            assert_eq!(
                tool.get("inputSchema")
                    .and_then(|s| s.get("additionalProperties")),
                Some(&json!(false))
            );
        }
    }

    #[test]
    fn publish_post_description_contains_irreversible() {
        let list = tools_list();
        let tools = list.get("tools").and_then(|v| v.as_array()).unwrap();
        let publish = tools
            .iter()
            .find(|t| t.get("name") == Some(&json!("publish_post")))
            .unwrap();
        let desc = publish.get("description").and_then(|v| v.as_str()).unwrap();
        assert!(desc.contains("IRREVERSIBLE"));
    }
}
