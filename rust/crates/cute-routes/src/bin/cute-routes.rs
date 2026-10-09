//! cute-routes: automated route verification for cutecity.cc (city-006).
//!
//! - `verify`: 13 public routes must 200; /nope must 404; admin routes must
//!   401/403 without a key; feed/sitemap/robots must 200.
//! - `accept-matrix`: Accept-header behavior matrix for one path (city-017).
//! - `timing`: TTFB + total time for key pages (city-018).
//!
//! HTTP via curl (std-only). JSON report on stdout; exit code 0 = all green.

use std::process::Command;

const UA: &str = "CuteCity-RouteVerifier/1.0";

/// The 13 public routes that must return 200 (extracted from the deployed
/// worker bundle's route table, 2026-10-07).
pub const ROUTES_200: &[&str] = &[
    "/",
    "/about",
    "/accessibility",
    "/blog",
    "/city",
    "/city/harbor",
    "/city/krampus",
    "/contact",
    "/faq",
    "/portfolio",
    "/privacy",
    "/releases",
    "/api/health",
];

pub const ROUTES_404: &[&str] = &["/nope", "/api/admin", "/definitely-not-here"];

/// Secret-gated admin routes: must 401/403 without a key (city-007).
pub const ROUTES_GATED: &[&str] = &[
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

pub const ROUTES_FEED: &[&str] = &["/feed.xml", "/sitemap.xml", "/robots.txt", "/manifest.webmanifest"];

#[derive(Debug)]
pub struct Probe {
    pub path: String,
    pub status: u16,
    pub content_type: String,
    pub total_ms: u128,
    pub ttfb_ms: u128,
    pub bytes: usize,
    pub accept: Option<String>,
}

fn curl_probe(base: &str, path: &str, accept: Option<&str>) -> Result<Probe, String> {
    let url = format!("{}{}", base.trim_end_matches('/'), path);
    let mut cmd = Command::new("curl");
    cmd.args([
        "-sS", "-o", "/tmp/cute-routes-body.tmp",
        "-D", "/tmp/cute-routes-head.tmp",
        "-w", "%{http_code} %{time_total} %{time_starttransfer} %{size_download}",
        "--max-time", "30",
        "-H", &format!("User-Agent: {UA}"),
    ]);
    if let Some(a) = accept {
        cmd.args(["-H", &format!("Accept: {a}")]);
    }
    cmd.arg(&url);
    let out = cmd.output().map_err(|e| format!("curl spawn: {e}"))?;
    if !out.status.success() {
        return Err(format!("curl failed for {url}"));
    }
    let meta = String::from_utf8_lossy(&out.stdout);
    let parts: Vec<&str> = meta.split_whitespace().collect();
    if parts.len() < 4 {
        return Err(format!("bad curl -w output for {url}"));
    }
    let status: u16 = parts[0].parse().map_err(|_| "bad status")?;
    let total_ms = (parts[1].parse::<f64>().unwrap_or(0.0) * 1000.0) as u128;
    let ttfb_ms = (parts[2].parse::<f64>().unwrap_or(0.0) * 1000.0) as u128;
    let bytes: usize = parts[3].parse().unwrap_or(0);
    let head = std::fs::read_to_string("/tmp/cute-routes-head.tmp").unwrap_or_default();
    let content_type = head
        .lines()
        .find(|l| l.to_lowercase().starts_with("content-type:"))
        .map(|l| l["content-type:".len()..].trim().to_string())
        .unwrap_or_default();
    Ok(Probe {
        path: path.to_string(),
        status,
        content_type,
        total_ms,
        ttfb_ms,
        bytes,
        accept: accept.map(|s| s.to_string()),
    })
}

/// POST probe without a key: returns the status code (auth boundary check).
fn post_probe(url: &str) -> Result<u16, String> {
    let out = Command::new("curl")
        .args([
            "-sS", "-o", "/dev/null",
            "-w", "%{http_code}",
            "--max-time", "30",
            "-X", "POST",
            "-H", &format!("User-Agent: {UA}"),
            "-H", "Content-Type: application/json",
            "-d", "{}",
            url,
        ])
        .output()
        .map_err(|e| format!("curl spawn: {e}"))?;
    String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse::<u16>()
        .map_err(|_| "bad status".to_string())
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn probe_json(p: &Probe) -> String {
    format!(
        "{{\"path\":\"{}\",\"status\":{},\"content_type\":\"{}\",\"total_ms\":{},\"ttfb_ms\":{},\"bytes\":{},\"accept\":{}}}",
        esc(&p.path),
        p.status,
        esc(&p.content_type),
        p.total_ms,
        p.ttfb_ms,
        p.bytes,
        p.accept.as_deref().map(|a| format!("\"{}\"", esc(a))).unwrap_or_else(|| "null".into()),
    )
}

fn run_verify(base: &str) -> i32 {
    let mut checks: Vec<String> = Vec::new();
    let mut failures = 0;
    let mut check = |name: &str, ok: bool, detail: String| {
        if !ok {
            failures += 1;
        }
        checks.push(format!(
            "{{\"name\":\"{}\",\"ok\":{},\"detail\":\"{}\"}}",
            esc(name),
            ok,
            esc(&detail)
        ));
    };

    for path in ROUTES_200 {
        match curl_probe(base, path, None) {
            Ok(p) => {
                let ok = p.status == 200;
                check(&format!("200 {path}"), ok, format!("got {}", p.status));
            }
            Err(e) => check(&format!("200 {path}"), false, e),
        }
    }
    for path in ROUTES_404 {
        match curl_probe(base, path, None) {
            Ok(p) => {
                let ok = p.status == 404;
                check(&format!("404 {path}"), ok, format!("got {}", p.status));
            }
            Err(e) => check(&format!("404 {path}"), false, e),
        }
    }
    // Gate checks: no method may 200 without a key. GET 401/403 = strict gate;
    // GET 405 = no GET handler (also closed). POST must 401/403, or 405 when
    // the route is already 401/403-gated on GET (e.g. /api/admin/arts).
    for path in ROUTES_GATED {
        let url = format!("{}{}", base.trim_end_matches('/'), path);
        let get_status = match curl_probe(base, path, None) {
            Ok(p) => Some(p.status),
            Err(_) => None,
        };
        let post_status = post_probe(&url).ok();
        let get_ok = matches!(get_status, Some(401) | Some(403) | Some(405));
        let post_ok = matches!(post_status, Some(401) | Some(403))
            || (matches!(get_status, Some(401) | Some(403)) && matches!(post_status, Some(405)));
        check(
            &format!("gated {path}"),
            get_ok && post_ok,
            format!("GET(no-key)={get_status:?} POST(no-key)={post_status:?}"),
        );
    }
    for path in ROUTES_FEED {
        match curl_probe(base, path, None) {
            Ok(p) => {
                let ok = p.status == 200;
                check(&format!("feed {path}"), ok, format!("got {}", p.status));
            }
            Err(e) => check(&format!("feed {path}"), false, e),
        }
    }
    println!(
        "{{\"base\":\"{}\",\"failures\":{failures},\"checks\":[{}]}}",
        esc(base),
        checks.join(",")
    );
    if failures > 0 { 1 } else { 0 }
}

fn run_accept_matrix(base: &str, path: &str) -> i32 {
    let accepts = [
        None,
        Some("text/html"),
        Some("application/json"),
        Some("application/json, text/plain, */*"),
        Some("*/*"),
    ];
    let mut probes = Vec::new();
    for a in accepts {
        match curl_probe(base, path, a) {
            Ok(p) => probes.push(probe_json(&p)),
            Err(e) => probes.push(format!("{{\"path\":\"{path}\",\"error\":\"{}\"}}", esc(&e))),
        }
    }
    // Inconsistency = different statuses across Accept headers for same path.
    println!(
        "{{\"base\":\"{}\",\"path\":\"{}\",\"probes\":[{}]}}",
        esc(base),
        esc(path),
        probes.join(",")
    );
    0
}

fn run_timing(base: &str) -> i32 {
    let paths = ["/", "/blog", "/portfolio", "/api/health", "/feed.xml"];
    let mut probes = Vec::new();
    for path in paths {
        match curl_probe(base, path, None) {
            Ok(p) => probes.push(probe_json(&p)),
            Err(e) => probes.push(format!("{{\"path\":\"{path}\",\"error\":\"{}\"}}", esc(&e))),
        }
    }
    println!("{{\"base\":\"{}\",\"timing\":[{}]}}", esc(base), probes.join(","));
    0
}

fn usage() -> i32 {
    eprintln!("usage: cute-routes verify --base <url>");
    eprintln!("       cute-routes accept-matrix --base <url> --path <path>");
    eprintln!("       cute-routes timing --base <url>");
    2
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let base = args
        .windows(2)
        .find(|w| w[0] == "--base")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| "https://cutecity.cc".to_string());
    let code = match args.get(1).map(|s| s.as_str()) {
        Some("verify") => run_verify(&base),
        Some("accept-matrix") => {
            let path = args
                .windows(2)
                .find(|w| w[0] == "--path")
                .map(|w| w[1].clone())
                .unwrap_or_else(|| "/api/feed".to_string());
            run_accept_matrix(&base, &path)
        }
        Some("timing") => run_timing(&base),
        _ => usage(),
    };
    std::process::exit(code);
}
