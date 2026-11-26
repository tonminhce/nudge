#!/usr/bin/env bash
set -euo pipefail

reference_db="${REFERENCE_DB:-nudge_ref}"
target_db="${TARGET_DB:-default}"
postgres_container="${POSTGRES_CONTAINER:-infra-db-1}"

compare() {
  local name="$1" query="$2"
  local differences

  docker exec "$postgres_container" psql -U postgres -d "$reference_db" -t -A -c "$query" \
    | grep -v '_typeorm' | sort > /tmp/verify-ref.txt
  docker exec "$postgres_container" psql -U postgres -d "$target_db" -t -A -c "$query" \
    | grep -v '_typeorm' | sort > /tmp/verify-target.txt

  differences="$(diff /tmp/verify-ref.txt /tmp/verify-target.txt | grep -cE '^[<>]' || true)"

  printf '%-12s reference=%-5s target=%-5s differences=%s\n' \
    "$name" \
    "$(wc -l < /tmp/verify-ref.txt)" \
    "$(wc -l < /tmp/verify-target.txt)" \
    "$differences"

  if [ "$differences" != "0" ]; then
    diff /tmp/verify-ref.txt /tmp/verify-target.txt | head -20
    return 1
  fi
}

status=0

compare "columns" "SELECT c.relname||'|'||a.attname||'|'||format_type(a.atttypid, a.atttypmod)||'|'||a.attnotnull||'|'||COALESCE(pg_get_expr(d.adbin, d.adrelid), '-')
  FROM pg_attribute a
  JOIN pg_class c ON c.oid = a.attrelid
  JOIN pg_namespace n ON n.oid = c.relnamespace
  LEFT JOIN pg_attrdef d ON d.adrelid = c.oid AND d.adnum = a.attnum
  WHERE n.nspname = 'core' AND c.relkind = 'r' AND a.attnum > 0 AND NOT a.attisdropped
    AND c.relname NOT LIKE '\_typeorm%'" || status=1

compare "enums" "SELECT t.typname||'|'||string_agg(e.enumlabel, ',' ORDER BY e.enumsortorder)
  FROM pg_type t
  JOIN pg_namespace n ON n.oid = t.typnamespace
  JOIN pg_enum e ON e.enumtypid = t.oid
  WHERE n.nspname = 'core'
  GROUP BY t.typname" || status=1

compare "indexes" "SELECT indexname||'|'||indexdef FROM pg_indexes WHERE schemaname = 'core'" || status=1

compare "constraints" "SELECT c.conrelid::regclass||'|'||c.conname||'|'||c.contype::text||'|'||pg_get_constraintdef(c.oid)
  FROM pg_constraint c
  JOIN pg_namespace n ON n.oid = c.connamespace
  WHERE n.nspname = 'core' AND c.conrelid::regclass::text NOT LIKE 'core.\_typeorm%'" || status=1

compare "functions" "SELECT p.proname||'|'||pg_get_functiondef(p.oid)
  FROM pg_proc p
  JOIN pg_namespace n ON n.oid = p.pronamespace
  WHERE n.nspname = 'public' AND p.prokind = 'f'
  ORDER BY 1" || status=1

if [ "$status" = "0" ]; then
  echo "core schema matches the reference"
else
  echo "core schema differs from the reference"
fi

exit "$status"
