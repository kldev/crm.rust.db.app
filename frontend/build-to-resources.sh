#!/usr/bin/env bash
# Builds the React app and copies dist/ into server/static, the directory the binary serves (CRM_STATIC_DIR,
# default `static`) when started with `cargo run` from server/, so the website comes from the same origin as the
# API (session and CSRF cookies just work). The Docker image runs the same build in its own stage.
# The copied files are git-ignored: the source of truth is frontend/.
set -euo pipefail
cd "$(dirname "$0")"

target=../server/static

corepack enable >/dev/null 2>&1 || true
yarn install --immutable
yarn build

rm -rf "$target"
mkdir -p "$target"
cp -R dist/. "$target/"
echo "Copied $(find "$target" -type f | wc -l) files to server/static"
