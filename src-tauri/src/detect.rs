//! Secret & Deanonymization Detectors (SeedScan Engine)
//! Author: UBro-dev
//! License: MIT (Copyright (c) 2026 UBro-dev)
//!
//! Every detector validates a cryptographic checksum where the format has one:
//! - BIP39 seed phrases (12/15/18/21/24 words) -> BIP39 checksum validation
//! - Extended private keys (xprv/yprv/zprv/tprv/uprv/vprv) -> Base58Check validation
//! - WIF private keys (5..., K..., L..., 9..., c...) -> Base58Check validation
//! - Nostr secret keys (nsec1...) -> NIP-19 Bech32 checksum validation
//! - Solana keypair files (id.json) -> 64-byte JSON array validation
//! - Lightning Nostr Wallet Connect (nostr+walletconnect://?secret=...) -> Secret leak detection
//! - OS Path Deanonymization (/home/<user>/, C:\Users\<user>\) -> OpSec pre-post scrubbing

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::OnceLock;

use regex::Regex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Bip39Mnemonic,
    ExtendedPrivateKey,
    WifPrivateKey,
    NostrSecretKey,
    SolanaKeypairFile,
    NwcWalletSecret,
    OsUserPathLeak,
    /// 12–24 BIP39 words in a row whose checksum does not match: a seed with
    /// a typo or a missing word is still a seed (text mode only).
    PossibleMnemonic,
    /// A Solana secret key as one base58 string (Phantom/Solflare export).
    SolanaPrivateKey,
    /// Text mode: an e-mail address in a log or draft.
    EmailAddress,
    /// Text mode: a public IPv4 address in a log or draft.
    PublicIpAddress,
}

/// Disk: many files nobody is about to publish — report only real secrets
/// and paths with *your own* username. Text: a draft or pasted log you are
/// about to post — report any username path and near-miss seed phrases too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Disk,
    Text,
}

