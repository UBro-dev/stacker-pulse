# StackerPulse v3.1.1 — Support the developer

## One page for donation addresses (`DONATE.md`)
- New `DONATE.md`: a table of public donation addresses (Monero, USDC on
  Solana, USDT on Tron, Lightning) with network warnings, a check against
  clipboard malware, and what a donation is and is not (a voluntary gift, no
  service, no refund; the software stays MIT whether or not you donate).
- **It is the only place the addresses are written.** The README links to it,
  GitHub's *Sponsor* button (`.github/FUNDING.yml`) points to it, and the app
  embeds it at build time. Changing an address means editing one line.
- In the app: *Settings and privacy → Support the developer*, one row per
  address with a Copy button and the network warning. The card stays hidden
  while no address is set. Nothing is opened or sent; the address is only
  copied. DONATE.md and `support.rs` are also in the source viewer.

## Every address is checked before it can ship (`src-tauri/src/support.rs`)
- **Monero:** decoded from Monero's block base58 and checked against its
  Keccak-256 checksum; main-net standard (4…) and subaddresses (8…) only, no
  testnet, no integrated addresses. Keccak is implemented in the module (about
  60 lines) and tested against the published vectors and the Monero General
  Fund address, so no new dependency and no change to `THIRD-PARTY.md`.
- **Tron:** Base58Check checksum, `T…`, 34 characters, main-net byte 0x41.
- **Solana:** must decode to exactly 32 bytes (Solana addresses have no
  checksum). **A 64-byte value is refused as a private key**, with a message
  to delete it and never share it.
- **Lightning:** `name@domain`, lower case; a one-time `lnbc…` invoice is
  refused.
- Anything with spaces is refused as a possible seed phrase, and SeedScan
  itself must find no secret in DONATE.md.
- `the_shipped_donate_md_is_valid` runs in CI on all three systems and in the
  release workflow before anything is built, so a typo stops the release
  instead of sending someone's money to a wrong address.

## Also
- A test that every file offered in the source viewer is actually embedded.

Tests: 80 library + 22 CLI (`cargo test --release --features cli`).

---

# StackerPulse v3.1.0 — macOS, and releases built by CI

## macOS
- New target: a **universal** `.dmg` / `.app` (Apple Silicon and Intel,
  macOS 10.15+). The code needed no changes: everything Windows-specific was
  already behind `cfg(target_os = "windows")`, and the rest is shared with
  Linux. The bundle got an `icon.icns`, and an **ad-hoc signature** (`-`),
  because Apple Silicon refuses to run completely unsigned code. It is not
  notarised: a Developer ID needs a real-name Apple account. The README
  explains how to open it.
- `build_macos.sh` for local builds on a Mac.
- **Checked without a Mac**, as far as that goes. The official Rust standard
  library for both Apple targets (1.97.0, from conda-forge) was installed
  into the toolchain, and `cargo clippy --all-targets -D warnings` passes for
  `aarch64-apple-darwin` and `x86_64-apple-darwin`: every line of our code
  and of Tauri/WebKit's macOS code type-checks and lints clean, tests
  included. `seedscan-cli`, which needs no Apple frameworks, was **built and
  linked** into real Mach-O executables for both architectures with Zig and
  merged into a universal binary with `lipo`, the same step CI runs; its
  only system dependency is `/usr/lib/libSystem.B.dylib`. What could not be
  done here: linking the app itself (that needs Apple's SDK frameworks),
  bundling the `.dmg`, and running anything. The first `macos-latest` CI run
  is the final check.
- **`THIRD-PARTY.md` was missing the 31 macOS-only crates** (the `objc2-*`
  Cocoa, WebKit and Core Graphics bindings, `block2`, `dispatch2`,
  `core-foundation`, `swift-rs`…). A macOS release without them would have
  broken their licence terms. The generator now covers all four targets:
  421 crates, all permissive.

## Continuous integration (`.github/workflows/ci.yml`)
On every push to `main` and every pull request: `cargo fmt --check`,
`cargo clippy -D warnings` and the full test suite on **Windows, Linux and
macOS**, plus a check that `THIRD-PARTY.md` matches the dependencies.

Getting there required `cargo fmt` over the whole codebase (formatting only,
134 hunks), renaming `as_test_vector` → `into_test_vector` (it consumes
`self`), and switching three tests to an async `tokio::sync::Mutex`, because
a `std` guard was held across `.await`.

## Releases (`.github/workflows/release.yml`)
Pushing tag `vX.Y.Z`:
1. refuses to run unless the tag matches `Cargo.toml` and `THIRD-PARTY.md` is
   current;
2. creates a **draft** release with notes taken from the top of this file;
3. on each system: runs the tests, builds the installers with a pinned Tauri
   CLI (`.msi`, `-setup.exe`, `.deb`, `.AppImage`, `.dmg`), builds
   `seedscan-cli` separately and packages it with the licence files, and
   attaches everything;
4. publishes `SHA256SUMS.txt` and a **build-provenance attestation** for every
   file (`gh attestation verify <file> --repo …`), which stands in for the
   code-signing certificates a pseudonymous project cannot get;
5. leaves the release as a draft until you press *Publish*.

Every third-party action is pinned to a **commit SHA**, not a tag. For three
of them the tag pointed to a tag object, not the commit, so the SHAs were
resolved explicitly. Jobs get the minimum permissions they need, and checkout
does not keep the token. Both workflows pass `actionlint` with `shellcheck`.

## No developer paths inside the binaries

rustc embeds the absolute path of every source file in panic messages and
debug info. A release built by `tauri build` contained **448 copies** of the
builder's home directory (`/root/.cargo/registry/…` here; on a Windows
machine, `C:\Users\<account name>\.cargo\…`), and `seedscan-cli` 98 more.
For a pseudonymous author, one locally built `.exe` handed to someone would
have published the account name behind the nym.

