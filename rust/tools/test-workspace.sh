#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

container="${NUDGE_TEST_DATABASE_CONTAINER:-infra-db-1}"
database_name="nudge_workspace_test_$$"
database_url="$(
  python3 - "$database_name" <<'PY'
import pathlib
import re
import sys
from urllib.parse import urlsplit, urlunsplit

source = pathlib.Path("tools/run-same-db-server.py").read_text()
url = re.search(r'PG_DATABASE_URL": "([^"]+)"', source).group(1)
parts = urlsplit(url)
print(urlunsplit((parts.scheme, parts.netloc, "/" + sys.argv[1], "", "")))
PY
)"

cleanup() {
  docker exec "$container" dropdb -U postgres --if-exists "$database_name" >/dev/null
}
trap cleanup EXIT

docker exec "$container" createdb -U postgres "$database_name"
printf 'Running workspace tests against isolated database %s\n' "$database_name"
NUDGE_TEST_DATABASE_URL="$database_url" cargo test --workspace -- --test-threads=1
