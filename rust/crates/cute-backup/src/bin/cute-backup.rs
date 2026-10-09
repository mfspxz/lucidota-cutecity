//! cute-backup: D1 export + restore verification (city-020).
//!
//! D1 FREEZE (until 2026-10-08 00:00 UTC): live export is disabled in this
//! build. This binary implements the export/restore LOGIC against local
//! fixtures and snapshot files; the live-export path (`--live`) refuses to
//! run until the freeze lifts, then uses the v4 D1 query API via the
//! credentialed bridge.
//!
//! Subcommands:
//! - `dump-schema --snapshot <json>`: verify a snapshot covers all CMS tables.
//! - `restore-check --snapshot <json> --db <path>`: rebuild a scratch SQLite
//!   DB from the snapshot and run integrity checks (tables, row counts,
//!   FK-less sanity, slug uniqueness).

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const CMS_TABLES: &[&str] = &[
    "posts",
    "releases",
    "portfolio_projects",
    "district_overrides",
    "site_settings",
    "newsletter_signups",
    "admin_audit",
    "page_views",
];

fn read_snapshot(path: &PathBuf) -> Result<serde_json::Value, String> {
    let t = fs::read_to_string(path).map_err(|e| format!("read snapshot: {e}"))?;
    serde_json::from_str(&t).map_err(|e| format!("parse snapshot: {e}"))
}