- Every build path now passes `--remap-path-prefix=<home>=~` and
  `--remap-path-prefix=<project>=.` through `CARGO_ENCODED_RUSTFLAGS`
  (which, unlike `RUSTFLAGS`, survives spaces such as
  `C:\Users\Ivan Petrov`): the release workflow and all three local build
  scripts. Cargo's own `trim-paths` would do this, but it is still unstable
  in Rust 1.97.
- Each of them then **refuses to finish** if the home path is still inside
  `stacker-pulse` or `seedscan-cli`. Verified on Linux (ELF: 448 → 0) and
  on macOS (Mach-O: 98 → 0); the PowerShell check was run in real PowerShell
  7.6 against a path with a space in it.
- A `CARGO_HOME` outside the home folder (say `D:\cargo`) holds the crate
  sources too, so the scripts remap and check it as well when it is set.
- **Windows is checked on every push, not only at release time.** `ci.yml`
  builds both binaries in release mode (`--features tauri/custom-protocol`,
  which embeds the same strings as `tauri build`) with the same flags on
  Windows, Linux and macOS, and fails if the runner's home path is inside.
  No Windows compiler was available in the development sandbox, so the
  first CI run on `windows-latest` is where the Windows path format is
  actually exercised.
- The one remaining absolute path, the project folder, appears only in
  development builds (`cargo build` without the Tauri CLI) and is absent
  from `tauri build` output, which is what gets released.

## SeedScan: a chosen folder could be skipped as a whole

Found by the first CI run on Windows. The test that scans a folder in the
temp directory reported **0 files** on `windows-latest`, while Linux and
macOS passed: on Windows the temp directory is `…\AppData\Local\Temp`, and
the skip rule for `AppData/Local/Temp` was applied to the folder the user
had chosen, not only to folders found inside it. Ask SeedScan to check
something under Temp and it answered "0 files", which reads as "clean".
The same code had two more faults:

- **Hidden roots were skipped.** Choose `~/.electrum` or another folder whose
  name starts with a dot, and without `--hidden` nothing was scanned — the
  very folders where wallets keep their files.
- **Skip rules matched substrings of the path**, not folder names: the
  rule for `target` also skipped `targeted_notes` (any path containing
  `/target`) and `mytarget` (any name ending in `target`).

Now a folder the user chose is **always** scanned, and skip rules compare
whole folder names at the end of the path, case-insensitively
(`AppData/Local/Temp` needs all three in order). Fixed in the app and in
`seedscan-cli`. Four new tests reproduce the failures on any system (by
building `AppData/Local/Temp`, `.electrum` and `targeted_notes` folders in a
temp directory); three of them **fail on the old code** and pass on the new.

Also: `actions/checkout` v4 → v5 (pinned commit), which runs on Node 24 and
removes GitHub's Node 20 deprecation warning.

## Smaller fixes
- `tools/gen-third-party.py` now runs `cargo fetch` first; on a fresh CI
  runner the crate sources (and their LICENSE files) were not downloaded, so
  the regenerated file would never have matched.
- The `.deb` listed its WebKitGTK and GTK dependencies twice; Tauri already
  adds them.
- A real Tauri bundle (`.deb`) was built locally to confirm the bundle config:
  version 3.x taken from `Cargo.toml`, `LICENSE`, `THIRD-PARTY.md` and
  `DISCLAIMER.md` installed next to the app.

### The first macOS release build failed, now fixed
Tauri puts **every** binary of the package into the bundle, `seedscan-cli`
included, but a universal macOS build merges only the app binary with `lipo`.
The bundler then looked for a universal `seedscan-cli` that did not exist and
stopped (`Failed to copy binary … universal-apple-darwin/release/seedscan-cli`).
The Tauri CLI source shows the way out: a binary whose `required-features` are
not enabled is left out of the bundle. `seedscan-cli` is now opt-in
(`required-features = ["cli"]`):
- the installers contain only the app, on every system (the `.deb` no longer
  puts `seedscan-cli` into `/usr/bin`);
