# Twenty CRM — Record Query Layer (wave 4)

Research spec for the Rust port (`nudge`). Reference source (read-only):
`/Users/tonminh-mac/Documents/GitHub/twenty` @ commit `77a32af` (2026-11-27, Twenty 2.61.0).

All paths relative to the Twenty repo root. `packages/twenty-server/src` is `srv/`,
`packages/twenty-shared/src` is `shared/`. Every line number below was re-read in this
`77a32af` checkout. Schema facts are pinned in `data-model.md`; metadata/DTS facts in
`workspace-migrations.md`; the auth/session contract in `auth-permissions.md`; an earlier,
broader API sweep in `graphql-rest-api.md` — this doc does not restate those, it adds the
record-data-plane detail wave 4 needs and corrects/extends the anchors.

The wave-3 crates that already exist and are consumed here: `metadata::flat` (the flat maps
and their sqlx row mappers), `metadata::cache` (`WorkspaceFlatMapsCache`), `auth` (token
validation, bearer dispatch, session cookies, CSRF), `db` (the core `PgPool`).

> **Two data planes, two endpoints.** Twenty runs a *metadata* GraphQL schema at `/metadata`
> (static, code-first NestJS resolvers) and a *record* data plane at `/graphql` + `/rest`. This
> document is about the record plane only. The metadata plane was landed in wave 3
> (`metadata::api`). Do not confuse the two filter DSLs: the record plane uses the operator
> DSL of §3; the metadata plane uses column filters
> (`graphql-rest-api.md §3.2`, `srv/engine/metadata-modules/object-metadata/dtos/object-filter.input.ts`).

---

## 1. Entry points

### 1.1 HTTP surface and boot

`srv/main.ts` builds the Nest express app:

| Concern | File:line |
|---|---|
| body parsers (`json`, `urlencoded`) limited to `settings.maxRequestBodySize` | `srv/main.ts:83-87` |
| `text/plain` body parser, 1024 kb cap | `srv/main.ts:88` |
| `graphqlUploadExpress` mounted on `/graphql` | `srv/main.ts:90-97` |
| `graphqlUploadExpress` mounted on `/metadata` | `srv/main.ts:99-106` |
| `keepAliveTimeout` = `SERVER_KEEP_ALIVE_TIMEOUT_MS`, `headersTimeout` = +1000 | `srv/main.ts:108-113` |
| `trust proxy` from `TRUST_PROXY` | `srv/main.ts:69-73` |
| credentialed CORS | `srv/main.ts:75` |
| express-session (`getSessionStorageOptions`) | `srv/main.ts:77` |

The path prefix enum is `shared/src/types/ApiPath.ts:3-26`: `GraphQL='graphql'` (:15),
`Metadata='metadata'` (:18), `Rest='rest'` (:22), `OpenApi='open-api'` (:20),
`Health='healthz'` (:16). Nudge already registers `/healthz` and the auth routes; wave 4 adds
`/graphql`, `/metadata` (record plane = `/graphql` only), `/rest`, and `/open-api`/`/rest/open-api`.

### 1.2 Two GraphQL schemas

`/graphql` and `/metadata` are **separate `GraphQLModule.forRootAsync` instances** with
different module includes and a `resolverSchemaScope`:

- core: `srv/app.module.ts:52-56` (`GraphQLModule.forRootAsync` with `useClass: GraphQLConfigService`).
  Config at `srv/engine/api/graphql/graphql-config/graphql-config.service.ts:84-95`:
  `autoSchemaFile: true`, `include: [CoreEngineModule, ModulesModule]`,
  `resolverSchemaScope: 'core'` (:86-87), `path: /graphql` (:92), `context: () => ({ loaders })` (:93-95),
  `resolvers: { JSON: GraphQLJSON }` (:90). Apollo Playground only in debug mode (:98-102).
- metadata: `srv/engine/api/graphql/metadata-graphql-api.module.ts:22-47` (a second `GraphQLModule`),
  factory at `srv/engine/api/graphql/metadata.module-factory.ts:38-82`: `resolverSchemaScope: 'metadata'`
  (:41), plugins `useGraphQLErrorHandlerHook` (:52), `useCachedMetadata` (:58), disable
  introspection for unauthenticated in production (:67), complexity limit
  `maximumAllowedRootResolvers: 10, maximumAllowedNestedFields: 10` (:70-75), `path: /metadata` (:77).

The record data plane is the **workspace schema**, not a resolver set. It is served by a Yoga
`onRequest` plugin that executes directly (`use-direct-execution.hook.ts`, §2.1).

### 1.3 Middleware chain (order is part of the contract)

`configure(consumer)` in `srv/app.module.ts:85-142`, executed in declaration order:

1. `ApiAccessLogMiddleware` on every `ApiPath` except `healthz`, both `/{path}` and `/{path}/*path` (:87-94).
2. `CookieSessionCsrfMiddleware` on `*path` (ALL methods), excluding `POST /auth/saml/callback/:identityProviderId` (:97-104). **Already ported** in Nudge `auth/csrf.rs`.
3. `/graphql` (ALL): `ApiRequestContextMiddleware → GraphQLHydrateRequestFromTokenMiddleware → GraphQLRefuseSuspendedWorkspaceMiddleware → WorkspaceAuthContextMiddleware` (:106-113).
4. `/metadata` (ALL): `GraphQLHydrateRequestFromTokenMiddleware → WorkspaceAuthContextMiddleware` (:115-120).
5. `/admin-panel` (ALL): same as `/metadata` (:122-127).
6. `/mcp` (ALL): `ApiRequestContextMiddleware → McpMethodGuardMiddleware` (:129-131).
7. `/rest/*path` for DELETE, POST, PATCH, PUT, GET (the `MIGRATED_REST_METHODS` array at `srv/app.module.ts:38-44`):
   `ApiRequestContextMiddleware → RestCoreMiddleware → WorkspaceAuthContextMiddleware` (:133-141).

### 1.4 Workspace resolution per request

The workspace id comes from the **token payload**, not the host header.

`GraphQLHydrateRequestFromTokenMiddleware` (`srv/engine/middlewares/graphql-hydrate-request-from-token.middleware.ts:14-21`)
calls `MiddlewareService.hydrateGraphqlRequest` (`srv/engine/middlewares/middleware.service.ts:155-186`):

- If no token is present (`isTokenPresent`, :48-58 — JWT or session cookie), set `request.locale`
  from `Accept-Language` and return — an anonymous request reaches the direct-execution hook
  and is refused there (§1.5) (:156-160).
- Otherwise `accessTokenService.validateTokenByRequest(request)`; on failure the dead session
  cookie is cleared (`clearDeadSessionCookie`, :135-153, code set :29-35) and the error is
  **re-thrown, never swallowed** (:166-171).
- Agent tokens are refused on the record path: `if (isDefined(data.agentId))` → `AuthException('This token is only accepted by MCP', FORBIDDEN_EXCEPTION)` (:173-179).
- Seed/read `metadataVersion` (`getOrSeedMetadataVersion`, :188-206: Redis copy wins, else
  `workspace.metadataVersion`, seeding Redis when missing), then `bindDataToRequestObject` (:185).

`hydrateRestRequest` (:118-133) is the same minus the agent check, plus hard requirements:
`data.workspace` must exist and `workspace.databaseSchema` must be non-empty, else
`throw new Error('No data sources found')` (:124-130).

`bindDataToRequestObject` (`srv/engine/utils/bind-data-to-request-object.util.ts`) sets on the
request: `user, apiKey, application, agentId, userWorkspace, workspace, workspaceId,
workspaceMetadataVersion, workspaceMemberId, userWorkspaceId, authProvider, impersonationContext,
tokenType, authenticatedAt, locale`.

The workspace-member lookup that backs `workspaceMemberId` runs through the ORM with a *system*
auth context: `AccessTokenService.resolveAuthContextFromRequest` (`srv/engine/core-modules/auth/token/services/access-token.service.ts:206`) → for the user branch,
`buildSystemAuthContext(workspaceId)` + `getRepositoryWithoutPermissionChecks('workspaceMember')`
and `AuthException('User is not a member of the workspace', FORBIDDEN_EXCEPTION)` when no member
row (`access-token.service.ts:83-102`). Nudge already resolves this in `auth/`.

`WorkspaceAuthContextMiddleware` (`srv/engine/core-modules/auth/middlewares/workspace-auth-context.middleware.ts:12-26`)
builds `WorkspaceAuthContext` from the hydrated request and runs the rest of the request inside
`withWorkspaceAuthContext(authContext, next)` (AsyncLocalStorage). **This is the per-request
scope the record plane later reads with `getWorkspaceAuthContext()`** — Nudge replaces it with
an axum extractor/extension (§11 deviation D1).

Host/subdomain resolution (`resolveWorkspaceAndPublicDomain`,
`srv/engine/core-modules/domain/workspace-domains/services/workspace-domains.service.ts:112`,
used by `getPublicWorkspaceDataByDomain` at `srv/engine/core-modules/workspace/workspace.resolver.ts:569-570`)
is **public-path only** (`getPublicWorkspaceDataByDomain` reads the `Origin` header) and does not
participate in authenticated record requests. Nudge keeps the token-first model; subdomain routing
is a wave 5+ concern (ADR-011 flattened it onto `FRONTEND_URL`).

### 1.5 Direct execution hook — the record entry point

`use-direct-execution.hook.ts` (`srv/engine/api/graphql/direct-execution/hooks/use-direct-execution.hook.ts`):

- No `req.workspace` → `AuthenticationError('Authentication required', { subCode: 'UNAUTHENTICATED' })` (:49).
- Subscription operations bail early (`isSubscriptionOperation`, :89; util `.../utils/is-subscription-operation.util.ts:5`) — see §8.
- Top-level fields are classified into `hasIntrospectionFields / hasWorkspaceFields / hasCoreFields`
  (`classify-top-level-fields.util.ts:11-30`; introspection set = `__schema`, `__type` at :3).
- **Core + workspace fields mixed → error**: `UserInputError('This query cannot be executed as a
  single request. Please split it into separate queries.')` (:106-108). This rule is preserved (§11 D4).
- Core-only → falls through to normal Nest/Yoga. Workspace-only or introspection →
  `DirectExecutionService.execute(req, document, hasIntrospectionFields, hasWorkspaceFields)` (:133-137).

`DirectExecutionService` (`srv/engine/api/graphql/direct-execution/direct-execution.service.ts`):
`execute` (:94-110) runs introspection and workspace execution concurrently
(`Promise.all`); `executeIntrospectionQuery` (:238) → `getOrComputeSchemaSDL` (:248) →
`buildSchema` (:257); `executeWorkspaceQuery` (:131) resolves the workspace resolver name map,
extracts args from the AST, builds a partial ResolveInfo, derives field selections, and calls
`WorkspaceResolverRunnerService.run`. Result shaping: `graphql-format-result-from-selected-fields.util.ts`.

Nudge already knows the gap: HANDOFF.md — "`/graphql` + `/metadata` answers are the wave 4 gap".

### 1.6 REST core controller

`RestApiCoreController` — `@Controller(ApiPath.Rest)` = `/rest`
(`srv/engine/api/rest/core/controllers/rest-api-core.controller.ts:27`), guard stack
`JwtAuthGuard, AccessPolicyGuard({userSession standard/impersonated/playground, apiKey, oauthClient, application, requiredPermissionFlags: []}), CustomPermissionGuard`
(:28-45), filters `RestApiExceptionFilter, AuthRestApiExceptionFilter, PermissionsRestApiExceptionFilter` (:46-50):

| Route | Handler | Status | File:line |
|---|---|---|---|
| `POST /rest/batch/*path` | createMany | 201 | `rest-api-core.controller.ts:55-66` |
| `POST /rest/*path` | createOne | 201 | `:81-92` |
| `POST /rest/*path/duplicates` | findDuplicates | 200 | `:68-79` |
| `GET /rest/*path` | findOne (with id) / findMany | 200 | `:107-118` |
| `GET /rest/*path/groupBy` | groupBy | 200 | `:94-105` |
| `PATCH /rest/*path` | updateOne (with id) / updateMany | 200 | `:159-170` |
| `PUT /rest/*path` | legacy alias of PATCH (undocumented, kept) | 200 | `:173-184` |
| `PATCH /rest/*path/restore` | restore | 200 | `:133-144` |
| `PATCH /rest/*path/merge` | mergeMany | 200 | `:146-157` |
| `DELETE /rest/*path` | delete/destroy | 200 | `:120-131` |

