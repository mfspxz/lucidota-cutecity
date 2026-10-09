//! Slug generation: lowercase, ASCII-safe, deterministic.

/// Convert a title to a URL slug. Non-ASCII alnum chars are dropped (not
/// transliterated — no silent surprises). Returns None when nothing survives.
pub fn slugify(title: &str) -> Option<String> {
    let mut out = String::new();
    let mut prev_dash = true; // treat start as dash to avoid leading '-'
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// A slug is valid if it matches [a-z0-9]+(-[a-z0-9]+)*.
pub fn is_valid_slug(s: &str) -> bool {
    !s.is_empty()
        && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_slug() {
        assert_eq!(slugify("How City Hall Runs the Website"), Some("how-city-hall-runs-the-website".into()));
    }

    #[test]
    fn slug_trims_and_collapses() {
        assert_eq!(slugify("  Why   Most of the City Is Mist!! "), Some("why-most-of-the-city-is-mist".into()));
    }

    #[test]
    fn slug_drops_non_ascii() {
        assert_eq!(slugify("Café ☕"), Some("caf".into()));
    }

    #[test]
    fn slug_empty_when_nothing_survives() {
        assert_eq!(slugify("!!! ☕ ☕"), None);
    }

    #[test]
    fn valid_slug_rules() {
        assert!(is_valid_slug("hello-world-123"));
        assert!(!is_valid_slug("Hello-World"));
        assert!(!is_valid_slug("-hello"));
        assert!(!is_valid_slug("hello-"));
        assert!(!is_valid_slug("hello--world"));
        assert!(!is_valid_slug(""));
        assert!(!is_valid_slug("héllo"));
    }
}
