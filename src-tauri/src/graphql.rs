//! StackerPulse Zero-Telemetry & Privacy-by-Design GraphQL Transport
//! Author: UBro-dev
//! License: MIT (Copyright (c) 2026 UBro-dev)
//!
//! Privacy by Design Guarantees:
//!   1. Strict Outbound Allowlist: Only `stacker.news` and its official `.onion` v3 hidden service
//!      can ever be contacted. Any other host is rejected before opening a socket.
//!   2. Airgap Lock Mode (`airgap_mode: true`): Blocks 100% of network activity when using
//!      local tools like SeedScan or offline Post Studio.
//!   3. Anti-Fingerprinting: Strips Referer, Cookie, Accept-Language (OS locale leak), and custom UA.

use reqwest::{redirect::Policy, Client, Proxy};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use std::time::Duration;
use url::Url;

pub const SN_CLEARNET_ENDPOINT: &str = "https://stacker.news/api/graphql";
pub const SN_ONION_ENDPOINT: &str =
    "http://snsnsnya6h3ot563f3p566wuhfoklkg5f62hokdlaqzcaub3gf4xlxyd.onion/api/graphql";

const ALLOWED_HOSTS: &[&str] = &[
    "stacker.news",
    "snsnsnya6h3ot563f3p566wuhfoklkg5f62hokdlaqzcaub3gf4xlxyd.onion",
];

/// User-Agent sent when the user turns off "strip User-Agent", and always to
/// GitHub (which rejects requests without one). Matches Tor Browser's UA
/// (Firefox ESR on Windows), the most common UA among Tor users. Update it
/// when Tor Browser moves to a new ESR.
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

pub const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; rv:140.0) Gecko/20100101 Firefox/140.0";

/// `#[serde(default)]`: any field the UI omits falls back to `Default`
/// instead of failing the whole call. The UI does not send `airgap_mode`,
/// and without this every `sn_execute_graphql` invocation was rejected with
/// "missing field `airgap_mode`".
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct NetworkPrivacyConfig {
    pub airgap_mode: bool,
    pub use_tor: bool,
    pub socks5_proxy: String,
    pub use_onion_endpoint: bool,
    pub strip_user_agent: bool,
    /// With Tor on: if the .onion service does not answer, retry once via
    /// https://stacker.news through the same Tor proxy (IP stays hidden).
    pub onion_fallback: bool,
}

impl Default for NetworkPrivacyConfig {
    fn default() -> Self {
        Self {
            airgap_mode: false,
            use_tor: false,
            socks5_proxy: "socks5h://127.0.0.1:9050".to_string(),
            use_onion_endpoint: true,
            strip_user_agent: true,
            onion_fallback: true,
        }
    }
}

fn host_in(list: &[&str], host: &str) -> bool {
    let host = host.to_lowercase();
    list.iter().any(|&allowed| allowed == host)
}

fn host_is_allowlisted(host: &str) -> bool {
    host_in(ALLOWED_HOSTS, host)
}

/// Redirect policy that re-applies the allowlist on every hop.
///
/// reqwest's default policy follows up to 10 redirects to any host, which
/// would let a single `302` take the client outside the allowlist and, on
/// clearnet, reveal the user's IP to that host. This policy stops the chain
/// the moment a hop leaves the allowlist, and caps the chain at 3.
fn allowlist_redirect_policy() -> Policy {
    redirect_policy_for(ALLOWED_HOSTS)
}

/// The policy itself, parameterised by allowlist so tests exercise the exact
/// code production runs, just with a local host in the list.
fn redirect_policy_for(allowed_hosts: &'static [&'static str]) -> Policy {
    Policy::custom(move |attempt| {
        let host = attempt.url().host_str().unwrap_or("<none>").to_string();
        let allowed = host_in(allowed_hosts, &host);
        if !allowed {
            attempt.error(format!(
                "Blocked by UBro-dev Privacy-by-Design Firewall: redirect to '{host}' is not in allowlist"
            ))
        } else if attempt.previous().len() >= 3 {
            attempt.error("Blocked: too many redirects")
        } else {
            attempt.follow()
        }
    })
}