impl Kind {
    pub fn label(&self) -> &'static str {
        match self {
            Kind::Bip39Mnemonic => "BIP39 seed phrase",
            Kind::ExtendedPrivateKey => "extended private key (xprv/zprv)",
            Kind::WifPrivateKey => "WIF private key",
            Kind::NostrSecretKey => "Nostr secret key (nsec)",
            Kind::SolanaKeypairFile => "Solana keypair file",
            Kind::NwcWalletSecret => "Lightning NWC secret URI",
            Kind::OsUserPathLeak => "OS username path leak (deanonymization)",
            Kind::PossibleMnemonic => "possible seed phrase (checksum does not match)",
            Kind::SolanaPrivateKey => "Solana private key (base58)",
            Kind::EmailAddress => "e-mail address",
            Kind::PublicIpAddress => "public IP address",
        }
    }

    pub fn advice(&self) -> &'static str {
        match self {
            Kind::Bip39Mnemonic => {
                "Anyone who reads this file controls the wallet. Move it offline \
                 (paper or a hardware wallet) and delete the file."
            }
            Kind::ExtendedPrivateKey => {
                "An xprv derives every child key of the wallet. Treat it like the \
                 seed phrase itself."
            }
            Kind::WifPrivateKey => {
                "This is a single-address private key. Sweep the funds to a new \
                 address, then delete the file."
            }
            Kind::NostrSecretKey => {
                "An nsec is your Nostr identity. Anyone holding it can post as you. \
                 Rotate it if this file was ever shared or synced."
            }
            Kind::SolanaKeypairFile => {
                "A Solana keypair file is an unencrypted private key. Keep it out of \
                 synced or shared folders, and never commit it."
            }
            Kind::NwcWalletSecret => {
                "A nostr+walletconnect:// URI with a secret parameter allows spending \
                 from your Lightning wallet. Revoke this connection in Alby/LNbits immediately."
            }
            Kind::OsUserPathLeak => {
                "Contains a local OS home path (/home/user or C:\\Users\\user) that can \
                 deanonymize your real username when posting logs or stack traces on Stacker News."
            }
            Kind::PossibleMnemonic => {
                "These are BIP39 words in a row, but the checksum does not match. If this is \
                 your seed with a typo or a missing word, it can still be recovered by brute \
                 force — treat it as a real seed."
            }
            Kind::SolanaPrivateKey => {
                "A base58 Solana secret key (the format wallets export). It controls the \
                 account: move the funds to a new wallet and delete it."
            }
            Kind::EmailAddress => {
                "An e-mail address can link this text to your identity. Remove it before \
                 posting."
            }
            Kind::PublicIpAddress => {
                "A public IP address can reveal your location or provider. Remove it before \
                 posting logs."
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub kind: Kind,
    pub label: String,
    /// 1-based line number, or 0 when the whole file is the secret.
    pub line: usize,
    /// Already masked. The real secret is never stored here.
    pub masked: String,
    pub advice: String,
    /// A well-known example from a spec or docs (BIP39, BIP32, NIP-19,
    /// NIP-47…), not someone's private key.
    #[serde(default)]
    pub test_vector: bool,
}

impl Finding {
    pub fn new(kind: Kind, line: usize, masked: String) -> Self {
        Self {
            label: kind.label().to_string(),
            advice: kind.advice().to_string(),
            kind,
            line,
            masked,
            test_vector: false,
        }
    }

    fn into_test_vector(mut self) -> Self {
        self.test_vector = true;
        self.label = format!("{} (public test vector)", self.kind.label());
        self.advice = "A well-known example from a specification or documentation, not a \
                       private key. Anyone can use it; funds sent to it are taken by bots \
                       within seconds."
            .to_string();
        self
    }
}

// ------------------------------------------------- known public examples

/// Secrets published in specs and docs. Finding them in a project's source
/// or tests is expected; they are reported as test vectors, not leaks.
const KNOWN_SECRETS: &[&str] = &[
    // BIP39 spec vectors (besides the "abandon … about" family, see below)
    "legal winner thank year wave sausage worth useful legal winner thank yellow",
    "letter advice cage absurd amount doctor acoustic avoid letter advice cage above",
    // BIP32 test vector 1, master xprv
    "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi",
    // Bitcoin wiki WIF example
    "5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ",
    // NIP-19 example nsec
    "nsec1vl029mgpspedva04g90vltkh6fvh240zqtv9k0t9af8935ke9laqsnlfe5",
    // NIP-47 example connection secret
    "71a8c14c1407c113601079c4302dab36460f0ccd0ad506f1f2dc73b5100e4f3c",
];

fn is_known_secret(s: &str) -> bool {
    KNOWN_SECRETS.iter().any(|k| s.contains(k))
}

/// "abandon abandon … about/agent/art", "zoo zoo … wrong/vote": every
/// word but the last is the same — the BIP39 spec's all-zero/all-one entropy.
fn is_repeated_word_vector(words: &[&str]) -> bool {
    words.len() >= 12 && words[..words.len() - 1].iter().all(|w| *w == words[0])
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} — {}", self.label, self.masked)
    }
}

/// Keep the first and last `edge` characters, blank out the middle.
/// Never returns enough of the secret to be useful to a shoulder-surfer.
pub fn mask(secret: &str, edge: usize) -> String {
    let chars: Vec<char> = secret.chars().collect();
    if chars.len() <= edge * 2 + 4 {
        return "*".repeat(chars.len().min(24));
    }
    let head: String = chars[..edge].iter().collect();
    let tail: String = chars[chars.len() - edge..].iter().collect();
    format!("{head}{}{tail}", "*".repeat(10))
}

/// Mask a mnemonic as "first ***** last (24 words)".
fn mask_mnemonic(words: &[&str]) -> String {
    format!(
        "{} ***** {} ({} words)",
        words[0],
        words[words.len() - 1],
        words.len()
    )
}

// ---------------------------------------------------------------- BIP39

const MNEMONIC_LENGTHS: [usize; 5] = [24, 21, 18, 15, 12];
/// A run of BIP39 words longer than this is a word list or prose, not a seed.
const MAX_RUN: usize = 30;

enum Tok {
    /// A BIP39 word (lowercased) and its line number.
    Word(String, usize),
    /// "1." / "12)" style numbering between words: ignored.
    Skip,
    /// Anything else ends a run of words.
    Break,
}

