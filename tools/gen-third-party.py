#!/usr/bin/env python3
"""Regenerates THIRD-PARTY.md: every crate that ships in a release build,
its licence, and the full text of each licence used.

MIT and Apache-2.0 both require that the licence text and the copyright
notice travel with the binaries, so this file has to go out with the
installers, not only with the source.

Usage (from the repository root):
    python tools/gen-third-party.py

Run it again whenever dependencies change; CI checks that it is current.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "src-tauri" / "Cargo.toml"
OUT = ROOT / "THIRD-PARTY.md"
# The platforms release builds are produced for.
TARGETS = [
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
    # The universal macOS build links both architectures.
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
]

# Assets vendored into ui/vendor that are not Cargo dependencies.
VENDORED = [
    {
        "name": "Chart.js",
        "version": "4.4.1",
        "license": "MIT",
        "repository": "https://github.com/chartjs/Chart.js",
        "copyright": "Copyright (c) 2014-2023 Chart.js Contributors",
        "note": "ui/vendor/chart.umd.js, unmodified, with its licence banner intact.",
    },
    {
        "name": "Tailwind CSS",
        "version": "3.4.17",
        "license": "MIT",
        "repository": "https://github.com/tailwindlabs/tailwindcss",
        "copyright": "Copyright (c) Tailwind Labs, Inc.",
        "note": "ui/vendor/tailwind.css is generated at build time by the Tailwind CLI.",
    },
]


def shipped_packages():
    """Packages reachable from the root crate for the release targets,
    excluding dev-dependencies (test-only code is not distributed)."""
    # Licence files are read from the downloaded crate sources; on a fresh
    # machine `cargo metadata` alone does not download them.
    subprocess.run(["cargo", "fetch", "--manifest-path", str(MANIFEST)],
                   check=True, capture_output=True)
    found = {}
    for target in TARGETS:
        raw = subprocess.run(
            ["cargo", "metadata", "--format-version", "1",
             "--filter-platform", target, "--manifest-path", str(MANIFEST)],
            capture_output=True, text=True, check=True,
        ).stdout
        meta = json.loads(raw)
        pkgs = {p["id"]: p for p in meta["packages"]}
        nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
        root = meta["resolve"]["root"]
        seen, stack = set(), [root]
        while stack:
            cur = stack.pop()
            if cur in seen:
                continue
            seen.add(cur)
            for dep in nodes[cur]["deps"]:
                kinds = {(k.get("kind") or "normal") for k in dep["dep_kinds"]}
                if kinds <= {"dev"}:
                    continue
                stack.append(dep["pkg"])
        for pid in seen:
            p = pkgs[pid]
            if pid == root:
                continue
            found[(p["name"], p["version"])] = p
    return found


def normalise(expr):
    """"MIT/Apache-2.0" and "Apache-2.0 OR MIT" name the same choice."""
    return " OR ".join(sorted(re.split(r"\s*(?:/|\sOR\s)\s*", expr.strip())))


def licence_files(pkg):
    """The licence texts a crate ships, so each copyright holder is credited."""
    src = pkg.get("manifest_path")
    if not src:
        return {}
    out = {}
    for f in sorted(Path(src).parent.glob("LICENSE*")) + sorted(Path(src).parent.glob("LICENCE*")):
        if f.is_file() and f.stat().st_size < 60_000:
            try:
                out[f.name] = f.read_text(encoding="utf-8", errors="replace")
            except OSError:
                pass
    return out



# Licence texts are identified by their content, because crates name the same
# file LICENSE, LICENSE-MIT, LICENSE.txt or LICENSE.md interchangeably.
SIGNATURES = [
    ("Apache License, Version 2.0 (with LLVM exceptions)", ["llvm exception"]),
    ("Apache License 2.0", ["apache license", "version 2.0"]),
    ("MIT License", ["permission is hereby granted, free of charge"]),
    ("Mozilla Public License 2.0", ["mozilla public license"]),
    ("BSD 3-Clause License", ["neither the name of", "redistributions of source code"]),
    ("BSD 2-Clause License", ["redistributions of source code"]),
    ("ISC License", ["permission to use, copy, modify, and/or distribute"]),
    ("zlib License", ["altered source versions must be plainly marked"]),
    ("Boost Software License 1.0", ["boost software license"]),
    ("The Unlicense", ["this is free and unencumbered software released into the public domain"]),
    ("CC0 1.0 Universal", ["cc0 1.0 universal"]),
    ("Unicode License v3", ["unicode"]),
    ("0BSD License", ["permission to use, copy, modify, and/or distribute this software for any purpose"]),
]


def classify(text, fname, crate):
    low = " ".join(text.lower().split())
    for label, needles in SIGNATURES:
        if all(n in low for n in needles):
            return label
    return f"{fname} (from {crate})"

COPYRIGHT_RE = re.compile(r"^.*copyright\s*(?:\(c\)|©)?\s*\d{4}.*$", re.I | re.M)


def copyright_lines(texts):
    lines = []
    for text in texts.values():
        for m in COPYRIGHT_RE.findall(text)[:3]:
            line = " ".join(m.split())
            # Skip the boilerplate line inside the Apache-2.0 appendix.
            if "[yyyy]" in line or "[name of copyright owner]" in line:
                continue
            if line not in lines:
                lines.append(line)
    return lines


def main():
    pkgs = shipped_packages()
    by_licence = {}
    rows = []
    all_texts = {}

    for (name, version), pkg in sorted(pkgs.items(), key=lambda kv: (kv[0][0].lower(), kv[0][1])):
        expr = normalise(pkg.get("license") or "")
        texts = licence_files(pkg)
        holders = copyright_lines(texts)
        rows.append((name, version, expr, pkg.get("repository") or "", holders))
        by_licence.setdefault(expr, []).append(f"{name} {version}")
        for fname, text in texts.items():
            all_texts.setdefault(classify(text, fname, name), text)

    for v in VENDORED:
        rows.append((v["name"], v["version"], v["license"], v["repository"], [v["copyright"]]))

    lines = [
        "# Third-party licences",
        "",
        "StackerPulse is MIT-licensed (see `LICENSE`). It is built on the open-source",
        "work listed below, and each entry keeps its own licence and copyright.",
        "",
        f"This file is generated by `tools/gen-third-party.py` and covers the",
        f"**{len(pkgs)} crates** linked into a release build for "
        + " and ".join(f"`{t}`" for t in TARGETS)
        + ", plus the assets vendored into `ui/vendor`. Test-only dependencies are",
        "not listed because they are not distributed.",
        "",
        "## Weak copyleft: MPL-2.0",
        "",
        "A few crates are under the Mozilla Public License 2.0. They are used",
        "unmodified, and the MPL applies per file: their source is available from the",
        "repositories listed below and on crates.io. If you modify one of those files",
        "and distribute the result, that file must stay under the MPL-2.0 and you must",
        "make its source available.",
        "",
        "## Packages",
        "",
        "| Package | Version | Licence | Source |",
        "|---|---|---|---|",
    ]
    for name, version, expr, repo, _ in rows:
        link = f"[{repo}]({repo})" if repo.startswith("http") else repo or "—"
        lines.append(f"| {name} | {version} | {expr} | {link} |")

    lines += ["", "## Copyright holders", ""]
    for name, version, expr, _repo, holders in rows:
        if holders:
            lines.append(f"- **{name} {version}** — " + "; ".join(holders))

    lines += ["", "## Licence texts", ""]
    for key in sorted(all_texts):
        lines += [f"### {key}", "", "```", all_texts[key].strip(), "```", ""]

    OUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(pkgs)} crates + {len(VENDORED)} vendored assets, "
          f"{len(by_licence)} distinct licence expressions")
    for expr, members in sorted(by_licence.items(), key=lambda kv: -len(kv[1])):
        print(f"  {len(members):4}  {expr}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
