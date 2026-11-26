# Twenty CRM — GraphQL & REST API (port research)

Reference source (read-only): `C:\Users\tonminh123\Documents\crm-research\twenty`
Target: Rust server (axum + async-graphql dynamic schema + sqlx).

All paths below are relative to the Twenty repo root. Server code lives in
`packages/twenty-server/src`, shared types in `packages/twenty-shared/src`,
front-end in `packages/twenty-front`.

---

## 1. Server boot

### 1.1 Entry point and HTTP surface

`packages/twenty-server/src/main.ts`

- NestJS `NestFactory.create<NestExpressApplication>(AppModule, { rawBody: true, forceCloseConnections: true, snapshot: NODE_ENV === 'development' })`, optional TLS via `SSL_KEY_PATH`/`SSL_CERT_PATH`.
- `app.set('trust proxy', ...)`, credentialed CORS (`applyCredentialedCors`), `express-session` with `getSessionStorageOptions`.
- Body parsers: `json`, `urlencoded`, `text/plain` all limited by `settings.maxRequestBodySize`.
- `graphqlUploadExpress` mounted on `/${ApiPath.GraphQL}` and `/${ApiPath.Metadata}` (max 10 files).
- `keepAliveTimeout = SERVER_KEEP_ALIVE_TIMEOUT_MS`; `headersTimeout = keepAliveTimeout + 1000`.
- Graceful shutdown on SIGTERM/SIGINT: drain HTTP server → flush usage + event recorders → `app.close()`.

`packages/twenty-shared/src/types/ApiPath.ts` — the full path enum (nginx ingress routes these prefixes to the server):

```
AdminPanel='admin-panel', App='app', ApplicationRegistrationClaim='application-registration-claim',
Apps='apps', Auth='auth', ClientConfig='client-config', Cloudflare='cloudflare', Emailing='emailing',
File='file', FileUpload='file-upload', Files='files', GraphQL='graphql', Health='healthz',
Mcp='mcp', Metadata='metadata', OAuth='oauth', OpenApi='open-api', PublicAssets='public-assets',
Rest='rest', RouteTrigger='s', Webhooks='webhooks', WellKnown='.well-known'
```

### 1.2 Root module and middleware chain

`packages/twenty-server/src/app.module.ts`

Modules imported: `GraphQLModule.forRootAsync` (Yoga driver, config from `GraphQLConfigService`), `TwentyOrmModule`, `ClickHouseModule`, `CoreEngineModule`, `ModulesModule`, `WorkspaceCacheStorageModule`, `MetadataGraphQLApiModule`, `AdminPanelGraphQLApiModule`, `RestApiModule`, `McpModule`, `MiddlewareModule`, `JwtModule`, `UserSessionModule`, `WorkspaceMetadataVersionModule`, `FrontendModule`.

Global providers:
- `APP_FILTER` → `UnhandledExceptionFilter` (`src/filters/unhandled-exception.filter.ts`)
- `APP_GUARD` → `WorkspaceNotSuspendedGuard` (`src/engine/guards/workspace-not-suspended.guard.ts`)

Middleware order (Nest `MiddlewareConsumer`), executed in declaration order:

1. `ApiAccessLogMiddleware` on every `ApiPath` except `healthz`, both `/path` and `/path/*path` — `src/engine/middlewares/api-access-log.middleware.ts`. No-op unless `API_ACCESS_LOG_ENABLED`; logs one line on response `finish`/`close`.
2. `CookieSessionCsrfMiddleware` on `*path` (all methods) — `src/engine/middlewares/cookie-session-csrf.middleware.ts`. Excluded for `POST /auth/saml/callback/:identityProviderId`. Rules: safe methods (GET/HEAD/OPTIONS) pass; a request carrying an `Authorization` JWT passes (Bearer is never sent cross-site); cookie-authenticated unsafe requests must pass `isRequestOriginAllowed`, else `403 { statusCode, messages:['Request origin is not allowed for cookie-authenticated requests'], error:'CSRF_ORIGIN_MISMATCH' }`.
3. `/graphql` (ALL): `ApiRequestContextMiddleware` → `GraphQLHydrateRequestFromTokenMiddleware` → `GraphQLRefuseSuspendedWorkspaceMiddleware` → `WorkspaceAuthContextMiddleware`.
4. `/metadata` (ALL): `GraphQLHydrateRequestFromTokenMiddleware` → `WorkspaceAuthContextMiddleware`.
5. `/admin-panel` (ALL): same two as `/metadata`.
6. `/mcp` (ALL): `ApiRequestContextMiddleware` → `McpMethodGuardMiddleware`.
7. `/rest/*path` for DELETE, POST, PATCH, PUT, GET: `ApiRequestContextMiddleware` → `RestCoreMiddleware` → `WorkspaceAuthContextMiddleware`.

`src/engine/middlewares/middleware.module.ts` wires `MiddlewareService`, `CookieSessionCsrfMiddleware`, `ApiAccessLogMiddleware`.

### 1.3 Guards

`packages/twenty-server/src/engine/guards/`

| guard | purpose |
|---|---|
| `jwt-auth.guard.ts` | JWT verification (`JwtAuthStrategy`) |
| `access-policy.guard.ts` | declarative `AccessPolicyGuard({ authPrincipals: {...}, requiredPermissionFlags: [...] })` |
| `custom-permission.guard.ts`, `no-permission.guard.ts` | object/field-level permission flags |
| `admin-panel.guard.ts`, `admin-panel-or-impersonate.guard.ts`, `server-level-impersonate.guard.ts`, `impersonate-permission.guard.ts` | admin panel + impersonation |
| `application-target.guard.ts` | application-owned vs workspace-owned entities |
| `billing-enabled.guard.ts`, `billing-disabled.guard.ts` | billing gating |
| `public-endpoint.guard.ts` | unauthenticated public endpoints |
| `workspace-not-suspended.guard.ts` | global `APP_GUARD` |

The REST core controller guard stack (`src/engine/api/rest/core/controllers/rest-api-core.controller.ts`):

```ts
@UseGuards(
  JwtAuthGuard,
  AccessPolicyGuard({
    authPrincipals: {
      userSession: { standard: true, impersonated: true, playground: true, workspaceAgnostic: false },
      apiKey: true, oauthClient: true, application: true,
    },
    requiredPermissionFlags: [],
  }),
  CustomPermissionGuard,
)
@UseFilters(RestApiExceptionFilter, AuthRestApiExceptionFilter, PermissionsRestApiExceptionFilter)
```

### 1.4 Request context and workspace resolution

`src/engine/middlewares/middleware.service.ts`

- `isTokenPresent(request)`: JWT from request OR session cookie (`JwtWrapperService.extractJwtFromRequest`, `UserSessionCookieService.extractSessionTokenFromRequest`).
- `hydrateGraphqlRequest(request)`: if no token → set `request.locale` only (anonymous). Otherwise `accessTokenService.validateTokenByRequest(request)`; on error the dead session cookie is cleared and the error is re-thrown (never swallowed — otherwise the caller gets "Cannot query field" instead of an auth error). If `data.agentId` is present the request is refused with `AuthException('This token is only accepted by MCP', FORBIDDEN_EXCEPTION)`.
- `hydrateRestRequest(request)`: same but requires `data.workspace` and a non-empty `workspace.databaseSchema`, else `throw new Error('No data sources found')`.
- Both then seed/read the workspace **metadata version** from `WorkspaceCacheStorageService.getMetadataVersion(workspace.id)` (fallback: `workspace.metadataVersion`) and call `bindDataToRequestObject(data, request, metadataVersion)`.

`src/engine/utils/bind-data-to-request-object.util.ts` sets on `request`:
`user`, `apiKey`, `application`, `agentId`, `userWorkspace`, `workspace`, `workspaceId`, `workspaceMetadataVersion`, `workspaceMemberId`, `workspaceMember`, `userWorkspaceId`, `authProvider`, `impersonationContext`, `tokenType`, `authenticatedAt`, `locale` (user workspace locale, else `Accept-Language`).

`src/engine/core-modules/auth/middlewares/workspace-auth-context.middleware.ts` builds a `WorkspaceAuthContext` via `buildWorkspaceAuthContextFromRequest(req)`, applies Sentry context, and runs the rest of the request inside `withWorkspaceAuthContext(authContext, next)` (AsyncLocalStorage). This is what `getWorkspaceAuthContext()` reads later in the ORM layer.

`src/engine/core-modules/auth/token/services/access-token.service.ts`
- `validateTokenByRequest(request)` → `resolveAuthContextFromRequest` → JWT verify → `JwtAuthStrategy.validate(decoded)` → `RawAuthContext` `{ user, workspace, userWorkspace, workspaceMemberId, apiKey?, application?, agentId?, impersonationContext?, tokenType, authenticatedAt }`.
- Workspace resolution is from the **token payload** (`workspaceId`), not from the host header; `workspaceMemberId` is resolved through the workspace ORM (`buildSystemAuthContext(workspaceId)` + `getRepositoryWithoutPermissionChecks('workspaceMember')`), throwing `AuthException('User is not a member of the workspace', FORBIDDEN_EXCEPTION)` when the user has no member row.
- `generateAccessToken` payload: `{ sub, userId, workspaceId, workspaceMemberId, userWorkspaceId, type: 'ACCESS', authProvider, isImpersonating, impersonatorUserWorkspaceId, impersonatedUserWorkspaceId }`; `ACCESS_TOKEN_EXPIRES_IN` config drives expiry. `generatePlaygroundToken` uses `type: 'PLAYGROUND'`.

**Host-based workspace resolution** exists but only for the *public* path, not for authenticated data calls:
`src/engine/core-modules/domain/workspace-domains/services/workspace-domains.service.ts` →
`resolveWorkspaceAndPublicDomain(origin)` splits the origin into subdomain / custom domain via
`DomainServerConfigService.getSubdomainAndCustomDomainFromUrl`, and returns the default workspace when
`IS_MULTIWORKSPACE_ENABLED` is false. It is used by `getPublicWorkspaceDataByDomain`
(`@OriginHeader() originHeader: string`) and by route triggers, not by `hydrateGraphqlRequest`.

`src/engine/middlewares/graphql-refuse-suspended-workspace.middleware.ts`: if `isWorkspaceSuspended(req.workspace)` → `AuthException('Workspace is suspended', WORKSPACE_SUSPENDED)` written as a GraphQL error body with HTTP 200.

### 1.5 GraphQL server configuration

`src/engine/api/graphql/graphql-config/graphql-config.service.ts`

```ts
const config: YogaDriverConfig = {
  autoSchemaFile: true,
  include: [CoreEngineModule, ModulesModule],
  resolverSchemaScope: 'core',
  buildSchemaOptions: {},
  resolvers: { JSON: GraphQLJSON },
  plugins: [useDirectExecution(...), useGraphQLErrorHandlerHook(...), useValidateGraphqlQueryComplexity(...)],
  path: `/${ApiPath.GraphQL}`,
  context: () => ({ loaders: this.dataloaderService.createLoaders() }),
};
```

In development, `config.renderGraphiQL = () => renderApolloPlayground()`.
Complexity limits: `GRAPHQL_MAX_FIELDS`, `GRAPHQL_MAX_ROOT_RESOLVERS`, `checkDuplicateRootResolvers: true`
(`src/engine/core-modules/graphql/hooks/use-validate-graphql-query-complexity.hook.ts`).

### 1.6 Other endpoints

| path | controller |
|---|---|
| `/healthz` | `engine/core-modules/health/controllers/health.controller.ts` |
| `/auth/**` | `engine/core-modules/auth/controllers/*` (google, microsoft, google-apis, microsoft-apis, sso/saml, oauth-propagator) + `engine/core-modules/application/connection-provider/*` |
| `/oauth/**` | `engine/core-modules/application/application-oauth/controllers/oauth-{discovery,registration,token}.controller.ts` |
| `/file/**`, `/file-upload` | `engine/core-modules/file/controllers/file.controller.ts`, `engine/core-modules/file/file-upload/controllers/file-upload.controller.ts` |
| `/webhooks` | `engine/core-modules/workflow/controllers/workflow-trigger.controller.ts`; also `engine/core-modules/billing-webhook/billing-webhook.controller.ts`, `modules/messaging-webhooks/messaging-webhooks.controller.ts` |
| `/rest/**` | `engine/api/rest/core/controllers/rest-api-core.controller.ts` + per-entity controllers under `engine/metadata-modules/*/controllers/*.controller.ts` |
| `/mcp` | `engine/api/mcp/controllers/mcp-core.controller.ts` |
| `/s` | `engine/core-modules/server-route-trigger/server-route-trigger.controller.ts` |
| `/open-api` | `engine/core-modules/open-api/open-api.controller.ts` |
| `/.well-known` | `engine/core-modules/well-known/controllers/well-known.controller.ts` |
| `/client-config` | `engine/core-modules/client-config/client-config.controller.ts` |

### 1.7 SSE / subscriptions

