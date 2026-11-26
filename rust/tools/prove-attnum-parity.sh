#!/usr/bin/env bash
set -uo pipefail

CONTAINER="${CONTAINER:-infra-db-1}"
MINE_DB="${MINE_DB:?set MINE_DB}"
REF_DB="${REF_DB:-twenty_ref}"

pick_sql="SELECT n.nspname FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.relkind = 'r' AND n.nspname LIKE 'workspace_%' GROUP BY n.nspname HAVING count(*) = 39 ORDER BY min(c.oid) LIMIT 1"
MINE_SCHEMA="$(printf '%s' "$pick_sql" | docker exec -i "$CONTAINER" psql -U postgres -d "$MINE_DB" -tA)"
[ -n "$MINE_SCHEMA" ] || { echo "no 39-table workspace schema found in $MINE_DB"; exit 1; }
REF_SCHEMA="$(printf '%s' "$pick_sql" | docker exec -i "$CONTAINER" psql -U postgres -d "$REF_DB" -tA)"
[ -n "$REF_SCHEMA" ] || { echo "no 39-table workspace schema found in $REF_DB"; exit 1; }

dump_sql() {
  local schema="$1"
  cat <<EOF
WITH t AS (
  SELECT c.relname AS table_name,
         string_agg(a.attname::text, ',' ORDER BY a.attnum) AS cols
  FROM pg_attribute a
  JOIN pg_class c ON c.oid = a.attrelid
  JOIN pg_namespace n ON n.oid = c.relnamespace
  WHERE n.nspname = '$schema' AND c.relkind = 'r' AND a.attnum > 0 AND NOT a.attisdropped
  GROUP BY c.relname
)
SELECT table_name || '|' || cols FROM t ORDER BY table_name
EOF
}

mine="$(dump_sql "$MINE_SCHEMA" | docker exec -i "$CONTAINER" psql -U postgres -d "$MINE_DB" -tA)"
theirs="$(dump_sql "$REF_SCHEMA" | docker exec -i "$CONTAINER" psql -U postgres -d "$REF_DB" -tA)"

mine_n="$(printf '%s\n' "$mine" | grep -c .)"
theirs_n="$(printf '%s\n' "$theirs" | grep -c .)"

same=0
diff=0
difflist=""
while IFS= read -r line; do
  [ -z "$line" ] && continue
  tbl="$(printf '%s' "$line" | cut -d'|' -f1)"
  ref="$(printf '%s\n' "$theirs" | awk -F'|' -v t="$tbl" '$1==t {print; exit}')"
  if [ "$line" = "$ref" ]; then
    same=$((same + 1))
  else
    diff=$((diff + 1))
    difflist="${difflist}${tbl}\n"
  fi
done <<EOF
$mine
EOF

printf 'mine schema=%s ref schema=%s mine tables=%s ref tables=%s identical ordered columns=%s differing=%s\n' "$MINE_SCHEMA" "$REF_SCHEMA" "$mine_n" "$theirs_n" "$same" "$diff"
if [ "$diff" -ne 0 ]; then
  printf 'differing tables:\n%b' "$difflist"
  exit 1
fi
if [ "$same" -ne 39 ]; then
  echo "expected 39 tables, got $same"
  exit 1
fi
echo "PASS 39/39 tables identical ordered columns"
