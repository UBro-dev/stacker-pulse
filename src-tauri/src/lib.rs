//! StackerPulse + SeedScan Core Library
//! Author: UBro-dev
//! License: MIT (Copyright (c) 2026 UBro-dev)

pub mod detect;
pub mod github;
pub mod graphql;
pub mod sanitizer;
pub mod seedscan;
pub mod sn;
pub mod support;

use detect::Finding;
use graphql::{NetworkPrivacyConfig, SnGraphqlClient};
use sanitizer::{sanitize_stacker_url_with, SanitizedUrlResult};
use seedscan::{scan_folders, scan_text_content, DiskScanReport};
use serde_json::Value;
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use tauri::Manager;

/// Live feed. `sort`: "hot" | "new" | "top"; `when` applies to "top".
#[tauri::command]
async fn sn_fetch_feed(
    sub: Option<String>,
    sort: Option<String>,
    when: Option<String>,
    privacy: Option<NetworkPrivacyConfig>,
) -> Result<Vec<sn::FeedItem>, String> {
    let client = SnGraphqlClient::with_privacy_config(&privacy.unwrap_or_default())?;
    sn::fetch_feed(
        &client,
        sub.as_deref(),
        sort.as_deref().unwrap_or("hot"),
        when.as_deref(),
    )
    .await
}

