# Feature parity with the upstream CRM

What Nudge has, what it does not, and what it takes to close the gap. Status is measured, not
estimated: every "done" row has a test or a verification script behind it.

Legend: **done** = implemented and verified · **part** = partially implemented · **todo** = not
started. The wave numbers are the ones in [MAP.md](MAP.md).

## Summary

| Area | Upstream size | Nudge today |
|---|---|---|
| A. Data model and metadata engine | ~15k LOC server, 5.8k LOC UI | **part** (seeding, DDL runner, metadata CRUD API and deferred queue merged; validation rules and UI editors remain) |
| B. Record APIs (GraphQL, REST, query runner) | ~30k LOC server | **part** (find/create/update/delete/restore, pagination, groupBy/aggregate, dynamic GraphQL and REST verified live; search, duplicates/merge, permissions, SSE and hooks remain) |
| C. Auth and access control | ~12k LOC server | **part** (wave 2 complete; P1+P2 permission model/context/kernel and currentUserWorkspace payload landed, with 27 flags and 37 objects matching the live reference; enforcement P3–P5 next) |
| D. Web interface (Leptos) | ~330k LOC React | **part** (design system landed; front-core in progress; app shell and product screens remain) |
| E. Automations and integrations | ~60k LOC server | todo |
| F. Platform and operations | ~10k LOC | **part** |

### Measured checkpoints (2026-11-27)
- Wave 4 live record parity: `tools/parity-compare.py --records` — **18/18 captures match** the real `twentycrm/twenty:v2.61.0` server on the same database.
- REST error/shape parity: `tools/parity-rest-errors.py` — **18/18 probes match** live.
- Auth + metadata + record E2E: `tools/auth-e2e.sh` — **367/367 checks pass**.
- Metadata content: `tools/metadata-parity.sh` — **5 sections, 0 differences** (37 objects / 636 fields / 65 options / 123 indexes / 34 search fields).
- Workspace schema: `tools/prove-attnum-parity.sh` — **39/39 tables have identical ordered columns**.
- Quality gates: fmt clean, clippy `-D warnings` clean, **92 core + 468 server tests** pass.
- Frontend gates: Trunk build, native clippy/tests and `wasm32-unknown-unknown` check pass; the app is served at `http://127.0.0.1:8080`.
- UI evidence: reference/Nudge auth captures are in `parity/auth/`; live People-table and record-show captures are in `parity/people-index/` and `parity/record-show/`. These are evidence snapshots, not a full-parity certificate.

## A. Data model and metadata engine

| Feature | Status | Wave | Notes |
|---|---|---|---|
| `core` schema, byte-identical to the reference | **done** | 1 | 80 tables, 57 enums, 368 indexes, 389 constraints; 0 differences |
| Workspace schema per tenant (`workspace_<base36>`) | **done** | 1 | created on sign-up |
| Object and field metadata CRUD | **done** | 3 | metadata GraphQL API + update/delete matrix merged; covered by the metadata E2E fragments in `tools/auth-e2e.sh` |
| Standard objects (person, company, opportunity, note, task, attachment, timeline, workspace member) | **done** | 3 | U7: 37 objects / 636 fields / 123 indexes / 34 search fields / 65 options seeded on activation; `tools/metadata-parity.sh` proves field/option/index/search-field content is 0-diff vs the live reference and the catalog parity test proves the workspace schema is equivalent (one open column: `targetTableName='DEPRECATED'` semantics, U10) |
| Runtime DDL: create and alter workspace tables from metadata | **done** | 3 | create/update/delete matrix, nullable/default/onDelete changes, enum rename→recreate→backfill and tsvector rebuild merged and covered by the workspace migration tests |
| Composite fields (full name, emails, phones, links, address, currency) and their flattening rule | **done** | 3 | `<field><SubProperty>` column names, U4 column_defs (2.61.0 has no server-only composite columns — verified against the tag and the live schema) |
| Field types: 25+ (select with options, rating, relation, morph relation, raw json, ts vector, ...) | **done** | 3 | U4 + U7 (composite enum types created, morph heads skipped, relation join columns + FK `onDelete` from settings) |
| Index metadata and physical index creation (hashed names) | **done** | 3 | U6 runner (hashed deterministic names, `indexFieldMetadata` rows) + U8 deferred `build_index` CONCURRENTLY |
| Search vectors and the GIN index | **done** | 3 | inline `to_tsvector('simple', ...)` generated column with the upstream expression + GIN index; search-field rows pre-inserted against the deferrable core FKs |
| Access storage tables (`accessGroup`, `recordShare`) | **done** | 3 | U7 `standard_objects/permissions.rs`: enums, identity, CHECK, 8 named indexes, created before the metadata migration exactly like upstream |
| `_isPrivate` / `_accessGroupIds` enrichment + derived indexes | **done** | 3 | U7, per `enrich-record-table-schema-*.util.ts` (GIN on `integer[]`, partial btree on `id WHERE "_isPrivate"`) |
| Deferred workspace operations (queue) | **done** | 3 | U8: claim SKIP LOCKED, delete-on-success, `30s*2^n` backoff, FAILED at 3 attempts, stale recovery, worker loop from startup |
| Validation rules | todo | 5 | 3.9k LOC upstream |
| Custom fields and objects edited from the UI | todo | 5 | |
| Data model settings screens | todo | 5 | 5.8k LOC upstream |

