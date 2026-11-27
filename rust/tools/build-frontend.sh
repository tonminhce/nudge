#!/bin/sh
set -eu
cd "$(dirname "$0")/../crates/front-app"
trunk build --release
mkdir -p dist/fonts
cp assets/fonts/inter-400.woff2 assets/fonts/inter-500.woff2 assets/fonts/inter-600.woff2 dist/fonts/