/// Validates that the destination URL is strictly inside the Privacy-by-Design allowlist.
pub fn verify_allowlisted_endpoint(endpoint_url: &str) -> Result<(), String> {
    let parsed = Url::parse(endpoint_url).map_err(|e| format!("Invalid endpoint URL: {e}"))?;
    let host = parsed.host_str().unwrap_or_default().to_lowercase();
    if host_is_allowlisted(&host) {
        Ok(())
    } else {
        Err(format!(
            "Blocked by UBro-dev Privacy-by-Design Firewall: host '{host}' is not in allowlist"
        ))
    }
}

pub struct SnGraphqlClient {
    http: Client,
    endpoint: String,
    /// Clearnet endpoint tried through Tor when the .onion does not answer.
    fallback_endpoint: Option<String>,
    use_tor: bool,
    proxy_url: String,
    airgap_mode: bool,
    /// Set when this client's answer came via the backup route.
    fell_back: AtomicBool,
}

// Process-wide memory of whether SN's onion answered recently.
const ONION_UNKNOWN: u8 = 0;
const ONION_UP: u8 = 1;
const ONION_DOWN: u8 = 2;
static ONION_STATE: AtomicU8 = AtomicU8::new(ONION_UNKNOWN);
static ONION_DOWN_AT: AtomicU64 = AtomicU64::new(0);
/// How long to skip the onion after it failed before trying it again.
const ONION_RETRY_AFTER_SECS: u64 = 600;

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn onion_recently_down() -> bool {
    let at = ONION_DOWN_AT.load(Ordering::Relaxed);
    at != 0 && now_secs().saturating_sub(at) < ONION_RETRY_AFTER_SECS
}

/// "unknown" | "up" | "down" — for the UI.
pub fn onion_status() -> &'static str {
    match ONION_STATE.load(Ordering::Relaxed) {
        ONION_UP => "up",
        ONION_DOWN => "down",
        _ => "unknown",
    }
}

/// Forget that the onion was down (the user asked to try it again).
pub fn reset_onion_status() {
    ONION_DOWN_AT.store(0, Ordering::Relaxed);
    ONION_STATE.store(ONION_UNKNOWN, Ordering::Relaxed);
}

/// Tor ports to look for: Tor Browser first, then the tor service.
pub const TOR_PORTS: [u16; 2] = [9150, 9050];

/// True if something accepts TCP connections on 127.0.0.1:port.
pub fn local_port_open(port: u16) -> bool {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(800)).is_ok()
}

/// Full error text including every `source()`, without the URL.
fn error_chain(e: &reqwest::Error) -> String {
    use std::error::Error as _;
    let mut out = e.to_string();
    let mut src = e.source();
    while let Some(x) = src {
        let t = x.to_string();
        if !out.contains(&t) {
            out.push_str(": ");
            out.push_str(&t);
        }
        src = x.source();
    }
    out
}

/// Network failure kinds the UI explains in the user's language. The
/// message is `NET:<KIND>|<detail>`; the UI maps KIND to text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetKind {
    TorRefused,
    Timeout,
    OnionUnreachable,
    Tls,
    Other,
}

impl NetKind {
    fn tag(self) -> &'static str {
        match self {
            NetKind::TorRefused => "TOR_REFUSED",
            NetKind::Timeout => "TIMEOUT",
            NetKind::OnionUnreachable => "SOCKS_FAIL",
            NetKind::Tls => "TLS",
            NetKind::Other => "OTHER",
        }
    }
}

pub fn classify(chain: &str, timed_out: bool, use_tor: bool) -> NetKind {
    let c = chain.to_lowercase();
    // reqwest drops the io error behind "failed to create underlying
    // connection", so that phrase itself means: nothing answered on the proxy port.
    let proxy_down = c.contains("refused")
        || c.contains("10061")
        || c.contains("os error 111")
        || c.contains("failed to create underlying connection");
    if use_tor && proxy_down {
        NetKind::TorRefused
    } else if timed_out
        || c.contains("timed out")
        || c.contains("ttl expired")
        || c.contains("10060")
    {
        NetKind::Timeout
    } else if use_tor && c.contains("socks") {
        NetKind::OnionUnreachable
    } else if c.contains("certificate") || c.contains("tls") {
        NetKind::Tls
    } else {
        NetKind::Other
    }
}

fn port_of(proxy_url: &str) -> Option<u16> {
    Url::parse(proxy_url).ok()?.port()
}

