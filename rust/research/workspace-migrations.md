# Twenty CRM — Workspace Migrations & Metadata Engine (wave 3)

Research doc for the Rust port (`nudge`). Reference source (read-only):
`/Users/tonminh-mac/Documents/GitHub/twenty` @ commit `77a32af` (2026-11-26).

All paths relative to the Twenty repo root. `packages/twenty-server/src` is abbreviated `srv/`,
`packages/twenty-shared/src` is abbreviated `shared/`. Schema facts already pinned in
`data-model.md` are cross-referenced (§) and not re-derived here.

> **Path drift vs data-model.md.** data-model.md pins commit `51cff3a`; this doc was taken at
> `77a32af` (8 days later). The metadata API lives in `srv/src/engine/metadata-modules/{object,field,index}-metadata/`
> (there is no `srv/src/engine/core-modules/metadata/`). The engine was refactored to
> *universal actions* (universalIdentifier-keyed) transpiled to *flat actions* (id-keyed) at run time;
> line numbers below are from the `77a32af` checkout.

---

## 1. Mental model

```
desired metadata (universal flat maps)
  → diff against current maps          (builders, per metadata name)
  → universal actions (create/update/delete per metadata name, universalIdentifier-keyed)
  → aggregation (fold fields into object creates, pair relation fields, deprioritize tsvector)
  → ordered action list                (WorkspaceMigration { actions, applicationUniversalIdentifier })
  → runner: per action → transpile to flat action (ids) → execute twice in ONE transaction:
        executeForMetadata         — INSERT/UPDATE/DELETE rows in core metadata tables
        executeForWorkspaceSchema  — DDL in the workspace schema (CREATE TYPE/TABLE, ALTER TABLE, CREATE INDEX)
  → post-commit: cache invalidation + core.workspace.metadataVersion increment
  → deferred operations persisted in core."deferredWorkspaceOperation" and run by a queue job
```

The built migration is **ephemeral** — there is no `workspaceMigration` table in `core`
(verified: `rust/baseline/core-2.61.0.sql` has no such table). Only deferred operations are persisted.
Recovery after a failed run = rollback + best-effort metadata rollback (§4.4), not replay.

## 2. The action model

### 2.1 Action type enum

Only three: `delete`, `create`, `update` —
`srv/src/engine/workspace-manager/workspace-migration/workspace-migration-builder/constants/workspace-migration-action-type.constant.ts:1-5`.
All DDL variety (create table, add/drop/rename column, create/drop/rename enum, create/drop index,
tsvector rebuild) is *derived inside the runner handlers* from these three metadata-level verbs
plus the flat entity contents.

### 2.2 The migration envelope

```ts
// srv/.../workspace-migration-builder/types/workspace-migration.type.ts:3-9
type WorkspaceMigration<TActions = AllUniversalWorkspaceMigrationAction> = {
  actions: TActions[];
  applicationUniversalIdentifier: string;
};
```

`AllUniversalWorkspaceMigrationAction` =
`MetadataUniversalWorkspaceMigrationAction<TMetadataName, TActionType>` —
`srv/.../workspace-migration-builder/types/workspace-migration-action-common.type.ts:9-29`
(the same file defines the handler registry key `` `${actionType}_${metadataName}` `` at :23-29).
Per-metadata-name wiring:
`srv/src/engine/metadata-modules/flat-entity/types/metadata-workspace-migration-action.type.ts:6-25`.

### 2.3 Base action shapes

| Kind | Interface | Fields |
|---|---|---|
| universal create | `BaseUniversalCreateWorkspaceMigrationAction<T>` `srv/.../types/base-universal-create-workspace-migration-action.type.ts:7-19` | `flatEntity` (universal flat entity minus FK-aggregator properties), `type: 'create'`, `metadataName: T`, `id?` (optional preallocated uuid) |
| universal update | `BaseUniversalUpdateWorkspaceMigrationAction<T>` `.../base-universal-update-...:8-17` | `type: 'update'`, `metadataName`, `universalIdentifier`, `update: UniversalFlatEntityUpdate<T>`, `flatEntity?`, `diff?` |
| universal delete | `BaseUniversalDeleteWorkspaceMigrationAction<T>` `.../base-universal-delete-...:6-13` | `universalIdentifier`, `type: 'delete'`, `metadataName`, `flatEntity?` |
| flat create | `BaseFlatCreateWorkspaceMigrationAction<T>` `.../base-flat-create-...:6-11` | `flatEntity` (id-keyed), `type`, `metadataName` |

The "universal" variant stores `universalIdentifier` where the flat variant stores DB ids
(`objectMetadataId`, `relationTargetFieldMetadataId`, …). Transpilation resolves them through the
flat-entity maps cached from `core` (§4.2).

### 2.4 The action types that matter for wave 3

`objectMetadata` — `srv/.../builders/object/types/workspace-migration-object-action.type.ts:11-35`:

```ts
type UniversalCreateObjectAction = Base<...> & {
  universalFlatFieldMetadatas: Omit<UniversalFlatFieldMetadata, FkAggregator>[];
  fieldIdByUniversalIdentifier?: Record<string, string>;
};
type UniversalUpdateObjectAction = Base<...>;                       // :20-21
type UniversalDeleteObjectAction = Base<...>;                       // :23-24
type FlatCreateObjectAction = BaseFlat<...> & { flatFieldMetadatas: FlatFieldMetadata[] };  // :26-29
```

`fieldMetadata` — `srv/.../builders/field/types/workspace-migration-field-action.type.ts:11-42`:

```ts
type UniversalCreateFieldAction = Base<...> & {
  relatedUniversalFlatFieldMetadata?: ...;   // other side of a relation, migrated in the same action
  relatedFieldId?: string;
  fieldIdByUniversalIdentifier?: Record<string, string>;
};
type UniversalUpdateFieldAction = Base<...> & { rebuildSearchVector?: boolean };   // :23-26
type FlatUpdateFieldAction     = BaseFlat<...> & { rebuildSearchVector?: boolean }; // :36-39
```

`index` — `srv/.../builders/index/types/workspace-migration-index-action.type.ts:10-31`:
update actions carry the full new entity (`updatedUniversalFlatIndex` / `updatedFlatIndex`);
comment at :29: *"Literally drop and create under the hood"*.

The remaining ~41 metadata names (`view`, `viewField`, `role`, `pageLayout*`, `navigationMenuItem`,
`webhook`, `agent`, …) use the base shapes verbatim; the full mapping table is
`srv/src/engine/metadata-modules/flat-entity/types/all-flat-entity-types-by-metadata-name.type.ts:480+`
(each entry lists `universalActions` and `flatActions` records keyed by create/update/delete).
Nudge should implement object/field/fieldMetadataOption/index/searchFieldMetadata handlers first
(§11) — everything else is metadata-row-only CRUD.

### 2.5 Universal flat entity (the JSON shape)

A universal create action's `flatEntity` for `objectMetadata` is exactly the object produced by
`createStandardObjectFlatMetadata` (`srv/src/engine/workspace-manager/twenty-standard-application/utils/object-metadata/create-standard-object-flat-metadata.util.ts:98-160`):
`universalIdentifier`, `applicationId`, `workspaceId`, `nameSingular`, `namePlural`, `labelSingular`,
`labelPlural`, `color`, `description`, `icon`, `isRemote`, `isActive`, `isSystem`, `isSearchable`,
`isAuditLogged`, `isUIEditable`, `isUICreatable`, `disabledObjectPermissionActions`, `openRecordIn`,
`isLabelSyncedWithName`, `overrides`, `duplicateCriteria`, `shortcut`,
`labelIdentifierFieldMetadataId` + `...UniversalIdentifier`, `imageIdentifierFieldMetadataId` + `...`,
`targetTableName: 'DEPRECATED'`, the `*Ids: []` / `*UniversalIdentifiers: []` child lists,
`createdAt`/`updatedAt`, `id`, `applicationUniversalIdentifier`.
For `fieldMetadata` see `create-standard-field-flat-metadata.util.ts:86-151`; conventions in §8.2.

