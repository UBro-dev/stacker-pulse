# Contributing

Patches, bug reports and detector rules are welcome.

Before reporting a **security** problem, read [`SECURITY.md`](SECURITY.md):
those go through a private channel, not a public issue.

## Licensing of contributions

By opening a pull request you agree that your contribution is licensed under
the **MIT License**, the same licence as the rest of the project
(see [`LICENSE`](LICENSE)), and you confirm that you have the right to submit
it: that you wrote it, or that it comes from a source whose licence allows it
and that you have credited that source in the pull request.

Please do not paste code from a project under a licence that is incompatible
with MIT, and do not copy code out of a proprietary codebase you work on.

You keep the copyright in what you write. There is no contributor licence
agreement to sign, and no copyright assignment.

You may contribute under a pseudonym. A `Signed-off-by:` line is welcome but
not required, and a nym is fine there too.

## Ground rules for this codebase

This is a privacy tool, so some things are not up for negotiation:

- **Nothing may leave the machine.** No telemetry, no analytics, no crash
  reporting, no update check, no remote fonts, scripts or stylesheets.
- **Network access stays in Rust**, behind the host allowlist, and the
  allowlist is re-checked on every redirect. The webview never makes its own
  requests; the Content-Security-Policy must keep it that way.
- **Secrets are masked everywhere** — on screen, in reports, in logs and in
  test output. A test may use a published test vector, never a live key.
- **The scanner is read-only.** It must not write, move, rename or delete a
  file it scanned.
- **Dependencies are added sparingly** and must be permissively licensed
  (MIT, Apache-2.0, BSD, ISC, Zlib or similar). A new dependency needs a
  reason in the pull request. After adding one, run
  `python tools/gen-third-party.py` and commit the regenerated
  `THIRD-PARTY.md`.
- **Binaries must not name their builder.** Build distributable binaries
  with `build_*.sh` / `build_windows.ps1` or CI, which remap home and
  project paths out of the binary and fail if any remain.

## Before you open a pull request

```bash
cd src-tauri
cargo test --release     # all tests must pass
cargo clippy --release -- -D warnings
cargo fmt --check
```

After changing CSS classes in the interface, rebuild Tailwind:

```bash
npx tailwindcss@3.4.17 -c tools/tailwind.config.js -i tools/tailwind.input.css -o ui/vendor/tailwind.css --minify
```

**Every detector change needs a test**, and a new detector needs both: a case
that must fire and a case that must not. False positives are treated as bugs
of the same severity as false negatives — a scanner nobody trusts is a
scanner nobody runs.

The interface is bilingual (Russian and English). New strings go into both;
`ui/i18n.js` holds the table.