/// BTC price in USD and today's rewards pool, both from SN.
/// Which local Tor ports accept connections (9150 Tor Browser, 9050 tor service).
/// Only connects to 127.0.0.1; sends nothing.
#[tauri::command]
async fn sn_detect_tor() -> Vec<u16> {
    tauri::async_runtime::spawn_blocking(|| {
        graphql::TOR_PORTS
            .iter()
            .copied()
            .filter(|&p| graphql::local_port_open(p))
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// The app version, from Cargo.toml at compile time (the only place it is set).
#[tauri::command]
fn sn_app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Public donation addresses from DONATE.md (embedded at build time, checked
/// by tests). No network access.
#[tauri::command]
fn sn_support_addresses() -> Result<Vec<support::SupportAddress>, String> {
    support::addresses()
}

/// Whether SN's onion answered recently: "unknown", "up" or "down".
/// `reset: true` forgets a failure so the next request tries the onion again.
#[tauri::command]
fn sn_onion_status(reset: Option<bool>) -> &'static str {
    if reset == Some(true) {
        graphql::reset_onion_status();
    }
    graphql::onion_status()
}

#[tauri::command]
async fn sn_fetch_market(privacy: Option<NetworkPrivacyConfig>) -> Result<sn::Market, String> {
    let client = SnGraphqlClient::with_privacy_config(&privacy.unwrap_or_default())?;
    sn::fetch_market(&client).await
}

/// Bounties on SN that nobody has been paid for yet.
#[tauri::command]
async fn sn_fetch_sn_bounties(
    privacy: Option<NetworkPrivacyConfig>,
) -> Result<Vec<sn::FeedItem>, String> {
    let client = SnGraphqlClient::with_privacy_config(&privacy.unwrap_or_default())?;
    sn::fetch_active_bounties(&client).await
}

/// Public stats of one stacker for a period ("day" | "week" | "month" | "year" | "forever").
#[tauri::command]
async fn sn_fetch_stacker(
    name: String,
    period: String,
    privacy: Option<NetworkPrivacyConfig>,
) -> Result<sn::StackerStats, String> {
    let client = SnGraphqlClient::with_privacy_config(&privacy.unwrap_or_default())?;
    sn::fetch_stacker(&client, &name, &period).await
}

/// Real economics of every territory for a period, from SN's own topSubs.
#[tauri::command]
async fn sn_fetch_territories(
    period: String,
    privacy: Option<NetworkPrivacyConfig>,
) -> Result<Vec<sn::TerritoryStats>, String> {
    let client = SnGraphqlClient::with_privacy_config(&privacy.unwrap_or_default())?;
    sn::fetch_territories(&client, &period).await
}

#[tauri::command]
async fn sn_execute_graphql(
    query: String,
    variables: Option<Value>,
    privacy: Option<NetworkPrivacyConfig>,
) -> Result<Value, String> {
    let cfg = privacy.unwrap_or_default();
    let client = SnGraphqlClient::with_privacy_config(&cfg)?;
    client.execute_raw(&query, variables).await
}

/// Read-only list of issues from stackernews/stacker.news with PR awards
/// computed from their labels. See github.rs for the privacy rules.
/// `allow_clearnet` must be true for a request without Tor; the UI only sets
/// it after the user confirms that GitHub will see their IP.
#[tauri::command]
async fn sn_fetch_github_issues(
    query: Option<String>,
    state: Option<String>,
    privacy: Option<NetworkPrivacyConfig>,
    allow_clearnet: Option<bool>,
) -> Result<github::GhFetchResult, String> {
    let cfg = privacy.unwrap_or_default();
    github::fetch_issues(
        query.as_deref().unwrap_or(""),
        state.as_deref().unwrap_or("open"),
        &cfg,
        allow_clearnet.unwrap_or(false),
    )
    .await
}

#[tauri::command]
fn sn_sanitize_url(
    raw_url: String,
    mode: String,
    instance: Option<String>,
) -> Result<SanitizedUrlResult, String> {
    sanitize_stacker_url_with(&raw_url, &mode, instance.as_deref())
}

/// Offline read-only disk scanner (SeedScan by UBro-dev)
#[tauri::command]
async fn sn_run_seedscan_disk(
    paths: Vec<String>,
    max_size_mb: u64,
    hidden: bool,
) -> Result<DiskScanReport, String> {
    // A big folder takes a while; run it off the UI thread so the window
    // does not freeze while walking the disk.
    let path_bufs: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    tauri::async_runtime::spawn_blocking(move || scan_folders(path_bufs, max_size_mb, hidden))
        .await
        .map_err(|e| format!("scan task failed: {e}"))
}

/// Offline text & post-draft scanner for BIP39 seeds, xprv, WIF, nsec1, NWC secrets & OS path leaks
#[tauri::command]
async fn sn_scan_text_secrets(content: String) -> Result<Vec<Finding>, String> {
    // Synchronous commands run on the UI thread: a large pasted log froze the
    // window on every keystroke. Run the scan on a worker thread instead.
    if content.len() > 8 * 1024 * 1024 {
        return Err(
            "Text is larger than 8 MB; scan the file with the folder scanner instead".into(),
        );
    }
    tauri::async_runtime::spawn_blocking(move || scan_text_content(&content))
        .await
        .map_err(|e| format!("scan task failed: {e}"))
}

/// Returns the real source of a module for the in-app code viewer.
///
/// `include_str!` embeds the files into the binary at compile time, so the
/// viewer always shows exactly the code that was compiled — it cannot drift
/// the way hand-copied strings in the UI did. Only names on this list are
/// served; nothing is read from disk at runtime.
#[tauri::command]
fn sn_get_rust_source(name: String) -> Result<&'static str, String> {
    Ok(match name.as_str() {
        "detect_rs" => include_str!("detect.rs"),
        "seedscan_rs" => include_str!("seedscan.rs"),
        "graphql_rs" => include_str!("graphql.rs"),
        "github_rs" => include_str!("github.rs"),
        "sn_rs" => include_str!("sn.rs"),
        "lib_rs" => include_str!("lib.rs"),
        "sanitizer_rs" => include_str!("sanitizer.rs"),
        "support_rs" => include_str!("support.rs"),
        "cargo_toml" => include_str!("../Cargo.toml"),
        "license_mit" => include_str!("../LICENSE"),
        // MIT and Apache-2.0 require the licence text and copyright notices to
        // travel with the binaries, not only with the source.
        "third_party" => include_str!("../../THIRD-PARTY.md"),
        "disclaimer" => include_str!("../../DISCLAIMER.md"),
        "donate" => support::DONATE_MD,
        _ => return Err(format!("unknown source file: {name}")),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Acrylic is Windows-only; on other platforms there is nothing to do,
            // so the window lookup is compiled only where it is used.
            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window_vibrancy::apply_acrylic(&window, Some((14, 16, 26, 195)));
            }
            #[cfg(not(target_os = "windows"))]
            let _ = &app;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            sn_fetch_feed,
            sn_fetch_sn_bounties,
            sn_fetch_market,
            sn_detect_tor,
            sn_onion_status,
            sn_app_version,
            sn_support_addresses,
            sn_fetch_stacker,
            sn_fetch_territories,
            sn_execute_graphql,
            sn_sanitize_url,
            sn_run_seedscan_disk,
            sn_scan_text_secrets,
            sn_get_rust_source,
            sn_fetch_github_issues
        ])
        .run(tauri::generate_context!())
        .expect("error while running StackerPulse by UBro-dev");
}

#[cfg(test)]
mod notice_tests {
    /// MIT and Apache-2.0 require the licence texts and copyright notices to
    /// be distributed with the binaries, so they are compiled into the
    /// executable rather than only sitting in the repository.
    #[test]
    fn licence_notices_are_embedded_in_the_binary() {
        for name in ["license_mit", "third_party", "disclaimer"] {
            let text = super::sn_get_rust_source(name.to_string()).expect(name);
            assert!(text.len() > 400, "{name} is empty or truncated");
        }
        let third = super::sn_get_rust_source("third_party".into()).unwrap();
        assert!(third.contains("MIT License"), "MIT text missing");
        assert!(third.contains("Apache License"), "Apache-2.0 text missing");
        assert!(
            third.contains("Mozilla Public License"),
            "MPL-2.0 text missing"
        );
        assert!(
            third.matches("| ").count() > 300,
            "dependency table looks short"
        );
    }

    /// Every file offered in the source viewer must exist in the binary.
    #[test]
    fn every_source_viewer_option_resolves() {
        let html = include_str!("../../ui/index.html");
        let start = html.find("id=\"rustFileSelect\"").expect("viewer select");
        let end = start + html[start..].find("</select>").expect("end of select");
        let mut n = 0;
        for part in html[start..end].split("<option value=\"").skip(1) {
            let key = &part[..part.find('"').unwrap()];
            assert!(
                super::sn_get_rust_source(key.to_string()).is_ok(),
                "viewer option {key} has no file"
            );
            n += 1;
        }
        assert!(n >= 12, "only {n} viewer options found");
    }
}

#[cfg(test)]
mod version_tests {
    /// v3.0.0 showed "v3.0" in the header while the build was newer. The
    /// version must exist in exactly one place: Cargo.toml.
    #[test]
    fn version_is_set_only_in_cargo_toml() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert!(
            conf.get("version").is_none(),
            "tauri.conf.json must not repeat the version"
        );
        let html = include_str!("../../ui/index.html");
        let js = include_str!("../../ui/app.js");
        let re = regex::Regex::new(r"(Suite|SeedScan|StackerPulse|seedscan) v\d").unwrap();
        assert!(!re.is_match(html), "hard-coded version in index.html");
        assert!(!re.is_match(js), "hard-coded version in app.js");
        assert!(!env!("CARGO_PKG_VERSION").is_empty());
    }
}