There is **no** GraphQL `Subscription` operation in the workspace data API: `GqlOperation` contains `Query`, `Mutation`, `Subscription`
(`src/engine/api/graphql/workspace-schema-builder/enums/gql-operation.enum.ts`) but the workspace schema generator only
builds Query + Mutation (`workspace-graphql-schema.factory.ts`), and the direct-execution hook explicitly bails on
subscription operations (`src/engine/api/graphql/direct-execution/utils/is-subscription-operation.util.ts`,
`hooks/use-direct-execution.hook.ts` returns early for `isSubscriptionOperation`).

Real-time delivery is done with **server-sent events** over the event emitter:
`src/engine/subscriptions/` + `src/engine/workspace-event-emitter/`. The `WorkspaceEventEmitter` emits
`DatabaseEvent` payloads (`src/engine/api/graphql/graphql-query-runner/enums/database-event-action.ts`,
`decorators/on-database-batch-event.decorator.ts`, `listeners/entity-events-to-db.listener.ts`) and the front-end
subscribes over SSE to the core `/graphql` endpoint's subscription transport.

---

## 2. Dynamic workspace GraphQL schema

### 2.1 Two-layer model (important)

Twenty's `/graphql` endpoint is **not** a single schema with all workspace objects baked in. It has:

1. **Core schema** — NestJS code-first resolvers from `CoreEngineModule` + `ModulesModule` (auth, metadata-ish core ops, MCP helpers, etc.), served by Yoga.
2. **Workspace data schema** — generated dynamically per workspace from object/field metadata. It is used for
   **introspection** (SDL), and the actual data operations are executed by a **direct execution** path that bypasses
   normal GraphQL resolvers.

`src/engine/api/graphql/direct-execution/hooks/use-direct-execution.hook.ts` (a Yoga `onRequest` plugin):
- No `req.workspace` → returns `AuthenticationError('Authentication required', { subCode: 'UNAUTHENTICATED' })`.
- Parses the query; extracts top-level fields; classifies them
  (`src/engine/api/graphql/direct-execution/utils/classify-top-level-fields.util.ts`) into
  `hasIntrospectionFields` (`__schema`, `__type`), `hasWorkspaceFields` (name in the workspace resolver name map), `hasCoreFields` (anything else).
- **Core + workspace fields in the same request → error**: `UserInputError('This query cannot be executed as a single request. Please split it into separate queries.')`.
- Core-only → falls through to the normal Nest/Yoga execution.
- Workspace-only or introspection → `DirectExecutionService.execute(req, document, hasIntrospectionFields, hasWorkspaceFields)`.

`src/engine/api/graphql/direct-execution/direct-execution.service.ts`:
- Introspection: `WorkspaceGraphqlSchemaSDLService.getOrComputeSchemaSDL(workspace, applicationId)` → `buildSchema(sdl)` → `graphql.execute(...)`.
- Workspace data: for each top-level field, look up `graphQLResolverNameMap[field.name.value]`, extract args from the AST
  (`utils/extract-arguments-from-ast.util.ts`), build a partial ResolveInfo (`utils/graphql-build-partial-resolve-info.util.ts`),
  derive field selections (`src/engine/api/graphql/workspace-resolver-builder/utils/get-graphql-field-selections.util.ts`),
  then `WorkspaceResolverRunnerService.run({ method, args, fieldSelections, workspaceSchemaBuilderContext })`.
- Result shaping: `utils/graphql-format-result-from-selected-fields.util.ts`.
- Root-resolver limits: `GRAPHQL_MAX_ROOT_RESOLVERS` and duplicate-root-resolver detection → `UserInputError`.
- Cost telemetry: `compute-graphql-direct-execution-query-cost.util.ts` → histogram
  `twenty_graphql_direct_execution_estimated_result_field_count`.

The resolver name map is a **derived workspace cache**:
`src/engine/api/graphql/direct-execution/services/workspace-resolver-name-map-cache.service.ts`
(`@WorkspaceDerivedCache('graphQLResolverNameMap')`, source key `flatObjectMetadataMaps`), built by
`utils/build-resolver-name-map.util.ts`.

### 2.2 Schema generation pipeline

`src/engine/api/graphql/workspace-schema-builder/`

- `workspace-graphql-schema.factory.ts` → `WorkspaceGraphQLSchemaGenerator.generateSchema(context)`:
  `gqlTypeGenerator.buildAndStore(context)` → take `Query` and `Mutation` object types → `new GraphQLSchema({ query, mutation, types: rest })`.
- `graphql-type-generators/gql-type.generator.ts` → `GqlTypeGenerator.buildAndStore(context)`:
  1. For each composite type in `compositeTypeDefinitions` → enum type + object type + input type (Create/Update/Filter/OrderBy/GroupBy).
  2. `GroupByDateGranularityInputTypeGenerator.buildAndStore()` (DateGranularityEnum, FirstDayOfTheWeek, GroupByDateGranularityInput, OrderByDateGranularityInput).
  3. For each object metadata → enum field types, plain object type, edge type, connection type, groupBy connection type, relation connect input type, all input types, and (if it has relation/morph fields) the "with relations" object type.
  4. `QueryTypeGenerator.buildAndStore(context)`, `MutationTypeGenerator.buildAndStore(context)`.

`SchemaGenerationContext` (`types/schema-generation-context.type.ts`) carries:
`flatObjectMetadataMaps`, `flatFieldMetadataMaps`, `flatFieldMetadataOptionMaps`, `flatIndexMaps`.

### 2.3 Exact operation names per object

`src/engine/utils/get-resolver-name.util.ts` — the single source of truth:

| method | GraphQL name | notes |
|---|---|---|
| `findMany` | `camelCase(namePlural)` e.g. `people` | Connection type |
| `findOne` | `camelCase(nameSingular)` e.g. `person` | Plain type |
| `findDuplicates` | `camelCase(nameSingular) + 'Duplicates'` e.g. `personDuplicates` | only when `objectMetadata.duplicateCriteria` is set |
| `createOne` | `create + PascalCase(nameSingular)` e.g. `createPerson` | |
| `createMany` | `create + PascalCase(namePlural)` e.g. `createPeople` | |
| `updateOne` | `update + PascalCase(nameSingular)` e.g. `updatePerson` | |
| `updateMany` | `update + PascalCase(namePlural)` e.g. `updatePeople` | |
| `deleteOne` | `delete + PascalCase(nameSingular)` e.g. `deletePerson` | soft delete |
| `deleteMany` | `delete + PascalCase(namePlural)` e.g. `deletePeople` | soft delete |
| `destroyOne` | `destroy + PascalCase(nameSingular)` e.g. `destroyPerson` | hard delete |
| `destroyMany` | `destroy + PascalCase(namePlural)` e.g. `destroyPeople` | hard delete |
| `restoreOne` | `restore + PascalCase(nameSingular)` e.g. `restorePerson` | un-soft-delete |
| `restoreMany` | `restore + PascalCase(namePlural)` e.g. `restorePeople` | |
| `mergeMany` | `merge + PascalCase(namePlural)` e.g. `mergePeople` | |
| `groupBy` | `camelCase(namePlural) + 'GroupBy'` e.g. `peopleGroupBy` | GroupByConnection type |

Query set = `findMany, findOne, findDuplicates, groupBy`; Mutation set = `createMany, createOne, updateOne, deleteOne,
updateMany, deleteMany, destroyOne, destroyMany, restoreOne, restoreMany, mergeMany`
(`src/engine/api/graphql/workspace-resolver-builder/constants/workspace-resolver-builder-method-names.ts`,
`constants/resolver-method-names.ts`).

`WorkspaceResolverBuilderService.shouldBuildResolver` only gates `findDuplicates` on `duplicateCriteria` presence.

### 2.4 Root type generation and args

`src/engine/api/graphql/workspace-schema-builder/graphql-type-generators/root-types/root-type.generator.ts`
- For each object × each method, builds a field on `Query`/`Mutation` with `resolve: undefined` (the direct-execution
  path fills the data), output type chosen by `getObjectTypeDefinitionKindByMethodName`:
  `findMany`/`findDuplicates` → `Connection`, `groupBy` → `GroupByConnection`, everything else → `Plain`.
- Array-wrapped output for `updateMany, deleteMany, createMany, findDuplicates, restoreMany, destroyMany, groupBy`.

`src/engine/api/graphql/workspace-schema-builder/utils/get-resolver-args.util.ts` — exact arg shapes:

- `findMany`: `first: Int`, `last: Int`, `offset: Int`, `before: String`, `after: String`, `filter: <Obj>FilterInput`, `orderBy: [<Obj>OrderByInput]`
- `findOne`: `filter: <Obj>FilterInput` (non-null)
- `findDuplicates`: `ids: [UUID]`, `data: [<Obj>CreateInput]`
- `createOne`: `data: <Obj>CreateInput` (non-null), `upsert: Boolean`, `shareWith: [ShareWithInput!]`
- `createMany`: `data: [<Obj>CreateInput]!`, `upsert: Boolean`, `shareWith: [ShareWithInput!]`
- `updateOne`: `id: UUID!`, `data: <Obj>UpdateInput!`
- `updateMany`: `data: <Obj>UpdateInput!`, `filter: <Obj>FilterInput!`
- `deleteOne` / `destroyOne` / `restoreOne`: `id: UUID!`
- `deleteMany` / `destroyMany` / `restoreMany`: `filter: <Obj>FilterInput!`
- `mergeMany`: `ids: [UUID]!`, `conflictPriorityIndex: Int!`, `dryRun: Boolean`
- `groupBy`: `groupBy: [<Obj>GroupByInput]!`, `filter: <Obj>FilterInput`, `orderBy: [<Obj>OrderByWithGroupByInput]`, `orderByForRecords: [<Obj>OrderByInput]`, `viewId: UUID`, `limit: Int`, `offsetForRecords: Int`

Input type names (`computeObjectMetadataInputTypeKey`): `<PascalSingular>CreateInput`, `<PascalSingular>UpdateInput`,
`<PascalSingular>FilterInput`, `<PascalSingular>OrderByInput`, `<PascalSingular>OrderByWithGroupByInput`,
`<PascalSingular>GroupByInput`; relation connect: `<PascalSingular>RelationInput` / `<PascalSingular>ConnectInput` /
`<PascalSingular>WhereUniqueInput`.

### 2.5 Output types

`graphql-type-generators/object-types/`

- `object-metadata-gql-object-type.generator.ts` → type `<PascalSingular>` (kind Plain ⇒ name is just `PascalSingular`).
  Fields = metadata fields; relation/morph fields contribute their join column `<field>Id` (see
  `src/engine/metadata-modules/field-metadata/utils/compute-morph-or-relation-field-join-column-name.util.ts` → `${name}Id`),
  except ONE_TO_MANY relations which are omitted here.
- `object-metadata-with-relations-gql-object-type.generator.ts` → **extends the same type key** with relation fields:
  a MANY_TO_ONE relation exposes the target's `Plain` type; a ONE_TO_MANY relation exposes the target's `Connection` type
  **with the `findMany` args attached** (`first/last/offset/before/after/filter/orderBy`).
- `edge-gql-object-type.generator.ts` → `<PascalSingular>Edge` with `node: <PascalSingular>!` and `cursor: Cursor!`.
- `connection-gql-object-type.generator.ts` → `<PascalSingular>Connection` with all aggregation fields + `edges: [<PascalSingular>Edge!]!` + `pageInfo: PageInfo!`.
- `group-by-connection-gql-object-type.generator.ts` → `<PascalSingular>GroupByConnection` = all connection fields + `groupByDimensionValues: JSON!`.
- `aggregation-type.generator.ts` + `utils/get-available-aggregations-from-object-fields.util.ts` → per-field aggregations:
  `countUniqueValues<Field>: Int`, `countEmpty<Field>: Int`, `countNotEmpty<Field>: Int`, `percentageEmpty<Field>: Float`,
  `percentageNotEmpty<Field>: Float`; date-ish fields add `min<Field>`/`max<Field>: DateTime`; BOOLEAN adds
  `countTrue<Field>`/`countFalse<Field>: Int`; NUMBER adds `min/max/avg/sum<Field>: Float`; RATING adds `min/max/avg`; CURRENCY adds
  `min/max/sum/avg<Field>AmountMicros: Float`. Plus a global **`totalCount: Int`** ("Total number of records in the connection").
- `PageInfo` (`graphql-types/object/page-into.object-type.ts`): `startCursor: ConnectionCursor`, `endCursor: ConnectionCursor`,
  `hasNextPage: Boolean!`, `hasPreviousPage: Boolean!`.

### 2.6 Filter input types

Sources: `graphql-types/input/*.input-type.ts`, `utils/create-gql-enum-filter-type.util.ts`,
`services/type-mapper.service.ts`.

