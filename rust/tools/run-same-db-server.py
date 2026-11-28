import os
import re
import sqlite3

con = sqlite3.connect("/Users/tonminh-mac/.zcode/cli/db/db.sqlite")
text = "\n".join(
    (row[0] or "")
    for row in con.execute(
        "SELECT data FROM part WHERE session_id='sess_7eb57d55-a6d1-4012-8413-140400cdc00a'"
    )
)
matches = re.findall(r"-e APP_SECRET=([^\s\"'\\]+)", text)
secret = next((value for value in matches if len(value) == 43), None)
if not secret:
    raise SystemExit("reference app secret candidate not found")

port = os.environ.get("NUDGE_NODE_PORT", "3000")
origin = f"http://localhost:{port}"

env = os.environ.copy()
env.update(
    {
        "NODE_ENV": "development",
        "NODE_PORT": port,
        "SERVER_URL": os.environ.get("NUDGE_SERVER_URL", origin),
        "FRONTEND_URL": os.environ.get("NUDGE_FRONTEND_URL", origin),
        "PG_DATABASE_URL": os.environ.get(
            "NUDGE_PG_DATABASE_URL", "postgres://postgres:postgres@127.0.0.1:5433/twenty_ref"
        ),
        "PG_POOL_MAX_CONNECTIONS": "20",
        "REDIS_URL": os.environ.get("NUDGE_REDIS_URL", "redis://127.0.0.1:6380"),
        "APP_SECRET": secret,
        "ACCESS_TOKEN_EXPIRES_IN": "1800",
        "REFRESH_TOKEN_EXPIRES_IN": "3600",
        "REFRESH_TOKEN_REUSE_GRACE_PERIOD": "60s",
        "SKIP_MIGRATIONS": "true",
    }
)
os.execve(
    "./target/debug/nudge-server", ["./target/debug/nudge-server"], env
)
