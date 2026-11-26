# Port map: Twenty CRM -> Rust (axum + Postgres) + Leptos UI + Tauri 2

Reference (read-only): `C:\Users\tonminh123\Documents\crm-research\twenty`
Decisions: [DECISIONS.md](DECISIONS.md) (ADR-001..009). Research: [research/](research/).
Handoff: [HANDOFF.md](HANDOFF.md). Glossary: [GLOSSARY.md](GLOSSARY.md). Rules: [RULES.md](RULES.md).

Goal: maximum parity, Postgres + GraphQL + DB compatible with Twenty, Leptos UI in a Tauri 2 shell.

Pinned to Twenty **2.61.0** (`srv/src/engine/core-modules/upgrade/constants/twenty-current-version.constant.ts:13`).

## Toolchain state (2026-11-26)
- rustc/cargo 1.98.1 (msvc), wasm32-unknown-unknown added. Docker 29.8.2 present.
- Not installed yet: `trunk`, `cargo-tauri`.
- Postgres 16 + Redis 7 in Docker (`infra/docker-compose.yml`, network `infra_default`).
- Reference database `nudge_ref` holds the migrated 2.61.0 schema; its container (`twenty-ref`,
  image `twentycrm/twenty:v2.61.0`) is stopped now that the baseline dump is committed.

## Waves (each wave must build, pass tests, run end to end)
| Wave | Content | State |
|------|---------|-------|
| 0 | Grilling, ADRs, research (data model, API, auth, workers, UI) | **done** — 5 research docs in `research/` |
| 1 | Server foundation: config, Postgres/Redis, core-schema migrations, health | **done** — schema parity verified against the live 2.61.0 reference |
| 2 | Auth: sign-up/in, JWT (ES256), workspaces, members, roles/permissions, API keys | **in progress** — passwords, ES256 signing, login tokens, refresh rotation done; SSO, 2FA enrolment, API keys, sessions pending |
| 3 | Metadata engine: standard objects seeding, workspace schema creation | open |
| 4 | Workspace GraphQL (dynamic schema) + REST + query runner (filter/sort/paginate/groupBy/aggregate) | open |
| 5 | Leptos UI shell + auth + record table/kanban/show page + settings, wrapped in Tauri | open |
| 6 | Webhooks, files, email (SMTP), search, import/export, timeline, favorites, notes/tasks | open |
| 7 | Workers/queues (own queue, not BullMQ — ADR-007), workflows, messaging + calendar sync, AI agents, MCP, billing | open |

## Wave 1 result (verified)
- `crates/core` (`nudge-core`): config from the environment (upstream-compatible variable names), error type,
  `uuid_to_base36` / `workspace_schema_name`.
- `crates/server` (`nudge-server`): axum app, `GET /healthz`, sqlx pool (`search_path = core, public`),
  Redis client, sqlx migrations for the `core` schema.
- Migrations: `20260101000000_core_bootstrap.sql` (schemas, extensions, `public.unaccent_immutable`)
  and `20260101000100_core_baseline.sql`, generated from the `pg_dump` of the live reference by
  `tools/extract-core-baseline.sh`, with one fixup
  (`tools/fixups/command-menu-item-engine-key-coherence.check.sql`).
- `tools/verify-core-schema.sh` compares the Rust-migrated database with the reference database:
  columns 1207/1207, enums 57/57, indexes 368/368, constraints 389/389, functions 390/390, all
  **0 differences**. The only deliberate omissions are the TypeORM bookkeeping tables
  (`core._typeorm_migrations`, `core._typeorm_generated_columns_and_materialized_views`).
- `cargo test` 4/4, `cargo clippy --all-targets -- -D warnings` clean, `cargo fmt --check` clean,
  `/healthz` returns 200 with database and redis up.

### The core schema baseline, in numbers (2.61.0)
80 tables, 57 enum types, 368 indexes, 389 constraints in `core`, plus the extensions
`uuid-ossp`, `unaccent`, `citext` and the function `public.unaccent_immutable(text)` in `public`.

## Traps found and handled (details in DECISIONS.md and research/data-model.md)
- The TypeORM entities are not the physical schema: `isCustom` and `standardOverrides` are dropped
  from the metadata tables; `rolePermissionFlag.flag` is dropped; `favorite` does not exist
  (favourites are `core.navigationMenuItem` rows).
- Enum type names are all-lowercase (`core."view_calendarlayout_enum"`), not camelCase.
- `pg_get_constraintdef` does not round-trip: `x NOT IN (a,b,c)` deparses as `x <> ALL (ARRAY[...])`
  and re-parsing stores a different expression tree. One fixup restores the original statement.
- `public.unaccent_immutable` must be `IMMUTABLE` (not STRICT/PARALLEL SAFE) with the parameter
  named `input`, and the body indented, to match the original exactly.
- Extensions must live in `public` (the pool sets `search_path = core, public`).
- `core.workspace` <-> `core.application` form a cycle; the workspace FK is
  `DEFERRABLE INITIALLY DEFERRED`.
- `sqlx::migrate!` creates `_sqlx_migrations` unqualified: with the pool's `search_path` set to
  `core, public` the table lands in whichever schema exists first and a later start re-runs
  applied migrations. Migrations run on a dedicated connection pinned to `search_path public`.
- `core."twoFactorAuthenticationMethod"` is keyed by `userWorkspaceId`, not `userId`.
- The legacy HS256 secret is `sha256(APP_SECRET ‖ body ‖ type)` with **no separator** between parts.
- A rotated refresh token stays usable for `REFRESH_TOKEN_REUSE_GRACE_PERIOD` (1 minute).

## Wave 2 progress (auth)
Done and verified: the `enc:v2:` envelope (AES-256-GCM, HKDF-SHA256, key id from the first 8 hex
of `sha256(rawKey)`), ES256 signing with a rotating `core.signingKey` (PKCS#8/SPKI PEM, `kid`
header), the legacy HS256 verification path, bcrypt cost 10 with the `^.{8,50}$` policy, sign-up
(workspace + Admin/Member roles with the fixed universal identifiers + workspace schema), the
two-step sign-in through a single-use login token held in Redis, refresh rotation under a shared
Postgres advisory lock, and the bearer auth context dispatch.

Verification: 16 unit tests (including JWT vectors signed by Node, see
`tools/generate-jwt-test-vectors.mjs`) and `tools/auth-e2e.sh` with 23 checks, passing both with
the default 1 minute reuse grace and with it set to 0.

Still to do in wave 2: SSO (Google, Microsoft, OIDC, SAML), 2FA enrolment and the OTP step,
session cookies, API keys, the permission checks in the query runner, invitation flows and the
email verification and password reset tokens.

## Open questions
- None blocking wave 2.

## Out of scope until the user says otherwise
- Reusing twenty-front React code; marketing website (twenty-website); docs site.

## Units
Claims are recorded here: `open`, `claimed by <session> <date>`, or `done` with gate/replay results.