Path → `{object, id?}` via `parseCorePath`
(`srv/engine/api/rest/input-request-parsers/path-parser-utils/parse-core-path.utils.ts:7`);
`batch` (:38), `duplicates`/`groupBy`/`merge` (:43-45) are reserved second segments, `/restore`
last-segment PATCH-only (:15-18). DELETE routes soft vs hard by the `soft_delete` query param
(`RestApiCoreService.delete`, `srv/engine/api/rest/core/services/rest-api-core.service.ts:78-95`:
`soft_delete=false` → destroy, `=true` → soft delete).

Object resolution: `RestApiBaseHandler.getObjectMetadata` (`rest-api-base.handler.ts`) validates the
workspace `metadataVersion` before reading the flat maps (`rest-api-base.handler.ts:181-197`;
missing and not seedable → `BadRequestException('Workspace metadata version not found in database')`).

`/rest/metadata/*` controllers (object/field/view/webhook/apiKey/etc.) exist (enumerated in
`graphql-rest-api.md §4.4`); they are the **metadata** plane and already exist in Nudge wave 3.
Wave 4 does not duplicate them.

### 1.7 OpenAPI surface

`GET /open-api/core` and `GET /rest/open-api/core` (and `/open-api/metadata`,
`/rest/open-api/metadata`) serve the generated OpenAPI document
(`srv/engine/core-modules/open-api/open-api.controller.ts:23-41`). The record-plane OpenAPI is
**generated from metadata at request time** for the workspace. Probed live on the reference the
unauthenticated code is `403 {"statusCode":403,"messages":["Missing authentication token"],"error":"FORBIDDEN_EXCEPTION"}`
(the same guard-failure shape as `/rest`). Scoping decision in §8.6.

---

## 2. The TwentyORM record path

### 2.1 Is there a datasource per request?

Yes — a **fresh, stateless `WorkspaceDataSource` per repository call**, over a **process-wide pool**,
with the flat metadata maps supplied by the workspace cache. There is no per-workspace `DataSource`
object held in a registry.

`WorkspaceOrmManager.executeInWorkspaceContext(fn, authContext?)`
(`srv/engine/twenty-orm/workspace-orm.manager.ts:93-104`) loads an `ORMWorkspaceContext`
into AsyncLocalStorage (`withWorkspaceContext`). `loadWorkspaceContext` (:118-177) does
`workspaceCacheService.getOrRecompute(workspaceId, ['flatObjectMetadataMaps',
'flatFieldMetadataMapsOrm', 'flatFieldMetadataOptionMaps', 'flatIndexMaps', 'featureFlagsMap',
'billingEntitlements'])` (:123-137), computes `objectIdByNameSingular` (:139-140),
resolves `permissionsPrincipals` (:142-145) and `recordReadRestrictions` (:146-150),
and conditionally `flatValidationRuleMaps` behind the enterprise flag (:152-162).
A `lite` variant (`loadLiteWorkspaceContext`, :179-217) skips index maps and permissions
(`flatIndexMaps` = empty, `permissionsPrincipals` = undefined, `recordReadRestrictions` =
`NO_RECORD_READ_RESTRICTIONS`). `executeTransactionInWorkspaceContext` (:106-116) composes
context + transaction.

`getRepository(nameSingular, {useReplica, shouldSkipEventEmission})` (:44-52) →
`workspaceDataSourceService.getDataSource({useReplica})` (:67-71) → `WorkspaceDataSource.getRepository(nameSingular)`.

`WorkspaceDataSourceService.getDataSource`
(`srv/engine/twenty-orm/datasource/workspace-data-source.service.ts:30-38`) reads
`getWorkspaceContext()` (the AsyncLocalStorage context) and constructs a `WorkspaceDataSource`
carrying `pool` (from the shared executor service, :34), `internalContext`
(`buildInternalContext`, :48-70 — workspaceId, the four flat maps, `objectIdByNameSingular`,
feature flags, billing entitlements, permission principals, record read restrictions, event
emitter, record stock, the three record-permission services, the core DataSource) and
`authContext` (:35-37).

`WorkspaceDataSource` (`srv/engine/twenty-orm/datasource/workspace-data-source.ts`):
`getRepository` (:58-71) and `getRepositoryWithoutPermissionChecks` (:73-86) wrap a
`TwentyOrmQueryExecutor → PoolQueryExecutor` over the pool.
`buildRepositoryForObjectMetadataId` (:230-297) resolves the `flatObjectMetadata` by id
(`getFlatObjectMetadataOrThrow`, :336-343), computes `shouldRepositoryBypassPermissionChecks`
= `shouldBypass || isSystemAuthContext(authContext)` (:250-251), and builds the table shape via
`getTableShape` (:299-334). Unknown object name → `TwentyOrmException('Object "<x>" does not exist in this workspace', UNKNOWN_OBJECT)`
(:213-218).

**Table shape is cached** across contexts by a two-level `WeakMap`:
`tableShapeCache = new WeakMap<flatFieldMetadataMaps, WeakMap<flatObjectMetadata, WorkspaceTableShape>>`
(`workspace-data-source.ts:33-36`), populated in `getTableShape` (:304-333). The formula
computed-column shapes are merged in (:321-329). This is the mechanism that avoids rebuilding the
per-object column/relation model on every request — Nudge mirrors it (§11 D6).

`transaction(work)` (:88-175) builds a dedicated `transactionalInternalContext` (:94-125) that
defers event emission, record-stock and access-group work to `afterCommit` callbacks
(:96-124), obtains a **client** (`runInClientTransaction`, :177-193, `runInRollbackSafeTransaction`),
exposes `getRepository`/`getRepositoryWithoutPermissionChecks`/`executeRawQuery`/`afterCommit` in
the scope, and drains the after-commit callbacks after the client transaction resolves (:163-172).

The pool itself lives in `WorkspaceQueryExecutorService`
(`srv/engine/workspace-query-executor/services/workspace-query-executor.service.ts`):
one primary pool created in `onModuleInit` (:42-49) and an optional replica pool (:55-67),
created by `createPool` (:96-135) with `PG_POOL_MAX_CONNECTIONS`, `PG_POOL_IDLE_TIMEOUT_MS`,
`PG_POOL_ALLOW_EXIT_ON_IDLE`, `query_timeout` and a `DATE_ONLY_POOL_TYPES` override (:24-29)
that keeps Postgres `date` values as `'YYYY-MM-DD'` strings (no JS Date coercion).
`getPoolOrThrow({useReplica})` (:76-89) returns primary, falling back to primary when a replica
is requested but not configured (`this.replicaPool ?? this.primaryPool`, :77-79). **So a single
shared pg pool serves every workspace; the schema name disambiguates** (data-model §1).

> **Decision for Nudge:** a single `sqlx::PgPool` (already in `AppState.core_pool`), with the
> workspace schema selected by fully-qualifying every table (`"workspace_<b36>"."<table>"`) rather
> than `SET search_path`, plus a second read-only pool when `PG_DATABASE_REPLICA_URL` is set.
> The `DATE_ONLY_POOL_TYPES` behaviour is reproduced by binding DATE columns as `text`/`chrono::NaiveDate`
> formatted `YYYY-MM-DD` on read. See §11 D2.

### 2.2 Repository → query builder

`WorkspaceRepository` (`srv/engine/twenty-orm/repository/workspace-repository.ts`, 2514 lines)
holds `options` (`WorkspaceRepositoryOptions`, :130-156) with the `tableShape`,
`flatObjectMetadata`, `internalContext`, `authContext`, `executor`, `permissionsPrincipals`,
`shouldBypassPermissionChecks`, `shouldSkipEventEmission`, and callbacks to build sibling
repositories/table shapes (:140-153). Public API with line anchors:

| Method | Line | Notes |
|---|---|---|
| `executeRaw(sql, parameters)` | :184-191 | compiles `:name` params → positional (§2.5) |
| `createQueryBuilder(alias?)` | :193-206 | default alias = `tableShape.nameSingular`; carries `recordReadRestrictions`, `onBeforeExecute`, `formatResult` |
| `find(options)` | :254 | |
| `findOne(options)` | :292 | |
| `count(options)` | :343 | |
| `update(criteria, data)` | :619-634 | routes through `runMutation(kind:'update')` |
| `updateMany(inputs)` | :638-665 | `> QUERY_MAX_RECORDS` → `TOO_MANY_RECORDS_TO_UPDATE`; batch (§2.4) |
| `delete(criteria)` | :668-687 | **hard** delete → `kind:'delete'` |
| `softDelete(criteria)` | :690-701 | `kind:'soft-delete'` |
| `restore(criteria)` | :704-714 | `kind:'restore'` |
| `upsert` | :875 | |
| `runInsert(records, columnsToReturn, onConflictDoNothing?)` | :978-1090 | see §4.3 |
| `runBatchUpdate(inputs, columnsToReturn)` | :1095-1107 | see §4.4 |
| `runMutation(args)` | :1511-1529 | soft/hard/update dispatch, then re-reads computed columns |
| `applyRowLevelPermissionPredicates(qb, opType?)` | :2170 | runs in `onBeforeExecute` |
| `validateQueryIsPermitted(qb)` | :2127 | column-level read check, runs in `onBeforeExecute` |
| `makeRlsRuleFieldsUnreadable(qb)` | :2045 | field masking, runs in `onBeforeExecute` |

`onBeforeExecute` is the hook the select builder calls just before it emits SQL (:193-206 wires
it), and it runs the three permission passes in order: RLS predicates → query-permitted check →
RLS-field-masking (`repository.ts:2037-2041`). This is the single choke point where permissions
enter the read path (§5).

### 2.3 Relation expansion (FK joins compiled to SQL)

The `WorkspaceSelectQueryBuilder` (`srv/engine/twenty-orm/query-builder/workspace-select-query-builder.ts`,
1478 lines) is **not** TypeORM's QueryBuilder — it builds SQL from the table shape. `addJoin`
(:434-530):

- relation path must be `<parentAlias>.<relationField>` (:447-452).
- Resolves the parent table shape (`getTableShapeForAlias`, :528+) and looks up
  `relationShapeByFieldName[relationField]`; unknown → `TwentyOrmException(UNKNOWN_RELATION)` (:476-481).
- The join condition is `"parentAlias"."<joinColumnName>" = "alias"."id"` (:508-511) — a plain FK
  equality against the target `id`. The join column name is `relationShape.joinColumnName`, which
  for a MANY_TO_ONE relation field is `<field>Id` (`computeMorphOrRelationFieldJoinColumnName`;
  data-model §3.5, `workspace-migrations.md §5.5`).
- **To-many** relations (`joinColumnName` undefined) are not joined inline; `buildToManyJoin`
  (:485-493) constructs a de-duplicated lateral form used only when `allowToManyJoin` is set, and
  the find-many runner rejects any row-duplicating join (§3.5). `toManyDedupOrder`,
  `toManyPlainCondition`, `additionalOnConditions` are carried on the join clause (:518-527).

`addRelationExistsFilter({relationFieldName, applyWhere})` (:1146) returns an EXISTS **token**: for
a ONE_TO_MANY sub-filter the builder emits a correlated `EXISTS (SELECT ... WHERE <sub> AND
deletedAt IS NULL)` against the child table instead of a join (`build-select-statement.util.ts:626-660`,
`buildExistsFilterClauseFrom…`), so filtering through a to-many relation never duplicates parent
rows (§3.4). The soft-delete gate for the EXISTS sub-table is `hasDeletedAtColumn` at :651-655.

Nested one-to-many relation *data* (not filters) is **not** joined; it is loaded afterward by
`ProcessNestedRelationsHelper` (`srv/engine/api/common/common-nested-relations-processor/process-nested-relations.helper.ts`):

- one task per selected relation, run with concurrency `NESTED_RELATION_QUERY_MAX_CONCURRENCY = 4`
  (:51, :69-76, `createConcurrencyLimiter`), `Promise.all` over the entries (:94-114).
- ONE_TO_MANY: `findRelations` issues **one batched query per relation** keyed on the parent FK,
  `.take(perParentLimit * parentRecordsCount)` (:278-296, :512) then distributes rows per parent
  (`attachToManyRelationToRecords`, `repository/utils/attach-relations.util.ts:39-68`, a
  single-pass grouping map). MANY_TO_ONE: distinct FK ids collected
  (`collectForeignKeys`, `attach-relations.util.ts:4-11`) and targets loaded by `id IN (...)`
  (`attachToOneRelationToRecords`, :17-37). Per-parent limit
  `perParentLimit = Math.min(relationSelection.first ?? limit, limit)` (:295); default
  `limit = QUERY_MAX_RECORDS_FROM_RELATION = 60` (`shared/src/constants/QueryMaxRecordsFromRelation.ts:1`).