- the CLI ships as its own archive per system, built explicitly
  (`--features cli`; on macOS for both architectures, then merged);
- tests, clippy, the leak checks and the build scripts all pass
  `--features cli`, so the CLI's 22 tests still run everywhere. Running it
  from source: `cargo run --release --features cli --bin seedscan-cli -- <folders>`.

Verified locally: a real `tauri build --bundles deb` contains only
`stacker-pulse`, and the same test, clippy and leak commands as CI pass.

Tests: 67 library + 22 CLI (`cargo test --release --features cli`).

---

# StackerPulse v3.0.6 — the legal paperwork

Nothing here changes how the app behaves. It closes the gap between "the code
is MIT" and "the release is actually distributable".

- **`THIRD-PARTY.md`** — the one real obligation that was missing. MIT and
  Apache-2.0 both require the licence text and the copyright notice to travel
  with the binaries, and the installers bundle 390 crates. The file lists
  every crate that ships in a Windows or Linux release build (test-only
  dependencies excluded), with its licence, source and copyright holders,
  followed by the full text of each licence used. It is generated by
  `tools/gen-third-party.py`, shipped next to the installed app, **and
  compiled into the binary** so the notices cannot be separated from it.
  A test checks the texts are embedded; CI fails if the file is stale.
- Licence audit of the dependency tree: everything is permissive. Five crates
  (`cssparser`, `selectors`, `dtoa-short`, `cssparser-macros`, `option-ext`)
  are MPL-2.0, weak copyleft per file; they are used unmodified, which the
  MPL allows, and `THIRD-PARTY.md` says so and points at their source. No
  GPL or AGPL anywhere.
- **`DISCLAIMER.md`** (EN + RU) — no warranty; "no findings" is not a clean
  bill of health; scan only machines you own or are authorised to examine;
  this is not an antivirus and not professional advice; the report itself
  contains your paths and username, so treat it as sensitive; third-party
  names are used nominatively, with no affiliation implied.
- **`SECURITY.md`** — private disclosure first, what counts as a
  vulnerability in a tool whose job is to keep secrets local (a secret
  leaving the machine, an Airgap bypass, a masking failure, deanonymisation),
  and a rule that reports must use published test vectors, never live keys.
- **`CONTRIBUTING.md`** — contributions are MIT, inbound equals outbound, no
  CLA, pseudonyms welcome; plus the rules that are not negotiable in a
  privacy tool (nothing leaves the machine, network only in Rust behind the
  allowlist, secrets always masked, scanner stays read-only, every detector
  change needs a test both ways).
- Both notices are readable inside the app, on the Settings & privacy tab.

Tests: 63 library + 22 CLI.

---

# StackerPulse v3.0.5 — one version number, everywhere

Reported: the header said "v3.0" in every 3.0.x build, and SeedScan said
"v0.2". The version was typed by hand in four places (Cargo.toml,
tauri.conf.json, two labels in index.html) and they drifted.

- **Cargo.toml is now the only place the version is set.** tauri.conf.json no
  longer repeats it (Tauri v2 then takes it from Cargo.toml, so the installer
  gets the same number). The header and the SeedScan label ask Rust
  (`sn_app_version`, `env!("CARGO_PKG_VERSION")`) at startup.
- `seedscan-cli --version` works and the report header shows the version.
- A test fails the build if tauri.conf.json gets a `version` again or a
  version number is hard-coded in the UI.
- The release workflow refuses to build when the git tag (`v3.0.5`) does not
  match Cargo.toml.

---

# StackerPulse v3.0.4 — SeedScan without the noise

A real scan of a Windows profile (124,471 files) reported 8,637 "findings" in
50 files. Almost all were noise:

- **8,612 path "leaks"** were `/home/someone/` and `/Users/<name>/` inside
  third-party READMEs and code. In the folder scan a path now counts only if
  it contains **your own** account name, and it is reported once per file
  (`C:\Users\<user>\ (12×)`). The draft/inspector still flags any name,
  because there you are about to publish the text.
- **Seed phrases in Python code** (`target_width, crop_w)` was split into
  BIP39 words) and in a quoted word list. Words now have to be plain words
  separated by spaces or new lines (numbering like `1.` and commas are fine);
  a word glued to `_`, quotes or brackets ends the run. Runs longer than 30
  words (word lists, prose) are ignored. Seeds written one word per line are
  now found too.
- **Solana keypair files** are now verified: the last 32 bytes must be the
  ed25519 public key of the first 32. Random 64-number arrays no longer count.
  Real keys inside `tests/`, `fixtures/`, `examples/` are listed separately as
  likely test data.
- **Spec examples** (BIP39 `abandon … about`, BIP32 xprv, NIP-19 nsec, NIP-47
  NWC secret, the Bitcoin-wiki WIF) are marked "public test vector" instead of
  alarming you about the app's own sample text.
