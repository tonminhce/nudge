# Handoff

Written for the next AI session. Read this, then MAP.md, then RULES.md before touching anything.

## MANDATORY: work through rust-anything
The next session MUST load the `rust-anything` skill and continue the whole-app flow it defines
(`whole-app.md`): claim a unit in MAP.md, follow its steps per unit, and never mark a unit done
without its gate passing. Read `RULES.md` first; obey R-001 (no comments in code) everywhere and
R-002 (verify auth behaviour against upstream, never guess).

## Status after this session (2026-11-26)
- Wave 0 done: research documents in `research/` (data model, GraphQL/REST, auth,
  workers/workflows, front-end inventory, workspace migrations).
- Wave 1 done and verified: server boots, migrates the `core` schema,
  `tools/verify-core-schema.sh` proves parity with the live reference.
- Wave 2 done and verified end to end (297 e2e checks, 0 failures, in both refresh-grace
  modes; 312 unit tests):
  - password auth, ES256 signing with rotating keys, the `enc:v2:` envelope, the two-step
    sign-in, refresh rotation with the reuse grace, bearer dispatch (`auth/` core files);
  - session cookies: `sess_` tokens, sha256 storage, opt-in + navigation access, CSRF origin
    middleware (fail closed), Redis session cache with idle touch, superseded replacement,
    renewal bridge, impersonator parking, daily 03:00 cleanup (`session.rs`, `csrf.rs`);
  - API keys with roleTargets and live revocation checks (`api_keys.rs`);
  - 2FA: otplib-exact TOTP (Node cross-checked vectors), workspace-scoped secret envelopes,
    OTP replay guard, backup codes, admin recovery codes, login token consumed only after the
    OTP passes (`two_factor.rs`);
  - email verification, password reset (3/15 min throttle) and workspace invitations with
    upstream per-type storage rules (`account_tokens.rs`);
  - second-workspace creation + activation with per-workspace role seeding, impersonation
    start/exchange/stop with cookie parking and restore (`workspace_lifecycle.rs`);
  - the first user receives `canImpersonate` + `canAccessFullAdminPanel`
    (`shouldGrantServerAdmin`, in `service.rs`);
  - SSO: Google/Microsoft OAuth, OIDC PKCE S256 with real RS256 JWKS verification, SAML flows
    with signature verification flag-gated off (ADR-013) (`sso.rs` + `sso/`);
  - CI (fmt, clippy, tests, e2e against Postgres/Redis services in both grace modes), the
    production Dockerfile and `docker-compose.prod.yml` (image build + healthz smoke tested).
- **Wave 3 mostly done** (U1–U8 merged; U9 + U10 on their branches):
  - U1/U2 naming+ids and DDL primitives; U3/U5 flat maps, the universal action model,
    builders, aggregate and ordering; U4 column defs + tsvector (person expression byte-equal).
  - U6 runner: single transaction with `SET LOCAL lock_timeout`, per-action dispatch, guards,
    cache invalidation, `metadataVersion` bump, rollback of rows AND DDL (DB-gated tests).
  - U7 standard objects: `standard_objects/definitions.rs` (generated from the 2.61.0 dumps:
    37 objects / 636 fields / 123 indexes / 34 search fields / 65 options via
    `tools/generate-standard-objects.py`), `permissions.rs` (the reference `accessGroup` /
    `recordShare` storage tables), enrichment on every metadata table
    (`_accessGroupIds`, `_isPrivate` + the two derived indexes), preallocated ids so
    search-field rows insert first against the deferrable core FKs; seeded from
    `/auth/workspace/activate` (idempotent, retryable). DB-gated parity test compares the
    seeded workspace catalog against the live Node reference; `tools/metadata-parity.sh`
    compares seeded metadata content (all five tables, zero-diff on the reference).
  - U7 physical parity: the generator orders each object's fields by the attnum of the field's
    first physical column in `baseline/workspace-schema-2.61.0.sql` (not by name), and
    `object_create_action` carries a per-table `column_plan` that `apply_object_create` uses to
    sort the CREATE-TABLE column definitions into the exact upstream order (base columns, the
    six tables whose `createdAt`/`updatedAt`/`deletedAt` sit mid-table, the access pair, and the
    trailing relation join columns). `tools/verify-standard-objects-attnum.py` re-checks all 37
    standard tables offline against the baseline dump; `tools/prove-attnum-parity.sh` diffs the
    full 39-table workspace schema (ordered `attname` per table) of a live seeded database
    against the reference database. The DB-gated parity test now compares ordered attnum column
    vectors, so a reordering can no longer hide behind the old set comparison. Known residual:
    `core.objectMetadata.targetTableName` stores the computed name for the 37 standard objects,
    whereas the 2.61.0 reference stores the literal `DEPRECATED`; this is the `wave3/u10` fix
    and is independent of column order (all other four parity sections are zero-diff).
  - U8 deferred queue: claim with `FOR UPDATE SKIP LOCKED`, delete-on-success,
    `30s * 2^n` retry backoff, FAILED at `MAX_ATTEMPTS=3`, stale recovery,
    non-transactional execution for `CREATE INDEX CONCURRENTLY`;
    `workspace_migrations::worker::spawn_deferred_worker` runs from `main.rs`.
  - Next: merge `wave3/u9` (metadata GraphQL API) and `wave3/u10` (update/delete matrix,
    includes the `targetTableName='DEPRECATED'` parity fix), then wave 4 (record query
    layer — spec in `research/query-layer.md`) and wave 5 (Leptos front — spec in
    `research/frontend-architecture.md`).