| input type | operators |
|---|---|
| `StringFilter` | `eq, gt, gte, in, lt, lte, neq, startsWith, endsWith, like, ilike, regex, iregex, is` |
| `UUIDFilter` | `eq, gt, gte, in, lt, lte, neq, is` |
| `BooleanFilter` | `eq, is` |
| `DateFilter`, `DateTimeFilter` | `eq, gt, gte, in, lt, lte, neq, is` + field-reference operators |
| `IntFilter`, `FloatFilter`, `BigIntFilter`, `BigFloatFilter` | `eq, gt, gte, in, lt, lte, neq, is` + field-reference operators |
| `<Enum>Filter` (generated per enum field) | `eq, neq, in, containsAny, is, isEmptyArray, gt, gte, lt, lte` |
| `SelectFilter` | `in, is` |
| `MultiSelectFilter` | `containsAny, is, isEmptyArray` |
| `ArrayFilter` | `containsIlike, is, isEmptyArray` |
| `RawJsonFilter` | `is, like, ilike` |
| `RichTextFilter` | `blocknote: RichTextLeafFilter{ilike}`, `markdown: RichTextLeafFilter{ilike}` |
| `TSVectorFilter` | `search` |

- `FilterIs` enum: `NULL`, `NOT_NULL`.
- Field-reference operators (numeric/date only): `gtField, gteField, ltField, lteField`
  (`packages/twenty-shared/src/constants/FieldReferenceFilterOperators.ts` → SQL `>`, `>=`, `<`, `<=`).
- Every object filter input also has `and: [<Obj>FilterInput]`, `or: [<Obj>FilterInput]`, `not: <Obj>FilterInput`.

Type → filter mapping (`TypeMapperService.mapToFilterType`): UUID/RELATION/MORPH_RELATION → `UUIDFilter`; TEXT → `StringFilter`;
DATE_TIME → `DateTimeFilter`; DATE → `DateFilter`; BOOLEAN → `BooleanFilter`; NUMBER → Int/Float/BigInt per `settings.dataType`;
NUMERIC → `BigFloatFilter`; POSITION → `FloatFilter`; FILES/RAW_JSON → `RawJsonFilter`; RICH_TEXT → `RichTextFilter`;
ARRAY → `ArrayFilter`; MULTI_SELECT → `MultiSelectFilter`; SELECT → `SelectFilter`; TS_VECTOR → `TSVectorFilter`.

**Runtime-allowed operators are narrower than the schema** — `src/engine/api/common/common-args-processors/filter-arg-processor/constants/filter-operators.constant.ts`:
TEXT = `eq,neq,gt,gte,lt,lte,in,is,like,ilike,startsWith,endsWith` (**no `regex`/`iregex`**);
NUMBER/NUMERIC/POSITION = `eq,neq,gt,gte,lt,lte,in,is` + field-refs; BOOLEAN = `eq,is`;
DATE/DATE_TIME = `eq,neq,gt,gte,lt,lte,in,is` + field-refs; UUID = `eq,neq,gt,gte,lt,lte,in,is`;
ARRAY = `containsIlike,is,isEmptyArray`; MULTI_SELECT = `containsAny,is,isEmptyArray`;
SELECT/RATING = `eq,neq,gt,gte,lt,lte,in,containsAny,is,isEmptyArray`; RAW_JSON/FILES = `is,like,ilike`;
RICH_TEXT = `ilike`; TS_VECTOR/composites (ACTOR, ADDRESS, CURRENCY, EMAILS, FULL_NAME, LINKS, PHONES) = `eq,neq,is`.
`MAX_RELATION_FILTER_DEPTH = 1` (relation filters traverse exactly one hop).

### 2.7 OrderBy inputs

- `OrderByDirection` enum (`graphql-types/enum/order-by-direction.enum-type.ts`): `AscNullsFirst`, `AscNullsLast`, `DescNullsFirst`, `DescNullsLast`.
- Every scalar field in `<Obj>OrderByInput` maps to `OrderByDirection`
  (`TypeMapperService.mapToOrderByType`).
- Date/DATE_TIME fields in the group-by flavour map to `OrderByDateGranularityInput { orderBy: OrderByDirection, granularity: DateGranularityEnum }`.
- Relation fields contribute their join column plus a nested `<Target>OrderByInput` (`generateSimpleRelationFieldOrderByInputType`).
- `<Obj>OrderByWithGroupByInput` has two parts: `aggregate: <Obj>OrderByWithGroupByAggregateInput` (per-aggregation
  `OrderByDirection` keys) and the plain dimension fields (`generateOrderByOnDimensionValuesFields`).
- `DateGranularityEnum` values: `DAY, MONTH, QUARTER, YEAR, WEEK, DAY_OF_THE_WEEK, MONTH_OF_THE_YEAR, QUARTER_OF_THE_YEAR, NONE`
  (`packages/twenty-shared/src/types/ObjectRecordGroupByDateGranularity.ts`); `FirstDayOfTheWeek`: `MONDAY, SUNDAY, SATURDAY`.

### 2.8 Create / Update inputs

- `ObjectMetadataCreateGqlInputTypeGenerator`: scalar fields → mapped input scalar, `NonNull` when
  `isNullable === false` and there is no default value (`utils/apply-type-options-for-create-input.util.ts`);
  MULTI_SELECT → list; relation fields → `<field>Id` + `<field>` connect input; enum → generated enum; composite → composite input type.
- `ObjectMetadataUpdateGqlInputTypeGenerator`: same fields but **always nullable** (`apply-type-options-for-update-input.util.ts` only wraps arrays).
- `computeFieldInputTypeOptions` sets `isIdField: field.name === 'id'` (so `id` maps to `GraphQLID`).

### 2.9 Relation fields and nested writes

- Join column naming: `<field>Id` (`computeMorphOrRelationFieldJoinColumnName`).
- Nested write input `RelationConnectGqlInputTypeGenerator` builds `<PascalSingular>RelationInput` with:
  - `connect: <PascalSingular>ConnectInput` — where-input built from the target's **unique constraints**
    (`getUniqueConstraintsFields`) → `<PascalSingular>WhereUniqueInput` keyed by unique field names
    (composite fields expose only their `isIncludedInUniqueConstraint` properties).
  - `create: <PascalSingular>CreateInput`
  - `disconnect: Boolean`
  - keywords from `packages/twenty-shared/src/constants/RelationNestedQueriesKeyword.ts`:
    `CONNECT='connect'`, `CONNECT_WHERE='where'`, `CREATE='create'`, `DISCONNECT='disconnect'`.
- Relation nested create depth cap: `MAX_NESTED_RELATION_CREATE_DEPTH = 5`
  (`src/engine/api/common/common-query-runners/common-create-many-query-runner/common-create-many-query-runner.service.ts`).

### 2.10 Composite types

`packages/twenty-shared/src/types/composite-types/composite-type-definitions.ts` registers 8 composite types:

| type | properties |
|---|---|
| `LINKS` | `primaryLinkLabel` (TEXT), `primaryLinkUrl` (TEXT, unique), `secondaryLinks` (RAW_JSON) |
| `CURRENCY` | `amountMicros` (NUMERIC), `currencyCode` (TEXT) |
| `FULL_NAME` | `firstName` (TEXT), `lastName` (TEXT) |
| `ADDRESS` | `addressStreet1/2`, `addressCity`, `addressPostcode`, `addressState`, `addressCountry` (TEXT), `addressLat`, `addressLng` (NUMERIC) |
| `ACTOR` | `source` (SELECT enum: EMAIL/CALENDAR/WORKFLOW/AGENT/API/IMPORT/MANUAL/SYSTEM/WEBHOOK/APPLICATION), `workspaceMemberId` (UUID, hidden from input), `name` (TEXT, hidden from input), `context` (RAW_JSON) |
| `EMAILS` | `primaryEmail` (TEXT, unique), `additionalEmails` (RAW_JSON) |
| `PHONES` | `primaryPhoneNumber`, `primaryPhoneCountryCode`, `primaryPhoneCallingCode` (TEXT, all unique), `additionalPhones` (RAW_JSON) |
| `RICH_TEXT` | `blocknote` (TEXT, deprecated), `markdown` (TEXT), `tiptap` (TEXT) |

Composite output type = `PascalCase(type)`; composite inputs = `PascalCase(type)CreateInput|UpdateInput|FilterInput|OrderByInput|GroupByInput`.
RAW_JSON-backed sub-fields are exposed as real GraphQL types via
`utils/get-composite-sub-field-gql-types.util.ts`:
`AdditionalPhone{number,callingCode,countryCode}`, `SecondaryLink{label,url}`, `ActorContext{provider,applicationId,workflowId}`.
Physical column naming for composites: `<fieldName><CapitalizedSubField>`
(`src/engine/twenty-orm/utils/format-column-names-from-composite-field-and-subfield.util.ts`).

### 2.11 Scalars

`graphql-types/scalars/`: `UUID` (validated), `Cursor` (string), `ConnectionCursor` (string), `Date` (`yyyy-MM-dd`),
`BigFloat` (string↔number), `BigInt` (string↔bigint), `Position` (`'first'|'last'|number`), `Time` (epoch ms), `TSVector` (string).
`GraphQLISODateTime` from `@nestjs/graphql` for DATE_TIME; `GraphQLJSON` for RAW_JSON.

### 2.12 Introspection SDL caching

`src/engine/api/graphql/workspace-graphql-schema-sdl/workspace-graphql-schema-sdl.service.ts`
- Cache dependencies: `SCHEMA_SDL_CACHE_DEPENDENCIES = ['flatObjectMetadataMaps','flatFieldMetadataMaps','flatFieldMetadataOptionMaps','flatIndexMaps','flatApplicationMaps']`.
- Hash-combined and looked up in `WorkspaceCacheStorageService.getGraphQLTypeDefs(workspaceId, metadataCacheHash, applicationId)`.
- Application-scoped schemas (application tokens) filter the flat maps to the app's objects/fields/indexes.
- Core workflow app operations are appended to the SDL for app-scoped schemas
  (`workspace-graphql-schema-sdl/core-workflow-app-operations-sdl.constants.ts`).

### 2.13 Query runner and SQL building

`src/engine/api/common/common-query-runners/`

Base class `CommonBaseQueryRunnerService` (`common-base-query-runner.service.ts`):
1. Root operation → `usageLimitSpeedService.consumeOrThrow` (API request speed limit).
2. `validate(args, ctx)` — per-operation validation.
3. `CommonQueryParser.parseSelectedFields(args.selectedFields)`.
4. `processArgs` → `computeArgs` → **pre-query hooks** (see 2.15) → `processedArgs`.
5. `validateQueryComplexity` — `COMMON_QUERY_COMPLEXITY_LIMIT`; rejects nested one-to-many inside one-to-many
   (`TOO_COMPLEX_QUERY`); complexity = `1 + relationFieldCount` (v2 = max fields per record).
6. `workspaceOrmManager.executeInWorkspaceContext(...)` → `executeQueryAndEnrichResults`.
7. `run(...)` (abstract, per operation) → then `enrichResultsWithGettersAndHooks`:
   `commonResultGettersService.processRecordArray` → `commonRecordVisibilityService.applyVisibilityRestrictions`
   → **post-query hooks** (deferred to `transactionScope.afterCommit` when inside a transaction).

Operation services:
`common-find-many-query-runner.service.ts`, `common-find-one-query-runner.service.ts`,
`common-find-duplicates-query-runner.service.ts`, `common-group-by-query-runner.service.ts`,
`common-create-one-query-runner.service.ts`, `common-create-many-query-runner/common-create-many-query-runner.service.ts`,
`common-update-one-query-runner.service.ts`, `common-update-many-query-runner.service.ts`,
`common-delete-one-query-runner.service.ts`, `common-delete-many-query-runner.service.ts`,
`common-destroy-one-query-runner.service.ts`, `common-destroy-many-query-runner.service.ts`,
`common-restore-one-query-runner.service.ts`, `common-restore-many-query-runner.service.ts`,
`common-merge-many-query-runner.service.ts`.

Shared bases: `common-record-query-runner.service.ts`, `common-filtered-mutation-one-query-runner.service.ts`
(delete/destroy/restore one → runs the "many" variant with `filter: { id: { eq: id } }`, throws `RECORD_NOT_FOUND` when empty),
`common-filtered-mutation-many-query-runner.service.ts` (requires a non-empty filter → `INVALID_ARGS_FILTER`).

**findMany SQL** (`common-find-many-query-runner.service.ts`):
- `queryBuilder = readRepository.createQueryBuilder(nameSingular)`; a clone is the aggregate builder.
- `applyFilterToBuilder` → `applyDeletedAtToBuilder` (a `deletedAt` key anywhere in the filter tree enables `withDeleted()`).
- Order-by leaves resolved with `resolveOrderByLeaves`, appending `DEFAULT_ID_ORDER_BY_TIEBREAKER = { id: AscNullsLast }`
  (`src/engine/api/common/constants/default-id-order-by-tiebreaker.constant.ts`).
- Cursor: `getCursor(args)` decodes base64url JSON (`src/engine/api/common/utils/cursors.util.ts`);
  `computeCursorArgFilter` turns it into a keyset predicate merged as `{ or: [...] }` / `{ and: [filter, {or}] }`.