Update payloads are property-name → new-value maps validated per metadata name by
`sanitizeUniversalFlatEntityUpdate` (`srv/.../workspace-migration/universal-flat-entity/utils/sanitize-universal-flat-entity-update.util.ts`, called from the handler base `.../interfaces/workspace-migration-runner-action-handler-service.interface.ts:214-232`).

## 3. The builder pipeline

### 3.1 Entry points

`srv/.../workspace-migration/services/workspace-migration-validate-build-and-run-service.ts`:

| Method | Input | Use |
|---|---|---|
| `validateBuildAndRunWorkspaceMigrationFromTo` (:105-250) | `from`/`to` universal flat maps + build options | standard-application sync, app install/upgrade |
| `validateBuildAndRunWorkspaceMigration` (:252-278) | operation matrix `{ metadataName: { flatEntityToCreate/ToUpdate/ToDelete } }` | metadata API mutations (object/field/index services) |
| `validateBuildAndRunLegacyWorkspaceMigration` (:295-329) | same matrix, `skipSideEffectExpandEngine: true` | pre-2.19 upgrade commands only (deprecated) |

The matrix path first expands side effects (`metadataSideEffectEngineService.expandWithSideEffects`,
:369-387) — e.g. a field delete drags its options, indexes and page-layout widgets; an object create
gains system views/record-form page layout. Then `computeFromToAllFlatEntityMapsAndBuildOptions`
turns the matrix into from/to maps (`srv/.../services/workspace-migration-flat-entity-maps.service.ts`).

`buildOptions`: `isSystemBuild`, `inferDeletionFromMissingEntities`, `applicationUniversalIdentifier`,
`applicationDefaultRoleUniversalIdentifier?`, `authContext?` (type `srv/.../types/workspace-migration-builder-options.type.ts`).

Build result: `{ status: 'fail', report } | { status: 'success', workspaceMigration }`
(`srv/.../types/workspace-migration-orchestrator.type.ts`). On fail the caller throws
`WorkspaceMigrationBuilderException` carrying the per-metadata error report
(`object-metadata.service.ts:620-625`).

### 3.2 Orchestrator

`srv/.../services/workspace-migration-build-orchestrator.service.ts`:

1. `setupOptimisticCache` (:361-395) seeds the working maps with `from` maps + dependency maps.
2. Runs 44 per-metadata builder tasks in a **fixed order** (:183-358) — comment at :183:
   *"Execution order: builders mutate optimisticAllFlatEntityMaps that downstream builders read, so do not reorder"*.
   Head of the order: `objectMetadata → fieldMetadata → fieldMetadataOption → index → view → viewFieldGroup → viewField → viewFilterGroup → viewFilter → viewGroup → viewSort → searchFieldMetadata → logicFunction → role → …`.
3. `crossEntityTransversalValidation` (:438-447).
4. `computeSearchVectorRebuildTargetUniversalIdentifiers` (:463-472) — objects whose search fields
   changed get `rebuildSearchVector: true` injected into their `searchVector` field update actions.
5. `aggregateOrchestratorActionsReport` (:474-480).
6. Final list = `computeOrderedMigrationActions(aggregated)` (:487-489).

Each builder extends `WorkspaceEntityMigrationBuilderService<T>` and implements
`validateFlatEntityCreation/Update/Deletion`; e.g. the object builder
(`srv/.../builders/object/workspace-migration-object-actions-builder.service.ts:22-114`) delegates to
`FlatObjectMetadataValidatorService` and wraps the result into the action (§9.4).

### 3.3 Aggregation (what turns N actions into the DDL-friendly set)

`srv/.../utils/aggregate-orchestrator-actions-report.util.ts:7-28` runs four passes:

| Pass | File | Effect |
|---|---|---|
| fold non-relation fields into object creates | `aggregate-non-relation-fields-into-object-actions.util.ts:9-69` | every non-relation field create whose object is also created is pushed into `UniversalCreateObjectAction.universalFlatFieldMetadatas`; the object action creates table + columns in one go |
| pair relation fields | `aggregate-relation-field-pairs.util.ts:7-76` | the two sides of a relation found in the same report merge into one action: `flatEntity` (owner) + `relatedUniversalFlatFieldMetadata` + `relatedFieldId` |
| deprioritize tsvector creates | `aggregate-orchestrator-actions-report-deprioritize-ts-vector-create-field-actions.util.ts` | `searchVector` field creates run after all other field creates (the expression references sibling columns) |
| deprioritize search-vector updates | `.../aggregate-orchestrator-actions-report-deprioritize-search-vector-update-field-actions.util.ts` | same for rebuild updates |

### 3.4 Final action ordering

`srv/.../utils/compute-ordered-migration-actions.util.ts:4-183` — exact emission order (abridged;
keep the full function as a table in Rust):

1. `searchFieldMetadata` delete/create/update
2. `index` delete; `fieldMetadataOption` delete; `fieldMetadata` delete; `objectMetadata` delete
3. `objectMetadata` create/update; `fieldMetadata` create/update; `fieldMetadataOption` create/update; `index` create/update
4. `view*` (create view, then viewField/viewGroup/viewFilter/viewSort deltas, group deletes before group creates per the CASCADE comments at :38-39, view delete last)
5. `workflow`/`workflowVersion` (archive-active-before-create rule, :52-54)
6. `logicFunction`, `role`, `roleTarget` deletes before agent/webhook deletes (:65-66), agent, webhook, roleTarget create/update, object permissions, row-level rules before predicates (:83-85), permission flags, skill, agentChatChannel*, commandMenu/settingsMenu/frontComponent (delete items → create components → repoint items → delete components, :104-121), pageLayout* (widgets → tabs → layout deletes per CASCADE, :128-134), navigationMenuItem (creates/updates before folder deletes, :136-139), fieldPermission, rowLevelPermissionPredicate*, mcpServer, applicationVariable, connectionProvider/messagingProvider, timelineActivityType, relatedRecordPath, validationRule.

Violating this order breaks FK/CASCADE constraints — it is part of the contract.

### 3.5 ID pre-allocation

`enrichCreateWorkspaceMigrationActionsWithIds` (`srv/.../services/utils/enrich-create-workspace-migration-action-with-ids.util.ts`, called at validate-build-and-run :192-196) fills `id` / `fieldIdByUniversalIdentifier` from the provided `idByUniversalIdentifierByMetadataName` so that
related rows created in later actions of the same migration reference stable ids; the runner builds the
same map from the actions themselves (`build-preallocated-id-by-universal-identifier-from-actions.util.ts`, used at runner :377-378).

`dryRun: true` or `actions.length === 0` short-circuits with `hasSchemaMetadataChanged: false`
(:198-204).

## 4. The runner

`srv/.../workspace-migration-runner/services/workspace-migration-runner.service.ts`.

### 4.1 Guards before the transaction

* `WORKSPACE_SCHEMA_DDL_LOCKED` config → `WorkspaceMigrationRunnerException` code `DDL_LOCKED` (:249-255).
* If `shouldDeferActions` (default `true`, :188) and any action touches a schema-affecting metadata
  name, abort while schema-affecting deferred operations are PENDING/IN_PROGRESS (:190-193, :220-234;
  `in-flight-deferred-workspace-operations.service.ts:16-36`, error code
  `DEFERRED_WORKSPACE_MIGRATION_ACTIONS_IN_PROGRESS` — `exceptions/workspace-migration-runner.exception.ts:11-17`).
