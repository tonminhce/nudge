# Port map: Twenty CRM -> Rust (axum + Postgres) + Leptos UI + Tauri 2

Reference (read-only): `C:\Users\tonminh123\Documents\crm-research\twenty`
Decisions: [DECISIONS.md](DECISIONS.md) (ADR-001..009). Research: [research/](research/).
Handoff: [HANDOFF.md](HANDOFF.md). Glossary: [GLOSSARY.md](GLOSSARY.md). Rules: [RULES.md](RULES.md).
Feature status: [FEATURE-PARITY.md](FEATURE-PARITY.md).

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
Postgres advisory lock, the bearer auth context dispatch, and session cookies: `sess_` +
base64url tokens stored as `sha256` hex in `core.userSession`, the
`x-twenty-session-cookie` opt-in plus `Sec-Fetch-Mode: navigate` access rules, the
`__Host-twenty-session` / `twenty-session` cookie pair with the upstream attribute set, the
Redis session cache (60 s TTL) with the idle-touch logic, superseded-session replacement on
sign-in, the renewal bridge reuse, the CSRF origin middleware (fail closed, `CSRF_ORIGIN_MISMATCH`),
session-token-as-Bearer rejection, and the daily 03:00 cleanup of sessions and refresh tokens
older than 30 days.

Verification: 34 unit tests (JWT vectors signed by Node, cookie and origin rules, payload shapes,
see `tools/generate-jwt-test-vectors.mjs`) and `tools/auth-e2e.sh` with 48 checks, passing both
with the default 1 minute reuse grace and with it set to 0. The signed-out refresh token stays
renewable inside the grace window, matching upstream
(`is-revoked-refresh-token-still-renewable.util.ts`: a recorded reason blocks renewal, sign-out
sets none).

Still to do in wave 2: SSO (Google, Microsoft, OIDC, SAML), 2FA enrolment and the OTP step,
API keys, the permission checks in the query runner, invitation flows and the email verification
and password reset tokens, impersonation, and the workspace-creation-without-sign-up route.

## Open questions
- None blocking wave 2.

## Out of scope until the user says otherwise
- Reusing twenty-front React code; marketing website (twenty-website); docs site.

## Units
Claims are recorded here: `open`, `claimed by <session> <date>`, or `done` with gate/replay results.

### wave2/account-tokens (done 2026-11-26)
Email verification, password reset and workspace invitations, in
`crates/server/src/auth/account_tokens.rs`.
- Token storage per type (all `core.appToken`): `EMAIL_VERIFICATION_TOKEN` sha256-hashed,
  1 h, row deleted on verify; `PASSWORD_RESET_TOKEN` sha256-hashed, 5 min, one live per user;
  `INVITATION_TOKEN` / `ONBOARDING_INVITATION_TOKEN` verbatim, 30 d. Plain tokens are
  `randomBytes(32).hex`.
- Reset-link throttle 3 per 15 min per email (atomic Redis), unknown emails silently ignored;
  invitation throttles 10/7 d per email, 500/7 d per workspace, 20 + 50/1 d per sender.
- Password change revokes all refresh tokens and all sessions with `PASSWORD_CHANGED`
  (`session.revoke_all_sessions_for_user`).
- Gate green; e2e fragments in `tools/e2e/account-tokens.inc.sh`.

### wave2/two-factor (done 2026-11-26)
TOTP enrolment, backup codes, admin recovery codes and the OTP login step, in
`crates/server/src/auth/two_factor.rs`.
- TOTP sha1/6 digits/30 s, window ±1, secrets ascii-masked then base32 (otplib quirk preserved),
  stored as `enc:v2:` workspace-scoped envelopes; otpauth URI byte-identical to otplib 12.0.1.
- OTP replay guard 120 s; PENDING→VERIFIED under the user advisory lock; 10 backup codes
  (`5-5-5-5`, sha256-hashed) issued at enrolment; admin recovery codes 1 h, one pending per
  (userId, workspaceId); redemption consumes backup code first, else full 2FA reset with
  `TWO_FACTOR_AUTHENTICATION_RESET` revocations.
- The login token is consumed only AFTER the OTP passes (`service.rs` fix, upstream
  `auth.resolver.ts:944-946`).
- Vectors: RFC 6238 plus Node cross-check (`tools/generate-totp-test-vectors.mjs`).
- Gate green; e2e fragments in `tools/e2e/two-factor.inc.sh`.