- `limit = getFindManyPageSize(args)` = `min(first ?? last ?? QUERY_MAX_RECORDS, QUERY_MAX_RECORDS)`; `QUERY_MAX_RECORDS = 200`
  (`packages/twenty-shared/src/constants/QueryMaxRecords.ts`), `QUERY_MAX_RECORDS_FROM_RELATION = 60`
  (`QueryMaxRecordsFromRelation.ts`), `QUERY_DEFAULT_LIMIT_RECORDS = 60` (REST default).
  The builder fetches `limit + 1` rows; `buildCursorPage` trims the sentinel and flips order for backward pages.
- Aggregates computed on the clone: `ProcessAggregateHelper.addSelectedAggregatedFieldsQueriesToQueryBuilder`.
- `totalCount` comes from `aggregatedValues.totalCount`.
- Filtering/ordering through a non-to-one relation is rejected: `INVALID_QUERY_INPUT` ("only to-one relations are supported").
- `findMany` arg validation: cannot pass both `first` and `last`, both `before` and `after`, `before`+`first`, `after`+`last`; `first`/`last` must be ≥ 0.

**Filter → SQL** (`src/engine/api/common/common-query-parsers/common-query-filter/`):
- `CommonQueryFilterConditionParser.parse` → `applyFilterEntriesToWhereExpression` (`utils/apply-filter-entries-to-where-expression.util.ts`):
  `and`/`or` → bracketed groups; `not` → `NotBrackets`; anything else → field parser.
- `CommonQueryFilterFieldParser.parse` resolves the field by name **or join-column name**; relation fields are matched by
  `fieldMetadata.name` (sub-filter) while `<field>Id` is the id filter; composite fields are expanded to
  `<fieldName><CapitalizedSubField>` columns; then `computeWhereConditionParts` builds the SQL.
- SQL per operator (`src/engine/engine/record-filter/utils/compute-where-condition-parts.ts` — actual path
  `src/engine/record-filter/utils/compute-where-condition-parts.ts`):
  `eq` → `=`; `neq` → `!= OR IS NULL`; `gt/gte/lt/lte` → comparisons (DATE_TIME `eq` uses a `[ts, ts+1ms)` range);
  `in` → `IN (:...p)`; `is` → `IS NULL` / `IS NOT NULL`; `like` → `::text LIKE`; `ilike` → `::text ILIKE`;
  `startsWith` → `::text ^@`; `endsWith` → `RIGHT(::text, LENGTH()) =`; `contains` → `@> ARRAY[...]`;
  `containsAny` → `::text[] && ARRAY[...]`; `notContains` → `NOT (::text[] && ARRAY[...])`;
  `containsIlike` → `EXISTS (SELECT 1 FROM unnest(col) elem WHERE elem ILIKE ...)`;
  `search` (TS_VECTOR) → `col @@ to_tsquery('simple', public.unaccent_immutable(:ts)) OR public.unaccent_immutable(col::text) ILIKE public.unaccent_immutable(:like)`;
  `isEmptyArray` → `= '{}' OR IS NULL` / `IS NOT NULL AND != '{}'`.
  Internal-only strict variants: `isStrictly`, `gtStrict`, `eqStrict` (used by keyset cursors).
  Unknown operator → `RecordFilterException` `UNSUPPORTED_OPERATOR`.
- Values are bound as **parameters** (`removeSqlDDLInjection` on the param name, random hex suffix); identifiers are escaped
  (`escapeIdentifier`).
- **Null-equivalence widening**: `src/engine/record-filter/utils/find-postgres-default-null-equivalent-value.util.ts` +
  `constants/null-equivalent-values.constant.ts` — Postgres stores `''` / `'{}'` where Twenty stores `NULL`, so `eq: ''`
  becomes `(col = '' OR col IS NULL)`; TEXT null-equivalent `''`, ARRAY/MULTI_SELECT null-equivalent `'{}'`.
- Relation sub-filters: to-one relations join an alias; to-many relations use a correlated `EXISTS`
  (`addRelationExistsFilter`); depth > 1 → `INVALID_QUERY_INPUT`; a to-many sub-filter through a joined to-one is rejected.
- Field-reference operators produce a raw column-vs-column comparison (`"alias"."field" > "other"."field"`); the referenced
  field must be the record the relation filter starts from.

**Order → SQL** (`common-query-order/common-query-order.parser.ts`):
`resolveOrderByLeaves` → per-leaf column expression; `convertOrderByToFindOptionsOrder` maps direction to
`{ order: 'ASC'|'DESC', nulls: 'NULLS FIRST'|'NULLS LAST' }` using `getEffectiveScanOrder(direction, isForwardPagination)`
so a backward page inverts both. Text-ish columns get `LOWER(...)` / `::text` casts (`shouldUseCaseInsensitiveOrder`,
`shouldCastToText`). Relation order-by adds a join alias and the joined columns are added to the select list
(`addRelationOrderColumnsToBuilder`, must run after `setFindOptions`).

**Cursors** (`src/engine/api/common/utils/cursors.util.ts`):
- `encodeCursorData` = `Buffer.from(JSON.stringify(cursorData)).toString('base64url')`; payload = the ordered leaf values + `id`.
- `decodeCursor` = base64 → JSON, invalid → `INVALID_CURSOR`.
- `getCursor(args)` prefers `after`, then `before`.
- `buildCursorPage` (in `src/engine/api/utils/build-cursor-page.util.ts`) trims the sentinel row and derives
  `hasNextPage`/`hasPreviousPage`.

**Selected fields / result getters**:
`src/engine/api/common/common-select-fields/common-select-fields-builder.ts`,
`src/engine/api/common/common-result-getters/common-result-getters.service.ts`,
`src/engine/api/common/common-nested-relations-processor/process-nested-relations.helper.ts` (loads nested relations
with `limit = QUERY_MAX_RECORDS_FROM_RELATION`), `src/engine/api/common/common-record-visibility/common-record-visibility.service.ts`
(applies record read restrictions).

**ORM layer** (`src/engine/twenty-orm/`):
- `workspace-orm.manager.ts` — `getRepository(nameSingular, { useReplica })`, `getRepositoryWithoutPermissionChecks`,
  `runInWorkspaceTransaction`, `executeInWorkspaceContext` (loads flat maps, feature flags, billing entitlements,
  permissions principals and record read restrictions into an `ORMWorkspaceContext`).
- `datasource/workspace-data-source.ts` + `workspace-data-source.service.ts` — per-request data source on the workspace
  pool (primary or replica), carrying an `internalContext` (flat maps, permissions, event emitter, record stock).
- `repository/workspace-repository.ts` — `createQueryBuilder`, `runInsert`, `runMutation` (soft-delete / hard-delete /
  update), `getRawAndEntities`, `getRawOne`; query builders in `query-builder/`.
- `utils/format-data.util.ts` — composites are exploded into their physical columns before insert/update;
  `utils/format-result.util.ts` — rows are re-assembled into composite objects and null-equivalent `''`/`'{}'`
  are mapped back to `null`.
- Physical table naming: `computeTableName(nameSingular, isCustom)` → `_<nameSingular>` for custom objects
  (`src/engine/utils/compute-table-name.util.ts`); schema name = `workspace_<base36(uuid)>`
  (`src/engine/workspace-datasource/utils/get-workspace-schema-name.util.ts`).
- Data arg processing (`src/engine/api/common/common-args-processors/data-arg-processor/data-arg-processor.service.ts`)
  coerces/validates each field by type (`validator-utils/validate-*-field-or-throw.util.ts`) and transforms composite
  payloads (`transformer-utils/transform-*-field.util.ts`).

### 2.14 Direct execution: result shaping

`src/engine/api/graphql/direct-execution/services/workspace-resolver-runner.service.ts`
- Selection kind per method: `findMany|findDuplicates|groupBy` → `'connection'`, everything else → `'record'`.
- `findMany` returns `{ kind:'connection', connection:{ records, connectionValues:{...aggregatedValues, totalCount}, relationAggregatedValues, hasNextPage, hasPreviousPage, order, orderByValuesByRecordId } }`.
- `groupBy` returns `kind:'groupByConnections'` — one connection per group, `connectionValues` = group aggregates + `totalCount`.
- `findDuplicates` returns `kind:'connections'` — one connection per duplicate set, `connectionValues:{ totalCount }`, order `[{ id: AscNullsFirst }]`.
- `createMany|updateMany|deleteMany|destroyMany|restoreMany` → `kind:'records'`; single-record ops → `kind:'record'`.
- `mergeMany` → `kind:'record'`.

`utils/build-common-selected-fields.util.ts` converts the GraphQL selection set into `CommonSelectedFields`
(`{ fields, relations, aggregates }`), and `utils/graphql-format-result-from-selected-fields.util.ts` renders the
runner result back into the GraphQL shape (connection → `{ edges: [{ node, cursor }], pageInfo, ...aggregations }`).

### 2.15 Pre/post query hooks

`src/engine/api/graphql/workspace-query-runner/workspace-query-hook/`

- `@WorkspaceQueryHook('<object>.<method>')` decorator (`decorators/workspace-query-hook.decorator.ts`), options
  `{ key, type: 'PRE_HOOK'|'POST_HOOK', scope }`.
- Key format `` `${string}.${WorkspaceResolverBuilderMethodNames}` ``; the storage supports a `*.<method>` wildcard which is
  matched **before** the object-specific key (`storage/workspace-query-hook.storage.ts`).
- `WorkspaceQueryHookService.executePreQueryHooks(authContext, objectName, methodName, payload)` — the returned payload is
  **merged** into the args (`merge`), so a hook can rewrite filter/orderBy/data.
  `executePostQueryHooks(authContext, objectName, methodName, payload)` — no return value; runs after the result getters.
- Pre-hooks run inside `processArgs` (before SQL); post-hooks run after the visibility restrictions, deferred to
  `transactionScope.afterCommit` when a transaction is open.

Registered hooks (all under `packages/twenty-server/src`; "PRE" = pre-query, "POST" = post-query):

| key | type | file |
|---|---|---|
| `*.createOne`, `*.createMany` | PRE | `engine/core-modules/actor/query-hooks/created-by.create-{one,many}.pre-query-hook.ts` (stamps `createdBy`/`updatedBy`) |
| `*.updateOne`, `*.updateMany` | PRE | `engine/core-modules/actor/query-hooks/updated-by.update-{one,many}.pre-query-hook.ts` |
| `workspaceMember.*` (all 10 methods) | PRE | `modules/workspace-member/query-hooks/*` — every one throws `PermissionsException` |
| `blocklist.*` (all 12 methods) | PRE | `modules/blocklist/query-hooks/*` |
| `dashboard.createOne/createMany/destroyOne/destroyMany/mergeMany` | PRE | `modules/dashboard/query-hooks/*` |
| `workflowRun.createOne/createMany/updateOne/updateMany/deleteOne/deleteMany` | PRE | `modules/workflow/common/query-hooks/workflow-run/*` |
| `timelineActivity.createOne/createMany/updateOne/updateMany` | PRE | `modules/timeline/query-hooks/*` |
| `calendarEventTarget.createOne/createMany` | PRE | `modules/calendar/common/query-hooks/calendar-event-target/*` |
| `messageThreadTarget.createOne/createMany` | PRE | `modules/messaging/common/query-hooks/message-thread-target/*` |
| `callRecording.createOne/createMany` | PRE | `modules/call-recording/query-hooks/*` |
| `note.deleteOne/deleteMany/restoreOne/restoreMany` | POST | `modules/note/query-hooks/*` |
| `task.deleteOne/deleteMany/restoreOne/restoreMany` | POST | `modules/task/query-hooks/*` |
| `agentChatThread.createOne/createMany/deleteOne/deleteMany` | POST | `engine/metadata-modules/ai/ai-chat/query-hooks/*` |

---

## 3. The `/metadata` GraphQL API

### 3.1 How it is mounted

- `src/engine/api/graphql/metadata-graphql-api.module.ts` registers a **second** `GraphQLModule.forRootAsync` with
  `resolverSchemaScope: 'metadata'`, `path: '/metadata'`, `include: [MetadataGraphQLApiModule, ModulesModule]`.
- `src/engine/api/graphql/metadata.module-factory.ts` — plugins: `useGraphQLErrorHandlerHook`, `useCachedMetadata`,
  `useDisableIntrospectionAndSuggestionsForUnauthenticatedUsers` (production), `useValidateGraphqlQueryComplexity`
  (`maximumAllowedRootResolvers: 10`, `maximumAllowedNestedFields: 10`, `checkDuplicateRootResolvers: true`).
- `src/engine/api/graphql/graphql-config/decorators/metadata-resolver.decorator.ts` — `@MetadataResolver()` marks a class
  as part of the metadata schema (scopes: `core` | `metadata` | `admin`,
  `src/engine/api/graphql/graphql-config/types/resolver-schema-scope.type.ts`).
