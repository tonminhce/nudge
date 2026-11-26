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
- **Wave 2 done** and verified end to end (297 e2e checks, 0 failures, in both refresh-grace
  modes; 130 unit tests):
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
- **Next: wave 3**, the metadata engine. The full plan with citations is
  `research/workspace-migrations.md`: units U1-U10, critical path U1 → U2 → U4 → U6 → U7.

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
cd rust && cargo test                                          # 130 unit tests
cargo run -p nudge-server &                                    # the server on :3000
rust/tools/auth-e2e.sh                                         # 297 end-to-end checks
EXPECT_ROTATED_REUSE=401 rust/tools/auth-e2e.sh                # against REFRESH_TOKEN_REUSE_GRACE_PERIOD=0s
```
The script sources every `tools/e2e/*.inc.sh` fragment after the core checks. The cookie and
impersonation fragments hardcode `origin: http://127.0.0.1:3000`, so run the server on port
3000 (the CSRF origin check is origin-strict by design).

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
