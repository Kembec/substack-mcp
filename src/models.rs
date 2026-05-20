#![allow(dead_code)]

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: u64,
    pub name: String,
    pub handle: Option<String>,
    pub bio: Option<String>,
    pub photo_url: Option<String>,
    pub follower_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub id: u64,
    pub title: String,
    pub slug: String,
    pub published_at: Option<String>,
    pub canonical_url: Option<String>,
    pub description: Option<String>,
    pub comment_count: Option<u64>,
    pub audience: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: u64,
    pub body: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Comment {
    pub id: u64,
    pub body: Option<String>,
    pub author_id: Option<u64>,
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub id: u64,
    pub draft_title: Option<String>,
    pub audience: Option<String>,
}