- `src/engine/metadata-modules/metadata-engine.module.ts` — the module that imports every metadata submodule
  (ObjectMetadata, FieldMetadata, View, Role, Permissions, Webhook, McpServer, FrontComponent, LogicFunction,
  Skill, ValidationRule, AiAgent/AiChat, PageLayout*, NavigationMenuItem, SettingsMenuItem, CommandMenuItem,
  TimelineActivityType, RelatedRecordPath, MetadataTranslation, ConnectedAccount, MessageChannel/Folder,
  CalendarChannel, SearchFieldMetadata, RouteTrigger, ServerRouteTrigger, PermissionFlag, RecordPermissions, …).
- Response caching: `src/engine/api/graphql/graphql-config/hooks/use-cached-metadata.ts` caches the operations listed in
  `METADATA_GRAPHQL_OPERATIONS_TO_CACHE` (`graphql-config/constants/metadata-graphql-operations-to-cache.constant.ts`):
  - `ObjectMetadataItems` — scope `workspace`, deps `flatObjectMetadataMaps, flatFieldMetadataMaps, flatFieldMetadataOptionMaps, flatIndexMaps, flatSearchFieldMetadataMaps, flatApplicationMaps, featureFlagsMap`
  - `FIND_ALL_VIEWS_GRAPHQL_OPERATION` (`findAllViews`) — scope `userWorkspace`, deps `flatViewMaps, flatViewFieldMaps, flatViewFieldGroupMaps, flatViewFilterMaps, flatViewFilterGroupMaps, flatViewSortMaps, flatViewGroupMaps, flatObjectMetadataMaps, flatFieldMetadataMaps, flatApplicationMaps`
  Cache key = `graphql:operations:<op>:<workspaceId>:<dependencyHash>[:<userWorkspaceId>]:<locale>:<sha256(query+variables)>`.

### 3.2 Conventions

- Metadata list queries use **cursor pagination** with a `CursorPagingInput` (`first`/`last`/`before`/`after`) and
  return `CursorConnection<T>` = `{ edges: [{ node, cursor }], pageInfo: { hasNextPage, hasPreviousPage, startCursor, endCursor }, totalCount }`.
  Helper: `findManyWithCursorPagination` (`src/engine/core-modules/cursor-pagination/`), DTOs `ObjectConnectionDTO`,
  `FieldConnectionDTO`, `ObjectFieldsConnectionDTO`, `ObjectIndexMetadatasConnectionDTO`.
- Metadata filters are **column-based**, not the workspace filter DSL: `ObjectFilterInput`,
  `FieldFilterInput`, `IndexFilterInput` + `OBJECT_FILTER_COLUMN_BY_FILTER_FIELD` / `FIELD_FILTER_COLUMN_BY_FILTER_FIELD`
  (`src/engine/metadata-modules/object-metadata/dtos/object-filter.input.ts`,
  `field-metadata/dtos/field-filter.input.ts`), applied by `applyMetadataFilterToQueryBuilder`.
- Access control uses `AccessPolicyGuard(<NAME>_ACCESS_POLICY)` where the policy declares `authPrincipals`
  (userSession / apiKey / oauthClient / application) and `requiredPermissionFlags` (`PermissionFlagType.DATA_MODEL`, etc.).
- Metadata resolvers also register `FlatEntityMapsGraphqlApiExceptionFilter` and `PermissionsGraphqlApiExceptionFilter`
  globally (`metadata-engine.module.ts`).

### 3.3 Resolver inventory (file → operations → DTOs)

Format: `operationName: ReturnType` — args are noted where they differ.

**Data model**

| file | operations |
|---|---|
| `engine/metadata-modules/object-metadata/object-metadata.resolver.ts` | `objects: ObjectConnectionDTO` (`paging: CursorPagingInput = {first:10}`, `filter: ObjectFilterInput = {}`), `object: ObjectMetadataDTO` (`id: UUID!`), `objectRecordCounts: [ObjectRecordCountDTO]`, `mostlyEmptyFieldMetadataIds: [UUID!]` (`objectMetadataId: UUID!`), `createOneObject: ObjectMetadataDTO` (`input: CreateOneObjectInput!`), `updateOneObject: ObjectMetadataDTO` (`input: UpdateOneObjectInput!`), `deleteOneObject: ObjectMetadataDTO` (`input: DeleteOneObjectInput!`), `updateManyObjects: [ObjectMetadataDTO]` (`inputs: [UpdateOneObjectInput!]!`); ResolveFields: `fields: ObjectFieldsConnectionDTO`, `indexMetadatas: ObjectIndexMetadatasConnectionDTO`, `isUIReadOnly: Boolean` (deprecated → `isUIEditable`) |
| `engine/metadata-modules/field-metadata/field-metadata.resolver.ts` | `fields: FieldConnectionDTO` (`paging`, `filter: FieldFilterInput`), `field: FieldMetadataDTO` (`id: UUID!`), `createOneField: FieldMetadataDTO` (`input: CreateOneFieldMetadataInput!`), `updateOneField: FieldMetadataDTO` (`input: UpdateOneFieldMetadataInput!`), `deleteOneField: FieldMetadataDTO` (`input: DeleteOneFieldInput!`); ResolveFields: `object: ObjectMetadataDTO`, `isUIReadOnly`, `label`/`description`/`icon` (i18n-aware overrides) |
| `engine/metadata-modules/index-metadata/index-metadata.resolver.ts` | `createOneIndex: IndexMetadataDTO` (`input: CreateOneIndexInput!`), `deleteOneIndex: IndexMetadataDTO` (`input: DeleteOneIndexInput!`); ResolveFields `isCustom`, `indexFieldMetadataList: [IndexFieldMetadataDTO]` |
| `engine/metadata-modules/minimal-metadata/minimal-metadata.resolver.ts` | `minimalMetadata: MinimalMetadataDTO` — one call returning `{ objectMetadataItems: [MinimalObjectMetadataDTO], views: [MinimalViewDTO], collectionHashes: [CollectionHashDTO] }`; `MinimalObjectMetadataDTO` = `{ id, nameSingular, namePlural, labelSingular, labelPlural, icon?, color?, isActive, isSystem, isRemote }` |

**Views**

| file | operations |
|---|---|
| `engine/metadata-modules/view/resolvers/view.resolver.ts` | `getViews: [ViewDTO]` (optional `objectMetadataId`), `getView: ViewDTO` (`id: String!`), `createView: ViewDTO` (`input: CreateViewInput!`), `updateView: ViewDTO` (`id`, `input: UpdateViewInput!`), `deleteView: Boolean` (`id`), `destroyView: Boolean` (`id`) |
| `engine/metadata-modules/view-field/resolvers/view-field.resolver.ts` | `getViewFields: [ViewFieldDTO]` (`viewId`), `getViewField: ViewFieldDTO` (`id`), `createViewField: ViewFieldDTO` (`input: CreateViewFieldInput!`), `createManyViewFields: [ViewFieldDTO]` (`inputs: [CreateViewFieldInput!]!`), `updateViewField: ViewFieldDTO` (`input: UpdateViewFieldInput!`), `deleteViewField: ViewFieldDTO` (`input: DeleteViewFieldInput!`), `destroyViewField: ViewFieldDTO` (`input: DestroyViewFieldInput!`) |
| `engine/metadata-modules/view-filter/resolvers/view-filter.resolver.ts` | `getViewFilters: [ViewFilterDTO]`, `getViewFilter: ViewFilterDTO`, `createViewFilter`, `updateViewFilter`, `deleteViewFilter`, `destroyViewFilter` (same input DTO family) |
| `engine/metadata-modules/view-filter-group/resolvers/view-filter-group.resolver.ts` | `getViewFilterGroups`, `getViewFilterGroup`, `createViewFilterGroup`, `updateViewFilterGroup`, `deleteViewFilterGroup`, `destroyViewFilterGroup` |
| `engine/metadata-modules/view-group/resolvers/view-group.resolver.ts` | `getViewGroups`, `getViewGroup`, `createViewGroup`, `createManyViewGroups` (`inputs: [CreateViewGroupInput!]!`), `updateViewGroup`, `updateManyViewGroups` (`inputs: [UpdateViewGroupInput!]!`), `deleteViewGroup`, `destroyViewGroup` |
| `engine/metadata-modules/view-sort/resolvers/view-sort.resolver.ts` | `getViewSorts`, `getViewSort`, `createViewSort`, `updateViewSort`, `deleteViewSort: Boolean`, `destroyViewSort: Boolean` |
| `engine/metadata-modules/view-field-group/resolvers/view-field-group.resolver.ts` | view field groups CRUD |

**Permissions & roles**

| file | operations |
|---|---|
| `engine/metadata-modules/role/role.resolver.ts` | `getRoles: [RoleDTO]`, `getRole: RoleDTO` (`id`), `accessGroupPropagationProgress: AccessGroupPropagationProgressDTO`, `updateWorkspaceMemberRole: WorkspaceMemberDTO`, `createOneRole: RoleDTO` (`createRoleInput`), `updateOneRole: RoleDTO` (`updateRoleInput`), `deleteOneRole: String` (`roleId`), `upsertObjectPermissions: [ObjectPermissionDTO]`, `upsertPermissionFlags: [RolePermissionFlagDTO]`, `upsertFieldPermissions: [FieldPermissionDTO]`, `upsertRowLevelPermissionRules: UpsertRowLevelPermissionRulesResultDTO`, `assignRoleToAgent: Boolean`, `removeRoleFromAgent: Boolean` |
| `engine/metadata-modules/permission-flag/permission-flag.resolver.ts` | `getPermissionFlags: [PermissionFlagDTO]` |
| `engine/permissions/record-permissions/record-permissions.resolver.ts` | `recordPermissions: [RecordPermissionsResult]` (`targets`) |
| `engine/permissions/record-share/resolvers/record-sharing.resolver.ts` | `recordSharing: RecordSharingDTO` (`target: RecordTargetInput!`), `setRecordPrivate: RecordSharingDTO` (`target`, `isPrivate: Boolean!`), `setRecordShare: RecordSharingDTO` (`target`, `principal: RecordSharePrincipalInput!`, `accessLevel: RecordShareAccessLevel!`), `removeRecordShare: RecordSharingDTO` (`target`, `principal`) |

**Workspace / members / settings**

| file | operations |
|---|---|
| `engine/core-modules/workspace/workspace.resolver.ts` | `currentWorkspace: WorkspaceEntity`, `activateWorkspace`, `updateWorkspace`, `updateWorkspaceAllowedEmbeddingOrigins`, `updateWorkspaceIframeMediaPermissions`, `deleteCurrentWorkspace`, `getPublicWorkspaceDataByDomain: PublicWorkspaceDataDTO` (`origin`, public), `getPublicWorkspaceDataById: PublicWorkspaceDataSummaryDTO` (public), `checkCustomDomainValidRecords`; ResolveFields `featureFlags`, `isInternalImportEnabled`, `aiDataPolicy`, `billingSubscriptions`, `defaultRole`, `workspaceCustomApplication`, `installedApplications`, `currentBillingSubscription`, `isTrialAvailable`, `billingCustomer`, `workspaceMembersCount`, `logo`, `views` |
| `engine/core-modules/user/user.resolver.ts` | `currentUser: UserEntity`, `workspaceMember: WorkspaceMemberDTO` (`id`), `workspaceMembers: [WorkspaceMemberDTO]`, `deleteUserFromWorkspace: UserWorkspaceEntity`, plus user/session mutations |
| `engine/core-modules/api-key/api-key.resolver.ts` | `apiKeys: [ApiKeyEntity]`, `apiKey: ApiKeyEntity` (`input`), `getApiKeyRoles: [RoleDTO]`, `createApiKey: ApiKeyEntity` (`input`), `updateApiKey: ApiKeyEntity` (`input`), `revokeApiKey: ApiKeyEntity` (`input`), `assignRoleToApiKey: Boolean`; ResolveField `role: RoleDTO` |
| `engine/core-modules/workspace-invitation/workspace-invitation.resolver.ts` | `workspaceInvitations`, `sendInvitations`, `resendWorkspaceInvitation`, `deleteWorkspaceInvitation` |
| `engine/core-modules/impersonation/impersonation.resolver.ts` | `impersonate: ImpersonateDTO` (`userId`, `workspaceId`), `stopImpersonation: StopImpersonationDTO` |

**Webhooks**

`engine/metadata-modules/webhook/webhook.resolver.ts`: `webhooks: [WebhookDTO]`, `webhook: WebhookDTO` (`id: UUID!`),
`createWebhook: WebhookDTO` (`input: CreateWebhookInput!`), `updateWebhook: WebhookDTO` (`input: UpdateWebhookInput!`),
`deleteWebhook: WebhookDTO` (`id: UUID!`).

**Auth** (`engine/core-modules/auth/auth.resolver.ts`)

