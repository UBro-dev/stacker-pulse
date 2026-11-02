#!/usr/bin/env bash
# Builds the Linux packages (.AppImage and .deb).
set -euo pipefail
echo "Building StackerPulse for Linux (.AppImage and .deb)..."
ROOT="$(cd "$(dirname "$0")" && pwd)"

# rustc embeds the absolute path of every source file in panic messages;
# remap them so the binaries do not carry /home/<your name>/... inside.
SEP=$'\x1f'
FLAGS="--remap-path-prefix=$HOME=~"
# A CARGO_HOME outside $HOME holds the crate sources too.
if [ -n "${CARGO_HOME:-}" ]; then FLAGS+="${SEP}--remap-path-prefix=$CARGO_HOME=~/.cargo"; fi
# The project last: when two prefixes match, rustc applies the last one.
FLAGS+="${SEP}--remap-path-prefix=$ROOT=."
export CARGO_ENCODED_RUSTFLAGS="$FLAGS"

cd "$ROOT/src-tauri"
cargo tauri build --bundles appimage,deb
# The command-line scanner is opt-in and ships separately from the packages.
cargo build --release --features cli --bin seedscan-cli

for p in "$HOME" ${CARGO_HOME:+"$CARGO_HOME"}; do
  if grep -aqF "$p" target/release/stacker-pulse target/release/seedscan-cli; then
    echo "The build still contains $p - do not distribute it." >&2; exit 1
  fi
done
echo "Done: src-tauri/target/release/bundle/ and target/release/seedscan-cli (no home-folder paths inside)"
