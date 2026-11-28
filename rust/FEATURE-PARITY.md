# Feature parity with the upstream CRM

What Nudge has, what it does not, and what it takes to close the gap. Status is measured, not
estimated: every "done" row has a test or a verification script behind it.

Legend: **done** = implemented and verified · **part** = partially implemented · **todo** = not
started. The wave numbers are the ones in [MAP.md](MAP.md).

## Summary

| Area | Upstream size | Nudge today |
|---|---|---|
| A. Data model and metadata engine | ~15k LOC server, 5.8k LOC UI | **part** (seeding, DDL runner, metadata CRUD API and deferred queue merged; validation rules and UI editors remain) |
| B. Record APIs (GraphQL, REST, query runner) | ~30k LOC server | **part** (find/create/update/delete/restore, pagination, groupBy/aggregate, dynamic GraphQL, REST, Search and duplicate detection implemented; destructive merge, file-backed Search images, advanced access groups, full SSE and hooks remain) |
| C. Auth and access control | ~12k LOC server | **part** (wave 2 and P1–P3 landed; object/field enforcement, transactional update RLS and feature-gated relation-hop RLS are implemented; access-group synchronization, sharing and the remaining permission surfaces are open) |
| D. Web interface (Leptos) | ~330k LOC React | **part** (front-core, design system, app shell and core record surfaces landed; exact full-feature UI parity and the remaining product screens are open) |
| E. Automations and integrations | ~60k LOC server | todo |
| F. Platform and operations | ~10k LOC | **part** |

### Measured checkpoints (2026-11-28)
- Wave 4 live record parity: `tools/parity-compare.py --records` — **18/18 captures match** the real `twentycrm/twenty:v2.61.0` server on the same database.
- REST error/shape parity: `tools/parity-rest-errors.py` — **18/18 probes match** live.
- Auth + metadata + record E2E: `tools/auth-e2e.sh` — **372/372 checks pass**.
- Metadata content: `tools/metadata-parity.sh` — **5 sections, 0 differences** (37 objects / 636 fields / 65 options / 123 indexes / 34 search fields).
- Workspace schema: `tools/prove-attnum-parity.sh` — **39/39 tables have identical ordered columns**.
- Quality gates: fmt clean, clippy `-D warnings` clean, **5 core + 534 server tests** pass; `tools/test-workspace.sh` runs the full workspace serially against an isolated Postgres database and drops it on exit.
- Frontend gates: `tools/build-frontend.sh`, native clippy/tests and `wasm32-unknown-unknown` check pass; 32 front-core tests, 138 front-ui tests and 17 front-app tests pass. The app is served at `http://127.0.0.1:3000`. The release wasm is 2.4MB; the missing `target_features` section warning remains open until `wasm-opt` is available.
- Browser checkpoint: the built-in Browser renders `/gallery` with 8 data rows, no malformed row attributes and no horizontal overflow. Real pointer interactions pass for reset, sorting, selection, row activation, popover, collapsible and toast. The welcome heading/font metrics now match the reference; authenticated product-surface parity remains open.
- Auth browser checkpoint: email branching, validation, password visibility and reset notice are implemented. A strict live recapture of `/welcome` is stored in `parity/ui/welcome-diff.json`; it currently records 12 open differences at tolerance 0, including subpixel geometry, browser-resolved minimum sizes and the footer wrapper margin. Password, sign-in and toast ledgers are legacy evidence and require recapture.
- Auth surface checkpoint: the exact upstream Inter 400/500/600 font files are bundled. `parity/ui/welcome-reference.json`, `welcome-nudge.json` and `welcome-diff.json` are regenerated with the strict harness; full auth-surface parity remains open.
- UI evidence: reference/Nudge auth captures are in `parity/auth/`; live People-table and record-show captures are in `parity/people-index/` and `parity/record-show/`. The reusable DOM/style/icon/animation harness is `tools/ui-parity-harness.mjs`; it records all attributes plus selector/tolerance/ignore metadata, and `tools/ui-parity-harness.test.mjs` covers strict diff behavior. Evidence under `parity/ui/` is not a full-parity certificate.

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
| `findDuplicates`, `mergeMany` | **part** | 4→6 | duplicate detection supports `id`/`data` criteria through GraphQL and REST; merge is dry-run only, with destructive writes, child migration and collision handling open |
| `createOne`/`createMany`, `updateOne`/`updateMany` | **done** | 4 | mutation round-trip capture matches live, including actor fields and upsert |
| `deleteOne` (soft), `destroyOne`, `restoreMany` | **done** | 4 | soft-delete visibility, destroy and restore envelopes match live |
| `groupBy` and aggregates | **done** | 4 | aggregate runner + group-by E2E and native tests |
| Full text search | **part** | 4→6 | cross-object ranking, filters, cursors, CJK fallback, favicons, metadata overrides, standard/application label catalogs and signed core-picture avatars are implemented; Files ownership guards and replica routing remain |
| Record permissions, sharing, private records | **part** | 4 | P1–P3 and core/advanced RLS landed: role/object/field gates, predicate groups, current-member substitution, transactional create/update rollback, nested target read scope, upstream relative-date ranges and relation-hop feature gating. Access-group synchronization and sharing remain. |
| Export records | todo | 6 | |
| REST API with the upstream shape | **done** | 4 | 18/18 live error/shape probes; fields/depth, error bodies, envelopes and messages match |
| Dynamic GraphQL schema generated from metadata | **done** | 4 | direct workspace execution with resolver classification, introspection and connection shaping |
| Live updates over SSE (`OnEventSubscription`) | **part** | 4→5 | workspace-scoped streams, query registration, named next frames, post-commit `updated` events and browser readiness/cancellation are implemented; create/delete, filter-level matching, metadata/queue events, reconnect, multi-node transport and hooks remain |
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
| Browser email/password auth flow | **part** | 5 | Existing/new email branching, reference-matched second step, validation, password visibility and reset notice are implemented; SSO/2FA screens and full account-recovery UI remain |
| Role, object, field and row level permission enforcement | **part** | 2→4 | P1–P3 plus transactional update RLS, upstream relative-date ranges and feature-gated relation-hop RLS landed; access-group synchronization and sharing remain |

