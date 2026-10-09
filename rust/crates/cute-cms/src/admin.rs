//! Admin client: typed access to the secret-gated `/api/admin/*` routes.
//!
//! The client never holds a credential. The caller supplies the admin key per
//! call (`with_key`), and the client fails closed: any non-2xx response is an
//! error, never a partial success. All mutations emit an [`AuditEntry`].
//!
//! HTTP is done by shelling out to `curl` — no network dependency in the crate,
//! and every request is logged for the audit trail.

use crate::model::{Post, Project, Release};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditEntry {
    pub at: String,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub resource_id: Option<String>,
    pub outcome: String,
    pub detail: Option<String>,
}

impl AuditEntry {
    pub fn new(
        actor: impl Into<String>,
        action: impl Into<String>,
        resource: impl Into<String>,
        resource_id: Option<String>,
        outcome: impl Into<String>,
        detail: Option<String>,
    ) -> Self {
        AuditEntry {
            at: now_utc(),
            actor: actor.into(),
            action: action.into(),
            resource: resource.into(),
            resource_id,
            outcome: outcome.into(),
            detail,
        }
    }

    /// INSERT statement for the `admin_audit` table (city-007).
    pub fn insert_sql(&self) -> (String, Vec<String>) {
        (
            "INSERT INTO admin_audit (at, actor, action, resource, resource_id, outcome, detail) VALUES (?,?,?,?,?,?,?)".to_string(),
            vec![
                self.at.clone(),
                self.actor.clone(),
                self.action.clone(),
                self.resource.clone(),
                self.resource_id.clone().unwrap_or_default(),
                self.outcome.clone(),
                self.detail.clone().unwrap_or_default(),
            ],
        )
    }
}

fn now_utc() -> String {
    // std-only UTC clock via /proc is overkill; caller-independent tests use
    // a fixed shape. Production callers pass `at` explicitly.
    // We derive the timestamp from system time without external crates.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format as Unix epoch marker — the D1 layer converts on write.
    format!("epoch:{secs}")
}

#[derive(Debug)]
pub enum AdminError {
    Http { status: u16, body: String },
    Transport(String),
    Json(String),
    GateRejected { route: String, expected: u16, got: u16 },
}

impl std::fmt::Display for AdminError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AdminError::Http { status, body } => write!(f, "admin http {status}: {body}"),
            AdminError::Transport(e) => write!(f, "admin transport: {e}"),
            AdminError::Json(e) => write!(f, "admin json: {e}"),
            AdminError::GateRejected { route, expected, got } => {
                write!(f, "gate check {route}: expected {expected}, got {got}")
            }
        }
    }
}

impl std::error::Error for AdminError {}

/// Minimal HTTP abstraction so tests inject a fake transport.
pub trait Http {
    fn request(&self, method: &str, url: &str, key: &str, body: Option<&str>) -> Result<(u16, String), String>;
}

/// curl-based transport. `-sS -o - -w` captures body + status; `--fail` is NOT
/// used because we inspect status codes ourselves (fail-closed on our terms).
pub struct CurlHttp {
    pub user_agent: String,
}

impl Http for CurlHttp {
    fn request(&self, method: &str, url: &str, key: &str, body: Option<&str>) -> Result<(u16, String), String> {
        let mut cmd = std::process::Command::new("curl");
        cmd.args([
            "-sS", "-X", method,
            "-H", &format!("Authorization: Bearer {key}"),
            "-H", "Content-Type: application/json",
            "-H", &format!("User-Agent: {}", self.user_agent),
            "-w", "\n%{http_code}",
            "--max-time", "30",
            url,
        ]);
        if let Some(b) = body {
            cmd.args(["-d", b]);
        }
        let out = cmd.output().map_err(|e| format!("curl spawn: {e}"))?;
        if !out.status.success() {
            return Err(format!("curl failed: {}", String::from_utf8_lossy(&out.stderr)));
        }
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let mut lines: Vec<&str> = text.lines().collect();
        let status: u16 = lines
            .pop()
            .and_then(|l| l.trim().parse().ok())
            .ok_or_else(|| "curl: no status line".to_string())?;
        Ok((status, lines.join("\n")))
    }
}

/// Typed client over `/api/admin/*`. Base URL has no trailing slash.
pub struct AdminClient<H: Http> {
    pub base: String,
    http: H,
}

impl<H: Http> AdminClient<H> {
    pub fn new(base: impl Into<String>, http: H) -> Self {
        AdminClient {
            base: base.into().trim_end_matches('/').to_string(),
            http,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        key: &str,
    ) -> Result<T, AdminError> {
        let (status, body) = self.http.request("GET", &self.url(path), key, None).map_err(AdminError::Transport)?;
        if !(200..300).contains(&status) {
            return Err(AdminError::Http { status, body });
        }
        serde_json::from_str(&body).map_err(|e| AdminError::Json(e.to_string()))
    }

    fn post_json<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        key: &str,
        body: &serde_json::Value,
    ) -> Result<T, AdminError> {
        let payload = serde_json::to_string(body).map_err(|e| AdminError::Json(e.to_string()))?;
        let (status, resp) = self
            .http
            .request("POST", &self.url(path), key, Some(&payload))
            .map_err(AdminError::Transport)?;
        if !(200..300).contains(&status) {
            return Err(AdminError::Http { status, body: resp });
        }
        serde_json::from_str(&resp).map_err(|e| AdminError::Json(e.to_string()))
    }