impl SnGraphqlClient {
    pub fn with_privacy_config(cfg: &NetworkPrivacyConfig) -> Result<Self, String> {
        // Airgap means no network-capable client exists at all, rather than a
        // client that every call site has to remember not to use.
        if cfg.airgap_mode {
            return Err("Blocked by Airgap Mode (100% Offline Privacy Lock is active)".to_string());
        }

        let onion = cfg.use_tor && cfg.use_onion_endpoint;
        let endpoint = if onion {
            SN_ONION_ENDPOINT
        } else {
            SN_CLEARNET_ENDPOINT
        }
        .to_string();
        let fallback_endpoint =
            (onion && cfg.onion_fallback).then(|| SN_CLEARNET_ENDPOINT.to_string());

        verify_allowlisted_endpoint(&endpoint)?;

        // Onion circuits can take well over 20 s to build on a cold Tor.
        let mut builder = Client::builder()
            .no_proxy() // never pick up HTTP(S)_PROXY or the Windows system proxy
            .connect_timeout(Duration::from_secs(if cfg.use_tor { 45 } else { 10 }))
            .timeout(Duration::from_secs(if cfg.use_tor { 60 } else { 15 }))
            .gzip(true)
            .referer(false)
            .redirect(allowlist_redirect_policy());

        if !cfg.strip_user_agent {
            builder = builder.user_agent(BROWSER_UA);
        }

        let proxy_url = if cfg.socks5_proxy.trim().is_empty() {
            "socks5h://127.0.0.1:9050".to_string()
        } else {
            cfg.socks5_proxy.trim().to_string()
        };
        if cfg.use_tor {
            let proxy_url = proxy_url.as_str();
            if !proxy_url.starts_with("socks5h://") {
                return Err(format!("NET:BAD_PROXY|{proxy_url}"));
            }
            let proxy = Proxy::all(proxy_url)
                .map_err(|e| format!("Invalid Tor SOCKS5h proxy URI ({proxy_url}): {e}"))?;
            builder = builder.proxy(proxy);
        }

        let http = builder
            .build()
            .map_err(|e| format!("Failed to initialize zero-telemetry reqwest client: {e}"))?;

        Ok(Self {
            http,
            endpoint,
            fallback_endpoint,
            use_tor: cfg.use_tor,
            proxy_url,
            airgap_mode: cfg.airgap_mode,
            fell_back: AtomicBool::new(false),
        })
    }

    pub async fn execute_raw(
        &self,
        query: &str,
        variables: Option<Value>,
    ) -> Result<Value, String> {
        if self.airgap_mode {
            return Err("Blocked by Airgap Mode (100% Offline Privacy Lock is active)".to_string());
        }
        let body = json!({
            "query": query,
            "variables": variables.unwrap_or_else(|| json!({}))
        });
        // The onion failed a moment ago: go straight to the backup route
        // instead of waiting up to a minute for it again.
        if let Some(fb) = &self.fallback_endpoint {
            if onion_recently_down() {
                let r = self.post_once(fb, &body).await.map_err(|(_, m)| m);
                if r.is_ok() {
                    self.fell_back.store(true, Ordering::Relaxed);
                }
                return r;
            }
        }
        match self.post_once(&self.endpoint, &body).await {
            Ok(v) => {
                if self.endpoint == SN_ONION_ENDPOINT {
                    ONION_DOWN_AT.store(0, Ordering::Relaxed);
                    ONION_STATE.store(ONION_UP, Ordering::Relaxed);
                }
                Ok(v)
            }
            Err((kind, msg)) => {
                // Tor works but the onion did not answer: same Tor proxy, clearnet host.
                let retry = matches!(
                    kind,
                    Some(NetKind::Timeout) | Some(NetKind::OnionUnreachable)
                );
                if retry && self.endpoint == SN_ONION_ENDPOINT {
                    ONION_DOWN_AT.store(now_secs(), Ordering::Relaxed);
                    ONION_STATE.store(ONION_DOWN, Ordering::Relaxed);
                }
                match (&self.fallback_endpoint, retry) {
                    (Some(fb), true) => {
                        let r = self.post_once(fb, &body).await.map_err(|(_, m2)| {
                            format!(
                                "{msg}\n(.onion) · stacker.news via Tor: {}",
                                m2.trim_start_matches("NET:")
                            )
                        });
                        if r.is_ok() {
                            self.fell_back.store(true, Ordering::Relaxed);
                        }
                        r
                    }
                    _ => Err(msg),
                }
            }
        }
    }