* Schema-affecting metadata names: `objectMetadata`, `fieldMetadata`, `fieldMetadataOption`, `index`
  (`constants/schema-affecting-workspace-migration-metadata-names.constant.ts:5-10`; predicate
  `utils/is-schema-affecting-workspace-migration.util.ts:5-13`).

### 4.2 Execution

* One `queryRunner` from the **core** datasource; `startTransaction()` (:382-383) — the per-workspace
  DDL runs on the same connection inside the same Postgres transaction (Postgres DDL is transactional).
* First statement in the tx: `SET LOCAL lock_timeout = '8s'` (:406).
* Loads the flat-entity maps for the involved metadata names (+`index` when a tsvector rebuild is
  present, +`fieldMetadata`/`objectMetadata` for option enum changes) from the workspace cache
  (:264-312), then `structuredClone`s them as the working maps (:342).
* Resolves the owning application from `applicationUniversalIdentifier` → `flatApplicationMaps` (:357-375),
  error `APPLICATION_NOT_FOUND` otherwise.
* For each action in order (:417-465):
  `registry.executeActionHandler` → handler `execute()` which runs
  `transpileUniversalActionToFlatAction` (universal→id-keyed, resolve + preallocate ids),
  then **`executeForMetadata` and `executeForWorkspaceSchema` concurrently** via `Promise.allSettled`
  (`interfaces/workspace-migration-runner-action-handler-service.interface.ts:257-314`, allSettled at :263-273).
  Any rejection → `WorkspaceMigrationRunnerException` code `EXECUTION_FAILED` with both errors attached (:278-289).
  The handler also derives `MetadataEvent[]` (created/updated/deleted with before/after payloads —
  `types/metadata-event.type.ts:12-38`) and at most one `DeferredWorkspaceOperation` (:296-305), and
  returns the optimistic map delta (:307-311).
* After all actions: persist deferred ops **inside the tx** (:467-474), then commit (:478).

### 4.3 Post-commit

* `invalidateCache({allFlatEntityMapsKeys...})` (:583-612) → `workspaceCacheService.invalidateAndRecompute`
  for the touched flat-map keys (+ derived keys: `rolesPermissions`/`rlsWorkspaceMemberRelationValueMaps`
  when roles or morph relations changed, `workflowAutomatedTriggerMaps`, `userApplicationVariableValueMaps`
  — :90-106). If schema-affecting, `workspaceMetadataVersionService.incrementMetadataVersion(workspaceId)`
  (:113-117) — bumps `core.workspace.metadataVersion` (data-model §2.3) and the Redis copy
  (`workspace-metadata-version.service.ts:32-53`); clients re-fetch metadata on version change.
* `dispatchAfterCommit` (:622-628): when ops were persisted → enqueue the BullMQ job
  `run-deferred-workspace-operations.job.ts` (dedup id `deferred-workspace-operations:<workspaceId>`,
  retry/backoff/priority constants — `deferred-workspace-operation-runner.service.ts:260-282`);
  when `shouldDeferActions=false` (seeding/upgrades) → execute inline, best-effort (:107-118, :284-319).

### 4.4 Failure path

On any error (:501-576):

1. Rollback the tx (guarded; :515-531).
2. Snapshot `pg_stat_activity` for diagnosis (:148-175, :533).
3. Run `rollbackForMetadata` for the actions **in reverse order** — outside any transaction, best-effort
   (:535-549; base class :316-330). Handlers only override it to delete core rows already inserted
   (e.g. `create-object` deletes the objectMetadata row); there is **no compensating DDL** —
   the tx rollback already reverted the workspace schema.
4. Invalidate cache (errors swallowed, :551-566), rethrow with code preserved (:568-576).

Idempotency: table and index creation are `IF NOT EXISTS` (`workspace-schema-table-manager.service.ts:30`,
`workspace-schema-index-manager.service.ts:66`) but enum creation is a plain `CREATE TYPE`
(`workspace-schema-enum-manager.service.ts:51`), so re-running a committed migration fails on the
first enum its fields create. A failed migration that rolled back commits nothing; the real
re-runsafety comes from the from/to diff being empty on second run, not from DDL idempotency.

### 4.5 Deferred operations

Row shape (`core.deferredWorkspaceOperation`, entity
`srv/src/engine/metadata-modules/deferred-workspace-operation/deferred-workspace-operation.entity.ts:16-66`;
physical schema `rust/baseline/core-2.61.0.sql:1269-1283`): `workspaceId`, `applicationUniversalIdentifier`,
`name`, `payload jsonb`, `position`, `runByVersion` (APP_VERSION at persist — runner service :85),
`status varchar default 'PENDING'`, `attempts int default 0`, `lastError text`, `startedAt`,
`createdAt`/`updatedAt`. (A compat view `core."deferredWorkspaceMigrationAction"` still exists — baseline SQL :1288-1298.)

| Name | Payload | Producer |
|---|---|---|
| `build_index` | `{ indexMetadataId }` | create-index when deferrable (§6.3) |
| `validate_foreignKey` | `{ fieldMetadataId }` | create-field with deferred FK validation (`get-deferred-foreign-key-validation.util.ts`) |
| `rebuild_object` | `{ objectMetadataId }` | object table rebuilds |
| `delete_logicFunctionResources` | `{ flatLogicFunction }` | logic-function delete |
| `delete_workflowRuns` | `{ coreWorkflowId, coreWorkflowVersionId? }` | workflow delete |
| `compute_recordAccessGroupIds` | `{ objectComputes[] }` | access-group permission tables |
| `compute_storedFormulaFields` | `{ objectMetadataId, formulaFieldMetadataIds, afterRecordId? }` | formula field create/update |

Types: `types/deferred-workspace-operation.type.ts:3-49`. Schema-affecting subset (gates new
migrations): `build_index`, `validate_foreignKey`, `rebuild_object`
(`constants/schema-affecting-deferred-workspace-operations.constant.ts:5-9`).

Job execution (`deferred-workspace-operation-runner.service.ts`):

* `runPendingOperations` (:121-158): all PENDING ordered by `createdAt, position`; stops on first failure.
* Claim: `UPDATE ... SET status='IN_PROGRESS', attempts=attempt, startedAt=now() WHERE id=? AND status='PENDING'`
  — a 0-row claim is treated as done by another worker (:348-368).
* Execute on a dedicated 1-connection datasource with `statement_timeout`
  (`DEFERRED_WORKSPACE_OPERATION_STATEMENT_TIMEOUT_MS`, :462-486).
* Success → delete the row **conditioned on status+attempts** (:388-392).
* Failure → back to `PENDING`, or `FAILED` when `attempt >= MAX_ATTEMPTS` (:413-437).
* Handlers: `deferred-operation-handlers/services/{build-index,validate-foreign-key,rebuild-object,delete-logic-function-resources,delete-workflow-runs}-*.service.ts`.
* Recovery cron `crons/jobs/deferred-workspace-operation-recovery.cron.job.ts` + admin commands
  `commands/retry-failed-deferred-workspace-operations.command.ts`, `rebuild-workspace-object.command.ts`,
  `flat-cache-invalidate.command.ts`; cross-workspace lookup of stuck PENDING/IN_PROGRESS rows at :182-222.

**Activation status flow.** There is no `PENDING_MIGRATION` workspace status. The gating is:
`activationStatus` lifecycle `PENDING_CREATION → ONGOING_CREATION → CREATED/ACTIVE` during which
`WorkspaceManagerService.init` runs the standard-application sync (data-model §3.1,
`workspace.service.ts:689-737`); afterwards every schema-affecting migration runs inline and only the
heavy follow-ups are deferred (§4.5), with in-flight deferrals blocking new migrations (§4.1).

## 5. Metadata-to-DDL rules per field type

