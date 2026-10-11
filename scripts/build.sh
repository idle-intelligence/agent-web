#!/usr/bin/env bash
# Builds the agent-wasm package, checks the built wasm for local paths, and
# copies the package to _site/pkg.
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

# SIMD128 comes from .cargo/config.toml. The remap flag (keeps the home
# directory out of compiled-in source paths) is passed with --config, which
# cargo merges with that file's rustflags; RUSTFLAGS would replace them and
# drop +simd128.
REMAP_CONFIG="target.wasm32-unknown-unknown.rustflags=[\"--remap-path-prefix=$HOME=/home\"]"
echo "==> Building agent-wasm (web feature) for ENGINE_BUILD=$ENGINE_BUILD"
wasm-pack build crates/agent-wasm --target web --release --config "$REMAP_CONFIG"

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

echo "==> Wrote _site/pkg"