- The report is grouped: **secrets**, **your username in files**, **likely
  test data**; the toast counts only real secrets.

The inspector ("Secrets, NWC and deanonymization inspector"):

- **A seed with a typo is now flagged.** Before, 12 BIP39 words with a wrong
  checksum gave "nothing found" — which is what you get when you type a test
  seed by hand. Now it shows "possible seed phrase (checksum does not match)":
  a seed with one wrong word can still be brute-forced.
- New for drafts and logs: **Solana private key in base58** (wallet export,
  verified with ed25519, so transaction signatures do not match),
  **e-mail addresses** and **public IP addresses** (private ranges and
  `example.com` are skipped).
- Labels and advice in Russian when the interface is in Russian; colours by
  severity (red secret, amber identity, grey test example).
- The scan runs off the UI thread and after a 250 ms pause in typing, so a
  big pasted log no longer freezes the window on every keystroke.

Tests: 61 library + 22 CLI. New: the exact ComfyUI line that was flagged,
a quoted word list and the full 2048-word BIP39 list, one-word-per-line and
numbered seeds, test-vector marking, typo seeds (text mode only), own-username
filtering, ed25519 keypair checks, e-mail/IP.

---

# StackerPulse v3.0.3 — the disk scan scanned nothing

Reported: the leak finder "did not work". The text scanner was fine (the
sample text gives 4 findings in Rust), but the disk scan was broken:

- **`~` was not expanded.** The default folders were `~/Downloads,
  ~/Documents, …`, and neither Rust nor PowerShell expands `~`, so the scan
  looked for a folder literally named `~`, found nothing, and reported
  `scanned_files: 0` as if everything were clean. Now `~`, `~/x`, `~\x`,
  `%USERPROFILE%` and `$HOME` are expanded (app and CLI).
- **Missing folders are reported** instead of silently counted as clean
  (`roots` / `missing` in the report; the CLI prints a warning).
- **Readable report** in the interface language instead of raw JSON, same
  layout as `seedscan-cli`.
- **The window no longer freezes** during a long scan: the scan runs off the
  UI thread, and the button is disabled while it runs.
- **All tabs are visible.** The tab bar scrolled sideways, so on a normal
  window "SeedScan & OpSec" and "Settings & privacy" were off-screen; it
  now wraps to a second row.
- Default folders now include `~/OneDrive` (where Windows often keeps
  Documents and Desktop) instead of `~/Dropbox`.

Tests: 52 library + 13 CLI. New: `~` / `%USERPROFILE%` / `$HOME` expansion;
a scan of a real temp folder plus a missing `~/…` folder finds the seed
phrase and lists the missing folder.

---

# StackerPulse v3.0.2 — when SN's .onion is offline

Second report from Windows: the feed loaded (through the backup route), but
links copied as `.onion` gave "Onion site not found" in Tor Browser. The
address is still the official one (it is the `Onion-Location` header in SN's
current nginx config), so the onion service itself was down.

- **Links follow the route that worked.** If the backup route answered,
  Rust builds links as `https://stacker.news/...`, and the UI no longer
  rewrites them to `.onion` on copy or when Tor is toggled. They open in Tor
  Browser over Tor.
- **The dead onion is skipped for 10 minutes** after it fails, so each load
  no longer waits up to a minute before the backup route. After 10 minutes
  (or "↻ Try the .onion again" in Settings) it is tried again.
- **Visible route.** The feed status shows `Tor → .onion` or
  `Tor → stacker.news`; Settings shows whether the onion answers.
- New command `sn_onion_status` (read or reset the remembered state).

Tests: 50 library + 13 CLI. New: after an onion failure the next request goes
straight to `stacker.news`; links stop pointing at the onion once the backup
route answered.

---

# StackerPulse v3.0.1 — Tor errors you can act on

Reported from the first real run on Windows: with Tor on, the feed failed with
`error sending request for url (http://snsn…onion/api/graphql)` and nothing
else. A new test reproduced the cause: reqwest 0.12 reports "nothing is
listening on the proxy port" as `SOCKS error: failed to create underlying
connection` and drops the OS error, so the real reason (usually Tor Browser
on 9150 while the app pointed at 9050) was invisible.

- **Readable network errors.** Rust now returns `NET:<kind>|<detail>` with the
  full error chain; the UI explains it in RU/EN: Tor not running on that
  port (and on which port it *was* found), timeout, onion unreachable, bad
  proxy scheme, TLS. The text also appears inside the empty feed box, not
  only in a cut-off badge.
- **Find Tor.** New `sn_detect_tor` command checks 127.0.0.1:9150 and :9050
  (TCP connect only, nothing sent). Runs automatically when Tor is turned on
  and switches the proxy address if Tor is on the other port; also a
  "🔎 Find Tor" button. The chosen proxy is remembered between launches.
- **Backup route.** If the .onion times out or Tor reports it unreachable, the
  request is retried once to `https://stacker.news` through the same Tor
  proxy (IP stays hidden). On by default, switchable in Settings. Not used
  when Tor itself is not reachable.
