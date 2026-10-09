//! JSON Feed 1.1 + RSS 2.0 generation for Cute City dispatches.
//!
//! Pure functions: content models in, serialized feed out. Pagination is
//! slice-based so the caller (cute-pub / D1 reader) controls the page size.

use serde::{Deserialize, Serialize};

/// One feed entry. Fields kept minimal and honest: no field is fabricated —
/// missing optional data stays absent from the JSON.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeedItem {
    pub id: String,
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_html: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_published: Option<String>, // RFC3339
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_modified: Option<String>, // RFC3339
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FeedMeta {
    pub title: String,
    pub home_page_url: String,
    pub feed_url: String,
    pub description: String,
    pub author_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    pub number: u32, // 1-based
    pub size: u32,
}

impl Page {
    pub fn slice<T: Clone>(&self, items: &[T]) -> Vec<T> {
        let start = ((self.number.saturating_sub(1)) * self.size) as usize;
        items.iter().skip(start).take(self.size as usize).cloned().collect()
    }

    /// Total pages for `total` items at this page size. 0 items -> 0 pages.
    pub fn total_pages(&self, total: usize) -> u32 {
        if total == 0 || self.size == 0 {
            return 0;
        }
        ((total + self.size as usize - 1) / self.size as usize) as u32
    }
}

#[derive(Debug)]
pub enum FeedError {
    MissingMeta(&'static str),
}

impl std::fmt::Display for FeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeedError::MissingMeta(field) => {
                write!(f, "feed meta missing required field: {field}")
            }
        }
    }
}

impl std::error::Error for FeedError {}

/// Build a JSON Feed 1.1 document (https://www.jsonfeed.org/version/1.1/).
pub fn json_feed(meta: &FeedMeta, items: &[FeedItem]) -> Result<String, FeedError> {
    if meta.title.is_empty() {
        return Err(FeedError::MissingMeta("title"));
    }
    if meta.feed_url.is_empty() {
        return Err(FeedError::MissingMeta("feed_url"));
    }
    #[derive(Serialize)]
    struct Author<'a> {
        name: &'a str,
    }
    #[derive(Serialize)]
    struct Doc<'a> {
        version: &'a str,
        title: &'a str,
        home_page_url: &'a str,
        feed_url: &'a str,
        description: &'a str,
        authors: Vec<Author<'a>>,
        items: &'a [FeedItem],
    }
    let doc = Doc {
        version: "https://jsonfeed.org/version/1.1",
        title: &meta.title,
        home_page_url: &meta.home_page_url,
        feed_url: &meta.feed_url,
        description: &meta.description,
        authors: vec![Author {
            name: &meta.author_name,
        }],
        items,
    };
    Ok(serde_json::to_string_pretty(&doc).expect("feed serialization"))
}

/// Escape XML special chars. Minimal, correct for the fields we emit.
pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Build an RSS 2.0 document (mirrors the live /feed.xml shape).
pub fn rss_feed(meta: &FeedMeta, items: &[FeedItem]) -> Result<String, FeedError> {
    if meta.title.is_empty() {
        return Err(FeedError::MissingMeta("title"));
    }
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\">\n  <channel>\n");
    out.push_str(&format!(
        "    <title>{}</title>\n    <link>{}</link>\n    <description>{}</description>\n",
        xml_escape(&meta.title),
        xml_escape(&meta.home_page_url),
        xml_escape(&meta.description),
    ));
    for item in items {
        out.push_str("    <item>\n");
        out.push_str(&format!("      <title>{}</title>\n", xml_escape(&item.title)));
        out.push_str(&format!(
            "      <link>{}</link>\n      <guid isPermaLink=\"true\">{}</guid>\n",
            xml_escape(&item.url),
            xml_escape(&item.id)
        ));
        if let Some(d) = &item.date_published {
            out.push_str(&format!("      <pubDate>{}</pubDate>\n", xml_escape(d)));
        }
        if let Some(s) = item.summary.as_ref().or(item.content_text.as_ref()) {
            out.push_str(&format!("      <description>{}</description>\n", xml_escape(s)));
        }
        out.push_str("    </item>\n");
    }
    out.push_str("  </channel>\n</rss>\n");
    Ok(out)
}