- A target object whose read is disabled reads as empty rather than erroring (:210-232).
- MANY_TO_ONE relation loads include the soft-deleted target and select `deletedAt` (:256-260) so
  the front end can show deleted related records — reproduced verbatim.

**Batching summary for find-many:** 1 query for the root page + 1 aggregate query (§3.6) +
1 EXISTS/join per filter relation (inline) + ≤1 query per *selected* nested relation (batched by
parent, capped at 4 concurrent). This is the N+1 ceiling the Rust design must not exceed (§10).

### 2.4 Flat metadata maps at query time

Record queries consume the **ORM projection of the flat maps**, not raw DB rows:
`flatObjectMetadataMaps`, `flatFieldMetadataMaps` (typed `FlatEntityMaps<OrmFlatFieldMetadata>`),
`flatFieldMetadataOptionMaps`, `flatIndexMaps`. These are the same universal-flat maps Nudge
loaded in wave 3 (`metadata/flat`, `metadata/cache.rs::WorkspaceFlatMapsCache`), keyed by id and
universalIdentifier. At request time the runner resolves a field by `fieldIdByName` /
`fieldIdByJoinColumnName` maps derived from the object's flat fields
(`buildFieldMapsFromFlatObjectMetadata`, used e.g. `common-query-filter-field.parser.ts:81-88`).
Upstream's `WorkspaceCacheService.getOrRecompute` keeps a local LRU + Redis, hash-checked
(`srv/engine/workspace-cache/services/workspace-cache.service.ts`: `MAX_LOCAL_CACHE_ENTRIES = 6_000`
:68, sweep 60 s :71, `getOrRecompute` :224, Redis read :338/:587-603, TTL write :501/:755-757,
pub/sub invalidation `PUBLISH_WORKSPACE_CACHE_SCRIPT` :19/:781). Nudge's `WorkspaceFlatMapsCache`
is the in-process equivalent; wave 4 must key it on `(workspaceId, metadataVersion)` so the wave-3
runner's invalidation (which already bumps `metadataVersion`) naturally retires stale shapes.

### 2.5 Statement compilation

`TwentyOrmQueryExecutor` (`srv/engine/twenty-orm/datasource/twenty-orm-query-executor.ts`) +
`compileTwentyOrmStatement` translate the TypeORM-style `:name`/`:...name` (spread) parameter
syntax into positional `$1..$n` before hitting `pg` (`repository.ts:184-191` calls it). The Rust
design emits sqlx `query_as`/`query!` with the same `:name → $n` rewrite in one place (§10).

---

## 3. Query semantics (read path)

All read operations run through `CommonBaseQueryRunnerService.execute`
(`srv/engine/api/common/common-query-runners/common-base-query-runner.service.ts:130-197`):
consume API speed limit on the root op (:144-146) → `validate` (:148) → parse selected fields
(:150-158) → `processArgs` (= `computeArgs` then pre-query hooks, :252-269) →
`validateQueryComplexity` (:165-169, `COMMON_QUERY_COMPLEXITY_LIMIT = 2000`,
`srv/engine/core-modules/twenty-config/config-variables.ts:2087`) →
`executeInWorkspaceContext(executeQueryAndEnrichResults)` (:181-190) → result getters + visibility +
post-query hooks (`enrichResultsWithGettersAndHooks`, :322-380; post hooks deferred to
`transactionScope.afterCommit` when in a tx, :365-372). Complexity is `1 + relationFieldCount`
(:220-230) and v2 = max fields per record (:232-248); too-nested (one-to-many inside one-to-many)
→ `TOO_COMPLEX_QUERY` (:697-719).

### 3.1 Filter DSL — operators

**Schema-wide operators** (the widest, per-type GraphQL input types):
`srv/engine/api/graphql/workspace-schema-builder/graphql-types/input/` — e.g. `StringFilter`
exposes `eq,gt,gte,in,lt,lte,neq,startsWith,endsWith,like,ilike,regex,iregex,is`
(`string-filter.input-type.ts:20-32`; `regex`/`iregex` at :30-31). `FilterIs` enum (`NULL`,
`NOT_NULL`) in `filter-is.input-type.ts`. Every object `<Obj>FilterInput` also carries
`and: [<Obj>FilterInput]`, `or: [<Obj>FilterInput]`, `not: <Obj>FilterInput`
(`graphql-rest-api.md §2.6`). Full per-input table in `graphql-rest-api.md §2.6`.

**Runtime-allowed operators** are narrower (schema exposes more than the SQL builder accepts).
`srv/engine/api/common/common-args-processors/filter-arg-processor/constants/filter-operators.constant.ts`:

| Field group | Allowed operators | file:line |
|---|---|---|
| STRING (TEXT) | `eq,neq,gt,gte,lt,lte,in,is,like,ilike,startsWith,endsWith` — **no regex/iregex** | `filter-operators.constant.ts:10-23` |
| NUMBER/NUMERIC/POSITION | `eq,neq,gt,gte,lt,lte,in,is` + field-refs | `:25-35` |
| BOOLEAN | `eq,is` | `:37` |
| DATE/DATE_TIME | `eq,neq,gt,gte,lt,lte,in,is` + field-refs | `:39-49` |
| UUID | `eq,neq,gt,gte,lt,lte,in,is` | `:51-60` |
| ARRAY | `containsIlike,is,isEmptyArray` | `:62-66` |
| MULTI_SELECT | `containsAny,is,isEmptyArray` | `:68-72` |
| SELECT/RATING (ENUM) | `eq,neq,gt,gte,lt,lte,in,containsAny,is,isEmptyArray` | `:74-85` |
| RAW_JSON/FILES | `is,like,ilike` | `:87-91` |
| RICH_TEXT | `ilike` | `:93` |
| TS_VECTOR/composites | `eq,neq,is` (plus `search` on TSVector at SQL layer) | `graphql-rest-api.md §2.6` |

**Field-reference operators** `gtField,gteField,ltField,lteField` map to SQL `>,>=,<,<=`
(`shared/src/constants/FieldReferenceFilterOperators.ts:6-11`), numeric/date only, and compare
column-vs-column (`common-query-filter-field.parser.ts:155-171`, `computeFieldReferenceCondition`
at :324; comment at :323: column names come from metadata, so the filter value never reaches SQL).
Composite sub-fields **reject**
field-reference operators (`filter-arg-processor.service.ts:457-468`).

`relation field` filters: filtering by a relation's **name** (e.g. `company: {name: {eq}}`) is
allowed; filtering directly by the relation object is redirected to the join column —
`throwUseJoinColumnInstead` (`filter-arg-processor.service.ts:32-42`, code
`INVALID_ARGS_FILTER`). A bare `<field>Id` key resolves via `fieldIdByJoinColumnName`
(`common-query-filter-field.parser.ts:107-113`).

### 3.2 Filter tree → WHERE

`srv/engine/api/common/common-query-parsers/utils/apply-filter-entries-to-where-expression.util.ts`:

- `and`/`or` → bracketed groups, each element wrapped in its own `Brackets`, joined `andWhere`/
  `orWhere` (`applyLogicalGroup`, :90-116).
- `not` → `NotBrackets` (:67-75).
- anything else → the field parser (:76-86).
- **Empty filter returns the builder untouched** — no `WHERE 1=0` (`common-query-filter-condition.parser.ts:50-51`).

`CommonQueryFilterFieldParser.parse` (`common-query-filter/common-query-filter-field.parser.ts:90-190`):
resolve field by name-or-join-column (:107-113) → `assertFieldIsReadableOrThrow` (:119-123) →
relation sub-filter (:125-141) or composite expansion (:143-152) or scalar operator (:153-189).

### 3.3 Per-operator SQL

`computeWhereConditionParts` (`srv/engine/record-filter/utils/compute-where-condition-parts.ts:62-240`)
is the exact text; every branch was re-read (values bind as parameters, param name =
`removeSqlDDLInjection(key)` + a random 5-byte hex suffix, identifiers escaped):

| op | SQL (`:p` = bound param) | line |
|---|---|---|
| `isEmptyArray` (true) | `(col = '{}' OR col IS NULL)` | :63-69 |
| `isEmptyArray` (false) | `(col IS NOT NULL AND col != '{}')` | :71-74 |
| `eq` (DATE_TIME) | `(col >= :p AND col < :p::timestamptz + interval '1 millisecond')[ OR col IS NULL]` | :75-81 |
| `eq` | `col = :p[ OR col IS NULL]` | :83-86 |
| `neq` (null-equiv) | `(col != :p AND col IS NOT NULL)` | :87-95 |
| `neq` (null) | `col IS NOT NULL` | :97-102 |
| `neq` (DATE_TIME) | `(col < :p OR col >= :p::timestamptz + interval '1 ms' OR col IS NULL)` | :104-109 |
| `neq` | `(col != :p OR col IS NULL)` | :111-114 |
| `gt`/`lt` | `> :p` / `< :p` | :115-126, :132-136 |
| `gt` (DATE_TIME) | `>= :p::timestamptz + interval '1 ms'` | :116-121 |
| `gte`/`lte` | `>= :p` / `<= :p` | :127-131, :137-148 |
| `lte` (DATE_TIME) | `< :p::timestamptz + interval '1 ms'` | :138-142 |
| `in` | `col IN (:...p)` | :149-153 |
| `is` | `col IS NULL`/`IS NOT NULL[ OR col = :p2]` | :154-162 |
| `isStrictly` | `col IS NULL`/`NOT NULL` (no widening) | :165-169 |
| `eqStrict` | `col = :p` (no widening) | :175-179 |
| `gtStrict` | `col > :p` | :170-174 |
| `like`/`ilike` | `col::text LIKE/ILIKE :p[ OR col IS NULL]` | :180-189 |
| `startsWith` | `col::text ^@ :p` | :190-194 |
| `endsWith` | `RIGHT(col::text, LENGTH(:p)) = :p` | :195-199 |
| `contains` | `col @> ARRAY[:...p]` | :200-204 |
| `search` | see §6 | :205-218 |
| `notContains` | `NOT (col::text[] && ARRAY[:...p]::text[])` | :219-223 |
| `containsAny` | `col::text[] && ARRAY[:...p]::text[]` | :224-228 |
| `containsIlike` | `EXISTS (SELECT 1 FROM unnest(col) elem WHERE elem ILIKE :p)` | :229-233 |
| unknown | `RecordFilterException(UNSUPPORTED_OPERATOR)` | :234-239 |

`isStrictly`/`gtStrict`/`eqStrict` are **internal** (used only by keyset cursors, §3.5); they are
not exposed in the schema.

**Null-equivalence widening.** Postgres stores `''` for NULL text and `'{}'` for empty arrays, so
`findPostgresDefaultNullEquivalentValue` (`srv/engine/record-filter/utils/find-postgres-default-null-equivalent-value.util.ts`)
+ `constants/null-equivalent-values.constant.ts:3-9` (`POSTGRES_DEFAULT_TEXT_FIELD_NULL_EQUIVALENT_VALUE=''`,
`POSTGRES_DEFAULT_ARRAY_FIELD_NULL_EQUIVALENT_VALUE='{}'`) make `eq: ''`, `like`, `ilike`,
`is:NOT_NULL` widen to also match `IS NULL`. Composite null-equivalents (per sub-field) are
enumerated at `null-equivalent-values.constant.ts:11-61`. This is a hard parity requirement
(`graphql-rest-api.md §Porting note 5`).

### 3.4 Sort DSL

`OrderByDirection` enum: `AscNullsFirst, AscNullsLast, DescNullsFirst, DescNullsLast`
(`shared/src/types/ObjectRecordGroupBy.ts:4-9`; the GraphQL enum type is
`srv/engine/api/graphql/workspace-schema-builder/graphql-types/enum/order-by-direction.enum-type.ts:7-20`).
Multi-key: `orderBy: [<Obj>OrderByInput]`. Each scalar field maps to a direction; date fields in
the group-by flavour carry `{orderBy, granularity}` (`graphql-rest-api.md §2.7`).

