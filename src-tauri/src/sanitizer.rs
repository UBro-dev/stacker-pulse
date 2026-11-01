//! Author: UBro-dev | License: MIT (Copyright (c) 2026 UBro-dev)
use serde::{Deserialize, Serialize};
use url::Url;

/// Tracking parameters that mean nothing but tracking on any site.
const GLOBAL_TRACKERS: &[&str] = &[
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "utm_id",
    "fbclid",
    "gclid",
    "dclid",
    "msclkid",
    "mc_eid",
    "_bhiiv",
    "bhcl_id",
];

/// Parameters that are trackers only on specific hosts. Elsewhere they carry
/// meaning: `t` is a YouTube timestamp, `s` and `t` are search parameters on
/// many sites, `ref` routes on others. Stripping them globally silently changes
/// where a link points, which is worse than leaving a tracker in.
fn host_scoped_trackers(host: &str) -> &'static [&'static str] {
    match host {
        "x.com" | "www.x.com" | "twitter.com" | "www.twitter.com" | "mobile.twitter.com" => {
            &["s", "t", "ref_src", "ref_url"]
        }
        "youtube.com" | "www.youtube.com" | "m.youtube.com" | "youtu.be" | "music.youtube.com" => {
            &["si", "feature", "pp"]
        }
        "open.spotify.com" => &["si"],
        "instagram.com" | "www.instagram.com" => &["igsh", "igshid"],
        "github.com" => &["ref", "ref_cta", "ref_loc", "ref_page"],
        _ => &[],
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SanitizedUrlResult {
    pub original: String,
    pub sanitized: String,
    pub removed_params_count: usize,
    pub frontend_rewritten: bool,
    pub normalized_domain_path: String,
}

/// Implements Stacker News Issue #2664 (auto-convert x.com to xcancel.com/nitter.net)
/// and strips tracking query parameters to prevent duplicate link penalties.
pub fn sanitize_stacker_url(raw_url: &str, mode: &str) -> Result<SanitizedUrlResult, String> {
    sanitize_stacker_url_with(raw_url, mode, None)
}

/// Default public instances. Public Nitter / Redlib instances come and go, so
/// the UI can pass its own via `instance`.
pub const DEFAULT_NITTER: &str = "nitter.net";
pub const DEFAULT_REDLIB: &str = "redlib.catsarch.com";

/// Accepts a bare hostname only (`nitter.example.org`): no scheme, port, path
/// or credentials, so a pasted value cannot smuggle anything into the link.
pub fn validate_instance_host(h: &str) -> Result<String, String> {
    let h = h.trim().trim_end_matches('.').to_ascii_lowercase();
    let ok = !h.is_empty()
        && h.len() <= 253
        && h.contains('.')
        && h.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        });
    if ok {
        Ok(h)
    } else {
        Err(format!(
            "Instance must be a bare hostname like nitter.example.org, got: {h}"
        ))
    }
}

