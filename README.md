# StackerPulse ⚡ + SeedScan 🛡️

**Author:** `UBro-dev` · **License:** MIT (Copyright (c) 2026 UBro-dev) · [Русская версия](README.ru.md)

A desktop companion for [Stacker News](https://stacker.news), written in Rust
(Tauri v2) for Windows, Linux and macOS. It reads live SN data through Tor if you
want, shows what territories really earn, checks your posts for leaked secrets
before you publish, cleans links, and tracks bounties — without telemetry,
without demo numbers, and without talking to anything you did not ask for.

> Status: v3.2.1. Tested on Linux (release build, 110 tests, real window).
> Every release is built and tested by CI on Windows, Linux and macOS.
> Every GraphQL query is validated against the schema in the
> `stackernews/stacker.news` source.

## What it does

- **SeedScan — offline secret scanner.** Finds secrets left in plain text in
  folders you choose and in your post draft:
  - BIP39 seed phrases (12–24 words) — **checksum validated**
  - xprv/yprv/zprv/tprv — Base58Check validated
  - WIF private keys — Base58Check validated
  - Nostr `nsec1…` — Bech32 validated
  - Solana keypairs: `id.json` files and base58 exports, verified with
    ed25519 (the public half must match the secret half)
  - Lightning NWC URIs with `secret=`
  - OS paths that reveal your username (`C:\Users\<name>\`, `/home/<name>/`)

  Read-only, offline, and every finding is masked
  (`abandon ***** about (12 words)`, `C:\Users\<user>\`). Ordinary English
  text does not trigger it, because checksums are checked. A CLI version is
  included: `cargo run --release --features cli --bin seedscan-cli -- <folders>`.

- **Pre-publish OpSec guard.** Warns while you type a post if it contains any
  of the above, an e-mail or a public IP address, and can scrub paths and
  keys from the draft.

- **Link sanitizer** (SN issue #2664). Removes trackers (`utm_*`, `fbclid`,
  `gclid`, plus `s`/`t` only on x.com, `si` only on YouTube/Spotify…) without
  breaking meaningful parameters like YouTube timestamps, and can rewrite
  x.com → xcancel / Nitter, reddit → Redlib, medium → scribe.rip with your own
  instance.

- **PR bounty radar.** Lists issues from `stackernews/stacker.news` and shows
  the award from the README table: difficulty label (20k / 100k / 250k / 500k /
  1M sats) × priority (×0.5 / ×1.5 / ×2 / ×3), as an upper bound. Issues
  without a difficulty label show no amount. A personal tracker (in progress /
  PR sent / paid, own hours, own tasks, CSV) lives only on your machine.

- **Live feed.** Hot / new / top posts for any territory and period, loaded
  from SN on demand, with local filters, search, bookmarks (notes and tags)
  and CSV export. Nothing is shown until you load it.

- **SN bounties.** Active bounties from SN itself, next to GitHub PR awards
  and your own tasks, in one tracker.

- **Real territory economics.** Uses SN's own `topSubs` stats: what each
  founder actually earned in the period, zaps, number of posts, and whether
  that covers the plan (50k sats a month, 500k a year, or 3M once). Revenue is
  scaled to 30 days so it compares with the plan price; for one-time plans the
  app shows how many months the price takes to earn back.

- **Territory trends.** SN's own growth charts for any territory, per hour,
  day or month: what was earned there and where it came from (zaps, founder
  revenue, rewards, bounties), what was spent, posts and comments, and how
  many people earned. A quick trend says whether earnings are growing.

- **Stacker stats.** Public stats for any @nym — stacked and spent per day,
  week, month, year and all time, items, streaks. If someone hides their stats
  in SN, the app says so instead of guessing.

- **Post studio** with real territory post prices, **GraphQL playground**,
  **RU/EN interface**, and a **Settings & privacy** tab (Tor proxy, and a
  viewer for the Rust source that is compiled into the app).

- **Updates you ask for.** *Settings → Updates → Check for updates* looks for
  a new version and installs it in one click. See the privacy notes below.

## Privacy model

- **Allowlist enforced in Rust.** The SN client connects only to
  `stacker.news` and its official onion
  `snsnsnya6h3ot563f3p566wuhfoklkg5f62hokdlaqzcaub3gf4xlxyd.onion` — on the
  first request and on every redirect (max 3).
- **Tor via `socks5h://`**, so DNS and `.onion` resolution stay inside Tor.
  The app finds Tor on 9150 (Tor Browser) or 9050 (tor service) by itself.
  If the `.onion` does not answer, it retries once via `stacker.news` through
  the same Tor (can be turned off in Settings); links then point at
  `stacker.news` too, because an onion link would not open either.
- **Airgap mode** refuses to build any HTTP client.
- **GitHub is opt-in.** The bounty radar talks to `api.github.com` only after
  you enable it, only when you press the button, without a token, with no
  redirects, and only for two fixed paths of one repository. Without Tor the
  app asks first, because GitHub would see your IP.
- **Updates only when you ask, and only signed ones.** There is no background
  check. *Check for updates* fetches `latest.json` from this repository's
  GitHub releases over Tor if Tor is on (and asks first if it is off). Every
  redirect and the download itself must stay on GitHub's hosts. The package's
  minisign signature is checked against the key built into the app, and the
  signature also covers the version number, so an older package can't be
  passed off as a newer one.
- **No remote code.** Chart.js and Tailwind are bundled; a strict CSP allows
  only local scripts and Tauri IPC — the webview cannot fetch anything.
- **Links are copied, never opened**, so nothing leaves through your default
  browser behind Tor's back. With Tor on, SN links are copied as `.onion`.
- **No `Referer`, cookies or `Accept-Language`.** By default no
  `User-Agent` is sent either; that is unusual in itself, so you can switch to
  Tor Browser's UA. GitHub always gets that UA because it rejects requests
  without one.

## Installing

Download from the repository's **Releases** page:

| System | File |
|---|---|
| Windows 10/11 | `StackerPulse_<version>_x64-setup.exe` (English or Russian, picked from Windows), or `…_x64_en-US.msi` / `…_x64_ru-RU.msi` |
| Linux | `StackerPulse_<version>_amd64.AppImage` or `…_amd64.deb` |
| macOS 10.15+ (Apple Silicon and Intel) | `StackerPulse_<version>_universal.dmg` |
| Command-line scanner only | `seedscan-cli-<system>.tar.gz` / `.zip` |

**Check the file before you run it.** Every release carries `SHA256SUMS.txt`
and a build-provenance attestation, which proves the file was built by this
repository's GitHub Actions workflow and not modified afterwards:

```bash
sha256sum -c SHA256SUMS.txt --ignore-missing        # Linux
shasum -a 256 -c SHA256SUMS.txt --ignore-missing    # macOS
gh attestation verify <file> --repo UBro-dev/stacker-pulse
```

On Windows: `Get-FileHash <file> -Algorithm SHA256` and compare with
`SHA256SUMS.txt`.

**The installers are not code-signed.** Signing certificates from Microsoft
and Apple are issued only to a verified real identity, and this project is
pseudonymous. The attestation above is the substitute. Expect one warning on
first launch:

- **Windows** (SmartScreen, "Windows protected your PC"): *More info* →
  *Run anyway*. If WebView2 is missing (older Windows 10), the installer
  downloads it from Microsoft; Windows 11 already has it.
- **macOS**: drag the app to *Applications* and open it once. When macOS says
  it cannot verify the developer, open *System Settings → Privacy & Security*
  and press *Open Anyway* (on macOS 14 and older, Control-click the app →
  *Open* also works). Or, in Terminal:
  `xattr -dr com.apple.quarantine /Applications/StackerPulse.app`
- **Linux AppImage**: `chmod +x StackerPulse_*.AppImage`, then run it.

## Build and run

Requires Rust (stable). On Linux also `libwebkit2gtk-4.1-dev`.

```bash
cd src-tauri
cargo test --release --features cli    # 110 tests
cargo run --release      # start the app
```

Installers (`.msi` / `.exe`, `.AppImage` / `.deb`, `.dmg`):

```bash
cargo install tauri-cli --version "^2"
./build_linux.sh          # Linux
.\build_windows.ps1       # Windows (PowerShell)
./build_macos.sh          # macOS, universal binary
```

The build scripts strip your home folder out of the binaries (rustc would
otherwise embed `C:\Users\<your name>\…` hundreds of times) and stop if it is
still there. Build through them, not with a bare `cargo tauri build`, if you
will share the result. Without the project's updater key they build plain
installers (`tauri.no-updater.conf.json`); those can still check for updates.

Releases are built by CI, not by hand: pushing a tag `vX.Y.Z` that matches
the version in `src-tauri/Cargo.toml` runs `.github/workflows/release.yml`,
which tests on all three systems, builds every installer and the CLI,
attaches them to a **draft** release with checksums and attestations, and
waits for you to press *Publish*. Users are only offered a release once it is
published.

### The updater key (maintainers, once)

Update packages are signed with a minisign key that belongs to the project,
not to a person, so no real name is involved. Generate it on your own
machine, never in CI:

```bash
npx @tauri-apps/cli@2.12.4 signer generate -w ~/.tauri/stackerpulse.key
```

1. Put the one-line public key (`~/.tauri/stackerpulse.key.pub`) into
   `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`. The release
   workflow refuses to run while the placeholder is still there.
2. Add two repository secrets (*Settings → Secrets and variables → Actions*):
   `TAURI_SIGNING_PRIVATE_KEY` (the content of `~/.tauri/stackerpulse.key`)
   and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
3. Keep an offline backup of the key file and its password. Without them no
   installed copy can be updated again; users would have to reinstall by hand.

After changing CSS classes in the UI, rebuild Tailwind:

```bash
npx tailwindcss@3.4.17 -c tools/tailwind.config.js -i tools/tailwind.input.css -o ui/vendor/tailwind.css --minify
```

## Limits

- SeedScan knows only the formats listed above. It does not read archives,
  PDFs or office files, and "no findings" is not a clean bill of health.
- Territory and stacker numbers are what SN reports. A founder's share can
  be changed per territory, so the app uses actual revenue, not a formula.
- PR awards are an upper bound; Stacker News pays at its own discretion.

## Support

StackerPulse is free, with no ads, telemetry or paid tier. If it helped you,
the donation addresses (Monero, stablecoins, Lightning) are on one page:
**[DONATE.md](DONATE.md)**. The same addresses are built into the app
(*Settings and privacy → Support the developer*) and checked by the tests, so
a release cannot ship with a mistyped one. Check the address after pasting:
clipboard malware swaps crypto addresses.

## Licence, scope and security

- [`LICENSE`](LICENSE) — MIT.
- [`DISCLAIMER.md`](DISCLAIMER.md) — what the tool does not promise, and the
  rule that you scan only your own machines.
- [`THIRD-PARTY.md`](THIRD-PARTY.md) — every dependency with its licence and
  copyright, generated by `tools/gen-third-party.py` and shipped with the
  installers.
- [`SECURITY.md`](SECURITY.md) — how to report a vulnerability privately.
- [`DONATE.md`](DONATE.md) — voluntary donations; what they are and are not.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — contributions are MIT; pseudonyms
  are fine.

## Changelog

See [`FIXES.md`](FIXES.md) for every security and correctness fix, with the
reason and the test that covers it.