### wave2/workspace-lifecycle (done 2026-11-26)
Workspace creation for an existing user plus impersonation, in
`crates/server/src/auth/workspace_lifecycle.rs` (additive helpers in `session.rs`).
- `POST /auth/workspace` mirrors the upstream `signUpInNewWorkspace` mutation
  (`engine/core-modules/auth/auth.resolver.ts:740` → `sign-in-up.service.ts`
  `signUpOnNewWorkspace:691`): workspace row in `PENDING_CREATION`, the `Custom`
  workspace application (`application.service.ts:572`), the `userWorkspace` row, then a
  LOGIN token. Response `{ loginToken, workspace: { id } }`.
- `POST /auth/workspace/activate` mirrors `activateWorkspace`
  (`workspace.resolver.ts:164` → `workspace.service.ts:689`): the
  `PENDING_CREATION → ONGOING_CREATION` lock with the 5 minute stale reclaim, the
  `workspace_<base36>` schema, the Admin/Member/Workspace-Custom-Application roles with the
  fixed per-workspace universal identifiers (`core.role` uniqueness is
  `(workspaceId, "universalIdentifier")`, so the same identifiers are reused for every
  workspace; `standard-role.constant.ts`, `twenty-shared MemberRoleUniversalIdentifier`),
  the Admin roleTarget assignment from `setupDefaultRoles`
  (`workspace-manager.service.ts:116`), `defaultRoleId` = Member, `CREATED`.
- `POST /auth/impersonate` mirrors the `impersonate` mutation (`impersonation.resolver.ts`):
  the standard-session guard, the `canImpersonate OR IMPERSONATE-flag` check
  (`impersonate-permission.guard.ts`, role-wide access via `canUpdateAllSettings` in
  `has-role-wide-access-to-permission-flag.util.ts`), the self-impersonation rejection and
  the server/workspace level authorization (`impersonation-authorization.service.ts`).
  Issues an impersonation LOGIN token naming the impersonator `userWorkspaceId`.
- `POST /auth/impersonation/tokens` is the impersonation branch of upstream
  `getAuthTokensFromLoginToken` (`auth.resolver.ts:897-970`) as its own REST route because
  the shared `/auth/tokens` handler is outside this unit: re-validation, single-use
  consumption, the 1 day refresh token (`refresh-token.service.ts:113`), the impersonation
  access claims (`access-token.service.ts:130`), then the session cookie with the
  impersonator parking from `session.issue_session_for_token_pair`.
- `POST /auth/stop-impersonation` mirrors `stopImpersonation`
  (`impersonation.service.ts`): revokes the presented session with
  `IMPERSONATION_ENDED`, restores the parked impersonator cookie session only when it is a
  standard ACCESS payload for the impersonator `userWorkspaceId`, always clears the
  impersonator cookies, returns `{ canRestoreImpersonatorSession }`.
- Deviations: `SERVER_ADMIN_WORKSPACE_ID` is not in the port config yet, so the
  server-admin workspace gate treats it as unset (allowed, the upstream default); the
  email-derived subdomain skips the free-mail provider list (not ported).
- The success-path impersonation e2e landed in `tools/e2e/impersonation-success.inc.sh`
  after `service.rs` gained the first-user `shouldGrantServerAdmin` grant: server admin
  lookup, cross-workspace impersonation, token exchange with the parked impersonator
  cookie, stop restoring the admin session and clearing the impersonator cookies.
- Gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test -p nudge-server -p nudge-core` all green; the combined suite passes in both
  reuse-grace modes.

### wave2/sso (done 2026-11-26)
SSO (Google, Microsoft, OIDC PKCE S256, SAML) — done (wave2/sso, 2026-11-26). Routes
`/auth/google`, `/auth/google/redirect`, `/auth/microsoft`, `/auth/microsoft/redirect`,
`/auth/oidc/login/{id}`, `/auth/oidc/callback`, `/auth/saml/metadata/{id}`, `/auth/saml/login/{id}`,
`/auth/saml/callback/{id}`, plus the REST mirror `POST /auth/sso/exchange-token` of the upstream
`getAuthTokensFromSSOExchangeToken` mutation. Gate: cargo fmt --check, clippy -D warnings clean,
76 unit tests, e2e 99 checks (0 failures) without IdP credentials and 104 checks (0 failures)
with AUTH_GOOGLE_* set. SAML signature verification is a flagged gap (ADR-013); OIDC verifies
RS256 only (ADR-012); the SSO billing entitlement gate is not enforced (no billing module).