- **Longer Tor timeouts:** 45 s to connect, 60 s total (was 25 s total —
  too short for a cold onion circuit).
- **No system proxy.** The SN client now ignores `HTTP(S)_PROXY` and the
  Windows system proxy, like the GitHub client already did.
- **`socks5://` is refused** for the SN client too (DNS would bypass Tor);
  only `socks5h://` is accepted.
- **BTC price** is no longer an editable field with a made-up default
  (82,450). It shows "—" until SN returns its price, then `$98,765 ·
  $1 = 1,013 sats`; dollar amounts show `$—` until then. "SN rewards pool"
  is renamed "SN rewards today" with a tooltip saying what it is.
- Playground errors use the same readable text.

Tests: 49 library + 13 CLI. New: nothing on the proxy port → `TOR_REFUSED`;
a fake SOCKS5 server answering "host unreachable" → the onion is tried, then
`stacker.news` through the same proxy; fallback off → only one attempt;
`socks5://` refused; Windows/Linux/reqwest error texts classified.

---

# StackerPulse v3.0 — live data, no demo numbers

**Verified on Linux:** release build with zero warnings; `cargo test --release`
passes (44 library + 13 CLI tests). All four GraphQL queries the app sends
(`FEED_QUERY`, `MARKET_QUERY`, `USER_QUERY`, `TOP_SUBS_QUERY`) and all four
playground presets validate against the schema built from the
`stackernews/stacker.news` source (`api/typeDefs`). The UI was driven in a
headless browser against a mocked IPC layer (feed, bounties, territories,
stacker stats, wipe, RU/EN), and the real Tauri window was launched on a
virtual display: pressing "Load from SN" reaches the Rust command (the sandbox
has no network, so it correctly reports the connection error).

**Not verified here:** responses from the live SN API (no network in the test
sandbox) and the Windows bundles. Smoke-test both before release.

## Removed: everything that was made up
- Hard-coded feed posts, bounties, territory table and stacker profiles
  (with invented nyms) are gone. Every tab starts empty and fills only from
  SN, on request.
- The activity heatmap, the "68%" claim and the SATS estimator had no data
  behind them and were removed.
- The local territory formula (`economics.rs`) is gone: a founder's share is
  configurable per territory on SN, so a formula with defaults was wrong for
  many territories. The app now shows actual revenue.
- Developer-only controls (build cards, IPC test button, glass/OS toggles)
  are gone. The Rust tab is now **Settings & privacy**: Tor proxy and the
  source viewer.

## Added: live SN data, computed in Rust (`src-tauri/src/sn.rs`)
- **Feed** — `items(sub, sort, type, when, limit)`; sorts `hot`/`new`/`top`
  (SN treats unknown sorts as `hot`, so input is validated).
- **SN bounties** — active, unpaid bounties.
- **Territory economics** — `topSubs` with paging (up to 8 pages): founder
  revenue in the period, scaled to 30 days and compared with the plan
  (MONTHLY 50k, YEARLY 500k → 41,096 per 30 days, ONCE 3M → months to earn
  back; `covers_cost` is empty for one-time plans rather than a fake yes/no).
- **Stacker stats** — `user(name)` for day / week / month / year / forever;
  hidden stats stay hidden (shown as "hides their stats").
- **Market** — BTC/USD (`price` returns -1 when unknown; treated as none)
  and the rewards pool.
- Input validation in Rust: period, sort and nym (letters, digits, `_`;
  leading `@`/`~` stripped). Responses are capped at 4 MB while streaming;
  GraphQL `errors` become a readable error instead of an empty table.

## Fixed
- Two playground presets queried fields that do not exist on SN
  (`User.sats`, `User.stacked`, `User.ncomments`, `Sub.postCost`) and always
  failed. They now use `optional { stacked spent streak maxStreak }` and
  `baseCost`.
- Status lines kept the language they were written in; switching RU/EN now
  re-labels them. Long network errors are truncated in the toolbar (full text
  on hover).
- After a failed stacker lookup the previous person's numbers stayed on
  screen under the new nym; they are now cleared.
- "Wipe local data" now also clears loaded feed, bounties, territories and
  stacker stats and resets their status lines.
- Turning Tor on or off overwrote the feed status with "demo data"; it no
  longer touches it.
- The header claimed "Linux Wayland" blur; acrylic is applied on Windows
  only, so the badge now just says Windows · Linux.

---

# Security and correctness fixes — v2.9.0

Every fix below has a test.