## D. Web interface (Leptos)

The design-system crate, front logic layer, app shell and core record surfaces have landed. Exact
full-feature UI parity remains open. The upstream is about 330k lines of React across 12,700 files;
the Leptos port follows the unit plan in `research/frontend-architecture.md` and is compared against
the live reference in the browser.

| Feature | Status | Wave | Notes |
|---|---|---|---|
| Front logic layer (clients, metadata store, cache, SSE decode, query builder) | **done** | 5 | U2 `crates/front-core`; native tests, wasm check and a live Twenty round trip pass |
| Design system: tokens, primitives, icons | **part** | 5 | Tokens, 4187 Tabler icons, 46 custom icons and the upstream motion layer landed. Controlled forms, status, navigation/feedback primitives and a production `RecordTable` are covered by 138 front-ui tests; the gallery has browser-verified sorting, selection, row activation and custom cells. The email/password auth flow is compared against the live reference. Exact component and page parity remains open. |
| Application shell, navigation, command menu | **part** | 5 | U3 router, drawer/header, theme and session persistence landed; the command menu uses global Search with keyboard selection and stale-response guards; contextual command sections and exact side-panel motion remain |
| Record table: virtualised, drag and drop, groups, aggregates | **part** | 5 | U4a loads live metadata/records; U4b adds server filters, sorting, 240-slot virtualization, Person/Company create dialogs, relation selection and functional List/Kanban/Calendar view dispatch; drag/drop, groups and aggregates remain |
| Record kanban board | **part** | 5 | U4b grouping and view dispatch landed; upstream drag/drop and live visual parity remain |
| Record calendar and list views | **part** | 5 | U4b month/week/calendar layouts and list dispatch landed; live behavior and exact UI parity remain |
| Record show page and per type field display and input | **part** | 5 | U4a record route and live show page landed; U4b adds Person name/email/job-title editing and company relation selection while preserving identity and timestamps; per-type editors for other objects remain |
| Views, filters, sorts, group by | **part** | 5 | server filters, sorting and List/Kanban/Calendar dispatch exist; named views, groups, view options and exact upstream behavior remain |
| Side panel (57 kinds) | **part** | 5 | record-details panel and Escape/focus behavior exist; routed 57-kind content, resizing, navigation stack and exact motion remain |
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
| File storage: local disk and S3 | **part** | 6 | signed local core-picture/files-field download supports workspace/folder binding, expiry policy, safe paths, MIME, cache and ranges; upload, S3, remote files and Files ownership guards remain |
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

1. Permissions P4–P5 and record-plane gaps: access-group synchronization, private/share overlays, destructive merge, file-backed Search images, full SSE events and hooks.
2. Frontend exact parity: production Dialog/Popover/Menu/Tabs contracts, record index/show surfaces, contextual command menu, routed side panel and settings leaves.
3. Wave 6: webhooks, files, SMTP email, search/import/export, timeline, favorites, notes/tasks.
4. Wave 7: queues, workflows, messaging/calendar sync, AI agents, MCP, billing and the remaining platform features.
5. Full-app live UI/behavior/performance verification against Twenty 2.61.0 before cutover.

The upstream is roughly 1.8M lines across 25,000 files. A faithful port is a multi-session
program; the waves in [MAP.md](MAP.md) are ordered so that every wave leaves a usable server.
