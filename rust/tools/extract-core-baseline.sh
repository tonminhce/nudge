#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source_dump="${1:-$repo_root/baseline/core-2.61.0.sql}"
target="${2:-$repo_root/crates/server/migrations/core/20260101000100_core_baseline.sql}"
fixup="$repo_root/tools/fixups/command-menu-item-engine-key-coherence.check.sql"

if [[ ! -f "$source_dump" ]]; then
  echo "missing dump: $source_dump" >&2
  exit 1
fi

if [[ ! -f "$fixup" ]]; then
  echo "missing fixup: $fixup" >&2
  exit 1
fi

mkdir -p "$(dirname "$target")"

awk -v fixup="$fixup" '
  /^\\restrict/ { next }
  /^\\unrestrict/ { next }

  /^SELECT pg_catalog\.set_config\(.search_path., .., false\);$/ { next }

  /^CREATE SCHEMA core;$/ { print "CREATE SCHEMA IF NOT EXISTS core;"; next }

  /^-- Name: _typeorm/ { skip = 1; next }
  skip {
    if (/^\);$/) { skip = 0; next }
    if (/;[[:space:]]*$/) { skip = 0; next }
    next
  }
  /_typeorm/ { next }

  /^[[:space:]]*--/ { next }

  /CONSTRAINT "CHK_CMD_MENU_ITEM_ENGINE_KEY_COHERENCE" CHECK \(/ {
    while ((getline line < fixup) > 0) print line
    close(fixup)
    next
  }

  { print }
' "$source_dump" > "$target"

echo "wrote $target ($(wc -l < "$target") lines)"