**Verified end to end on Linux:** the full Tauri project builds in release
mode with zero warnings, and `cargo test --release` inside `src-tauri` passes
(43 library tests + 13 CLI tests). The app was launched in a real Tauri window
on a virtual display under the new strict CSP; a probe build confirmed the
UI → Rust IPC works for `sn_sanitize_url`, `sn_execute_graphql` (with the exact
object the UI sends), Airgap refusal, and `sn_scan_text_secrets`.

**Not verified here:** the Windows `.exe` / `.msi` bundles. Run
`build_windows.ps1` and smoke-test before release.

## High

### The webview loaded remote JavaScript on every launch
`ui/index.html` pulled Tailwind from `cdn.tailwindcss.com` and Chart.js from
`cdn.jsdelivr.net`, without integrity hashes, and `tauri.conf.json` had
`"csp": null`. That meant:

- two non-allowlisted hosts were contacted at startup, outside Tor and outside
  the Rust allowlist, even with Airgap mode on;
- whatever those CDNs served ran inside the privileged webview, with access to
  every Tauri command, including the disk scanner.

**Fixed:** Chart.js 4.4.1 is vendored from the official npm package
(`ui/vendor/chart.umd.js`, sha256
`74401d738dd3e03ee5dfb3b6841210fe2c4ead8a960c4011ca4ba0b78a9fd8f3`). Tailwind
is compiled at build time with the same theme (`tools/tailwind.config.js` →
`ui/vendor/tailwind.css`). The inline app script moved to `ui/app.js`. A strict
CSP is now set: `script-src 'self'`, `connect-src` limited to Tauri IPC.

Verified in Chromium: zero external requests on load, no JS errors, Chart.js
loads, styles apply, and a simulated injected `<script src="https://…">` is
refused by the CSP.

Rebuild the CSS after changing classes in the UI:

```
npx tailwindcss@3.4.17 -c tools/tailwind.config.js -i tools/tailwind.input.css -o ui/vendor/tailwind.css --minify
```

### The host allowlist did not survive a redirect (`graphql.rs`)
The allowlist was checked on the initial URL only, and reqwest's default policy
follows up to 10 redirects to any host. A single `302` took the client off the
allowlist.

**Fixed:** a custom redirect policy re-applies the allowlist on every hop and
caps the chain at 3. Tests run two local servers: with the old default policy
the client reaches the second host (bug reproduced); with the new policy it
does not.

### The NWC mask printed the tail of the secret (`detect.rs`)
`nostr+walletco**********23456789abcdef` — 14 of 32 hex characters.

**Fixed:** the pubkey and the whole secret value are redacted. A test asserts no
6-character run of the secret appears in the output.

## Medium

### `sn_execute_graphql` rejected every call from the UI
The UI never sends `airgap_mode`, and `NetworkPrivacyConfig` required it, so
every invocation failed with "missing field `airgap_mode`".
**Fixed:** `#[serde(default)]` on the struct; a test uses the exact object the
UI sends.

### The CLI skipped NWC secrets and path leaks
`seedscan_cli.rs` never called `find_nwc_and_opsec`, though the README says it
detects both. **Fixed.**

### Link sanitizer stripped `t` and `s` from every host (`sanitizer.rs`)
YouTube timestamps and search queries were silently removed.
**Fixed:** trackers are global (`utm_*`, `fbclid`, `gclid`, …) or host-scoped
(`s`/`t` only on x.com, `si` only on YouTube/Spotify, `ref` only on GitHub).