Table/column names: `getWorkspaceSchemaContextForMigration`
(`srv/.../workspace-migration-runner/utils/get-workspace-schema-context-for-migration.util.ts:18-32`) =
`getWorkspaceSchemaName(workspaceId)` (data-model §1) + `computeObjectTargetTable`
(`srv/src/engine/utils/compute-object-target-table.util.ts:8-17` → `computeTableName(nameSingular, isCustom)`,
`srv/src/engine/utils/compute-table-name.util.ts:1-5`; `isCustom` ⇔ application ≠ twenty-standard).

### 5.1 Column type mapping

`fieldMetadataTypeToColumnType` (`srv/.../workspace-migration-runner/utils/field-metadata-type-to-column-type.util.ts:9-47`;
composite types never reach it — comment at :13-14):

| FieldMetadataType | Postgres |
|---|---|
| TEXT, ARRAY, (legacy RICH_TEXT via `is-text-column-type.util.ts:5-12`) | `text` |
| UUID | `uuid` |
| NUMERIC | `numeric` |
| NUMBER, POSITION | `float` (float8) |
| BOOLEAN | `boolean` |
| DATE_TIME | `timestamptz` |
| DATE | `date` |
| RATING, SELECT, MULTI_SELECT | `enum` (schema-qualified name at DDL build) |
| FILES, RAW_JSON | `jsonb` |
| TS_VECTOR | `tsvector` |
| RELATION / MORPH_RELATION / composites | not mapped; see below |
| anything else | throw `UNSUPPORTED_FIELD_METADATA_TYPE` (:42-46) |

Array flag: `ARRAY` and `MULTI_SELECT` get `type[]` (`generate-column-definitions.util.ts:171-173`;
composite properties honor `property.isArray`, :81-84).

### 5.2 Emission order of a column definition

`buildSqlColumnDefinition` (`srv/src/engine/twenty-orm/workspace-schema-manager/utils/build-sql-column-definition.util.ts:11-54`):

```
"name" TYPE[[]] [GENERATED ALWAYS AS (expr) STORED|VIRTUAL] [GENERATED ALWAYS AS IDENTITY] [PRIMARY KEY] [NOT NULL] [DEFAULT <literal>]
```

* `type === 'enum'` is replaced by `"schema"."table_column_enum"` at the call sites
  (`generate-column-definitions.util.ts:86-91,166-169`).
* The tsvector expression passes `assertSafeTsVectorExpression` at the sink (:24-33).
* `DEFAULT` omitted for tsvector and when default is undefined (:49-51).

### 5.3 Scalar fields

`generateColumnDefinition` (`generate-column-definitions.util.ts:143-177`):
`name = computeColumnName(field.name)` (identity; `compute-column-name.util.ts:20-40`),
`isNullable = field.isNullable ?? true` (:170), `isPrimary = (field.name === 'id')` (:175),
default = `serializeDefaultValue` (:156-162).

### 5.4 Composite fields (flattening)

`generateColumnDefinitions` (:199-216) → one column per composite property
(`generateCompositeColumnDefinition` :34-100) **plus** server-only composite columns
(`build-server-only-composite-column-definitions.util.ts`). Rules:

* Column name = `fieldName + pascalCase(property.name)` (`computeCompositeColumnName`,
  `compute-column-name.util.ts:42-62`). Examples in data-model §3.4.
* **Composite columns are always nullable** — comment :92-93 *"Required sub-fields are defaulted on
  read; rows are written without them"*; `isPrimary: false`.
* Default per column = `parent.defaultValue[property.name]` after `nullifyEmptyCompositeDefaultValue` (:64-79).
* Enum-typed sub-properties (only `ACTOR.source`) cast to `"<schema>"."<table>_<compositeColumn>_enum"` (:86-91).
* Relation-typed properties inside composites throw `UNSUPPORTED_COMPOSITE_COLUMN_TYPE` (:50-58).
* Name updates rename every sub-column (§5.8).

### 5.5 Relation fields

* `MORPH_RELATION` head fields (no join column) → **zero columns** (`generate-column-definitions.util.ts:190-192`).
* `RELATION`/`MORPH_RELATION` with `settings.relationType !== 'MANY_TO_ONE'` → no column (:125-127).
* MANY_TO_ONE → one column `<name>Id`, type `uuid`, nullable, no default
  (`generateRelationColumnDefinition` :120-141; join column name util cited in data-model §3.5).
* FK constraint added by the create-field handler after the column
  (`create-field-action-handler.service.ts:288-304`): `ADD CONSTRAINT <fkName> FOREIGN KEY (<col>)
  REFERENCES <schema>.<refTable> (id)` (`workspace-schema-foreign-key-manager.service.ts:26-40`);
  name = `namingStrategy.foreignKeyName(table, [col], "<schema>.<refTable>", ["id"])` (:75-90) =
  TypeORM `FK_<sha1[0:27]>` (data-model §6).
* `onDelete` mapping: `SET_NULL→SET NULL`, `CASCADE→CASCADE`, `RESTRICT→RESTRICT`, `NO_ACTION→NO ACTION`
  (`convert-on-delete-action-to-on-delete.util.ts:7-25`); **default when unset: `CASCADE`**
  (`compute-foreign-key-on-delete.util.ts:5-10`). Referenced table via `computeObjectTargetTable`
  (`build-many-to-one-foreign-key-definition.util.ts:10-33`).
* Deleting a relation field drops the column (FK goes with it) — `delete-field-action-handler.service.ts:82-95`.

### 5.6 Enum types (SELECT / MULTI_SELECT / RATING + ACTOR.source)

`ENUM_FIELD_TYPES = [SELECT, MULTI_SELECT, RATING]`
(`srv/src/engine/metadata-modules/field-metadata/types/enum-field-metadata-type.type.ts:4-11`).

* Name = `removeSqlDDLInjection(table + '_' + column + '_enum')` = `[a-zA-Z0-9_]` only
  (`compute-postgres-enum-name.util.ts:8-13`; sanitizer `remove-sql-injection.util.ts:4-6`).
* Created before the column/table: `CREATE TYPE "schema"."name" AS ENUM ('v1','v2',…)`; empty value
  list throws; values are `escapeLiteral`d (`workspace-schema-enum-manager.service.ts:29-54`).
  `escapeLiteral` doubles quotes and prefixes `E` on backslash (`remove-sql-injection.util.ts:97-123`).
* Collected per field or whole object via `collectEnumOperationsForField/Object`
  (`workspace-schema-enum-operations.util.ts:47-181`): basic enum fields (:47-96), composite
  properties of enum type (:98-150, only ACTOR.source). Batch execution `Promise.all`
  (:204-255). Operations: CREATE / DROP / RENAME (name rename follows table *and* field renames,
  options `{newTableName,newFieldName}`).
* Value changes go through `alterEnumValues` (`workspace-schema-enum-manager.service.ts:146-254`):
  rename type→`<name>_old` → create new type → rename column→`<col>_old` (63-char-safe suffix,
  :15-26) → re-add column with new enum → data-migrate with `CASE` (atomic and `enum[]` variants,
  :313-431) → drop old column and old type. Runs inside the caller's tx when one is active (:168-253).
* Since 2.56 option rows live in `core.fieldMetadataOption`; the runner computes
  from/to option-row diffs per field (`compute-field-metadata-option-enum-changes.util.ts`, invoked at
  runner :393-415 through `FieldMetadataOptionEnumSchemaService.applyEnumChangesOfFieldsWithoutFieldAction`)
  so option edits drive `ALTER TYPE` even when no field action exists.
* Defaults for enum columns are cast: `'NEW'::"schema"."table_column_enum"`
  (`serialize-default-value.util.ts:56-59`).

### 5.7 Defaults serialization