/// Seeds are written as plain words separated by spaces or new lines,
/// sometimes numbered ("1. abandon") or with commas. Code like
/// `target_width, crop_w)` or a quoted word list `'abandon','ability'` is not,
/// so a word glued to `_`, quotes or brackets ends the run instead of being
/// split out of it (that splitting caused false positives in Python files).
fn classify_token(tok: &str, line: usize) -> Tok {
    let t = tok.trim_end_matches([',', ';', '.']);
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 2 {
        return Tok::Break;
    }
    let mut rest = &t[digits..];
    if digits > 0 {
        rest = rest.strip_prefix(['.', ')', ':', '-']).unwrap_or(rest);
        if rest.is_empty() {
            return Tok::Skip;
        }
    }
    if !(3..=8).contains(&rest.len()) || !rest.chars().all(|c| c.is_ascii_alphabetic()) {
        return Tok::Break;
    }
    let lower = rest.to_ascii_lowercase();
    if bip39::Language::English.find_word(&lower).is_none() {
        return Tok::Break;
    }
    Tok::Word(lower, line)
}

/// Finds seed phrases in a whole text. Runs of words may span lines (one word
/// per line is a common backup format); a blank line ends a run.
pub fn find_mnemonics(content: &str, mode: Mode) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut run: Vec<(String, usize)> = Vec::new();
    let flush = |run: &mut Vec<(String, usize)>, out: &mut Vec<Finding>| {
        if (12..=MAX_RUN).contains(&run.len()) {
            evaluate_run(run, mode, out);
        }
        run.clear();
    };
    for (i, line) in content.lines().enumerate() {
        if line.len() > 20_000 {
            flush(&mut run, &mut out);
            continue;
        }
        if line.trim().is_empty() {
            flush(&mut run, &mut out);
            continue;
        }
        for tok in line.split_whitespace() {
            match classify_token(tok, i + 1) {
                Tok::Word(w, l) => run.push((w, l)),
                Tok::Skip => {}
                Tok::Break => flush(&mut run, &mut out),
            }
        }
    }
    flush(&mut run, &mut out);
    out
}

fn evaluate_run(run: &[(String, usize)], mode: Mode, out: &mut Vec<Finding>) {
    let words: Vec<&str> = run.iter().map(|(w, _)| w.as_str()).collect();
    let mut start = 0;
    let mut found_any = false;
    'outer: while start + 12 <= words.len() {
        for &len in &MNEMONIC_LENGTHS {
            if start + len > words.len() {
                continue;
            }
            let window = &words[start..start + len];
            if bip39::Mnemonic::parse_normalized(&window.join(" ")).is_ok() {
                let mut f = Finding::new(Kind::Bip39Mnemonic, run[start].1, mask_mnemonic(window));
                if is_repeated_word_vector(window) || is_known_secret(&window.join(" ")) {
                    f = f.into_test_vector();
                }
                out.push(f);
                found_any = true;
                start += len;
                continue 'outer;
            }
        }
        start += 1;
    }
    // Any 12+ BIP39 words in a row: a seed may sit next to another word
    // ("mail", "seed" and "wallet" are BIP39 words too).
    if !found_any && mode == Mode::Text && words.len() >= 12 {
        out.push(Finding::new(
            Kind::PossibleMnemonic,
            run[0].1,
            mask_mnemonic(&words),
        ));
    }
}

/// Single-line convenience used by tests.
#[cfg(test)]
pub fn find_bip39(line: &str) -> Option<Finding> {
    find_mnemonics(line, Mode::Disk).into_iter().next()
}

// ------------------------------------------------- base58check keys

fn base58_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[1-9A-HJ-NP-Za-km-z]{50,120}").unwrap())
}