`convertOrderByToFindOptionsOrder` (`common-query-parsers/common-query-order/utils/convert-order-by-to-find-options-order.ts:13-33`)
→ `{order:'ASC'|'DESC', nulls:'NULLS FIRST'|'NULLS LAST'}` via `getEffectiveScanOrder`
(:25-28). Invalid direction → `INVALID_DIRECTION` (:18-22). Text-ish columns get case-insensitive
and cast handling: `shouldUseCaseInsensitiveOrder` = TEXT/SELECT/MULTI_SELECT, `shouldCastToText`
= SELECT/MULTI_SELECT (`common-query-order/utils/build-order-by-column-expression.util.ts:3-20`);
wrapping with `LOWER()`/`::text` happens in the select builder's `getOrderByRawSQL`. Relation
order-by adds a join alias and pushes its columns into the select list
(`addRelationOrderColumnsToBuilder`, must run **after** `setFindOptions` —
`common-find-many-query-runner.service.ts:188-193`). `CommonQueryOrderFieldParser.parse`
(`common-query-order/common-query-order.parser.ts:45-106`) resolves leaves through the shared
`resolveOrderByLeaves`, so ordering and cursors use the same leaf model (:25 comment).

### 3.5 Pagination

Cursor encoding (`srv/engine/api/common/utils/cursors.util.ts`):
`encodeCursorData = base64url(JSON.stringify(cursorData))` (:93-95); `decodeCursor` = base64 →
JSON, invalid → `INVALID_CURSOR` (:24-34); `getCursor(args)` prefers `after`, then `before` (:97-110).
The cursor payload = the ordered leaf values + `id` (:85-88; composite/nested leaf keys are rebuilt
as nested objects, :55-83). `orderByValuesFromScan` (exact SQL scan values, NULLs included) win
over record fields when present (:62) — cursors are built from the raw scan rows
(`buildOrderByValuesByRecordId`, `common-find-many-query-runner.service.ts:201-206`).