/// Convert an RFC3339 timestamp to an HTTP-date-ish pubDate string.
/// Returns None for unparseable input rather than guessing.
pub fn rfc3339_to_http_date(s: &str) -> Option<String> {
    // Minimal parser: YYYY-MM-DDTHH:MM:SSZ or with +00:00 offset.
    let s = s.trim_end_matches('Z');
    let s = s.split(['+', '.']).next().unwrap_or(s);
    let (date, time) = s.split_once('T')?;
    let mut dp = date.split('-');
    let (y, m, d) = (dp.next()?, dp.next()?, dp.next()?);
    let mut tp = time.split(':');
    let (hh, mm, ss) = (tp.next()?, tp.next()?, tp.next()?);
    // Validate numeric ranges loosely.
    for part in [y, m, d, hh, mm, ss] {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
    }
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let mi: usize = m.parse().ok()?;
    if !(1..=12).contains(&mi) {
        return None;
    }
    Some(format!(
        "{d} {} {y} {hh}:{mm}:{ss} GMT",
        months[mi - 1]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> FeedMeta {
        FeedMeta {
            title: "Cute City — Dispatches".into(),
            home_page_url: "https://cutecity.cc/blog".into(),
            feed_url: "https://cutecity.cc/feed.json".into(),
            description: "Street dispatches from Cute City.".into(),
            author_name: "Zackary Camoran".into(),
        }
    }

    fn item(n: u32) -> FeedItem {
        FeedItem {
            id: format!("https://cutecity.cc/blog/post-{n}"),
            url: format!("https://cutecity.cc/blog/post-{n}"),
            title: format!("Post {n}"),
            content_html: None,
            content_text: Some(format!("Body {n}")),
            summary: Some(format!("Summary {n}")),
            date_published: Some("2026-10-06T12:00:00Z".into()),
            date_modified: None,
            image: None,
            tags: vec!["dispatch".into()],
            author_name: None,
        }
    }

    #[test]
    fn json_feed_shape() {
        let doc = json_feed(&meta(), &[item(1), item(2)]).unwrap();
        let v: serde_json::Value = serde_json::from_str(&doc).unwrap();
        assert_eq!(v["version"], "https://jsonfeed.org/version/1.1");
        assert_eq!(v["title"], "Cute City — Dispatches");
        assert_eq!(v["items"].as_array().unwrap().len(), 2);
        // Optional empty fields must be absent, not null.
        assert!(v["items"][0].get("image").is_none());
        assert!(v["items"][0].get("content_html").is_none());
    }

    #[test]
    fn json_feed_rejects_missing_meta() {
        let mut m = meta();
        m.title.clear();
        assert!(matches!(
            json_feed(&m, &[]),
            Err(FeedError::MissingMeta("title"))
        ));
    }

    #[test]
    fn rss_feed_escapes_and_orders() {
        let mut i = item(1);
        i.title = "A <b> & \"quote\"".into();
        let xml = rss_feed(&meta(), &[i]).unwrap();
        assert!(xml.contains("<title>A &lt;b&gt; &amp; &quot;quote&quot;</title>"));
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<guid isPermaLink=\"true\">"));
    }

    #[test]
    fn pagination_slices_correctly() {
        let items: Vec<FeedItem> = (1..=25).map(item).collect();
        let p1 = Page { number: 1, size: 10 };
        let p3 = Page { number: 3, size: 10 };
        let p4 = Page { number: 4, size: 10 };
        assert_eq!(p1.slice(&items).len(), 10);
        assert_eq!(p3.slice(&items).len(), 5);
        assert!(p4.slice(&items).is_empty());
        assert_eq!(p1.total_pages(25), 3);
        assert_eq!(p1.total_pages(0), 0);
        assert_eq!(p1.total_pages(10), 1);
    }

    #[test]
    fn rfc3339_conversion() {
        assert_eq!(
            rfc3339_to_http_date("2026-10-06T12:00:00Z"),
            Some("06 Oct 2026 12:00:00 GMT".into())
        );
        assert_eq!(
            rfc3339_to_http_date("2026-08-18T07:14:01+00:00"),
            Some("18 Aug 2026 07:14:01 GMT".into())
        );
        assert_eq!(rfc3339_to_http_date("not-a-date"), None);
        assert_eq!(rfc3339_to_http_date("2026-13-01T00:00:00Z"), None);
    }

    #[test]
    fn xml_escape_covers_all_specials() {
        assert_eq!(xml_escape("&<>\"'"), "&amp;&lt;&gt;&quot;&apos;");
    }
}
