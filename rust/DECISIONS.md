# Decisions (ADRs)

Source of truth for the port. Original (reference only): C:\Users\tonminh123\Documents\crm-research\twenty
Output (this repo): C:\Users\tonminh123\Downloads\rust-crm

## ADR-001: Scope = maximum parity
Status: accepted (2026-11-26)
Context: User chose "Maximum parity" (every feature in the monorepo).
Decision: Target every feature, delivered in waves (see MAP.md). Each wave must build, pass tests and run end to end before the next starts.
Consequences: Multi-session effort (~1.8M LOC reference). Waves are ordered so every wave ships a usable app.

## ADR-002: Architecture = Postgres server + thin Tauri client
Status: accepted (2026-11-26)
Decision: axum server (crate `server`) on PostgreSQL (sqlx), Redis for cache/pubsub/queues; Tauri 2 desktop shell hosts the Leptos UI and talks to the server (URL configurable).
Consequences: Needs Postgres + Redis (docker compose provided). Tauri does not embed the backend.

## ADR-003: UI = Leptos (pure Rust), built with trunk, wrapped by Tauri
Status: accepted (2026-11-26)
Decision: Re-implement Twenty's UX (table, kanban, record page, settings, command menu) in Leptos; no reuse of twenty-front React code.
Consequences: UI parity is the largest cost centre; it is ported by feature area.

## ADR-004: Compatibility = full GraphQL + DB compatible
Status: accepted (2026-11-26)
Decision: Same Postgres layout (core schema + one schema per workspace `workspace_<id>`), same GraphQL schema on /graphql and /metadata (generated at runtime from metadata, as the original does), same REST /rest endpoints, so a live Twenty database and Twenty clients/SDK/Zapier can point at the Rust server.
Consequences: Use async-graphql dynamic schema; migrations must be byte-compatible with the original TypeORM migrations; research docs in rust/research/ pin the exact contracts.

## ADR-005: Equivalence bar
Status: accepted (2026-11-26)
Decision: GraphQL responses equal after JSON normalization (key order, float formatting); error `extensions.code` and HTTP status equal, message text best effort. Recorded cases replayed against both servers over the same database.

## ADR-006: The `core` schema is reproduced by versioned SQL migrations in the Rust server
Status: accepted (2026-11-26)
Context: Twenty's TypeORM entities are not the physical schema: upgrade commands drop columns (`isCustom`, `standardOverrides`), rename columns and change defaults. The end state after 324 instance commands is the contract, and the constraint and index names are TypeORM hashes.
Decision: The Rust server owns the `core` schema through sqlx migrations in `crates/server/migrations/core/`, written against the physical schema of 2.61.0 (see `research/data-model.md`). The authoritative check is a `pg_dump --schema-only` of a migrated 2.61.0 database; the migrations are diffed against it once the `twentycrm/twenty:2.61.0` image is available.
Consequences: A Rust server can create its own database from scratch, and can also attach to a database a Node server migrated, provided the remaining tables are added. Column and constraint names must match the original exactly; the migrations are append-only, never rewritten after they have run.

## ADR-007: The Rust server uses its own job queue instead of BullMQ
Status: accepted (2026-11-26), applies to wave 7
Context: The original runs 29 BullMQ queues on Redis, with two custom Lua subsystems (fair virtual-time scheduling, job-memory admission). Reproducing BullMQ's Redis key layout in Rust would tie the Rust worker to BullMQ internals without gaining anything.
Decision: The Rust server implements its own queue (Redis or Postgres backed, chosen in wave 7) with the same queue names and job payloads. The Node server and the Rust server do not share a queue.
Consequences: A cutover runs one worker or the other, never both against the same Redis. Job payloads stay compatible so the same handlers can be ported.

## ADR-008: Leptos UI, rendered client side, wrapped by Tauri 2
Status: accepted (2026-11-26)
Context: The original front end is 12,700 React files built with Vite. A pure Rust port (ADR-003) re-implements the UX in Leptos and ships it as static assets.
Decision: The UI is a Leptos CSR app built with `trunk`; Tauri 2 serves the built assets and hosts the shell. The UI talks to the server over the same HTTP API any other client uses, so the server stays usable without the desktop app.
Consequences: The desktop app needs the server running. `trunk` and `cargo-tauri` must be installed before wave 5.

## ADR-009: The baseline migration is generated from the live reference dump
Status: accepted (2026-11-26)
Context: Hand-writing 80 tables' DDL produced small but real mismatches (enum type name casing, default spellings, nullability, default values of auth booleans). The user approved pulling and running `twentycrm/twenty:v2.61.0`, whose migration produces the authoritative schema in database `nudge_ref`.
Decision: `crates/server/migrations/core/20260101000100_core_baseline.sql` is generated by `tools/extract-core-baseline.sh` from `baseline/core-2.61.0.sql` (a `pg_dump --schema-only --schema=core` of the reference), minus the TypeORM bookkeeping tables and with one statement fixup under `tools/fixups/` because `pg_get_constraintdef` does not round-trip. `tools/verify-core-schema.sh` proves parity (0 differences on columns, enums, indexes, constraints, functions).
Consequences: The schema cannot drift silently: any change must go through the dump + script, and the verify script must pass. The dump files are committed as evidence. When Twenty is re-pinned to a newer version, redo the dump, regenerate, and re-verify.