`serializeDefaultValue` (`srv/.../workspace-migration-builder/utils/serialize-default-value.util.ts:28-101`):

* `undefined|null → 'NULL'` (:35-37).
* Function defaults (`{type:'function', args:{function:'now'|'uuid'}}`) → `now()` /
  `public.uuid_generate_v4()` via `serializeFunctionDefaultValue` (:39-51; util cited in data-model §3.6).
* Strings **must already be single-quoted** in metadata (`"'NEW'"`), else `INVALID_FIELD_INPUT` (:66-74);
  quotes stripped then re-escaped with `escapeLiteral`.
* booleans/numbers → literal + `::type` cast (:75-78); Date → ISO string (:80-82);
  arrays → `ARRAY['a','b']::<type>[]` (:84-90); objects → JSON string cast (:92).
* Enum columns get the schema-qualified enum cast suffix (:56-59).

### 5.8 Field update matrix

`update-field-action-handler.service.ts` (`executeForWorkspaceSchema` :214-472):

| Changed property | DDL |
|---|---|
| `name` (:256-266, :474-567) | plain field: `RENAME COLUMN`; composite: rename every sub-column + add missing/rename server-only composite columns (:483-525, :570-627); MANY_TO_ONE relation: rename `<name>Id` (:526-541); enum fields additionally `ALTER TYPE … RENAME TO` the new enum name (:552-566) |
| `options` (enum fields) (:268-305) | value diff → `alterEnumValues` (§5.6); the column is recreated, so a new default travels with it (:286-304) |
| `defaultValue` (:311-325, :629-702) | `ALTER COLUMN … SET/DROP DEFAULT` per column (composite: per sub-column; :80-114 in the column manager). Skipped when the enum update already wrote it (:311-313) |
| `isNullable` (:327-338, :704-766) | `SET NOT NULL` with optional `UPDATE … SET col = <backfill> WHERE col IS NULL` first (:134-142); composite: forced nullable per column (:718-746); relations/tsvector skipped (:711-715) |
| `settings.onDelete` on MANY_TO_ONE (:340-415) | look up FK name → `DROP CONSTRAINT` → re-`ADD CONSTRAINT` with the new action |
| `rebuildSearchVector` + TS_VECTOR (:417-471) | `DROP COLUMN` + re-`ADD COLUMN` with the recomputed expression, then recreate the associated GIN index (:455-470) |

Metadata-row update (:182-212) persists the update minus `isUnique`/`isSearchable` (derived values,
comment :193-197). `ALTER COLUMN … DROP NOT NULL`/`SET NOT NULL` in
`workspace-schema-column-manager.service.ts:116-147`.

### 5.9 Create/delete object

* Create object (`create-object-action-handler.service.ts:124-214`): insert `objectMetadata` + all
  `fieldMetadata` rows into `core` (:99-122) → collect enum ops for enum/composite fields (:169-186) →
  enrich the table schema with enterprise columns `_isPrivate` (boolean NOT NULL default false) and
  `_accessGroupIds`/`accessGroup` extras (:188-197, utils in `srv/src/engine/permissions/`) →
  `CREATE TABLE IF NOT EXISTS "schema"."table" (<columns>)`; empty column list falls back to a bare
  `"id" uuid PRIMARY KEY DEFAULT gen_random_uuid()` (`workspace-schema-table-manager.service.ts:24-28`) →
  create the table's indexes (:206-213).
* Delete object (`delete-object-action-handler.service.ts:55-115`): delete the metadata row (:65) →
  `DROP TABLE IF EXISTS … CASCADE` (:88-93) → drop the object's enum types (:95-115).
* Update object (`update-object-action-handler.service.ts:149-214`): only a `nameSingular` change
  touches the schema — `ALTER TABLE … RENAME TO` (:180) + `ALTER TYPE … RENAME TO` for every enum
  (:200-209); everything else (labels, icon, `labelIdentifierFieldMetadataUniversalIdentifier` → id
  resolution :60-75) is a metadata-row update (:133-148).

### 5.10 Field create

`create-field-action-handler.service.ts:122-305`: skip morph heads (:247-249) → enum ops (:251-255) →
`ADD COLUMN` (single statement, multi-column; `shouldSkipExistingColumns` available for
idempotent re-runs — column manager :8-33) → FK for MANY_TO_ONE (:288-304) with optional deferred
validation (`isNotValid` → constraint added `NOT VALID` + deferred `VALIDATE CONSTRAINT`,
foreign-key manager :56-72).

## 6. Index metadata model and physical creation

### 6.1 Metadata rows

`core.indexMetadata` + `core.indexFieldMetadata` (column lists in data-model §2.13; physical schema
`rust/baseline/core-2.61.0.sql:1512+`). `isUnique`, `indexType` (`BTREE|GIN`), `indexWhereClause`,
`isSystemSideEffect`; the friendly `name` in builders is NOT the DB name —
`indexMetadata.name` **is** the physical hashed name.

### 6.2 Hashed index name

`generateDeterministicIndexName`
(`srv/src/engine/metadata-modules/index-metadata/utils/generate-deterministic-index-name.ts:16-38`):

```
sha256( tableName ‖ col1 ‖ col2 … ‖ [indexWhereClause] ).hex[0..27]
name = "IDX_" + ("UNIQUE_" if isUnique) + hex27
```

* `tableName = computeTableName(nameSingular, !belongsToTwentyStandardApp(obj))` — inside the function body (:16-38).
* `hash.update()` is called per part with **no separator** (sequential update, same body).
* The WHERE clause participates in the hash (inline comment in the same function).
* Callers: index builders; `indexWhereClause` validated against the allowlist before use.

### 6.3 Physical creation

* Column resolution `computeFlatIndexFieldColumnNames`
  (`action-handlers/index/utils/index-action-handler.utils.ts:24-113`): MANY_TO_ONE relation →
  `<name>Id` (:52-65); composite with `subFieldName` → that one sub-column (:79-97); composite
  without → **all properties flagged `isIncludedInUniqueConstraint`** (:99-108); scalar → field name.
* `CREATE [UNIQUE] INDEX [CONCURRENTLY] IF NOT EXISTS "name" ON "schema"."table" [USING type] (cols) [WHERE …]`
  (`workspace-schema-index-manager.service.ts:21-78`); allowed types BTREE/HASH/GIST/SPGIST/GIN/BRIN
  (:11-18); non-BTREE emitted as `USING`; CONCURRENTLY forbidden inside a tx (:34-39); WHERE clause
  must be in the allowlist `{"deletedAt" IS NULL, "_isPrivate"}` — anything else throws
  (`utils/validate-index-where-clause.util.ts:1-24`).
* Deferral: a **non-unique, no-where-clause, all-MANY_TO_ONE-columns** index is skipped in the tx and
  persisted as a `build_index` deferred op (`is-index-creation-deferrable.util.ts:9-45`; handler
  `create-index-action-handler.service.ts:84-89` for the skip, `:103-114` for the deferred op). Unique/partial/GIN indexes are built inline.
* Delete: `DROP INDEX IF EXISTS "schema"."name"` (:80-94). Update: drop + recreate
  (`update-index-action-handler.service.ts`; action comment §2.4).
* The standard `searchVector` GIN index is created through the same path (`searchVectorGinIndex` on
  every searchable standard object; custom objects get one built by
  `srv/src/engine/metadata-modules/object-metadata/utils/build-search-vector-gin-index-for-custom-object.util.ts`).

## 7. searchVector (tsvector)

### 7.1 What feeds it