## How to run everything
```
docker compose -f rust/infra/docker-compose.yml up -d          # Postgres 16 :5432, Redis 7 :6379
cd rust
export PG_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/default
export REDIS_URL=redis://127.0.0.1:6379
export APP_SECRET=dev-secret-change-me
cargo run -p nudge-server                                      # http://127.0.0.1:3000
curl http://127.0.0.1:3000/healthz
```
If ports 5432/6379 are taken by another stack, run the compose with a port override file
(`ports: !override ["5433:5432"]`) and point the env vars at those ports.

## How to run the tests
```
cd rust && cargo test                                          # 312 unit tests
cargo run -p nudge-server &                                    # the server on :3000
rust/tools/auth-e2e.sh                                         # 297 end-to-end checks
EXPECT_ROTATED_REUSE=401 rust/tools/auth-e2e.sh                # against REFRESH_TOKEN_REUSE_GRACE_PERIOD=0s
```
The DB-gated suites need a scratch Postgres: `docker exec infra-db-1 createdb -U postgres
<name>` then run with `NUDGE_TEST_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5433/<name>
NUDGE_REFERENCE_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5433/twenty_ref cargo test
-p nudge-server` (the U7 parity test skips unless both env vars are set). The e2e harness reads
the metadata tables directly with `at_db`; it defaults to `-d default` but honors
`E2E_DATABASE_NAME` so both the server and the harness can target another database (e.g.
`e2e_u7`). The script sources every `tools/e2e/*.inc.sh` fragment after the core checks. The
cookie and impersonation fragments hardcode `origin: http://127.0.0.1:3000` while
workspace-lifecycle uses `http://localhost:3000`, so start the server with
`SERVER_URL=http://localhost:3000` and hit it on `127.0.0.1:3000` — the CSRF check allows both
(the request base origin plus the derived allowlist) and the check is origin-strict by design.

## Comparing against the live Node reference
The reference container `twenty-ref` (`twentycrm/twenty:v2.61.0`, port 3001, database
`twenty_ref`) runs the REAL product. Seed one workspace with `python3 -I /tmp/seed-ref.py`
(recreate the container per MAP.md's toolchain section first if it was removed; sign-up is
gated once a workspace exists in single-workspace mode — the first user is the admin).
- `python3 -I tools/parity-compare.py` — same request set against both servers, normalized
  and diffed. Current scoreboard: `/healthz` (Terminus shape) and the NestJS fallback 404
  body match; `/graphql` + `/metadata` answers are the wave 4 gap; the SPA catch-all is the
  wave 5 gap.