pub fn sanitize_stacker_url_with(
    raw_url: &str,
    mode: &str,
    instance: Option<&str>,
) -> Result<SanitizedUrlResult, String> {
    let custom = match instance.map(str::trim).filter(|s| !s.is_empty()) {
        Some(h) => Some(validate_instance_host(h)?),
        None => None,
    };
    let trimmed = raw_url.trim();
    let mut parsed = Url::parse(trimmed).map_err(|e| format!("Invalid URL: {e}"))?;

    let existing_pairs: Vec<(String, String)> = parsed
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    let host_lc = parsed.host_str().unwrap_or_default().to_lowercase();
    let scoped = host_scoped_trackers(&host_lc);

    let mut removed_params_count = 0usize;
    let kept_pairs: Vec<(String, String)> = existing_pairs
        .into_iter()
        .filter(|(k, _)| {
            let is_tracker = GLOBAL_TRACKERS
                .iter()
                .chain(scoped.iter())
                .any(|tp| tp.eq_ignore_ascii_case(k.as_str()));
            if is_tracker {
                removed_params_count += 1;
            }
            !is_tracker
        })
        .collect();

    if kept_pairs.is_empty() {
        parsed.set_query(None);
    } else {
        let mut serializer = url::form_urlencoded::Serializer::new(String::new());
        for (k, v) in kept_pairs {
            serializer.append_pair(&k, &v);
        }
        parsed.set_query(Some(&serializer.finish()));
    }

    let mut frontend_rewritten = false;
    if let Some(host) = parsed.host_str().map(|h| h.to_lowercase()) {
        match mode {
            "xcancel"
                if host == "x.com"
                    || host == "www.x.com"
                    || host == "twitter.com"
                    || host == "www.twitter.com" =>
            {
                let _ = parsed.set_host(Some("xcancel.com"));
                frontend_rewritten = true;
            }
            "nitter"
                if host == "x.com"
                    || host == "www.x.com"
                    || host == "twitter.com"
                    || host == "www.twitter.com" =>
            {
                let _ = parsed.set_host(Some(custom.as_deref().unwrap_or(DEFAULT_NITTER)));
                frontend_rewritten = true;
            }
            "scribe" if host == "medium.com" || host.ends_with(".medium.com") => {
                let _ = parsed.set_host(Some("scribe.rip"));
                frontend_rewritten = true;
            }
            "redlib"
                if host == "reddit.com" || host == "www.reddit.com" || host == "old.reddit.com" =>
            {
                let _ = parsed.set_host(Some(custom.as_deref().unwrap_or(DEFAULT_REDLIB)));
                frontend_rewritten = true;
            }
            _ => {}
        }
    }

    let sanitized = parsed.to_string();
    let normalized_domain_path = format!(
        "{}{}",
        parsed
            .host_str()
            .unwrap_or_default()
            .trim_start_matches("www."),
        parsed.path().trim_end_matches('/')
    )
    .to_lowercase();

    Ok(SanitizedUrlResult {
        original: trimmed.to_string(),
        sanitized,
        removed_params_count,
        frontend_rewritten,
        normalized_domain_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nitter_uses_default_instance() {
        let r = sanitize_stacker_url("https://x.com/a/status/1", "nitter").unwrap();
        assert_eq!(r.sanitized, "https://nitter.net/a/status/1");
    }

    #[test]
    fn custom_instance_is_used() {
        let r = sanitize_stacker_url_with(
            "https://x.com/a/status/1?s=20",
            "nitter",
            Some("nitter.example.org"),
        )
        .unwrap();
        assert_eq!(r.sanitized, "https://nitter.example.org/a/status/1");
        let r = sanitize_stacker_url_with(
            "https://www.reddit.com/r/btc/",
            "redlib",
            Some(" Redlib.Example.ORG "),
        )
        .unwrap();
        assert_eq!(r.sanitized, "https://redlib.example.org/r/btc/");
    }

    #[test]
    fn empty_instance_falls_back_to_default() {
        let r =
            sanitize_stacker_url_with("https://reddit.com/r/btc", "redlib", Some("  ")).unwrap();
        assert!(r.sanitized.starts_with("https://redlib.catsarch.com/"));
    }

    #[test]
    fn instance_must_be_a_bare_hostname() {
        for bad in [
            "https://evil.com",
            "evil.com/path",
            "user@evil.com",
            "evil.com:8080",
            "localhost",
            "-a.com",
            "a..com",
            "a b.com",
        ] {
            assert!(
                sanitize_stacker_url_with("https://x.com/a", "nitter", Some(bad)).is_err(),
                "accepted {bad}"
            );
        }
    }

    #[test]
    fn test_xcancel_and_utm_strip() {
        let raw = "https://x.com/stacker_news/status/18491029381029?s=20&t=abc&utm_source=twitter";
        let res = sanitize_stacker_url(raw, "xcancel").unwrap();
        assert_eq!(
            res.sanitized,
            "https://xcancel.com/stacker_news/status/18491029381029"
        );
        assert_eq!(res.removed_params_count, 3);
        assert!(res.frontend_rewritten);
    }

    #[test]
    fn youtube_timestamp_survives_tracker_does_not() {
        let raw = "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=120&si=TRACKER123";
        let res = sanitize_stacker_url(raw, "none").unwrap();
        assert_eq!(
            res.sanitized,
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=120"
        );
        assert_eq!(res.removed_params_count, 1);
    }

    #[test]
    fn search_parameters_survive() {
        let raw = "https://duckduckgo.com/?q=zcash&t=h_";
        let res = sanitize_stacker_url(raw, "none").unwrap();
        assert_eq!(res.sanitized, raw);
        assert_eq!(res.removed_params_count, 0);
    }

    #[test]
    fn utm_is_stripped_everywhere() {
        let raw = "https://example.org/post?id=7&utm_source=newsletter&utm_medium=email";
        let res = sanitize_stacker_url(raw, "none").unwrap();
        assert_eq!(res.sanitized, "https://example.org/post?id=7");
        assert_eq!(res.removed_params_count, 2);
    }

    #[test]
    fn stacker_news_links_are_untouched() {
        let raw = "https://stacker.news/items/12345";
        assert_eq!(sanitize_stacker_url(raw, "none").unwrap().sanitized, raw);
    }
}