pub fn find_base58_keys(line: &str) -> Vec<Finding> {
    let mut out = Vec::new();

    for m in base58_re().find_iter(line) {
        let s = m.as_str();

        let kind = if (111..=112).contains(&s.len()) {
            const XPRV_PREFIXES: [&str; 6] = ["xprv", "yprv", "zprv", "tprv", "uprv", "vprv"];
            if !XPRV_PREFIXES.iter().any(|p| s.starts_with(p)) {
                continue;
            }
            Kind::ExtendedPrivateKey
        } else if (86..=88).contains(&s.len()) {
            match bs58::decode(s).into_vec() {
                Ok(b) if is_solana_keypair(&b) => {
                    out.push(Finding::new(Kind::SolanaPrivateKey, 0, mask(s, 4)));
                }
                _ => {}
            }
            continue;
        } else if (51..=52).contains(&s.len()) {
            let first = s.chars().next().unwrap();
            if !matches!(first, '5' | 'K' | 'L' | '9' | 'c') {
                continue;
            }
            Kind::WifPrivateKey
        } else {
            continue;
        };

        if bs58::decode(s).with_check(None).into_vec().is_err() {
            continue;
        }

        let f = Finding::new(kind, 0, mask(s, 6));
        out.push(if is_known_secret(s) {
            f.into_test_vector()
        } else {
            f
        });
    }

    out
}

// ---------------------------------------------------------------- Nostr

fn nsec_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"nsec1[02-9ac-hj-np-z]{50,70}").unwrap())
}

pub fn find_nostr(line: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for m in nsec_re().find_iter(line) {
        let s = m.as_str();
        match bech32::decode(s) {
            Ok((hrp, _, _)) if hrp == "nsec" => {
                let f = Finding::new(Kind::NostrSecretKey, 0, mask(s, 8));
                out.push(if is_known_secret(s) {
                    f.into_test_vector()
                } else {
                    f
                })
            }
            _ => {}
        }
    }
    out
}

// ----------------------------------------- Lightning NWC & OS Path Leaks

fn nwc_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"nostr\+walletconnect://[^\s]+secret=[0-9a-fA-F]{32,64}").unwrap()
    })
}

fn os_path_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?P<pre>/home/|C:\\Users\\|/Users/)(?P<user>[a-zA-Z0-9._-]+)(?P<sep>[/\\])")
            .unwrap()
    })
}

/// Account names that are not a person: built-in Windows profiles and the
/// placeholders people use in documentation. Reporting these is pure noise.
const NON_PERSONAL_USERS: &[&str] = &[
    "public",
    "default",
    "user",
    "username",
    "you",
    "yourname",
    "your_name",
    "me",
    "name",
];

/// An NWC URI is `nostr+walletconnect://<wallet-pubkey>?relay=...&secret=<hex>`.
/// The secret is the final component, so a generic head/tail mask would print
/// part of it. Redact the pubkey and the whole secret value; keep only the
/// scheme so the user recognises what was found.
fn mask_nwc(_uri: &str) -> String {
    "nostr+walletconnect://<redacted>?...&secret=****************".to_string()
}

/// Path leaks with the account name, for the disk-mode "is it mine" check.
/// The name itself never leaves this module: callers get it only to compare.
fn find_os_paths_with_user(line: &str) -> Vec<(Finding, String)> {
    os_path_re()
        .captures_iter(line)
        .filter(|c| !NON_PERSONAL_USERS.contains(&c["user"].to_lowercase().as_str()))
        .map(|c| {
            let shown = format!("{}<user>{}", &c["pre"], &c["sep"]);
            (
                Finding::new(Kind::OsUserPathLeak, 0, shown),
                c["user"].to_string(),
            )
        })
        .collect()
}

pub fn find_nwc_and_opsec(line: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for m in nwc_re().find_iter(line) {
        let f = Finding::new(Kind::NwcWalletSecret, 0, mask_nwc(m.as_str()));
        out.push(if is_known_secret(m.as_str()) {
            f.into_test_vector()
        } else {
            f
        });
    }
    for caps in os_path_re().captures_iter(line) {
        let user = &caps["user"];
        if NON_PERSONAL_USERS.contains(&user.to_lowercase().as_str()) {
            continue;
        }
        // Keep the shape so the user knows what to look for, hide the name:
        // the whole point of this detector is that the name must not travel.
        let shown = format!("{}<user>{}", &caps["pre"], &caps["sep"]);
        out.push(Finding::new(Kind::OsUserPathLeak, 0, shown));
    }
    out
}