- `python3 -I tools/parity-bench.py` — 8 workers x 250 requests per scenario on this
  10-core machine: healthz rust 2125 rps / p99 7.9ms vs node 1466 rps / p99 73.8ms.
- `rust-anything app record --http` against the reference is wired in
  `rust/app-cases/rust-anything.json` but currently unusable: the tool abandons server starts
  slower than ~30-60s (the Node image boots in 2-5 minutes) even with `--ready-timeout 600`.
  A later session should retry with a start wrapper that blocks until the port answers.
- Wave 3 golden fixtures live in `rust/baseline/`: `workspace-schema-2.61.0.sql` (39 tables,
  the U7 equivalence target), `object-metadata-2.61.0.json` (37 objects),
  `field-metadata-2.61.0.json` (636), `index-metadata-2.61.0.json` (123),
  `field-metadata-option-2.61.0.json`.

## Hard rules (see RULES.md)
- R-001: no comments in code (Rust, SQL, shell). Prose goes in the .md docs instead.
- Migrations are append-only once they have run; never rewrite one, add a new file.
- Keep every env var name identical to the Node server's (`research/auth-permissions.md` section 4).

## Wave 3 (metadata engine) reading list
1. `research/workspace-migrations.md` in full — the action model, the builder pipeline, the
   runner transaction, the metadata-to-DDL rules (column types, composite flattening, enum
   lifecycle, relation FKs, deterministic index names, searchVector), standard object seeding,
   and the metadata API validation.
2. `research/data-model.md` for the physical schema contract.
3. The baseline dump `baseline/core-2.61.0.sql` for the metadata tables
   (`object`, `field`, `index`, `deferredWorkspaceOperation`, ...).

## Wave 3 units (from the research doc, each independently verifiable)
- U1 `metadata::names` + `ids` — pure naming/hash functions with golden vectors.
- U2 `workspace_migrations::ddl` primitives — emitted SQL byte-equal to upstream strings.
- U3 `metadata::flat` + cache — flat maps loaded from the migrated `core` schema.
- U4 `column_defs` + `tsvector` — per-type columns; person tsvector expression byte-equal.
- U5 actions + builders + aggregate + order — pure ordering/relation-pairing tests.
- U6 runner transaction loop — one run creates object+field; failure rolls back rows AND DDL.
- U7 standard-object registry + activation seeding — `pg_dump` of a workspace schema equal to
  a live 2.61.0 reference workspace (the primary equivalence gate).

## Known gaps / deferred (documented, deliberate)
- SAML signature verification: callback returns 403 `SSO_AUTH_FAILED` unless
  `AUTH_SAML_ACCEPT_UNVERIFIED_ASSERTIONS=true` (ADR-013). No sound pure-Rust path; revisit
  samael+xmlsec1 or contribute exclusive c14n upstream.
- OIDC verifies RS256 only; other advertised algorithms fail closed (ADR-012).
- SSO billing entitlement gate awaits the billing module (wave 7).
- Email delivery is LOGGER-only (`tracing::info`), SMTP lands in wave 6.
- Rate limiting beyond the upstream-specific throttles lands in wave 6.
- Workspace-level impersonation needs the IMPERSONATE permission flag, which lands with the
  query runner enforcement (wave 4).

## Known traps
- A failed sqlx migration is recorded and blocks reruns while developing; reset with
  `DROP SCHEMA core CASCADE; DROP TABLE public._sqlx_migrations;` (dev databases only).
- `pg_get_constraintdef` output does not round-trip; keep `tools/fixups/` in sync if the
  baseline is regenerated.
- Do not run `docker rm -f` on containers you did not create.
- The e2e fragments require the server on port 3000 (origin-strict CSRF).

## Git
- Repo root: this repository; remote `origin` = `https://github.com/tonminhce/nudge.git`.
- `rust/target/` and `worktrees/` are git-ignored; `rust/baseline/` dumps are committed on
  purpose. Parallel units are developed in git worktrees on `wave2/*`-style branches, then
  merged one by one with the full gate and e2e re-run.

## Skill to load next
`rust-anything` (this session's skill), then follow `whole-app.md` from the map.
