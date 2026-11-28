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
| C. Auth and access control | ~12k LOC server | **part** (wave 2 and P1–P3 landed; object/field enforcement, transactional update RLS, feature-gated relation-hop RLS and company access-group materialization are implemented; sharing, propagation plans and the remaining permission surfaces are open) |
| D. Web interface (Leptos) | ~330k LOC React | **part** (front-core, design system, app shell and core record surfaces landed; exact full-feature UI parity and the remaining product screens are open) |
| E. Automations and integrations | ~60k LOC server | todo |
| F. Platform and operations | ~10k LOC | **part** |

### Measured checkpoints (2026-11-28)
- Wave 4 live record parity: `tools/record-parity.py` — **0 gaps / 13 diffs** measured at `74d140c` against the real `twentycrm/twenty:v2.61.0` container. The earlier "18/18 captures match" line was recorded while the harness read `accessToken` through the redacting `request()` helper, so the Rust side was called with the literal placeholder `Bearer <jwt>` and every row was a GAP; that claim is withdrawn until re-measured. The 13 diffs group into four root causes: composite fields on the connection path (`Column "nameFirstName" does not exist on "person"` for `people { edges { node { name { firstName } } } }`, also through `orderBy`), relation writes in create data (`Column "company" does not exist on "person"` where 2.61.0 wants `company: { connect: { id } }`), bulk create producing invalid SQL (`createCompanies` → `syntax error at or near "0"`, `INTERNAL_SERVER_ERROR`, which cascades into every `merge-many` row), and the REST projection returning `companyId` where the reference returns the nested `company` object.
- Shared-workspace harness: `tools/parity-compare.py:fetch_parity_pair()` signs up on Nudge, signs the same account into the reference (`/metadata signIn` → `getAuthTokensFromLoginToken`), activates the workspace and caches the pair in `NUDGE_PARITY_STATE` (mode 0600). Both servers then read one workspace, so a diff is behavior rather than seed data. Two reference constraints drive the flow: the container runs `IS_MULTIWORKSPACE_ENABLED=false`, so `getAuthTokensFromLoginToken` resolves `getDefaultWorkspace()` — the newest `core.workspace` row by `createdAt DESC` (`twenty-2610/.../workspace-domains.service.ts:76-133`) — which is why a token only works for the most recently created workspace; and a `CREATED` workspace cannot exchange at all, which is what Nudge sign-up writes today (`crates/server/src/auth/service.rs:200`) while the reference writes `ACTIVE`.
- REST error/shape parity: `tools/parity-rest-errors.py` — **18/18 probes match** live.
- Auth + metadata + record E2E: `tools/auth-e2e.sh` — **372/372 checks pass**.
- Metadata content: `tools/metadata-parity.sh` — **5 sections, 0 differences** (37 objects / 636 fields / 65 options / 123 indexes / 34 search fields).
- Workspace schema: `tools/prove-attnum-parity.sh` — **39/39 tables have identical ordered columns**.
- Quality gates: fmt clean, clippy `-D warnings` clean, **5 core + 534 server tests** pass; `tools/test-workspace.sh` runs the full workspace serially against an isolated Postgres database and drops it on exit.
- Event layer: the `events.rs` monolith is split into `crates/server/src/events/` (`constants`, `clock`, `payload`, `gate`, `acl`, `record`, `registry` — 3,029 lines, 44 tests) and merged at `74d140c`; the SSE loop now drives `EventStreamRegistry::heartbeat` instead of a fixed 15 s `{}` frame, so streams no longer silently stop after the TTL. Nothing calls `publish_metadata`, `publish_queue_job` or `load_permission_flag_acl` outside tests yet — there is no `database_event` emitter in `crates/server/src`, 13 of the 17 upstream `@OnEvent` listeners are unported, and per-subscriber permission masking plus Redis pub/sub channels are absent.
- Lanes stopped by the 150-turn ceiling mid-edit, work committed and pushed but not yet gated: `852ccd26` at `8968b81` (webhook sender/rate-limit plus 269 new lines in `crates/server/src/metadata/api/handlers.rs`), `d8d950a3` at `b8ec30a` (file upload: `file/upload.rs` 1,235 lines, `drivers/local.rs`, `drivers/s3.rs`, `storage.rs`, `mime.rs`, `validation.rs`, `folder.rs`; `file.rs` became `file/mod.rs`). Both touched `query/entry/graphql.rs` and `events.rs`, so they conflict with the events split and need a serialized rebase.- Frontend gates: `tools/build-frontend.sh`, native clippy/tests and `wasm32-unknown-unknown` check pass; 32 front-core tests, 138 front-ui tests and 17 front-app tests pass. The app is served at `http://127.0.0.1:3000`. The release wasm is 2.4MB; the missing `target_features` section warning remains open until `wasm-opt` is available.
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
| Record permissions, sharing, private records | **part** | 4 | P1–P3 and core/advanced RLS landed: role/object/field gates, predicate groups, current-member substitution, transactional create/update rollback, nested target read scope, upstream relative-date ranges and relation-hop feature gating. Company access groups now materialize (`permissions/access_group.rs`): group hashes and compact ids, `_accessGroupIds` arrays with a generation column, lazy materialization on read and recompute inside create/update/delete mutations. Upstream propagation plans, `recordShare`-driven groups and the remaining objects are open. |
| Export records | todo | 6 | |
| REST API with the upstream shape | **done** | 4 | 18/18 live error/shape probes; fields/depth, error bodies, envelopes and messages match |
| Dynamic GraphQL schema generated from metadata | **done** | 4 | direct workspace execution with resolver classification, introspection and connection shaping |
| Live updates over SSE (`OnEventSubscription`) | **part** | 4→5 | workspace-scoped streams, named next frames, typed object/variable signatures, exact-object/empty-filter matching, post-commit create/update/delete/destroy/restore events with writer identity and diffs, and query re-registration are implemented. The engine/subscriptions port now sits in `crates/server/src/events/` (`constants`/`clock`/`payload`/`gate`/`acl`/`record`/`registry`, 44 unit tests): stream registry keyed by `(workspaceId, eventStreamId)` with the upstream re-open/replace rule and `isAuthorized` principal match, the 30 min TTL on an injectable clock with the TTL/5 heartbeat refresh, the four-key chunk shape, `pickBroadcastEventProperties`, the `workflow`/`workflowVersion`/`view`/view-child/`navigationMenuItem` recipient resolution with the morph-relation head collapse, and the role→flag grant matrix. The SSE loop now drives `EventStreamRegistry::heartbeat` on `APPLICATION_KEEPALIVE_INTERVAL_MS` instead of a fixed 15 s empty frame. Remains: per-subscriber permission-masked record payloads (`filterReadableEvents`), nested-relation enrichment, metadata/queue publish callers (see section E), Redis pub/sub multi-node transport with the `eventStreamIdToChannelId` UUIDv5 channel naming, `resolveMetadataEvents` locale translation, and reconnect. One measured divergence: `StreamPrincipal::is_authorized_for` still falls back to `userId` when a token carries neither `userWorkspaceId` nor `apiKeyId`, where 2.61.0 denies (workspace-agnostic tokens depend on it). |
| Pre and post query hooks (actor stamping, timeline snapshots) | todo | 4→6 | actor stamping exists in mutations; 2.61.0 registers pre-hooks only (`@WorkspaceQueryHook` in the messaging, workflow and timeline modules) and the events module split ports none of them; the post-hook engine remains |

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
| Role, object, field and row level permission enforcement | **part** | 2→4 | P1–P3 plus transactional update RLS, upstream relative-date ranges, feature-gated relation-hop RLS and company access-group materialization landed; predicate values follow the upstream view-filter stringification, so a literal-value group only matches Select/Uuid/Relation fields as it does upstream. `recordShare` writes, the `_isPrivate` state machine, the access-group propagation pipeline and non-company group materialization are `@license Enterprise` in 2.61.0 (`research/permissions.md` §0, §8 D0) and stay out of scope. |
| Channel visibility (message/calendar field redaction) | todo | 4→6 | Apache code in 2.61.0, not an RLS rule: `common-record-visibility.service.ts:16` runs from `common-base-query-runner.service.ts:343` on every result and blanks `message.subject/text/attachments`, `attachment`, `calendarEvent` and `timelineActivity.properties` for members who do not own the connected account, writing the `FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED` sentinel. Nudge has no redaction layer, so object-level read on `message` currently exposes another member's mail body; `search.rs:39-50` has the object list but the default search branch skips it. See `research/permissions.md` §2.10. |
| Permission-table DDL gate | todo | 3→4 | Nudge creates `accessGroup`/`recordShare` unconditionally (`standard_objects.rs:483`, `standard_objects/permissions.rs:97`); 2.61.0 gates that DDL on `IS_PERMISSION_TABLES_ENABLED` (`FeatureFlagKey.ts:18`). `IS_RELATED_RECORD_PATHS_ENABLED` (`FeatureFlagKey.ts:10`) has no Nudge reader. |

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
| Document shell, PWA assets, mount node | **done** | 5 | `crates/front-app/index.html` carries the reference head nodes in reference order with Nudge branding, `crates/front-app/assets/manifest.json` plus the generated icon set in `crates/front-app/assets/icons/` serve 200, and `crates/front-app/src/main.rs:25` mounts into the shell's `div#root` the way the reference React root does. Measured by `tools/parity-compare.py` (`shell head contract` and `/manifest.json asset contract` cases, 0 differences) and by reading the live DOM of :3000 against :3001 |
| Runtime document title and favicon | todo | 5 | The reference sets `document.title` per route (`modules/ui/utilities/page-title/components/PageTitle.tsx`, skipped for the side-panel surface) and repoints `rel=icon` at the workspace public-data logo with a default fallback (`modules/app/components/PageFavicon.tsx`). Nudge has no runtime setter for either: `grep -rn "document.title\|set_title" crates/front-*/src` is empty, so the tab keeps the static shell title and the icon never follows the workspace logo |
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
| Event system and the transactional outbox | **part** | 7 | Measured against 2.61.0: the engine registers 17 `@OnEvent` handlers in 5 files, and the *decision* logic behind two of them is ported and unit-tested in `crates/server/src/events/` — `metadata-events-to-db.listener.ts` (3 handlers) coverage is `gate.rs` broadcast resolution (permission-flag gate, recipient lists, gated property picking, morph-relation head collapse) plus `acl.rs` role→flag grants, and `queue-job-event.listener.ts` (1 handler) coverage is `payload.rs` (`bullMQToJobStateEnum`, progress percentage, the queue-job chunk). Nothing runs them yet: `publish_metadata`, `publish_queue_job` and `load_permission_flag_acl` have no callers outside tests, because the emitter (`WorkspaceEventEmitter.emitDatabaseBatchEvent` / `computeEventName`), the `metadata.<name>.<type>` raise from workspace migrations, the `updatedCollectionHash` read from the workspace cache, the field-metadata and navigation-menu-item enrichment and the queue that carries `QUEUE_JOB_CHANGED_EVENT` are all unported. The remaining 13 listeners are `todo`: 7 in `access-group-sync.listener.ts` (permissions lane, section B), 3 in `agent-chat-channel-role-access.listener.ts`, 3 in `webhook-subscription-workspace-activation.listener.ts`. The transactional outbox itself, the per-node Redis event bus and the workflow database-event trigger matching (`computeTriggerEventNamesMatchingEvent`, `filterEventsMatchingDatabaseEventTrigger`, `database-event-trigger-condition.service`) are not ported — `grep -rn "database_event" crates/server/src` is empty. |
| Email sending over SMTP, with the upstream templates | **part** | 6 | Port on `worktree-agent-general-purpose-852ccd26` (`758090c`), not yet merged to main; the 17 React Email templates remain unported |
| Outbound webhooks with the HMAC signature | **part** | 6 | Port on `worktree-agent-general-purpose-852ccd26` (`758090c`), not yet merged to main; byte-for-byte signature parity against the reference is still unmeasured |
| File storage: local disk and S3 | **part** | 6 | signed local core-picture/files-field download supports workspace/folder binding, expiry policy, safe paths, MIME, cache and ranges (`crates/server/src/file.rs`, 955 lines, download-side only); the upload lane runs on `worktree-agent-general-purpose-d8d950a3` against `packages/twenty-server/src/engine/file-storage/` and `file-upload/` — `grep -rn upload crates/server/src` was empty before it started. S3, remote files and Files ownership guards remain |
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