    /// Error side: (network kind if it was a network failure, message).
    async fn post_once(
        &self,
        endpoint: &str,
        body: &Value,
    ) -> Result<Value, (Option<NetKind>, String)> {
        verify_allowlisted_endpoint(endpoint).map_err(|e| (None, e))?;
        let response = self
            .http
            .post(endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Cache-Control", "no-store")
            .json(body)
            .send()
            .await
            .map_err(|e| {
                let timed_out = e.is_timeout();
                let chain = error_chain(&e.without_url());
                let kind = classify(&chain, timed_out, self.use_tor);
                let mut detail = chain.chars().take(400).collect::<String>();
                if kind == NetKind::TorRefused {
                    // Say where Tor actually is, if it is running on the other port.
                    let configured = port_of(&self.proxy_url);
                    let found = TOR_PORTS
                        .iter()
                        .copied()
                        .find(|&p| Some(p) != configured && local_port_open(p));
                    detail = format!(
                        "{}|{}",
                        self.proxy_url,
                        found.map(|p| p.to_string()).unwrap_or_default()
                    );
                }
                (Some(kind), format!("NET:{}|{detail}", kind.tag()))
            })?;
        let endpoint = endpoint.to_string();

        let status = response.status();
        let mut response = response;
        // Cap the body: a GraphQL answer for these queries is far below this.
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|e| {
            (
                None,
                format!("Reading the response from {endpoint} failed: {e}"),
            )
        })? {
            if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return Err((
                    None,
                    "Stacker News response is larger than 4 MB; refused".to_string(),
                ));
            }
            body.extend_from_slice(&chunk);
        }
        let payload: Value = serde_json::from_slice(&body).map_err(|e| {
            (
                None,
                format!("Stacker News answered HTTP {status}, not JSON: {e}"),
            )
        })?;

