#!/usr/bin/env bash
# Builds the agent-wasm package and assembles the deployed site into
# _site/, following the pattern of llm-web's scripts/build.sh (build tag
# rewrite, local-path leak check on the built wasm).
#
# The web/ page (phase 2 of this repo) is not built yet; this currently
# just builds and checks the wasm package at _site/pkg. Once web/ exists,
# add its copy and ENGINE_BUILD rewrite here, following llm-web's script.
#
# Requires wasm-pack and wasm-bindgen-cli (version matching Cargo.lock's
# wasm-bindgen exactly: `cargo install wasm-bindgen-cli --version <ver>
# --locked`).
#
# Usage: ENGINE_BUILD=<tag> scripts/build.sh
# ENGINE_BUILD defaults to "dev" for local builds; CI passes the commit sha.
set -euo pipefail

ENGINE_BUILD="${ENGINE_BUILD:-dev}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "==> Building agent-wasm (web feature) for ENGINE_BUILD=$ENGINE_BUILD"
RUSTFLAGS="-C target-feature=+simd128 --remap-path-prefix=$HOME=/home" \
  wasm-pack build crates/agent-wasm --target web --release

WASM="crates/agent-wasm/pkg/agent_wasm_bg.wasm"

LEAKS=$(strings "$WASM" | grep -F -e "$HOME" -e "Code/" -e ".claude/" -e "/Users/" || true)
USER_HITS=$(strings "$WASM" | grep -Fw -e "$(id -un)" || true)
if [ -n "$LEAKS$USER_HITS" ]; then
    echo "error: $WASM contains local paths or the user name:" >&2
    printf '%s\n%s\n' "$LEAKS" "$USER_HITS" | grep -v '^$' | head -20 >&2
    exit 1
fi
echo "==> no local paths in built wasm"

echo "==> Assembling _site"
rm -rf _site
mkdir -p _site/pkg
cp -R crates/agent-wasm/pkg/. _site/pkg/

echo "==> Wrote _site (pkg only; no page yet, see this script's header)"
