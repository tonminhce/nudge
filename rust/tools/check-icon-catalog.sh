#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

python3 crates/front-ui/tools/gen_icons.py "$tmp/generated.rs"
python3 crates/front-ui/tools/gen_custom_icons.py "$tmp/custom.rs"
rustfmt --edition 2021 "$tmp/generated.rs" "$tmp/custom.rs"
diff -u crates/front-ui/src/icons/generated.rs "$tmp/generated.rs"
diff -u crates/front-ui/src/icons/custom.rs "$tmp/custom.rs"