## ADR-010: No comments in code
Status: accepted (2026-11-26)
Context: The user set a hard rule banning comments in code.
Decision: No comments in any source the port writes: Rust (including doc comments), SQL and shell. Reasoning lives in the documentation files (DECISIONS.md, MAP.md, GLOSSARY.md, HANDOFF.md, research/). Recorded as rule R-001 in RULES.md.
Consequences: `gate`'s `// INFALLIBLE:` convention is replaced by trusting the recorded cases; code must be self-explanatory; the generated baseline strips the dump's comment lines.



## ADR-011: SSO (Google, Microsoft, OIDC, SAML) lives in `auth/sso/` with a Redis state store
Status: accepted (2026-11-26), wave 2 unit
Context: Upstream implements social sign-in through passport strategies whose per-attempt state (the OIDC PKCE verifier, the SAML relay payload, the identity provider id) rides in express-session backed by Redis under the `engine:session:` prefix with a 30 minute cookie maxAge (`engine/core-modules/session-storage/session-storage.module-factory.ts`).
Decision: The port re-implements the flows directly (`crates/server/src/auth/sso.rs` + `sso/{config,state_store,flow,google,microsoft,oidc,saml}.rs`) without porting express-session. OIDC and SAML attempts store a JSON payload (identity provider id, PKCE code verifier or invite relay data) under `engine:session:sso:<32-byte hex>` with a 1800 s TTL, and the key is consumed atomically with `GETDEL`. Google and Microsoft keep the upstream passport contract: the OAuth `state` parameter carries the JSON state (`{workspaceInviteHash, workspaceId, billingCheckoutSessionState, action, locale, returnToPath}`, plus `oauthRetryCount` for Microsoft) instead of a server-side record. The SSO exchange token matches upstream exactly: opaque `randomBytes(32).hex`, stored as `sha256(hex)` in `core.appToken` type `SSO_EXCHANGE_TOKEN` (partial unique index `IDX_APP_TOKEN_TYPE_VALUE_SSO_EXCHANGE_UNIQUE`), 5 minute expiry, single use via DELETE with `revokedAt/deletedAt IS NULL`, redeemed for a workspace-agnostic token + refresh token (upstream mutation `getAuthTokensFromSSOExchangeToken`, exposed as REST `POST /auth/sso/exchange-token` in line with the port's other REST mirrors of auth mutations).
Consequences: No express-session surface to maintain; attempts expire with the same 30 minute window upstream uses. Redirects for disabled providers and guard failures go to `<frontend>/verify?errorMessage=...` (upstream `guard-redirect` behaviour flattened onto `FRONTEND_URL` because the port has no subdomain routing yet).

## ADR-012: OIDC id-token verification is RS256 only, against a cached JWKS
Status: accepted (2026-11-26), wave 2 unit
Context: Upstream resolves the id-token algorithm from discovery (prefer RS256, else the first advertised signed algorithm, `none` filtered, `sso/utils/resolve-id-token-signing-algorithm.util.ts`) and delegates verification to openid-client.
Decision: The port discovers the issuer (RFC 8414 path-aware `/.well-known/openid-configuration`, exact `issuer` match enforced), caches discovery and JWKS per URL for 10 minutes in-process, respects `kid` (single-key fallback when absent), and verifies RS256 PKCS#1 v1.5 signatures with the `rsa` + `sha2` crates. When discovery advertises only non-RS256 algorithms the sign-in fails with a clear error instead of skipping verification.
Consequences: RS256 covers the overwhelming majority of IdPs; ES256 id tokens are a documented gap. IdP rows and the billing entitlement gate (`SSO` plan) upstream applies through `EnterpriseFeaturesEnabledGuard` are not enforced because billing does not exist in the port yet.

## ADR-013: SAML ships without XML signature verification and is disabled by default
Status: accepted (2026-11-26), wave 2 unit
Context: SAML verification requires XML DSig with exclusive canonicalization. The Rust ecosystem: `samael` binds the C `xmlsec1` library (breaks the workspace's rustls/no-OpenSSL posture), and the pure-Rust crates (`saml-rs`, `saml`) are immature and self-described as not security primitives. 2025 saw multiple signature-bypass CVEs in mature SAML stacks (CVE-2025-47949, CVE-2025-25292), so a hand-rolled canonicalization would be the weakest link in the codebase.
Decision: Everything except signature verification is implemented: SP metadata, AuthnRequest redirect binding (raw DEFLATE + base64), assertion parsing, Conditions window checks, bearer SubjectConfirmationData `NotOnOrAfter` (max wins, upstream zod schema), audience and recipient checks, email extraction (`email` attribute, then `mail`, then NameID), and replay protection in Redis (`saml-assertion:consumed:<idpId>:<assertionId>` with the bearer `NotOnOrAfter` TTL). `POST /auth/saml/callback/:id` returns 403 `SSO_AUTH_FAILED` unless the operator sets `AUTH_SAML_ACCEPT_UNVERIFIED_ASSERTIONS=true`; the flag defaults to disabled, so unverified assertions are never accepted by default.
Consequences: SAML sign-in requires an explicit, deliberate operator opt-in and must only be used behind a trusted network or an identity-aware proxy. Recommended long-term fix: adopt `samael` with `xmlsec1` for deployments that allow a C toolchain, or contribute a pure-Rust exclusive-c14n implementation and drop the flag.