// ------------------------------------------- text mode: e-mail, public IP

fn email_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b[A-Za-z0-9._%+-]{1,64}@[A-Za-z0-9-]{1,63}(?:\.[A-Za-z0-9-]{1,63})*\.[A-Za-z]{2,24}\b").unwrap())
}

fn ipv4_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})\b").unwrap())
}

/// Addresses that say nothing about you: private, loopback, documentation.
fn ip_is_public(o: [u8; 4]) -> bool {
    !matches!(
        o,
        [10, ..]
            | [127, ..]
            | [0, ..]
            | [169, 254, ..]
            | [192, 168, ..]
            | [172, 16..=31, ..]
            | [100, 64..=127, ..]
            | [192, 0, 2, _]
            | [198, 51, 100, _]
            | [203, 0, 113, _]
            | [224..=255, ..]
    )
}

pub fn find_identity_hints(line: &str) -> Vec<Finding> {
    let mut out = Vec::new();
    for m in email_re().find_iter(line) {
        let e = m.as_str();
        let (local, domain) = e.split_once('@').unwrap_or((e, ""));
        if matches!(
            domain.to_lowercase().as_str(),
            "example.com" | "example.org" | "example.net"
        ) {
            continue;
        }
        let shown = format!("{}***@{}", local.chars().next().unwrap_or('*'), domain);
        out.push(Finding::new(Kind::EmailAddress, 0, shown));
    }
    for c in ipv4_re().captures_iter(line) {
        let parts: Option<Vec<u8>> = (1..=4).map(|i| c[i].parse::<u8>().ok()).collect();
        let Some(p) = parts else { continue };
        let o = [p[0], p[1], p[2], p[3]];
        // "1.2.3.4" inside a version string like v1.2.3.4 is usually not an IP.
        let before = &line[..c.get(0).unwrap().start()];
        if before.ends_with(['v', 'V']) || !ip_is_public(o) {
            continue;
        }
        out.push(Finding::new(
            Kind::PublicIpAddress,
            0,
            format!("{}.{}.*.*", o[0], o[1]),
        ));
    }
    out
}

// ------------------------------------------------------- whole content

/// The current OS account name, used in disk mode: a path with *your* name
/// deanonymizes you; `/home/someone/` in a third-party README does not.
pub fn own_username() -> Option<String> {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .ok()
        .filter(|u| !u.trim().is_empty())
}

/// Every detector over one text. In disk mode path leaks are kept only for
/// `own_user` and collapsed to one finding per file.
pub fn scan_content(content: &str, mode: Mode, own_user: Option<&str>) -> Vec<Finding> {
    if let Some(f) = find_solana_keypair(content) {
        return vec![f];
    }
    let mut findings = find_mnemonics(content, mode);
    let mut path_hits: Vec<Finding> = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let lineno = i + 1;
        if line.len() > 20_000 {
            continue;
        }
        for mut f in find_base58_keys(line).into_iter().chain(find_nostr(line)) {
            f.line = lineno;
            findings.push(f);
        }
        for mut f in find_nwc_and_opsec(line)
            .into_iter()
            .filter(|f| f.kind != Kind::OsUserPathLeak)
        {
            f.line = lineno;
            findings.push(f);
        }
        if mode == Mode::Text {
            for mut f in find_identity_hints(line) {
                f.line = lineno;
                findings.push(f);
            }
        }
        for (mut f, user) in find_os_paths_with_user(line) {
            let mine = own_user
                .map(|o| o.eq_ignore_ascii_case(&user))
                .unwrap_or(false);
            if mode == Mode::Text || mine {
                f.line = lineno;
                path_hits.push(f);
            }
        }
    }
    if mode == Mode::Disk && path_hits.len() > 1 {
        let n = path_hits.len();
        let mut first = path_hits.swap_remove(0);
        first.masked = format!("{} ({n}×)", first.masked);
        path_hits = vec![first];
    }
    findings.extend(path_hits);
    findings.sort_by_key(|f| f.line);
    findings
}