Queries: `checkUserExists: CheckUserExistDTO` (`email`, `captchaToken?`), `checkWorkspaceInviteHashIsValid: WorkspaceInviteHashValidDTO`,
`findWorkspaceFromInviteHash: WorkspaceEntity`, `checkWorkspaceSubdomainAvailability: SubdomainAvailabilityDTO`,
`getWorkspaceCreationDefaults: WorkspaceCreationDefaultsDTO`, `validatePasswordResetToken: ValidatePasswordResetTokenDTO`.

Mutations: `getAuthorizationUrlForSSO: GetAuthorizationUrlForSsoDTO`, `getLoginTokenFromCredentials: LoginTokenDTO`,
`signIn: AvailableWorkspacesAndAccessTokensDTO`, `verifyEmailAndGetLoginToken: VerifyEmailAndGetLoginTokenDTO`,
`verifyEmailAndGetWorkspaceAgnosticToken: AvailableWorkspacesAndAccessTokensDTO`,
`getAuthTokensFromOTP: AuthTokens`, `getAuthTokensFromTwoFactorAuthenticationRecoveryCode: TwoFactorAuthenticationRecoveryCodeRedemptionDTO`,
`signUp: AvailableWorkspacesAndAccessTokensDTO`, `signUpInWorkspace: SignUpDTO`,
`createNewWorkspaceLogoUpload: FileUploadTargetDTO`, `completeNewWorkspaceLogoUpload: FileWithSignedUrlDTO`,
`generateTransientToken: TransientTokenDTO`, `getAuthTokensFromLoginToken: AuthTokens`,
`getAuthTokensFromSSOExchangeToken: AuthTokens`, `authorizeApp: AuthorizeAppDTO`, `renewToken: AuthTokens`,
`signOut: Boolean`, `generateApiKeyToken: ApiKeyToken`, `generatePlaygroundToken: AuthToken`,
`emailPasswordResetLink: EmailPasswordResetLinkDTO`, `updatePasswordViaResetToken: InvalidatePasswordDTO`.

**Billing** (`engine/core-modules/billing/billing.resolver.ts`): `billingPortalSession`, `billingPartnerCode`,
`setBillingPartnerCode`, `checkoutSession`, `createSubscriptionPaymentIntent`, `createBillingPaymentMethodSetupIntent`,
`switchSubscriptionInterval`, `switchBillingPlan`, `cancelSwitchBillingPlan`, `cancelSwitchBillingInterval`,
`setResourceCreditSubscriptionPrice`, `billingPlans`, `listPlans`, `endSubscriptionTrialPeriod`,
`cancelSwitchResourceCreditPrice`, `billingCustomer`, `billingResourceCreditUsage`.

**Applications** (`engine/core-modules/application/application.resolver.ts` + `application-*/`): `applicationSdkClientChecksums`,
`isApplicationStopped`; `findManyApplications`, `findOneApplication`, `installApplication`, `uninstallApplication`,
`triggerInstallApplication`, `triggerUninstallApplication`, `findInstallApplicationJobStatus`,
`triggerUpgradeApplication`, `findUpgradeApplicationJobStatus`, `exportApplication`, `applicationCoreGraphqlSchema: String`,
`runApplicationHealthCheck`, `grantApplicationCapabilities`, `renewApplicationToken`, `enqueueJob`, `getJobs`,
`appKeyValue`, `setAppKeyValue`, `deleteAppKeyValue`, `myApplicationPreferences`, `findManyMarketplaceApps`,
`findMarketplaceAppDetail`, `publicMarketplaceApps`, `publicMarketplaceAppDetail`, plus application-registration CRUD.

**Navigation / layout / misc metadata**

| file | operations |
|---|---|
| `engine/metadata-modules/navigation-menu-item/navigation-menu-item.resolver.ts` | `navigationMenuItems: [NavigationMenuItemDTO]`, `navigationMenuItem`, `createNavigationMenuItem`, `createManyNavigationMenuItems`, `updateNavigationMenuItem`, `updateManyNavigationMenuItems`, `deleteNavigationMenuItem`, `deleteManyNavigationMenuItems` |
| `engine/metadata-modules/settings-menu-item/settings-menu-item.resolver.ts` | settings menu item CRUD |
| `engine/metadata-modules/command-menu-item/command-menu-item.resolver.ts` | `commandMenuItems: [CommandMenuItemDTO]`, `commandMenuItem`, `createCommandMenuItem`, `updateCommandMenuItem`, `resetCommandMenuItem`, `deleteCommandMenuItem` |
| `engine/metadata-modules/page-layout/resolvers/page-layout.resolver.ts` | `getPageLayouts`, `getPageLayout`, `createPageLayout`, `updatePageLayout`, `destroyPageLayout`, `updatePageLayoutWithTabsAndWidgets`, `resetPageLayoutToDefault`, `resetPageLayoutWidgetToDefault`, `resetPageLayoutTabToDefault` |
| `engine/metadata-modules/page-layout-tab/resolvers/page-layout-tab.resolver.ts` | `getPageLayoutTabs`, `getPageLayoutTab`, `createPageLayoutTab`, `updatePageLayoutTab`, `destroyPageLayoutTab` |
| `engine/metadata-modules/page-layout-widget/resolvers/page-layout-widget.resolver.ts` | `getPageLayoutWidgets`, `getPageLayoutWidget`, `createPageLayoutWidget`, `updatePageLayoutWidget`, `destroyPageLayoutWidget` |
| `engine/metadata-modules/timeline-activity-type/timeline-activity-type.resolver.ts` | `findManyTimelineActivityTypes`, `updateTimelineActivityTypeIsActive`, `resetTimelineActivityType` |
| `engine/metadata-modules/metadata-translation/metadata-translation.resolver.ts` | `metadataTranslations: [MetadataTranslationDTO]` (`input`) |
| `engine/metadata-modules/related-record-path/related-record-path.resolver.ts` | `relatedRecordPaths: [RelatedRecordPathDTO]`, `createRelatedRecordPath`, `deleteRelatedRecordPath` |
| `engine/metadata-modules/validation-rule/validation-rule.resolver.ts` | validation rule CRUD + `findValidationRulesReadingField` |
| `engine/metadata-modules/logic-function/logic-function.resolver.ts` | `findOneLogicFunction`, `findManyLogicFunctions`, `getAvailablePackages: JSON`, `createOneLogicFunction`, `updateOneLogicFunction`, `deleteOneLogicFunction`, `executeOneLogicFunction`, `getLogicFunctionSourceCode: String` |
| `engine/metadata-modules/front-component/front-component.resolver.ts` | `frontComponents: [FrontComponentDTO]`, `frontComponent`, `createFrontComponent`, `updateFrontComponent`, `deleteFrontComponent`, `generateFrontComponentApplicationTokenPair: ApplicationTokenPairDTO` |
| `engine/metadata-modules/mcp-server/mcp-server.resolver.ts` | `mcpServers: [McpServerDTO]`, `mcpServer`, `createMcpServer`, `updateMcpServer`, `syncMcpServerTools`, `deleteMcpServer` |
| `engine/metadata-modules/skill/skill.resolver.ts` | `skills: [SkillDTO]`, `skill: SkillDTO` (`id`), `createSkill`, `updateSkill`, `activateSkill`, `deactivateSkill`, `deleteSkill`; ResolveFields `label`, `description` |
| `engine/metadata-modules/ai/ai-agent/agent.resolver.ts` | `findManyAgents`, `findOneAgent`, `createOneAgent`, `updateOneAgent`, `deleteOneAgent` |
| `engine/metadata-modules/ai/ai-chat/resolvers/*` | `chatThread`, `chatMessages`, `chatStreamCatchupChunks`, `sendChatMessage`, `retryChatMessage`, `stopAgentChatStream`, `deleteQueuedChatMessage`, `openDirectMessage`, `createChatThread`, `agentChatChannels`, `agentChatChannelRoles`, `agentChatChannelSummaries`, `agentChatChannelThreadIds`, `agentChatOpenThreadsSummary`, `updateAgentChatThreadInboxState`, `assignAgentChatThread`, `addAgentChatMessageReaction`, `removeAgentChatMessageReaction`, `aiChatUsage`, `aiChatUserCreditsUsage`, `startWorkspaceSetupChat` |

**Files, records, misc**

`engine/core-modules/file/file-upload/resolvers/file-upload.resolver.ts` (`createFileUpload: FileUploadTargetDTO`,
`completeFileUpload: FileWithSignedUrlDTO`), `engine/core-modules/file/file-core-picture/resolvers/file-core-picture.resolver.ts`
(`uploadProfilePicture`, `uploadWorkspaceLogo`, `completeWorkspaceLogoUpload`), `engine/core-modules/file/files-field/resolvers/files-field.resolver.ts`
(`uploadFilesFieldFileByUniversalIdentifier`), `engine/core-modules/record-export/record-export.resolver.ts`
(`exportRecords` — a **subscription**), `engine/core-modules/event-logs/event-logs.resolver.ts` (`eventLogs`),
`event-logs-live.resolver.ts` (`eventLogsLive` — **subscription**), `engine/core-modules/geo-map/resolver/geo-map.resolver.ts`
(`getAutoCompleteAddress`, `getAddressDetails`), `engine/core-modules/tool-provider/resolvers/tool-index.resolver.ts`
(`getToolIndex`, `getToolInputSchema`), `engine/core-modules/usage-limit/usage-limit.resolver.ts`
(`usageLimits`, `createUsageLimit`, `deleteUsageLimit`, …).

### 3.4 The metadata subscription (event stream)

`src/engine/subscriptions/event-stream.resolver.ts` — `@MetadataResolver()` on `/metadata`:

- Subscription `onEventSubscription(eventStreamId: String!): EventSubscriptionDTO` with payload
  `{ eventStreamId, objectRecordEventsWithQueryIds: [{ objectRecordEvent: { action, objectNameSingular, recordId, userId, workspaceMemberId, properties { updatedFields, before, after, diff } }, queryIds }], metadataEvents: [{ type, metadataName, recordId, updatedCollectionHash, properties }], queueJobEvents: [{ jobId, state, attemptsMade, failedReason, progress, enqueuedAt, startedAt, finishedAt }] }`.
- Mutations `addQueryToEventStream(input: AddQuerySubscriptionInput!)` and `removeQueryFromEventStream(input: RemoveQueryFromEventStreamInput!)`.
- DTOs: `src/engine/subscriptions/dtos/{event-subscription,object-record-event,object-record-event-properties,metadata-event,add-query-subscription.input,remove-query-subscription.input}.dto.ts`.
- Front-end client: `packages/twenty-front/src/modules/sse-db-event/graphql/subscriptions/OnEventSubscription.ts`,
  `.../mutations/AddQueryToEventStreamMutation.ts`, `.../RemoveQueryFromEventStreamMutation.ts`;
  the transport is SSE (`src/modules/sse-db-event/components/SSE*.tsx`).

---

## 4. REST API

### 4.1 Core routes

`src/engine/api/rest/core/controllers/rest-api-core.controller.ts` — `@Controller(ApiPath.Rest)` = `/rest`.

| Method | Path | Handler | Response key |
|---|---|---|---|
| POST | `/rest/batch/*path` | `RestApiCreateManyHandler` | `data.create<Plural>` |
| POST | `/rest/*path` | `RestApiCreateOneHandler` | `data.create<Singular>` |
| POST | `/rest/*path/duplicates` | `RestApiFindDuplicatesHandler` | `data.<singular>Duplicates` (array of connections) |
| GET | `/rest/*path` | `RestApiFindOneHandler` (with id) / `RestApiFindManyHandler` | `data.<singular>` / `data.<plural>` |
| GET | `/rest/*path/groupBy` | `RestApiGroupByHandler` | array of group items |
| PATCH | `/rest/*path` | `RestApiUpdateOneHandler` (with id) / `RestApiUpdateManyHandler` | `data.update<Singular>` / `data.update<Plural>` |
| PUT | `/rest/*path` | same as PATCH (legacy alias) | same |
| PATCH | `/rest/*path/restore` | `RestApiRestoreOneHandler` / `RestApiRestoreManyHandler` | `data.restore<Singular>` / `data.restore<Plural>` |
| PATCH | `/rest/*path/merge` | `RestApiMergeManyHandler` | `data.merge<Singular>` |
| DELETE | `/rest/*path` | `RestApiDeleteOne/Many` (soft) or `RestApiDestroyOne/Many` (hard) | `data.delete<Singular>` / `data.delete<Plural>` |

Path parsing: `src/engine/api/rest/input-request-parsers/path-parser-utils/parse-core-path.utils.ts`
→ `{ object, id? }`; the object segment is the **plural** name (`namePlural`); `id` must be a UUID;
`batch`, `duplicates`, `groupBy`, `merge` are reserved second segments; `/rest/<obj>/<id>/restore` is PATCH-only (max 3 segments).

Object lookup: `RestApiBaseHandler.getObjectMetadata` resolves `idByNamePlural[parsedObject]`; a miss yields
`400 object '<x>' not found. eg: companies` (with a "Did you mean '<plural>'?" hint when the singular matched).

### 4.2 Query parameters

`src/engine/api/rest/input-request-parsers/`

