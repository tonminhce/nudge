#!/usr/bin/env bash
set -uo pipefail

CONTAINER="${CONTAINER:-infra-db-1}"
DATABASE="${E2E_DATABASE_NAME:-default}"
REFERENCE="${REFERENCE_DATABASE_NAME:-twenty_ref}"

dump() {
  local db="$1" sql="$2"
  printf '%s' "$sql" | docker exec -i "$CONTAINER" psql -U postgres -d "$db" -tA
}

compare() {
  local label="$1" sql="$2"
  local ref mine diff_count
  ref="$(dump "$REFERENCE" "$sql")"
  mine="$(dump "$DATABASE" "$sql")"
  diff_count="$(diff <(printf '%s\n' "$ref" | sort) <(printf '%s\n' "$mine" | sort) | grep -c '^[<>]')"
  printf '%-22s ref=%-5s mine=%-5s diff=%s\n' "$label" "$(printf '%s\n' "$ref" | grep -c .)" "$(printf '%s\n' "$mine" | grep -c .)" "$diff_count"
  [ "$diff_count" -eq 0 ] || return 1
}

objects="SELECT \"universalIdentifier\"::text||'|'||coalesce(\"nameSingular\",'')||'|'||coalesce(\"namePlural\",'')||'|'||coalesce(\"labelSingular\",'')||'|'||coalesce(\"labelPlural\",'')||'|'||\"isActive\"||'|'||\"isSystem\"||'|'||\"isSearchable\"||'|'||coalesce(\"description\",'')||'|'||coalesce(icon,'')||'|'||coalesce(color,'')||'|'||coalesce(\"targetTableName\",'')||'|'||coalesce(\"isRemote\",false)||'|'||coalesce(\"isAuditLogged\",false)||'|'||coalesce(\"isLabelSyncedWithName\",false) FROM core.\"objectMetadata\" ORDER BY 1"

fields="SELECT f.\"universalIdentifier\"::text||'|'||o.\"universalIdentifier\"::text||'|'||coalesce(f.type,'')||'|'||coalesce(f.name,'')||'|'||coalesce(f.label,'')||'|'||coalesce(f.description,'')||'|'||coalesce(f.icon,'')||'|'||f.\"isActive\"||'|'||f.\"isSystem\"||'|'||f.\"isNullable\"||'|'||f.\"isLabelSyncedWithName\"||'|'||f.\"isUIEditable\"||'|'||f.\"isSystemSideEffect\"||'|'||f.\"isAuditLogged\"||'|'||coalesce(f.\"writability\"::text,'~')||'|'||coalesce(f.\"defaultValue\"::text,'~')||'|'||coalesce(f.options::text,'~')||'|'||coalesce(f.settings::text,'~')||'|'||coalesce(f.overrides::text,'~')||'|'||coalesce(f.\"morphId\"::text,'~')||'|'||coalesce(rf.\"universalIdentifier\"::text,'~')||'|'||coalesce(ro.\"universalIdentifier\"::text,'~') FROM core.\"fieldMetadata\" f LEFT JOIN core.\"objectMetadata\" o ON o.id=f.\"objectMetadataId\" LEFT JOIN core.\"fieldMetadata\" rf ON rf.id=f.\"relationTargetFieldMetadataId\" LEFT JOIN core.\"objectMetadata\" ro ON ro.id=f.\"relationTargetObjectMetadataId\" ORDER BY 1"

options="SELECT o.\"universalIdentifier\"::text||'|'||f.\"universalIdentifier\"::text||'|'||o.value||'|'||o.label||'|'||coalesce(o.color,'~')||'|'||o.position||'|'||o.\"isActive\"||'|'||coalesce(o.overrides::text,'~') FROM core.\"fieldMetadataOption\" o JOIN core.\"fieldMetadata\" f ON f.id=o.\"fieldMetadataId\" ORDER BY 1"

indexes="SELECT i.\"universalIdentifier\"::text||'|'||o.\"universalIdentifier\"::text||'|'||i.name||'|'||i.\"isUnique\"||'|'||i.\"indexType\"::text||'|'||coalesce(i.\"indexWhereClause\",'~')||'|'||i.\"isSystemSideEffect\"||'|'||(SELECT string_agg(f.\"universalIdentifier\"::text||':'||im.\"order\"::text||':'||coalesce(im.\"subFieldName\",'~'), ',' ORDER BY im.\"order\") FROM core.\"indexFieldMetadata\" im JOIN core.\"fieldMetadata\" f ON f.id=im.\"fieldMetadataId\" WHERE im.\"indexMetadataId\"=i.id) FROM core.\"indexMetadata\" i JOIN core.\"objectMetadata\" o ON o.id=i.\"objectMetadataId\" ORDER BY 1"

search_fields="SELECT s.\"universalIdentifier\"::text||'|'||o.\"universalIdentifier\"::text||'|'||f.\"universalIdentifier\"::text||'|'||t.\"universalIdentifier\"::text||'|'||s.position||'|'||s.\"isSystemSideEffect\" FROM core.\"searchFieldMetadata\" s JOIN core.\"objectMetadata\" o ON o.id=s.\"objectMetadataId\" JOIN core.\"fieldMetadata\" f ON f.id=s.\"fieldMetadataId\" JOIN core.\"fieldMetadata\" t ON t.id=s.\"tsVectorFieldMetadataId\" ORDER BY 1"

failed=0
compare objectMetadata "$objects" || failed=1
compare fieldMetadata "$fields" || failed=1
compare fieldMetadataOption "$options" || failed=1
compare indexMetadata "$indexes" || failed=1
compare searchFieldMetadata "$search_fields" || failed=1

if [ "$failed" -ne 0 ]; then
  echo "metadata content parity: FAIL"
  exit 1
fi

echo "metadata content parity: PASS"
