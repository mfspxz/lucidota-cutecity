//! Validation: fail-closed content checks. Nothing enters the CMS that fails these.

use crate::model::{DistrictOverride, Post, PostStatus, Project, Release};
use crate::slug::is_valid_slug;

#[derive(Debug, PartialEq)]
pub enum ValidationError {
    Empty(&'static str),
    TooLong { field: &'static str, max: usize, got: usize },
    BadSlug(String),
    BadStatus(String),
    BadTimestamp { field: &'static str, value: String },
    MissingPlatform,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::Empty(field) => write!(f, "{field} must not be empty"),
            ValidationError::TooLong { field, max, got } => {
                write!(f, "{field} too long: {got} > {max}")
            }
            ValidationError::BadSlug(s) => write!(f, "bad slug: {s}"),
            ValidationError::BadStatus(s) => write!(f, "bad status: {s}"),
            ValidationError::BadTimestamp { field, value } => {
                write!(f, "{field} has unparseable timestamp: {value}")
            }
            ValidationError::MissingPlatform => write!(f, "release platform must not be empty"),
        }
    }
}

impl std::error::Error for ValidationError {}

fn check_len(field: &'static str, v: &str, max: usize, errs: &mut Vec<ValidationError>) {
    if v.len() > max {
        errs.push(ValidationError::TooLong { field, max, got: v.len() });
    }
}

/// Accepts RFC3339 (2026-10-06T12:00:00Z) or SQLite datetimes
/// (2026-10-06 12:00:00). Returns false for anything else — no guessing.
pub fn is_valid_timestamp(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 16 {
        return false;
    }
    let b = t.as_bytes();
    let digits = |i: usize| b.get(i).map(|c| c.is_ascii_digit()).unwrap_or(false);
    for i in [0, 1, 2, 3, 5, 6, 8, 9] {
        if !digits(i) {
            return false;
        }
    }
    if b.get(4) != Some(&b'-') || b.get(7) != Some(&b'-') {
        return false;
    }
    let sep = b.get(10).copied().unwrap_or(b' ');
    if sep != b'T' && sep != b' ' {
        return false;
    }
    for i in [11, 12, 14, 15, 17, 18] {
        if !digits(i) {
            return false;
        }
    }
    b.get(13) == Some(&b':') && b.get(16) == Some(&b':')
}

pub fn validate_post(p: &Post) -> Vec<ValidationError> {
    let mut errs = Vec::new();
    if p.title.trim().is_empty() {
        errs.push(ValidationError::Empty("title"));
    }
    check_len("title", &p.title, 200, &mut errs);
    if !is_valid_slug(&p.slug) {
        errs.push(ValidationError::BadSlug(p.slug.clone()));
    }
    if p.body.trim().is_empty() {
        errs.push(ValidationError::Empty("body"));
    }
    check_len("excerpt", &p.excerpt, 400, &mut errs);
    check_len("seo_title", &p.seo_title, 70, &mut errs);
    check_len("seo_description", &p.seo_description, 160, &mut errs);
    if p.status != PostStatus::Draft && p.published_at.is_none() {
        errs.push(ValidationError::Empty("published_at (required when not draft)"));
    }
    if let Some(ts) = &p.published_at {
        if !is_valid_timestamp(ts) {
            errs.push(ValidationError::BadTimestamp {
                field: "published_at",
                value: ts.clone(),
            });
        }
    }
    errs
}

pub fn validate_release(r: &Release) -> Vec<ValidationError> {
    let mut errs = Vec::new();
    if r.platform.trim().is_empty() {
        errs.push(ValidationError::MissingPlatform);
    }
    if r.label.trim().is_empty() {
        errs.push(ValidationError::Empty("label"));
    }
    if r.state.trim().is_empty() {
        errs.push(ValidationError::Empty("state"));
    }
    if r.published && r.version.trim().is_empty() {
        errs.push(ValidationError::Empty("version (required when published)"));
    }
    errs
}

pub fn validate_project(p: &Project) -> Vec<ValidationError> {
    let mut errs = Vec::new();
    if p.title.trim().is_empty() {
        errs.push(ValidationError::Empty("title"));
    }
    if !is_valid_slug(&p.slug) {
        errs.push(ValidationError::BadSlug(p.slug.clone()));
    }
    if p.summary.trim().is_empty() {
        errs.push(ValidationError::Empty("summary"));
    }
    check_len("summary", &p.summary, 300, &mut errs);
    if !["draft", "public"].contains(&p.visibility.as_str()) {
        errs.push(ValidationError::BadStatus(p.visibility.clone()));
    }
    errs
}

pub fn validate_district(d: &DistrictOverride) -> Vec<ValidationError> {
    let mut errs = Vec::new();
    if d.id.trim().is_empty() {
        errs.push(ValidationError::Empty("id"));
    }
    if d.href.trim().is_empty() {
        errs.push(ValidationError::Empty("href"));
    }
    errs
}

/// Voice-pass gate: content may only move staging -> prod when the
/// `voice_passed` flag is true AND a `passed_by` actor is recorded.
/// Anything else fails closed.
pub fn voice_pass_ok(voice_passed: bool, passed_by: Option<&str>) -> bool {
    voice_passed && passed_by.map(|s| !s.trim().is_empty()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn post() -> Post {
        Post {
            id: 1,
            slug: "hello-city".into(),
            title: "Hello City".into(),
            excerpt: "Short.".into(),
            body: "# Hi\n\nBody.".into(),
            tags_raw: "dispatch".into(),
            status: PostStatus::Published,
            cover_image: "".into(),
            cover_alt: "".into(),
            author: "Cute City".into(),
            seo_title: "Hello City".into(),
            seo_description: "Hi.".into(),
            published_at: Some("2026-10-06T12:00:00Z".into()),
            created_at: "2026-10-06 12:00:00".into(),
            updated_at: "2026-10-06 12:00:00".into(),
        }
    }

    #[test]
    fn valid_post_passes() {
        assert!(validate_post(&post()).is_empty());
    }

    #[test]
    fn post_collects_all_errors() {
        let mut p = post();
        p.title.clear();
        p.slug = "Bad Slug!".into();
        p.body = "   ".into();
        p.published_at = Some("yesterday".into());
        let errs = validate_post(&p);
        assert_eq!(errs.len(), 4);
        assert!(errs.contains(&ValidationError::Empty("title")));
        assert!(errs.contains(&ValidationError::BadSlug("Bad Slug!".into())));
        assert!(errs.contains(&ValidationError::Empty("body")));
    }

    #[test]
    fn published_without_timestamp_fails() {
        let mut p = post();
        p.published_at = None;
        assert!(validate_post(&p).contains(&ValidationError::Empty(
            "published_at (required when not draft)"
        )));
        p.status = PostStatus::Draft;
        assert!(validate_post(&p).is_empty());
    }

    #[test]
    fn sqlite_and_rfc3339_timestamps_accepted() {
        assert!(is_valid_timestamp("2026-08-18 07:14:01"));
        assert!(is_valid_timestamp("2026-08-18T07:14:01Z"));
        assert!(is_valid_timestamp("2026-08-18T07:14:01.000+00:00"));
        assert!(!is_valid_timestamp("yesterday"));
        assert!(!is_valid_timestamp("18/08/2026"));
        assert!(!is_valid_timestamp(""));
    }

    #[test]
    fn release_platform_key() {
        let mut r = Release {
            platform: "".into(),
            label: "iOS".into(),
            state: "live".into(),
            version: "1.0".into(),
            download_url: "".into(),
            checksum_url: "".into(),
            signature_url: "".into(),
            sort_order: 0,
            published: true,
        };
        assert!(validate_release(&r).contains(&ValidationError::MissingPlatform));
        r.platform = "ios".into();
        assert!(validate_release(&r).is_empty());
    }

    #[test]
    fn project_visibility_whitelist() {
        let mut p = Project {
            id: 1,
            slug: "krampus-express".into(),
            title: "Krampus Express".into(),
            status: "LAB".into(),
            visibility: "semi-public".into(),
            summary: "Scrapers.".into(),
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
        assert!(validate_project(&p).iter().any(|e| matches!(e, ValidationError::BadStatus(_))));
        p.visibility = "public".into();
        assert!(validate_project(&p).is_empty());
    }

    #[test]
    fn voice_pass_gate_fails_closed() {
        assert!(voice_pass_ok(true, Some("Zackary Camoran")));
        assert!(!voice_pass_ok(true, None));
        assert!(!voice_pass_ok(true, Some("  ")));
        assert!(!voice_pass_ok(false, Some("Zackary Camoran")));
    }

    #[test]
    fn status_roundtrip() {
        assert_eq!("published".parse::<PostStatus>().unwrap(), PostStatus::Published);
        assert!("bogus".parse::<PostStatus>().is_err());
    }
}
