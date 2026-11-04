//! Checking for and installing new versions, only when the user asks.
//! Author: UBro-dev
//! License: MIT (Copyright (c) 2026 UBro-dev)
//!
//! Built on the official `tauri-plugin-updater`, with our network rules on top:
//!
//! - **Nothing runs on its own.** There is no background check; the user
//!   presses "Check for updates", then "Install".
//! - **Tor when Tor is on.** Requests go through the same `socks5h://` proxy
//!   as everything else, so DNS stays inside Tor. Without Tor the user must
//!   agree first, because GitHub then sees their IP (the same rule as the
//!   bounty radar). Airgap mode blocks it outright.
//! - **Fixed hosts.** The update manifest comes from this repository's GitHub
//!   releases, and every redirect, as well as the download URL listed in the
//!   manifest, must stay on GitHub's own hosts.
//! - **Signed.** Every package is signed with the project's own minisign key
//!   (not a real-name certificate). The plugin checks the signature against
//!   the public key in `tauri.conf.json` before anything is installed, and
//!   `requireSignedVersion` binds the signature to the version number, so an
//!   older signed package can't be passed off as a newer one.

use crate::graphql::{NetworkPrivacyConfig, BROWSER_UA};
use serde::Serialize;
use std::time::Duration;
use url::Url;

/// The manifest `tauri-action` attaches to every release. `latest` only
/// resolves to a published release, so drafts are never offered.
pub const UPDATE_ENDPOINT: &str =
    "https://github.com/UBro-dev/stacker-pulse/releases/latest/download/latest.json";

/// GitHub serves release files from `github.com` and redirects downloads to
/// its asset hosts. Nothing else may be contacted while updating.
pub const UPDATE_HOSTS: &[&str] = &[
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];

pub const ERR_TOR_REQUIRED: &str = "TOR_REQUIRED";
const MAX_REDIRECTS: usize = 5;