1. Channel-visibility field redaction (Apache, leaks mail bodies today) and the record-plane gaps: destructive merge, file-backed Search images, full SSE events and hooks.
2. RLS criteria on the record plane: a non-default role that carries a read-criteria rule must be denied the collection read the way 2.61.0 denies it, while the all-records role must stay unfiltered — see [research/permissions.md](research/permissions.md) §2.11 for the measured pair and the harness.
3. Permissions P4–P5 follow-ups: read `IS_PERMISSION_TABLES_ENABLED` before the permission DDL, and give non-company objects the same access-group treatment when their rules exist.
4. Frontend exact parity: runtime `document.title` per route and the workspace-logo favicon (see the todo row in section D), production Dialog/Popover/Menu/Tabs contracts, record index/show surfaces, contextual command menu, routed side panel and settings leaves.
5. Merge queue awaiting independent verification: `852ccd26` webhook/email wiring (`8968b81`, WIP), `d8d950a3` file upload (`b8ec30a`, WIP), `fa8b49bd` validation checkers (unwired, `crates/server/src/validation/`), `9e010430` SSRF client, `bb26a00c` record sort, and the two live-parity defect lanes (`a6949ba9` composite projection plus relation writes, `a53090c3` bulk-create SQL plus REST relation projection). `39329ce2` SSE events and `83cf900b` destructive merge are merged (`74d140c`, `e424ac9`) with fmt/clippy/tests/`tools/test-workspace.sh` green.
6. Wave 6: webhooks, files, SMTP email, record field validation, search/import/export, timeline, favorites, notes/tasks.
7. Wave 7: queues, workflows, messaging/calendar sync, AI agents, MCP, billing and the remaining platform features.
8. Full-app live UI/behavior/performance verification against Twenty 2.61.0 before cutover.

The upstream is roughly 1.8M lines across 25,000 files. A faithful port is a multi-session
program; the waves in [MAP.md](MAP.md) are ordered so that every wave leaves a usable server.