### The path detector printed the username it warns about (`detect.rs`)
**Fixed:** shown as `C:\Users\<user>\`. Built-in profiles and doc placeholders
(`Public`, `Default`, `user`, …) are no longer reported.

### ROI model panicked on large input (`economics.rs`)
**Fixed:** all arithmetic saturates. Results for normal inputs are unchanged
(test pins the previous output; rounding may differ by at most 1 sat).

### The project could not build: no icons
`tauri::generate_context!()` requires `src-tauri/icons/icon.png`, and the
archive had no `icons/` folder, so compilation stopped with "failed to open
icon". **Fixed** with a placeholder set (yellow square with a lightning bolt,
matching the UI logo) registered under `bundle.icon`. To use your own artwork,
replace `icons/source-1024.png` and run `cargo tauri icon icons/source-1024.png`.

## Low

- Comment revenue was hardcoded at 1 sat. Added `comment_cost_sats` with a
  serde default of 1, so the current UI keeps working.
- Airgap mode now refuses to construct an HTTP client at all.
- `detect.rs` regained its test suite, plus tests for every fix above.
- `tauri.conf.json` version aligned with `Cargo.toml` (2.8.0).
- Linux: `<select>` controls rendered white with near-invisible text, because
  WebKitGTK uses the light GTK theme unless the page declares `color-scheme:
  dark`. Declared; controls are now dark.
- Two compiler warnings on non-Windows targets (`window`, `tauri::Manager`
  only used for Acrylic) — now compiled only on Windows. Build is warning-free.

## Round 2 — former open items

### The UI sanitizer ran its own copy with the old bugs (`app.js`)
The "Очистить URL" button never called Rust: it used a JS copy that still
stripped `t`/`s`/`ref` from every host, and matched hosts with
`hostname.includes('x.com')` — so `fox.com`, `box.com` or `netflix.com` links
were rewritten to `xcancel.com`. **Fixed:** the button calls `sn_sanitize_url`;
the browser-preview fallback mirrors `sanitizer.rs` with exact host matching.
The duplicate check now compares paths regardless of host.

### The live secret scanner and the draft guard ran the JS copy (`app.js`)
Both used `scanTextForSecretsJS`, which printed 14 characters of the NWC secret
and the full username — the same bugs fixed in `detect.rs` — and has no BIP39
checksum. Its `<user>` placeholder would also have been eaten by `innerHTML`.
**Fixed:** inside the app both call the Rust `sn_scan_text_secrets`; the JS
fallback masks NWC and paths like Rust does; every field is HTML-escaped.

### Failures were shown as success or as fake data (`app.js`)
- "Проверить мост Rust IPC" said "Вызов Rust IPC выполнен" when the call threw.
  **Now shows the error.**
- A failed GraphQL request showed a "verified snapshot" built from the local
  demo feed, labelled as an API response. **Now shows the error.**
- A failed disk scan fell through to a hardcoded "Scanned 4,812 files. Found 3
  item(s)" report. **Now shows the error;** the browser preview labels the
  sample as DEMO.
- Live GraphQL results are now merged into the feed in the app too (before,
  only the browser `fetch()` path did that).

### Browser fallback removed (`app.js`)
Without `window.__TAURI__`, the GraphQL playground no longer calls `fetch()`;
it explains that requests run only through the Rust client. The UI also passes
`airgap_mode` to Rust as a second lock.

### Stale source viewer replaced (`lib.rs`, `app.js`)
The hand-copied `RUST_SOURCES` strings are gone. A new command
`sn_get_rust_source` serves the real files via `include_str!`, so the viewer
shows exactly what was compiled. Only a fixed list of names is served; nothing
is read from disk at runtime.

### Bundle identifier
`news.stacker.pulse` → `dev.ubro.stackerpulse`.

### Repository link removed (`Cargo.toml`)
`repository = "https://github.com/UBro-dev/stacker-pulse"` pointed to a
repository that does not exist. If anyone else registered that name, the
package metadata (and the in-app `Cargo.toml` viewer) would send readers to
them. Add the field back once the real repository exists.

### `cargo run` did not know which binary to start (`Cargo.toml`)
The package has two binaries (`stacker-pulse`, `seedscan-cli`). Added
`default-run = "stacker-pulse"`, so `cargo run --release` opens the app;
the CLI is `cargo run --release --bin seedscan-cli -- <folders>`.

### `DNT` / `Sec-GPC` removed (`graphql.rs`)
They added fingerprint bits and are ignored by almost every server.

### Nitter / Redlib instance is configurable (`sanitizer.rs`, UI)
New optional field next to the frontend selector; `sn_sanitize_url` takes an
optional `instance`. Only a bare hostname is accepted (no scheme, port, path or
`user@`), tested. Empty → defaults `nitter.net` / `redlib.catsarch.com`. Added
the missing `reddit.com → Redlib` option to the selector.

Status of the defaults as of October 2026: X sent Nitter a cease and desist on
24 Aug 2026 and nitter.net went offline; the developers announced in early
September that it would return. xcancel was reported back online at the same
time. Neither could be reached from the build sandbox, so check both before
release.

## Round 3 — found while testing on Windows

### Links did nothing in the app (`app.js`)
Every link was a plain `<a target="_blank">`. A Tauri window ignores those
without an opener, so "Открыть SN" and every title link were dead. Opening
them in the system browser would also bypass Tor. **Now every external link is
copied to the clipboard** (as the `.onion` address when Tor is on), with a
toast saying where to paste it. No new dependencies. The buttons read
"Копировать ссылку" / "Copy link".

### Airgap never blocked the GraphQL playground (`app.js`)
`airgapMode` and `currentLang` were declared inside the `DOMContentLoaded`
handler, but `executeSnGraphql` lives outside it. `typeof airgapMode` was
always `"undefined"` there, so the Airgap check was always false, and the
language toasts in that function threw a `ReferenceError`. **Both moved to
top level;** verified that "Sync" with Airgap on now returns
`BLOCKED_BY_AIRGAP_LOCK`.

### English mode was half Russian (`i18n.js`, `app.js`)
The switch translated about 26 elements; about 300 strings were hardcoded.
**Fixed:** `ui/i18n.js` holds a RU→EN dictionary for all static text,
placeholders and titles, applied by a DOM walker plus a MutationObserver for
content rendered later. Strings with numbers or names use `L(ru, en)` in
`app.js`. The choice is saved (`sn_pulse_lang`) and wiped by "Сбросить
локальный кэш". Verified in Chromium: 433 Cyrillic text nodes in Russian mode,
0 from the UI in English mode (only the sample draft body, which is user
content, stays Russian).

### Demo data used real Stacker News nyms (`app.js`, `index.html`)
Invented posts, bounties, territory founders and per-user stacking statistics
were attributed to real people, and one bounty linked to an unrelated item ID.
**Replaced** with `demo_alpha` … `demo_zulu`; SN-core attributions on invented
bounties read "SN repo (demo)"; demo items link to their territory page
instead of item IDs that may not exist; the dataset and the source badge say
"demo". The GraphQL example query for `@k00b`'s public profile is kept — it is
a real API call, not attributed content.

### Smaller
- Live API titles, nyms and URLs are HTML-escaped before rendering (the CSP
  already blocked scripts; this also blocks injected markup).
- The disk scan no longer runs automatically when the app starts; it runs when
  the button is pressed. The browser preview still shows the labelled sample.
- The GraphQL panel text no longer promises a "cache fallback on CORS
  block" that was removed.

## Round 4 — v2.9.0: bounty radar, territory model, tracker

Features taken as ideas from a review by another model (its v3.0 archive did
not compile and disconnected the UI from Rust), rewritten here with tests.

### GitHub PR bounty radar (`github.rs`, new)
Stacker News pays for PRs that close issues in `stackernews/stacker.news`.
The award comes from labels (README, "Pull request awards"):
`difficulty:good-first-issue` 20k, `easy` 100k, `medium` 250k,
`medium-hard` 500k, `hard` 1m sats; `priority:low` ×0.5, `medium` ×1.5,
`high` ×2, `urgent` ×3. The radar shows that figure as "up to", because each
substantial review change takes 10% off and awards are discretionary. Issues
without a difficulty label show **no amount** — nothing is guessed.

Privacy rules, enforced in Rust:
- off by default ("Allow requests to GitHub"), requests only on button press;
- host `api.github.com` only, two fixed paths for one repository, URLs built
  from constants and checked again before sending; search words containing
  `:` are dropped, so a search cannot leave the repository;
- no redirects at all;
- without Tor the request is refused (`TOR_REQUIRED`) unless the user confirms
  in the UI that GitHub will see their IP; with Tor only `socks5h://` is
  accepted;