/// HTTPS on one of [`UPDATE_HOSTS`], default port, no credentials in the URL.
pub fn update_url_allowed(url: &Url) -> bool {
    url.scheme() == "https"
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url
            .host_str()
            .map(|h| UPDATE_HOSTS.contains(&h.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
}

/// The proxy to use, or an error when the request must not happen at all.
pub fn route(cfg: &NetworkPrivacyConfig, allow_clearnet: bool) -> Result<Option<String>, String> {
    if cfg.airgap_mode {
        return Err("Blocked by Airgap Mode (100% Offline Privacy Lock is active)".into());
    }
    if !cfg.use_tor {
        return if allow_clearnet {
            Ok(None)
        } else {
            Err(ERR_TOR_REQUIRED.into())
        };
    }
    let proxy = match cfg.socks5_proxy.trim() {
        "" => "socks5h://127.0.0.1:9050",
        p => p,
    };
    let parsed = Url::parse(proxy).ok().filter(|u| {
        u.scheme() == "socks5h" && u.host_str().is_some_and(|h| !h.is_empty()) && u.port().is_some()
    });
    if parsed.is_none() {
        return Err(format!("NET:BAD_PROXY|{proxy}"));
    }
    Ok(Some(proxy.to_string()))
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct UpdateInfo {
    pub current: String,
    /// The newer version, or None when this is already the latest.
    pub available: Option<String>,
    pub notes: Option<String>,
    pub date: Option<String>,
    pub via_tor: bool,
}

#[cfg(desktop)]
mod desktop {
    use super::*;
    use reqwest_updater as rq;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use tauri::{AppHandle, Runtime, State};
    use tauri_plugin_updater::{Update, UpdaterExt};
    use tokio::sync::Mutex;

    /// The update found by the last check, kept until the user installs it,
    /// and the download progress the UI polls (no event permission needed).
    #[derive(Default)]
    pub struct PendingUpdate {
        pub update: Mutex<Option<Update>>,
        pub downloaded: Arc<AtomicU64>,
        /// 0 while unknown.
        pub total: Arc<AtomicU64>,
    }

    #[derive(Debug, Serialize, Clone, PartialEq)]
    pub struct UpdateProgress {
        pub downloaded: u64,
        pub total: Option<u64>,
    }

    /// Applied to every request the updater makes (manifest and download).
    /// Runs after the plugin's own settings, so it has the last word.
    fn client_rules(
        proxy: Option<rq::Proxy>,
        tor: bool,
    ) -> impl Fn(rq::ClientBuilder) -> rq::ClientBuilder + Send + Sync + 'static {
        move |b| {
            // GitHub refuses requests without a User-Agent; send the same
            // Tor Browser one as the bounty radar.
            let b = b
                .user_agent(BROWSER_UA)
                .referer(false)
                .connect_timeout(Duration::from_secs(if tor { 45 } else { 15 }))
                // per read, not for the whole file: an AppImage over Tor
                // can take minutes
                .read_timeout(Duration::from_secs(90))
                .redirect(rq::redirect::Policy::custom(|attempt| {
                    if attempt.previous().len() >= MAX_REDIRECTS {
                        attempt.error("too many redirects")
                    } else if update_url_allowed(attempt.url()) {
                        attempt.follow()
                    } else {
                        let host = attempt.url().host_str().unwrap_or("?").to_string();
                        attempt.error(format!("update redirect to {host} is not allowed"))
                    }
                }))
                .no_proxy(); // never the system or environment proxy
            match &proxy {
                Some(p) => b.proxy(p.clone()),
                None => b,
            }
        }
    }

    pub async fn check<R: Runtime>(
        app: &AppHandle<R>,
        pending: &PendingUpdate,
        cfg: &NetworkPrivacyConfig,
        allow_clearnet: bool,
    ) -> Result<UpdateInfo, String> {
        let proxy = route(cfg, allow_clearnet)?;
        let via_tor = proxy.is_some();
        let rq_proxy = proxy
            .as_deref()
            .map(rq::Proxy::all)
            .transpose()
            .map_err(|e| format!("NET:BAD_PROXY|{e}"))?;
        let endpoint = Url::parse(UPDATE_ENDPOINT).map_err(|e| e.to_string())?;
        let updater = app
            .updater_builder()
            .endpoints(vec![endpoint])
            .map_err(|e| e.to_string())?
            .configure_client(client_rules(rq_proxy, via_tor))
            .build()
            .map_err(|e| e.to_string())?;
        let found = updater.check().await.map_err(|e| e.to_string())?;
        let current = app.package_info().version.to_string();
        let info = match &found {
            Some(u) => {
                if !update_url_allowed(&u.download_url) {
                    return Err(format!(
                        "the update manifest points outside GitHub ({}); refusing it",
                        u.download_url.host_str().unwrap_or("?")
                    ));
                }
                UpdateInfo {
                    current,
                    available: Some(u.version.clone()),
                    notes: u.body.clone().filter(|b| !b.trim().is_empty()),
                    date: u.date.map(|d| d.to_string()),
                    via_tor,
                }
            }
            None => UpdateInfo {
                current,
                available: None,
                notes: None,
                date: None,
                via_tor,
            },
        };
        *pending.update.lock().await = found;
        Ok(info)
    }

    /// Downloads with the same rules as the check, verifies the signature
    /// and installs. On Windows the installer takes over and the app exits;
    /// elsewhere the app restarts into the new version.
    pub async fn install<R: Runtime>(
        app: &AppHandle<R>,
        pending: &PendingUpdate,
    ) -> Result<(), String> {
        let update = pending
            .update
            .lock()
            .await
            .clone()
            .ok_or("Check for updates first")?;
        if !update_url_allowed(&update.download_url) {
            return Err("the update download is not on GitHub; refusing it".into());
        }
        let (done, size) = (pending.downloaded.clone(), pending.total.clone());
        done.store(0, Ordering::Relaxed);
        size.store(0, Ordering::Relaxed);
        let bytes = update
            .download(
                move |chunk, total| {
                    done.fetch_add(chunk as u64, Ordering::Relaxed);
                    size.store(total.unwrap_or(0), Ordering::Relaxed);
                },
                || {},
            )
            .await
            .map_err(|e| e.to_string())?;
        // `install` verifies the signature (and the signed version) again
        // before touching anything.
        update.install(bytes).map_err(|e| e.to_string())?;
        app.restart()
    }

    #[tauri::command]
    pub async fn sn_check_update<R: Runtime>(
        app: AppHandle<R>,
        pending: State<'_, PendingUpdate>,
        privacy: Option<NetworkPrivacyConfig>,
        allow_clearnet: Option<bool>,
    ) -> Result<UpdateInfo, String> {
        check(
            &app,
            &pending,
            &privacy.unwrap_or_default(),
            allow_clearnet.unwrap_or(false),
        )
        .await
    }

    #[tauri::command]
    pub fn sn_update_progress(pending: State<'_, PendingUpdate>) -> UpdateProgress {
        let total = pending.total.load(Ordering::Relaxed);
        UpdateProgress {
            downloaded: pending.downloaded.load(Ordering::Relaxed),
            total: (total > 0).then_some(total),
        }
    }

    #[tauri::command]
    pub async fn sn_install_update<R: Runtime>(
        app: AppHandle<R>,
        pending: State<'_, PendingUpdate>,
    ) -> Result<(), String> {
        install(&app, &pending).await
    }
}

#[cfg(desktop)]
pub use desktop::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(tor: bool, proxy: &str) -> NetworkPrivacyConfig {
        NetworkPrivacyConfig {
            use_tor: tor,
            socks5_proxy: proxy.into(),
            ..Default::default()
        }
    }

    #[test]
    fn only_github_hosts_over_https() {
        for ok in [
            UPDATE_ENDPOINT,
            "https://objects.githubusercontent.com/x/y",
            "https://release-assets.githubusercontent.com/github-production-release-asset/1/2?sig=x",
            "https://GitHub.com/UBro-dev/stacker-pulse/releases/download/v3.2.0/a.msi",
        ] {
            assert!(update_url_allowed(&Url::parse(ok).unwrap()), "{ok}");
        }
        for bad in [
            "http://github.com/UBro-dev/stacker-pulse/releases/latest/download/latest.json",
            "https://github.com.evil.example/latest.json",
            "https://evil.example/github.com",
            "https://user:pw@github.com/x",
            "https://github.com:8443/x",
            "https://api.github.com/repos/x",
            "https://raw.githubusercontent.com/x",
            "file:///etc/passwd",
        ] {
            assert!(!update_url_allowed(&Url::parse(bad).unwrap()), "{bad}");
        }
    }

    #[test]
    fn routing_follows_the_privacy_settings() {
        // Airgap wins over everything.
        let mut air = cfg(true, "socks5h://127.0.0.1:9150");
        air.airgap_mode = true;
        assert!(route(&air, true).unwrap_err().contains("Airgap"));
        // No Tor: only with explicit consent.
        assert_eq!(route(&cfg(false, ""), false).unwrap_err(), ERR_TOR_REQUIRED);
        assert_eq!(route(&cfg(false, ""), true).unwrap(), None);
        // Tor: socks5h only, so DNS stays inside Tor.
        assert_eq!(
            route(&cfg(true, " socks5h://127.0.0.1:9150 "), false).unwrap(),
            Some("socks5h://127.0.0.1:9150".into())
        );
        assert_eq!(
            route(&cfg(true, ""), false).unwrap(),
            Some("socks5h://127.0.0.1:9050".into())
        );
        for bad in [
            "socks5://127.0.0.1:9050",
            "http://127.0.0.1:8118",
            "socks5h://",
        ] {
            assert!(
                route(&cfg(true, bad), true)
                    .unwrap_err()
                    .starts_with("NET:BAD_PROXY|"),
                "{bad}"
            );
        }
    }

    #[test]
    fn config_matches_the_code() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let up = &conf["plugins"]["updater"];
        assert_eq!(up["endpoints"], serde_json::json!([UPDATE_ENDPOINT]));
        assert_eq!(
            up["requireSignedVersion"], true,
            "signatures must be bound to the version"
        );
        assert_eq!(
            up["dangerousInsecureTransportProtocol"],
            serde_json::Value::Null
        );
        assert_eq!(conf["bundle"]["createUpdaterArtifacts"], true);
        let key = up["pubkey"].as_str().unwrap();
        assert!(!key.trim().is_empty(), "updater pubkey must be set");
    }
}
