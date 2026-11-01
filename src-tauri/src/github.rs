//! Read-only GitHub client for the Stacker News PR bounty radar.
//! Author: UBro-dev
//! License: MIT (Copyright (c) 2026 UBro-dev)
//!
//! Stacker News pays for pull requests that close issues in
//! `stackernews/stacker.news`. The award comes from the issue's labels
//! (README, "Pull request awards"):
//!
//!   difficulty:good-first-issue 20k · easy 100k · medium 250k ·
//!   medium-hard 500k · hard 1m sats
//!   priority:low ×0.5 · medium ×1.5 · high ×2 · urgent ×3
//!
//! Awards are discretionary and each substantial review change takes 10% off,
//! so the computed figure is an upper bound. Issues without a difficulty label
//! get no amount at all — nothing is guessed.
//!
//! Privacy rules enforced here, not in the UI:
//!   1. Host `api.github.com` only, and only two fixed paths for one repository.
//!      URLs are built from constants; the final URL is checked again before
//!      the request is sent.
//!   2. No redirects at all. The API has no reason to redirect these calls.
//!   3. Airgap refuses to build a client.
//!   4. Without Tor the call fails unless the user explicitly allowed clearnet
//!      for this request, because GitHub then sees the user's IP.
//!   5. No token, no cookies, no Referer. GitHub rejects requests without a
//!      User-Agent, so a common browser UA is sent.
//!   6. The response body is capped, and only the fields below are kept.

use crate::graphql::{NetworkPrivacyConfig, BROWSER_UA};
use reqwest::{redirect::Policy, Client, Proxy, Url};
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

pub const GITHUB_HOST: &str = "api.github.com";
pub const REPO: &str = "stackernews/stacker.news";
const ISSUES_PATH: &str = "/repos/stackernews/stacker.news/issues";
const SEARCH_PATH: &str = "/search/issues";
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const MAX_QUERY_CHARS: usize = 100;
const PER_PAGE: &str = "50";