| param | parser | meaning |
|---|---|---|
| `limit` | `limit-parser-utils/parse-limit-rest-request.util.ts` | page size; default `QUERY_DEFAULT_LIMIT_RECORDS = 60`, capped at `QUERY_MAX_RECORDS = 200`; non-integer or negative → `INVALID_LIMIT_QUERY_PARAM` |
| `starting_after` | `starting-after-parser-utils/...` | cursor → `after` |
| `ending_before` | `ending-before-parser-utils/...` | cursor → `before` |
| `depth` | `depth-parser-utils/...` | relation nesting, allowed `0` or `1` |
| `fields` | `fields-parser-utils/parse-fields-rest-request.util.ts` | comma-separated field allowlist; empty → `INVALID_FIELDS_QUERY_PARAM` |
| `order_by` | `order-by-parser-utils/parse-order-by-rest-request.util.ts` | `field1[AscNullsFirst],field2[DescNullsLast]`; default direction `AscNullsFirst`; `id` is appended automatically (`add-default-order-by-id.util.ts`); nested `a.b` supported |
| `filter` | `filter-parser-utils/parse-filter-rest-request.util.ts` | see below |
| `soft_delete` | `soft-delete-parser-utils/...` | `true` → soft delete, else hard delete |
| `upsert` | `upsert-parser-utils/...` | `true` → upsert on create |
| `viewId` | `view-id-parser-utils/...` | group-by view |
| `group_by` | `group-by-parser-utils/...` | JSON array, e.g. `[{"firstField": true},{"dateField":{"granularity":"DAY"}}]` |
| `aggregate` | `aggregate-fields-parser-utils/...` | JSON array of aggregation keys, e.g. `["countNotEmptyId","countEmptyField"]` |
| `include_records_sample` | `group-by-with-records/parse-include-records-sample-rest-request.util.ts` | include a record sample in group-by results |

**Filter DSL** (`filter-parser-utils/`):
- Grammar: `and(...)` / `or(...)` / `not(...)`; leaf = `field[comparator]:value`, e.g. `price[gte]:10`,
  `name[ilike]:%acme%`, `id[in]:["a","b"]`.
- Comparators (`FilterComparators`): `eq, neq, in, containsAny, is, gt, gte, lt, lte, startsWith, endsWith, like, ilike`.
- A missing root conjunction is auto-wrapped in `and(...)` (`add-default-conjunction.util.ts`).
- Bracket balance is validated first (`check-filter-query.util.ts`).
- `not(...)` must contain exactly one condition.
- Nested fields use dots: `field.subfield[op]:value` → nested filter object.
- `in`/`containsAny` require a bracketed array value; quoting is stripped; numbers/booleans are coerced by
  `format-field-values.util.ts` (`NUMERIC` → int, `NUMBER`/`POSITION` → float, `BOOLEAN` → bool).

### 4.3 Response shape

`src/engine/api/rest/core/handlers/rest-api-base.handler.ts` defines `FormatResult`:

```ts
{ data?: { [operation: string]: object }, pageInfo?: PageInfo, totalCount?: number }
```
`PageInfo = { hasNextPage?: boolean; hasPreviousPage?: boolean; startCursor: string|null; endCursor: string|null }`.

- findMany → `{ data: { <plural>: [...] }, totalCount: <number>, pageInfo }`
- findOne → `{ data: { <singular>: {...} } }`
- createOne → `{ data: { create<Singular>: {...} } }`; createMany → `{ data: { create<Plural>: [...] } }`
- updateOne/Many → `update<Singular>` / `update<Plural>`; delete/destroy → `delete<Singular>` / `delete<Plural>`;
  restore → `restore<Singular>` / `restore<Plural>`; merge → `merge<Singular>`
- groupBy → the raw `CommonGroupByOutputItem[]` array (no `data` wrapper)
- records are trimmed to the selected fields by `core/utils/pick-rest-response-fields.util.ts` (always keeps `id`)
- bulk delete/destroy require a non-empty filter → 400 "Filters are mandatory for bulk delete operations."

### 4.4 `/rest/metadata/*` controllers

REST endpoints for metadata entities (each a separate Nest controller; all under
`src/engine/metadata-modules/<module>/controllers/<module>.controller.ts` unless noted):

`/rest/metadata/objects` (object-metadata), `/rest/metadata/fields` (field-metadata), `/rest/metadata/views` (view),
`/rest/metadata/viewFields` (view-field), `/rest/metadata/viewFilters` (view-filter),
`/rest/metadata/viewFilterGroups` (view-filter-group), `/rest/metadata/viewGroups` (view-group),
`/rest/metadata/viewSorts` (view-sort), `/rest/metadata/pageLayouts` (page-layout),
`/rest/metadata/pageLayoutTabs` (page-layout-tab), `/rest/metadata/pageLayoutWidgets` (page-layout-widget),
`/rest/metadata/webhooks` (webhook; also `/rest/webhooks`),
`/rest/apiKeys` (`src/engine/core-modules/api-key/controllers/api-key.controller.ts`; also `/rest/metadata/apiKeys`),
`/rest/dashboards` (`src/modules/dashboard/controllers/dashboard.controller.ts`),
`/rest/front-components`, `/rest/front-component-shared-dependencies`, `/rest/ai-front-components`, `/rest/ai`,
`/rest/sdk-client`.

Metadata REST pagination helpers: `src/engine/api/rest/metadata/utils/paginate-metadata-rest-items.util.ts`,
`paginate-by-id-cursor.util.ts`, `parse-metadata-rest-pagination.util.ts`; response types
`metadata/types/metadata-rest-list-response.type.ts`, `metadata/types/rest-cursor-page-info.type.ts`.

---

## 5. Error codes and format

### 5.1 GraphQL

`src/engine/core-modules/graphql/utils/graphql-errors.util.ts`

Top-level `ErrorCode` enum (goes into `extensions.code`):
`GRAPHQL_PARSE_FAILED, GRAPHQL_VALIDATION_FAILED, UNAUTHENTICATED, FORBIDDEN, PERSISTED_QUERY_NOT_FOUND,
PERSISTED_QUERY_NOT_SUPPORTED, BAD_USER_INPUT, NOT_FOUND, METHOD_NOT_ALLOWED, CONFLICT, TIMEOUT,
INTERNAL_SERVER_ERROR, METADATA_VALIDATION_FAILED, APPLICATION_INSTALLATION_FAILED, RATE_LIMITED, QUOTA_EXHAUSTED`.

Error classes → `extensions.code`: `SyntaxError`→`GRAPHQL_PARSE_FAILED`, `ValidationError`→`GRAPHQL_VALIDATION_FAILED`,
`NotFoundError`→`NOT_FOUND`, `AuthenticationError`→`UNAUTHENTICATED`, `ForbiddenError`→`FORBIDDEN`,
`UserInputError`→`BAD_USER_INPUT`, `MethodNotAllowedError`→`METHOD_NOT_ALLOWED`, `ConflictError`→`CONFLICT`,
`TimeoutError`→`TIMEOUT`, `InternalServerError`→`INTERNAL_SERVER_ERROR`.

Extensions payload:
```json
{ "extensions": { "code": "BAD_USER_INPUT", "subCode": "<exception.code>", "userFriendlyMessage": "<i18n message>" } }
```
`userFriendlyMessage` is a lingui `MessageDescriptor` translated with the request locale
(`src/engine/api/graphql/direct-execution/direct-execution.service.ts#formatError`, and the
`useGraphQLErrorHandlerHook` plugin).

Domain exception codes:
- `CommonQueryRunnerExceptionCode` (`src/engine/api/common/common-query-runners/errors/common-query-runner.exception.ts`):
  `MISSING_FLAT_INDEX_MAPS, RECORD_NOT_FOUND, INVALID_QUERY_INPUT, INVALID_AUTH_CONTEXT, ARGS_CONFLICT,
  INVALID_ARGS_DATA, INVALID_ARGS_FILTER, INVALID_ARGS_FIRST, INVALID_ARGS_LAST,
  UPSERT_MULTIPLE_MATCHING_RECORDS_CONFLICT, MISSING_SYSTEM_FIELD, INVALID_CURSOR, TOO_MANY_RECORDS_TO_UPDATE,
  BAD_REQUEST, INTERNAL_SERVER_ERROR, TOO_COMPLEX_QUERY, MISSING_TIMEZONE_FOR_DATE_GROUP_BY, INVALID_TIMEZONE`.
- `GraphqlQueryRunnerExceptionCode` (`src/engine/api/graphql/graphql-query-runner/errors/graphql-query-runner.exception.ts`):
  `INVALID_QUERY_INPUT, MAX_DEPTH_REACHED, INVALID_CURSOR, INVALID_DIRECTION, UNSUPPORTED_OPERATOR, ARGS_CONFLICT,
  FIELD_NOT_FOUND, MISSING_SYSTEM_FIELD, OBJECT_METADATA_NOT_FOUND, RECORD_NOT_FOUND, INVALID_ARGS_FIRST,
  INVALID_ARGS_LAST, RELATION_SETTINGS_NOT_FOUND, RELATION_TARGET_OBJECT_METADATA_NOT_FOUND, NOT_IMPLEMENTED,
  INVALID_POST_HOOK_PAYLOAD, UPSERT_MULTIPLE_MATCHING_RECORDS_CONFLICT, UPSERT_MAX_RECORDS_EXCEEDED`.
- `AuthExceptionCode` (`src/engine/core-modules/auth/auth.exception.ts`) — e.g. `UNAUTHENTICATED`,
  `USER_WORKSPACE_NOT_FOUND`, `FORBIDDEN_EXCEPTION`, `USER_NOT_FOUND`, `WORKSPACE_NOT_FOUND`, `WORKSPACE_SUSPENDED`,
  `INVALID_INPUT`.
- `RecordFilterExceptionCode` — `UNSUPPORTED_OPERATOR` etc.

Mapping to GraphQL errors: `src/engine/api/common/common-query-runners/utils/common-query-runner-to-graphql-api-exception-handler.util.ts`
→ `RECORD_NOT_FOUND` ⇒ `NotFoundError`; arg/validation codes ⇒ `UserInputError`; `INVALID_AUTH_CONTEXT` ⇒
`AuthenticationError`; `MISSING_FLAT_INDEX_MAPS`/`MISSING_SYSTEM_FIELD`/`INTERNAL_SERVER_ERROR` ⇒ `InternalServerError`.
`src/engine/api/graphql/workspace-query-runner/utils/workspace-query-runner-graphql-api-exception-handler.util.ts`
is the direct-execution entry point.

Auth failures during hydration are written with HTTP **200** and a body of `{ "errors": [...] }`
(`MiddlewareService.writeGraphqlResponseOnExceptionCaught`, `AuthGraphqlApiExceptionFilter`).

### 5.2 REST

`src/engine/api/rest/rest-api-exception.filter.ts` → `HttpExceptionHandlerService.handleError`
(`src/engine/core-modules/exception-handler/http-exception-handler.service.ts`):

```json
{ "statusCode": 400, "error": "BadRequestException", "messages": ["<message>"], "code": "<custom error code>" }
```
`error` is the exception class name, or a name derived from the status code
(400 BadRequest, 401 Unauthorized, 402 PaymentRequired, 403 Forbidden, 404 NotFound, 405 MethodNotAllowed,
409 Conflict, 416 RequestedRangeNotSatisfiable, 422 UnprocessableEntity, 500 InternalServerError).
`QueryFailedError` → 400; `TwentyOrmException` user-input errors → 400; `PostgresException` → 500.

Domain → HTTP mapping (`src/engine/api/common/common-query-runners/utils/common-query-runner-to-rest-api-exception-handler.util.ts`):
`ARGS_CONFLICT, INVALID_ARGS_FIRST, INVALID_ARGS_LAST, INVALID_QUERY_INPUT, INVALID_ARGS_DATA, INVALID_ARGS_FILTER,
UPSERT_MULTIPLE_MATCHING_RECORDS_CONFLICT, INVALID_CURSOR, TOO_MANY_RECORDS_TO_UPDATE, BAD_REQUEST, TOO_COMPLEX_QUERY,
MISSING_TIMEZONE_FOR_DATE_GROUP_BY, INVALID_TIMEZONE` → **400**; `RECORD_NOT_FOUND` → **404** ("Record not found");
`INVALID_AUTH_CONTEXT` → **401**; `MISSING_FLAT_INDEX_MAPS, MISSING_SYSTEM_FIELD, INTERNAL_SERVER_ERROR` → **500**.

`src/engine/api/rest/utils/workspace-query-runner-rest-api-exception-handler.util.ts` also maps
`RestInputRequestParserException` → 400, `CommonSelectFieldsException` → 400 ("'fields' parameter invalid. …"),
`UsageLimitException` / `ThrottlerException` / `RecordValidationRuleException` to their dedicated handlers.

