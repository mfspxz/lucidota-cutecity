//! cute-pub: static publisher (city-002, city-016, city-023).
//!
//! Reads JSON snapshots on stdin, writes deterministic static artifacts to
//! --out: feed.json (JSON Feed 1.1), feed.xml (RSS), v1/*.json (public API),
//! sitemap.xml, and per-page OG meta snippets.
//!
//! Input comes from a frozen D1 snapshot or public HTTP — never fabricated.

use cute_cms::model::parse_tag_column;
use cute_cms::slug::is_valid_slug;
use cute_cms::validate::is_valid_timestamp;
use cute_feed::{json_feed, rss_feed, FeedItem, FeedMeta, Page};
use std::fs;
use std::path::PathBuf;

/// Normalize a D1/SQLite timestamp ("2026-08-16 00:00:00") or RFC3339 to
/// RFC3339 UTC. Returns None for unparseable input (caller omits the field).
pub fn to_rfc3339(s: &str) -> Option<String> {
    let t = s.trim();
    if !is_valid_timestamp(t) {
        return None;
    }
    let mut r = t.replace(' ', "T");
    if !r.ends_with('Z') && !r.contains('+') {
        r.push('Z');
    }
    Some(r)
}

#[derive(Debug, serde::Deserialize)]
struct Snapshot {
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    posts: Vec<PostRow>,
    #[serde(default)]
    releases: Vec<serde_json::Value>,
    #[serde(default)]
    portfolio_projects: Vec<serde_json::Value>,
    #[serde(default)]
    district_overrides: Vec<serde_json::Value>,
}

#[derive(Debug, serde::Deserialize)]
struct PostRow {
    #[serde(default)]
    id: i64,
    #[serde(default)]
    slug: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    excerpt: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    tags: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    published_at: Option<String>,
}

fn feed_items(snap: &Snapshot) -> Vec<FeedItem> {
    let base = snap.base_url.trim_end_matches('/');
    let mut items: Vec<FeedItem> = snap
        .posts
        .iter()
        .filter(|p| p.status == "published" && is_valid_slug(&p.slug))
        .filter_map(|p| {
            let url = format!("{base}/blog/{}", p.slug);
            let date_published = p.published_at.as_deref().and_then(to_rfc3339);
            // Content: prefer excerpt, then first 300 chars of body, else omit.
            let summary = if !p.excerpt.trim().is_empty() {
                Some(p.excerpt.trim().to_string())
            } else if !p.body.trim().is_empty() {
                let b: String = p.body.chars().take(300).collect();
                Some(b)
            } else {
                None
            };
            Some(FeedItem {
                id: url.clone(),
                url,
                title: p.title.clone(),
                content_html: None,
                content_text: if p.body.trim().is_empty() { None } else { Some(p.body.clone()) },
                summary,
                date_published,
                date_modified: None,
                image: None,
                tags: parse_tag_column(&p.tags),
                author_name: Some("Cute City".into()),
            })
        })
        .collect();
    // Newest first.
    items.sort_by(|a, b| b.date_published.cmp(&a.date_published));
    items
}

fn feed_meta(snap: &Snapshot) -> FeedMeta {
    let base = snap.base_url.trim_end_matches('/').to_string();
    FeedMeta {
        title: "Cute City — Dispatches".into(),
        home_page_url: format!("{base}/blog"),
        feed_url: format!("{base}/feed.json"),
        description: "Street dispatches from Cute City.".into(),
        author_name: "Zackary Camoran".into(),
    }
}

fn write(out: &PathBuf, name: &str, content: &str) -> Result<(), String> {
    let p = out.join(name);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    fs::write(&p, content).map_err(|e| format!("write {}: {e}", p.display()))?;
    println!("wrote {}", p.display());
    Ok(())
}

fn run_feed(snap: &Snapshot, out: &PathBuf) -> Result<(), String> {
    let meta = feed_meta(snap);
    let items = feed_items(snap);
    let page = Page { number: 1, size: 20 };
    let page_items = page.slice(&items);
    let total_pages = page.total_pages(items.len());
    write(out, "feed.json", &json_feed(&meta, &page_items).map_err(|e| e.to_string())?)?;
    write(out, "feed.xml", &rss_feed(&meta, &page_items).map_err(|e| e.to_string())?)?;
    // Paginated overflow pages (only when needed).
    for n in 2..=total_pages {
        let p = Page { number: n, size: 20 };
        let sub = p.slice(&items);
        let mut m = meta.clone();
        m.feed_url = format!("{}/feed-{n}.json", snap.base_url.trim_end_matches('/'));
        write(out, &format!("feed-{n}.json"), &json_feed(&m, &sub).map_err(|e| e.to_string())?)?;
    }
    println!("feed: {} published posts, {} page(s)", items.len(), total_pages.max(1));
    Ok(())
}

