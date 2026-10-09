//! Content models for the Cute City CMS.
//!
//! Field-for-field match with D1 `cute-city-production` as of 2026-10-07
//! (schema pulled live via the v4 API; see `schema::LIVE_SCHEMA_NOTE`).
//! Where a storage format was not observable, the type records the raw TEXT
//! and the accessor documents the assumption as UNKNOWN.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Post {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub excerpt: String,
    pub body: String, // markdown
    pub tags_raw: String, // TEXT column; parse via tags()
    pub status: PostStatus,
    pub cover_image: String, // R2 key or URL; "" = none
    pub cover_alt: String,
    pub author: String,
    pub seo_title: String,
    pub seo_description: String,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl Post {
    /// Parse the tags column. UNKNOWN format: try JSON array, fall back to
    /// comma-split, then whitespace-split. Never fails — worst case returns [].
    pub fn tags(&self) -> Vec<String> {
        parse_tag_column(&self.tags_raw)
    }
}

/// Parse a tags TEXT column of unknown format.
pub fn parse_tag_column(raw: &str) -> Vec<String> {
    let t = raw.trim();
    if t.is_empty() {
        return vec![];
    }
    if t.starts_with('[') {
        if let Ok(v) = serde_json::from_str::<Vec<String>>(t) {
            return v.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }
    }
    let sep = if t.contains(',') { ',' } else { ' ' };
    t.split(sep)
        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PostStatus {
    Draft,
    Published,
    Archived,
}

impl Default for PostStatus {
    fn default() -> Self {
        PostStatus::Draft
    }
}

impl std::str::FromStr for PostStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "draft" => Ok(PostStatus::Draft),
            "published" => Ok(PostStatus::Published),
            "archived" => Ok(PostStatus::Archived),
            other => Err(format!("unknown post status: {other}")),
        }
    }
}

/// Releases are keyed by platform (e.g. "ios", "android", "web").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Release {
    pub platform: String,
    pub label: String,
    pub state: String, // e.g. "live", "beta", "retired"
    pub version: String,
    pub download_url: String,
    pub checksum_url: String,
    pub signature_url: String,
    pub sort_order: i64,
    pub published: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub status: String,     // e.g. "LAB", "LIVE"
    pub visibility: String, // "draft" | "public"
    pub summary: String,
    pub problem: String,
    pub approach: String,
    pub what_was_built: String,
    pub operator_role: String,
    pub results: String,
    pub limitations: String,
    pub evidence_links: String,
    pub source_url: String,
    pub live_url: String,
    pub tags_raw: String,
    pub client_permission: bool,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

impl Project {
    pub fn tags(&self) -> Vec<String> {
        parse_tag_column(&self.tags_raw)
    }

    /// R2 media keys referenced by this project (cover/evidence).
    /// UNKNOWN: no dedicated media column exists in the live schema, so media
    /// is discovered from evidence_links/source text at publish time.
    pub fn is_public(&self) -> bool {
        self.visibility == "public"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistrictOverride {
    pub id: String, // text pk, e.g. "harbor"
    pub state: String,
    pub name: String,
    pub blurb: String,
    pub href: String,
    pub updated_at: String,
    pub updated_by: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_column_json_array() {
        assert_eq!(
            parse_tag_column(r#"["dispatch","city-hall"]"#),
            vec!["dispatch".to_string(), "city-hall".to_string()]
        );
    }

    #[test]
    fn tag_column_comma_list() {
        assert_eq!(
            parse_tag_column("dispatch, city-hall"),
            vec!["dispatch".to_string(), "city-hall".to_string()]
        );
    }

    #[test]
    fn tag_column_empty_is_empty() {
        assert!(parse_tag_column("").is_empty());
        assert!(parse_tag_column("   ").is_empty());
    }

    #[test]
    fn tag_column_malformed_json_falls_back() {
        assert_eq!(parse_tag_column("[broken"), vec!["[broken".to_string()]);
    }

    #[test]
    fn project_public_flag() {
        let mut p = Project {
            id: 1,
            slug: "x".into(),
            title: "X".into(),
            status: "LAB".into(),
            visibility: "draft".into(),
            summary: "".into(),
            problem: "".into(),
            approach: "".into(),
            what_was_built: "".into(),
            operator_role: "".into(),
            results: "".into(),
            limitations: "".into(),
            evidence_links: "".into(),
            source_url: "".into(),
            live_url: "".into(),
            tags_raw: "".into(),
            client_permission: false,
            sort_order: 0,
            created_at: "".into(),
            updated_at: "".into(),
        };
        assert!(!p.is_public());
        p.visibility = "public".into();
        assert!(p.is_public());
    }
}
