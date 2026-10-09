//! D1 schema: canonical migration statements + the verified live schema.
//!
//! Existing tables (created pre-August 2026 by the original site build):
//! posts, releases (keyed by platform), portfolio_projects, district_overrides,
//! site_settings, newsletter_signups, newsletter_tokens, audit_events,
//! ledger, _cf_KV, d1_migrations.
//!
//! This module adds: `admin_audit` (city-007, richer than audit_events) and
//! `page_views` (city-019, aggregate-only analytics).

/// Live schema note (verified 2026-10-07 via v4 D1 API, table-level DDL):
/// - posts(id, slug, title, excerpt, body, tags TEXT, status, cover_image,
///   cover_alt, author, seo_title, seo_description, published_at, created_at,
///   updated_at). NOTE: slug is NOT unique in live DDL — dedupe enforced in
///   validation/client, not the DB.
/// - releases(platform TEXT PK, label, state, version, download_url,
///   checksum_url, signature_url, sort_order, published)
/// - portfolio_projects(id, slug, title, status, visibility, summary,
///   problem, approach, what_was_built, operator_role, results, limitations,
///   evidence_links, source_url, live_url, tags TEXT, client_permission,
///   sort_order, created_at, updated_at)
/// - district_overrides(id TEXT PK, state, name, blurb, href, updated_at,
///   updated_by)
/// - audit_events(id, actor_email, action, target, snapshot, created_at)
///   exists already; admin_audit adds outcome/detail/resource_id granularity.
pub const LIVE_SCHEMA_NOTE: &str = "verified 2026-10-07 via D1 query API";

/// Migration 001: CMS core. Idempotent (CREATE TABLE IF NOT EXISTS).
pub const MIGRATION_001_CMS_CORE: &str = r#"
CREATE TABLE IF NOT EXISTS posts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  slug TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  excerpt TEXT,
  body_markdown TEXT NOT NULL,
  published_at TEXT,
  status TEXT NOT NULL DEFAULT 'draft',
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_posts_status_pub ON posts(status, published_at);
CREATE INDEX IF NOT EXISTS idx_posts_slug ON posts(slug);

CREATE TABLE IF NOT EXISTS releases (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  slug TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  version TEXT NOT NULL,
  notes_markdown TEXT NOT NULL,
  released_at TEXT,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_releases_released ON releases(released_at);

CREATE TABLE IF NOT EXISTS portfolio_projects (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  slug TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  summary TEXT NOT NULL,
  body_markdown TEXT NOT NULL DEFAULT '',
  media_keys TEXT NOT NULL DEFAULT '[]',
  sort_order INTEGER NOT NULL DEFAULT 0,
  featured INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
CREATE INDEX IF NOT EXISTS idx_projects_sort ON portfolio_projects(sort_order);
"#;

/// Migration 002: admin audit log (city-007). Every admin mutation writes a row.
pub const MIGRATION_002_ADMIN_AUDIT: &str = r#"
CREATE TABLE IF NOT EXISTS admin_audit (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
  actor TEXT NOT NULL,
  action TEXT NOT NULL,
  resource TEXT NOT NULL,
  resource_id TEXT,
  outcome TEXT NOT NULL,
  detail TEXT
);
CREATE INDEX IF NOT EXISTS idx_audit_at ON admin_audit(at);
CREATE INDEX IF NOT EXISTS idx_audit_resource ON admin_audit(resource, resource_id);
"#;

/// Migration 003: private analytics (city-019). Aggregate-only: no IP, no UA,
/// no fingerprint. `bucket` is an hourly bucket; counts are increments.
pub const MIGRATION_003_PAGE_VIEWS: &str = r#"
CREATE TABLE IF NOT EXISTS page_views (
  path TEXT NOT NULL,
  bucket TEXT NOT NULL,
  views INTEGER NOT NULL DEFAULT 1,
  PRIMARY KEY (path, bucket)
);
"#;

/// All migrations in order. The caller applies each via the D1 API.
pub const MIGRATIONS: &[(&str, &str)] = &[
    ("001_cms_core", MIGRATION_001_CMS_CORE),
    ("002_admin_audit", MIGRATION_002_ADMIN_AUDIT),
    ("003_page_views", MIGRATION_003_PAGE_VIEWS),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent_and_ordered() {
        assert_eq!(MIGRATIONS.len(), 3);
        assert!(MIGRATIONS[0].0.starts_with("001"));
        for (name, sql) in MIGRATIONS {
            assert!(
                sql.contains("CREATE TABLE IF NOT EXISTS"),
                "{name} must be idempotent"
            );
        }
    }

    #[test]
    fn audit_table_has_fail_closed_columns() {
        let sql = MIGRATION_002_ADMIN_AUDIT;
        for col in ["actor", "action", "resource", "outcome"] {
            assert!(sql.contains(col), "missing audit column {col}");
        }
        assert!(sql.contains("NOT NULL"));
    }

    #[test]
    fn page_views_stores_no_pii_columns() {
        let sql = MIGRATION_003_PAGE_VIEWS.to_lowercase();
        for forbidden in ["ip", "user_agent", "fingerprint", "cookie"] {
            assert!(
                !sql.contains(forbidden),
                "analytics schema must not contain {forbidden}"
            );
        }
    }
}
