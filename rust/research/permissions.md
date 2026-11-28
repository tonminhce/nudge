# Twenty CRM — Permission Enforcement (the permission wave)

Research spec for the Rust port (`nudge`). This document is the wave that closes the gap
`query-layer.md §5.3 / §11 D9` left open: `crates/server/src/permissions/` — the enforcement
engine the wave-4 record runner plugs into.

Upstream source (read-only, authoritative): `/Users/tonminh-mac/Documents/GitHub/twenty`
**@ tag `twenty/v2.61.0`** (commit `4bac294`, 2026-11-22, "Emit permissions metrics for adoption,
access group sync, propagation and the read path (#31148)"). Every `file:line` below was read
from `git show twenty/v2.61.0:<path>` on that tag (verified in this session; the working checkout
is 2.65 and NOT used as evidence). Paths are abbreviated as in the sibling docs:
`srv/` = `packages/twenty-server/src`, `shared/` = `packages/twenty-shared/src`.
Enterprise-licensed upstream files are marked **[ENT]** (`/* @license Enterprise */` header,
first line of the file); everything else in the module is CC (**[CC]**).

Prior docs consumed, not restated: `auth-permissions.md` (roles, roleTargets, permissionFlags,
principals, flags — wave 2 landed token/session/role *seeding*, not enforcement),
`query-layer.md` (the wave-4 record runner; §5 describes the permission slots this wave fills;
D6/D9 referenced below), `workspace-migrations.md §5.9` (the enrichment columns),
`data-model.md` (physical schema). HANDOFF.md's gap list: "`/graphql` + `/metadata` answers are
the wave 4 gap" and the wave-2 note that permission *enforcement* moves to wave 4 "with the
query runner" — which the wave-4 spec then deferred to *this* wave (query-layer.md §5.3, D9).

Reference environment (for the acceptance gates): live Node reference `twenty-ref`
(`twentycrm/twenty:v2.61.0`, port 3001) with database `twenty_ref`
(`docker exec infra-db-1 psql -U postgres -d twenty_ref`); Nudge on :3000; parity harness
`rust/tools/parity-compare.py`. The reference workspace schema observed in this session is
`workspace_6n57znb4w89f8r7xudebmf0`.

---

## 0. The headline finding: the license boundary is the feature boundary

At `twenty/v2.61.0`, `srv/engine/permissions/` contains **367 non-test files: 121 CC and
246 [ENT]** (measured from the first line of `git show twenty/v2.61.0:<file>` for each file in
the tag tree). The split is not incidental:

- **The record-read orchestrator is [ENT].** `srv/engine/permissions/record-permissions-read.service.ts:1`
  is `/* @license Enterprise */`; so are `record-field-permissions-read.service.ts:1`,
  `record-event-permissions-read.service.ts:1`, `principal-permissions-read.service.ts`,
  `user-workspace-permissions-read.service.ts`, `record-access-group-write.service.ts`, and
  `record-sharing-permissions.service.ts`.
- **The entire SQL row-access engine is [ENT]:** `internal/combine/utils/build-access-sql-condition.util.ts:1`,
  `resolve-row-access-outcome.util.ts:1`, `internal/rls/utils/build-rls-access-sql-condition.util.ts:1`,
  `build-rls-rule-sql-condition.util.ts:1`, `render-rls-record-filter-to-sql.util.ts:1`,
  `internal/private-record/utils/build-private-record-state-sql-condition.util.ts:1`,
  `internal/access-group/utils/build-principals-access-groups-sql-condition.util.ts:1`,
  `internal/record-share/utils/build-record-share-sql-condition.util.ts:1` — all [ENT].
- **The *kernel* of role→object/field permission computation is [CC]:**
  `internal/role/utils/compute-permission-union.util.ts`, `compute-permission-intersection.util.ts`,
  `validate-operation-is-permitted-or-throw.util.ts`, `get-enforced-operation-type.util.ts`,
  `follow-update-when-create-permission-is-disabled.util.ts`, `is-object-operation-disabled.util.ts`,
  `restrict-records-permissions-to-disabled-actions.util.ts`,
  `restrict-records-permissions-to-runtime-limiting-role.util.ts`,
  `role/utils/compute-role-records-permissions.util.ts`, `role-permissions.service.ts`,
  `setting-permissions.service.ts`, `permissions.exception.ts`, the metadata-plane guard stack
  (`srv/engine/guards/access-policy.guard.ts`, `guards/access-policy/access-policy.service.ts` — CC),
  the record-permissions GraphQL facade (`record-permissions/record-permissions.resolver.ts`, CC),
  the sharing DTOs (`record-share/dtos/record-sharing.dto.ts`, CC),
  `role-assignment/role-assignment-write.service.ts` (CC), and the workspaceMember-individual-role
  utils (CC).
- But the *wiring* around the [CC] kernel is [ENT]: `internal/role/utils/compute-principals-records-permissions.util.ts:1`,
  `get-permissions-principals-records-permissions.util.ts:1`,
  `internal/combine/services/permissions-metadata-maps.service.ts:1`,
  `is-object-operation-disabled-to-principals.util.ts:1`, `is-object-operation-denied-to-principals.util.ts:1`,
  `can-any-source-access-operation.util.ts:1`, `internal/role/utils/build-record-read-restrictions.util.ts:1`,
  every principal type/util (`internal/principal/*`, [ENT]), and the whole `internal/access-group/*`
  service layer and `internal/record-share/services/*` [ENT].

Upstream ships all of it compiled into the official self-hosted Docker image — verified live this
session (`/app/packages/twenty-server/dist/engine/permissions/internal/combine/utils/build-access-sql-condition.util.js`
exists inside `twenty-ref`); the `@license Enterprise` marker governs *source availability to
forks*, not runtime gating. What gates the features at runtime on a self-hosted instance is:

1. **Feature flags** (`core.featureFlag`, auth-permissions.md §3.6) — all off by default for the
   permission features: `isAccessGroupsEnabled` reads `IS_ACCESS_GROUPS_ENABLED ?? false`
   (`internal/access-group/utils/is-access-groups-enabled.util.ts:5-10` **[ENT]**),
   `isPrivateRecordsEnabled` = `isPrivateByDefaultObject(obj) || (IS_PRIVATE_RECORDS_ENABLED ?? false)`
   (`internal/private-record/utils/is-private-records-enabled.util.ts:8-16` **[ENT]**),
   `isRecordSharingEnabled` = `IS_RECORD_LEVEL_SHARING_ENABLED ?? false`
   (`record-share/utils/is-record-sharing-enabled.util.ts:5-10` **[ENT]**),
   `isRelatedRecordPathsEnabled` = `IS_RELATED_RECORD_PATHS_ENABLED ?? false`
   (`internal/related-record-path/utils/is-related-record-paths-enabled.util.ts:5-10` **[ENT]**),
   `isRlsRelationHopEnabled` = `IS_ROW_LEVEL_PERMISSION_RELATION_HOP_ENABLED ?? false &&
   isAccessGroupsEnabled` (`internal/rls/utils/is-rls-relation-hop-enabled.util.ts:7-14` **[ENT]**).
   `DEFAULT_FEATURE_FLAGS` at activation enables none of them
   (`srv/engine/workspace-manager/workspace-migration/constant/default-feature-flags.ts:3-9`).
2. **Billing entitlements + the enterprise key.** `hasEntitlement` returns **true when billing is
   disabled** (`core-modules/billing/services/billing.service.ts:129-143` — `if
   (!isBillingEnabled) return true`), and `CUSTOM_ROLES` is granted even without billing
   (`core-modules/billing/constants/entitlement-keys-granted-without-billing.constant.ts:6-8`
   **[ENT]**). RLS *creation* is additionally gated by a valid enterprise plan:
   `hasRowLevelPermissionFeature = enterprisePlanService.isValid() && hasEntitlement(RLS)`
   (`metadata-modules/row-level-permission-predicate/services/row-level-permission-predicate.service.ts:515-525`
   **[ENT]**), enforced with `ROW_LEVEL_PERMISSION_FEATURE_DISABLED` before any rule upsert
   (`:89`). `EnterprisePlanService.isValid()` requires a valid enterprise validity token
   (`core-modules/enterprise/services/enterprise-plan.service.ts:214-216` **[ENT]**), which a
   plain self-host has none of.

**Consequences for the boundary** (each with the upstream evidence):

- **Object- and field-level permissions (roles, objectPermission, fieldPermission, disabled
  actions) are fully self-hostable and on by default**: the computation kernel is [CC], the
  custom-role entitlement passes with `CUSTOM_ROLES` (above), and `validate-operation-is-permitted-or-throw.util.ts`
  (CC) is what the repository actually calls on every query/mutation. **Nudge implements these
  completely** — they are the observable permission behavior of a stock self-hosted Twenty.
- **Row-level rules (RLS) can be *computed* in CC code** (`compute-role-records-permissions.util.ts:253-286`
  attaches `rowLevelRecordsPermissions` to the role's permissions regardless of license), and the
  *degradedRead* strategy renders RLS predicates directly to SQL
  (`build-rls-access-sql-condition.util.ts:28-34`: when `principalsAccessGroupCompactIds` is
  undefined — i.e. `IS_ACCESS_GROUPS_ENABLED` off, `resolve-records-access-sql-condition-strategy.util.ts:8-31`
  **[ENT]** → strategy `degradedRead` → `renderRlsRecordFilterToSql`) — **but writing RLS rules
  requires an enterprise plan** (`row-level-permission-predicate.service.ts:515-525`). A
  self-host without an enterprise key therefore has: no way to *create* rules through the API,
  and the rule engine inert. Channel visibility is *not* an RLS rule at this commit: there is no
  `engine/permissions/internal/channel-visibility/` module in 2.61.0 (only
  `engine/core-modules/messaging/utils/get-effective-message-channel-visibility.util.ts`, an
  Apache label resolver used by `timeline-messaging.service.ts:238,839`). Enforcement is the
  Apache post-query field-redaction layer
  (`engine/api/common/common-record-visibility/common-record-visibility.service.ts:16`, called
  unconditionally from `common-base-query-runner.service.ts:343`), which blanks
  `message.subject/text/attachments`, `attachment`, `calendarEvent` and
  `timelineActivity.properties` for members who do not own the connected account
  (§2.10).
- **`_isPrivate`, `_accessGroupIds`, recordShare reads, related-record paths** are each
  flag-gated AND [ENT]-implemented. On a stock self-host all four flags are false →
  `buildAccessSQLCondition` degenerates (§2.2) and the read path reduces to the object-level role
  decision. The **storage side** of the same features, however, is self-hosted-on-by-default
  schema: `createOneObject` enriches every workspace table with `_isPrivate`/`_accessGroupIds`
  (+ derived indexes) unconditionally at the call site — `create-object-action-handler.service.ts:188-197`
  [CC] calls `enrichRecordTableSchemaWithAccessGroupIds`/`enrichAccessGroupTableSchema` (CC) and
  `enrichRecordTableSchemaWithIsPrivate` (`private-record/utils/enrich-record-table-schema-with-is-private.util.ts`
  [ENT] but *called*, not gated, by the CC handler; `workspace-migrations.md §5.9`), and Nudge's
  wave-3 U7 already creates the `accessGroup`/`recordShare` tables
  (`crates/server/src/standard_objects/permissions.rs:11-29`, matching the live reference DDL in
  §4.1). The two "private by default" objects — `agentChatThread`, `agentChatThreadParticipant`
  (`internal/private-record/constants/private-by-default-object-universal-identifiers.constant.ts:5-8`
  **[ENT]**) — *are* in Nudge's seeded standard-object catalog (`definitions.rs`, generated from
  2.61.0 dumps), so the columns and rows exist locally even though enforcement is off.

**Nudge decision (D0, restated in §8): implement CC-license-equivalent semantics exactly as a
stock self-hosted Twenty v2.61.0 exhibits them** — object/field/disabled-action permissions fully
enforced; the row-access predicate slot live and correct; `_isPrivate`/`_accessGroupIds`/shares/
related-paths implemented behind their upstream feature flags (default off ⇒ the slot emits no
condition, identical to upstream self-host today); and **RLS-rule *enforcement* implemented
(degradedRead path, no access-group materialization) while RLS-rule *creation* is gated on the
same `BillingEntitlementKey.RLS && enterprise-validity` conjunction with Nudge's own billing stub
as the single decision point (§8 D3)**. Enterprise-only machinery (access-group sync/propagation,
related-record paths, `shareWith`, member-successor share transfer, permission tables viewer) is
documented deferred parity with the [ENT] file evidence above — never a silent placeholder.

---

## 1. The enforcement choke point (what wave-4 must call, and where)

### 1.1 Upstream: one entry, three ordered passes

`WorkspaceRepository.onBeforeExecute` (`srv/engine/twenty-orm/repository/workspace-repository.ts:1929-1935`
[CC file]) is the single entry into the read path:

```ts
1929  private async onBeforeExecute(
1930    queryBuilder: WorkspaceSelectQueryBuilder,
1931  ): Promise<void> {
1932    await this.applyRowLevelPermissionPredicates(queryBuilder);
1933    await this.validateQueryIsPermitted(queryBuilder);
1934    await this.makeRlsRuleFieldsUnreadable(queryBuilder);
1935  }
```

It is wired into the select builder as a context callback at repository construction
(`workspace-repository.ts:191-204`, the `onBeforeExecute:` slot at `:200`) and fired by every
execution point of `WorkspaceSelectQueryBuilder`
(`srv/engine/twenty-orm/query-builder/workspace-select-query-builder.ts` [CC file]):
`executeSelect` calls it first (`:1305-1309`), `getCount` runs it before building the count
statement (`:717-720`), and the public `applyRowLevelPermissions()` seam exposes it for
write-side reuse (`:711-715`). A repository built with
`onBeforeExecute: () => undefined` (`:1469`, `findRecordSnapshotsBypassingPermissions`) reads
without permission checks.

The three passes (line numbers at the tag):

1. **`applyRowLevelPermissionPredicates(qb, operationType = 'read')`**
   (`workspace-repository.ts:2062-2093`): returns early when
   `options.shouldBypassPermissionChecks` (`:2066-2068`; the flag is computed as
   `shouldBypass || isSystemAuthContext(authContext)` at
   `datasource/workspace-data-source.ts:250-251` [CC file]). Applies the predicate to the **main
   alias** (`:2070-2075`) and to every **join alias read-only** (`:2077-2092`, joins always get
   `operationType: 'read'`). Per alias:
   - `applyRowLevelPermissionPredicateForAlias` (`:2095-2133`): idempotence guard
     `markRowLevelPermissionApplied(alias)` (`:2106-2108`, builder set at `:108`, method
     `workspace-select-query-builder.ts:825-833`); builds the condition; `true` → nothing,
     `false` → `denyAccessForAlias` (`:2121-2128`), else `addConditionForAlias` (`:2132`).
   - `buildPermissionsConditionForAlias` (`:2135-2174`): for the **main alias only**, first
     `recordPermissions.assertCanRunOperationOrThrow({principals, objectMetadataId,
     operationType, selectedColumns: []})` (`:2146-2155`) — the object-level gate throws here for
     writes and reads alike; then `read` → `buildReadRecordsAccessSQLCondition` (`:2160-2166`),
     write → `buildWriteRecordsAccessSQLCondition` (`:2168-2173`).
   - `denyAccessForAlias` (`:2270-2289`): main alias → throw `PermissionsException(PERMISSION_DENIED)`;
     join alias → `addJoinCondition(alias, ALWAYS_FALSE_CONDITION)` with
     `ALWAYS_FALSE_CONDITION = '1=0'` (`:99`) — the joined relation reads empty rather than failing
     the parent query.
   - `addConditionForAlias` (`:2291-2310`): main alias → `queryBuilder.addRowAccessCondition(sql,
     parameters)` (`workspace-select-query-builder.ts:235-248` — pushes onto the private
     `rowAccessConditions: string[]`, `:93`); join/exists alias → `addJoinCondition(alias, sql)`
     (`:797-812`, appended to the join's `additionalOnConditions`).

   The write-side variant is reached through `applyWriteRowLevelPermissions(qb, kind)`
   (`workspace-repository.ts:230-238`) with `OPERATION_TYPE_BY_MUTATION_KIND`
   (`:110-118`: delete→destroy, restore→softDelete, soft-delete→softDelete, update→update — note
   restore is gated as **softDelete**).

2. **`validateQueryIsPermitted(qb)`** (`:2019-2060`): bypass check (`:2022-2024`), then per alias
   from `qb.getReferencedColumnNamesByAlias()` (`:2026`, builder `:835+`): skip unknown shapes
   (`:2034-2036`); **`recordPermissions.isReadDisabled(question)` short-circuits** (`:2044-2050`)
   — a disabled main alias already threw in pass 1, and a disabled join reads empty (the comment at
   `:2043`); otherwise `assertCanRunOperationOrThrow({operationType:'read', selectedColumns})`
   (`:2052-2058`) — the column-level read check against `unreadableFields`.

3. **`makeRlsRuleFieldsUnreadable(qb)`** (`:1937-2017`): bypass check (`:1940-1942`); for main +
   join aliases with `markUnreadableColumnsApplied(alias)` idempotence (`:1952-1958`, builder
   `:732-740`): `recordFieldPermissions.buildUnreadableFieldsSQLConditions({principals,
   objectMetadataId, tableAlias})` (`:1960-1967`) → per condition group, map fieldMetadataIds to
   physical columns via `getColumnNameToFieldMetadataIdMap` (`:1977-1982`) and install
   `qb.makeColumnsUnreadableWhen({alias, columnNames, condition, placeholderColumnNames})`
   (`:1994-2005`; builder `:763-795` — records `unreadableColumnConditions[alias][column]`,
   placeholder params) and `qb.makeRelationsUnreadableWhen({alias, relationFieldNames, condition})`
   (`:2006-2014`; builder `:742+`). Only RLS-scoped field masks go through here; the *role-level*
   unreadable fields are excluded earlier, at selection time (§3.2).

**Where row access lands in the SQL:** `renderWhereExpressionWithRowAccess`
(`srv/engine/twenty-orm/sql/utils/build-select-statement.util.ts:541-566` [CC file], called at
`:692`, fed by `rowAccessConditions` at `:704-705`): empty list → bare user expression
(`:555-557`); otherwise `(userExpr) AND (cond) AND …` — the comment at `:539-540` states the
invariant: *"Row access conditions stay outside the user expression so an OR in it can never
bypass them."*

### 1.2 The write-side gates (not in `onBeforeExecute`)

All in `workspace-repository.ts` [CC file]; the service methods they call are [ENT]:

| Gate | Site | Effect |
|---|---|---|
| `validateWriteIsPermitted({operationType, columnsToReturn, updatedColumns})` | insert: `:1035-1040`; batch-update: `:1207`; performMutation: `:1585-1593`; method body `:1807-1830` (bypass `:1816-1818`, then `recordPermissions.assertCanRunOperationOrThrow` `:1820-1830`) | object-level + column-level (read of returned columns + uneditable of updated columns) |
| `assertCreatedRecordsMatchRowLevelPermissionsOrThrow` | `:1041-1047` (only when not bypassing) | created rows must satisfy the RLS insert shape (§2.6) |
| `resolveWritableRecordIds({ids, operationType})` | `:1484-1507` — builds a `withDeleted()` select, runs `applyRowLevelPermissionPredicates(qb, operationType)` on it (`:1499`), returns the surviving id set; batch-update call site `:1125` | filters non-accessible rows out of batch updates |
| `assertUpdatesKeepRecordAccessOrThrow` (a.k.a. `validateRLSPredicatesForUpdatedRecords`) | `:1837-1851` → `recordPermissions.assertUpdatesKeepRecordAccessOrThrow` | update must not move a record outside its RLS rule set (§2.6) |
| RLS re-validation for filtered mutations | `:1231` (`applyRowLevelPermissionPredicates` before the UPDATE) | the UPDATE's WHERE gains the row-access condition |
| `shouldWriteAtLeastOneRecord` → `RECORD_NOT_EDITABLE` | `runMutation` `:1509-1544`: 0 rows written but a readable record matched → throw (`:1536-1541`, message `permissions.exception.ts:120-121`) | distinguishes "not yours" from "not there" |
| `recomputeRecordAccessGroupIdsAfterWrite` | `:1078`, `:1275`, `:1725`; method `:1853+` | the [ENT] access-group maintenance hook — Nudge keeps the seam (§8 D4) |
| `isReadDisabled()` / `filterRecordIdsWithReadableRelation()` | `:2237-2246` / `:2176-2235` | used by nested relation loads and relation reads (see 1.4) |
| `resolvePermissionsPrincipalsOrThrow` | `:2248-2259` — undefined principals (non-system context without them) → `PermissionsException(PERMISSION_DENIED)` | fail-closed |

### 1.3 Context plumbing — where principals/restrictions come from per request

`WorkspaceOrmManager.loadWorkspaceContext` (`srv/engine/twenty-orm/workspace-orm.manager.ts` [CC
file]) resolves the two permission inputs **once per request**:
`permissionsPrincipals = principalPermissionsReadService.resolvePermissionsPrincipals({authContext})`
(`:142-145`) and `recordReadRestrictions = permissionsPrincipals ?
recordPermissionsReadService.findRecordReadRestrictions({principals}) :
NO_RECORD_READ_RESTRICTIONS` (`:146-150`). The `lite` context
(`loadLiteWorkspaceContext` `:179-217`) sets `permissionsPrincipals: undefined` +
`recordReadRestrictions: NO_RECORD_READ_RESTRICTIONS` (`:212-213`).
`WorkspaceDataSourceService.buildInternalContext` (`datasource/workspace-data-source.service.ts`
[CC file]) carries them into every repository (`:60-61`) along with the three record-permission
services (`recordPermissions`/`recordFieldPermissions`/`recordAccessGroupWrite`, `:64-67`).
Repository construction: `shouldBypassPermissionChecks` per repo
(`workspace-data-source.ts:63/:78/:134/:149/:196-203/:250-251/:261/:274/:284-285`).
The repository getter used *without* permission checks for internal writes is
`getRepositoryWithoutPermissionChecks` (`:72-79`).

`NO_RECORD_READ_RESTRICTIONS = {unreadableObjectMetadataIds: ∅, unreadableFieldMetadataIds: ∅}`
(`constants/no-record-read-restrictions.constant.ts:1-4` [CC]).

### 1.4 Consumers outside the repository (same choke-point contract)

- **Field selection at the API boundary drops unreadable fields before SQL is built**: the runner
  passes `recordReadRestrictions.unreadableFieldMetadataIds` into
  `CommonSelectFieldsBuilder` (`srv/engine/api/common/common-select-fields/common-select-fields-builder.ts:119`
  [CC file]: `.filter(flatField => !unreadableFieldMetadataIds.has(flatField.id))`; also
  `build-field-selection.util.ts:55-60`, relation selection
  `build-relation-selection-from-depth.util.ts:71`).
- **Filter and order-by references to unreadable fields THROW**:
  `assertFieldIsReadableOrThrow` (`srv/engine/api/common/common-query-parsers/utils/assert-field-is-readable-or-throw.util.ts:12-28`
  [CC file]; the comment at `:11`: filtering/ordering by a non-readable field would leak its
  values because orderBy embeds them into cursors) — called from the filter field parser
  (`common-query-filter-field.parser.ts:119-123`) and order-leaf resolution
  (`srv/engine/api/utils/resolve-order-by-leaves.utils.ts:135-139, :248`).
- **Nested to-many relation loads read empty when the target's read is disabled**:
  `process-nested-relations.helper.ts:202-216` (`isReadDisabled()` → empty aggregate; the comment
  at `:202` — "a joined relation to such records reads as empty, so a nested one does too instead
  of failing the parent").
- **The REST/OpenAPI field picker and group-by** consult the same restriction set
  (`group-by-tool.json-schema.ts:74-84`, `record-filter.json-schema.ts:39-57`,
  `common-group-by-query-runner.service.ts:285`,
  `generate-update-many-record-input-schema.util.ts:11-22`).
- **Event payloads omit unreadable fields** (§3.5).

### 1.5 What Nudge's wave-4 runner must call (the seam, concretely)

`query-layer.md §5.1/§5.3` already requires the runner to keep the `(userWhere) AND
(<rowAccess>)` shape and an `onBeforeExecute` slot. This wave fills it. The Rust equivalents
(new module `crates/server/src/permissions/`, no wave-4 file edits):

```
query::runner::*            calls, exactly once per select/mutation execution:
  permissions::row_access::apply_read(ctx, qb_alias)        // pass 1 for reads: returns
                                                            // NoCondition | Condition{sql,params} | Deny
  permissions::row_access::apply_write(ctx, qb, op)         // pass 1 for update/destroy/softDelete
                                                            // (also via resolve_writable_record_ids)
  permissions::validate::query_permitted(ctx, alias_cols)   // pass 2: per-alias column read check
  permissions::field_masks::apply(ctx, qb)                  // pass 3: unreadable-column/relation masking
  permissions::validate::write_permitted(ctx, op, cols, updated)  // before every insert/update/mutation
  permissions::validate::created_match_rls / updates_keep_access // the two RLS write assertions
  permissions::restrictions::for(ctx)                       // role-level RecordReadRestrictions for
                                                            // select-fields, filter, order (query §3.2)
```

Everything above takes a `PermissionsContext` built by this crate (§7 P1) — never re-derives from
the HTTP layer. The runner's existing bypass concept (`shouldBypassPermissionChecks`,
query-layer.md §2.1: system auth context + `getRepositoryWithoutPermissionChecks`) already exists
in the wave-4 `QueryContext`; `permissions/` must honor it at each entry (mirror
`workspace-repository.ts:2066-2068, :2022-2024, :1940-1942` semantics).

---

## 2. Row-level semantics — the full predicate model

### 2.1 The permission algebra (types, [CC] unless noted)

`ObjectRecordsOperationType = 'create' | 'read' | 'update' | 'softDelete' | 'destroy'`
(`shared/src/types/ObjectRecordsOperationType.ts:1-6`).

Per-object permissions (`shared/src/types/ObjectRecordsPermissions.ts:3-13`):

```
ObjectRecordsPermissions {
  canCreateAllRecords: boolean
  canReadAllRecords: boolean
  canUpdateAllRecords: boolean
  canSoftDeleteAllRecords: boolean
  canDestroyAllRecords: boolean
  unreadableFields: string[]           // fieldMetadataIds hidden for read
  uneditableFields: string[]           // fieldMetadataIds hidden for write
  rowLevelRecordsPermissions?: RowLevelRecordsPermissions[] | null
  canReadSomeRecords?: boolean
}
```

`RoleRecordsPermissions` (`srv/engine/permissions/types/role-records-permissions.type.ts:3-9`)
wraps `canRead/Update/SoftDelete/DestroyAllRecords` (role-wide) +
`objects: Record<objectMetadataId, ObjectRecordsPermissions>`.
`RowLevelRecordsPermissions` (`shared/src/types/RowLevelRecordsPermissions.ts:4-15`):
`{ruleId, canCreateRecords, canReadRecords, canUpdateRecords, canSoftDeleteRecords,
canDestroyRecords, predicates[], predicateGroups[], unreadableFields[], uneditableFields[]}`.
`SQLCondition = {sql: string, parameters: Record<string, unknown>}`
(`srv/engine/permissions/types/sql-condition.type.ts`, [CC]).

### 2.2 `buildAccessSQLCondition` — the row-access condition (the shape Nudge reproduces)

`srv/engine/permissions/internal/combine/utils/build-access-sql-condition.util.ts:18-53` **[ENT]**
is the whole model in 35 lines:

```ts
18  export const buildAccessSQLCondition = ({tableAlias, ...request}) => {
28    const outcome = resolveRowAccessOutcome(sqlRequest);      // fast paths: false | true
30    if (isDefined(outcome)) return outcome;
34    const allRecordsOrRlsSQLCondition = combineSQLConditionOutcomesWithOr([
35      canAllRecordsAccessOperation(sqlRequest),               // true  → unconditional allow
36      buildRlsAccessSQLCondition(sqlRequest),                 // SQL | false
37    ]);
39    return combineSQLConditionOutcomesWithOr([
40      arePrivateRecordsExcludedFromAllRecordsAndRls(sqlRequest)
41        ? combineSQLConditionOutcomesWithAnd([
42            allRecordsOrRlsSQLCondition,
43            buildPrivateRecordStateSQLCondition({tableAlias, isPrivate: false}),  // AND NOT private
44          ])
45        : allRecordsOrRlsSQLCondition,
49      buildRecordShareAccessSQLCondition(sqlRequest) ?? false,        // shares path
50      buildPrivateRecordAccessSQLCondition(sqlRequest) ?? false,      // creator-of-own-private
51      buildRelatedRecordPathAccessGroupsSQLCondition(sqlRequest) ?? false,
52    ]);
53  };
```

`combineSQLConditionOutcomesWithOr` / `…WithAnd`
(`internal/record-permission-repository/utils/combine-sql-condition-outcomes-with-{or,and}.util.ts`)
merge `boolean | SQLCondition` outcomes: any `true` → `true`; all `false` → `false`; else one
merged `SQLCondition` (OR joins `(...) OR (...)`, AND joins `(...) AND (...)`, parameters merged).
The repository maps the tri-state result (§1.1: `true` no condition, `false` deny/1=0, condition
appended).

**Fast path — `resolveRowAccessOutcome`** (`internal/combine/utils/resolve-row-access-outcome.util.ts:9-31`
**[ENT]**): returns `false` when `isObjectOperationDisabledToPrincipals(request)` or
`isObjectOperationDeniedToPrincipals(request)`; returns `true` when the operation is reachable
through record share AND the request carries a `hypotheticalRecordShareAccessLevel` that satisfies
the requirement (used to pre-approve rows being created with `shareWith`); else `undefined`
(evaluate the SQL).

- `isObjectOperationDisabledToPrincipals` (`internal/role/utils/is-object-operation-disabled-to-principals.util.ts:8-24`
  **[ENT]**): the *disabled actions* of the object metadata (application/system lockdown, §3.4)
  for the enforced operation, AND the caller does not own the object
  (`isPermissionsPrincipalsOwningObject`, `internal/principal/utils/is-permissions-principals-owning-object.util.ts:9-26`:
  acting application's id == `objectMetadata.applicationId`). Underlying [CC] piece:
  `isObjectOperationDisabled` maps operationType→action name and checks
  `objectMetadata.disabledObjectPermissionActions` (`is-object-operation-disabled.util.ts:6-35`).
- `isObjectOperationDeniedToPrincipals` (`internal/combine/utils/is-object-operation-denied-to-principals.util.ts:8-13`
  **[ENT]**): `true` when a limiting role (agent / application role) does NOT reach all records
  (`areLimitingRolesAccessingAllRecords`, `internal/role/utils/are-limiting-roles-accessing-all-records.util.ts:12-43`
  **[ENT]** — every limiting role and the runtime-limiting role must grant
  `canAccessThroughAllRecordsPermission` for the enforced op), OR when no source can access the
  operation (`canAnySourceAccessOperation`, `internal/combine/utils/can-any-source-access-operation.util.ts:9-19`
  **[ENT]**: `canAllRecordsAccessOperation || canRlsAccessOperation ||
  canRecordShareAccessOperation || canPrivateRecordAccessOperation`) and related-record paths
  cannot extend access.
- `canAllRecordsAccessOperation` (`internal/all-records/utils/can-all-records-access-operation.util.ts:9-24`
  **[ENT]**): role-union (`getPermissionsPrincipalsRecordsPermissions`) → object permissions →
  `canAccessThroughAllRecordsPermission({objectRecordsPermissions, operationType:
  getEnforcedOperationType(...)})` (CC matcher
  `shared/src/utils/canAccessThroughAllRecordsPermission.ts:6-40`, a 5-case switch).

**Enforced operation aliasing (CC):** `getEnforcedOperationType`
(`internal/role/utils/get-enforced-operation-type.util.ts:6-16`): `create` behaves as `update`
unless `IS_CREATE_RECORDS_PERMISSION_ENABLED` (default false — the flag is in `FeatureFlagKey`
but not in `DEFAULT_FEATURE_FLAGS`, auth-permissions.md §3.6) — i.e. **create follows update**
today. `follow-update-when-create-permission-is-disabled`
(`follow-update-when-create-permission-is-disabled.util.ts:27-52`) applies the same aliasing to
the DTO returned to the front end (copies `canUpdateAllRecords`→`canCreateAllRecords` and per-rule
flags, incl. RLS rules).

**Self-host reduction (what Nudge executes when all four flags are off):** with
`IS_PRIVATE_RECORDS_ENABLED`, `IS_RECORD_LEVEL_SHARING_ENABLED`, `IS_RELATED_RECORD_PATHS_ENABLED`
false and no enterprise-plan RLS rules, `buildAccessSQLCondition` collapses to
`OR(canAllRecordsAccessOperation, false /*rls*/, false, false, false)`:

- role grants all-records for the operation → `true` → **no row-access condition at all**
  (repository `:2117-2119` returns, SQL keeps the bare user WHERE).
- role does not → `resolveRowAccessOutcome` already returned `false` (denied: not reachable
  through any source, `canAnySourceAccessOperation` false) → throw on main alias / `1=0` on joins.
  (Object-level read *disabled* vs *denied*: disabled = the API refuses the operation entirely
  with PERMISSION_DENIED from `assertCanRunOperationOrThrow`; denied = no source reaches it.)

So the *observable* self-hosted semantics = **object-level allow-all or deny** — exactly what
query-layer.md D9 assumed for Admin/Member — and Nudge's P1/P2 units make it a real, general
engine rather than an assumption.

### 2.3 RLS rules: the data model (all [ENT] on the metadata side)

Entities (all `core.*`, all carrying `/* @license Enterprise */`):

| Table | File | Columns (condensed) |
|---|---|---|
| `rowLevelPermissionRule` | `metadata-modules/row-level-permission-predicate/entities/row-level-permission-rule.entity.ts:21-56` | `roleId`, `objectMetadataId`, `canReadRecords`, `canCreateRecords`, `canUpdateRecords`, `canSoftDeleteRecords`, `canDestroyRecords` (all `boolean default false`) |
| `rowLevelPermissionPredicateGroup` | `.../row-level-permission-predicate-group.entity.ts` | `logicalOperator: AND\|OR` (`shared/src/types/RowLevelPermissionPredicateGroupLogicalOperator.ts:3-6` **[ENT]**), `parentRowLevelPermissionPredicateGroupId` (nesting), `positionInRowLevelPermissionPredicateGroup`, per-op grant flags `canRead/Update/SoftDelete/DestroyRecords`, `roleId`, `objectMetadataId` |
| `rowLevelPermissionPredicate` | `.../row-level-permission-predicate.entity.ts:33-187` **[ENT]** | `fieldMetadataId`, `relationFieldMetadataId?` (hop via a relation, 2.55), `objectMetadataId`, `operand` (enum, default `CONTAINS`, `:97-103`), `value jsonb` (`:105-106`), `subFieldName` (`:108-109`), `workspaceMemberFieldMetadataId` + `workspaceMemberSubFieldName` + `workspaceMemberRelationFieldMetadataId` (`:111-139`, compare against the acting member — 2.61), `rowLevelPermissionPredicateGroupId` (`:141-142`), `rowLevelPermissionRuleId` (`:144-149`), `positionInRowLevelPermissionPredicateGroup double precision` (`:158-159`), `roleId` (`:161-162`) |

**Operand enum** (`shared/src/types/RowLevelPermissionPredicateOperand.ts:3-20` **[ENT]**):
`IS, IS_NOT_NULL, IS_NOT, LESS_THAN_OR_EQUAL, GREATER_THAN_OR_EQUAL, IS_BEFORE, IS_AFTER,
CONTAINS (default), DOES_NOT_CONTAIN, IS_EMPTY, IS_NOT_EMPTY, IS_RELATIVE, IS_IN_PAST,
IS_IN_FUTURE, IS_TODAY, VECTOR_SEARCH`. Value type: `string | string[] | boolean | number |
{isCurrentWorkspaceMemberSelected?, selectedRecordIds[]} | object | null`
(`shared/src/types/RowLevelPermissionPredicateValue.ts:8-15` **[ENT]**).

Operands map 1:1 onto the record filter DSL operands (`shared/src/utils/filter/mapRLSOperandToRecordFilterOperand.ts:9-48`,
CC — every case identity-maps to `RecordFilterOperand`), so **RLS predicates reuse the wave-4
filter machinery** (query-layer.md §3): the rule shape becomes a `RecordGqlOperationFilter` and is
rendered with `computeWhereConditionParts`.

### 2.4 RLS evaluation: rule/predicateGroup tree → filter → SQL (all [ENT], but small)

`buildRlsAccessSQLCondition` (`internal/rls/utils/build-rls-access-sql-condition.util.ts:37-47`):
`canRlsAccessOperation` gate (`internal/rls/utils/can-rls-access-operation.util.ts:12-38`: NOT
all-records AND some rule grants the enforced op — via CC matcher
`canAccessThroughRlsPermission`, `shared/src/utils/canAccessThroughRlsPermission.ts:7-17`, which
requires `rowLevelRecordsPermissions.some(rule => canRlsRuleAccessOperation)`), then
`buildRlsRulesSQLCondition` (`:12-35`): resolve the combined rule filter
(`resolveRlsRuleRecordFilter` → `internal/rls/utils/resolve-rls-rule-record-filter.util.ts:56-74`)
then **strategy split** (`:21-34`):

- `principalsAccessGroupCompactIds` present (strategy `accessGroups` — requires
  `IS_ACCESS_GROUPS_ENABLED`): one array-overlap condition
  `alias._accessGroupIds && CAST(:ids AS integer[])`
  (`internal/access-group/utils/build-principals-access-groups-sql-condition.util.ts:9-22`; the
  column name constant `_accessGroupIds` at
  `internal/record-permission-repository/constants/access-group-column-names.constant.ts:2` [CC]).
- otherwise strategy `degradedRead`/`withoutRowLevelPermissions`
  (`resolve-records-access-sql-condition-strategy.util.ts:8-31`: `withoutRowLevelPermissions` when
  the main roles have no RLS rule at all; else `accessGroups` if enabled, else `degradedRead`):
  render the filter tree directly (`renderRlsRecordFilterToSql`).

**Filter construction** (`internal/rls/utils/build-rls-record-filter.util.ts` **[ENT]**, 386 lines):

- Grouping semantics (`buildRecordFilterForRule` `:297-364`): predicates with
  `rowLevelPermissionPredicateGroupId == null` and root groups are AND-combined at the rule level
  (`combineRecordFilters` `:287-295`: 1 filter → itself; n → `{and: [...]}`); a group joins its
  members by its `logicalOperator` (`:329-351`: OR → `{or:[...]}`, else `{and:[...]}`), recursing
  into child groups; empty group → dropped (`:343-345`). Rules OR-combine
  (`buildRlsRecordFilter` `:366-386`: one rule → its filter; many → `{or:[...]}`; none satisfiable
  → `UNSATISFIABLE_RECORD_FILTER`, `internal/rls/constants/unsatisfiable-record-filter.constant.ts`
  **[ENT]**).
- Predicate → leaf filter (`buildPredicateRecordFilter` `:127-285`): resolves field metadata
  (unknown → `PermissionsException(FIELD_METADATA_NOT_FOUND)` `:158-161`); operand support check
  against the field's allowed filter operands (`:154-159`, `INVALID_ARG`); **relation hop**
  (`relationFieldMetadataId` set, or `workspaceMemberRelationFieldMetadataId`) requires
  `isRelationHopEnabled` else `UNSATISFIABLE_RECORD_FILTER` (`:161-167`); MORPH_RELATION IS_EMPTY
  special case → `<field>Id {is: NULL|NOT_NULL}` (`:169-184`); **workspace-member-bound
  predicates**: `predicateValue` comes from the acting member's field values
  (`memberFieldValues[buildMemberFieldKey(predicate)]`), an option filter drops non-option values,
  and a missing/empty member value yields UNSATISFIABLE — *"Without the member's value the
  predicate matches nothing rather than being dropped"* (`:198-212`); compatibility and value-
  validity guards (`isWorkspaceMemberFieldCompatibleWithRlsPredicate`, `isRecordFilterValueValid`,
  `getFilterValueValidationIssue`) → UNSATISFIABLE on mismatch (`:249-282`); otherwise a value
  predicate. `filterValueDependencies: {currentWorkspaceMemberId}` flows into the shared filter
  computation so `IS_RELATIVE`/current-member tokens resolve (`:235`).
- Member value resolution (`internal/rls/utils/resolve-workspace-member-predicate-value.util.ts`
  **[ENT]**, `:30-79`): relation field → the `<name>Id` join value only for MANY_TO_ONE owning
  side (`:37-49`); composite + `workspaceMemberSubFieldName` → sub-field value (`:59-64`);
  SELECT/MULTI_SELECT string values wrapped into arrays (`:70-76`).

**Filter → SQL** (`internal/rls/utils/render-rls-record-filter-to-sql.util.ts` **[ENT]**, 339
lines; Nudge ports this as the *RLS renderer* — thin over the wave-4 filter compiler):
`renderFilterAsConjunction` (`:79-90`) walks entries; `and`/`or` → bracketed join with empty-list
→ `1=1` (`ALWAYS_TRUE_CONDITION` `:32`, `renderLogicalGroup` `:127-147`); `not` →
`NOT (<cond|1=1>)` (`:110-121`); leaf → `renderFieldSQLCondition` (`:210-279`) which reuses the
same `computeWhereConditionParts` from `srv/engine/record-filter/utils/compute-where-condition-parts`
(query-layer.md §3.3 — one operator semantics, two callers) against the physical column (relation
via join column, composite via `<field><CapitalizedSub>` `:326`); relation sub-filters compile to
a correlated `EXISTS (SELECT 1 FROM <wsTable> AS alias_field WHERE alias_field."target" =
alias."source" AND alias_field."deletedAt" IS NULL AND (<sub>))` (`renderRelationSQLCondition`
`:164-208`). Unknown field → `RLS_PREDICATE_FIELD_NOT_FOUND` (`:230-235`).

### 2.5 `_isPrivate` — creator-or-successor visibility [ENT feature, self-hosted columns]

- Column: `_isPrivate boolean NOT NULL DEFAULT false` on every workspace object table; constant
  `private-record/constants/is-private-column-name.constant.ts:4` **[ENT]**; Nudge already creates
  it + the partial index (`workspace_migrations/runner.rs:969-970, :1051-1080`; verified live:
  `IDX_PRIVATE_RECORD_6870246ad0ff4464949b6d9501dc1a72 btree (id) WHERE "_isPrivate"` on
  `person`, §4.1). The name rationale — *"Field names cannot start with an underscore, so no custom
  field can ever take this column"* — is in the constant comment.
- **Applicability**: `arePrivateRecordsApplicable` (`internal/private-record/utils/are-private-records-applicable.util.ts:7-19`):
  `operationType !== 'create'` AND NOT a permission-storage object (recordShare/accessGroup are
  exempt, `utils/temporary-exclude-permission-storage-object.util.ts:5-9` + `constants/permission-storage-object-universal-identifiers.constant.ts:4-7`
  [CC]) AND `isPrivateRecordsEnabled` (§0.2). The flag itself is
  `isPrivateByDefaultObject || IS_PRIVATE_RECORDS_ENABLED`
  (`is-private-records-enabled.util.ts:8-16`); private-by-default universal ids =
  agentChatThread + agentChatThreadParticipant (`private-by-default-object-universal-identifiers.constant.ts:5-8`).
- **State SQL**: `alias._isPrivate = TRUE|FALSE` (`build-private-record-state-sql-condition.util.ts:7-16`
  **[ENT]**).
- **Role-level interaction**: when private records apply and there is no hypothetical share,
  the all-records/RLS arm is AND-ed with `_isPrivate = FALSE`
  (`build-access-sql-condition.util.ts:40-48` + `are-private-records-excluded-from-all-records-and-rls.util.ts:7-9`)
  — **private beats Admin**: even `canReadAllRecords` does not read a private row.
- **Creator arm**: `buildPrivateRecordAccessSQLCondition` (`build-private-record-access-sql-condition.util.ts:12-33`
  **[ENT]`) = `_isPrivate = TRUE AND createdByWorkspaceMemberId = :memberId`. The creator id comes
  from the auth context only when the main principal IS a workspace member and the object HAS a
  createdBy field (`resolve-private-record-creator-workspace-member-id.util.ts:8-18`; join column
  constant `createdByWorkspaceMemberId`, `internal/record-permission-repository/constants/created-by-workspace-member-id-column-name.constant.ts:3-4`
  — the physical column of the composite `createdBy` actor field, data-model §3). **Successors**:
  when a member leaves, `recordShare` OWNER rows transfer (`record-share.repository.ts:148-216`
  **[ENT]**, §4.4), and the creator arm of those rows is the *member* id the share carries via the
  OWNER `rowCause` — read path for members with OWNER shares goes through the share arm, not the
  creator arm; the creator arm covers rows whose private state was set by this member without
  share rows.
- **Enforcement of state**: the write of `_isPrivate` itself is an internal/system write
  (`private-record/services/private-record-system-write.service.ts` [CC file] using
  `getRepositoryWithoutPermissionChecks`), driven by the [ENT] `setRecordPrivacy` mutation and
  the creator-share write path (§4.3). `canSetPrivateRecordState.util.ts` [CC file] computes who
  may toggle (FULL share or admin-equivalent). Nudge: **the column stays default-false for every
  record because the toggling surface is deferred** (§8 D4); `canReadSomeRecords`/private DTO
  fields mirror that (always false/absent).

### 2.6 Write-side RLS assertions (create/update guards)

`RecordPermissionsReadService.assertCreatedRecordsMatchRowLevelPermissionsOrThrow`
(`record-permissions-read.service.ts:497-525` **[ENT]**): builds the `create` access request and
runs `validateRlsPredicatesForRecords` (`internal/rls/utils/validate-rls-predicates-for-records.util.ts`)
— evaluates each created record (in memory, `isRecordMatchingFilter` semantics, §2.4 filter model;
`internal/rls/utils/find-records-matching-record-filter.util.ts`) against every RLS predicate that
is *satisfiable at root conjunction* (`is-rls-predicate-at-root-conjunction.util.ts`) for the
create op, including relation-hopped ones (join values fetched via
`findRelationJoinValues`); failure → `PermissionsException(RLS_VALIDATION_FAILED)` — the user
message: *"Record does not satisfy security constraints."* (`permissions.exception.ts:172-173`).

`assertUpdatesKeepRecordAccessOrThrow` (`:527-617` **[ENT]**): short-circuits unless
`isMainRoleRestrictedByRlsRule` (`internal/rls/utils/is-main-role-restricted-by-rls-rule.util.ts:8-30`:
any main role has `rowLevelRecordsPermissions` on the object); then
(a) `assertUpdatedColumnsAreEditableOrThrow` — rule-scoped uneditable field sets evaluated per
record (§3.3); (b) evaluates `findAccessibleRecordIds` before and after the update
(`internal/combine/utils/find-accessible-record-ids.util.ts:12-61` — the in-memory mirror of
`buildAccessSQLCondition`: all-records ∪ RLS-match (minus private when applicable) ∪ share ∪
creator-of-private); any record that *was* updatable and *is not* after → `RLS_VALIDATION_FAILED`
(`:607-616`). **Net effect: you cannot edit a record out of (or into) your rule set.**

The field-level carve-out on create lives in the [CC] validator: RLS-predicate fields are exempt
from the `uneditableFields` check for `create` (see §3.1).

### 2.7 Multi-role combination and principals

Principals (`principal/types/permissions-principals-with-roles-context.type.ts` **[ENT]**):
`{workspaceId, mainPrincipal: {roleIds, kind}, limitingPrincipal?: agent,
applicationPrincipal, runtimeLimitingRole?}`; role ids union (`internal/principal/utils/get-permissions-principals-role-ids.util.ts:7-21`:
limiting ∪ main ∪ application, deduped); share keys (`get-permissions-principals-share-keys.util.ts:9-20`:
member id + role ids — §4).

Per-object permission computation (entry `computePrincipalsRecordsPermissions`,
`internal/role/utils/compute-principals-records-permissions.util.ts:19-94` **[ENT]**, composed of
CC kernels):

1. Per role: `computeRoleRecordsPermissions` (CC, §2.8) →
   `restrictRecordsPermissionsToDisabledActions` (CC, §3.4) — object/field disabled actions,
   application-owner exemption (`actingApplicationId == object.applicationId` ⇒ no restriction,
   `restrict-records-permissions-to-disabled-actions.util.ts:61-76`).
2. `main roles → computePermissionUnion` (CC `compute-permission-union.util.ts:52-94`): **some**
   role grants all-records → granted; unreadable/uneditable field lists **unioned** (`:13-22`);
   row-level rules concatenated (`:23-25, :46-48`). Empty list → `NO_ROLE_RECORDS_PERMISSIONS`
   (all false, `constants/no-role-records-permissions.constant.ts`).
3. `× limiting roles → computePermissionIntersection` (CC
   `compute-permission-intersection.util.ts:109-149`): every role must grant each all-records flag
   (`:31-38`); field lists unioned; each rule's per-op flag AND-ed with "other roles reach all
   records for that op" (`:65-106`).
4. `× runtimeLimitingRole → restrictRecordsPermissionsToRuntimeLimitingRole` (CC
   `restrict-records-permissions-to-runtime-limiting-role.util.ts:8-56`) — the AI run scope
   (out of wave scope; seam kept).

### 2.8 Role → object/field permission kernel (CC — the heart Nudge ports verbatim)

`role/utils/compute-role-records-permissions.util.ts` **[CC]**, `:216-319`, given
`roleDefinition = {canRead/Update/SoftDelete/DestroyAllObjectRecords,
canCreateAllObjectRecords, isPermissionFlagGranted, objectPermissions[], fieldPermissions[],
rowLevelPermissionRules[], rowLevelPermissionPredicates[], rowLevelPermissionPredicateGroups[]}`
(`:23-67`):

- `resolveObjectRecordsAccess` (`:87-152`) per object:
  - `workflowRun` object ⇒ every flag := `isPermissionFlagGranted(WORKFLOWS)` (`:19-21, :99-105`).
  - `workspaceMember` object ⇒ every flag := `isPermissionFlagGranted(WORKSPACE_MEMBERS)` **but
    canRead: always true** (`:107-116` — members are enumerable to everyone).
  - otherwise per operation: `resolveObjectPermissionValue({objectPermission override, hasAll
    role flag, isSystemObject})` where the shared helper (CC
    `shared/src/application/utils/resolveObjectPermissionValue.ts:1-9`) is
    `objectValue ?? (isSystemObject || hasAllObjectsPermission)` — **system objects default
    readable AND (unless `areSystemObjectsReadOnlyByDefault`, the AI-scope case `:118-119`)
    writable** regardless of the role flags; a stored `objectPermission` boolean overrides.
  - `agentChatThread` requires the `AI` flag additionally (`:248-251, :301-307`).
- Field masks (`buildUnreadableAndUneditableFields` `:154-194`): a `fieldPermission` with
  `canReadFieldValue === false` marks the field unreadable+uneditable (labelIdentifier exempt from
  unreadable at object level only, `:169-175`); `canUpdateFieldValue === false` → uneditable;
  **morph relation expansion** via `resolveMorphRelationPermissionFieldIds` (`:177`) — restricting
  a morph head restricts its target fields (builder
  `metadata-modules/role/utils/build-resolve-morph-relation-permission-field-ids.util.ts` [CC]).
- RLS rules per object (`:253-286`): rules with zero predicates dropped (`:262`); each rule's
  flags AND-ed with `hasAiAccess`; per-rule field masks from the rule-scoped `fieldPermissions`
  (`rowLevelPermissionRuleId` match, `:279-285`); workflow objects skip field masks (`:275-276`).
- Top-level `RoleRecordsPermissions` from the role's own flags (`:312-318`).

The per-role cache: `WorkspaceRolesPermissionsCacheService`
(`metadata-modules/role/services/workspace-roles-permissions-cache.service.ts` **[CC]** — no
license header) computes `rolesPermissions: Record<roleId, RoleRecordsPermissions>` **once per
workspace cache version** (`@WorkspaceCache('rolesPermissions', {packingPonderation: 2})` `:52`,
rows requirement `:22-49`: role + objectPermission + rolePermissionFlag + permissionFlag +
fieldPermission + RLS triples + objectMetadata + morph fields). RLS upserts invalidate it
(`row-level-permission-predicate.service.ts:510-512` **[ENT]**). This is Nudge's anchor for the
"compile once per (workspaceId, metadataVersion)" budget (§7 perf).

### 2.9 The object-level enforcement site (CC) — `validateOperationIsPermittedOrThrow`

`internal/role/utils/validate-operation-is-permitted-or-throw.util.ts:29-140` **[CC]**, called
from `assertCanRunOperationOrThrow` (`record-permissions-read.service.ts:328-374` **[ENT]**,
which first applies the disabled/deny throws `:345-357`). Semantics:

- Unknown object → `PERMISSION_DENIED` (`:39-58`).
- `read | destroy | softDelete`: `selectedColumns` ∩ `unreadableFields` → throw PERMISSION_DENIED
  with `no permission to read field "<name>" on "<object>"` (`:71-81, :165-203`).
- `create`: same read check + `updatedColumns` ∩ `uneditableFields` **except columns that are
  RLS predicate fields** (`:92-114` — the insert-guard exemption: you must be able to write the
  very columns that decide the rule you fall into).
- `update`: read check + full `updatedColumns` vs `uneditableFields` (`:116-133`).
- Unknown op → `UNKNOWN_OPERATION_NAME` (`:135-139`).
- A column with no field-metadata mapping (e.g. `id`, `createdAt` are fields; a truly unknown
  column) → `InternalServerError` (`:185-189`) — Nudge maps to the same class (programming error,
  not permission).

Note `assertCanRunOperationOrThrow` passes `columnsToReturn` as `selectedColumns` on writes
(`workspace-repository.ts:1585-1593`): returning an unreadable column is itself a read violation.

### 2.10 `recordReadRestrictions` — the role-level read view (used at selection/parse time)

`buildRecordReadRestrictions` (`internal/role/utils/build-record-read-restrictions.util.ts:11-43`
**[ENT]**, from `record-permissions-read.service.ts:302-313`) produces
`{unreadableObjectMetadataIds, unreadableFieldMetadataIds}` where an object is unreadable when
`!canAccessThroughAllRecordsOrRlsPermission(read)` (CC matcher
`shared/src/utils/canAccessThroughAllRecordsOrRlsPermission.ts`) and fields are the union of
`objects[id].unreadableFields`. This set is what the API boundary drops/throws on (§1.4) — it is
the *cheap*, metadata-only read restriction (no SQL), cached per context (`workspace-orm.manager.ts:146-150`).
Nudge stores it on `PermissionsContext` (P1).

### 2.10 Channel visibility is field redaction, not row filtering (Apache — open gap)

At 2.61.0 there is **no** `engine/permissions/internal/channel-visibility/` module and no RLS rule
behind channel visibility. Enforcement is a post-query redaction layer that is plain Apache code and
runs **unconditionally** — no feature flag, no license or billing check:

- `engine/api/common/common-record-visibility/common-record-visibility.service.ts:16` holds a
  `objectNameSingular -> handler` registry (`registerHandler`, `applyVisibilityRestrictions`), and
  `engine/api/common/common-query-runners/common-base-query-runner.service.ts:343` calls it for every
  result. `collectRecordsByObjectNameSingular` walks nested relations too, so a body cannot leak
  through a related record.
- Handlers self-register in `onModuleInit` and are wired from `modules/modules.module.ts:43-44`:
  `modules/messaging/common/query-hooks/message/apply-messages-visibility-restrictions.service.ts:22`
  restricts `subject`, `text`, `attachments`; `.../attachment/apply-attachments-visibility-restrictions.service.ts`;
  `modules/calendar/common/query-hooks/calendar-event/services/apply-calendar-events-visibility-restrictions.service.ts:41-76`;
  `modules/timeline/query-hooks/apply-timeline-activities-visibility-restrictions.service.ts:22-43`
  (redacts `timelineActivity.properties`).
- Redaction writes the `FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED` sentinel into the text
  fields and an empty array into `attachments`
  (`apply-messages-visibility-restrictions.service.ts:68-78`). A record whose channel is
  `SUBJECT`-visible keeps `subject` and loses `text`/`attachments`; `SHARE_EVERYTHING` keeps all;
  anything else loses all three.
- Visibility comes from the channel row, not the record: `messageChannel.visibility`
  (`:132`, `:159-167`) for messages/attachments/participants and the calendar channel equivalent.
  Default for a created channel is `METADATA`
  (`engine/core-modules/auth/services/create-message-channel.service.ts:54`).
- Ownership (`:174-197`): a user principal owns a channel through `connectedAccount.userWorkspaceId`;
  an API-key/application principal through `connectedAccount.applicationId`.

Consequence: rows are always returned, fields are blanked. Nudge has no redaction layer at all — the
only related code is the search-side `CHANNEL_VISIBILITY_OBJECTS` object list in
`crates/server/src/query/runner/search.rs:39-50`, and even that is skipped by the default global
search branch (`search.rs:188-192`). Two members with separate mailboxes therefore see each other's
plaintext `message.text`/`subject` on Nudge where 2.61.0 returns the sentinel. This is the highest
severity remaining permission gap in the default self-host configuration, and it is in scope for the
port. Search must apply the same exclusion in its default branch, and the record-read path must run
the handler registry after RLS.

---

## 3. Field-level semantics

### 3.1 The fieldPermission model

`core.fieldPermission` (`metadata-modules/object-permission/field-permission/field-permission.entity.ts`
**[CC]**, `:37-121`): `roleId`, `objectMetadataId`, `fieldMetadataId`,
`rowLevelPermissionPredicateGroupId?` (RLS-scoped mask), `rowLevelPermissionRuleId?`,
`canReadFieldValue: boolean|null` (`:103-104`), `canUpdateFieldValue: boolean|null`
(`:106-114`, validated `isFalseOrNull` — a fieldPermission may only DENY, never grant). Unique
indexes: `(fieldMetadataId, roleId)` where group is null (`:25-29`);
`(fieldMetadataId, roleId, rowLevelPermissionPredicateGroupId)` where group is not null
(`:30-34`). `canReadFieldValue=false` also implies uneditable (`buildUnreadableAndUneditableFields`
`util.ts:184-186` reads `isUnreadable || canUpdateFieldValue === false`), and morph expansion at
§2.8.

### 3.2 Where fields are STRIPPED from results (silent) — role-level

The role-level `unreadableFields` are removed *before the SQL is assembled* and *after results are
read* — the client simply never sees the field (it is not nulled, it is absent):

- Selection: `common-select-fields-builder.ts:119` filters `readableFlatFields`
  (`build-field-selection.util.ts:55-60` skips unreadable/morph-head;
  `build-relation-selection-from-depth.util.ts:71` drops unreadable relations).
- Group-by record fields: `common-group-by-query-runner.service.ts:285`
  (`.filter(field => !unreadableFieldMetadataIds.has(field.id))`).
- Events: §3.5.

Requesting an unreadable field explicitly (GraphQL selection / REST `fields=`) is the THROW path:
`assertFieldIsReadableOrThrow` (§1.4) is invoked on filter and order-by leaves; for *selected*
columns, `validateOperationIsPermittedOrThrow(read, selectedColumns)` (§2.9) throws
`PERMISSION_DENIED` from `validateQueryIsPermitted` (§1.1 pass 2) if a selected column resolves to
an unreadable field. `makeRlsRuleFieldsUnreadable` (§1.1 pass 3) is separate — it handles the
RLS-*conditional* mask.

### 3.3 Where fields are MASKED (nulled conditionally) — RLS-scoped

Distinct from §3.2: an RLS rule can make a field unreadable **only for rows matching that rule**.
`record-field-permissions-read.service.ts:46-112` **[ENT]** `buildUnreadableFieldsSQLConditions`:

- `findRlsRuleInaccessibleFields` (`internal/field-access/utils/find-rls-rule-inaccessible-fields.util.ts`)
  yields per-rule `{rowLevelPermissionRule, fieldMetadataIds}` for the object.
- Each rule becomes an SQL condition: `renderRlsRecordFilterToSql(buildRlsRulesRecordFilter([rule]))`
  — the rule's own predicate tree rendered (§2.4) — falling back to `TRUE` if the rule has no
  renderable filter (`:90-91`).
- `buildFieldsUnreadableByEveryMatchingRuleSQLConditions`
  (`internal/field-access/utils/build-fields-unreadable-by-every-matching-rule-sql-conditions.util.ts:16-61`
  **[CC]**): per field, `condition = (OR of hiding rules) AND ((OR of showing rules) IS NOT TRUE)`
  (`:52-58`) — a field is masked on exactly the rows where every rule that hides it matches and no
  rule that shows it matches.
- Installed as `makeColumnsUnreadableWhen` / `makeRelationsUnreadableWhen` (§1.1 pass 3): masked
  columns render as NULL (or a placeholder string for restricted-placeholder fields —
  `constants/restricted-placeholder-field-universal-identifiers.constant.ts` [CC]:
  message.subject/text, calendarEvent.title/description → replaced by the
  `FIELD_RESTRICTED_ADDITIONAL_PERMISSIONS_REQUIRED` placeholder token, builder
  `workspace-select-query-builder.ts:784-792`); masked relations render as absent. A masked order
  column still sorts via `andWhereReadingUnreadableColumnsAsNull`
  (`workspace-select-query-builder.ts:210-226`, query-layer.md §3.5).

In-memory mirror for write guards and events:
`resolveUnreadableFieldsOnMatchingRecords` / `resolveUneditableFieldsOnMatchingRecords`
(`internal/field-access/utils/resolve-*.util.ts` **[CC]`) evaluate the same rule filters against
concrete record snapshots (`isRecordMatchingFilter` §2.6).

### 3.4 Disabled permission actions (application/system lockdown) — CC

The 2.53 upgrade moved the old `writability: MetadataWritability {OPEN, APPLICATION, SYSTEM}` enum
(`shared/src/types/MetadataWritability.ts:1-5`, still on `fieldMetadata` `field-metadata.entity.ts:162-170`,
runtime no longer reads it for these — see the 2.53 command `WRITABILITY_MOVES` `:16-20` which
rewrites APPLICATION→OPEN + `disabledFieldPermissionActions=['canUpdateFieldValue']`) onto two
jsonb columns:

- `objectMetadata.disabledObjectPermissionActions: DisableableObjectPermissionAction[]` — action
  names `canReadObjectRecords, canUpdateObjectRecords, canSoftDeleteObjectRecords,
  canDestroyObjectRecords, canCreateObjectRecords`
  (`shared/src/application/objectPermissionActionType.ts:1-14`).
- `fieldMetadata.disabledFieldPermissionActions: FieldPermissionAction[]` —
  `canReadFieldValue, canUpdateFieldValue` (`shared/src/application/fieldPermissionActionType.ts:1-5`).
- `fieldMetadata.isUIReadOnly` is superseded by `isUIEditable` (`field-metadata.entity.ts:155-181`;
  `isUIReadOnly` marked `WasRemovedInUpgrade`, retained only for rolling-deploy compat).
- `isUIEditable`/`isUICreatable` on the object DTO gate UI affordances only, not API (§6).

Enforcement (`restrict-records-permissions-to-disabled-actions.util.ts` **[CC]**, `:15-198`): a
disabled action forces the matching `can…AllRecords`/rule flag off (`:26-57`) and forces the field
into `unreadableFields`/`uneditableFields` (`:95-130`). **The acting application is exempt for its
own objects/fields** (`getDisabledObjectPermissionActions:70-76` and the field filter `:166-171`:
`flatObjectMetadata.applicationId === actingApplicationId ⇒ []`). The label identifier is never
forced unreadable (`getEnforcedFieldPermissionActions:87-93`). `isPermissionsPrincipalsOwningObject`
(§2.2) makes the object-level deny non-fatal for the owning app (`is-object-operation-disabled-to-principals.util.ts:21-24`).

Nudge: `disabled_object_permission_actions` and `disabled_field_permission_actions` are already on
the flat maps (`metadata/flat/object.rs:50, field.rs:52`) — P2 ports this restriction verbatim.

### 3.5 Event payload masking (out of wave-4 read path, kept for wave-6 events)

`record-event-permissions-read.service.ts` **[ENT]** + `omitUnreadableFieldsFromEvent`
(`internal/field-access/utils/omit-unreadable-fields-from-event.util.ts` **[CC]**, `:38-97`) strip
unreadable field names from `updatedFields/before/after/diff` and drop events that became empty
(`is-update-of-hidden-fields-only.util.ts:6-12`). Nudge keeps this as a pure util in P3 for the
wave-6 event emitter; no SQL cost on the record read path today.

---

## 4. Record sharing / recordShare

### 4.1 Row-shape contract (already created by wave-3 U7)

Live reference DDL, `workspace_6n57znb4w89f8r7xudebmf0."recordShare"` (from
`docker exec infra-db-1 psql -U postgres -d twenty_ref -c '\d+ …"recordShare"'`):

| Column | Type | Null | Default |
|---|---|---|---|
| id | uuid | not null | uuid_generate_v4() |
| createdAt | timestamptz | not null | now() |
| updatedAt | timestamptz | not null | now() |
| deletedAt | timestamptz | yes | |
| recordId | uuid | not null | |
| objectMetadataId | uuid | not null | |
| principalId | uuid | not null | |
| principalType | `recordShare_principalType_enum` | not null | |
| accessLevel | `recordShare_accessLevel_enum` | not null | 'READ' |
| rowCause | `recordShare_rowCause_enum` | not null | |
| sourceId | uuid | not null | |

Indexes: `recordShare_pkey(id)`; `IDX_4a277b3dad18715d2619fee7c5b (principalId)`;
`IDX_5519b21a3b491ab8b1ce594b119 (sourceId)`;
`IDX_UNIQUE_2a8112c19cab880dde4b9b122e0 UNIQUE (objectMetadataId, recordId, principalId, rowCause,
sourceId)`. Enums (live DB): principalType `EVERYONE|WORKSPACE_MEMBER|ROLE`
(`shared/src/types/RecordSharePrincipalType.ts:1-5`); accessLevel `READ|READ_WRITE|FULL|NONE`
(`shared/src/types/RecordShareAccessLevel.ts:1-6`); rowCause `OWNER|MANUAL|RULE|APPLICATION`
(`shared/src/types/RecordShareRowCause.ts:1-6`).

Nudge's `standard_objects/permissions.rs:11-19` creates byte-equivalent DDL (verified against the
same reference in wave 3, HANDOFF.md U7; `create_permission_tables` runs on activation). The
companion `accessGroup` table (§4.5) is likewise created (`permissions.rs:21-29`; live DDL +
CHECK `CHK_ACCESS_GROUP_TYPE_COLUMNS` confirmed).

### 4.2 Read paths — how shares feed row-access predicates

Two distinct mechanisms, both [ENT]:

**(a) SQL condition (the read path).** `buildRecordShareAccessSQLCondition`
(`internal/record-share/utils/build-record-share-access-sql-condition.util.ts:10-28`): resolves
the requirement (`resolve-record-share-access-requirement.util.ts:11-37`: principalIds = share keys
[§2.7 `getPermissionsPrincipalsShareKeys`], accessLevels = required levels for the op, gated on
`canRecordShareAccessOperation`), then `buildRecordShareSQLCondition`
(`build-record-share-sql-condition.util.ts:11-56`) emits, for the uncorrelated read case (`:47-48`):

```sql
"alias"."id" = ANY(ARRAY(SELECT "alias_recordShare"."recordId"
   FROM <ws>.recordShare AS "alias_recordShare"
   WHERE "alias_recordShare"."objectMetadataId" = :recordShareObjectMetadataId
     AND "alias_recordShare"."principalId" = ANY(:recordSharePrincipalIds)
     AND "alias_recordShare"."accessLevel" IN (:...recordShareAccessLevels)
     AND "alias_recordShare"."deletedAt" IS NULL))
```

(The correlated `EXISTS(SELECT 1 …)` variant `:49` is used in related-record-path joins.) Table
expression = fully-qualified `"workspace_<b36>"."recordShare"`
(`build-record-share-table-expression.util.ts:6-14` → `build-workspace-table-expression.util.ts:6-13`).
The comment at `:45-46` notes the uncorrelated form lets Postgres read grants once and OR against
an indexed row filter rather than probing per row — Nudge keeps this form.

Required access levels per operation (`resolve-required-record-share-access-levels.util.ts:25-28`
**[ENT]**, from the map at `:8-23`): read → `[READ, READ_WRITE, FULL]`; create → `[]` (no share row
exists yet — create stays the role's decision); update → `[READ_WRITE, FULL]`; softDelete → `[FULL]`;
destroy → `[FULL]`. Op accessibility check (`can-record-share-access-operation.util.ts:11-26` +
`resolve-operation-types-accessible-through-record-share.util.ts:12-30`): the object must not have
the corresponding `disabledObjectPermissionAction` for that op and `areRecordSharesApplied`
(`record-share/utils/are-record-shares-applied.util.ts:15-21`: `isPrivateByDefaultObject ||
isRecordSharingEnabled`).

**(b) In-memory evaluation (write guards / the `recordPermissions` query).**
`findAccessibleRecordIds` (§2.6) includes `findRecordIdsAccessibleThroughRecordShare`; `findShares`
uses `buildFindRecordSharesQuery` (`internal/record-permission-repository/utils/build-find-record-shares-query.util.ts:5-16`):
`SELECT id,recordId,objectMetadataId,principalId,principalType,accessLevel,rowCause,sourceId FROM
<ws>.recordShare WHERE objectMetadataId=:o AND recordId = ANY(:ids) AND deletedAt IS NULL`.

### 4.3 Write paths and the create-time share contract

`RecordShareRepository` (`internal/record-permission-repository/repositories/record-share.repository.ts`
**[ENT]**): `insertRecordShares` (`build-insert-record-shares-query.util.ts:7-34`, `ON CONFLICT DO
NOTHING` when `shouldIgnoreConflicts`), `deleteRecordShares`
(`build-delete-record-shares-query-or-throw.util.ts`), `lockRecordForSharingChange`
(advisory lock `pg_advisory_xact_lock(hashtextextended(buildRecordShareLockKey,0))` + `SELECT … FOR
UPDATE` on the record row, `:37-73`), `insertOwnerRecordSharesFromSelect`
(`:124-146` — grants FULL/OWNER to a grantee), `transferMemberRecordShares`
(`:148-216` — the **member-successor** transfer: re-point OWNER/MANUAL FULL shares from the
departing member to a successor, else delete).

`buildRecordShareInputsForCreatedRecords`
(`internal/record-share/utils/build-record-share-inputs-for-created-records.util.ts:117-173`
**[ENT]**) is what the create path (`shareWith` arg, query-layer.md §4.6 deferred here) feeds:
the creator gets a `FULL`/`OWNER` WORKSPACE_MEMBER share (user context) or a `FULL` ROLE share
(api-key/application, deduped against shareWith); `shouldWriteCreatorRows = isPrivateByDefault ||
(isPrivate && !isUserAuthContext)` (`:151-152`). `validateShareWithArgOrThrow` (`:15-46`) enforces
shareWith presence for private-object creation by non-user contexts and rejects duplicate
principals (`INVALID_SHARE_WITH`).

GraphQL surface: `RecordSharingResolver` (`record-share/resolvers/record-sharing.resolver.ts`
**[CC file]**, `@MetadataResolver`, `:40-119`): `recordSharing(target)` query (`:46-52`) and
`setRecordPrivate`, `setRecordShare`, `removeRecordShare` mutations (`:54-97`), each behind
`AccessPolicyGuard({userSession standard/impersonated/playground, apiKey:false,
oauthClient:{withUser,withoutUser:false}, application:{withUser,withoutUser:false}, flags:[]})` +
`CustomPermissionGuard` (`:22-39`) — i.e. a logged-in user only, no fixed permission flag; the
actual authorization is custom (record-level FULL/manage) inside the [ENT]
`RecordSharingWriteService`/`RecordSharingPermissionsService`. DTOs
(`record-share/dtos/record-sharing.dto.ts` [CC]): `RecordSharingDTO {isShareable, isPrivate,
canManageSharing, permissions: RecordPermissionsDTO, hasManagedGeneralAccess, roles[], shares[]}`
(`:72-94`), `RecordSharingGrantDTO` (`:33-53`), `RecordSharingRoleDTO` (`:55-70`).

### 4.4 Nudge scope decision for sharing

`IS_RECORD_LEVEL_SHARING_ENABLED` is a paid (BillingEntitlementKey.RECORD_SHARING) + flag feature
(auth-permissions.md §5.1/§5.4) and every read/write service is [ENT]. A stock self-host has it
**off** and the `recordShare` table **empty** (confirmed live: `count=0` in §4.1 workspace). Nudge:
**defer** the sharing read/write services and `shareWith` create integration (D4), but implement
the `recordShare`-condition renderer (`build_record_share_sql_condition`) *behind the flag* so the
predicate algebra in §2.2 is complete and the deferral is a feature-toggle, not a code gap. The
transfer/lock machinery (successor migration) is wave-7 (workspace-lifecycle adjacent).

### 4.5 accessGroup (deferred parity, with evidence)

`core`-absent, workspace-scoped `accessGroup` table (§4.1 DDL; live reference) materializes RLS
rule/literal/member-value/ALL_RECORDS groups with a `compactId` identity used as the
`_accessGroupIds` array element. The computation/sync/propagation engine is entirely [ENT]
(`internal/access-group/services/*`, all ENT; `record-access-group-write.service.ts` ENT;
`build-recompute-record-access-group-ids-query.util.ts` ENT — the UPDATE that recomputes each
record's `_accessGroupIds` array from its matching rules, `:53-69`). This is the *fast* read path
(array-overlap instead of re-rendering every predicate). **Nudge decision: not implemented (D4).**
When `IS_ACCESS_GROUPS_ENABLED` is off (self-host default), RLS reads via `degradedRead` (§2.4),
which Nudge *does* implement (P4), so custom-role RLS works without access-groups on the write
side — matching the upstream self-host code path exactly. The `compute_recordAccessGroupIds`
deferred op (query-layer.md §5.2) stays a documented deferred parity item; `recomputeRecordAccessGroupIdsAfterWrite`
becomes a no-op recorder behind the `permissions` seam.

### 3.6 Schema exposure note (front-relevant, verified)

The generated workspace SDL does **not** prune unreadable fields (searched
`srv/engine/api/graphql/workspace-schema-builder` at the tag for `unreadable|permission|restricted`
— zero hits; `git grep unreadable twenty/v2.61.0 -- packages/twenty-server/src/engine/api/graphql/workspace-schema-builder`
→ empty). Field-level permission is **runtime-only**: implicit selections drop the field, explicit
selections throw. The front end therefore cannot read permissions from introspection — it must use
the permission state fed to `objectPermissionsByObjectMetadataId` (§6).

---

## 5. Metadata-plane permissions (who may change metadata and roles)

### 5.1 The guard stack (all [CC])

Endpoint authorization is `AccessPolicyGuard(policy)` (`srv/engine/guards/access-policy.guard.ts:18-43`)
→ `AccessPolicyService.assertRequestAccessPolicySatisfiedOrThrow`
(`guards/access-policy/access-policy.service.ts:32-74`): principal classification (`:39-46`,
`classify-auth-principal.util.ts` maps token type → userSession standard/impersonated/playground/
workspaceAgnostic, apiKey, oauthClient{withUser,withoutUser}, application{withUser,withoutUser,
withAgent}; refused → `AuthPrincipalRefusedException`), required feature flags (`:50-65`), then per
required `PermissionFlagType`: `SettingPermissionsService.hasPermissionFlag` (`:67-73`) — with the
not-yet-activated-workspace escape hatch `WORKSPACE_ACTIVATION_STATUSES_WITHOUT_ROLES` (`:124-130`).
`SettingPermissionsService.hasPermissionFlag` (`permissions/setting-permissions.service.ts:102-149`
**[CC file]**): system auth context ⇒ true (`:113-115`); else principals required (`:117-124`);
granted iff `runtimeLimitingRole` (AI scope, if present) grants the flag AND **some main role**
grants it AND **every limiting (agent) role** grants it (`:141-148`), using
`isPermissionFlagGrantedToFlatRole` = role-wide access OR explicit flag row (`:212-238` →
`metadata-modules/flat-role/utils/is-permission-flag-granted-to-flat-role.util.ts:8-22` →
`role/utils/has-role-wide-access-to-permission-flag.util.ts:8-17` **[CC]**: tool flags ⇒
`role.canAccessAllTools`, else `role.canUpdateAllSettings`). A role missing from the cache grants
nothing (`:223-237`; comment at `:211`). `CustomPermissionGuard` is an always-true documentation
marker (`guards/custom-permission.guard.ts:18-23`).

Nudge already runs the metadata API (wave-3 U9 on `wave3/u9`: `metadata/api/handlers.rs` dispatch,
`create_one_object` at `:190`, `create_one_field` at `:435`) — **today it enforces no policy**. This
wave supplies the guard layer it must call (P5).

### 5.2 The metadata mutation policies (per resolver, at the tag)

| Operation | Guard | Required flag | File:line |
|---|---|---|---|
| `createOneObject` / `updateOneObject` / `updateManyObjects` / `deleteOneObject` | `AccessPolicyGuard(OBJECT_METADATA_WRITE_ACCESS_POLICY)` | **DATA_MODEL** | `metadata-modules/object-metadata/object-metadata.resolver.ts:362-363, :400-401, :419-420, :381-382`; policy `constants/object-metadata-write-access-policy.constant.ts:5-18` (userSession standard/impersonated/playground, apiKey true, oauthClient true, application true, flags `[DATA_MODEL]`) |
| object/field **reads** (`objects`, `object`, `fields` queries) | `AccessPolicyGuard(OBJECT_METADATA_READ_ACCESS_POLICY)` / `FIELD_METADATA_READ_ACCESS_POLICY` | none (`flags: []`) | `object-metadata.resolver.ts:97, :141`; `constants/object-metadata-read-access-policy.constant.ts:3-15`; `field-metadata.resolver.ts:77, :125`; `constants/field-metadata-read-access-policy.constant.ts:3-15` |
| `createOneField` / `updateOneField` / `deleteOneField` (+ many variants) | `AccessPolicyGuard(FIELD_METADATA_WRITE_ACCESS_POLICY)` | **DATA_MODEL** | `field-metadata.resolver.ts:243, :261, :279` (updateField at `:261-269`); `constants/field-metadata-write-access-policy.constant.ts:5-18` |
| `createOneIndex`/`deleteOneIndex` | `AccessPolicyGuard({userSession*, apiKey:true, oauthClient:true, application:true, flags:[DATA_MODEL]})` (class-level inline) | **DATA_MODEL** | `metadata-modules/index-metadata/index-metadata.resolver.ts:26-41` |
| pageLayout / tab / widget CRUD | `*_WRITE_ACCESS_POLICY` (flags `[LAYOUTS]`) | **LAYOUTS** | `metadata-modules/page-layout/constants/page-layout-write-access-policy.constant.ts:16`, `…-tab-…:16`, `…-widget-…:16` |
| view CRUD | `VIEW_WRITE_BY_VISIBILITY_ACCESS_POLICY` (`flags:[]`) + `CustomPermissionGuard`; VIEWS enforced **per-view** by `ViewAccessService` `hasPermissionFlag(VIEWS)` on publish | **VIEWS** (custom, not a fixed policy flag) | `metadata-modules/view/resolvers/view.resolver.ts:183-185, :205-207, :225-227, :242-244`; `…/view/constants/view-write-by-visibility-access-policy.constant.ts:15-17`; `permissions/metadata-permissions/services/view-access.service.ts:152, :168-170` |
| webhook / apiKey CRUD | `*_WRITE_ACCESS_POLICY` requiring `API_KEYS_AND_WEBHOOKS` (+ `ROLES` for apiKeys) | **API_KEYS_AND_WEBHOOKS** | `core-modules/webhook/constants/*-write-access-policy.constant.ts`, `core-modules/api-key/constants/*-write-access-policy.constant.ts` |
| other metadata modules | same shape — per-module `*_WRITE_ACCESS_POLICY`/inline constant; reads use the `READ`-shaped policy with `flags:[]` | per module | `metadata-modules/*/constants/*-access-policy.constant.ts` |
| role read `roles`, `role(id)` | `AccessPolicyGuard(ROLE_READ_ACCESS_POLICY)` | **ROLES** | `metadata-modules/role/role.resolver.ts:100-101, :108-109`; policy `role/constants/role-read-access-policy.constant.ts:5-17` (application true) |
| `createOneRole`/`updateOneRole`/`deleteOneRole`/`upsertObjectPermissions`/`upsertPermissionFlags`/`upsertFieldPermissions`/`upsertRowLevelPermissionRules`/`assignRoleToAgent`/`removeRoleFromAgent` | `AccessPolicyGuard(ROLE_WRITE_ACCESS_POLICY)` | **ROLES** (application **false**) | `role.resolver.ts:171-172, :192-193, :212-213, :250-251, :267-268, :295-296, :312-313, :327-328, :343-344`; policy `constants/role-write-access-policy.constant.ts:5-18` |
| `updateWorkspaceMemberRole` (the workspaceMember role assignment mutation) | `AccessPolicyGuard(ROLE_ASSIGNMENT_ACCESS_POLICY)` | **ROLES**; apiKey false; oauthClient withUser only | `role.resolver.ts:136-137`; `constants/role-assignment-access-policy.constant.ts:5-18` |
| `recordPermissions(targets)` query | `AccessPolicyGuard({userSession*, apiKey:false, oauthClient:{withUser}, application:{withUser}, flags:[]})` + `CustomPermissionGuard` | none (self-scoped, record-level logic) | `permissions/record-permissions/record-permissions.resolver.ts:24-41` **[CC]**; cap `MAX_PERMISSION_TARGETS = 100` (`:22`), non-user context → `AuthenticationError` (`:53-56`) |
| sharing panel (`recordSharing`, `setRecordPrivate/Share`, `removeRecordShare`) | same policy as recordPermissions + `CustomPermissionGuard` | none | §4.3 |
| `getCurrentUser` | `AccessPolicyGuard(USER_ACCESS_POLICY)`; response embeds the user's `permissionFlags` + `recordsPermissions` (§6) | — | `core-modules/user/user.resolver.ts:125-126, :172-179` |

The REST mirror on the metadata plane (`/rest/metadata/*`) uses the same
`AccessPolicyGuard + CustomPermissionGuard` stack via `RestCoreMiddleware` (query-layer.md §1.6) —
Nudge's `metadata/api.rs` dispatcher must consult the same table (P5).

### 5.3 Role-mutation business rules (beyond the flag)

- **Editability**: `role.isEditable` (Admin/Member are `isEditable=false`, auth-permissions.md §2.1)
  → `ROLE_NOT_EDITABLE` (`PermissionsExceptionCode:33`; the lockout validator deliberately defers
  to this downstream error — `metadata-modules/role/utils/validate-role-mutation-does-not-lock-out-actor.util.ts:12`
  comment). The DTO masks it: `roleDto.isEditable = flatRole.isEditable && <custom-role-owner check>`
  (`metadata-modules/role/role.service.ts:106-107`).
- **Delete rules** (`role.service.ts:344-480` [CC-region logic, `role.service.ts` itself is CC-file
  except RLS bits]): unknown role → `ROLE_NOT_FOUND`; deleting the `workspace.defaultRoleId` →
  `DEFAULT_ROLE_CANNOT_BE_DELETED` (`:437-445`); acting member's own editable role →
  `CANNOT_DELETE_OWN_ROLE` (`validate-role-mutation-does-not-lock-out-actor.util.ts:13-28`);
  remaining members are re-bound to the default role via `roleTarget` create/delete in the same
  migration (`role.service.ts:450-476` `buildRoleTargetsRebindToDefaultRole`). Member role:
  `MEMBER_ROLE_CANNOT_BE_DELETED` (`permissions.exception.ts:36`).
- **Update rules**: self-role update of an editable role is refused (`CANNOT_UPDATE_SELF_ROLE` —
  `role-assignment-write.service.ts:190-209` for assignment; the role-update lockout variant:
  revoking `canUpdateAllSettings` from your own role without an explicit ROLES flag →
  `CANNOT_REVOKE_OWN_SETTINGS_ACCESS`, `validate-role-mutation-does-not-lock-out-actor.util.ts:30-61`).
- **Object-permission consistency**: granting update/softDelete/destroy while read is false →
  `CANNOT_GIVE_WRITING_PERMISSION_ON_NON_READABLE_OBJECT`
  (`object-permission/object-permission.service.ts:264-331` **[CC]**); system objects cannot carry
  object/field permissions (`CANNOT_ADD_*_PERMISSION_ON_SYSTEM_OBJECT`, `field-permission.service.ts:409-417`;
  codes `permissions.exception.ts:39-40`).
- **Field-permission consistency**: only restrictions (`ONLY_FIELD_RESTRICTION_ALLOWED` — any
  `canReadFieldValue=true`/`canUpdateFieldValue=true` input throws,
  `field-permission.service.ts:379-392`); the role must already have an object permission
  (`OBJECT_PERMISSION_NOT_FOUND`, `:448-462`); duplicates in one input → `UserInputError`
  (`:369-377`); RLS-group-scoped masks must target a root group of the same role+object and cannot
  restrict non-nullable or morph fields (`:465-513`); write-privileges require read-privileges
  (`CANNOT_GIVE_WRITING_PERMISSION_WITHOUT_READING_PERMISSION`, code `permissions.exception.ts:44`).
- **Assignment** (`role-assignment/role-assignment-write.service.ts` **[CC file]**, `:34-97,
  :143-188, :305-363`): self-assignment refused (`CANNOT_UPDATE_SELF_ROLE`, `:190-209`); role must
  `canBeAssignedToUsers` (`ROLE_CANNOT_BE_ASSIGNED_TO_USERS`, `:167-172`); non-standard roles need
  the `CUSTOM_ROLES` entitlement (`CUSTOM_ROLES_NOT_ENTITLED`, `:174-187` → validator
  `validate-flat-role-target-role-is-entitled.util.ts:26-45`: system builds, agent targets, and the
  standard Admin/Member ids are exempt; with billing disabled entitlements are all-true, §0.2);
  demoting the **last admin** → `CANNOT_UNASSIGN_LAST_ADMIN` (`:341-363`, admin identified by
  `STANDARD_ROLE.admin.universalIdentifier`); unknown userWorkspace → `USER_WORKSPACE_NOT_FOUND`
  (`:256-265`); a member without any role → `NO_ROLE_FOUND_FOR_USER_WORKSPACE` (`:318-326`).
- **Role targets** (`core.roleTarget`): exactly one target FK (CHECK `CHK_role_target_single_entity`,
  auth-permissions.md §2.1); unique `(workspaceId, userWorkspaceId)` when
  `isSystemSideEffect=false`; `isSystemSideEffect` targets are excluded from user-facing role reads
  (`role-permissions.service.ts:86` filter; `role-assignment-write.service.ts:381-384` likewise).
- **Individual roles** (per-workspaceMember "you-only" roles):
  `workspace-member-individual-role/services/workspace-member-individual-role.service.ts` **[CC]**
  — used by chat/agent contexts; Nudge exposes the universal-identifier helper in P5, defers
  management ops.

### 5.4 Error envelopes

`PermissionsException` → GraphQL: `permissionGraphqlApiExceptionHandler`
(`utils/permission-graphql-api-exception-handler.util.ts:14-92` **[CC]**): PERMISSION_DENIED /
NO_AUTHENTICATION_CONTEXT → `ForbiddenError` with `subCode` (`:18-27`); the 13 "self/lockout/
editable" codes → `ForbiddenError` (`:28-43`); arg-shaped codes → `UserInputError` (`:44-54`);
not-found codes → `NotFoundError` (`:55-60`); the rest rethrown raw (`:61-83`). REST:
`permissionRestApiExceptionCodeToHttpStatus` (`utils/permission-rest-api-exception-code-to-http-status.util.ts:5-72`
**[CC]**): 403 group `:9-26`, 400 group `:26-36`, 404 group `:37-43`, 500 group `:44-65`,
`RECORD_PERMISSION_QUERY_INVALID`/`RLS_VALIDATION_FAILED` → 400 (`:66-68`). Nudge mirrors both maps
one-for-one (`permissions::error`).

---

## 6. Front-facing consequences for wave 5 (brief — the front agent reads this)

What the UI gates on, upstream at the tag (all fed from `getCurrentUser`, which embeds the
permission state — `user.resolver.ts:172-192`; DTO from
`fromUserWorkspacePermissionsToUserWorkspacePermissionsDto.ts`, [CC]):

- **`permissionFlags: Record<PermissionFlagType, boolean>`** — settings-route visibility. The
  front resolves it through a map selector (`useHasPermissionFlag`,
  `twenty-front/src/modules/settings/roles/hooks/useHasPermissionFlag.ts:5-8`) and each settings
  path checks its flag (frontend-architecture.md §1.4/§615). Flags come from
  `SettingPermissionsService.getUserWorkspacePermissions` (`setting-permissions.service.ts:31-90`:
  per-flag OR over the user's role ids, grant rule is-permission-flag-granted, auth-permissions.md
  §2.2) — **Nudge must serve this exact map on `/auth/me` / getCurrentUser**, computed by P2
  (`permissions::user_workspace`).
- **`recordsPermissions`** (the same `RoleRecordsPermissions` tree of §2.1, with the
  `followUpdateWhenCreatePermissionIsDisabled` aliasing applied server-side,
  `setting-permissions.service.ts:77-88`) — front store keyed by objectMetadataId
  (`objectPermissionsByObjectMetadataId` selector, `useObjectPermissions.ts:4-11`). It gates:
  command-menu object-pick filtering (readable+creatable:
  `buildGlobalRecordCreationCommandMenuItems.ts:33-49` — filter = `canAccessThroughAllRecordsOrRls
  (read) && canCreateRecordsForObjectMetadataItem`), record-index/toolbars, and per-object edit
  affordances (`useCanUpdateObjectRecords.ts:15-19` = same matcher for `update`).
- **`objectMetadataIdsWithReadableRecords`** — the set of objects the current principal can reach
  at all (server: `record-permissions-read.service.ts:148-179`, [ENT]; via `canAccessSomeRecords`
  `:181-226` which is exactly §2.2's algebra evaluated without SQL). Nudge: computed in P2 from the
  same algebra (no per-record SQL), served with the current-user payload.
- **Per-record actions**: `recordPermissions(targets)` query (`§5.1`, ≤100 targets) returns
  `{canRead, canUpdate, canDelete, canSoftDelete}` per record — the front refreshes it after
  mutations (`useRefreshRecordPermissions.ts`, record-sharing module) and gates row menus. Server
  side is `findRecordsActions` (`record-permissions-read.service.ts:108-146`, [ENT]):
  granted-record-ids per operation via the in-memory `findAccessibleRecordIds`
  (§2.6/§4.2b) — `canUpdate/canDelete/canSoftDelete` each additionally require `canRead` (`:130-133`).
  Nudge serves this from P4 (`permissions::record_actions`).
- **Field UI**: `isUIEditable` (and object `isUICreatable/isUIVisibleInRelatedRecordsList`) hide
  inputs only; the *hard* field masks are `unreadableFields`/`uneditableFields` inside
  `recordsPermissions.objects[id]` (hide read-only, disable write, tooltip "restricted").
- Sharing panel + private toggle are behind `IS_RECORD_LEVEL_SHARING_ENABLED`/enterprise — Nudge
  ships the DTO (`RecordSharingDTO`, §4.3) with `isShareable:false, isPrivate:false,
  canManageSharing:false, hasManagedGeneralAccess:false, roles:[], shares:[]` defaults while the
  flag is off (a well-formed empty state, not a placeholder endpoint — mirrors D10 style).

---

## 7. Units — P1..P5 (exclusive ownership inside `crates/server/src/permissions/`)

File ownership: this wave owns **only** `crates/server/src/permissions/**` (new) plus its wiring
lines in `crates/server/src/main.rs` (module registration) and one seam call each in
`crates/server/src/query/**` at the slots wave 4 already reserved (`runner/base.rs` pipeline,
`entry/graphql.rs`, `entry/rest.rs` context build) — listed per unit. The backend agents own
`metadata/api*` and `workspace_migrations/**`; **no file in this plan touches those trees** (P5
lands `permissions::metadata_guard` as a callable crate; the *call sites* in
`metadata/api/handlers.rs` are a hand-off list, not an edit). No other worktree is modified.

Dependency order against wave 4 (query-layer.md §12 U1–U7): this wave **requires U2 (table shape/
SQL), U3 (find_many/find_one + base pipeline), U5 (mutations), U6/U7 (entry points)** merged —
on their branches they exist (`wave4/u1` branch carries U1–U2 in progress); the units below assume
the merged wave-4 tree. The one wave-4 change that is *ours to make* is replacing the no-op
row-access slot (query-layer.md §5.3) with the real evaluator: P3/P4 add calls inside
`query::runner` — kept minimal and behind `permissions::` traits so the runner compiles unchanged
if this wave lands last.

### Module layout

```
crates/server/src/permissions/
  mod.rs              re-exports; PermissionError + code enum (§5.4 maps); feature/entitlement stubs
  context.rs          PermissionsContext, principal resolution (P1)
  model.rs            ObjectRecordsOperationType, ObjectRecordsPermissions, RoleRecordsPermissions,
                      RowLevelRecordsPermissions, SQLCondition, RecordReadRestrictions,
                      MetadataWritability (legacy), disabled-action enums (P1)
  kernel.rs           compute_role_records_permissions + union/intersection + create-follows-update
                      + runtime-limiting-role seam (P2)
  restrict.rs         disabled permission actions (object + field, morph, label-identifier) (P2)
  validate.rs         validate_operation_is_permitted_or_throw (§2.9) + object gate for writes/reads (P2)
  settings.rs         SettingPermissionsService port: hasPermissionFlag + user_workspace_permissions
                      (flags map + recordsPermissions + readable set) (P2)
  row_access.rs       build_access_sql_condition + outcome combine (and/or/bool) (P3)
  rls.rs              RLS rule shape → filter → SQL (degradedRead renderer) (P4)
  record_actions.rs   find_records_actions in-memory evaluation + targets cap (P4)
  share_sql.rs        build_record_share_sql_condition (flag-gated, behind trait) (P4)
  private_sql.rs      build_private_record_state / creator conditions (flag-gated) (P4)
  masks.rs            build_unreadable_fields_sql_conditions (per-rule hide/show algebra) (P4)
  metadata_guard.rs   AccessPolicy + auth-principal classification + the 5.2 table as data (P5)
  hooks.rs            role/roleTarget mutation rules: lockout, last-admin, default/member,
                      object/field permission consistency (§5.3) (P5)
  error.rs            exception codes/messages + GraphQL/REST mappings (§5.4) (P1)
```

### Units table

| # | Unit | Files | Contents (upstream anchors) | Acceptance criteria |
|---|---|---|---|---|
| P1 | **Model + context** | `mod.rs, model.rs, context.rs, error.rs` | Types per §2.1/§2.7; `PermissionsContext::from_auth_context` (principal resolution — port `resolve-permissions-principals-with-roles.util.ts` subset: workspaceMember/apiKey/agent/application/system branches, roleIds union §2.7); `NO_RECORD_READ_RESTRICTIONS`/`NO_ROLE_RECORDS_PERMISSIONS`; error codes with the §5.4 status maps verbatim | unit tests: every principal kind of an Nudge auth context resolves roleIds per `get-permissions-principals-role-ids.util.ts:7-21`; system context ⇒ `canReadSomeRecords=true` short-circuit; error→HTTP table byte-equal to `permission-rest-api-exception-code-to-http-status.util.ts:5-72` (table-driven test); no DB |
| P2 | **Kernel** (roles→permissions, CC parity) | `kernel.rs, restrict.rs, validate.rs, settings.rs` | §2.8 `compute_role_records_permissions` port (workflowRun/workspaceMember/agentChatThread special cases §2.8; `resolveObjectPermissionValue` §2.8); union/intersection per §2.7 with `NO_ROLE_RECORDS_PERMISSIONS` empty-guard; create-follows-update aliasing (§2.2 `get_enforced_operation_type` + `follow_update…`); disabled-actions restriction (§3.4 incl. application-owner exemption + label-identifier + inverse-relation propagation, `get-flat-field-metadatas-with-disabled-actions-by-object-id.util.ts:14-36`); `validate_operation_is_permitted_or_throw` (§2.9 incl. RLS-field create exemption); `has_permission_flag` (§5.1); `user_workspace_permissions` payload (§6); a `PermissionCache` keyed `(workspaceId, metadataVersion)` (D6) storing `rolesPermissions` + computed `RecordReadRestrictions`, invalidated by the wave-3 `metadataVersion` bump (`metadata/cache.rs:61-66` existing seam) | unit tests golden-tabled against every branch of `compute-permission-{union,intersection}.util.ts` and §2.8 (multi-role: 2× custom + limiting-agent case; morph restriction; `areSystemObjectsReadOnlyByDefault` case); integration vs live `twenty_ref`: for each seeded role (Admin/Member/any custom created through the reference API), the Rust-computed `objects[oid]` JSON equals the reference's `getCurrentUser.recordsPermissions` entry (normalized keys; the reference DB is the oracle, `docker exec infra-db-1 psql -U postgres -d twenty_ref`; e2e via `parity-compare.py` `getCurrentUser` case); cache test: same (ws, mv) computes once (counter), bump mv recomputes |
| P3 | **Row-access plumbing + gates (flags-off default = self-host behavior)** | `row_access.rs` + seam edits in `query/runner/{base,find_many,find_one}.rs`, `query/runner/{create,update,delete}.rs`, `query/repository/*` | Port §2.2 algebra: `resolve_row_access_outcome` (disabled/denied fast paths incl. owning-application exemption), all-records arm, and the combine helpers (`combine-sql-condition-outcomes-with-and/or.util.ts` semantics: bool×SQL merge, param collision suffixing); `build_read/write_records_access_sql_condition`; repository-side tri-state handling: main-alias deny → `PERMISSION_DENIED` (message format `workspace-repository.ts:2283`), join deny → `1=0` join condition (`:2288`, `:99`), condition → `rowAccessConditions` push (query-layer §5.1 already fixed the render position); `is_read_disabled` + nested-load/empty-relation behavior (§1.4); write gates wired: `validate_write_permitted` before insert/update/mutation (§1.2 table), `resolve_writable_record_ids` for batch updates, `shouldWriteAtLeastOneRecord → RECORD_NOT_EDITABLE` (`workspace-repository.ts:1536-1541`), `created_by`-creator condition seam (§2.5) **behind flags, default off**; REST `recordPermissions`-equivalent NOT here (P4) | e2e (both servers, reference `:3001` / nudge `:3000`, seeded tokens): (a) Member and Admin see identical `people` result sets on the reference vs nudge for 20 default roles including objectPermission overrides created via the reference metadata API (custom-role read deny on one object: set `objectPermission{role, person, canReadObjectRecords:false}` through the reference, query nudge → expect 403 PERMISSION_DENIED GraphQL `extensions.subCode` and REST 403 — codes per §5.4); (b) deny on a relation target (`company.read=false`, select `person { company { name } }`) → **no error, `company: null`** (join deny behavior, `:2270-2289`) on both; (c) filter/order by an unreadable field → `PERMISSION_DENIED` (assert vs reference); (d) update/delete with `updatedColumns` containing an uneditable field → 403 with the field name in `message`; (e) `updateOne` of a row the role can read but not update → `RECORD_NOT_EDITABLE`; (f) all flags off ⇒ emitted SQL for reads/mutations **equals the wave-4 SQL except for the absent row-access clause** (golden: capture reference `getQuery()` output on the same requests — `(userWhere)` only); perf budget per §7.1 |
| P4 | **Feature-flag-gated engines (implemented, default-off) + RLS degradedRead + record actions** | `rls.rs, masks.rs, share_sql.rs, private_sql.rs, record_actions.rs` | `rls.rs`: rule→filter builder per §2.4 (group AND/OR recursion, UNSATISFIABLE constant, member-value binding from `PermissionsContext.workspaceMember` row + cached `rlsWorkspaceMemberRelationValueMaps` equivalent, operand allowlist check, morph empty special case) rendered through **query::filter::sql** (`compute_where_condition_parts` reuse — single operator grammar, §2.4 note); relation-hop requires `IS_ROW_LEVEL_PERMISSION_RELATION_HOP_ENABLED` → UNSATISFIABLE when off (`build-rls-record-filter.util.ts:161-167`); `masks.rs`: §3.3 hide/show algebra verbatim (`(hide) AND (show) IS NOT TRUE`) + placeholder fields (constant §3.3), wired into `query::runner` pass-3 slot; `share_sql.rs`/`private_sql.rs`: the SQL shapes of §4.2(a)/§2.5 behind `IS_RECORD_LEVEL_SHARING_ENABLED`/`IS_PRIVATE_RECORDS_ENABLED` (both default false; the condition combine stays the single point of truth); `record_actions.rs`: `findRecordsActions` in-memory mirror (`find_accessible_record_ids.util.ts:12-61`) served at the `/metadata`-plane query `recordPermissions(targets)` (§5.1, cap 100, non-user → AuthenticationError) + `canReadSomeRecords`/`objectsWithReadableRecords` (§6) | unit tests: RLS filter trees (nested groups, each operand vs the wave-4 operator tables, UNSATISFIABLE branches, member-bound value cases incl. option-filter narrowing + incompatible field → unsatisfiable); mask algebra table (two rules hide/show the same field); integration vs reference: **against the seeded enterprise-capable reference this is not exercisable (rules cannot be created without the enterprise plan, §0.2 gate)** ⇒ parity scope = **flag-off equality** (identical results/errors with flags false vs reference) **plus** a fixture-driven suite: RLS/`_isPrivate`/share states injected directly into a scratch Nudge DB (rows written by the harness, not the API — the storage is ours), asserting (i) `degradedRead` SQL matches the reference renderer output captured from `renderRlsRecordFilterToSql` unit fixtures ported from upstream `internal/rls/utils/__tests__` (4 spec files at the tag; port their cases), (ii) `recordShare`-condition rows visible only to shared members, (iii) private rows invisible to Admin but visible to creator, (iv) `recordPermissions` equals the row visibility of the actual query results for the same targets; e2e gate: nudge flags off → the full wave-4 parity suite stays green (zero regressions when every permission feature is off = the self-host default contract) |
| P5 | **Metadata-plane guard + role mutation rules** | `metadata_guard.rs, hooks.rs` (+ hand-off doc of call sites) | §5.2 policy table as a `Policy` const-map + `classify_auth_principal` port (CC guard stack §5.1) + `required_feature_flags` support (used by `upsertRowLevelPermissionRules`: **Nudge decision D3 — RLS rule writes require `ROW_LEVEL_PERMISSION_FEATURE = hasEntitlement(RLS) && enterprise_valid`; with billing disabled upstream returns true for entitlements and `enterprisePlanService.isValid()` is false without a key (`billing.service.ts:129-143`, `enterprise-plan.service.ts:214-216` + validity-token load `:169-212`) ⇒ stock self-host = false; Nudge mirrors with an `Entitlements` stub (all-true, per `hasEntitlement`) + `EnterprisePlan::is_valid()` (false without `ENTERPRISE_KEY`/valid token) — same default-off posture, but the gate is a function, so enabling billing later flips behavior without code)**; `hooks.rs`: §5.3 full rule set (editable/default/member/last-admin/self-role/settings-revocation; object-permission read-implies-write consistency; field-permission restriction-only + system-object + root-group + nullable/morph rules; role-assignment `canBeAssignedToUsers` + CUSTOM_ROLES entitlement + roleTarget rebind on delete; `isSystemSideEffect` filtering); individual-role universal-id helper (§5.3 last bullet) | unit tests per rule with the exact `PermissionsExceptionCode` (§5.4); integration vs reference: drive the same metadata mutations against both servers with a Member-role token — expect 403 DATA_MODEL (`FORBIDDEN` subCode) on createOneObject/updateField (`curl -s` parity of status + `errors[].extensions.subCode` vs `code`), 200 with Admin token; role mutation matrix (create custom role → assign → demote last admin → expect `CANNOT_UNASSIGN_LAST_ADMIN`; delete default → `DEFAULT_ROLE_CANNOT_BE_DELETED`; update self role → `CANNOT_UPDATE_SELF_ROLE`) asserted code-equal vs reference responses; guard-stack e2e: apiKey token on `upsertObjectPermissions` allowed per policy `apiKey:true` vs `ROLE_ASSIGNMENT` `apiKey:false` refused |

### 7.1 Performance budget (the D6 anchor, extended)

- Permission data — `rolesPermissions` per role, flat maps, featureFlagsMap, workspaceMember row —
  is **compiled once per `(workspaceId, metadataVersion)`**: upstream does exactly this via the
  workspace cache (`workspace-roles-permissions-cache.service.ts:52` `@WorkspaceCache('rolesPermissions',
  {packingPonderation: 2})`; RLS upserts invalidate the key,
  `row-level-permission-predicate.service.ts:510-512`; auth-permissions.md §2.2 cache semantics;
  query-layer.md §2.4/D6 rationale). Nudge: `PermissionCache` in
  `permissions/mod.rs` = moka LRU keyed `(workspaceId, metadataVersion)` →
  `{roles_permissions, restrictions_by_role_hash}`; a metadataVersion bump (already the wave-3
  invalidation trigger, `metadata/cache.rs:195-197`) retires entries naturally; **zero SQL per
  record query** after warmup.
- Per request, the only added round-trips are the ones upstream would need but Nudge avoids:
  degradedRead evaluates member-bound predicates from the **context's** workspaceMember snapshot —
  upstream caches `rlsWorkspaceMemberRelationValueMaps` per workspace (`permissions-metadata-maps.service.ts:71-77`);
  Nudge loads it into the same cache version. Share/private fast paths issue **no** queries in the
  hot path (the share condition is inlined SQL; `recordPermissions` is the only per-record query and
  is batched: ≤100 targets, ≤1 query per (object, op) set — `find_record_ids_matching_sql_condition`).
- Compile-once per (workspaceId, metadataVersion, object, op) for the rendered RLS/sharing/private
  SQL condition (the condition is deterministic given those inputs; upstream re-renders per call,
  `build-access-sql-condition` — caching it is a pure Nudge optimization behind a hash of
  `rolesPermissions + flags`, invalidation = same metadataVersion key as D6).
- Budget gate: with all permission features **off** (the self-host default), p50 added latency on
  `/rest/people?limit=20` measured by `rust/tools/parity-bench.py` must be ≤ 1 ms vs the wave-4
  baseline (condition absent ⇒ same SQL plan); with a 3-rule RLS workspace fixture, ≤ 4 ms p50.

### 7.2 Sequencing and parallelism

P1 → P2 → {P3, P4} → P5. P3 and P4 may run in parallel (P4's modules are pure `permissions::`
files; P3 owns the runner seam edits). P5 is independent of P3/P4 (no record-path coupling) —
it can start after P1. Five engineers: {P1+P2}, {P3}, {P4}, {P5}, {tools: parity-compare record-
permission cases + fixtures}.

---

## 8. Documented decisions and deviations

Upstream behavior is followed unless stated. Numbered like query-layer.md §11.

- **D0 — Scope decision: CC-license-equivalent enforcement (the master decision).** The
  permission wave implements what a **stock self-hosted Twenty v2.61.0 (no ENTERPRISE_KEY,
  billing off, default feature flags) actually does**, with every row-level [ENT] engine
  *implemented* but flag-gated so enabling a flag is a config change, never a code gap. Rationale
  and evidence: §0 (module license split 121 CC / 246 [ENT]; self-host gates
  `is-access-groups/:private/:sharing/:related-paths/:rls-hop-enabled.util` all `?? false`;
  `billing.service.ts:129-143`; `row-level-permission-predicate.service.ts:515-525`). Object- and
  field-level permissions (the parts of the model that are [CC] and self-host-on) are ported
  **fully and by default**: they change real self-host behavior (a Member-role objectPermission
  deny must 403 in Nudge exactly as in the reference). Enterprise pieces become **documented
  deferred parity** with file evidence, not placeholders: access-group
  materialization/sync/propagation (`internal/access-group/services/*` ENT;
  `record-access-group-write.service.ts` ENT; `build-recompute-record-access-group-ids-query.util.ts`
  ENT), related-record paths (`internal/related-record-path/*` ENT), `shareWith` + sharing write
  services (`record-share/services/record-sharing-{write,read}.service.ts` ENT; §4.3/§4.4),
  member-successor share transfer (`record-share.repository.ts:148-216` ENT), permission-tables
  viewer (`permission-table/**` ENT), runtime-limiting AI scopes (seam only,
  `restrict-records-permissions-to-runtime-limiting-role.util.ts` CC — computation ported, wiring
  awaits the AI wave), and the metrics trio shipped with this very tag (`internal/metrics/*` ENT; D9).
  Not deferred: channel-visibility field redaction is Apache
  (`engine/api/common/common-record-visibility/common-record-visibility.service.ts:16`) and is still
  missing from Nudge — open gap, see §2.10.
- **D1 — The wave-4 no-op row-access slot becomes live.** query-layer.md §5.3/D9 shipped
  `onBeforeExecute` with an empty condition list. P3 replaces the slot body with §2's algebra.
  Observable output is unchanged while all flags are off (the empty-condition render path is
  byte-identical, `build-select-statement.util.ts:555-557`), so the wave-4 parity suite is the
  regression net.
- **D2 — Feature flags are read from the workspace flag map, fail-closed.** `IS_ACCESS_GROUPS_ENABLED`
  (`kernel.rs:512-516`), `IS_PRIVATE_RECORDS_ENABLED` / `IS_RECORD_LEVEL_SHARING_ENABLED`
  (`kernel.rs:518-527`) and the relation-hop conjunction (`model.rs:301-311`,
  `IS_ACCESS_GROUPS_ENABLED && IS_ROW_LEVEL_PERMISSION_RELATION_HOP_ENABLED` as in
  `internal/rls/utils/is-rls-relation-hop-enabled.util.ts:9-12`) all default to false when the flag
  row is absent, matching `?? false` upstream. The earlier hard-force of
  `is_access_groups_enabled = false` is gone now that company access-group materialization exists
  (`access_group.rs`); the flag alone decides. Two genuine drifts remain: Nudge creates the
  `accessGroup`/`recordShare` tables unconditionally (`standard_objects.rs:483`,
  `standard_objects/permissions.rs:97`) where 2.61.0 gates that DDL on `IS_PERMISSION_TABLES_ENABLED`
  (`FeatureFlagKey.ts:18`, read in `twenty-standard-application.service.ts:71`), and
  `IS_RELATED_RECORD_PATHS_ENABLED` (`FeatureFlagKey.ts:10`) has no Nudge reader.
- **D3 — RLS rules: enforce, don't allow creation (mirrors the self-host gate).**
  `upsertRowLevelPermissionRules` (and the predicate-group/predicate child writes) requires
  `billing.hasEntitlement(RLS) && enterprisePlan.isValid()` — Nudge implements the conjunction with
  the same defaults as upstream: entitlements all-true when billing is disabled
  (`billing.service.ts:133-137`), `isValid()` false without a valid validity token
  (`enterprise-plan.service.ts:214-216` + token load `:169-212`) ⇒ **stock self-host: rules cannot
  be created**, and neither can they in Nudge (code `ROW_LEVEL_PERMISSION_FEATURE_DISABLED` →
  Nudge maps to the same 4xx via the §5.4 table). The *enforcement* half is implemented regardless
  (P4 `rls.rs`) because rows may carry rules imported from a Twenty instance with a plan (the
  migration story), and because it is the only way to exercise the flag-on path with fixtures.
  Internal system builds that write rules directly (the workflow-run path) bypass the entitlement
  check; at 2.61.0 channel visibility is *not* one of them — it is the Apache field-redaction layer
  in §2.7, which has no rule rows behind it.
- **D4 — Sharing and privacy: condition renderers implemented, write services deferred, DTOs
  complete.** `build_record_share_sql_condition` and the private/creator conditions port the
  upstream SQL shapes verbatim (§4.2a, §2.5) behind `IS_RECORD_LEVEL_SHARING_ENABLED` /
  `IS_PRIVATE_RECORDS_ENABLED` (default off ⇒ `false` arms, upstream-exact). The write side
  (`setRecordPrivate/setRecordShare/removeRecordShare`, `shareWith` on create) lands **when the
  flags are productized** (needs the [ENT] `record-sharing-write.service.ts` semantics: advisory
  lock `build-record-share-lock-key.util.ts` [CC], FULL-management authorization, rowCause book-
  keeping, owner-share auto-write `build-record-share-inputs-for-created-records.util.ts:117-173`);
  the `recordShare`/`accessGroup` tables and columns already exist (U7). `recomputeRecordAccess-
  GroupIdsAfterWrite` (query-layer.md §5.2) is a **no-op recorder** behind
  `permissions::mod::access_group_ops_enabled()` — never a silent empty array claim.
- **D5 — `restore` is permissioned as `softDelete`** (upstream `OPERATION_TYPE_BY_MUTATION_KIND`
  maps restore→softDelete, `workspace-repository.ts:110-118`), even though `restore` reads as a
  different API verb. Reproduced exactly; Nudge's delete.rs must not "fix" this.
- **D6 — `PermissionCache` keyed `(workspaceId, metadataVersion)`** — one compile of
  `rolesPermissions` + per-context `RecordReadRestrictions` per metadata version (upstream:
  `@WorkspaceCache('rolesPermissions', {packingPonderation: 2})`
  `workspace-roles-permissions-cache.service.ts:52` + pub/sub invalidation,
  query-layer.md §2.4/D6). Bounded moka LRU; invalidated on the wave-3 `metadataVersion` bump
  (`metadata/cache.rs:195-197` existing seam) **and** by any role/permission mutation routed
  through Nudge's own metadata plane (the [ENT] `invalidateAndRecompute('rolesPermissions')`
  hook, `row-level-permission-predicate.service.ts:510-512`, plus the equivalent calls on
  objectPermission/fieldPermission/role upserts).
- **D7 — No AsyncLocalStorage / per-context WeakMap.** `PermissionsContext` is built once per
  request from the wave-4 `QueryContext` (token payload already carries `workspaceMemberId`;
  query-layer.md §1.4) and passed explicitly (query-layer.md D1). The upstream
  `WeakMap<PermissionsPrincipalsWithRolesContext, …>` memoizers
  (`record-permissions-read.service.ts:94-97`, `permissions-metadata-maps.service.ts:24-27`) are
  replaced by the request-scoped context — same effect, deterministic lifetime.
- **D8 — Error mapping is byte-parity.** Every throw site in §1–§5 reproduces upstream's
  `PermissionsExceptionCode` → GraphQL `extensions.subCode`/userFriendlyMessage and REST
  statusCode (the two CC maps in §5.4), and `RECORD_NOT_EDITABLE`/`RLS_VALIDATION_FAILED`/
  `PERMISSION_DENIED` surface to `/rest` as 403/400/403 respectively. ADR-005: codes equal, message
  text best-effort.
- **D9 — Metrics deferred.** The tag adds permission telemetry
  (`internal/metrics/services/permissions-configuration-gauge.service.ts`,
  `access-group-propagation-gauge.service.ts` ENT; counters
  `record-permissions-read.service.ts:406-413`). Nudge emits the equivalent signal as `tracing`
  span fields (`ws.id`, `op`, `strategy`) with no ClickHouse sink (observability wave); documented,
  not silently dropped.
- **D10 — `recordPermissions` stays `/metadata`-plane only.** Upstream exposes it as a
  `@MetadataResolver` query (`record-permissions.resolver.ts:24`); no REST route exists. Nudge
  mirrors: `/metadata` query + internal `permissions::record_actions` API for the front (wave 5);
  no invented REST surface.
- **D11 — `_isPrivate` rows stay non-private.** The storage defaults (`false`) are already in the
  wave-3 DDL; nothing writes `true` until D4's write services land; `isPrivateByDefault` objects
  (agentChatThread/Participant) are catalogued but the enforcement reads the same flag gate (§2.5).
  This matches a stock self-host with the flag off, where private state has no creation path.
- **D12 — System-object writability follows the kernel, not a rewrite.** `resolveObjectPermissionValue`
  keeps the `isSystem || hasAllObjects` default (CC helper, §2.8): system objects are
  readable/writable for *everyone* unless a role's objectPermission override or a
  disabled-action restriction says otherwise — the reason Nudge must implement
  `restrict-records-permissions-to-disabled-actions` (P2) before trusting any "Admin-only"
  assumption about system tables.

## 9. Open risks

- **Flag half-states.** D2 hard-closes access groups, but `IS_PRIVATE_RECORDS_ENABLED` alone
  (without sharing writes, D4) would make records un-readable-by-creator invisible after a
  creator-transfer — self-consistent with upstream (upstream has the same flag coupling via
  `areRecordSharesApplied`), yet worth an e2e case in P4 (flag matrix 2×2 of private/sharing off).
  Mitigation: the P4 fixture suite enumerates the four flag states.
- **DegradedRead cost.** RLS predicates re-rendered per query are O(rules × predicates) SQL text;
  upstream's fast path is the access-group array. A workspace with many rules will be measurably
  slower than a paid upstream instance — acceptable at default flags (rules are rare) but the §7.1
  budget test (3-rule fixture) keeps it honest; revisit with P-later if rules become common.
- **Cursor/orderBy interaction with masks.** Order-by on an RLS-masked column relies on
  `andWhereReadingUnreadableColumnsAsNull` + the masked select reading NULL
  (`workspace-select-query-builder.ts:210-226`, query-layer.md §3.5). Nudge's pass-3 slot (P4) and
  the keyset path (wave 4 U1/U3) must be tested *together*: pagination-walk e2e with a masked order
  column asserting no row skips/dupes.
- **`getCurrentUser` payload contract.** The front (wave 5) depends on
  `permissionFlags + recordsPermissions + objectMetadataIdsWithReadableRecords` shapes (§6); any
  drift (e.g. omitting `canReadSomeRecords`, or the create-follows-update aliasing D5/§2.2) breaks
  UI gating silently. Mitigation: the P2 integration test diffs the **normalized JSON of
  `getCurrentUser`** between nudge and reference for Admin, Member, and a reduced custom role
  (flag-already-required by the parity harness; add the reduced-role case).
- **The 1=0 join-deny vs aggregate clones.** `getCount` runs `onBeforeExecute` on the same builder
  (`:717-720`); if a join alias is denied, `totalCount` must reflect the same (empty) join, not a
  stale count. The clone path copies `rowAccessConditions` (`:143-144, :189-191`) — Nudge's
  builder-clone equivalent must copy the condition list too (assert in P3: count query SQL equals
  the main query's WHERE + row-access clause).
- **Upstream drift of the CC/ENT split.** The license boundary moved twice within recent releases
  (2.53 moved `writability` onto disabled actions, 2.57/2.58 restructured RLS rule links). Pinned
  to `twenty/v2.61.0`; a wave-8 re-pin must re-run this doc's §0 split scan (the python walk used
  here is cheap to automate into `rust/tools/`).
- **Parity-oracle limits.** The reference cannot *create* RLS rules / enable flags without an
  enterprise key (§0.2), so the flag-on acceptance is fixture-based (states injected into the
  scratch DB). Risk: a divergence between our fixture state and upstream's true flag-on behavior
  goes uncaught until a paid environment is available. Mitigation: upstream's own [ENT] unit specs
  (`internal/rls/utils/__tests__` — 7 files at the tag; `internal/combine/utils/__tests__` — 3;
  `internal/access-group/utils/__tests__` — 9) are the transplanted oracle; port their cases as
  Nudge golden tests.