fn run_api(snap: &Snapshot, out: &PathBuf) -> Result<(), String> {
    // v1/posts.json — public read shape (documented in PUBLIC-API.md).
    let posts: Vec<serde_json::Value> = feed_items(snap)
        .iter()
        .map(|i| {
            serde_json::json!({
                "id": i.id, "url": i.url, "title": i.title,
                "summary": i.summary, "date_published": i.date_published,
                "tags": i.tags,
            })
        })
        .collect();
    write(out, "v1/posts.json", &serde_json::to_string_pretty(&serde_json::json!({
        "version": "v1", "count": posts.len(), "posts": posts,
    })).unwrap())?;
    write(out, "v1/releases.json", &serde_json::to_string_pretty(&serde_json::json!({
        "version": "v1", "count": snap.releases.len(), "releases": snap.releases,
    })).unwrap())?;
    write(out, "v1/projects.json", &serde_json::to_string_pretty(&serde_json::json!({
        "version": "v1", "count": snap.portfolio_projects.len(), "projects": snap.portfolio_projects,
    })).unwrap())?;
    write(out, "v1/districts.json", &serde_json::to_string_pretty(&serde_json::json!({
        "version": "v1", "count": snap.district_overrides.len(), "districts": snap.district_overrides,
    })).unwrap())?;
    Ok(())
}

fn xml_esc(s: &str) -> String {
    cute_feed::xml_escape(s)
}

fn run_sitemap(snap: &Snapshot, out: &PathBuf) -> Result<(), String> {
    let base = snap.base_url.trim_end_matches('/');
    let mut urls = vec![
        (format!("{base}/"), None),
        (format!("{base}/about"), None),
        (format!("{base}/blog"), None),
        (format!("{base}/portfolio"), None),
        (format!("{base}/releases"), None),
        (format!("{base}/contact"), None),
        (format!("{base}/faq"), None),
        (format!("{base}/privacy"), None),
        (format!("{base}/accessibility"), None),
    ];
    for p in &snap.posts {
        if p.status == "published" && is_valid_slug(&p.slug) {
            let lastmod = p.published_at.as_deref().and_then(to_rfc3339);
            urls.push((format!("{base}/blog/{}", p.slug), lastmod));
        }
    }
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    for (loc, lastmod) in urls {
        xml.push_str(&format!("  <url><loc>{}</loc>", xml_esc(&loc)));
        if let Some(lm) = lastmod {
            xml.push_str(&format!("<lastmod>{}</lastmod>", xml_esc(&lm)));
        }
        xml.push_str("</url>\n");
    }
    xml.push_str("</urlset>\n");
    write(out, "sitemap.xml", &xml)?;
    Ok(())
}

/// city-023: per-page OG/social-card meta snippets (staged for voice pass).
fn run_og(snap: &Snapshot, out: &PathBuf) -> Result<(), String> {
    let base = snap.base_url.trim_end_matches('/');
    let mut pages: Vec<(String, String, String)> = vec![
        ("index".into(), "Cute City".into(), "Street dispatches from Cute City.".into()),
        ("blog".into(), "Dispatches — Cute City".into(), "Street dispatches from Cute City.".into()),
        ("portfolio".into(), "Portfolio — Cute City".into(), "Things built in the open.".into()),
        ("releases".into(), "Releases — Cute City".into(), "Release windows.".into()),
    ];
    for p in &snap.posts {
        if p.status == "published" && is_valid_slug(&p.slug) {
            pages.push((
                format!("blog-{}", p.slug),
                p.title.clone(),
                p.excerpt.chars().take(160).collect(),
            ));
        }
    }
    for (name, title, desc) in &pages {
        let snippet = format!(
            r#"<!-- OG/social card: {name} (STAGED — voice pass pending) -->
<meta property="og:title" content="{title}" />
<meta property="og:description" content="{desc}" />
<meta property="og:type" content="website" />
<meta property="og:url" content="{base}/{name}" />
<meta property="og:image" content="{base}/cute-city-og.png" />
<meta name="twitter:card" content="summary_large_image" />
<meta name="twitter:title" content="{title}" />
<meta name="twitter:description" content="{desc}" />
<meta name="twitter:image" content="{base}/cute-city-og.png" />
"#,
            title = xml_esc(title),
            desc = xml_esc(desc),
            name = xml_esc(name),
            base = xml_esc(base),
        );
        write(out, &format!("og/{name}.html"), &snippet)?;
    }
    Ok(())
}

fn usage() -> i32 {
    eprintln!("usage: cute-pub <feed|api|sitemap|og|all> --out <dir> < snapshot.json");
    2
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args
        .windows(2)
        .find(|w| w[0] == "--out")
        .map(|w| PathBuf::from(&w[1]));
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let out = match out {
        Some(o) => o,
        None => {
            usage();
            std::process::exit(2);
        }
    };
    let mut input = String::new();
    if std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).is_err() {
        eprintln!("failed to read stdin");
        std::process::exit(2);
    }
    let snap: Snapshot = match serde_json::from_str(&input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("bad snapshot JSON: {e}");
            std::process::exit(2);
        }
    };
    let result = match cmd {
        "feed" => run_feed(&snap, &out),
        "api" => run_api(&snap, &out),
        "sitemap" => run_sitemap(&snap, &out),
        "og" => run_og(&snap, &out),
        "all" => run_feed(&snap, &out)
            .and_then(|_| run_api(&snap, &out))
            .and_then(|_| run_sitemap(&snap, &out))
            .and_then(|_| run_og(&snap, &out)),
        _ => {
            usage();
            std::process::exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("cute-pub error: {e}");
        std::process::exit(1);
    }
}