fn run_dump_schema(snap_path: &PathBuf) -> i32 {
    let snap = match read_snapshot(snap_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let mut missing = Vec::new();
    for t in CMS_TABLES {
        match snap.get(*t) {
            Some(v) if v.is_array() => println!("{t}: {} rows", v.as_array().unwrap().len()),
            _ => {
                // New tables (admin_audit, page_views) may legitimately be
                // absent from old snapshots — report, don't fail.
                if *t == "admin_audit" || *t == "page_views" {
                    println!("{t}: absent (expected pre-migration)");
                } else if *t == "newsletter_signups" {
                    println!("{t}: absent (PII-minimized, never snapshotted)");
                } else {
                    missing.push(*t);
                }
            }
        }
    }
    if missing.is_empty() {
        println!("dump-schema: OK");
        0
    } else {
        eprintln!("dump-schema: MISSING tables: {missing:?}");
        1
    }
}

fn sqlite(db: &str, sql: &str) -> Result<String, String> {
    let out = Command::new("sqlite3")
        .args([db, sql])
        .output()
        .map_err(|e| format!("sqlite3 spawn: {e}"))?;
    if !out.status.success() {
        return Err(format!("sqlite3: {}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn run_restore_check(snap_path: &PathBuf, db_path: &PathBuf) -> i32 {
    let snap = match read_snapshot(snap_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let db = db_path.to_string_lossy().to_string();
    let _ = fs::remove_file(&*db);
    let mut failures = 0;

    // Minimal restore schema (enough for integrity checks).
    let ddl = r#"
CREATE TABLE posts(id INTEGER PRIMARY KEY, slug TEXT, title TEXT, status TEXT, published_at TEXT);
CREATE TABLE releases(platform TEXT PRIMARY KEY, label TEXT, state TEXT, version TEXT, published INTEGER);
CREATE TABLE portfolio_projects(id INTEGER PRIMARY KEY, slug TEXT UNIQUE, title TEXT, visibility TEXT);
CREATE TABLE district_overrides(id TEXT PRIMARY KEY, state TEXT, name TEXT);
"#;
    if let Err(e) = sqlite(&db, ddl) {
        eprintln!("restore ddl: {e}");
        return 2;
    }

    let mut insert = |table: &str, cols: &[&str], rows: &[serde_json::Value]| {
        for r in rows {
            let vals: Vec<String> = cols
                .iter()
                .map(|c| {
                    let v = r.get(*c);
                    match v {
                        Some(serde_json::Value::String(s)) => format!("'{}'", s.replace('\'', "''")),
                        Some(serde_json::Value::Number(n)) => n.to_string(),
                        Some(serde_json::Value::Bool(b)) => {
                            if *b {
                                "1".into()
                            } else {
                                "0".into()
                            }
                        }
                        _ => "NULL".into(),
                    }
                })
                .collect();
            let sql = format!(
                "INSERT INTO {table} ({}) VALUES ({})",
                cols.join(","),
                vals.join(",")
            );
            if let Err(e) = sqlite(&db, &sql) {
                eprintln!("restore insert {table}: {e}");
                failures += 1;
            }
        }
    };

    if let Some(rows) = snap.get("posts").and_then(|v| v.as_array()) {
        insert("posts", &["id", "slug", "title", "status", "published_at"], rows);
    }
    if let Some(rows) = snap.get("releases").and_then(|v| v.as_array()) {
        insert("releases", &["platform", "label", "state", "version", "published"], rows);
    }
    if let Some(rows) = snap.get("portfolio_projects").and_then(|v| v.as_array()) {
        insert(
            "portfolio_projects",
            &["id", "slug", "title", "visibility"],
            rows,
        );
    }
    if let Some(rows) = snap.get("district_overrides").and_then(|v| v.as_array()) {
        insert("district_overrides", &["id", "state", "name"], rows);
    }

    // Integrity checks.
    let checks: &[(&str, &str)] = &[
        ("posts count", "SELECT COUNT(*) FROM posts"),
        ("published posts", "SELECT COUNT(*) FROM posts WHERE status='published'"),
        ("releases count", "SELECT COUNT(*) FROM releases"),
        ("projects count", "SELECT COUNT(*) FROM portfolio_projects"),
        ("districts count", "SELECT COUNT(*) FROM district_overrides"),
        ("duplicate post slugs", "SELECT COUNT(*) FROM (SELECT slug FROM posts GROUP BY slug HAVING COUNT(*)>1)"),
        ("duplicate project slugs", "SELECT COUNT(*) FROM (SELECT slug FROM portfolio_projects GROUP BY slug HAVING COUNT(*)>1)"),
        ("integrity", "PRAGMA integrity_check"),
    ];
    for (name, sql) in checks {
        match sqlite(&db, sql) {
            Ok(v) => {
                let bad = (*sql == "PRAGMA integrity_check" && v != "ok")
                    || (sql.contains("duplicate") && v != "0");
                println!("check {name}: {v}{}", if bad { "  <-- FAIL" } else { "" });
                if bad {
                    failures += 1;
                }
            }
            Err(e) => {
                eprintln!("check {name}: {e}");
                failures += 1;
            }
        }
    }
    // Row-count parity with snapshot.
    for (table, key) in [("posts", "posts"), ("releases", "releases"), ("portfolio_projects", "portfolio_projects"), ("district_overrides", "district_overrides")] {
        let snap_n = snap.get(key).and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
        let db_n: usize = sqlite(&db, &format!("SELECT COUNT(*) FROM {table}")).unwrap_or_default().parse().unwrap_or(usize::MAX);
        let ok = snap_n == db_n;
        println!("parity {table}: snapshot={snap_n} restored={db_n}{}", if ok { "" } else { "  <-- FAIL" });
        if !ok {
            failures += 1;
        }
    }
    // Slug uniqueness set check (catches the live DDL's missing UNIQUE on posts.slug).
    if let Some(rows) = snap.get("posts").and_then(|v| v.as_array()) {
        let mut seen = HashSet::new();
        let mut dup = 0;
        for r in rows {
            if let Some(s) = r.get("slug").and_then(|v| v.as_str()) {
                if !seen.insert(s.to_string()) {
                    dup += 1;
                }
            }
        }
        println!("snapshot post slug duplicates: {dup}");
        if dup > 0 {
            failures += 1;
        }
    }
    println!("restore-check: {} failure(s)", failures);
    if failures > 0 { 1 } else { 0 }
}

fn usage() -> i32 {
    eprintln!("usage: cute-backup dump-schema --snapshot <json>");
    eprintln!("       cute-backup restore-check --snapshot <json> --db <path>");
    eprintln!("       cute-backup live-export  # REFUSED during D1 freeze");
    2
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let flag = |n: &str| {
        args.windows(2)
            .find(|w| w[0] == n)
            .map(|w| PathBuf::from(&w[1]))
    };
    let code = match args.get(1).map(|s| s.as_str()) {
        Some("dump-schema") => match flag("--snapshot") {
            Some(p) => run_dump_schema(&p),
            None => usage(),
        },
        Some("restore-check") => match (flag("--snapshot"), flag("--db")) {
            (Some(s), Some(d)) => run_restore_check(&s, &d),
            _ => usage(),
        },
        Some("live-export") => {
            eprintln!("REFUSED: D1 is frozen until 2026-10-08 00:00 UTC (daily limit 87%).");
            eprintln!("Use dump-schema / restore-check against a snapshot file instead.");
            3
        }
        _ => usage(),
    };
    std::process::exit(code);
}