`RestInputRequestParserExceptionCode` (`src/engine/api/rest/input-request-parsers/rest-input-request-parser.exception.ts`)
covers `INVALID_LIMIT_QUERY_PARAM`, `INVALID_DEPTH_QUERY_PARAM`, `INVALID_FIELDS_QUERY_PARAM`,
`INVALID_FILTER_QUERY_PARAM`, `INVALID_ORDER_BY_QUERY_PARAM`, `INVALID_GROUP_BY_QUERY_PARAM`,
`INVALID_AGGREGATE_FIELDS_QUERY_PARAM`, `INVALID_VIEW_ID_QUERY_PARAM`, …

---

## 6. Prioritized list of operations the original web UI actually calls

Sources: `packages/twenty-front/codegen.cjs` (core `/graphql` documents) and
`packages/twenty-front/codegen-metadata.cjs` (`/metadata` documents), plus the runtime-generated record documents
in `packages/twenty-front/src/modules/object-record/**` and `object-metadata/utils/generate*Mutation.ts`.

### P0 — bootstrap / must-have to render the app

| # | Operation | Endpoint | Server file |
|---|---|---|---|
| 1 | `minimalMetadata` | `/metadata` | `engine/metadata-modules/minimal-metadata/minimal-metadata.resolver.ts` |
| 2 | `objects` (query `ObjectMetadataItems`) | `/metadata` | `engine/metadata-modules/object-metadata/object-metadata.resolver.ts` |
| 3 | `currentWorkspace` | `/metadata` | `engine/core-modules/workspace/workspace.resolver.ts` |
| 4 | `currentUser` | `/metadata` | `engine/core-modules/user/user.resolver.ts` |
| 5 | `getViews` | `/metadata` | `engine/metadata-modules/view/resolvers/view.resolver.ts` |
| 6 | `<plural>` (query `FindMany<Plural>`) — **runtime generated** | `/graphql` | `engine/api/common/common-query-runners/common-find-many-query-runner.service.ts` |
| 7 | `getPermissionFlags` | `/metadata` | `engine/metadata-modules/permission-flag/permission-flag.resolver.ts` |
| 8 | `navigationMenuItems` | `/metadata` | `engine/metadata-modules/navigation-menu-item/navigation-menu-item.resolver.ts` |
| 9 | `signIn` / `getLoginTokenFromCredentials` / `getAuthTokensFromOTP` / `getAuthTokensFromLoginToken` | `/metadata` | `engine/core-modules/auth/auth.resolver.ts` |
| 10 | `renewToken`, `signOut` | `/metadata` | `engine/core-modules/auth/auth.resolver.ts` |

### P1 — record CRUD (all runtime-generated documents)

| # | Operation | Shape | Front-end generator |
|---|---|---|---|
| 11 | `createOne<Singular>` | `mutation CreateOne<Singular>($input: <Singular>CreateInput!) { create<Singular>(data: $input) {...} }` | `object-metadata/utils/generateCreateOneRecordMutation.ts` |
| 12 | `create<Plural>` | `mutation Create<Plural>($data: [<Singular>CreateInput!]!, $upsert: Boolean) { create<Plural>(data: $data, upsert: $upsert) {...} }` | `object-record/hooks/useCreateManyRecordsMutation.ts` |
| 13 | `updateOne<Singular>` | `mutation UpdateOne<Singular>($idToUpdate: UUID!, $input: <Singular>UpdateInput!) { update<Singular>(id: $idToUpdate, data: $input) {...} }` | `object-metadata/utils/generateUpdateOneRecordMutation.ts` |
| 14 | `update<Plural>` | `mutation UpdateMany<Plural>($filter: <Singular>FilterInput!, $data: <Singular>UpdateInput!)` | `object-metadata/utils/generateUpdateManyRecordsMutation.ts` |
| 15 | `delete<Singular>` / `delete<Plural>` | `mutation DeleteOne<Singular>($idToDelete: UUID!)`, `mutation DeleteMany<Plural>($filter: <Singular>FilterInput!)` | `object-record/hooks/useDeleteOneRecordMutation.ts`, `useDeleteManyRecordsMutation.ts` |
| 16 | `destroy<Singular>` / `destroy<Plural>` | `mutation DestroyOne<Singular>($idToDestroy: UUID!)`, `mutation DestroyMany<Plural>($filter:)` | `object-record/hooks/useDestroyOneRecordMutation.ts`, `useDestroyManyRecordsMutation.ts` |
| 17 | `restore<Plural>` | `mutation RestoreMany<Plural>($filter: <Singular>FilterInput!)` | `object-record/hooks/useRestoreManyRecordsMutation.ts` |
| 18 | `merge<Singular>` | `mutation Merge<Singular>($ids: [UUID!]!, $conflictPriorityIndex: Int!, $dryRun: Boolean)` | `object-record/hooks/useMergeManyRecordsMutation.ts` |
| 19 | `<singular>` (find one) | `query FindOne<Singular>($objectRecordId: UUID!)` | `object-record/hooks/useFindOneRecordQuery.ts` |
| 20 | `<singular>Duplicates` | `query FindDuplicate<Singular>($ids: [UUID!]!)` | `object-record/hooks/useFindDuplicatesRecordsQuery.ts` |
| 21 | `<plural>GroupBy` | `query GroupBy<Plural>($groupBy: [<Singular>GroupByInput!]!, $filter: <Singular>FilterInput, $orderBy: [<Singular>OrderByWithGroupByInput!], $orderByForRecords: [<Singular>OrderByInput], $viewId: UUID)` | `object-record/utils/generateGroupByRecordsQuery.ts` |
| 22 | aggregate query | `query Aggregate<Plural>($filter: <Singular>FilterInput) { <plural>(filter: $filter) { countNotEmpty<Field> ... } }` | `object-record/utils/generateAggregateQuery.ts` |

### P2 — record sharing, permissions, search, command menu

| # | Operation | Endpoint | Front-end file |
|---|---|---|---|
| 23 | `recordSharing`, `setRecordShare`, `setRecordPrivate`, `removeRecordShare` | `/metadata` | `front/src/modules/object-record/record-sharing/graphql/**` |
| 24 | `recordPermissions` | `/metadata` | `front/src/modules/object-record/record-sharing/graphql/**` |
| 25 | `search` (query `Search`) | `/graphql` (core) | `front/src/modules/command-menu/graphql/queries/search.ts` |
| 26 | `onEventSubscription` (SSE) + `addQueryToEventStream` / `removeQueryFromEventStream` | `/metadata` | `front/src/modules/sse-db-event/graphql/**` |
| 27 | `objectRecordCounts` | `/metadata` | `front/src/modules/object-metadata/graphql/queries.ts` |
| 28 | `workspaceMembers` | `/metadata` | `front/src/modules/workspace-member/graphql/**` |
| 29 | `commandMenuItems` | `/metadata` | `front/src/modules/command-menu-item/graphql/**` |

### P3 — settings screens (data model editing)

| # | Operation | Endpoint | Front-end file |
|---|---|---|---|
| 30 | `createOneObject` / `updateOneObject` / `deleteOneObject` (mutations `CreateOneObjectMetadataItem`, `UpdateOneObjectMetadataItem`, `DeleteOneObjectMetadataItem`) | `/metadata` | `front/src/modules/object-metadata/graphql/mutations.ts` |
| 31 | `createOneField` / `updateOneField` / `deleteOneField` | `/metadata` | `front/src/modules/object-metadata/graphql/mutations.ts` |
| 32 | `createOneIndex` / `deleteOneIndex` | `/metadata` | `front/src/modules/object-metadata/graphql/mutations.ts` |
| 33 | `getViews`, `createView`, `updateView`, `deleteView`, `destroyView` | `/metadata` | `front/src/modules/views/graphql/**` |
| 34 | `getViewFields`, `createViewField`, `updateViewField`, `deleteViewField`, `destroyViewField`, `createManyViewFields` | `/metadata` | `front/src/modules/views/graphql/**` |
| 35 | `getViewFilters`, `getViewFilterGroups`, `getViewGroups`, `getViewSorts` (+ create/update/delete/destroy) | `/metadata` | `front/src/modules/views/graphql/**` |
| 36 | `getPageLayouts`, `getPageLayoutTabs`, `getPageLayoutWidgets` (+ mutations) | `/metadata` | `front/src/modules/page-layout/**` |
| 37 | `getRoles`, `getRole`, `createOneRole`, `updateOneRole`, `deleteOneRole`, `upsertObjectPermissions`, `upsertFieldPermissions`, `upsertPermissionFlags`, `upsertRowLevelPermissionRules`, `updateWorkspaceMemberRole` | `/metadata` | `front/src/modules/settings/**/graphql/**` |
| 38 | `apiKeys`, `createApiKey`, `updateApiKey`, `revokeApiKey`, `assignRoleToApiKey` | `/metadata` | `front/src/modules/settings/**/graphql/**` |
| 39 | `webhooks`, `createWebhook`, `updateWebhook`, `deleteWebhook` | `/metadata` | `front/src/modules/settings/**/graphql/**` |
| 40 | `createFileUpload`, `completeFileUpload` | `/metadata` | `front/src/modules/file/graphql/**` |
| 41 | `metadataTranslations` | `/metadata` | `front/src/modules/settings/**` |
| 42 | `relatedRecordPaths`, `createRelatedRecordPath`, `deleteRelatedRecordPath` | `/metadata` | `front/src/modules/related-record-paths/graphql/**` |
| 43 | validation rules (`createValidationRule`, `deleteValidationRule`, `findValidationRulesReadingField`) | `/metadata` | `front/src/modules/validation-rules/graphql/**` |

### P4 — secondary features (implement later)

`timelineActivityTypes` + `updateTimelineActivityTypeIsActive` + `resetTimelineActivityType`; `logicFunction` CRUD +
`executeOneLogicFunction`; `mcpServer` CRUD + `syncMcpServerTools`; `frontComponent` CRUD; `agent` / `skill` CRUD +
`assignRoleToAgent`; `agentChat*` (threads, channels, messages, reactions) and the `OnAgentChatEvent` subscription;
`messageChannel` / `messageFolder` / `calendarChannel` / `connectedAccount`; `application` install/uninstall/marketplace;
billing (`checkoutSession`, `switchBillingPlan`, `currentBillingSubscription`, `billingPlans`);
`gettingStartedProgress` / `updateGettingStarted`; `useUsageLimit` / `usageLimits`; `getToolIndex` / `getToolInputSchema`;
`getAutoCompleteAddress` / `getAddressDetails`; `trackAnalytics`; `exportRecords` subscription; `eventLogs` / `eventLogsLive`;
workflows (`getCoreWorkflows`, `runCoreWorkflowVersion`, version/step/edge mutations — served by the **core** `/graphql`
schema per `codegen.cjs`).

### Porting notes / gotchas

1. **Two GraphQL schemas, not one.** `/graphql` = core code-first schema + workspace data operations (executed by the
   direct-execution path, not by generated resolvers). `/metadata` = the metadata/admin schema. A query that mixes a core
   field with a workspace field is **rejected** by Twenty (`useDirectExecution.hook.ts`); the Rust port should keep that
   rule or the front-end will silently break.
2. **The front-end never sees a static workspace schema.** It builds the query text at runtime from the metadata it
   fetched (`object-metadata/utils/generate*Mutation.ts`, `object-record/utils/generate*Query.ts`). The Rust port must
   therefore serve a *dynamically generated* schema whose type names match exactly:
   `<Singular>FilterInput`, `<Singular>OrderByInput`, `<Singular>OrderByWithGroupByInput`, `<Singular>GroupByInput`,
   `<Singular>CreateInput`, `<Singular>UpdateInput`, `<Singular>Connection`, `<Singular>Edge`, `PageInfo`.
3. **`totalCount` lives on the Connection type** alongside the per-field aggregations, and `groupByDimensionValues: JSON!`
   on the GroupByConnection.
4. **Filter operators in the schema are wider than at runtime** — `regex`/`iregex` are exposed on `StringFilter` but
   rejected by the SQL builder (`RecordFilterException` `UNSUPPORTED_OPERATOR`).
5. **Null-equivalence widening**: Postgres stores `''` for NULL text and `'{}'` for NULL arrays, so `eq: ''` and
   `eq: []` must also match `IS NULL`; the Rust SQL builder must reproduce this or filters will not match.
6. **Keyset pagination** uses base64url JSON cursors containing the ordered leaf values plus `id`; the `id` tiebreaker
   (`AscNullsLast`) is always appended. `first`+`last` / `before`+`after` combinations are rejected.
7. **`orderBy` must translate nulls-first/nulls-last** into `NULLS FIRST`/`NULLS LAST`, and backward pagination must
   invert both the direction and the nulls placement (`getEffectiveScanOrder`).
8. **Pre-query hooks** are part of the contract: `*.createOne`/`*.createMany` stamp `createdBy`/`updatedBy`;
   `workspaceMember.*` mutations are denied; `timelineActivity.*` stamps a type snapshot.
9. **REST and GraphQL share the same query runners** (`CommonFindManyQueryRunnerService` etc.) — the REST handlers are thin
   adapters over them. Port the runners once, expose both surfaces.