/// Error text the UI matches to show the "reveal your IP to GitHub?" prompt.
pub const ERR_TOR_REQUIRED: &str = "TOR_REQUIRED";

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GhIssue {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub state: String, // "open" | "closed"
    pub labels: Vec<String>,
    pub comments: u64,
    pub updated_at: String,
    pub difficulty: Option<String>,
    pub priority: Option<String>,
    /// Upper bound per the README table; None when the issue has no difficulty label.
    pub award_sats: Option<u64>,
    /// Rough effort guess from the difficulty label, for sats/hour sorting only.
    pub est_hours: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GhFetchResult {
    pub issues: Vec<GhIssue>,
    pub via_tor: bool,
    pub rate_limit_remaining: Option<u64>,
    pub rate_limit_reset_unix: Option<u64>,
}

/// Base award for a difficulty label, from the Stacker News README.
pub fn difficulty_award(label: &str) -> Option<(u64, f64)> {
    // (sats, rough hours)
    match label {
        "difficulty:good-first-issue" => Some((20_000, 2.0)),
        "difficulty:easy" => Some((100_000, 6.0)),
        "difficulty:medium" => Some((250_000, 16.0)),
        "difficulty:medium-hard" => Some((500_000, 30.0)),
        "difficulty:hard" => Some((1_000_000, 60.0)),
        _ => None,
    }
}

/// Priority multiplier in tenths (×1.5 → 15), so the math stays in integers.
pub fn priority_multiplier_tenths(label: &str) -> Option<u64> {
    match label {
        "priority:low" => Some(5),
        "priority:medium" => Some(15),
        "priority:high" => Some(20),
        "priority:urgent" => Some(30),
        _ => None,
    }
}

/// Award upper bound from a list of labels. No difficulty label → None.
pub fn award_from_labels(
    labels: &[String],
) -> (Option<String>, Option<String>, Option<u64>, Option<f64>) {
    let difficulty = labels
        .iter()
        .find(|l| difficulty_award(l).is_some())
        .cloned();
    let priority = labels
        .iter()
        .find(|l| priority_multiplier_tenths(l).is_some())
        .cloned();
    match difficulty.as_deref().and_then(difficulty_award) {
        Some((base, hours)) => {
            let tenths = priority
                .as_deref()
                .and_then(priority_multiplier_tenths)
                .unwrap_or(10);
            (
                difficulty,
                priority,
                Some(base.saturating_mul(tenths) / 10),
                Some(hours),
            )
        }
        None => (difficulty, priority, None, None),
    }
}

/// Free-text search words only. Anything with ':' is a GitHub qualifier
/// (`repo:`, `org:`, `user:` …) and is dropped, so a search can never leave
/// the Stacker News repository.
pub fn clean_search_words(raw: &str) -> String {
    let words: Vec<&str> = raw
        .split_whitespace()
        .filter(|w| !w.contains(':'))
        .collect();
    let mut out = words.join(" ");
    if out.chars().count() > MAX_QUERY_CHARS {
        out = out.chars().take(MAX_QUERY_CHARS).collect();
    }
    out
}

/// Builds the request URL from constants. `state` is "open" | "closed" | "all".
pub fn build_url(search: &str, state: &str) -> Result<Url, String> {
    let state = match state {
        "closed" => "closed",
        "all" => "all",
        _ => "open",
    };
    let words = clean_search_words(search);
    let mut url = Url::parse(&format!("https://{GITHUB_HOST}")).map_err(|e| e.to_string())?;
    if words.is_empty() {
        url.set_path(ISSUES_PATH);
        url.query_pairs_mut()
            .append_pair("state", state)
            .append_pair("per_page", PER_PAGE)
            .append_pair("sort", "updated");
    } else {
        let state_q = if state == "all" {
            String::new()
        } else {
            format!(" is:{state}")
        };
        url.set_path(SEARCH_PATH);
        url.query_pairs_mut()
            .append_pair("q", &format!("repo:{REPO} is:issue{state_q} {words}"))
            .append_pair("per_page", PER_PAGE)
            .append_pair("sort", "updated");
    }
    verify_url(&url)?;
    Ok(url)
}

/// The final check before any socket is opened.
pub fn verify_url(url: &Url) -> Result<(), String> {
    if url.scheme() != "https" {
        return Err("GitHub firewall: only https is allowed".into());
    }
    if url.host_str() != Some(GITHUB_HOST) || url.port().is_some() {
        return Err(format!(
            "GitHub firewall: host '{}' is not allowed",
            url.host_str().unwrap_or("")
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("GitHub firewall: credentials in URL are not allowed".into());
    }
    match url.path() {
        ISSUES_PATH => Ok(()),
        SEARCH_PATH => {
            let q = url
                .query_pairs()
                .find(|(k, _)| k == "q")
                .map(|(_, v)| v.into_owned())
                .unwrap_or_default();
            let qualifiers: Vec<&str> = q.split_whitespace().filter(|w| w.contains(':')).collect();
            let repo_ok = qualifiers
                .iter()
                .filter(|w| w.starts_with("repo:"))
                .all(|w| *w == format!("repo:{REPO}"))
                && qualifiers.contains(&format!("repo:{REPO}").as_str());
            let only_known = qualifiers.iter().all(|w| {
                w.starts_with("repo:") || *w == "is:issue" || *w == "is:open" || *w == "is:closed"
            });
            if repo_ok && only_known {
                Ok(())
            } else {
                Err("GitHub firewall: search must stay inside stackernews/stacker.news".into())
            }
        }
        other => Err(format!("GitHub firewall: path '{other}' is not allowed")),
    }
}

fn build_client(cfg: &NetworkPrivacyConfig) -> Result<Client, String> {
    if cfg.airgap_mode {
        return Err("Blocked by Airgap Mode (100% Offline Privacy Lock is active)".into());
    }
    let mut builder = Client::builder()
        .timeout(Duration::from_secs(if cfg.use_tor { 30 } else { 15 }))
        .referer(false)
        .redirect(Policy::none())
        .user_agent(BROWSER_UA);
    if cfg.use_tor {
        let proxy_url = if cfg.socks5_proxy.trim().is_empty() {
            "socks5h://127.0.0.1:9050"
        } else {
            cfg.socks5_proxy.trim()
        };
        if !proxy_url.starts_with("socks5h://") {
            return Err(
                "GitHub requests over Tor need a socks5h:// proxy, so DNS stays inside Tor".into(),
            );
        }
        builder =
            builder.proxy(Proxy::all(proxy_url).map_err(|e| format!("Invalid Tor proxy: {e}"))?);
    } else {
        // Never pick up HTTP(S)_PROXY from the environment for this client.
        builder = builder.no_proxy();
    }
    builder
        .build()
        .map_err(|e| format!("Failed to build GitHub client: {e}"))
}

fn header_u64(resp: &reqwest::Response, name: &str) -> Option<u64> {
    resp.headers().get(name)?.to_str().ok()?.parse().ok()
}

/// Turns one raw issue object into a `GhIssue`. Pull requests are skipped.
pub fn parse_issue(raw: &Value) -> Option<GhIssue> {
    if raw.get("pull_request").is_some() {
        return None;
    }
    let number = raw.get("number")?.as_u64()?;
    let labels: Vec<String> = raw
        .get("labels")
        .and_then(|l| l.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|l| l.get("name")?.as_str().map(str::to_string))
                .take(20)
                .collect()
        })
        .unwrap_or_default();
    let (difficulty, priority, award_sats, est_hours) = award_from_labels(&labels);
    let title: String = raw
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .chars()
        .take(300)
        .collect();
    Some(GhIssue {
        number,
        title,
        // Built from the number rather than trusted from the response.
        url: format!("https://github.com/{REPO}/issues/{number}"),
        state: if raw.get("state").and_then(|v| v.as_str()) == Some("closed") {
            "closed".into()
        } else {
            "open".into()
        },
        labels,
        comments: raw.get("comments").and_then(|v| v.as_u64()).unwrap_or(0),
        updated_at: raw
            .get("updated_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .chars()
            .take(40)
            .collect(),
        difficulty,
        priority,
        award_sats,
        est_hours,
    })
}

pub fn parse_response(body: &Value) -> Result<Vec<GhIssue>, String> {
    let list = if let Some(arr) = body.as_array() {
        arr
    } else if let Some(arr) = body.get("items").and_then(|v| v.as_array()) {
        arr
    } else {
        let msg = body
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unexpected response");
        return Err(format!(
            "GitHub API: {}",
            msg.chars().take(200).collect::<String>()
        ));
    };
    Ok(list.iter().filter_map(parse_issue).collect())
}

pub async fn fetch_issues(
    search: &str,
    state: &str,
    cfg: &NetworkPrivacyConfig,
    allow_clearnet: bool,
) -> Result<GhFetchResult, String> {
    if cfg.airgap_mode {
        return Err("Blocked by Airgap Mode (100% Offline Privacy Lock is active)".into());
    }
    if !cfg.use_tor && !allow_clearnet {
        return Err(ERR_TOR_REQUIRED.into());
    }
    let url = build_url(search, state)?;
    let client = build_client(cfg)?;
    verify_url(&url)?;
    let mut resp = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| format!("GitHub request failed: {e}"))?;

    let status = resp.status();
    let rate_limit_remaining = header_u64(&resp, "x-ratelimit-remaining");
    let rate_limit_reset_unix = header_u64(&resp, "x-ratelimit-reset");

    let mut body = Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("GitHub read failed: {e}"))?
    {
        if body.len() + chunk.len() > MAX_BODY_BYTES {
            return Err("GitHub response is larger than 2 MB; refused".into());
        }
        body.extend_from_slice(&chunk);
    }
    let json: Value = serde_json::from_slice(&body)
        .map_err(|e| format!("GitHub returned invalid JSON ({status}): {e}"))?;
    if status.as_u16() == 403 && rate_limit_remaining == Some(0) {
        return Err("GitHub rate limit reached (60 requests/hour without a token). Try later or switch Tor circuit.".into());
    }
    if !status.is_success() {
        return Err(parse_response(&json)
            .err()
            .unwrap_or_else(|| format!("GitHub API HTTP {status}")));
    }
    Ok(GhFetchResult {
        issues: parse_response(&json)?,
        via_tor: cfg.use_tor,
        rate_limit_remaining,
        rate_limit_reset_unix,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn labels(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn award_follows_the_readme_table() {
        assert_eq!(
            award_from_labels(&labels(&["difficulty:easy"])).2,
            Some(100_000)
        );
        assert_eq!(
            award_from_labels(&labels(&["difficulty:hard", "priority:urgent"])).2,
            Some(3_000_000)
        );
        assert_eq!(
            award_from_labels(&labels(&["priority:low", "difficulty:medium"])).2,
            Some(125_000)
        );
        assert_eq!(
            award_from_labels(&labels(&["difficulty:good-first-issue", "priority:medium"])).2,
            Some(30_000)
        );
    }

    #[test]
    fn no_difficulty_label_means_no_amount() {
        let (d, p, a, h) = award_from_labels(&labels(&["bug", "priority:high", "enhancement"]));
        assert_eq!((d, a, h), (None, None, None));
        assert_eq!(p.as_deref(), Some("priority:high"));
    }

    #[test]
    fn issue_list_url_is_fixed() {
        let u = build_url("", "open").unwrap();
        assert_eq!(u.host_str(), Some("api.github.com"));
        assert_eq!(u.path(), "/repos/stackernews/stacker.news/issues");
    }

    #[test]
    fn search_cannot_leave_the_repository() {
        let u = build_url("wallet repo:evil/evil org:evil user:x", "open").unwrap();
        let q: String = u
            .query_pairs()
            .find(|(k, _)| k == "q")
            .unwrap()
            .1
            .into_owned();
        assert_eq!(q, "repo:stackernews/stacker.news is:issue is:open wallet");
    }

    #[test]
    fn firewall_rejects_other_hosts_paths_and_repos() {
        for bad in [
            "https://github.com/repos/stackernews/stacker.news/issues",
            "http://api.github.com/repos/stackernews/stacker.news/issues",
            "https://api.github.com:8443/repos/stackernews/stacker.news/issues",
            "https://user@api.github.com/repos/stackernews/stacker.news/issues",
            "https://api.github.com/user",
            "https://api.github.com/repos/evil/evil/issues",
            "https://api.github.com/search/issues?q=repo:evil/evil",
            "https://api.github.com/search/issues?q=repo:stackernews/stacker.news+repo:evil/evil",
            "https://api.github.com/search/issues?q=repo:stackernews/stacker.news+org:evil",
            "https://api.github.com/search/issues?q=wallet",
        ] {
            let u = Url::parse(bad).unwrap();
            assert!(verify_url(&u).is_err(), "accepted {bad}");
        }
    }

    #[test]
    fn airgap_and_missing_tor_refuse_before_any_request() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let airgap = NetworkPrivacyConfig {
            airgap_mode: true,
            ..Default::default()
        };
        let err = rt
            .block_on(fetch_issues("", "open", &airgap, true))
            .unwrap_err();
        assert!(err.contains("Airgap"));
        let clearnet = NetworkPrivacyConfig::default();
        let err = rt
            .block_on(fetch_issues("", "open", &clearnet, false))
            .unwrap_err();
        assert_eq!(err, ERR_TOR_REQUIRED);
    }

    #[test]
    fn tor_needs_socks5h() {
        let cfg = NetworkPrivacyConfig {
            use_tor: true,
            socks5_proxy: "socks5://127.0.0.1:9050".into(),
            ..Default::default()
        };
        assert!(build_client(&cfg).is_err());
    }

    #[test]
    fn parses_issues_skips_prs_and_rebuilds_urls() {
        let body = json!([
            { "number": 2664, "title": "auto-convert x.com links", "state": "open",
              "html_url": "https://evil.example/phish", "comments": 4, "updated_at": "2026-10-20T10:00:00Z",
              "labels": [{ "name": "difficulty:easy" }, { "name": "priority:high" }] },
            { "number": 7, "title": "a PR", "pull_request": {}, "labels": [] },
            { "number": 9, "title": "no labels", "state": "closed", "labels": [] }
        ]);
        let issues = parse_response(&body).unwrap();
        assert_eq!(issues.len(), 2);
        assert_eq!(
            issues[0].url,
            "https://github.com/stackernews/stacker.news/issues/2664"
        );
        assert_eq!(issues[0].award_sats, Some(200_000));
        assert_eq!(issues[1].award_sats, None);
        assert_eq!(issues[1].state, "closed");
        let search =
            json!({ "total_count": 1, "items": [{ "number": 1, "title": "x", "labels": [] }] });
        assert_eq!(parse_response(&search).unwrap().len(), 1);
        assert!(
            parse_response(&json!({ "message": "API rate limit exceeded" }))
                .unwrap_err()
                .contains("rate limit")
        );
    }

    /// A local server answers with a redirect; the client must not follow it.
    #[test]
    fn github_client_never_follows_redirects() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let srv = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            let _ = s.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/evil\r\nContent-Length: 0\r\n\r\n");
        });
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let resp = rt
            .block_on(async {
                let client = build_client(&NetworkPrivacyConfig::default()).unwrap();
                client.get(format!("http://127.0.0.1:{port}/")).send().await
            })
            .unwrap();
        assert_eq!(
            resp.status().as_u16(),
            302,
            "redirect must be returned, not followed"
        );
        srv.join().unwrap();
    }
}