`core.searchFieldMetadata` rows: `(objectMetadataId, fieldMetadataId, tsVectorFieldMetadataId, position)`
(baseline SQL `rust/baseline/core-2.61.0.sql:2083-2093`). The fields for an object = its
`searchFieldMetadata` rows; per standard object the set comes from
`SEARCH_FIELDS_BY_STANDARD_OBJECT_NAME`
(`srv/src/engine/workspace-manager/twenty-standard-application/constants/search-fields-by-standard-object-name.constant.ts`),
e.g. person: name, emails, phones, jobTitle (data-model §3.11). Standard-field metadata carries
`isSearchable` as a *derived* flag at build time (`create-standard-field-flat-metadata.util.ts:102`).

### 7.2 Expression

Ordering: sort by `position` then `universalIdentifier` (deterministic; comment at
`srv/src/engine/metadata-modules/flat-search-field-metadata/utils/compute-search-vector-as-expression-from-search-field-metadatas.util.ts:34-36`,
function at :37-56),
then `getTsVectorColumnExpressionFromFields`
(`srv/src/engine/workspace-manager/utils/get-ts-vector-column-expression.util.ts:20-32`):

```
to_tsvector('simple', expr1 || ' ' || expr2 …)        -- empty → to_tsvector('simple', NULL)
```

Per-column expression (:114-135): `COALESCE(public.unaccent_immutable("col"), '')`; UUID →
`COALESCE("col"::text, '')`; PHONES → `COALESCE("col", '')`; EMAILS adds
`|| ' ' || COALESCE(public.unaccent_immutable(SPLIT_PART("col",'@',2)), '')`.
Composite fields expand to searchable sub-columns only (`isSearchableSubfield.util.ts:3-23`:
TEXT-typed subfields only; RICH_TEXT → `markdown` only; PHONES → phone number + calling code) plus
type-specific extras: PHONES international formats + additionalPhones regexp/TRANSLATE blob (:61-85),
LINKS secondaryLinks blob (:87-95), EMAILS additionalEmails variants (:97-105).
Full worked example (person) in data-model §3.7. Safety gate at the DDL sink:
`assertSafeTsVectorExpression` rejects `\0 ; -- /* */ $` and unbalanced parens
(`remove-sql-injection.util.ts:19-91`).

### 7.3 Rebuild lifecycle

* Stored column: `tsvector GENERATED ALWAYS AS (…) STORED`, nullable, no default
  (`generate-column-definitions.util.ts:102-118`).
* Rebuild = drop column + re-add + recreate GIN index (§5.8 last row). Triggered by
  `rebuildSearchVector: true` update actions injected by the builder (§3.2 step 4) for objects whose
  search-field rows changed.

## 8. Standard objects and fields

### 8.1 Where definitions live

| Artifact | File |
|---|---|
| object-level universal identifiers, index/view declarations | `shared/src/metadata/constants/standard-object.constant.ts:8-1603` (`STANDARD_OBJECTS`) |
| per-field universal identifiers | `shared/src/metadata/constants/standard-object-fields.constant.ts:10-1631` (`STANDARD_OBJECT_FIELDS`; base/system fields injected via `buildStandardObjectBaseFields`/`buildStandardObjectSystemFields`) |
| object universalIdentifier constants (person `20202020-e674-48e5-a542-72570eee7213`, workspaceMember `20202020-3319-4234-a34c-82d5c0e881a6`, …) | `shared/src/metadata/constants/standard-object-universal-identifiers.constant.ts:2-46` |
| full field definitions (type, label, icon, options, defaults, settings) | `srv/src/engine/workspace-manager/twenty-standard-application/utils/field-metadata/compute-<object>-standard-flat-field-metadata.util.ts` (one file per object) |
| object builders | `.../twenty-standard-application/utils/object-metadata/build-standard-flat-object-metadata-maps.util.ts` |
| relation/morph field helpers | `.../field-metadata/create-standard-relation-field-flat-metadata.util.ts`, `create-standard-morph-relation-head-flat-metadata.util.ts` |

**Never regenerate a universalIdentifier** (comment atop both constant files). The system-field
identifiers come from `getSystemRelationFieldUniversalIdentifier(...)`
(`standard-object-fields.constant.ts:5`).

### 8.2 Conventions

From `create-standard-object-flat-metadata.util.ts:56-160`:
`isSystem` default false per object (passed per definition, :67), `isSearchable` default false (:68),
`isAuditLogged` default true (:69), `isUIEditable`/`isUICreatable` default true (:70-71),
`openRecordIn` default `USER_CHOICE` (:73), `shortcut` default null (:74), `duplicateCriteria` holds
**flattened column names** (data-model §3.4), `isLabelSyncedWithName: false` always (:118),
`targetTableName: 'DEPRECATED'` always (:131), `labelIdentifierFieldMetadataId` /
`imageIdentifierFieldMetadataId` resolved from the field name via the `STANDARD_OBJECTS[object].fields`
map (:84-96, :122-130).
From `create-standard-field-flat-metadata.util.ts:51-151`: `isNullable` default true (:64),
`isUnique` default false (:65), `isSystem` default false (:63), `isSystemSideEffect` true only for
names in `PARTIAL_SYSTEM_FLAT_FIELD_METADATAS` (:99, constant
`srv/src/engine/metadata-modules/object-metadata/constants/partial-system-flat-field-metadatas.constant.ts`),
`isAuditLogged = isAuditLoggableFieldType(type)` (:103), `isSearchable` from the object's search list (:102).

Favorites: `navigationMenuItem` rows (type `OBJECT`/`RECORD`), not a separate object — data-model §3.12;
`createOneObject` seeds the workspace-level menu item (§9.2).

### 8.3 Production seeding path (fresh workspace)

1. `activateWorkspace` flips status and calls `WorkspaceManagerService.init`
   (`srv/src/engine/core-modules/workspace/services/workspace.service.ts:689,737`; full chain in
   data-model §3.1): create schema → set `databaseSchema` → create twenty-standard application +
   workspace-custom application → `synchronizeTwentyStandardApplicationOrThrow`.
2. `TwentyStandardApplicationService.synchronizeTwentyStandardApplicationOrThrow`
   (`srv/src/engine/workspace-manager/twenty-standard-application/services/twenty-standard-application.service.ts:41-151`):
   * compute the desired maps via `computeTwentyStandardApplicationAllFlatEntityMaps` (:62-68;
     `utils/twenty-standard-application-all-flat-entity-maps.constant.ts`) — deterministic ids from
     workspaceId + universalIdentifiers;
   * build from/to per metadata name scoped to the twenty-standard application (:78-101);
   * `validateBuildAndRunWorkspaceMigrationFromTo` with **`shouldDeferActions: false`**
     (:107), `isSystemBuild: true`, `inferDeletionFromMissingEntities: true` (:108-113). So the whole
     standard-object provisioning (42 standard objects per
     `STANDARD_OBJECT_UNIVERSAL_IDENTIFIERS:3-45`, plus their fields, views, page layouts, roles,
     feature-flag driven objects) lands in **one transaction**, no deferrals;
   * set the application default role (:131-136), upsert grant role (:138-142);
   * `seedStandardObjectInitialViewsOrThrow` (:144-150, :183-249) seeds each standard object's default
     table view + view fields under the *workspace-custom* application (legacy path, :222).
3. Back in `activateWorkspace`: default feature flags, `workspaceMember` row, prefill data
   (data-model §3.1 steps 5-6).

### 8.4 Dev-seeder path

* Command `workspace:seed:dev` (`srv/src/database/commands/data-seed-dev-workspace.command.ts:16-18`,
  `--light` flag) → `DevSeederWorkspaceService` (`.../dev-seeder/services/dev-seeder.workspace-service.ts`).