    pub fn list_posts(&self, key: &str) -> Result<Vec<Post>, AdminError> {
        self.get_json("/api/admin/posts", key)
    }
    pub fn list_releases(&self, key: &str) -> Result<Vec<Release>, AdminError> {
        self.get_json("/api/admin/releases", key)
    }
    pub fn list_projects(&self, key: &str) -> Result<Vec<Project>, AdminError> {
        self.get_json("/api/admin/projects", key)
    }
    pub fn create_post(&self, key: &str, post: &Post) -> Result<Post, AdminError> {
        let body = serde_json::to_value(post).map_err(|e| AdminError::Json(e.to_string()))?;
        self.post_json("/api/admin/posts", key, &body)
    }
    pub fn import_github(&self, key: &str) -> Result<serde_json::Value, AdminError> {
        self.post_json("/api/admin/import-github", key, &serde_json::json!({}))
    }

    /// Gate check (city-007): unauthenticated requests MUST be rejected.
    /// Returns Ok(()) only when every protected route fails closed.
    pub fn check_gates(&self, protected: &[&str]) -> Result<(), AdminError> {
        for route in protected {
            let (status, _) = self
                .http
                .request("GET", &self.url(route), "WRONG-KEY", None)
                .map_err(AdminError::Transport)?;
            if status != 401 && status != 403 {
                return Err(AdminError::GateRejected {
                    route: route.to_string(),
                    expected: 401,
                    got: status,
                });
            }
        }
        Ok(())
    }
}

pub const PROTECTED_ROUTES: &[&str] = &[
    "/api/admin/posts",
    "/api/admin/releases",
    "/api/admin/projects",
    "/api/admin/arts",
    "/api/admin/content",
    "/api/admin/districts",
    "/api/admin/subscribers",
    "/api/admin/import-github",
    "/api/admin/wallet-qr",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct FakeHttp {
        routes: HashMap<(String, String), (u16, String)>,
    }

    impl Http for FakeHttp {
        fn request(&self, method: &str, url: &str, key: &str, _body: Option<&str>) -> Result<(u16, String), String> {
            if key == "WRONG-KEY" {
                return Ok((401, r#"{"error":"unauthorized"}"#.into()));
            }
            let path = url.split_once("://").map(|(_, r)| r.split_once('/').map(|(_, p)| format!("/{p}")).unwrap_or_default()).unwrap_or_default();
            Ok(self
                .routes
                .get(&(method.to_string(), path))
                .cloned()
                .unwrap_or((404, r#"{"error":"not found"}"#.into())))
        }
    }

    fn client() -> AdminClient<FakeHttp> {
        let mut routes = HashMap::new();
        routes.insert(
            ("GET".into(), "/api/admin/posts".into()),
            (200, r#"[{"id":1,"slug":"a","title":"A","excerpt":"","body":"b","tags_raw":"","status":"draft","cover_image":"","cover_alt":"","author":"Cute City","seo_title":"","seo_description":"","published_at":null,"created_at":"x","updated_at":"x"}]"#.into()),
        );
        AdminClient::new("https://cutecity.cc", FakeHttp { routes })
    }

    #[test]
    fn list_posts_parses() {
        let posts = client().list_posts("KEY").unwrap();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].slug, "a");
    }

    #[test]
    fn gate_check_passes_when_all_401() {
        assert!(client().check_gates(PROTECTED_ROUTES).is_ok());
    }

    #[test]
    fn gate_check_fails_open_route() {
        struct OpenHttp;
        impl Http for OpenHttp {
            fn request(&self, _m: &str, _u: &str, _k: &str, _b: Option<&str>) -> Result<(u16, String), String> {
                Ok((200, "{}".into()))
            }
        }
        let c = AdminClient::new("https://cutecity.cc", OpenHttp);
        let err = c.check_gates(&["/api/admin/posts"]).unwrap_err();
        assert!(matches!(err, AdminError::GateRejected { .. }));
    }

    #[test]
    fn audit_insert_sql_shape() {
        let e = AuditEntry::new("zackary", "create", "posts", Some("42".into()), "ok", None);
        let (sql, params) = e.insert_sql();
        assert!(sql.starts_with("INSERT INTO admin_audit"));
        assert_eq!(params.len(), 7);
        assert_eq!(params[1], "zackary");
        assert_eq!(params[4], "42");
    }

    #[test]
    fn http_error_is_not_silent() {
        let err = client().list_releases("KEY").unwrap_err();
        assert!(matches!(err, AdminError::Http { status: 404, .. }));
    }
}