- Airgap refuses; no token, no cookies, no Referer; the system proxy is never
  used for the clearnet client;
- response capped at 2 MB; issue URLs are rebuilt from the number rather than
  trusted from the response; results cached locally with "updated N min ago"
  and the remaining GitHub rate limit.

Tests: award table and multipliers, no label → no amount, fixed URL, search
cannot leave the repo, firewall rejects other hosts/ports/paths/repos/
credentials/qualifiers, Airgap and missing consent refuse before any request,
`socks5://` rejected, PRs skipped and URLs rebuilt, and a local server proves
redirects are not followed. Verified in a real window: without Tor the UI asks
first and Rust returns `TOR_REQUIRED` if called without consent.

Not verified here: a live response from GitHub (the sandbox cannot reach it).
Test once through Tor.

### Territory model — configurable, calculated in Rust (`economics.rs`)
The UI calculator now calls `sn_simulate_territory` (a JS copy of the same
formula runs only in the browser preview). Every rate is an input, with
defaults from the FAQ in the SN repository, checked 2026-10-25: 70% of post,
comment and boost fees and 21% of zaps to the founder; 50k / month, 500k /
year, 3M once. The launch article (2023) said 50% and 100k / 1M / 3M, so the
UI names its source and has a "reset to FAQ values" button. Added a comment
cost slider. Yearly plans are charged at the start of each year in the
cumulative chart.

### Personal bounty tracker (UI)
Statuses per task (in progress / PR or claim sent / paid to me), editable
hours, own tasks with a form, "only ones I track" filter, CSV export. Stored
only in local storage and wiped by "Wipe local cache".

### Other
- User-Agent option in the Tor panel (off = no UA; on = Tor Browser's UA,
  Firefox ESR 140 on Windows — update `BROWSER_UA` when Tor Browser moves to a
  new ESR). GitHub always gets that UA because it rejects requests without one.
- Search refresh buttons and Enter for the radar and bounty search.
- Network calls share one `getPrivacyConfig()`.
- New strings translated; Tailwind CSS rebuilt; version 2.9.0.

## Open items — decide yourself

- **Unverified numbers in the UI text:** "68% fewer sats at US night hours
  (discussion #3014)" and the payouts in the SATS estimator ("Bsats app:
  207.8k sats", …). Check them against the sources or soften the wording
  before posting screenshots.
- **No `User-Agent` by default.** `strip_user_agent: true` sends none, which is
  itself unusual. Left as is because it is a documented option and a Rust TLS
  handshake does not look like a browser anyway, so a borrowed browser UA buys
  little. Worth a line in the README rather than calling it anonymity.