* `DevSeederMetadataService` (`.../dev-seeder/metadata/services/dev-seeder-metadata.service.ts:68-270`)
  defines two seeded workspaces (APPLE `5e9d9e1d-…`, YCOMBINATOR) with custom objects
  (`rocket`, `pet`, `surveyResult`, `employmentHistory`, `petCareAgreement` — constants in
  `.../dev-seeder/metadata/custom-objects/constants/`), custom fields on person/company, morph
  relations and junction fields; every seed call goes through the normal
  `objectMetadataService`/`fieldMetadataService` with **`shouldDeferActions: false`** (:232, :263, :360, :427, :453).
* It runs **on top of** the production standard sync — it does not replace it.

## 9. Metadata API surface (object/field CRUD)

Mounted at `/metadata` (GraphQL) + REST equivalents; path enums in data-model/graphql-rest-api docs.

### 9.1 Resolvers

| Resolver | Mutations | File:lines |
|---|---|---|
| object | `createOneObject`, `deleteOneObject`, `updateOneObject`, `updateManyObjects`; queries `objects` (cursor-paged + filter), `object` | `srv/src/engine/metadata-modules/object-metadata/object-metadata.resolver.ts:372-447, 99-164` |
| field | `createOneField`, `updateOneField`, `deleteOneField`; queries `fields`, `field`, `relation`, `morphRelations` | `srv/src/engine/metadata-modules/field-metadata/field-metadata.resolver.ts:249-305, 79-165` |
| index | index CRUD + nested `indexMetadatas` on object | `srv/src/engine/metadata-modules/index-metadata/index-metadata.resolver.ts` |

All mutations are guarded by `AccessPolicyGuard(..._WRITE_ACCESS_POLICY)` (requires the
`DATA_MODEL` permission flag — `object-metadata.resolver.ts:60-79`) and resolve errors through
`objectMetadataGraphqlApiExceptionHandler` / `fieldMetadataGraphqlApiExceptionHandler`.
There are no `createManyObject`/`deleteMany*` mutations; only `updateManyObjects`.

### 9.2 Service orchestration

* `createOneObject` (`object-metadata.service.ts:519-648`): transpile input →
  `flatObjectMetadataToCreate` + default fields (`fromCreateObjectInputToFlatObjectMetadataAndFlatFieldMetadatasToCreate`, :540-544)
  → a `navigationMenuItem` favorite (:546-553, :650-727) → initial table view + view fields for
  workspace-custom objects (:559-564) → one `validateBuildAndRunWorkspaceMigration` matrix
  `objectMetadata + fieldMetadata (+view +viewField +navigationMenuItem)` (:566-618) → recompute
  cache and return the created flat object (:627-647).
* `updateOneObject`/`updateManyObjects`/`deleteOneObject` (:63-278, :281-297, :299-517): same pattern —
  transpile inputs against cached maps, emit the matrix, call validate+run (updateMany at :178,
  delete at :467).
* `createOneField` → `createManyFields` (:108-131; `createManyFields` :472-575): matrix
  `fieldMetadata` (+`index` for unique flags, +`fieldMetadataOption`), relation fields create both
  sides in one matrix; validate+run at :552.
* `updateOneField` (:265-380): transpile `UpdateFieldInput` against 9 flat maps (:285-327); the update
  payload may include `options`, `defaultValue`, `isNullable`, `settings`, label/icon; validate+run at :382.
* `deleteOneField` (:133-263): computes side effects first — indexes to delete/update (:171-180),
  field-widget page-layout deletes (:202-212), option deletes (:223-229) — then matrix + run (:214-250).

### 9.3 Input validation and name conventions

* Object names: `/^[a-z][a-zA-Z0-9]*$/`, length 1..63 (`IDENTIFIER_MIN_CHAR_LENGTH = 1` —
  `srv/src/engine/metadata-modules/utils/constants/identifier-min-char-length.constants.ts:1`;
  `IDENTIFIER_MAX_CHAR_LENGTH = 63` — `shared/src/metadata/constants/identifier-max-char-length.constant.ts:2`),
  must be stable under `camelCase`, must not be in `RESERVED_METADATA_NAME_KEYWORDS`
  (`shared/src/metadata/constants/reserved-metadata-name-keywords.constant.ts:1+`), singular ≠ plural
  (case-insensitive) — `validateFlatObjectMetadataNames`
  (`srv/src/engine/metadata-modules/flat-object-metadata/validators/utils/validate-flat-object-metadata-name.util.ts:17-89`,
  regex constant at :19-20).
* Label → name: `computeMetadataNameFromLabel` (`shared/src/metadata/utils/compute-metadata-name-from-label.util.ts:5-33`):
  prefix `n` when starting with a digit, `slugify` (transliteration, separator `_`, alnum only),
  `camelCase`, then append a suffix if the result is reserved (`addCustomSuffixIfIsReserved`).
* Uniqueness: same nameSingular/namePlural cross-matched either way against all objects in the
  workspace (`doesOtherObjectWithSameNameExists`, `validate-flat-object-metadata-name-and-labels.util.ts:20-44`);
  duplicate `universalIdentifier` rejected at build (`flat-object-metadata-validator.service.ts:195-208`).
* `labelIdentifierFieldMetadataUniversalIdentifier: null` rejected (:91-105 in the validator).
* Column names derive mechanically from field names (§5.3-5.5); clients never supply a `columnName`.

### 9.4 Who may change what

* Non-system callers cannot update a `isSystem` object except `overrides`/`isActive` (`flat-object-metadata-validator.service.ts:64-77`).
* `isSystem` objects cannot be deleted (:153-159); objects owned by the twenty-standard app cannot be
  deleted by other callers (:161-170); remote objects unsupported (:145-151, :210-216).
* Standard apps talk through the same service with `isSystemBuild: true` + their own
  `applicationUniversalIdentifier` (§8.3).

### 9.5 Error codes surfaced to clients

* `WorkspaceMigrationRunnerExceptionCode`: `INTERNAL_SERVER_ERROR`, `EXECUTION_FAILED`,
  `APPLICATION_NOT_FOUND`, `DDL_LOCKED`, `DEFERRED_WORKSPACE_MIGRATION_ACTIONS_IN_PROGRESS`
  (`srv/.../workspace-migration-runner/exceptions/workspace-migration-runner.exception.ts:11-17`).
* `WorkspaceMigrationV2ExceptionCode`: `BUILDER_INTERNAL_SERVER_ERROR`, `RUNNER_INTERNAL_SERVER_ERROR`
  (`shared/src/metadata/types/MetadataValidationError.ts:27-30`); the build failure report shape
  (`FailedMetadataValidation`, per-metadata error arrays with `code/message/userFriendlyMessage/value`)
  is at :1-25.
* Per-domain codes: `ObjectMetadataExceptionCode` (`object-metadata.exception.ts` —
  `INVALID_OBJECT_INPUT`, `OBJECT_ALREADY_EXISTS`, `OBJECT_METADATA_NOT_FOUND`, …),
  `FieldMetadataExceptionCode` (`INVALID_FIELD_INPUT`, …).
* GraphQL: `WorkspaceMigrationGraphqlApiExceptionInterceptor` maps builder/runner/flat-maps exceptions
  (`interceptors/workspace-migration-graphql-api-exception.interceptor.ts:27-66`). REST:
  `WorkspaceMigrationRunnerRestApiExceptionFilter` maps `EXECUTION_FAILED` to 409-ish retryable errors
  (`filters/workspace-migration-runner-rest-api-exception.filter.ts:24-46`).

### 9.6 Cache invalidation flow (end to end)

mutation → service matrix → validate+build+run → runner commit →
`invalidateCache` (workspace-cache flat-map keys + derived keys) →
`incrementMetadataVersion` (schema-affecting only) → GraphQL metadata responses re-served from
recomputed maps; the ORM layer (workspace datasource) re-reads `metadataVersion`
(data-model §2.3) per query context. Cache keys: `WorkspaceCacheDataMap` = all flat entity maps +
additional maps (`srv/src/engine/workspace-cache/types/workspace-cache-key.type.ts:25-45`);
`WorkspaceCacheService.getOrRecompute` / `invalidateAndRecompute`
(`srv/src/engine/workspace-cache/services/workspace-cache.service.ts:224,420`).