## B. Record APIs

| Feature | Status | Wave | Notes |
|---|---|---|---|
| `findMany` with filter, sort, keyset pagination, `totalCount` | **done** | 4 | shared runner + live parity; totalCount ignores cursor predicates and composite sort matches the reference |
| `findOne` | **done** | 4 | nested relation projection matches the live reference |
| `findDuplicates`, `mergeMany` | todo | 4→6 | REST routes return the explicit deferred-slice error; duplicate/merge query engines remain |
| `createOne`/`createMany`, `updateOne`/`updateMany` | **done** | 4 | mutation round-trip capture matches live, including actor fields and upsert |
| `deleteOne` (soft), `destroyOne`, `restoreMany` | **done** | 4 | soft-delete visibility, destroy and restore envelopes match live |
| `groupBy` and aggregates | **done** | 4 | aggregate runner + group-by E2E and native tests |
| Full text search | todo | 4→6 | tsvector columns and GIN indexes exist; the cross-object Search resolver remains |
| Record permissions, sharing, private records | todo | 4 | P1–P5 permission wave next; access storage tables exist |
| Export records | todo | 6 | |
| REST API with the upstream shape | **done** | 4 | 18/18 live error/shape probes; fields/depth, error bodies, envelopes and messages match |
| Dynamic GraphQL schema generated from metadata | **done** | 4 | direct workspace execution with resolver classification, introspection and connection shaping |
| Live updates over SSE (`OnEventSubscription`) | todo | 4→5 | frontend decoder lands in front-core; server transport and query registration remain |
| Pre and post query hooks (actor stamping, timeline snapshots) | todo | 4→6 | actor stamping exists in mutations; timeline/post-hook engine remains |

## C. Auth and access control

| Feature | Status | Wave | Notes |
|---|---|---|---|
| Password hashing, bcrypt cost 10, `^.{8,50}$` policy | **done** | 2 | |
| ES256 tokens, rotating `core.signingKey`, `kid` header | **done** | 2 | verified against Node signed vectors |
| Legacy HS256 verification | **done** | 2 | `sha256(APP_SECRET‖body‖type)` |
| `enc:v2:` secret envelope | **done** | 2 | AES-256-GCM, HKDF-SHA256 |
| Sign-up with workspace, roles and schema | **done** | 2 | |
| Two step sign-in through a single use login token | **done** | 2 | |
| Refresh rotation under the shared advisory lock | **done** | 2 | 1 minute reuse grace |
| Bearer auth context dispatch | **done** | 2 | |
| Workspace creation without a sign-up | **done** | 2 | `signUpInNewWorkspace` + `activateWorkspace` mirrors with per-workspace role seeding |
| API keys | **done** | 2 | JWT `type=API_KEY` (`sub`=workspace, `jti`=key id), one roleTarget per key, DB re-check on every request (revoked/expired take effect immediately) |
| Session cookies and the CSRF origin check | **done** | 2 | `sess_` tokens, `sha256` storage, opt-in header, origin allowlist, renewal bridge, daily cleanup |
| Two factor authentication (TOTP, backup and recovery codes) | **done** | 2 | otplib-exact TOTP, `enc:v2:` workspace-scoped secrets, OTP replay guard, backup codes, admin recovery codes, login-token consumed only after OTP |
| SSO: Google, Microsoft, OIDC, SAML | **part** | 2 | Google/Microsoft OAuth + OIDC PKCE S256 with real RS256 JWKS verification done; SAML flows done but signature verification is flag-gated off (ADR-013, no sound Rust path yet); billing entitlement gate awaits the billing module |
| Impersonation (server and workspace level) | **done** | 2 | start/exchange/stop, 1 d sessions, parked-cookie restore; workspace level gated on permission flags (flags land with the query runner) |
| Invitations | **done** | 2 | verbatim tokens, resend rotation, upstream throttle set |
| Email verification and password reset | **done** | 2 | hashed tokens, upstream error matrix, 3/15 min reset throttle, `PASSWORD_CHANGED` revocations |
| Role, object, field and row level permission enforcement | todo | 2→4 | roles, roleTargets and flags seeded; enforcement lands with the query runner |

## D. Web interface (Leptos)