`computeCursorArgFilter` (`srv/engine/api/utils/compute-cursor-arg-filter.util.ts`) turns the
cursor into a keyset predicate list, **fail-loud if a leaf value is missing** (:54-61, `INVALID_CURSOR`).
Per leaf: `buildCursorLeafWhereCondition` (`build-cursor-leaf-where-condition.util.ts:23-41`) →
`buildCursorKeysetCondition` (`build-cursor-keyset-condition.util.ts:24-58`): equality uses
`eqStrict`/`isStrictly(NULL)` (:35-39); the ordering condition uses `gtStrict`/`lt` depending on
`isAscending` from `getEffectiveScanOrder`, plus an `or [main, isStrictly NULL]` branch when
nulls sort last and the column can hold null (:50-58). The full cursor filter is merged as
`{ or: [...] }` chained with the leading equality prefixes (`computeCursorArgFilter:64-88`), then
applied with `applyCursorFilterToBuilder` (`common-query-filter-condition.parser.ts:68-87`, which
uses `andWhereReadingUnreadableColumnsAsNull` so a masked order column still continues the scan).
`getEffectiveScanOrder` (`srv/engine/api/utils/get-effective-scan-order.util.ts:11-25`) is the one
function both ORDER BY and the keyset predicate must read from, or they disagree (#24333).

find-many flow (`common-find-many-query-runner.service.ts:59-245`):
clone builder for aggregates (:75) → apply filter + deletedAt to the aggregate clone (:79-88) →
resolve order leaves and append the `id` tie-breaker (`DEFAULT_ID_ORDER_BY_TIEBREAKER = {id:
AscNullsLast}`, `common/constants/default-id-order-by-tiebreaker.constant.ts:3-5`, appended at
:91-101) → `isForwardPagination = !isDefined(before)` (:103) → apply filter (:105-109) → cursor
filter (:111-130) → apply order (:132-137) → apply deletedAt (:139-142) → add aggregates on the
clone (:144-148) → `limit = getFindManyPageSize(args)` and `builder.limit(limit + 1)` (:150, :185) →
`setFindOptions({select})` (:152-167) → **reject to-many join** (`getNonToOneJoinAliases`,
:169-180, `INVALID_QUERY_INPUT` "only to-one relations are supported") → `offset` if provided (:182-184) →
`addRelationOrderColumnsToBuilder` (:188-193) → `getRawAndEntities()` (:196) → `buildCursorPage`
+ `getPageInfo` (:207-223) → aggregate `getRawOne` (:224-226) → nested relations (:228-232).

`buildCursorPage` (`srv/engine/api/utils/build-cursor-page.util.ts:7-33`): `hasMoreItems =
fetched.length > limit`, trim to `limit`, reverse for backward pages (:23-25),
`hasNextPage = backward ? hasBeforeCursor : hasMoreItems`, `hasPreviousPage = backward ?
hasMoreItems : hasAfterCursor` (:29-32). `getPageInfo` (`common/utils/get-page-info.util.ts`)
sets `startCursor`/`endCursor` from the first/last record's encoded cursor (:40-47).

**Arg validation** (`common-find-many-query-runner.service.ts:299-345`): both `first`+`last`,
both `before`+`after`, `before`+`first`, `after`+`last` → `ARGS_CONFLICT`; negative `first`/`last`
→ `INVALID_ARGS_FIRST`/`INVALID_ARGS_LAST`. Note the conflict guards use truthiness, and the page
size uses nullish coalescing (`first ?? last ?? QUERY_MAX_RECORDS`), so `first: 0` is legal and
returns an empty page whose `hasNextPage` reflects the sentinel row.

### 3.6 Limit / page size

`getFindManyPageSize` (`common-query-runners/utils/get-find-many-page-size.util.ts:3-16`):
`pageSize = first ?? last ?? QUERY_MAX_RECORDS`, capped to `min(pageSize, QUERY_MAX_RECORDS)`
unless `shouldBypassQueryMaxRecords`. `QUERY_MAX_RECORDS = 200`
(`shared/src/constants/QueryMaxRecords.ts:1`). The builder fetches `limit + 1` sentinel rows (:185).

### 3.7 findOne by id, findDuplicates

`findOne` (`common-find-one-query-runner.service.ts:32-92`): filter is **required** — empty/missing →
`INVALID_QUERY_INPUT` ("Missing filter argument", :115-126); apply filter + deletedAt (:49-58),
`getOne()` (:71), not found → `RECORD_NOT_FOUND` (:73-81), then nested relations (:85-89). The GraphQL
`findOne` field takes `filter` (non-null), not `id`; the REST `GET /rest/<obj>/<id>` maps to the
same runner via `RestApiFindOneHandler`.

`findDuplicates` (`common-find-duplicates-query-runner.service.ts`) — gated on
`objectMetadata.duplicateCriteria`; per input record it builds duplicate conditions
(`build-duplicate-conditions.util.ts`), runs a matching query `.take(QUERY_MAX_RECORDS)` with a
`getCount()` aggregate (:123-141), returns one connection per record ordered `{id: AscNullsFirst}`
(:144) with `hasNextPage/hasPreviousPage:false` (:155-158). **Deferred (§11 D10)** — wave 4 ships
the resolver slot and an empty result, full criteria matching in a follow-on unit.

### 3.8 groupBy

`CommonGroupByQueryRunnerService` (`common-group-by-query-runner.service.ts:86-186`). Group-by
SQL expressions: `getGroupByDefinitions` (`utils/get-group-by-definitions.util.ts:9-59`) resolves
the physical column (composite sub-column via `formatColumnNamesFromCompositeFieldAndSubfields`;
relation via the join alias). `getGroupByExpression` (`utils/get-group-by-expression.util.ts:24-117`):

| Case | SQL | line |
|---|---|---|
| scalar / non-date | the column expression | :34-39 |
| `shouldUnnest` (multi-select) | `UNNEST(CASE WHEN CARDINALITY(col)>0 THEN col ELSE ARRAY[col[1]] END)` | :34-36 |
| DAY_OF_THE_WEEK | `TRIM(TO_CHAR(col,'TMDay'))` | :90-91 |
| MONTH_OF_THE_YEAR | `TRIM(TO_CHAR(col,'TMMonth'))` | :92-93 |
| QUARTER_OF_THE_YEAR | `TRIM(TO_CHAR(col,'"Q"Q'))` | :94-95 |
| WEEK | `TO_CHAR(DATE_TRUNC('week', col [tz]) [AT TIME ZONE tz], 'YYYY-MM-DD')`, with +1d/+2d shift for SUNDAY/SATURAY week start | :96-109 |
| DAY/MONTH/QUARTER/YEAR | `TO_CHAR(DATE_TRUNC('<g>', col [tz]) [AT TIME ZONE tz], 'YYYY-MM-DD')` | :110-114 |
| NONE | column | :88-89 |

Time-zone requirement: granularities in `GROUP_BY_DATE_GRANULARITY_THAT_REQUIRE_TIME_ZONE` on a
DATE_TIME field without a tz → `MISSING_TIMEZONE_FOR_DATE_GROUP_BY` (:54-60); invalid IANA tz →
`INVALID_TIMEZONE` (:62-72); deprecated aliases normalized (:74-77). Relation group-by joins a
`leftJoin` on the join column (`addJoinForGroupByOnRelationFields`,
`common-group-by-query-runner.service.ts:420-467`, join at :450-458). The result is `getRawMany()`
limited by `getGroupLimit` (`resolveWithoutRecords`, :396-423); default group count
`DEFAULT_NUMBER_OF_GROUPS_LIMIT = 50`
(`shared/src/constants/DefaultNumberOfGroupsLimit.ts:1`, `get-group-limit.util.ts:3-13`).
`orderByForRecords`/`includeRecords` path uses `GroupByWithRecordsService` with
`RECORDS_PER_GROUP_LIMIT=10`, `RELATIONS_PER_RECORD_LIMIT=5`, and a `sub_query` window
(`group-by/services/group-by-with-records.constants.ts:1-5`).

### 3.9 Aggregates

`ProcessAggregateHelper.getAggregateExpression`
(`srv/engine/api/common/utils/process-aggregate.helper.ts:37-103`) — exact SQL; the concatenated
non-null column is `NULLIF(CONCAT(col1,col2,...), '')` (:70-74):

| op | SQL | line |
|---|---|---|
| COUNT_EMPTY | `CASE WHEN COUNT(*)=0 THEN NULL ELSE COUNT(*)-COUNT(expr) END` | :77-78 |
| COUNT_NOT_EMPTY | `CASE WHEN COUNT(*)=0 THEN NULL ELSE COUNT(expr) END` | :79-80 |
| COUNT_UNIQUE_VALUES | `CASE WHEN COUNT(*)=0 THEN NULL ELSE COUNT(DISTINCT expr) END` | :81-82 |
| PERCENTAGE_EMPTY | `CASE WHEN COUNT(*)=0 THEN NULL ELSE CAST((COUNT(*)-COUNT(expr)::decimal)/COUNT(*) AS DECIMAL) END` | :83-84 |
| PERCENTAGE_NOT_EMPTY | `CASE WHEN COUNT(*)=0 THEN NULL ELSE CAST(COUNT(expr)::decimal/COUNT(*) AS DECIMAL) END` | :85-86 |
| COUNT_TRUE | `... COUNT(CASE WHEN expr::boolean = TRUE THEN 1 END)` | :87-88 |
| COUNT_FALSE | `... COUNT(CASE WHEN expr::boolean = FALSE THEN 1 END)` | :90-91 |
| MIN/MAX/AVG/SUM (RATING) | `<op>(CAST(SPLIT_PART(col::text,'_',2) AS INTEGER))` | :95-100 |
| MIN/MAX/AVG/SUM | `<op>(col)` on the numeric sub-column | :100 |

Aggregation field *names* are generated by `getAvailableAggregationsFromObjectFields`
(`srv/engine/api/graphql/workspace-schema-builder/utils/get-available-aggregations-from-object-fields.util.ts`):
`countUniqueValues<Field>`, `countEmpty<Field>`, `countNotEmpty<Field>`, `percentageEmpty<Field>`,
`percentageNotEmpty<Field>` (:32-75); date kinds add `min/max<Field>` (:77-90); BOOLEAN adds
`countTrue/countFalse<Field>`; NUMBER/RATING numeric add `min/max/avg/sum`; CURRENCY adds
`<op><Field>AmountMicros` (uses `subFieldForNumericOperation='amountMicros'`, :200-210); and a
global `totalCount` with `aggregateOperation=COUNT` over
`FIELD_FOR_TOTAL_COUNT_AGGREGATE_OPERATION` (:217-224). **`totalCount` is computed on the
aggregate clone**, only when aggregates were requested (`common-find-many-query-runner.service.ts:224-226`).
The REST find-many handler always injects `aggregates:['totalCount']`
(`rest/core/handlers/rest-api-find-many.handler.ts:55`, response `totalCount` at :97).

### 3.10 Result formatting

Rows are re-assembled from physical columns back to the logical record shape:
`formatResult` (`srv/engine/twenty-orm/utils/format-result.util.ts`) reconstructs composites from
`<fieldName><CapitalizedSubField>` columns (:159-184) and maps null-equivalent `''`/`'{}'` back to
`null` (`format-composite-field-value.util.ts`). On write the inverse `formatData`
(`srv/engine/twenty-orm/utils/format-data.util.ts:18-100`) explodes composites into columns
(:81-100). These two are the read/write mirror of the composite flattening in
`workspace-migrations.md §5.4` and must be ported symmetrically (§11 D8).

---

## 4. Record mutations

### 4.1 Method set and semantics

Query methods `findMany,findOne,findDuplicates,groupBy`; mutation methods
`createMany,createOne,updateOne,deleteOne,updateMany,deleteMany,destroyOne,destroyMany,
restoreOne,restoreMany,mergeMany` (`srv/engine/api/graphql/workspace-resolver-builder/constants/resolver-method-names.ts:1-17`,
set in `.../constants/workspace-resolver-builder-method-names.ts:3-23`). Dispatch table in
`WorkspaceResolverRunnerService` (`srv/engine/api/graphql/direct-execution/services/workspace-resolver-runner.service.ts:138-330`).

- `createOne` runs through `CommonCreateManyQueryRunnerService` with `data:[data]`
  (`common-create-one-query-runner.service.ts:37-51`), so create-one and create-many share all
  insert/upsert/position logic.
- `updateOne` runs `updateMany.runOnOneRecord` (`common-update-one-query-runner.service.ts:36-57`);
  `computeArgs` processes data with `shouldBackfillPositionIfUndefined:false` (:79); not-found →
  `RECORD_NOT_FOUND` (:46-54); validate remote-object + UUID id (:85-92).
- `updateMany` uses `runFilteredMutation(kind:'update', data)`
  (`common-update-many-query-runner.service.ts:33-59`).
- delete/destroy/restore one and many all extend `CommonFilteredMutationManyQueryRunnerService`
  with `mutationKind` = `'soft-delete'` (delete), `'delete'` (destroy), `'restore'`
  (`common-delete-many-query-runner.service.ts:9`, `common-destroy-many-query-runner.service.ts:9`,
  `common-restore-many-query-runner.service.ts:9`). The many runners require a non-empty filter →
  `INVALID_ARGS_FILTER` (`common-base-query-runner.service.ts` via
  `common-filtered-mutation-many-query-runner.service.ts:42-47`), and one-variants call
  `runOnOneRecord` with `filter:{id:{eq}}, shouldWriteAtLeastOneRecord:true` (:50-66).
  `CommonFilteredMutationOneQueryRunnerService.run` throws `RECORD_NOT_FOUND` on empty result
  (`common-filtered-mutation-one-query-runner.service.ts:49-57`).

**Soft delete** = `UPDATE ... SET deletedAt = CURRENT_TIMESTAMP, updatedAt = CURRENT_TIMESTAMP`
(`query-builder/workspace-mutation-query-builder.ts:144-152`, the `updatedAt` maintenance appended at
:198-214; the `deletedAt IS NULL` predicate is auto-added so already-trashed rows are skipped unless
`withDeleted()` — `sql/utils/build-mutation-statement.util.ts:65-78`). **Destroy** = `DELETE FROM ...
[WHERE ...] RETURNING ...` (:87-95), releases record stock (`performMutation`,
repository.ts:1793). **Restore** = `UPDATE ... SET deletedAt = NULL, updatedAt =
CURRENT_TIMESTAMP` (mutation builder :155-161). Soft-delete/restore on an object with no
`deletedAt` column → `TwentyOrmException(UNSUPPORTED_OPERATION)` (:216-223).

**Upsert** = `createMany`/`createOne` with `upsert:true`. Path in
`common-create-many-query-runner.service.ts`:
`> QUERY_MAX_RECORDS` → `TOO_MANY_RECORDS_TO_UPDATE` (:116-124);
`getConflictingFields(flatObjectMetadata, flatFieldMetadataMaps, flatIndexMaps)` derives the
conflict column groups from unique indexes (`common-create-many-query-runner/utils/get-conflicting-fields.util.ts`,
composite `isIncludedInUniqueConstraint` sub-fields at :69-77, relation join column at :35-39);
`findExistingRecords` builds OR-of-conditions and queries `.withDeleted()` (:397-454);
`categorizeRecords` splits into update vs insert (`categorize-records.util.ts:8-34`, call at :316);
a record matching more than one existing row → `UPSERT_MULTIPLE_MATCHING_RECORDS_CONFLICT`
(`get-matching-record-id.util.ts:84`); updates go through `runBatchUpdate` **with `deletedAt:null`
injected** and `createdBy` stripped (`processRecordsToUpdate`, :459-507, esp. :471-487); inserts go
through `runInsert` (`processRecordsToInsert`, :513-551). Position backfill for inserts runs via
`backfillPositionForInserts` (:365-395); `computeArgs` sets
`shouldBackfillPositionIfUndefined = !args.upsert` (:198).

### 4.2 Actor fields (`createdBy` / `updatedBy`)

Injected by **pre-query hooks** before the runner touches data
(`srv/engine/core-modules/actor/query-hooks/`), registered as `@WorkspaceQueryHook('*.createOne'|
'*.createMany'|'*.updateOne'|'*.updateMany')` (`created-by.create-one.pre-query-hook.ts:18`,
`updated-by.update-one.pre-query-hook.ts:18`). The hooks are merged into the args in `processArgs`
(`common-base-query-runner.service.ts:252-269` → `executePreQueryHooks` :261-266).

`ActorFromAuthContextService.injectActorFieldsOnCreate` stamps `createdBy` then `updatedBy`
(`actor-from-auth-context.service.ts:40-58`); `injectUpdatedBy` stamps only `updatedBy` (:60-71).
Field-existence is checked against the object's `fieldIdByName` — objects without the field skip
injection (:109-111). Merge rule `injectActorToRecord` (:124-141): on create, `createdBy` is set
only if absent or nameless, preserving the caller's `source`; on update, `updatedBy` is overwritten
unconditionally. Actor shape from the auth context
(`build-actor-metadata-from-auth-context.util.ts:13-45`): user → `buildCreatedByFromFullNameMetadata`
producing `{workspaceMemberId, source:'MANUAL', name:"<firstName> <lastName>", context:{}}`
(`build-created-by-from-full-name-metadata.util.ts:12-21`); api-key / application / agent variants
dispatch on the auth context kind (:23-40). `createdBy`/`updatedBy` are composite ACTOR columns
(`workspace-migrations.md §5.4`, `data-model.md §3.x`).

### 4.3 Insert

`WorkspaceRepository.runInsert` (`repository.ts:978-1090`): strips undefined like TypeORM
(`formatWriteData`, :218-241), computes the files-field diff (deferred to wave 6, :1012-1043),
`buildInsertRows` → column names/rows/params/insertedColumns (:1033-1044),
`validateWriteIsPermitted('create', updatedColumns)` (:1037-1042), RLS
`assertCreatedRecordsMatchRowLevelPermissionsOrThrow` (:1044-1054), `assertRecordStockAvailable`
(:1054), `buildInsertStatement` (:1058-1064), execute, `acquireRecordStock`,
`recomputeRecordAccessGroupIdsAfterWrite`, `emitCreateEvents` (:1066-1086).
`buildInsertStatement` (`sql/utils/build-insert-statement.util.ts:29-63`) emits
`INSERT INTO schema.table (cols) VALUES (... DEFAULT/param ...) [ON CONFLICT DO NOTHING] RETURNING cols`;
a row column with no value becomes the literal `DEFAULT` (:47-51) — so the `id` default
(`gen_random_uuid()`) fills in when not supplied. Client-supplied `id` is bound as a parameter.

### 4.4 Batch update (upsert-update)

`runBatchUpdate`/`performBatchUpdate` (`repository.ts:1095-1230`): `resolveWritableRecordIds`
filters by write permission (:1124-1131); per-input before-image snapshot (:1145-1163);
files-field diff (:1169-1186); **distinct column-signature grouping** so the write-permission
check runs once per unique set of updated columns (:1196-1208); RLS re-validation (:1210-1217);
then one UPDATE per input via `runMutation` (:1219-1230). This is a loop, not a single statement —
an upstream perf cost (§10).

### 4.5 Position field

`RecordPositionService.overridePositionOnRecords`
(`srv/engine/core-modules/record-position/services/record-position.service.ts:53-154`), invoked
from `DataArgProcessorService.process` (:108-115) for every create and from the upsert insert path
(`backfillPositionForInserts`). Rules (only when the object has a `position` field, :73-77):

- `position === 'first'` → assigned `minPosition - index - 1` (:116-130);
- `position === 'last'` → `maxPosition + index + 1` (:132-146);
- numeric → kept (:96-98, `validateOverriddenPositionFieldOrThrow`,
  `data-arg-processor.service.ts:219-220`);
- `undefined` and `shouldBackfillPositionIfUndefined` → treated as first (:86-90);
- else untouched (:91-92);
- input order is preserved by returning first-bucket, last-bucket, numeric-bucket, untouched (:148-153);
- min/max read via `repository.minimum/maximum` on a system-auth context `getRepositoryWithoutPermissionChecks`
  (:197-237); empty table → 1 (`buildRecordPosition:38-50`).

The POSITION column is `float8` (`workspace-migrations.md §5.1`), so positions are floats and the
`min-1`/`max+1` arithmetic can go negative or fractional — reproduced exactly (no re-numbering).

### 4.6 Relation handling on writes (connect / disconnect / create)

Nested write inputs `<Obj>RelationInput { connect: <Obj>ConnectInput{where}, create:
<Obj>CreateInput, disconnect: Boolean }` (`workspace-schema-builder/graphql-type-generators/input-types/relation-connect-gql-input-type.generator.ts:69-84`,
`WhereUniqueInput` keyed by unique-constraint fields at :115, :187; keywords
`shared/src/constants/RelationNestedQueriesKeyword.ts`). Resolution runs in the runner before the
write: `resolveNestedRelationsForCreate` (create path,
`common-create-many-query-runner.service.ts:591`, called at :250, :490, :533) / `resolveNestedRelations`
(update path, `common-base-query-runner.service.ts:506`, called at :471), implemented in
`RelationNestedQueries` (`srv/engine/twenty-orm/field-operations/relation-nested-queries/relation-nested-queries.ts`).
Both `create` and a raw join-column value on the same field → conflict error
(`relation-nested-queries.ts:240` "Cannot provide both <field>.create and <field>Id"); nested
creates are executed with a caller-supplied `createRecords` (:265-296). Depth cap
`MAX_NESTED_RELATION_CREATE_DEPTH = 5` (`common-create-many-query-runner.service.ts:56`); a nested
create forces a wrapping transaction (`common-create-many-query-runner.service.ts:87-114`). Record
sharing on create is `shareWith` (`createOne`/`createMany` arg), Enterprise — deferred (§5.3).

---

## 5. Permission enforcement in the query pipeline

### 5.1 Where the hooks run

The **single entry point** is `WorkspaceRepository.onBeforeExecute` → three ordered passes
(`repository.ts:2037-2041`):
1. `applyRowLevelPermissionPredicates(qb)` (:2170-2214): adds a row-access condition per alias —
   main alias (:2178-2182) plus each join alias read-only (:2184-2200), then computed-column table
   sources (:2202+). Bypassed entirely when `shouldBypassPermissionChecks` (:2174-2176).
2. `validateQueryIsPermitted(qb)` (:2121-2160): for each alias's referenced columns, `isReadDisabled`
   short-circuits, else `assertCanRunOperationOrThrow('read', selectedColumns)` (column-level read check).
3. `makeRlsRuleFieldsUnreadable(qb)` (:2043-2119): builds unreadable-field SQL conditions and
   masks columns/relations when the condition holds
   (`queryBuilder.makeColumnsUnreadableWhen` / `makeRelationsUnreadableWhen`); a masked order
   column reads as NULL via `andWhereReadingUnreadableColumnsAsNull` so cursor continuation is safe.

Write-side permission gates (not in `onBeforeExecute`, called explicitly):
`validateWriteIsPermitted('create'|'update'|'destroy'|'softDelete', updatedColumns)` before the SQL
(`repository.ts:1037-1042` for insert, :1669-1674 for mutation), RLS re-validation
`assertUpdatesKeepRecordAccessOrThrow` after computing the update (`performMutation`, :1753-1768,
call site :1757-1768), `assertCreatedRecordsMatchRowLevelPermissionsOrThrow` on create (:1044-1054), and
`shouldWriteAtLeastOneRecord` → `RECORD_NOT_EDITABLE` when a filtered mutation writes 0 rows but a
readable record matched (`runMutationOnStoredColumns`, :1593-1626).

`buildAccessSQLCondition` (`srv/engine/permissions/internal/combine/utils/build-access-sql-condition.util.ts:21-64`)
is the row-access predicate (Enterprise code): `(allRecords OR rls) AND NOT isPrivate` combined OR-wise
with record-share / private-record / related-record-path conditions (:42-63). `resolveRowAccessOutcome`
(:29-33) short-circuits fully-open/fully-closed cases; `buildRlsAccessSQLCondition` handles the
row-level rules. The conditions are attached via `addRowAccessCondition` (`query-builder:264`) and
rendered **outside the user WHERE**: `renderWhereExpressionWithRowAccess`
(`sql/utils/build-select-statement.util.ts:560-586`) emits `(userExpr) AND (accessCond)`, so an OR
in the user filter cannot escape the row gate (:558-559 comment).

### 5.2 `_isPrivate` and `_accessGroupIds` in WHERE

- `_isPrivate` (boolean, column name constant
  `srv/engine/permissions/private-record/constants/is-private-column-name.constant.ts:4`):
  predicate `alias._isPrivate = TRUE|FALSE`
  (`internal/private-record/utils/build-private-record-state-sql-condition.util.ts:13-16`). Enterprise
  columns are added to every object table at creation (`workspace-migrations.md §5.9`). A record is
  private by default for objects in `PRIVATE_BY_DEFAULT_OBJECT_UNIVERSAL_IDENTIFIERS`
  (`permissions/private-record/utils/is-private-by-default-object.util.ts:6-10`).
- `_accessGroupIds` (`integer[]`, constant
  `internal/record-permission-repository/constants/access-group-column-names.constant.ts:2`):
  predicate `alias._accessGroupIds && CAST(:p AS integer[])`
  (`internal/access-group/utils/build-principals-access-groups-sql-condition.util.ts:18-22`). The
  access-group ids per record are maintained off-record writes and recomputed after every write
  (`recomputeRecordAccessGroupIdsAfterWrite`, `repository.ts:1931-1953`, deferred ops
  `compute_recordAccessGroupIds` per `workspace-migrations.md §4.5`).

`assertCanRunOperationOrThrow` and `buildReadRecordsAccessSQLCondition` live in
`srv/engine/permissions/record-permissions-read.service.ts` (:324-368, :372-428); field masking in
`record-field-permissions-read.service.ts`; both are wired through `internalContext.recordPermissions`
/ `recordFieldPermissions` (`datasource/workspace-data-source.service.ts:64-65`).

### 5.3 What wave 4 reproduces vs defers

MAP.md §wave-2 explicitly moves permission *enforcement* to wave 4 "with the query runner". The
non-enterprise, universally-required baseline wave 4 must reproduce:

- **Object-role gate** (`validateWriteIsPermitted` / read equivalent) driven by role→object
  permissions (the `rolePermissionFlag`/`objectPermission` model from wave 2 `auth/`). This is
  `PermissionsException(PERMISSION_DENIED)` (`srv/engine/permissions/permissions.exception.ts:9`).
- **Field-level masking for hidden fields** — the object `isActive`/`isSystem` and field-level
  permission-flag model (read/write on system objects, workspaceMember write deny via query hooks).
- **The query-hook contract** (`*.createOne/createMany/updateOne/updateMany` actor stamping;
  `workspaceMember.*` deny) — `graphql-rest-api.md §2.15`, ported in wave 4.
- The `onBeforeExecute` **slot** with a no-op row-access policy so the SQL shape `(userWhere) AND
  (<rowAccess>)` is already correct for the later permission wave.

Deferred to the **permission wave** (documented deviation, §11 D9): all `@license Enterprise` code —
`_isPrivate`/`_accessGroupIds` row predicates, row-level permission predicates (RLS rules), record
shares / `shareWith`, related-record-path access extension, access-group recompute deferral ops,
and the `assertCreatedRecordsMatchRowLevelPermissionsOrThrow`/`assertUpdatesKeepRecordAccessOrThrow`
calls. Until that wave, no row-access condition is emitted at all (the open case: empty
`rowAccessConditions` renders the bare user expression, `build-select-statement.util.ts:577-579`;
the fully-closed analogue is `ALWAYS_FALSE_CONDITION = '1=0'` at `repository.ts:100`). Nudge's
default Admin/Member roles from wave 2
grant full access, so the observable behavior is identical to upstream for those roles — only
custom RLS/sharing is unavailable until the permission wave.

---

## 6. Search & tsvector

**Two distinct search paths.**

### 6.1 The per-field `search` operator (record DSL)

`TSVectorFilter.search` on a `searchVector` column
(`compute-where-condition-parts.ts:205-218`) emits:

```sql
(col @@ to_tsquery('simple', public.unaccent_immutable(:pTs))
 OR public.unaccent_immutable(col::text) ILIKE public.unaccent_immutable(:pLike))
```

with `:pTs = formatSearchTerms(value,'and')` and `:pLike = '%value%'` (:206, :213-216).
`formatSearchTerms` (`srv/engine/core-modules/search/utils/format-search-terms.ts:1-16`) splits on
whitespace, escapes `[\\:'&|!()@<>]`, appends `:*` per term, and joins with `&` (and) — so
`search` is a **prefix-AND** tsquery plus a substring ILIKE fallback (accent-insensitive both sides
via `public.unaccent_immutable`, the IMMUTABLE wrapper pinned in `MAP.md`/`data-model.md`). The
`searchVector` column and its GIN index are created in wave 3 (`workspace-migrations.md §7`).
The record DSL `search` does **not** rank — it is a boolean predicate for `filter:
{searchVector: {search: "..."}}`.

### 6.2 The command-menu `search` query (core `/graphql`, ranked)

`Search` resolver is a **core** operation, not a workspace one: `@CoreResolver()`
`srv/engine/core-modules/search/search.resolver.ts:23`, `search()` at :55 → `SearchService`
(`srv/engine/core-modules/search/services/search.service.ts`, 879 lines) over
`SEARCH_VECTOR_FIELD` per object (:298-360):

```
ts_rank_cd(searchVector, to_tsquery('simple', public.unaccent_immutable(:searchTerms)))        -- :302
ts_rank (searchVector, to_tsquery('simple', public.unaccent_immutable(:searchTermsOr)))         -- :304
match:  searchVector @@ to_tsquery(..., :searchTerms) OR @@ to_tsquery(..., :searchTermsOr)     -- :335-339
ORDER BY tsRankCD DESC, tsRank DESC, id ASC NULLS FIRST                                         -- :356-358
```

`searchTerms` = AND prefix query, `searchTermsOr` = OR (broader recall). Empty search terms →
`searchVector IS NOT NULL` branch (:340-344). Cursor continuation over rank is
`computeCursorWhereCondition` (:306-310). This is the "web-search-like ranking": tsrank_cd + tsrank,
no custom scoring model. **Scoping (§8.6):** the per-object `search` operator is in wave 4 (it is
part of the record DSL and free once the SQL builder exists); the cross-object ranked `Search`
command-menu query is a **core** query and is deferred to the search wave (README roadmap puts
search in wave 6) — documented in §11 D10 as a deferred unit, not a placeholder.

---

## 7. Wire formats

### 7.1 GraphQL schema naming

Generated per workspace from metadata. Operation names come from `getResolverName`
(`srv/engine/utils/get-resolver-name.util.ts:8-53`) — the single source of truth:
`findMany → camelCase(namePlural)` (:13-14, e.g. `people`/`companies`), `findOne → camelCase(nameSingular)`
(:15-16), `findDuplicates → singular + 'Duplicates'` (:17-18), `create${Pascal(singular)}`/
`create${Pascal(plural)}` (:20-23), `update…` (:25-28), `delete…` (:30-33), `destroy…` (:35-38),
`restore…` (:40-43), `merge${Pascal(plural)}` (:45-46), `groupBy → camelCase(namePlural)+'GroupBy'`
(:48-49). Full operation-name table already in `graphql-rest-api.md §2.3`.

Output types (`workspace-schema-builder/graphql-type-generators/object-types/`):
`<PascalSingular>` plain object (relation fields contribute `<field>Id` join column; ONE_TO_MANY
omitted here), `<PascalSingular>Edge{node,cursor}`, `<PascalSingular>Connection` with edges +
`pageInfo: PageInfo!` (`connection-gql-object-type.generator.ts:74`, :82) + the aggregation fields
of §3.9, `<PascalSingular>GroupByConnection` adds `groupByDimensionValues: JSON!`, and the "with
relations" object type extends the plain type with relation fields (target's `Plain` for M2O,
target's `Connection` with `findMany` args for O2M) — full detail `graphql-rest-api.md §2.5`.
`PageInfo{startCursor,endCursor,hasNextPage!,hasPreviousPage!}`
(`graphql-types/object/page-into.object-type.ts:8-10`).
Root args per method (`workspace-schema-builder/utils/get-resolver-args.util.ts:19-54` `findMany`:
`first,last,offset,before,after,filter,orderBy`; :54 `findOne`: `filter`; `:138-209` groupBy args).
Input type names, filter/order-by/create/update inputs and scalars: `graphql-rest-api.md §2.6-2.11`.
Introspection SDL is cached per `(workspace, metadataCacheHash, applicationId)`
(`workspace-graphql-schema-sdl.service.ts:45-175`, deps `SCHEMA_SDL_CACHE_DEPENDENCIES`).

### 7.2 REST paths + OpenAPI

REST resource path is the **plural** name: `/rest/<namePlural>[/<id>]`, with reserved second
segments (`parse-core-path.utils.ts:7-45`); query params `limit, starting_after, ending_before,
depth(0|1), fields, order_by, filter, soft_delete, upsert, viewId, group_by, aggregate,
include_records_sample` (`graphql-rest-api.md §4.2`; default `limit=QUERY_DEFAULT_LIMIT_RECORDS=60`
capped to 200, `limit-parser-utils/parse-limit-rest-request.util.ts:13-16`, `:29`;
`depth` allowed `0|1`, `depth-parser-utils/parse-depth-rest-request.util.ts:18`). The REST filter
DSL grammar `and(...)/or(...)/not(...)` with leaf `field[op]:value` and `FilterComparators`
(`eq,neq,in,containsAny,is,gt,gte,lt,lte,startsWith,endsWith,like,ilike`,
`filter-parser-utils/parse-base-filter.util.ts:3-14`) is a **string parser** over the same
operator set as GraphQL (`graphql-rest-api.md §4.2`).

Response shape (`rest-api-base.handler.ts:33-46` `FormatResult`, `:84-105`): find-many
`{ data:{<plural>:[...]}, totalCount, pageInfo }` (`rest-api-find-many.handler.ts:84-99`), find-one
`{ data:{<singular>:{...}} }`, create/update/delete/restore/merge keyed `create<Singular|Plural>`
etc. (`graphql-rest-api.md §4.3`); group-by returns the raw array (`:747`); records trimmed to
selected fields keeping `id` (`pick-rest-response-fields.util.ts`). Content type is JSON; a
`csv`/`text/plain` record content-negotiation was searched for in the core REST handlers and is
**not present** on the record path (import/export is a separate wave-6 surface) — Nudge serves JSON only.

**OpenAPI** documents are generated at request time from metadata:
`GET /open-api/core`, `/open-api/metadata` and their `/rest/open-api/*` aliases
(`open-api.controller.ts:23-41`). Nudge wave 4 defers OpenAPI generation (§11 D11) — the reference
returns `403` unauthenticated (verified live), and there is no front-end dependency on it in the P0
operation list (`graphql-rest-api.md §6`).

### 7.3 Error envelope — record plane vs `/metadata`

GraphQL (record plane): auth-hydration failures are written as **HTTP 200** with `{"errors":[...]}`
(`middleware.service.ts:103-116`, `writeGraphqlResponseOnExceptionCaught`). Domain errors map to a
`GraphQLError` with `extensions.code` from `ErrorCode`
(`srv/engine/core-modules/graphql/utils/graphql-errors.util.ts:21-38`) and `extensions.subCode` =
the domain exception code, plus `userFriendlyMessage` (`graphql-rest-api.md §5.1`).
`RECORD_NOT_FOUND → NotFoundError`; arg/validation → `UserInputError`; `INVALID_AUTH_CONTEXT →
AuthenticationError`; `MISSING_FLAT_INDEX_MAPS/MISSING_SYSTEM_FIELD/INTERNAL_SERVER_ERROR →
InternalServerError` (`common-query-runner-to-graphql-api-exception-handler.util.ts:18-...`).
`CommonQueryRunnerExceptionCode` full list: `srv/engine/api/common/common-query-runners/errors/common-query-runner.exception.ts:5-27`
(plus `MAX_DEPTH_REACHED, INVALID_POST_HOOK_PAYLOAD, RELATION_SETTINGS_NOT_FOUND,
RELATION_TARGET_OBJECT_METADATA_NOT_FOUND` :26-29).

REST: `{ statusCode, error, messages:[...], code }`
(`graphql-rest-api.md §5.2`; `http-exception-handler.service.ts:124-128`, `error` = exception
class name or derived from status at :31-54). Domain→HTTP: `RECORD_NOT_FOUND → 404`,
`INVALID_AUTH_CONTEXT → 401`, arg/validation → 400, internal → 500
(`common-query-runner-to-rest-api-exception-handler.util.ts`, mapping in `graphql-rest-api.md §5.2`).

**`/metadata` difference:** the metadata plane wraps resolver exceptions with
`WorkspaceMigrationGraphqlApiExceptionInterceptor`/`FlatEntityMapsGraphqlApiExceptionFilter` and
carries `METADATA_VALIDATION_FAILED` (`ErrorCode:34`) and the metadata-domain codes
(`workspace-migrations.md §9.5`) rather than the record `CommonQueryRunnerExceptionCode`. Nudge
keeps them separate: record plane raises `QueryError{code: CommonQueryRunnerExceptionCode…}`,
metadata plane keeps the wave-3 `metadata::error` codes.

---

## 8. Streaming / realtime — scope decision

### 8.1 What exists upstream

- **No GraphQL `Subscription` on the record plane.** `GqlOperation` defines Subscription
  (`workspace-schema-builder/enums/gql-operation.enum.ts`) but the workspace schema generator
  builds Query+Mutation only (`workspace-graphql-schema.factory.ts`), and `use-direct-execution.hook.ts`
  bails on subscription operations (`isSubscriptionOperation`, :89). Confirmed in `graphql-rest-api.md §1.7`.
- **Realtime is SSE over `/metadata`.** `srv/engine/subscriptions/event-stream.resolver.ts`
  (`@MetadataResolver` :9) exposes `onEventSubscription(eventStreamId)` (:76-90), plus
  `addQueryToEventStream` (:217-219) / `removeQueryFromEventStream` (:264) mutations. Payload =
  `{ objectRecordEventsWithQueryIds:[{objectRecordEvent:{action, objectNameSingular, recordId,
  userId, workspaceMemberId, properties{updatedFields,before,after,diff}}, queryIds}], metadataEvents, queueJobEvents }`
  (`graphql-rest-api.md §3.4`). Transport is Redis pub/sub
  (`srv/engine/subscriptions/subscription.service.ts`: `getPubSubClient` :43/:61, `publish` :77-79,
  `publishToEventStream` :94-96).
- **Event emission from the ORM.** `DatabaseEventAction` enum: `CREATED, UPDATED, DELETED,
  DESTROYED, RESTORED, UPSERTED` (import `repository.ts:21`; `MUTATION_EVENT_ACTIONS_BY_KIND`
  at `repository.ts:102-111`: delete→DESTROYED, restore→RESTORED, soft-delete→DELETED,
  update→[UPDATED, UPSERTED]). `emitCreateEvents`/`emitMutationEvent` run inside the repository
  writes (:1083, :1848-1851), routed through `WorkspaceEventEmitter`
  (`srv/engine/workspace-event-emitter/workspace-event-emitter.ts`) and batched by
  `entity-events-to-db.listener.ts` to the SSE stream. In a transaction these are deferred to
  `afterCommit` (`workspace-data-source.ts:96-103`).
- **Cache-hint invalidation.** `metadataVersion` bumps (`workspace-metadata-version.service.ts`,
  wired in wave 3) tell clients to re-fetch metadata; the front end's `collectionHashes`
  (`minimal-metadata`) let it detect record-set changes cheaply. **There is no HTTP ETag /
  `Cache-Control` / Last-Modified on the record `/graphql` or `/rest` path** — searched
  `setHeader`/`Cache-Control` under `api/rest/core` and `api/graphql` and found none (the SSE
  event stream carries the freshness signal instead). `metadataVersion` is a Redis-stored
  server-side check, not a response header (`middleware.service.ts:188-206`,
  `rest-api-base.handler.ts:181-197`).

### 8.2 What Nudge wave 4 implements

- The **event-emission seam**: the write path builds the correct `DatabaseEventAction` per
  mutation kind (`repository.ts:103-109`) and calls an `emit_database_batch_event(...)` that, in
  wave 4, is a **no-op recorder behind a trait** (`EventEmitter` trait with a `NullEmitter` default,
  mirroring the `shouldSkipEventEmission` flag at `repository.ts:143-146`). This keeps the write
  SQL and ordering identical while leaving delivery to a later wave.
- The `afterCommit` deferral structure (events, record-stock, access-group work run after the tx,
  `workspace-data-source.ts:92-172`) — implemented as a callback list drained after commit (§11 D5).

### 8.3 What Nudge wave 4 defers (with justification)

- **SSE `onEventSubscription` + `/metadata` event stream** → **deferred to wave 6** (README puts
  realtime/webhooks/search in wave 6). Justification: it is a `/metadata`-mounted core resolver,
  not part of the record data plane; the front-end record caches tolerate its absence (they refetch
  on navigation); it needs the Redis pub/sub fan-out + `queueJobEvents`/`metadataEvents` payloads
  that ride on the webhook/worker wave. Building the emission seam now (8.2) means wave 6 only adds
  a transport, not a pipeline change.
- **Redis cache invalidation pub/sub** → deferred; Nudge's `WorkspaceFlatMapsCache` invalidates
  in-process on the wave-3 runner's `metadataVersion` bump (already landed). Single-binary
  deployments don't need cross-node pub/sub until horizontal scaling.
- **Webhook delivery on record events** → wave 6 (already the roadmap; needs the `webhook` metadata +
  HMAC sender).

---

## 9. Performance notes: where upstream is slow and what the Rust design does differently

| Upstream cost | file:line | Rust design |
|---|---|---|
| New `WorkspaceDataSource` + internal-context object allocated on **every** repository call | `workspace-data-source.service.ts:30-38` | Reuse one `PgPool`; build a lightweight per-request `QueryContext` (flat-map handle + auth + restrictions) — no per-call object graph. |
| Table shape rebuilt + WeakMap-chained; formula columns merged each miss | `workspace-data-source.ts:33-36, 299-333` | Cache table shapes keyed `(workspaceId, metadataVersion, objectId)` in an LRU (moka) with the same natural retirement as the flat-map cache; compute once per metadata version (§11 D6). |
| `randomBytes` per filter param suffix (crypto) | `compute-where-condition-parts.ts:42-45` | Use a monotonic per-query integer suffix (sqlx named→positional rewrite); identical bind behavior, no crypto per operator. |
| `runBatchUpdate` = N sequential UPDATE round-trips (per input) | `repository.ts:1145-1230` | Same observable semantics (per-record UPDATE + RETURNING, before-image snapshot) but pipelined / multi-statement in one connection checkout; keep the `distinctUpdatedColumns` permission grouping (:1196-1208). |
| find-many runs a **separate aggregate query** for totalCount/aggregates | `common-find-many-query-runner.service.ts:75, 144-148, 224-226` | Keep the second query only when aggregates are requested (REST always requests `totalCount`); consider a `COUNT(*) OVER ()` window when only `totalCount` is needed, but only after parity tests confirm identical `COUNT(*)=0→NULL` behavior (§3.9). |
| `processNestedRelations` spawns one query per selected relation (≤4 concurrent) | `process-nested-relations.helper.ts:51, 94-114` | Batch to-one loads into a single `WHERE id = ANY($1)`; keep O2M as one batched, per-parent-capped query (already batched); tune `NESTED_RELATION_QUERY_MAX_CONCURRENCY` via semaphore. |
| `WorkspaceCacheService.getOrRecompute` per operation reloads the 6 flat-map keys + hashes + Redis round-trip | `workspace-cache.service.ts:224, 338, 587-603` | Serve from the in-process `WorkspaceFlatMapsCache` (already loaded); Redis only on cold miss; avoid re-hash per read. |
| Per-request AsyncLocalStorage context (Node ALS overhead) | `workspace-orm.manager.ts:103` | Pass `QueryContext` explicitly as axum request extensions / function args — no CLS. |
| `structuredClone` of flat maps in the migration runner and actor injection | `actor-from-auth-context.service.ts:113`, `workspace-migrations.md §4.2` | Actor fields are built by value (Rust owns the record map) — no clone of the whole map, only the one field. |

**Non-negotiable parity constraints** that limit the above: (a) `totalCount`/aggregate NULL-on-empty
rule (§3.9) must survive any window-function rewrite; (b) the keyset predicate and ORDER BY must both
come from `getEffectiveScanOrder` (§3.5) or pagination breaks; (c) `buildCursorPage` must fetch
`limit+1` and reverse for backward pages (§3.5); (d) null-equivalence widening (§3.3) is load-bearing
for filter results; (e) row-access conditions render outside the user WHERE (§5.1). The Rust design
optimizes allocation and round-trips, never observable SQL semantics.

---

## 10. Module layout (crates/server/src, following the existing split)

```
crates/server/src/query/
  mod.rs               re-exports; Operation enum {FindMany,FindOne,CreateOne,...}
  context.rs           QueryContext {auth, workspace, flat_map handle, metadataVersion,
                       row_access, record_read_restrictions}
  entry/
    graphql.rs         axum handler for POST/GET /graphql: body parse, hydrate (§1.4), classify
                       top-level fields (§1.5), route core vs workspace vs introspection
    rest.rs            /rest router: path parse, query-param parse, dispatch (§1.6)
  schema/
    name.rs            getResolverName port (§7.1), camelCase/pascal from metadata::names
    sdl.rs             SDL generator from flat maps (introspection target); cached by metadataVersion
    resolver_map.rs    resolver-name→(object,method) map (graphQLResolverNameMap analogue)
  runner/
    base.rs            execute() pipeline: validate→args→complexity→ctx→run→getters/visibility/hooks (§3)
    find_many.rs       §3.5
    find_one.rs        §3.7
    group_by.rs        §3.8
    find_duplicates.rs §3.7 (stub first, full later — D10)
    create.rs          create-one/many + upsert (§4.1, §4.3, §4.4)
    update.rs          update-one/many (§4.1)
    delete.rs          delete/destroy/restore (§4.1)
    merge.rs           merge-many (§4.1)
  filter/
    model.rs           Filter tree type (and/or/not/leaf), operator enum (§3.1)
    sql.rs             compute_where_condition_parts port (§3.3)
    null_equiv.rs      §3.3
    arg_processor.rs   runtime operator allowlist per type + relation redirect (§3.1)
  order/
    mod.rs             OrderByDirection, resolve leaves, convert via getEffectiveScanOrder (§3.4)
  cursor/
    mod.rs             encode/decode base64url, buildCursorPage, keyset condition (§3.5)
  aggregate/
    mod.rs             ProcessAggregateHelper SQL + field-name generation (§3.9)
  relations/
    join.rs            addJoin/EXISTS compilation (§2.3)
    nested.rs          ProcessNestedRelationsHelper batching (§2.3)
  repository/
    mod.rs             WorkspaceRepository analogue: run_insert/run_mutation/batch_update (§2.2, §4)
    sql.rs             build_insert_statement / build_mutation_statement / build_select_statement (§2.3)
    table_shape.rs     build_workspace_table_shape + cache (§2.1)
    format.rs          format_data / format_result composite mirror (§3.10)
  compile.rs           :name / :...name → $n rewrite (§2.5)
  hooks/
    mod.rs             pre/post query hook registry (§4.2 actor stamping; workspaceMember deny)
  search_vector.rs     reuses workspace_migrations::tsvector; the `search` operator (§6.1)
  error.rs             QueryError + codes; GraphQL/REST mapping (§7.3)
```

Reuse, not re-add: `metadata::flat` + `metadata::cache` for maps (§2.4); `auth` for token→context
(§1.4); `db` pool; `workspace_migrations::tsvector` (§6); `metadata::names` for casing.

---

## 11. Documented decisions and deviations

Upstream behavior is followed unless stated. Deviations are deliberate and localized.

- **D1 — No AsyncLocalStorage.** The workspace auth context (hydrated request) is threaded to the
  runner as an axum extension/parameter (`QueryContext`), not via a Node ALS. Behaviorally identical
  for a single request; simpler and testable. Entry point §1.4.
- **D2 — Single shared pool, schema-qualified SQL.** One `PgPool` (plus a replica pool when
  `PG_DATABASE_REPLICA_URL` is set) serves all workspaces; every record query fully qualifies
  `"workspace_<b36>"."table"` (already the model from wave 3 `metadata::names` +
  `workspace_migrations`) instead of `SET search_path` per datasource. Mirrors §2.1 (upstream also
  uses one pool; Nudge just avoids the per-call DataSource object).
- **D3 — DATE returned as `YYYY-MM-DD` string** on the read path (reproduces
  `DATE_ONLY_POOL_TYPES`, `workspace-query-executor.service.ts:24-29`), not a JS Date → ISO with time.
  Achieved by reading DATE columns as text/`NaiveDate` and serializing `YYYY-MM-DD`.
- **D4 — Mix core+workspace fields → error.** Kept verbatim (`use-direct-execution.hook.ts:106-108`)
  so the existing front-end contract holds; Nudge's `/graphql` core schema is the auth/client-config
  resolvers already ported, workspace fields resolve to the record runners.
- **D5 — afterCommit callbacks deferred.** Events, record-stock, access-group work run after commit
  via a callback list (`workspace-data-source.ts:92-172`). In wave 4 the emitter is a null recorder
  (§8.2), but the *transaction* deferral structure is implemented so the record-stock accounting (used
  for limits) still lands post-commit.
- **D6 — Table-shape cache keyed on metadataVersion.** Upstream's `WeakMap<flatFieldMetadataMaps, …>`
  (per-context, process-scoped) is replaced by `(workspaceId, metadataVersion, objectId) → shape`
  in an LRU with the same content per version; a metadata change bumps the version and misses
  naturally. Semantically identical shape; the retention policy differs (bounded LRU vs. GC-driven).
- **D7 — No response cache headers.** Upstream sets none on the record plane (§8.1); Nudge matches —
  no ETag/`Cache-Control`/`Last-Modified`. This is conformance, not a deviation, but is stated so no
  implementer invents caching headers.
- **D8 — Composite read/write symmetry.** `formatData`/`formatResult` are ported as the exact inverse
  pair over the `<field><CapitalizedSubField>` columns + null-equivalence (§3.10, §4); the flattening
  rules come from `workspace-migrations.md §5.4` so DDL and query layers agree.
- **D9 — Enterprise permission features deferred.** `_isPrivate`, `_accessGroupIds` predicates, RLS
  rules, record shares/`shareWith`, related-record-path extension, and their write assertions are
  gated to a **permission wave**. Until then the row-access condition list stays empty (no-op, §5.1) and
  Admin/Member roles observe full access (§5.3). Deviation from *default-enabled-on-enterprise*
  upstream only where the enterprise code path is on; the OSS default behavior is preserved.
- **D10 — `findDuplicates` and cross-object `Search` stubbed first.** find-many/one/mutations are the
  P0 set; `findDuplicates` returns empty connections in the initial slice and `Search` (command-menu)
  is the wave-6 search surface (§6.2). Both have concrete defaults, not TBDs: `findDuplicates` → an
  empty array of `{records:[],totalCount:0,hasNextPage:false,hasPreviousPage:false,cursors:null}`
  (`common-find-duplicates-query-runner.service.ts:112-120` shape) when `duplicateCriteria` is unset
  (all objects until the criteria engine lands); the resolver slot exists so the front-end
  `personDuplicates` query parses. Full matching is a follow-on unit.
- **D11 — OpenAPI generation deferred.** `/open-api/*` returns the same `403` unauthenticated and is
  out of the P0 front-end path (§7.2); Nudge adds it after the record API is green.

---

## 12. Wave 4 implementation units

Ordered so each unit builds and verifies independently against the live reference
(`twenty-ref` on `127.0.0.1:3001`, DB `twenty_ref` via
`docker exec infra-db-1 psql -U postgres -d twenty_ref`; Nudge on `:3000`; parity harness
`rust/tools/parity-compare.py`). Style follows `workspace-migrations.md §11`.

| # | Unit | Files | Contents | Acceptance criteria |
|---|---|---|---|---|
| U1 | `query::filter` + `query::order` + `query::cursor` + `query::compile` (pure) | `query/filter/*`, `query/order/*`, `query/cursor/*`, `query/compile.rs` | Operator allowlist per type (§3.1), tree→WHERE walk (§3.2), `compute_where_condition_parts` verbatim (§3.3) incl. null-equivalence + DATE_TIME `[ts,ts+1ms)`, `getEffectiveScanOrder`+direction→`{order,nulls}` (§3.4), keyset cursor encode/decode + `buildCursorPage` (§3.5), `:name→$n` rewrite (§2.5) | unit tests: every operator in §3.3 emits the byte-equal SQL fragment quoted (captured from a reference `getQuery()`); null-equivalence widening cases; cursor round-trip base64url JSON leaf+id; keyset for asc/desc × nulls-first/last × forward/backward matches `getEffectiveScanOrder`; `:id[in]` spread → `IN ($1,$2,..)`; no DB needed |
| U2 | `query::repository::table_shape` + `query::repository::sql` | `table_shape.rs`, `sql.rs` | `build_workspace_table_shape` from flat maps (columns, composite sub-columns, relation shapes + join column names) (§2.1); `build_insert_statement` (§4.3), `build_mutation_statement` for update/soft-delete/delete/restore incl. `deletedAt IS NULL` auto-predicate + `updatedAt` maintenance (§4.1); LRU table-shape cache keyed `(wsId, metadataVersion, objectId)` | integration test vs `infra-db-1`: emitted SQL for a `person`/`company` insert & soft-delete matches a captured reference statement; every column name resolves (composite `<f><Sub>` + relation `<f>Id`); soft-delete on a `deletedAt`-less object → `UNSUPPORTED_OPERATION`; `updateMany>QUERY_MAX_RECORDS`→`TOO_MANY_RECORDS_TO_UPDATE` |
| U3 | `query::runner::find_many` + `find_one` + `query::relations::join` + `nested` | `find_many.rs`, `find_one.rs`, `relations/*`, `runner/base.rs` (execute pipeline, complexity) | §3.5, §3.7, §2.3 join/EXISTS compilation, batched nested relation load (≤4 concurrent, per-parent cap 60) (§2.3), `applyDeletedAtToBuilder` (§3.5), result getters + format_result (§3.10) | parity test (`parity-compare.py`, same request both servers, JSON-normalized per ADR-005): `people(first:2){edges{node{id name}cursor} pageInfo{...} totalCount}` returns identical keys/values; `filter:{company:{name:{eq}}}`, `orderBy`, `after`/`before`, `first/last/before/after` conflict errors (ARGS_CONFLICT codes equal), to-many filter/order rejection (INVALID_QUERY_INPUT), limit +1 sentinel trimming, backward-page reversal |
| U4 | `query::aggregate` + `query::runner::group_by` | `aggregate/mod.rs`, `group_by.rs` | Aggregate SQL expressions + field names (§3.9); group-by definitions + granularity SQL (§3.8); `totalCount` on the connection | parity: `companies{ countNotEmptyName minCreatedAt }`; `companiesGroupBy(groupBy:[{industry:true}], limit)` group keys + per-group `totalCount`/aggregates match; timezone error codes (`MISSING_TIMEZONE_FOR_DATE_GROUP_BY`, `INVALID_TIMEZONE`); default group limit 50 |
| U5 | `query::runner::create/update/delete` + `query::hooks` (actors, position) + upsert | `create.rs`, `update.rs`, `delete.rs`, `hooks/*`, `search_vector.rs`, `repository/mod.rs` (runInsert/runMutation/batchUpdate) | §4.1–§4.6: mutation kinds, soft/hard/restore/upsert conflict-group detection, `position` service (§4.5), actor stamping hooks (§4.2), composite `formatData` (§3.10), `search` operator SQL (§6.1) | parity: `createPerson` returns record with `createdBy/updatedBy` matching reference shape + `position` backfill; `updatePerson`, `deletePerson` (soft, `deletedAt` set, `updatedAt` bumped), `destroyPerson` (row gone), `restorePerson`; `createPeople(data:[..], upsert:true)` matches insert-or-update and `UPSERT_MULTIPLE_MATCHING_RECORDS_CONFLICT`; `searchVector:{search:"…"}` matches the reference's matching set; `RECORD_NOT_FOUND` on one-mutations of a missing id |
| U6 | `query::entry::graphql` + `query::schema` (SDL, resolver map) + `query::context` | `entry/graphql.rs`, `schema/*`, `context.rs`, workspace resolution §1.4, direct-exec classification §1.5 | GraphQL body parse (json + `application/graphql` + GET query param), hydrate/auth/CSRF reuse, classify core vs workspace vs introspection, execute via runners, format result from selections (§2.14 of `graphql-rest-api.md`), introspection SDL from `schema/sdl.rs`; core+workspace mix → error; agent token refused | parity: unauthenticated `POST /graphql {__typename}` → HTTP 200 `{"errors":[{...UNAUTHENTICATED...}]}` (verified reference shape §7.3); introspection `__schema` returns the workspace SDL whose type/field names match `get-resolver-name.util.ts`; a valid authenticated `people` query equals the reference; core+workspace mix rejected with `BAD_USER_INPUT` |
| U7 | `query::entry::rest` | `entry/rest.rs` | `/rest/*path` routes + `parseCorePath` + all query-param parsers (§1.6, §7.2), response envelope `FormatResult` (§7.2), REST filter DSL string parser (`and/or/not`, `field[op]:value`), error→HTTP mapping (§7.3), `soft_delete`/`upsert`/`fields`/`order_by` params | parity: `GET /rest/people?limit=2&filter=name[ilike]:%a%&order_by=name[AscNullsLast]&starting_after=<cursor>` equals reference (data+pageInfo+totalCount shape); `POST /rest/companies` 201 + `data.createCompany`; `DELETE /rest/companies/<id>?soft_delete=false` → destroy; `GET /rest/people/groupBy`; error codes: unknown object `400 object 'x' not found`, bad limit `INVALID_LIMIT_QUERY_PARAM`, bulk-delete empty filter `400`; `RECORD_NOT_FOUND→404`, `INVALID_AUTH_CONTEXT→401` |

**Dependency order:** U1 → U2 → U3 is the critical path (pure SQL model → shapes/statements → the
first runner). U4 depends on U3. U5 depends on U2 (and U1 for filters). U6 depends on U3+U4+U5
(it routes to the runners) but can stub introspection early. U7 depends on U5+U6 (shares the
runners; REST is a thin adapter, `graphql-rest-api.md §Porting note 9`). U6 and U7 can proceed in
parallel once U3+U5 land. Five parallel engineers map to: {U1,U2}, {U3}, {U4}, {U5}, {U6,U7} with
U6/U7 sequenced behind U3/U5.

**Parity harness extension** (add to `rust/tools/parity-compare.py`): the current `CASES` are
bootstrap-level (:45-52). Wave 4 adds a `RECORD_CASES` list of `(method, path, headers(token), body)`
that (1) authenticates against both servers using a seeded workspace token (reuse the `seed-ref`
login flow from MAP.md toolchain), (2) runs the same GraphQL/REST record queries, and (3)
normalizes before diffing: UUID→`<uuid>`, JWT→`<jwt>`, ISO/epoch timestamps→`<iso>`/`<epoch>`
(already in `normalize`, :20-26), plus **new**: cursor strings→`<cursor>` (base64url payloads differ
by record id), `metadataVersion`→`<mv>`, and float normalization for aggregates (`percentage*`
values compared to 6 dp). Key-order-insensitive JSON comparison per ADR-005. Record parity asserts
on the normalized `data` payload and `extensions.code`/`statusCode`, message text best-effort
(ADR-005).

---

## 13. Open risks

- **Cursor ↔ scan-order coupling** (§3.5): any Rust optimization that changes `ORDER BY` without
  updating the keyset predicate (or vice versa) silently skips/duplicates rows (#24333). Mitigation:
  one `effective_scan_order` function shared by both, plus a dedicated pagination-walk e2e that
  exhausts a >200-row table forward and backward and asserts the union equals the full set with no
  dupes (parity vs reference).
- **Composite null-equivalence widening** (§3.3, §3.10): easy to drop; a filter test matrix over
  `''`/`'{}'`/`NULL` per type, diffed against the reference, is part of U1 and U3 gates.
- **Enterprise permission surface (§5.3, D9)**: the row-access slot must already wrap the user
  WHERE correctly so the permission wave is additive, not a rewrite. U3/U5 tests assert the
  `(userWhere) AND (<rowAccess>)` shape even while the access condition is empty.
- **`findDuplicates`/`Search` deferral (D10)**: the resolver/type names must still exist so the
  front-end generated documents parse; tests assert empty-but-well-formed connections, not 404s.