## 10. Rust implementation outline

### 10.1 Module layout (inside `crates/server/src`, following the existing `auth/`, `db/`, `routes/` split)

```
crates/server/src/metadata/
  mod.rs                  module wiring, re-exports
  error.rs                MetadataError enum; codes mirror §9.5 exactly
  names.rs                compute_table_name, compute_object_target_table, compute_column_name,
                          compute_composite_column_name, pascal_case, compute_metadata_name_from_label,
                          validate_*_name, RESERVED_METADATA_NAME_KEYWORDS, IDENTIFIER_* lengths
  ids.rs                  sha256 index-name hash, TypeORM FK name (sha1[0:27]) port, uuid_to_base36 (data-model §1)
  flat/                   FlatObjectMetadata, FlatFieldMetadata, FlatIndexMetadata, … structs
                          + row mappers (sqlx FromRow) for the core tables
  cache.rs                per-workspace flat-map cache (in-memory + Redis, mirrors WorkspaceCacheService)
  actions.rs              UniversalAction {metadata_name, kind, payload} + serde shapes (§2.3-2.4)
  builders/               pure: matrix/from-to → Vec<UniversalAction> + error report
    object.rs field.rs field_option.rs index.rs search_field.rs view.rs …
  order.rs                compute_ordered_migration_actions (port of §3.4)
  aggregate.rs            the four aggregation passes (§3.3)
  validate.rs             cross-entity + per-entity validators (§9.4)
  api/                    axum handlers: GraphQL /metadata resolvers + REST DTOs (§9.1)

crates/server/src/workspace_migrations/
  mod.rs                  run_workspace_migration(ws_id, actions) entry
  runner.rs               transaction loop, guards, invalidation, metadataVersion bump (§4)
  ddl/
    table.rs column.rs enum.rs index.rs foreign_key.rs   1:1 ports of the five workspace-schema managers
  column_defs.rs          generate_column_definitions + build_sql_column_definition + serialize_default_value
  tsvector.rs             search-vector expression + is_searchable_subfield
  deferred.rs             deferred-op rows, claim/execute, recovery job
crates/server/src/standard_objects/
  mod.rs                  registry: object name → builder fn (§8.1 sources transcribed to Rust consts)
  person.rs company.rs opportunity.rs note.rs task.rs workspace_member.rs …
```

### 10.2 Pure functions vs IO

Pure (unit-testable, no DB): names.rs, ids.rs hashing, column_defs.rs, tsvector.rs, actions.rs,
builders/, aggregate.rs, order.rs, validate.rs. IO: cache.rs, runner.rs, ddl/* (sqlx executors),
deferred.rs, api/. The upstream split (`builder` = pure, `runner` = QueryRunner) maps cleanly.

### 10.3 sqlx + transactions

* One `sqlx::PgConnection` (`transaction()`), `SET LOCAL lock_timeout = '8s'` first.
* All core metadata-row writes and workspace-schema DDL on that one tx (Postgres DDL is transactional,
  matching upstream's single queryRunner).
* Enum value changes ported as the rename→recreate→backfill algorithm (§5.6) — works inside the tx.
* Deferred ops: persist in the same tx, then after commit push a job (Nudge queue per ADR-007) or run
  inline when `should_defer_actions=false`.
* Index deferral rule and the CONCURRENTLY restriction (§6.3) must be preserved.
* Gating: check for PENDING/IN_PROGRESS schema-affecting deferred ops before schema-affecting runs.

### 10.4 Conventions

* No comments in code (RULES.md R-001); the "why" lives in this doc + DECISIONS.md.
* Error enum per module with `code()` returning the upstream string (auth error.rs pattern).
* sqlx migrations already own `core` (ADR-006); workspace schemas are **runtime DDL**, never sqlx
  migrations.
* Verification harness: create a workspace through the Rust server, `pg_dump --schema-only
  --schema=workspace_<b36>` and diff against a 2.61.0 reference (data-model §7 procedure).

## 11. Wave 3 implementation plan

Ordered so every unit builds and verifies independently; each keeps R-001 and reuses prior units.

| # | Unit | Contents | Acceptance criteria |
|---|---|---|---|
| U1 | `metadata::names` + `metadata::ids` | pure naming/hashing functions + constants | unit tests: enum name, table name (`_` prefix), composite names, `IDX_`/`IDX_UNIQUE_` sha256 vectors (incl. where-clause case), FK `sha1[0:27]` vectors; golden values captured from a live Twenty DB |
| U2 | `workspace_migrations::ddl` primitives | table/column/enum/index/FK managers as sqlx executors, `build_sql_column_definition`, escape helpers, `alterEnumValues` algorithm | integration test against scratch Postgres: emitted SQL byte-equal to the upstream strings quoted in §5 (assert on captured queries); enum value-change recipe round-trips data |
| U3 | `metadata::flat` + `cache.rs` | flat entity structs + core-row mappers for object/field/fieldMetadataOption/index/searchFieldMetadata; load maps per workspace | test: load maps from a baseline-migrated `core` schema; round-trip serialize/deserialize |
| U4 | `workspace_migrations::column_defs` + `tsvector.rs` | full field-type → column-definition mapping, composite flattening, default serialization, search-vector expression | unit tests per field type (§5.1 table) + composite examples from data-model §3.4 + person tsvector expression byte-equal to data-model §3.7 snapshot |
| U5 | `metadata::actions` + `builders/{object,field,index,field_option,search_field}` + `aggregate.rs` + `order.rs` | universal action model, validators (§9.4), aggregation passes, ordering | pure tests: create-object matrix → aggregated actions in §3.4 order; relation pairing; tsvector deprioritization; builder failure report shape matches `FailedMetadataValidation` |
| U6 | `workspace_migrations::runner` | transaction loop, guards, optimistic map application, metadataVersion increment, cache invalidation hooks | integration test: create object + field in one run; failure mid-run rolls back both `core` rows and workspace DDL; metadataVersion bumped only when schema-affecting |
| U7 | `standard_objects` registry + activation seeding | transcribe STANDARD_OBJECTS/FIELDS universal identifiers + per-object field definitions; port the §8.3 sync (should_defer=false) | create a workspace → `pg_dump --schema-only --schema=workspace_<b36>` identical to a 2.61.0 reference workspace for person/company/opportunity/note/task/workflowRun first, then the full set; `core` metadata row counts and universalIdentifiers match |
| U8 | Deferred operations | rows, claim/execute/retry/FAILED, build_index + validate_foreignKey + rebuild_object handlers, recovery job on Nudge queue | integration test: deferred index builds after run; failure → PENDING → retry → FAILED after max attempts; in-flight gate blocks a second schema-affecting migration |
| U9 | Metadata API (`metadata::api`) | GraphQL+REST object/field/index CRUD with validation, error codes, cache invalidation flow (§9) | contract tests: error codes equal upstream (`extensions.code`), name validation rejections, create-object seeds favorite+view, delete-field removes options/indexes/widgets; metadataVersion bump observable |
| U10 | Object/field update + delete matrix | rename fields/objects (columns + enums + table), default/nullability changes, onDelete swap, tsvector rebuild, drop table cascade | integration tests per §5.8-5.9 row; final schema after each operation diffed clean vs a reference DB where the same mutation ran on Node Twenty |

Dependency order: U1→U2→U4→U6 is the critical path; U3 feeds U5+U6; U7 needs U6; U8/U9/U10 need U6
(U9 also needs U5). U7 can start as soon as U6 lands and is the wave's primary equivalence gate.