The design-system crate is landed; the front logic layer is in progress. The upstream is about
330k lines of React across 12,700 files; the Leptos port follows the unit plan in
`research/frontend-architecture.md` and is verified against the live reference in the browser.

| Feature | Status | Wave | Notes |
|---|---|---|---|
| Front logic layer (clients, metadata store, cache, SSE decode, query builder) | **done** | 5 | U2 `crates/front-core`; native tests, wasm check and a live Twenty round trip pass |
| Design system: tokens, primitives, icons | **part** | 5 | tokens and layout metrics landed; icon catalog ports 4187 Tabler icons plus 46 custom brand/illustration icons; the motion layer carries 89 upstream keyframes, 320 animation/transition rules, SMIL loaders and reduced-motion fallbacks (100 front-ui tests); remaining primitive inventory and gallery are `wave5/u1b` follow-up |
| Application shell, navigation, command menu | **part** | 5 | U3 router, drawer/header, theme and session persistence landed; U4b adds a Cmd/Ctrl+K command menu with navigation, create actions and debounced record search, plus a record-details side panel with Escape/focus behavior; broader side-panel pages remain |
| Record table: virtualised, drag and drop, groups, aggregates | **part** | 5 | U4a loads live metadata/records; U4b adds server filters, sorting, 240-slot virtualization, Person/Company create dialogs, relation selection and functional List/Kanban/Calendar view dispatch; drag/drop, groups and aggregates remain |
| Record kanban board | todo | 5 | |
| Record calendar and list views | todo | 5 | |
| Record show page and per type field display and input | **part** | 5 | U4a record route and live show page landed; U4b adds Person name/email/job-title editing and company relation selection while preserving identity and timestamps; per-type editors for other objects remain |
| Views, filters, sorts, group by | todo | 5 | 27 view operations |
| Side panel (57 kinds) | todo | 5 | |
| Settings pages | todo | 5 | 86k LOC upstream |
| Auth and onboarding screens | **part** | 5 | U3/U4a workspace Email → Password flow matches the captured reference DOM/layout; full browser verification awaits the Qoder connector runtime |
| Dashboards and page layouts | todo | 5 | |
| Spreadsheet import | todo | 6 | |
| AI chat | todo | 7 | |
| Marketplace, applications, front components | todo | 7 | |
| Desktop shell (Tauri) | todo | 5 | |

## E. Automations and integrations

| Feature | Status | Wave | Notes |
|---|---|---|---|
| Job queue | todo | 7 | 29 queues upstream; a Rust native queue replaces BullMQ (ADR-007) |
| Workflow engine: 23 action types, 4 trigger kinds | todo | 7 | 35k LOC upstream |
| Variable resolution (Handlebars compatible) | todo | 7 | must be ported literally |
| Event system and the transactional outbox | todo | 7 | 17 database listeners |
| Email sending over SMTP, with the upstream templates | todo | 6 | 17 React Email templates |
| Outbound webhooks with the HMAC signature | todo | 6 | signature must match byte for byte |
| File storage: local disk and S3 | todo | 6 | |
| Messaging sync: Gmail, Microsoft, IMAP | todo | 7 | |
| Calendar sync: Google, Microsoft, CalDAV | todo | 7 | |
| Contact auto creation and participant matching | todo | 7 | |
| Timeline activities | todo | 6 | |
| AI agents, chat and tools | todo | 7 | |
| MCP server | todo | 7 | |
| Logic functions and the app runtime | todo | 7 | needs a JavaScript runtime in Rust |

## F. Platform and operations

| Feature | Status | Wave | Notes |
|---|---|---|---|
| Postgres pool, schema per workspace, migrations | **done** | 1 | |
| Redis | **done** | 1 | |
| Health endpoint | **done** | 1 | |
| Docker compose for the stack | **done** | 1 | |
| Password authentication and token lifecycle | **done** | 2 | |
| Rate limiting and throttling | todo | 6 | |
| Usage limits and billing | todo | 7 | |
| Admin panel | todo | 7 | |
| Metrics and tracing beyond logs | todo | 7 | |
| Configuration stored in the database | todo | 7 | |

## What is left, in order

1. Permissions P1–P5 and the remaining record-plane gaps: search, duplicates/merge, SSE and hooks.
2. Wave 5 frontend: finish U1/U2, then U3 shell + auth, U4 record surfaces, U5 settings, U6 desktop/static serving.
3. Wave 6: webhooks, files, SMTP email, search/import/export, timeline, favorites, notes/tasks.
4. Wave 7: queues, workflows, messaging/calendar sync, AI agents, MCP, billing and the remaining platform features.
5. Full-app live UI/behavior/performance verification against Twenty 2.61.0 before cutover.

The upstream is roughly 1.8M lines across 25,000 files. A faithful port is a multi-session
program; the waves in [MAP.md](MAP.md) are ordered so that every wave leaves a usable server.