        Ok(payload)
    }

    /// Like `execute_raw`, but turns HTTP errors and GraphQL `errors` into Err,
    /// for the typed fetchers in sn.rs. The playground keeps the raw answer.
    pub async fn query_data(&self, query: &str, variables: Value) -> Result<Value, String> {
        let payload = self.execute_raw(query, Some(variables)).await?;
        if let Some(msg) = payload
            .get("errors")
            .and_then(|e| e.as_array())
            .and_then(|a| a.first())
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
        {
            return Err(format!(
                "Stacker News: {}",
                msg.chars().take(300).collect::<String>()
            ));
        }
        payload
            .get("data")
            .cloned()
            .ok_or_else(|| "Stacker News returned no data".to_string())
    }

    /// Links point at the onion only if the onion actually answered; if the
    /// backup route was used, an onion link would not open either.
    pub fn onion_links(&self) -> bool {
        self.endpoint == SN_ONION_ENDPOINT && !self.fell_back.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_accepts_only_exact_hosts() {
        assert!(verify_allowlisted_endpoint(SN_CLEARNET_ENDPOINT).is_ok());
        assert!(verify_allowlisted_endpoint(SN_ONION_ENDPOINT).is_ok());
        assert!(verify_allowlisted_endpoint("https://STACKER.NEWS/api/graphql").is_ok());
        for bad in [
            "https://stacker.news.evil.com/api/graphql",
            "https://evilstacker.news/api/graphql",
            "https://evil.com/?stacker.news",
            "https://stacker.news@evil.com/",
        ] {
            assert!(verify_allowlisted_endpoint(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn airgap_refuses_to_build_a_client() {
        let cfg = NetworkPrivacyConfig {
            airgap_mode: true,
            ..Default::default()
        };
        assert!(SnGraphqlClient::with_privacy_config(&cfg).is_err());
    }

    /// Spin up two local servers: an origin that answers 302 to a second,
    /// "evil" server. Returns (origin_port, evil_listener).
    fn redirect_fixture() -> (u16, std::net::TcpListener) {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let evil = TcpListener::bind("127.0.0.1:0").unwrap();
        let evil_port = evil.local_addr().unwrap().port();
        evil.set_nonblocking(true).unwrap();

        let origin = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin_port = origin.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = origin.accept() {
                let mut buf = [0u8; 2048];
                let _ = s.read(&mut buf);
                // "localhost" is a different host string from "127.0.0.1",
                // so it is outside the test allowlist.
                let resp = format!(
                    "HTTP/1.1 302 Found\r\nLocation: http://localhost:{evil_port}/steal\r\nContent-Length: 0\r\n\r\n"
                );
                let _ = s.write_all(resp.as_bytes());
            }
        });
        (origin_port, evil)
    }

    fn was_contacted(evil: &std::net::TcpListener) -> bool {
        std::thread::sleep(std::time::Duration::from_millis(300));
        evil.accept().is_ok()
    }

    /// Control: reqwest's DEFAULT policy (what the code used before the fix)
    /// follows the redirect and connects to the evil host. This is H-1.
    #[tokio::test]
    async fn control_default_policy_reaches_the_evil_host() {
        let (origin_port, evil) = redirect_fixture();
        let client = Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .unwrap();
        let _ = client
            .post(format!("http://127.0.0.1:{origin_port}/api/graphql"))
            .body("{}")
            .send()
            .await;
        assert!(
            was_contacted(&evil),
            "default policy should follow the redirect (bug reproduced)"
        );
    }

    /// Fix: the production redirect policy, with only 127.0.0.1 allowlisted,
    /// refuses the hop and the evil host is never contacted.
    #[tokio::test]
    async fn fixed_policy_blocks_redirect_out_of_allowlist() {
        static TEST_ALLOWLIST: &[&str] = &["127.0.0.1"];
        let (origin_port, evil) = redirect_fixture();
        let client = Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(3))
            .redirect(redirect_policy_for(TEST_ALLOWLIST))
            .build()
            .unwrap();
        let res = client
            .post(format!("http://127.0.0.1:{origin_port}/api/graphql"))
            .body("{}")
            .send()
            .await;
        assert!(res.is_err(), "redirect should have been refused");
        assert!(!was_contacted(&evil), "non-allowlisted host was contacted");
    }
}

#[cfg(test)]
mod ui_contract {
    use super::*;
    /// Exactly the object ui/index.html sends to sn_execute_graphql.
    #[test]
    fn ui_privacy_object_deserializes() {
        let from_ui = r#"{"use_tor":false,"socks5_proxy":"","use_onion_endpoint":false,"strip_user_agent":true}"#;
        let r: Result<NetworkPrivacyConfig, _> = serde_json::from_str(from_ui);
        println!(
            "  result: {:?}",
            r.as_ref().map(|_| "ok").map_err(|e| e.to_string())
        );
        assert!(r.is_ok());
    }
}

#[cfg(test)]
mod tor_errors {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    fn tor_cfg(proxy: String, fallback: bool) -> NetworkPrivacyConfig {
        NetworkPrivacyConfig {
            use_tor: true,
            socks5_proxy: proxy,
            use_onion_endpoint: true,
            onion_fallback: fallback,
            ..Default::default()
        }
    }

    /// Tests that touch the process-wide onion state run one at a time.
    static STATE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    fn closed_port() -> u16 {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port() // listener dropped: nothing listens here
    }

    /// Minimal SOCKS5 server: records each CONNECT host and answers
    /// "host unreachable" (0x04), like Tor does for an onion it cannot reach.
    fn unreachable_socks(conns: usize) -> (u16, Arc<Mutex<Vec<String>>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen2 = seen.clone();
        std::thread::spawn(move || {
            for _ in 0..conns {
                let Ok((mut s, _)) = l.accept() else { return };
                let mut b = [0u8; 512];
                let _ = s.read(&mut b); // greeting
                let _ = s.write_all(&[5, 0]);
                let n = s.read(&mut b).unwrap_or(0);
                if n > 5 && b[3] == 3 {
                    let len = b[4] as usize;
                    seen2
                        .lock()
                        .unwrap()
                        .push(String::from_utf8_lossy(&b[5..5 + len]).to_string());
                }
                let _ = s.write_all(&[5, 4, 0, 1, 0, 0, 0, 0, 0, 0]);
            }
        });
        (port, seen)
    }

    #[tokio::test]
    async fn nothing_on_the_tor_port_is_reported_as_tor_refused() {
        let _g = STATE.lock().await;
        reset_onion_status();
        let port = closed_port();
        let client = SnGraphqlClient::with_privacy_config(&tor_cfg(
            format!("socks5h://127.0.0.1:{port}"),
            true,
        ))
        .unwrap();
        let err = client
            .execute_raw("{ __typename }", None)
            .await
            .unwrap_err();
        assert!(
            err.starts_with(&format!("NET:TOR_REFUSED|socks5h://127.0.0.1:{port}|")),
            "{err}"
        );
    }

    #[tokio::test]
    async fn unreachable_onion_falls_back_to_stacker_news_through_the_same_tor() {
        let _g = STATE.lock().await;
        reset_onion_status();
        let (port, seen) = unreachable_socks(3);
        let client = SnGraphqlClient::with_privacy_config(&tor_cfg(
            format!("socks5h://127.0.0.1:{port}"),
            true,
        ))
        .unwrap();
        let err = client
            .execute_raw("{ __typename }", None)
            .await
            .unwrap_err();
        assert!(err.starts_with("NET:SOCKS_FAIL|"), "{err}");
        let hosts = seen.lock().unwrap().clone();
        assert_eq!(
            hosts,
            vec![
                "snsnsnya6h3ot563f3p566wuhfoklkg5f62hokdlaqzcaub3gf4xlxyd.onion".to_string(),
                "stacker.news".to_string(),
            ]
        );
        assert_eq!(onion_status(), "down");
        // Second request: the dead onion is skipped, straight to stacker.news.
        let client = SnGraphqlClient::with_privacy_config(&tor_cfg(
            format!("socks5h://127.0.0.1:{port}"),
            true,
        ))
        .unwrap();
        assert!(client.execute_raw("{ __typename }", None).await.is_err());
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(
            seen.lock().unwrap().last().map(String::as_str),
            Some("stacker.news")
        );
        assert_eq!(seen.lock().unwrap().len(), 3);
        reset_onion_status();
        assert_eq!(onion_status(), "unknown");
    }

    #[test]
    fn links_stop_pointing_at_the_onion_after_the_backup_route_answered() {
        let client =
            SnGraphqlClient::with_privacy_config(&tor_cfg("socks5h://127.0.0.1:9150".into(), true))
                .unwrap();
        assert!(client.onion_links());
        client.fell_back.store(true, Ordering::Relaxed);
        assert!(!client.onion_links());
    }

    #[tokio::test]
    async fn no_fallback_when_the_user_turned_it_off() {
        let _g = STATE.lock().await;
        reset_onion_status();
        let (port, seen) = unreachable_socks(2);
        let client = SnGraphqlClient::with_privacy_config(&tor_cfg(
            format!("socks5h://127.0.0.1:{port}"),
            false,
        ))
        .unwrap();
        assert!(client.execute_raw("{ __typename }", None).await.is_err());
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(seen.lock().unwrap().len(), 1);
    }

    #[test]
    fn socks5_without_h_is_refused_because_dns_would_leak() {
        let r =
            SnGraphqlClient::with_privacy_config(&tor_cfg("socks5://127.0.0.1:9050".into(), true));
        assert!(matches!(r, Err(e) if e.starts_with("NET:BAD_PROXY|")));
    }

    #[test]
    fn windows_and_linux_error_texts_are_classified() {
        let win = "error sending request: client error (Connect): socks connect error: No connection could be made because the target machine actively refused it. (os error 10061)";
        assert_eq!(classify(win, false, true), NetKind::TorRefused);
        assert_eq!(
            classify(
                "socks connect error: Connection refused (os error 111)",
                false,
                true
            ),
            NetKind::TorRefused
        );
        let reqwest_012 = "error sending request: client error (Connect): error connecting to socks proxy: SOCKS error: failed to create underlying connection";
        assert_eq!(classify(reqwest_012, false, true), NetKind::TorRefused);
        assert_eq!(
            classify("socks connect error: Host unreachable", false, true),
            NetKind::OnionUnreachable
        );
        assert_eq!(
            classify("operation timed out", true, true),
            NetKind::Timeout
        );
        assert_eq!(
            classify("invalid peer certificate: UnknownIssuer", false, false),
            NetKind::Tls
        );
    }
}
