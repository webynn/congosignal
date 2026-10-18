#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
mkdir -p data
if [ -d .toolchain/cargo ]; then
  export CARGO_HOME="$PWD/.toolchain/cargo"
  export RUSTUP_HOME="$PWD/.toolchain/rustup"
  export PATH="$CARGO_HOME/bin:$PATH"
fi
export AI_ENABLED="${AI_ENABLED:-1}"
export OLLAMA_MODEL="${OLLAMA_MODEL:-gemma4:12b}"
export POLL_SECONDS="${POLL_SECONDS:-900}"
cargo build --locked
exec target/debug/drc-osint-worker
