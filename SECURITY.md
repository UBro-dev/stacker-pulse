# Security policy

## Reporting a vulnerability

Please report security problems **privately first**, so that a fix can ship
before the details are public.

- **Preferred:** GitHub → *Security* → *Report a vulnerability* (private
  advisory). It is the only channel where the report, the discussion and the
  fix stay in one place and stay private until the advisory is published.

A useful report says what the bug is, how to reproduce it, and what an
attacker gains. A proof of concept helps.

**Never include a real secret in a report.** If a reproduction needs a seed
phrase, a private key or an `nsec`, use a published test vector — the BIP39
`abandon abandon … about` phrase, the BIP32 test vector `xprv…`, the NIP-19
example `nsec…` — or generate a throwaway key with no funds and say so.

## What counts as a vulnerability here

This project exists to keep secrets on the machine and to keep its user
anonymous, so the bugs that matter most are:

- **A secret leaving the machine.** Any path by which a seed phrase, key or
  scan result reaches the network, another process, the clipboard, a log file
  or a crash report.
- **A secret being written to disk** by the application, in any form,
  including unmasked findings, caches or temporary files.
- **Deanonymisation.** A request that bypasses the host allowlist or Tor, a
  DNS lookup outside the SOCKS proxy, a leaked `User-Agent`, `Referer` or
  locale header, or a build artefact that carries a developer's username or
  time zone.
- **A bypass of Airgap mode**, which must prevent every outbound connection.
- **The updater** installing anything not signed with the project key,
  accepting an older signed package as a newer version, contacting a host
  other than GitHub, or checking without being asked or without Tor when Tor
  is on.
- **Remote code execution in the webview**, including a CSP bypass, or any
  Tauri command that can be made to read or write a path it should not.
- **A masking failure**: output that reveals enough of a secret to be useful.

Lower priority, still worth reporting: a detector that misses a real secret
(false negative), or one that fires on ordinary data (false positive).

## Out of scope

- Findings produced by running the scanner on a machine you do not own.
- Reports that the tool "did not find" a format it never claimed to support;
  see [`DISCLAIMER.md`](DISCLAIMER.md) for the scope.
- Vulnerabilities in Stacker News itself. Report those to Stacker News.
- Vulnerabilities in dependencies, unless this project uses them in a way
  that makes the problem worse. Report those upstream as well.

## Response

This is a one-person project, so there is no paid bounty and no guaranteed
response time. Expect an acknowledgement within a week. Fixes for anything
that leaks a secret or deanonymises a user take priority over all other work.

Reporters are credited in the release notes by whatever name or nym they
choose, or not at all if they prefer.

## Supported versions

Only the latest release is supported. Security fixes go into a new release;
older versions are not patched.
