//! "Support the developer": public donation addresses.
//! Author: UBro-dev
//! License: MIT (Copyright (c) 2026 UBro-dev)
//!
//! The addresses live in one place, the table in `DONATE.md`, which GitHub
//! shows as a page and the app embeds at compile time. Every address is
//! checked here before it can be shown: a Monero address by its Keccak
//! checksum, a Tron address by its Base58Check checksum, a Solana address by
//! its length (it has no checksum), a Lightning address by its shape. A typo,
//! a stray character or, worst of all, a pasted private key fails the tests,
//! so it fails CI and never reaches a release.
//!
//! Nothing here touches the network.

use serde::Serialize;

/// The page the addresses come from, embedded at compile time.
pub const DONATE_MD: &str = include_str!("../../DONATE.md");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    Xmr,
    UsdcSolana,
    UsdtTron,
    Lightning,
}

impl Method {
    /// The text in the first column of the DONATE.md table.
    fn from_table_name(name: &str) -> Option<Method> {
        match name {
            "Monero (XMR)" => Some(Method::Xmr),
            "USDC (Solana)" => Some(Method::UsdcSolana),
            "USDT (Tron)" => Some(Method::UsdtTron),
            "Lightning" => Some(Method::Lightning),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SupportAddress {
    pub method: Method,
    pub name: &'static str,
    pub address: String,
    pub warning_ru: &'static str,
    pub warning_en: &'static str,
}

fn describe(method: Method) -> (&'static str, &'static str, &'static str) {
    match method {
        Method::Xmr => (
            "Monero (XMR)",
            "Только XMR в сети Monero.",
            "XMR on the Monero network only.",
        ),
        Method::UsdcSolana => (
            "USDC · Solana",
            "Только USDC в сети Solana. Другой токен или другая сеть — деньги потеряются.",
            "USDC on Solana only. Another token or another network loses the funds.",
        ),
        Method::UsdtTron => (
            "USDT · Tron (TRC-20)",
            "Только USDT в сети Tron (TRC-20). Другой токен или другая сеть — деньги потеряются.",
            "USDT on Tron (TRC-20) only. Another token or another network loses the funds.",
        ),
        Method::Lightning => (
            "Lightning ⚡",
            "Lightning-адрес: отправьте сатоши из любого Lightning-кошелька.",
            "Lightning address: send sats from any Lightning wallet.",
        ),
    }
}

/// The addresses in DONATE.md, in table order. Rows whose address is empty
/// or `—` are not set yet and are left out.
pub fn addresses() -> Result<Vec<SupportAddress>, String> {
    parse_donate_md(DONATE_MD)
}

pub fn parse_donate_md(md: &str) -> Result<Vec<SupportAddress>, String> {
    let mut out: Vec<SupportAddress> = Vec::new();
    for (i, line) in md.lines().enumerate() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim())
            .collect();
        if cells.len() < 2 {
            continue;
        }
        let first = cells[0];
        // Header and separator rows.
        if first.starts_with("Method") || first.chars().all(|c| c == '-' || c == ':') {
            continue;
        }
        let method = Method::from_table_name(first).ok_or_else(|| {
            format!(
                "DONATE.md line {}: unknown method \"{first}\". Use one of: \
                 Monero (XMR), USDC (Solana), USDT (Tron), Lightning.",
                i + 1
            )
        })?;
        let raw = cells[cells.len() - 1].trim_matches('`').trim();
        if raw.is_empty() || raw == "—" || raw == "-" {
            continue;
        }
        validate(method, raw).map_err(|e| format!("DONATE.md line {}: {e}", i + 1))?;
        if out.iter().any(|a| a.method == method) {
            return Err(format!(
                "DONATE.md line {}: {first} is listed twice.",
                i + 1
            ));
        }
        let (name, warning_ru, warning_en) = describe(method);
        out.push(SupportAddress {
            method,
            name,
            address: raw.to_string(),
            warning_ru,
            warning_en,
        });
    }
    Ok(out)
}

/// Checks that `s` is a well-formed *public* address for `method`.
pub fn validate(method: Method, s: &str) -> Result<(), String> {
    if s.chars().any(char::is_whitespace) {
        return Err(
            "an address is one word without spaces. Several words look like a \
                    seed phrase — never put a seed phrase here."
                .into(),
        );
    }
    match method {
        Method::Xmr => validate_monero(s),
        Method::UsdcSolana => validate_solana(s),
        Method::UsdtTron => validate_tron(s),
        Method::Lightning => validate_lightning(s),
    }
}

// ---------------------------------------------------------------- Monero ---

/// Main-net standard address (starts with 4) or subaddress (starts with 8).
/// Integrated addresses (106 characters) embed a payment ID and are not meant
/// for a public donation address.
fn validate_monero(s: &str) -> Result<(), String> {
    if s.len() == 106 {
        return Err("this is an integrated Monero address (with a payment ID); \
                    use a subaddress (starts with 8) instead."
            .into());
    }
    if s.len() != 95 {
        return Err(format!(
            "a Monero address is 95 characters, this one is {}.",
            s.len()
        ));
    }
    let bytes = monero_base58_decode(s).ok_or("not valid Monero base58.")?;
    // 1 network byte + 32 spend key + 32 view key + 4 checksum.
    if bytes.len() != 69 {
        return Err("wrong Monero address length after decoding.".into());
    }
    match bytes[0] {
        18 | 42 => {}
        24 | 36 | 53 | 63 => {
            return Err("this is a testnet or stagenet Monero address, not main net.".into())
        }
        _ => return Err("unknown Monero network byte.".into()),
    }
    let sum = keccak256(&bytes[..65]);
    if sum[..4] != bytes[65..] {
        return Err(
            "the Monero checksum does not match: a character is wrong or missing. \
                    Copy the address again from the wallet."
                .into(),
        );
    }
    Ok(())
}

const B58: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
/// Monero base58 encodes 8-byte blocks as 11 characters; a final shorter
/// block of n bytes takes ENCODED[n] characters.
const ENCODED: [usize; 9] = [0, 2, 3, 5, 6, 7, 9, 10, 11];

fn monero_base58_decode(s: &str) -> Option<Vec<u8>> {
    let chars = s.as_bytes();
    let mut out = Vec::with_capacity(chars.len() * 8 / 11 + 8);
    for block in chars.chunks(11) {
        let size = ENCODED.iter().position(|&n| n == block.len())?;
        if size == 0 {
            return None;
        }
        let mut num: u128 = 0;
        for &c in block {
            let d = B58.iter().position(|&a| a == c)? as u128;
            num = num * 58 + d;
        }
        if size < 8 && num >> (8 * size) != 0 {
            return None;
        }
        if num > u64::MAX as u128 {
            return None;
        }
        let be = (num as u64).to_be_bytes();
        out.extend_from_slice(&be[8 - size..]);
    }
    Some(out)
}

/// Keccak-256 as Monero uses it (original Keccak padding 0x01, not SHA-3's
/// 0x06). Small enough to keep here instead of adding a dependency.
pub(crate) fn keccak256(data: &[u8]) -> [u8; 32] {
    const RATE: usize = 136;
    let mut st = [0u64; 25];
    let absorb = |st: &mut [u64; 25], block: &[u8]| {
        for (lane, chunk) in st.iter_mut().zip(block.chunks(8)) {
            let mut b = [0u8; 8];
            b.copy_from_slice(chunk);
            *lane ^= u64::from_le_bytes(b);
        }
        keccak_f(st);
    };
    let mut blocks = data.chunks_exact(RATE);
    for block in &mut blocks {
        absorb(&mut st, block);
    }
    let rest = blocks.remainder();
    let mut last = [0u8; RATE];
    last[..rest.len()].copy_from_slice(rest);
    last[rest.len()] ^= 0x01;
    last[RATE - 1] ^= 0x80;
    absorb(&mut st, &last);
    let mut out = [0u8; 32];
    for (chunk, lane) in out.chunks_mut(8).zip(st.iter()) {
        chunk.copy_from_slice(&lane.to_le_bytes());
    }
    out
}

fn keccak_f(st: &mut [u64; 25]) {
    const RC: [u64; 24] = [
        0x0000000000000001,
        0x0000000000008082,
        0x800000000000808a,
        0x8000000080008000,
        0x000000000000808b,
        0x0000000080000001,
        0x8000000080008081,
        0x8000000000008009,
        0x000000000000008a,
        0x0000000000000088,
        0x0000000080008009,
        0x000000008000000a,
        0x000000008000808b,
        0x800000000000008b,
        0x8000000000008089,
        0x8000000000008003,
        0x8000000000008002,
        0x8000000000000080,
        0x000000000000800a,
        0x800000008000000a,
        0x8000000080008081,
        0x8000000000008080,
        0x0000000080000001,
        0x8000000080008008,
    ];
    const ROT: [u32; 24] = [
        1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14, 27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44,
    ];
    const PI: [usize; 24] = [
        10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4, 15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1,
    ];
    for rc in RC {
        let mut c = [0u64; 5];
        for (x, cx) in c.iter_mut().enumerate() {
            *cx = st[x] ^ st[x + 5] ^ st[x + 10] ^ st[x + 15] ^ st[x + 20];
        }
        for x in 0..5 {
            let d = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
            for y in 0..5 {
                st[5 * y + x] ^= d;
            }
        }
        let mut t = st[1];
        for (&j, &r) in PI.iter().zip(ROT.iter()) {
            let next = st[j];
            st[j] = t.rotate_left(r);
            t = next;
        }
        for y in 0..5 {
            let row = [
                st[5 * y],
                st[5 * y + 1],
                st[5 * y + 2],
                st[5 * y + 3],
                st[5 * y + 4],
            ];
            for x in 0..5 {
                st[5 * y + x] = row[x] ^ (!row[(x + 1) % 5] & row[(x + 2) % 5]);
            }
        }
        st[0] ^= rc;
    }
}

// ---------------------------------------------------------------- Solana ---

/// A Solana address is a base58 ed25519 public key: exactly 32 bytes. It has
/// no checksum, so only the length can be checked. A 64-byte value is a
/// secret key (Phantom/Solflare export), and is refused loudly.
fn validate_solana(s: &str) -> Result<(), String> {
    let bytes = bs58::decode(s)
        .into_vec()
        .map_err(|_| "not valid base58 (a Solana address uses no 0, O, I or l).".to_string())?;
    match bytes.len() {
        32 => Ok(()),
        64 => Err(
            "STOP: this is a Solana PRIVATE KEY, not an address. Delete it from \
                   the file now and never share it. Copy the address from the wallet's \
                   Receive screen instead."
                .into(),
        ),
        n => Err(format!(
            "a Solana address decodes to 32 bytes, this one to {n}."
        )),
    }
}

// ------------------------------------------------------------------ Tron ---

/// A Tron address is Base58Check over 0x41 + 20 bytes, so it starts with T
/// and is 34 characters long; the 4-byte checksum catches typos.
fn validate_tron(s: &str) -> Result<(), String> {
    if !s.starts_with('T') || s.len() != 34 {
        return Err("a Tron address starts with T and is 34 characters long.".into());
    }
    let payload = bs58::decode(s).with_check(None).into_vec().map_err(|_| {
        "the Tron checksum does not match: a character is wrong or missing. \
             Copy the address again from the wallet."
            .to_string()
    })?;
    if payload.len() != 21 || payload[0] != 0x41 {
        return Err("not a Tron main-net address.".into());
    }
    Ok(())
}

// ------------------------------------------------------------- Lightning ---

/// A Lightning address looks like an e-mail address: name@domain.tld.
fn validate_lightning(s: &str) -> Result<(), String> {
    let re = regex::Regex::new(r"^[a-z0-9._-]{1,64}@[a-z0-9-]{1,63}(\.[a-z0-9-]{1,63})+$")
        .expect("valid regex");
    if s.starts_with("lnbc") || s.starts_with("LNBC") {
        return Err(
            "this is a one-time Lightning invoice; put a Lightning address \
                    (name@domain) here instead."
                .into(),
        );
    }
    if !re.is_match(s) {
        return Err("a Lightning address looks like name@domain.com, in lower case.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Monero Project's General Fund donation address: a real, published
    /// main-net standard address.
    const XMR_GENERAL_FUND: &str = "44AFFq5kSiGBoZ4NMDwYtN18obc8AemS33DBLWs3H7otXft3XjrpDtQGv7SqSsaBYBb98uNbr2VBBEt7f2wfn3RVGQBEP3A";
    /// Tether's USDT contract on Tron: a real, published Tron address.
    const TRON_USDT_CONTRACT: &str = "TR7NHqjeKQxGTCi8q8ZY4pL8otSzgjLj6t";
    /// The SPL Token program id: a real, published Solana address.
    const SOLANA_TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

    fn monero_base58_encode(bytes: &[u8]) -> String {
        let mut out = String::new();
        for block in bytes.chunks(8) {
            let mut be = [0u8; 8];
            be[8 - block.len()..].copy_from_slice(block);
            let mut num = u64::from_be_bytes(be);
            let width = ENCODED[block.len()];
            let mut chars = vec![b'1'; width];
            for c in chars.iter_mut().rev() {
                *c = B58[(num % 58) as usize];
                num /= 58;
            }
            out.push_str(std::str::from_utf8(&chars).unwrap());
        }
        out
    }

    #[test]
    fn keccak256_matches_known_vectors() {
        let hex = |b: [u8; 32]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        assert_eq!(
            hex(keccak256(b"")),
            "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
        );
        assert_eq!(
            hex(keccak256(b"abc")),
            "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45"
        );
        // Longer than one 136-byte block.
        let long = vec![b'a'; 200];
        let once = keccak256(&long);
        assert_eq!(once, keccak256(&long));
        assert_ne!(once, keccak256(&long[..199]));
    }

    #[test]
    fn a_real_monero_address_passes() {
        validate(Method::Xmr, XMR_GENERAL_FUND).unwrap();
    }

    #[test]
    fn a_monero_subaddress_passes() {
        // Same keys with the subaddress network byte and a fresh checksum.
        let mut bytes = monero_base58_decode(XMR_GENERAL_FUND).unwrap();
        bytes[0] = 42;
        let sum = keccak256(&bytes[..65]);
        bytes[65..].copy_from_slice(&sum[..4]);
        let sub = monero_base58_encode(&bytes);
        assert!(sub.starts_with('8'), "{sub}");
        assert_eq!(sub.len(), 95);
        validate(Method::Xmr, &sub).unwrap();
        // The encoder agrees with the published address.
        let orig = monero_base58_decode(XMR_GENERAL_FUND).unwrap();
        assert_eq!(monero_base58_encode(&orig), XMR_GENERAL_FUND);
    }

    #[test]
    fn a_monero_typo_is_caught() {
        // Change one character in the middle.
        let mut s: Vec<char> = XMR_GENERAL_FUND.chars().collect();
        s[40] = if s[40] == 'A' { 'B' } else { 'A' };
        let typo: String = s.into_iter().collect();
        let err = validate(Method::Xmr, &typo).unwrap_err();
        assert!(err.contains("checksum"), "{err}");
        // One character missing.
        assert!(validate(Method::Xmr, &XMR_GENERAL_FUND[1..]).is_err());
    }

    #[test]
    fn tron_address_and_typo() {
        validate(Method::UsdtTron, TRON_USDT_CONTRACT).unwrap();
        let typo = TRON_USDT_CONTRACT.replace("Lj6t", "Lj6u");
        assert!(validate(Method::UsdtTron, &typo)
            .unwrap_err()
            .contains("checksum"));
        assert!(validate(Method::UsdtTron, "0x1234").is_err());
    }

    #[test]
    fn solana_address_ok_and_private_key_refused() {
        validate(Method::UsdcSolana, SOLANA_TOKEN_PROGRAM).unwrap();
        // 64 bytes of base58 is what a wallet exports as a secret key. These
        // bytes are made up and belong to nobody.
        let fake_secret = bs58::encode([7u8; 64]).into_string();
        let err = validate(Method::UsdcSolana, &fake_secret).unwrap_err();
        assert!(err.contains("PRIVATE KEY"), "{err}");
        assert!(validate(Method::UsdcSolana, "0OIl").is_err());
    }

    #[test]
    fn a_seed_phrase_is_never_accepted() {
        let seed = "abandon abandon abandon abandon abandon abandon abandon abandon \
                    abandon abandon abandon about";
        for m in [
            Method::Xmr,
            Method::UsdcSolana,
            Method::UsdtTron,
            Method::Lightning,
        ] {
            let err = validate(m, seed).unwrap_err();
            assert!(err.contains("seed"), "{m:?}: {err}");
        }
    }

    #[test]
    fn lightning_address_shape() {
        validate(Method::Lightning, "ubro_dev@stacker.news").unwrap();
        assert!(validate(Method::Lightning, "lnbc10u1pfake").is_err());
        assert!(validate(Method::Lightning, "UBro@Example.com").is_err());
        assert!(validate(Method::Lightning, "nobody").is_err());
    }

    #[test]
    fn table_parsing_skips_empty_rows_and_rejects_unknown_methods() {
        let md = format!(
            "| Method | Network | Address |\n|---|---|---|\n\
             | Monero (XMR) | Monero | `{XMR_GENERAL_FUND}` |\n\
             | USDC (Solana) | Solana | — |\n\
             | USDT (Tron) | Tron | `{TRON_USDT_CONTRACT}` |\n\
             | Lightning | Lightning | |\n"
        );
        let got = parse_donate_md(&md).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].method, Method::Xmr);
        assert_eq!(got[1].method, Method::UsdtTron);

        let bad = "| Method | Network | Address |\n|---|---|---|\n| Bitcoin | BTC | `x` |\n";
        assert!(parse_donate_md(bad).unwrap_err().contains("unknown method"));

        let twice = format!(
            "| Monero (XMR) | Monero | `{XMR_GENERAL_FUND}` |\n\
             | Monero (XMR) | Monero | `{XMR_GENERAL_FUND}` |\n"
        );
        assert!(parse_donate_md(&twice).unwrap_err().contains("twice"));
    }

    /// The file that ships: every address in it must be valid. This is the
    /// test that stops a release if DONATE.md has a typo or a private key.
    #[test]
    fn the_shipped_donate_md_is_valid() {
        if let Err(e) = addresses() {
            panic!("{e}");
        }
    }

    /// SeedScan itself must find no secret in the page we publish.
    #[test]
    fn seedscan_finds_no_secret_in_donate_md() {
        use crate::detect::{scan_content, Kind, Mode};
        let secrets: Vec<_> = scan_content(DONATE_MD, Mode::Text, None)
            .into_iter()
            .filter(|f| !matches!(f.kind, Kind::EmailAddress | Kind::PublicIpAddress))
            .collect();
        assert!(secrets.is_empty(), "DONATE.md: {secrets:?}");
    }

    /// The READMEs point to DONATE.md instead of repeating the addresses, so
    /// there is only one place to change them.
    #[test]
    fn readmes_link_to_donate_md_and_hold_no_addresses() {
        for (name, text) in [
            ("README.md", include_str!("../../README.md")),
            ("README.ru.md", include_str!("../../README.ru.md")),
        ] {
            assert!(
                text.contains("](DONATE.md)"),
                "{name} must link to DONATE.md"
            );
            for a in addresses().unwrap() {
                assert!(
                    !text.contains(&a.address),
                    "{name} repeats an address; keep them only in DONATE.md"
                );
            }
        }
    }
}