// --------------------------------------------------------------- Solana

/// True if the last 32 bytes are the ed25519 public key of the first 32 —
/// the layout of every Solana keypair. Random bytes and signatures fail this.
pub fn is_solana_keypair(bytes: &[u8]) -> bool {
    let Ok(arr) = <[u8; 64]>::try_from(bytes) else {
        return false;
    };
    let secret: [u8; 32] = arr[..32].try_into().unwrap();
    let key = ed25519_dalek::SigningKey::from_bytes(&secret);
    key.verifying_key().to_bytes() == arr[32..]
}

pub fn find_solana_keypair(content: &str) -> Option<Finding> {
    let trimmed = content.trim();
    if !trimmed.starts_with('[') || trimmed.len() > 8192 {
        return None;
    }
    let parsed: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let arr = parsed.as_array()?;
    if arr.len() != 64 {
        return None;
    }
    let bytes: Option<Vec<u8>> = arr
        .iter()
        .map(|v| v.as_u64().filter(|n| *n <= 255).map(|n| n as u8))
        .collect();
    if !is_solana_keypair(&bytes?) {
        return None;
    }
    Some(Finding::new(
        Kind::SolanaKeypairFile,
        0,
        "[64-byte secret key array]".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Published BIP39 test vector, valid checksum. Not a funded wallet.
    const VALID_12: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn detects_a_real_mnemonic() {
        let f = find_bip39(VALID_12).expect("should detect");
        assert_eq!(f.kind, Kind::Bip39Mnemonic);
        assert!(f.masked.contains("12 words"));
    }

    #[test]
    fn detects_a_mnemonic_embedded_in_a_sentence() {
        let line = format!("my backup is: {VALID_12} -- keep safe");
        assert!(find_bip39(&line).is_some());
    }

    #[test]
    fn rejects_a_bad_checksum() {
        let bad = VALID_12.replace("about", "abandon");
        assert!(find_bip39(&bad).is_none());
    }

    #[test]
    fn ignores_ordinary_prose() {
        let prose = "the quick brown fox jumps over the lazy dog while twelve \
                     ordinary english words sit here doing nothing at all today";
        assert!(find_bip39(prose).is_none());
    }

    #[test]
    fn never_leaks_the_whole_mnemonic() {
        let f = find_bip39(VALID_12).unwrap();
        assert!(!f.masked.contains(VALID_12));
        assert!(f.masked.contains("*****"));
    }

    #[test]
    fn detects_spec_test_vectors() {
        // BIP32 test vector 1 master key, Bitcoin-wiki WIF example, NIP-19 example.
        let xprv = "xprv9s21ZrQH143K3QTDL4LXw2F7HEK3wJUD2nW2nRk4stbPy6cq3jPPqjiChkVvvNKmPGJxWUtg6LnF5kejMRNNU3TGtRBeJgk33yuGBxrMPHi";
        let wif = "5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ";
        let nsec = "nsec1vl029mgpspedva04g90vltkh6fvh240zqtv9k0t9af8935ke9laqsnlfe5";
        assert_eq!(find_base58_keys(xprv)[0].kind, Kind::ExtendedPrivateKey);
        assert_eq!(find_base58_keys(wif)[0].kind, Kind::WifPrivateKey);
        assert_eq!(find_nostr(nsec)[0].kind, Kind::NostrSecretKey);
    }

    #[test]
    fn rejects_base58_with_a_broken_checksum() {
        let fake = "L".to_string() + &"1".repeat(51);
        assert!(find_base58_keys(&fake).is_empty());
    }

    #[test]
    fn sixty_four_numbers_that_are_not_a_keypair_are_ignored() {
        // Before v3.0.4 any 64-byte array counted; now the public half must
        // match the secret half (see solana_keypair_json_must_be_a_real_keypair).
        let bytes: Vec<String> = (0..64).map(|i| (i % 256).to_string()).collect();
        let json = format!("[{}]", bytes.join(","));
        assert!(find_solana_keypair(&json).is_none());
    }

    #[test]
    fn ignores_a_short_json_array() {
        assert!(find_solana_keypair("[1,2,3]").is_none());
    }

    #[test]
    fn masking_keeps_edges_only() {
        let m = mask("abcdefghijklmnopqrstuvwxyz0123456789", 4);
        assert!(m.starts_with("abcd") && m.ends_with("6789") && m.contains('*'));
    }

    // ---- H-2: NWC secret must not leak, not even partially

    #[test]
    fn nwc_mask_reveals_no_part_of_the_secret() {
        let secret = "0123456789abcdef0123456789abcdef";
        let line = format!(
            "nostr+walletconnect://b889ff5b1513b641e2a139f661a661364979c5beee91842f8f0ef42ab558e9d4?relay=wss://r.example&secret={secret}"
        );
        let f = find_nwc_and_opsec(&line);
        let nwc = f
            .iter()
            .find(|f| f.kind == Kind::NwcWalletSecret)
            .expect("detect");
        // No 6-character run of the secret may appear in the output.
        for i in 0..=secret.len() - 6 {
            assert!(
                !nwc.masked.contains(&secret[i..i + 6]),
                "leaked {}",
                &secret[i..i + 6]
            );
        }
        assert!(
            !nwc.masked.contains("b889ff5b"),
            "wallet pubkey should be redacted too"
        );
    }

    // ---- M-2: the username must not appear in the finding

    #[test]
    fn os_path_finding_hides_the_username() {
        for line in [
            r"error at C:\Users\Satoshi\Documents\notes.txt",
            "panicked at /home/satoshi/dev/app/src/main.rs:10",
            "see /Users/satoshi/Library/Logs/x.log",
        ] {
            let f = find_nwc_and_opsec(line);
            let p = f
                .iter()
                .find(|f| f.kind == Kind::OsUserPathLeak)
                .expect(line);
            assert!(p.masked.contains("<user>"), "{}", p.masked);
            assert!(!p.masked.to_lowercase().contains("satoshi"), "{}", p.masked);
        }
    }

    #[test]
    fn os_path_ignores_placeholders_and_builtin_profiles() {
        for line in [
            r"C:\Users\Public\Desktop",
            "/home/user/.bashrc",
            r"C:\Users\Default\NTUSER",
        ] {
            assert!(
                find_nwc_and_opsec(line)
                    .iter()
                    .all(|f| f.kind != Kind::OsUserPathLeak),
                "should ignore {line}"
            );
        }
    }

    // ---- false positives reported from a real Windows scan (v3.0.3)

    #[test]
    fn python_identifiers_are_not_a_seed_phrase() {
        // ComfyUI comfy_extras/nodes_clip_sdxl.py, the line that was flagged.
        let line = r#"        return io.NodeOutput(clip.encode_from_tokens_scheduled(tokens, add_dict={"width": width, "height": height, "crop_w": crop_w, "crop_h": crop_h, "target_width": target_width, "target_height": target_height}))"#;
        assert!(find_mnemonics(line, Mode::Text).is_empty());
    }

    #[test]
    fn a_quoted_word_list_and_the_plain_bip39_list_are_not_seeds() {
        let quoted = "const W = ['abandon','ability','able','about','above','absent','absorb','abstract','absurd','abuse','access','accident','account','accuse','achieve','acid'];";
        assert!(find_mnemonics(quoted, Mode::Text).is_empty());
        let list = bip39::Language::English.word_list().join("\n");
        assert!(
            find_mnemonics(&list, Mode::Disk).is_empty(),
            "2048-word list is not a seed"
        );
    }

    #[test]
    fn one_word_per_line_and_numbered_seeds_are_found() {
        let per_line = VALID_12.split(' ').collect::<Vec<_>>().join("\n");
        let f = find_mnemonics(&per_line, Mode::Disk);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].line, 1);
        let numbered: String = VALID_12
            .split(' ')
            .enumerate()
            .map(|(i, w)| format!("{}. {w} ", i + 1))
            .collect();
        assert_eq!(find_mnemonics(&numbered, Mode::Disk).len(), 1);
        let commas = VALID_12.replace(' ', ", ");
        assert_eq!(find_mnemonics(&commas, Mode::Disk).len(), 1);
    }

    #[test]
    fn spec_examples_are_marked_into_test_vectors() {
        assert!(find_bip39(VALID_12).unwrap().test_vector);
        assert!(
            find_nostr("nsec1vl029mgpspedva04g90vltkh6fvh240zqtv9k0t9af8935ke9laqsnlfe5")[0]
                .test_vector
        );
        assert!(
            find_base58_keys("5HueCGU8rMjxEXxiPuD5BDku4MkFqeZyd4dZ1jvhTVqvbTLvyTJ")[0].test_vector
        );
        assert!(find_bip39(VALID_12).unwrap().label.contains("test vector"));
    }

    #[test]
    fn a_seed_with_a_typo_is_flagged_in_text_mode_only() {
        // Last word changed: 12 valid BIP39 words, checksum fails.
        let typo = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon zoo";
        let f = find_mnemonics(typo, Mode::Text);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, Kind::PossibleMnemonic);
        assert!(find_mnemonics(typo, Mode::Disk).is_empty());
        // Followed by another BIP39 word on the next line ("mail" is one).
        let near = "abandon ability able about above absent absorb abstract absurd abuse access accident\nmail me";
        assert_eq!(
            find_mnemonics(near, Mode::Text)[0].kind,
            Kind::PossibleMnemonic
        );
    }

    #[test]
    fn disk_mode_reports_only_my_username_once_per_file() {
        let text = "a C:\\Users\\bob\\x\nb /home/alice/y\nc C:\\Users\\bob\\z\n";
        let disk = scan_content(text, Mode::Disk, Some("Bob"));
        assert_eq!(disk.len(), 1);
        assert_eq!(disk[0].kind, Kind::OsUserPathLeak);
        assert!(disk[0].masked.contains("(2×)"));
        assert!(!disk[0].masked.to_lowercase().contains("bob"));
        assert!(scan_content(text, Mode::Disk, Some("carol")).is_empty());
        assert_eq!(scan_content(text, Mode::Text, None).len(), 3);
    }

    // ---- Solana keys verified by ed25519, identity hints

    fn test_keypair() -> Vec<u8> {
        let sk = ed25519_dalek::SigningKey::from_bytes(&[7u8; 32]);
        let mut v = sk.to_bytes().to_vec();
        v.extend_from_slice(&sk.verifying_key().to_bytes());
        v
    }

    #[test]
    fn solana_keypair_json_must_be_a_real_keypair() {
        let kp = test_keypair();
        let json = serde_json::to_string(&kp).unwrap();
        assert!(find_solana_keypair(&json).is_some());
        let mut fake = kp.clone();
        fake[63] ^= 1; // 64 bytes, but not secret+pubkey
        assert!(find_solana_keypair(&serde_json::to_string(&fake).unwrap()).is_none());
    }

    #[test]
    fn solana_base58_secret_is_found_but_a_signature_is_not() {
        let kp = test_keypair();
        let b58 = bs58::encode(&kp).into_string();
        let f = find_base58_keys(&format!("phantom export: {b58}"));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, Kind::SolanaPrivateKey);
        assert!(!f[0].masked.contains(&b58[6..20]));
        // A transaction signature is also 64 bytes of base58, but random.
        let sig = bs58::encode([42u8; 64]).into_string();
        assert!(find_base58_keys(&sig).is_empty());
    }

    #[test]
    fn email_and_public_ip_only_in_text_mode() {
        let log = "contact ivan.petrov@proton.me from 85.143.22.7, lan 192.168.1.5, docs a@example.com, v1.2.3.4";
        let t = scan_content(log, Mode::Text, None);
        let kinds: Vec<Kind> = t.iter().map(|f| f.kind).collect();
        assert_eq!(kinds, vec![Kind::EmailAddress, Kind::PublicIpAddress]);
        assert_eq!(t[0].masked, "i***@proton.me");
        assert_eq!(t[1].masked, "85.143.*.*");
        assert!(scan_content(log, Mode::Disk, None).is_empty());
    }
}
