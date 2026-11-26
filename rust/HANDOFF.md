# Handoff

Written for the next AI session. Read this, then MAP.md, then RULES.md before touching anything.

## MANDATORY: work through rust-anything
The next session MUST load the `rust-anything` skill and continue the whole-app flow it defines
(`whole-app.md`): claim a unit in MAP.md, follow its steps per unit (bridge/record where they
apply, and for the app as a whole `app record` -> `app front-door` -> port -> `app wire` ->
`app replay` -> `app lift`). Never mark a unit done without its gate passing. Read `RULES.md`
first; obey R-001 (no comments in code) everywhere.

## Status after this session (2026-11-26)
- Wave 0 done: five research documents in `research/` (data model, GraphQL/REST, auth,
  workers/workflows, front-end inventory).
- Wave 1 done and verified: the Rust server boots, migrates the `core` schema, and
  `tools/verify-core-schema.sh` proves schema parity with the live Twenty 2.61.0 reference
  (columns 1207/1207, enums 57/57, indexes 368/368, constraints 389/389, functions 390/390).
- **Next unit: wave 2, auth.** Then wave 3 (workspace schemas + standard objects), wave 4
  (GraphQL + REST), wave 5 (Leptos UI + Tauri).

## How to run everything
```
docker compose -f rust/infra/docker-compose.yml up -d          # Postgres 16 :5432, Redis 7 :6379
cd rust
export PG_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/default
export REDIS_URL=redis://127.0.0.1:6379
export APP_SECRET=dev-secret-change-me
cargo run -p twenty-server                                     # http://127.0.0.1:3000
curl http://127.0.0.1:3000/healthz
```
Reference instance (Twenty 2.61.0 Node server, user approved):
container `twenty-ref` on port 3001, database `twenty_ref`, started with:
```
docker run -d --name twenty-ref --network infra_default -p 3001:3001 \
  -e NODE_PORT=3001 \
  -e PG_DATABASE_URL="postgres://postgres:postgres@infra-db-1:5432/twenty_ref" \
  -e REDIS_URL="redis://infra-redis-1:6379/1" -e REDIS_CACHE_PREFIX=twentyref \
  -e SERVER_URL="http://localhost:3001" -e FRONTEND_URL="http://localhost:3001" \
  -e PUBLIC_DOMAIN_URL="http://localhost:3001" \
  -e SERVER_ID="8f7d1e2c-3b4a-4c5d-9e6f-7a8b9c0d1e2f" \
  -e APP_SECRET="reference-only-secret-not-a-real-credential" \
  -e DISABLE_CRON_JOBS_REGISTRATION=true -e EMAIL_DRIVER=LOGGER -e STORAGE_TYPE=LOCAL \
  twentycrm/twenty:v2.61.0
```

## Regenerating the core baseline (only if Twenty is re-pinned to a new version)
```
docker exec infra-db-1 pg_dump -U postgres -d twenty_ref --schema-only --schema=core \
  --no-owner --no-privileges > rust/baseline/core-2.61.0.sql
rust/tools/extract-core-baseline.sh
cd rust && docker exec infra-db-1 psql -U postgres -d default \
  -c "DROP SCHEMA IF EXISTS core CASCADE; DROP TABLE IF EXISTS public._sqlx_migrations;" \
  && cargo run -p twenty-server
rust/tools/verify-core-schema.sh
```

## Hard rules (see RULES.md)
- R-001: no comments in code (Rust, SQL, shell). Prose goes in the .md docs instead.
- Migrations are append-only once they have run; never rewrite one, add a new file.
- Keep every env var name identical to the Node server's (`research/auth-permissions.md` section 4).

## Wave 2 (auth) reading list
1. `research/auth-permissions.md` in full; section 6 is the bit-compatibility checklist
   (bcrypt cost 10, ES256 P-256 with rotating `core.signingKey`, legacy HS256
   `sha256(APP_SECRET‖body‖type)`, `enc:v2:` AES-256-GCM + HKDF envelopes, token shapes,
   invitation tokens stored verbatim in `core.appToken.value`).
2. `research/graphql-rest-api.md` section 1 for the `/auth` routes and middleware order.
3. Endpoints to port: sign-up/sign-in, login-token two-step flow, refresh rotation
   (pg advisory lock + 1m reuse grace), verify/reset password, workspace creation
   (needs a slice of wave 3: schema creation + default roles).

## Known traps
- A failed sqlx migration is recorded and blocks reruns while developing; reset with
  `DROP SCHEMA core CASCADE; DROP TABLE public._sqlx_migrations;` (dev databases only).
- `pg_get_constraintdef` output does not round-trip; the baseline migration carries the original
  source SQL for `CHK_CMD_MENU_ITEM_ENGINE_KEY_COHERENCE` via a fixup file. If regenerating the
  baseline, keep `tools/fixups/` in sync.
- Do not run `docker rm -f` on containers you did not create; the permission system blocks it.

## Git
- Repo root: `C:\Users\tonminh123\Downloads\rust-crm`, remote `origin` = `https://github.com/tonminhce/nudge.git`.
- `rust/target/` is git-ignored; `rust/baseline/` dumps are committed on purpose.

## Skill to load next
`rust-anything` (this session's skill), then follow `whole-app.md` from the map.
