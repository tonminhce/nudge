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
    seeded workspace catalog against the live Node reference (ordered attnum column vectors —
    see the physical-parity bullet below); `tools/metadata-parity.sh` compares seeded metadata
    content keyed by universal identifier (field/option/index/search-field zero-diff; the
    objectMetadata residual is the `targetTableName` value owned by `wave3/u10`).
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
- **Wave 4 started** (spec `research/query-layer.md`):
  - U1 merged: pure `query::filter` / `query::order` / `query::cursor` / `query::compile`
    (the `:name -> $n` rewrite reused by U2's statement compilation).
  - U2 `query::repository` (branch `wave4/u2`): `table_shape.rs` ports
    `buildWorkspaceTableShape` (`twenty-server/src/engine/twenty-orm/table-shape/utils/
    build-workspace-table-shape.util.ts:25-128` @ `twenty/v2.61.0`) over the wave-3 flat maps:
    composite fields expand to `<field><PascalCase(property)>` columns via the shared
    `column_defs::composite_types` definitions (`computeCompositeColumnName`,
    `metadata-modules/field-metadata/utils/compute-column-name.util.ts:42-62`), MANY_TO_ONE
    relations add the `<field>Id` join column
    (`compute-morph-or-relation-field-join-column-name.util.ts:5-9`), morph heads
    (`universalIdentifier == morphId`, `twenty-shared/src/utils/fieldMetadata/
    isMorphRelationHead.ts:12-22`) are skipped, to-many relations get a shape without a join
    column, and `tableName` follows `computeObjectTargetTable`
    (`engine/utils/compute-object-target-table.util.ts:6-17`). The upstream per-context
    `WeakMap` shape cache (`engine/twenty-orm/datasource/workspace-data-source.ts:33-36,
    298-334`) is mirrored as the bounded LRU `TableShapeCache` keyed
    `(workspaceId, metadataVersion, objectId)` (deviation D6, §11). `sql.rs` ports
    `buildInsertStatement` (`engine/twenty-orm/sql/utils/build-insert-statement.util.ts:32-65`,
    incl. `DEFAULT` for omitted row columns) and `buildMutationStatement`
    (`build-mutation-statement.util.ts:81-112`) driven by the builder semantics of
    `workspace-mutation-query-builder.ts:139-223`: soft delete stamps
    `deletedAt + updatedAt = CURRENT_TIMESTAMP` and appends the `"alias"."deletedAt" IS NULL`
    live-row predicate outside the guarded expression (:65-79), restore sets `deletedAt = NULL`,
    update appends `updatedAt` maintenance unless the caller set it (:188-195), a missing
    `deletedAt` column raises `UNSUPPORTED_OPERATION` (:216-223), the `rowAccessConditions`
    render outside the user OR chain (:558-586), and the result aliases are the 63-byte
    `alias_column` names, sha256-hashed when longer
    (`workspace-query-executor/utils/build-postgres-safe-alias.util.ts:4-16`).
    `QUERY_MAX_RECORDS = 200` (`twenty-shared/src/constants/QueryMaxRecords.ts:1`) with
    `updateMany` overflow -> `TOO_MANY_RECORDS_TO_UPDATE`
    (`engine/twenty-orm/repository/workspace-repository.ts:636-646` @ the tag). Accepted
    statements are byte-equal to the upstream jest fixtures
    (`sql/utils/__tests__/build-insert-statement.util.spec.ts:45-49`,
    `query-builder/__tests__/workspace-mutation-query-builder.spec.ts:108-114, 147-152,
    204-210`); the DB-gated test seeds a real workspace via `seed_standard_objects`, proves
    every person/company shape column against `pg_attribute` (composite sub-columns, join
    columns, `searchVector`; the enrichment pair excluded because it is not a metadata field),
    executes the emitted insert/soft-delete/restore/update/destroy SQL, and proves the
    `deletedAt IS NULL` skip on an already-trashed row.
  - Loader fix U2 required: `metadata/flat/rows.rs` now populates the object's
    `application_universal_identifier` (subquery join to `core.application`); without it
    `computeObjectTargetTable` classified every standard object as custom (`_person`).
    Upstream's flat maps always carry it
    (`engine/metadata-modules/flat-object-metadata/types/flat-object-metadata.type.ts` @ the tag).
    Field/index/option/search-field identifiers stay empty on load (deferred; nothing in the
    record path reads them yet — listed under known gaps).
  - U3 `query::runner` (branch `wave4/u3`): a workspace model over the flat maps + table
    shapes, `RunnerSchema::build`, the filter schema adapter, `find_many` with the upstream
    truthiness conflict matrix, page size (`first ?? last ?? 200`, capped), keyset cursor
    filters from the shared `getEffectiveScanOrder` model, the automatic `id` tie-breaker,
    `deletedAt IS NULL`, one aggregate `COUNT(*)`, to-one filter/order joins, EXISTS-based
    to-many filters, batched nested relation loading (to-one by id and to-many grouped by the
    reverse join column with the 60-per-parent cap), and `find_one` with required-filter +
    `RECORD_NOT_FOUND` semantics. Result formatting rebuilds composite field objects and maps
    the null-equivalent empty string back to null. The DB-gated test seeds a workspace and
    exercises composite filters, relation filters/orders, pagination totals, nested to-one
    loading, findOne and arg conflicts. HTTP/GraphQL wiring remains for wave 4 U6/U7.
  - U4 `query::runner::aggregate` + `query::runner::group_by` (branch `wave4/u4`): the
    aggregate SQL expression matrix (`totalCount`, `countEmpty/NotEmpty`, unique counts,
    percentages, boolean counts, min/max/avg/sum with the rating split-part cast) and the
    upstream result names (`countNotEmptyName`, `minCreatedAt`, ...). `group_by` resolves
    physical/composite columns, emits the upstream date-granularity SQL, validates
    `MISSING_TIMEZONE_FOR_DATE_GROUP_BY` and `INVALID_TIMEZONE`, always computes
    `totalCount`, and defaults to 50 groups ordered by the group key. The DB-gated test
    seeds a workspace and proves company aggregate names/values, ordered person groups with
    per-group totalCount, and both timezone error codes.
  - U5 `query::runner::create` + `query::runner::mutation` + actor/position helpers
    (branch `wave4/u5`): createOne/createMany reuse the repository insert statements, stamp
    `createdBy`/`updatedBy` actor composites, backfill `position` first/last with min-1 /
    max+1 arithmetic, and reject over-limit creates with
    `TOO_MANY_RECORDS_TO_UPDATE`. Filtered update/soft-delete/destroy/restore runners
    compile through `build_mutation_statement`, expand composite write data, cast UUID /
    JSONB / timestamp / actor-enum parameters for sqlx, and return `RECORD_NOT_FOUND` on
    one-record mutations. Upsert detects unique index groups (including composite
    `isIncludedInUniqueConstraint` sub-fields), loads existing records with
    `deletedAt IS NULL`, raises
    `UPSERT_MULTIPLE_MATCHING_RECORDS_CONFLICT`, strips `createdBy`, clears `deletedAt`,
    and updates the existing row. The DB-gated test proves actor/position stamps, update,
    soft-delete/restore visibility, destroy, and company upsert by `domainName.primaryLinkUrl`.
  - U6 `query::entry` (branch `wave4/u6`): `/graphql` is mounted for POST JSON,
    POST `application/graphql` and GET query parameters. Requests hydrate through the existing
    bearer/session auth, refuse agent tokens (`INVALID_AUTH_CONTEXT`), classify top-level
    roots into introspection / workspace / core, and reject core+workspace mixes with the
    upstream split-request message (`BAD_USER_INPUT` / `CORE_WORKSPACE_MIX`). The workspace
    resolver map follows `getResolverName`: `people`, `person`, `createPerson`,
    `createPeople`, `updatePerson`, `deletePerson`, `destroyPerson`, `restorePerson`,
    `peopleGroupBy`. Find-many returns the connection shape with real per-record cursors,
    pageInfo and totalCount; findOne and create/update/delete/destroy/restore project the
    selected record shape and load selected one-level relations. `currentUser` and
    `currentWorkspace` are the first core resolvers. The metadata-derived introspection model
    exposes `__schema` root types, object/connection/edge/pageInfo fields and `__type`; SDL is
    available from `WorkspaceSchema::sdl()`. E2E covers unauthenticated, introspection,
    connection, create/update/delete visibility, and the mix rejection.
  - Next: U7 REST entry; then groupBy/root operation fan-out and wave 5 (Leptos front) spec
    `research/frontend-architecture.md`.

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

## Production deployment verification (re-run after wave 3)
`docker build -f rust/infra/Dockerfile -t nudge-server:u7 .` from the repo root produces a
170MB release image; booted against a fresh scratch DB it migrates `core` on startup, serves
`/healthz` in the Terminus shape within ~2s, and a `POST /auth/sign-up` followed by activation
seeds the 37 standard objects / 636 fields / workspace schema + enrichment indexes (verified by
row counts in the scratch DB, then dropped). `rust/infra/docker-compose.prod.yml` wires the same
image to the bundled Postgres/Redis services. CI runs all three jobs green on every push
(fmt/clippy/unit, the database-gated suites against a service Postgres, and both e2e modes).

## Comparing against the live Node reference
The reference container `twenty-ref` (`twentycrm/twenty:v2.61.0`, port 3001, database
`twenty_ref`) runs the REAL product. Seed a FRESH container's first workspace with
`python3 -I rust/tools/seed-reference.py [outdir]` (writes `ws-id.txt` + `ref-access.txt`; it
signs up, creates the `refhq` workspace through `signUpInNewWorkspace`, exchanges the login token
on the subdomain origin and activates). Do not re-run it against the already-seeded `twenty_ref`
— single-workspace mode gates further workspace creation, and the parity gates depend on that
`refhq` workspace existing.
- `python3 -I tools/parity-compare.py` — same request set against both servers, normalized
  and diffed. Current scoreboard: `/healthz` (Terminus shape) and the NestJS fallback 404
  body match; `/graphql` + `/metadata` answers are the wave 4 gap; the SPA catch-all is the
  wave 5 gap.
- `python3 -I tools/parity-bench.py` — 8 workers x 250 requests per scenario on this 10-core
  machine, release build (`cargo build --release`): healthz rust 2789 rps / p99 6.5ms vs node
  1155 rps / p99 68.8ms; graphql auth-error rust 7985 rps / p99 2.1ms vs node 2411 rps / p99
  10.1ms (rust wins throughput ~2.4-3.3x and tail latency ~5-16x with zero errors). The debug
  build is materially slower than release (the earlier 2125 rps healthz figure was a debug run),
  so measure with the release binary; production deploy uses `rust/infra/Dockerfile`, which
  builds `--release`.
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
- U7 standard-object registry + activation seeding — the workspace schema equals a live 2.61.0
  reference workspace in BOTH physical column order and content (the primary equivalence gate).
  The earlier claim that attnum order was a non-semantic "heap artifact" and that a sorted-set
  comparison sufficed was WRONG (supervising audit 2026-11-27): upstream order is deterministic
  and 36 of 39 seeded tables diverged from it. Fixed on `wave3/u7-attnum`: the generator reorders
  each object's fields to the baseline attnum, object creates carry a `column_plan` the runner
  applies before `CREATE TABLE`, and the DB-gated parity test now compares ordered attnum column
  vectors (only the schema name and the proven per-workspace objectMetadata id suffixes in
  enrichment-index names are normalized). `tools/prove-attnum-parity.sh` re-proves it live: 39/39
  tables identical ordered columns. `tools/dump-normalize.py` remains for whole-statement
  `pg_dump` diffs (its total-order sort hides order, which is why the attnum test supersedes it).

## Known gaps / deferred (documented, deliberate)
- Live-reference row audit (U7 parity work, `tools/metadata-parity.sh`): seeded `fieldMetadata`
  (636), `fieldMetadataOption` (65), `indexMetadata` (123) and `searchFieldMetadata` (34) rows
  are byte-identical to the reference when keyed by universal identifier; `objectMetadata`
  (37) matches on every column EXCEPT `targetTableName`, which upstream stores as the literal
  `'DEPRECATED'` while the Nudge seed writes the computed physical name — the fix (derive the
  table name from `nameSingular` + application id everywhere instead of reading the column)
  is U10's (`wave3/u10`, priority row-0). The seeded standard `application` row shape was
  corrected to the reference (`Standard`/`cli-sync`/`local`/`1.0.1`, id ≠ universalIdentifier)
  and both custom-application writers to `Custom`/`workspace-custom`/`local`/`1.0.1`
  (id = universalIdentifier) — verified row-for-row against `twenty_ref`.
- The reference seeds a 4th role per workspace, label `Standard role` (universalIdentifier
  `96321d41-8e4d-5046-bef7-e48fffca9c7e`, a deterministic v5 uuid), created by
  `applicationGrantRoleService.upsertGrantRole` during the standard-application sync (a grant
  role that copies the application's requested role,
  `application-grant-role.service.ts:21`, called at `twenty-standard-application.service.ts:139`).
  Nudge's activation seed does not yet create grant roles, nor the non-schema standard-application
  content the full upstream sync also seeds (views 53, viewFields 360, viewFieldGroups 36,
  pageLayouts 59, pageLayoutTabs 97, pageLayoutWidgets 115, commandMenuItems 114,
  navigationMenuItems 13, timelineActivityTypes 16 — verified row counts against `twenty_ref`).
  These are metadata rows, not workspace-schema objects, so they are outside the U7
  schema-parity gate; tracked for the permissions wave (grant-role plumbing) and a
  standard-application-content follow-on unit. There is no `core.applicationGrantRole` table;
  grants live in `roleTarget`/`rolePermissionFlag`.
- SAML signature verification: callback returns 403 `SSO_AUTH_FAILED` unless
  `AUTH_SAML_ACCEPT_UNVERIFIED_ASSERTIONS=true` (ADR-013). No sound pure-Rust path; revisit
  samael+xmlsec1 or contribute exclusive c14n upstream.
- OIDC verifies RS256 only; other advertised algorithms fail closed (ADR-012).
- SSO billing entitlement gate awaits the billing module (wave 7).
- Email delivery is LOGGER-only (`tracing::info`), SMTP lands in wave 6.
- Rate limiting beyond the upstream-specific throttles lands in wave 6.
- Workspace-level impersonation needs the IMPERSONATE permission flag, which lands with the
  query runner enforcement (wave 4).
- Wave 4 U2 loader gap: `metadata/flat/rows.rs` populates the object's
  `application_universal_identifier` (required by `query::repository::table_shape` for
  `computeObjectTargetTable`), but the field/index/option/search-field flat maps still load
  `application_universal_identifier` empty. Nothing on the record path reads those yet (the
  field-level delete guard in `metadata/api/handlers.rs` uses `is_system` +
  `is_system_side_effect` as its live conditions); populate the remaining maps with the same
  subquery pattern when a consumer needs them.

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
