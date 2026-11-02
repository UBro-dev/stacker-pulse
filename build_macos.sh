#!/usr/bin/env bash
# Builds a universal (Apple Silicon + Intel) .app and .dmg on a Mac.
# The result is ad-hoc signed, not notarised: see README, "Installing".
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"

# rustc embeds the absolute path of every source file in panic messages;
# remap them so the binaries do not carry /Users/<your name>/... inside.
SEP=$'\x1f'
FLAGS="--remap-path-prefix=$HOME=~"
# A CARGO_HOME outside $HOME holds the crate sources too.
if [ -n "${CARGO_HOME:-}" ]; then FLAGS+="${SEP}--remap-path-prefix=$CARGO_HOME=~/.cargo"; fi
# The project last: when two prefixes match, rustc applies the last one.
FLAGS+="${SEP}--remap-path-prefix=$ROOT=."
export CARGO_ENCODED_RUSTFLAGS="$FLAGS"

cd "$ROOT/src-tauri"
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo tauri build --target universal-apple-darwin

# The command-line scanner is opt-in and ships separately: build both
# architectures and merge them, as CI does.
for t in aarch64-apple-darwin x86_64-apple-darwin; do
  cargo build --release --features cli --bin seedscan-cli --target "$t"
done
CLI=target/universal-apple-darwin/release/seedscan-cli
lipo -create -output "$CLI" \
  target/aarch64-apple-darwin/release/seedscan-cli \
  target/x86_64-apple-darwin/release/seedscan-cli

for p in "$HOME" ${CARGO_HOME:+"$CARGO_HOME"}; do
  if grep -aqF "$p" target/universal-apple-darwin/release/stacker-pulse "$CLI"; then
    echo "The build still contains $p - do not distribute it." >&2; exit 1
  fi
done
echo "Done: src-tauri/target/universal-apple-darwin/release/bundle/ and $CLI (no home-folder paths inside)"
