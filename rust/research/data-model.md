# Twenty CRM — Data Model & Database Schema

Research doc for the Rust port (`rust-crm`). Reference source (read-only):
`C:\Users\tonminh123\Documents\crm-research\twenty` @ commit `51cff3a` (2026-11-18).

All paths below are relative to that repo. `packages/twenty-server/src/...` is abbreviated `srv/`.

## 0. Version pinning (read this first)

| Fact | Value | Source |
|---|---|---|
| Server version | **2.61.0** | `srv/src/engine/core-modules/upgrade/constants/twenty-current-version.constant.ts:13` |
| Monorepo root `package.json` version | `0.2.1` (root version is a placeholder, **not** the schema version) | `package.json:45` |
| Next version | `2.62.0` | `srv/src/engine/core-modules/upgrade/constants/twenty-next-versions.constant.ts:12` |
| Previous versions (cross-upgrade supported) | `1.21.0` … `2.60.0` | `srv/src/engine/core-modules/upgrade/constants/twenty-previous-versions.constant.ts` |

> **Trap:** `MAP.md` says "version 0.2.1 per root package.json". That is wrong for DB compatibility.
> The DB state is defined by **2.61.0** (the last `twenty-server` release). The root `package.json`
> version is bumped by `nx version:bump twenty-server` only in the server package's generated
> constants (`twenty-all-versions.constant.ts` header says "Auto-generated file").
> Pin the Rust schema to **2.61.0**.

## 1. Architecture: two Postgres schemas + per-workspace schemas

```
public            -- extensions, helper functions (unaccent_immutable)
core              -- all metadata + instance-wide tables (~80 tables)
workspace_<b36>   -- one schema per workspace; user data tables
```

* `core` schema is created by `srv/src/database/scripts/setup-db.ts:12-15`
  (`CREATE SCHEMA IF NOT EXISTS "core"`), `public` at `:8-10`.
* Extensions created by the same script: `uuid-ossp` (`:17`), `unaccent` (`:22`), `citext` (`:27`),
  and the function `public.unaccent_immutable(text)` (`:32-41`) — **`IMMUUTABLE` wrapper**, required
  because the tsvector generated columns call it in a `GENERATED ALWAYS AS` expression.
* Optional (FDW) extensions only when `IS_FDW_ENABLED=true`: `postgres_fdw`, `wrappers`, `mysql_fdw`
  plus Supabase wrappers (`:44-87`).
* The TypeORM datasource for `core` is `srv/src/database/typeorm/core/core.datasource.ts`:
  `schema: 'core'`, `synchronize: false`, `migrationsRun: false`,
  `migrationsTableName: '_typeorm_migrations'`,
  `metadataTableName: '_typeorm_generated_columns_and_materialized_views'`.
  Entities are loaded from `srv/src/engine/core-modules/**/*.entity` and
  `srv/src/engine/metadata-modules/**/*.entity` (billing entities excluded unless `IS_BILLING_ENABLED=true`).
* Per-workspace schema name = `workspace_` + base36 of the workspace UUID:
  `srv/src/engine/workspace-datasource/utils/get-workspace-schema-name.util.ts:3-5`
  → `uuidToBase36` in `packages/twenty-shared/src/utils/uuidToBase36.ts`:
  strip `-`, parse as hex BigInt, `toString(36)`. **Case matters**: base36 of a UUID yields
  lowercase digits/letters, e.g. UUID `20202020-0000-...` → `workspace_<base36>`.
* Schema creation: `srv/src/engine/workspace-datasource/workspace-schema.service.ts:34-47`
  (`queryRunner.createSchema(schemaName, true)` → `CREATE SCHEMA IF NOT EXISTS`).
  Deletion at `:49-60` (`dropSchema(name, true, true)` → CASCADE).
* `workspace.databaseSchema` (varchar, unique per workspace? no — plain nullable varchar) stores the
  schema name; `srv/src/engine/core-modules/workspace/workspace.entity.ts:284-286`.
  The `dataSource` table is **deprecated** and kept only for data preservation
  (`srv/src/engine/metadata-modules/data-source/data-source.entity.ts:12-14`).
* DDL is locked during hot upgrades via `WORKSPACE_SCHEMA_DDL_LOCKED`
  (`srv/src/engine/workspace-datasource/workspace-schema.service.ts:24-32`).

## 2. The `core` schema

### 2.1 Entity inheritance chain (column composition)

```
WorkspaceRelatedEntity      -> workspaceId uuid NOT NULL, FK workspace(id) ON DELETE CASCADE
                              srv/src/engine/workspace-manager/types/workspace-related-entity.type.ts:5-11
  +- SyncableEntity         -> universalIdentifier uuid NOT NULL,
  |                            applicationId uuid NOT NULL, FK application(id) ON DELETE CASCADE,
  |                            UNIQUE(workspaceId, universalIdentifier)
  |                            srv/src/engine/workspace-manager/types/syncable-entity.interface.ts:6-21
  |    +- OverridableEntity -> overrides jsonb NULL, isActive boolean NOT NULL DEFAULT true
  |                          srv/src/engine/workspace-manager/types/overridable-entity.type.ts:7-14
```

`@Entity('name')` without an explicit `schema` still lands in `core` because the datasource
default schema is `core`.

### 2.2 `core.user` — `srv/src/engine/core-modules/user/user.entity.ts`

| Column | Type | Null | Default | Notes |
|---|---|---|---|---|
| id | uuid | no | `uuid_generate_v4()` | PK, `@PrimaryGeneratedColumn('uuid')` |
| firstName | varchar | no | `''` | |
| lastName | varchar | no | `''` | |
| email | **citext** | no | — | `@BeforeInsert/@BeforeUpdate` lowercases it (`:50-54`) |
| isEmailVerified | boolean | no | false | |
| disabled | boolean | **yes** | false | column nullable though `@Field` says non-null |
| passwordHash | varchar | yes | — | |
| canImpersonate | boolean | no | false | |
| canAccessFullAdminPanel | boolean | no | false | |
| createdAt | timestamptz | no | now() | `@CreateDateColumn` |
| updatedAt | timestamptz | no | now() | `@UpdateDateColumn` |
| deletedAt | timestamptz | yes | — | `@DeleteDateColumn` (soft delete) |
| locale | varchar | no | `'en'` (`SOURCE_LOCALE`) | |

Indexes: `UQ_USER_EMAIL` UNIQUE on `(email)` WHERE `"deletedAt" IS NULL` (`:33-36`).
Note: `defaultAvatarUrl` existed historically (see the legacy migration `1700140427984-setupMetadataTables.ts`)
but is **no longer** on the entity.

### 2.3 `core.workspace` — `srv/src/engine/core-modules/workspace/workspace.entity.ts`

Columns (`:92-467`): `id uuid PK`; `displayName varchar NULL`; `logo varchar NULL` (deprecated);
`logoFileId uuid NULL` FK file(id) ON DELETE SET NULL; `inviteHash varchar NULL`;
`deletedAt timestamptz NULL`; `createdAt/updatedAt timestamptz`; `allowImpersonation bool default true`;
`isPublicInviteLinkEnabled bool default true`; `isCampaignClickTrackingEnabled bool default false`;
`isCampaignOpenTrackingEnabled bool default false`;
`workspaceDiscoverability enum('workspace_discoverability_enum') default 'PUBLIC'`;
`trashRetentionDays int default 14`; `eventLogRetentionDays int default 3` (`EVENT_LOG_RETENTION.defaultInDays`);
`activationStatus enum('workspace_activationStatus_enum') default 'INACTIVE'` + index `IDX_WORKSPACE_ACTIVATION_STATUS`;
`suspendedAt timestamptz NULL`; `metadataVersion int default 1` (deprecated);
`databaseSchema varchar NULL default null`; `subdomain varchar UNIQUE NOT NULL`;
`customDomain varchar UNIQUE NULL`; `isGoogleAuthEnabled`/`isGoogleAuthBypassEnabled`/
`isTwoFactorAuthenticationEnforced`/`isPasswordAuthEnabled`/`isPasswordAuthBypassEnabled`/
`isMicrosoftAuthEnabled`/`isMicrosoftAuthBypassEnabled`/`isCustomDomainEnabled` booleans;
`isInternalMessagesImportEnabled bool default false` (deprecated); `isInternalImportEnabled bool default false`;
`allowedEmbeddingOrigins varchar[] default '{}'` (was `allowedIframeOrigins`, renamed in 2.55);
`iframeMediaPermissions jsonb default []`; `editableProfileFields varchar[] default '{email,profilePicture,firstName,lastName}'`;
`defaultRoleId uuid NULL`; `aiChatModelTier varchar default 'fast'`; `aiAgentModelTier varchar default 'fast'`;
`isAutoModelSelectionEnabled bool default true`; `aiModelIdByTier jsonb default {}`;
`aiEvaluationModelId text NULL`; `aiChatDirectMessageAssistantMode varchar default 'MENTION_ONLY'`;
`aiChatInboxAssistantMode varchar default 'AUTOMATIC'`; `aiChatAssistantProfile jsonb default {...}`;
`aiAdditionalInstructions text NULL`; `aiDataPolicy jsonb default NO_AI_DATA_POLICY`;
`workspaceCustomApplicationId uuid NOT NULL` FK application(id) ON DELETE **RESTRICT** (`:456-462`).
The FK is `FK_3b1acb13a5dac9956d1a4b32755` and is **DEFERRABLE INITIALLY DEFERRED**
(`legacy-typeorm-migrations-do-not-add/common/1770050200000-makeWorkspaceAndApplicationFileFksDeferrable.ts:11`)
because `application.workspaceId → workspace.id` and `workspace.workspaceCustomApplicationId →
application.id` form a cycle.

Table CHECK constraints (`:82-89`):
* `onboarded_workspace_requires_default_role`: `"activationStatus" IN ('PENDING_CREATION','ONGOING_CREATION') OR "defaultRoleId" IS NOT NULL`
* `workspace_requires_database_schema`: `"activationStatus" IN ('PENDING_CREATION','ONGOING_CREATION') OR ("databaseSchema" IS NOT NULL AND "databaseSchema" <> '')`

`WorkspaceActivationStatus` enum values: `ONGOING_CREATION, PENDING_CREATION, CREATED, ACTIVE, INACTIVE, SUSPENDED`
(`packages/twenty-shared/src/workspace/types/WorkspaceActivationStatus.ts`).
`WorkspaceDiscoverability`: `PUBLIC, MEMBERS_AND_INVITEES, HIDDEN`
(`srv/src/engine/core-modules/workspace/types/workspace-discoverability.type.ts`).

### 2.4 `core.userWorkspace` — `srv/src/engine/core-modules/user-workspace/user-workspace.entity.ts`

`id uuid PK`; `userId uuid NOT NULL` FK user(id) CASCADE; `workspaceId uuid NOT NULL` FK workspace(id) CASCADE;
`defaultAvatarUrl varchar NULL`; `locale varchar NOT NULL default 'en'`; `createdAt/updatedAt timestamptz`;
`deletedAt timestamptz NULL`.

Indexes: `IDX_USER_WORKSPACE_USER_ID_WORKSPACE_ID_UNIQUE` UNIQUE `(userId, workspaceId)` WHERE `"deletedAt" IS NULL`;
`IDX_USER_WORKSPACE_USER_ID (userId)`; `IDX_USER_WORKSPACE_WORKSPACE_ID (workspaceId)`.

### 2.5 `core.appToken` — `srv/src/engine/core-modules/app-token/app-token.entity.ts`

`id uuid PK`; `userId uuid NULL` FK user CASCADE; `workspaceId uuid NULL` FK workspace CASCADE;
`type text NOT NULL default 'REFRESH_TOKEN'` (AppTokenType enum, `:21-33`: REFRESH_TOKEN, CODE_CHALLENGE,
AUTHORIZATION_CODE, PASSWORD_RESET_TOKEN, INVITATION_TOKEN, ONBOARDING_INVITATION_TOKEN,
EMAIL_VERIFICATION_TOKEN, ENTERPRISE_VALIDITY_TOKEN, SSO_EXCHANGE_TOKEN,
TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE, TWO_FACTOR_AUTHENTICATION_BACKUP_CODE);
`value text NULL`; `expiresAt timestamptz NOT NULL`; `deletedAt timestamptz NULL`; `revokedAt timestamptz NULL`;
`createdAt/updatedAt timestamptz`; `context jsonb NULL`.

Indexes: `IDX_APP_TOKEN_USER_ID (userId) WHERE "userId" IS NOT NULL`;
`IDX_APP_TOKEN_WORKSPACE_ID (workspaceId) WHERE "workspaceId" IS NOT NULL`;
`IDX_APP_TOKEN_TYPE_VALUE_SSO_EXCHANGE_UNIQUE` UNIQUE `(type, value)` WHERE
`"type" = 'SSO_EXCHANGE_TOKEN' AND "deletedAt" IS NULL AND "revokedAt" IS NULL`;
`IDX_APP_TOKEN_RECOVERY_CODE_PENDING_UNIQUE` UNIQUE `(userId, workspaceId)` WHERE
`"type" = 'TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE' AND "deletedAt" IS NULL AND "revokedAt" IS NULL`.

### 2.6 `core.apiKey` — `srv/src/engine/core-modules/api-key/api-key.entity.ts`

`id uuid PK`; `name varchar NOT NULL`; `expiresAt timestamptz NOT NULL`; `revokedAt timestamptz NULL`;
`createdAt/updatedAt timestamptz`; `workspaceId uuid NOT NULL` (inherited). Index `IDX_API_KEY_WORKSPACE_ID`.
**No token column** — the key value is not stored; only metadata.

### 2.7 `core.featureFlag` — `srv/src/engine/core-modules/feature-flag/feature-flag.entity.ts`

`id uuid PK`; `key text NOT NULL`; `value boolean NOT NULL`; `createdAt/updatedAt`;
`workspaceId uuid NOT NULL`. UNIQUE `IDX_FEATURE_FLAG_KEY_WORKSPACE_ID_UNIQUE (key, workspaceId)`.

### 2.8 `core.keyValuePair` — `srv/src/engine/core-modules/key-value-pair/key-value-pair.entity.ts`

`id uuid PK`; `userId uuid NULL` FK user CASCADE; `workspaceId uuid NULL` FK workspace CASCADE;
`applicationId uuid NULL` FK application CASCADE; `key text NOT NULL`; `value jsonb NULL`;
`textValueDeprecated text NULL`; `type enum NOT NULL default 'USER_VARIABLE'`
(`USER_VARIABLE, FEATURE_FLAG, CONFIG_VARIABLE, APPLICATION_VARIABLE` — note: TypeORM generates the
enum type name from the table+column, i.e. `core."keyValuePair_type_enum"`);
`createdAt/updatedAt`; `deletedAt timestamptz NULL`.

Indexes (all unique, partial):
* `IDX_KEY_VALUE_PAIR_KEY_USER_ID_WORKSPACE_ID_UNIQUE (key,userId,workspaceId)`
* `IDX_KEY_VALUE_PAIR_KEY_WORKSPACE_ID_NULL_USER_ID_UNIQUE (key,workspaceId)` WHERE `"userId" is NULL AND "applicationId" is NULL`
* `IDX_KEY_VALUE_PAIR_KEY_USER_ID_NULL_WORKSPACE_ID_UNIQUE (key,userId)` WHERE `"workspaceId" is NULL`
* `IDX_KEY_VALUE_PAIR_KEY_NULL_USER_ID_NULL_WORKSPACE_ID_UNIQUE (key)` WHERE all three NULL
* `IDX_KEY_VALUE_PAIR_KEY_APPLICATION_ID_WORKSPACE_UNIQUE (key,applicationId)` WHERE app NOT NULL AND ws NOT NULL
* `IDX_KEY_VALUE_PAIR_KEY_APPLICATION_ID_GLOBAL_UNIQUE (key,applicationId)` WHERE app NOT NULL AND ws NULL
* `IDX_KEY_VALUE_PAIR_APPLICATION_ID (applicationId)`

### 2.9 `core.application` — `srv/src/engine/core-modules/application/application.entity.ts`

`id uuid PK`; `universalIdentifier uuid NOT NULL`; `name text NOT NULL`; `description text NULL`;
`logo text NULL`; `logoFileId uuid NULL`; `version text NULL`; `sourceType text NOT NULL default 'LOCAL'`;
`state text NOT NULL default 'INSTALLED'`; `sourcePath text NOT NULL`; `packageJsonChecksum text NULL`;
`packageJsonFileId uuid NULL` FK file RESTRICT; `yarnLockChecksum text NULL`; `yarnLockFileId uuid NULL` FK file RESTRICT;
`availablePackages jsonb NOT NULL default {}`; `billing jsonb NOT NULL default {}`;
`grantedCapabilities varchar[] NOT NULL default '{}'`; `grantedIframeMediaPermissions jsonb NOT NULL default []`;
`logicFunctionLayerId uuid NULL`; `defaultRoleId uuid NULL`; `requestedRoleId uuid NULL`;
`settingsCustomTabFrontComponentId uuid NULL`; `uninstallLogicFunctionId uuid NULL`;
`uninstallHookCompletedForRequestedAt timestamptz NULL`; `healthCheckLogicFunctionId uuid NULL`;
`isUserHealthCheckEnabled bool NOT NULL default false`; `canBeUninstalled bool NOT NULL default true`;
`autoUpgrade bool NOT NULL default false`; `isSdkLayerStale bool NOT NULL default false`;
`sdkClientCoreChecksum text NULL`; `frontComponentSharedDependenciesChecksum text NULL`;
`frontComponentSharedDependenciesBuiltPath text NULL`; `applicationRegistrationId uuid NULL` FK SET NULL;
`primaryPublicDomainId uuid NULL` FK SET NULL; `createdAt/updatedAt`; `deletedAt timestamptz NULL`;
`workspaceId uuid NOT NULL`.

Indexes: `IDX_APPLICATION_WORKSPACE_ID (workspaceId)`;
`IDX_APPLICATION_UNIVERSAL_IDENTIFIER_WORKSPACE_ID_UNIQUE` UNIQUE `(universalIdentifier, workspaceId)`
WHERE `"deletedAt" IS NULL AND "universalIdentifier" IS NOT NULL`.

### 2.10 `core.objectMetadata` — `srv/src/engine/metadata-modules/object-metadata/object-metadata.entity.ts`

Extends `SyncableEntity` → has `workspaceId`, `universalIdentifier`, `applicationId`.

| Column | Type | Null | Default |
|---|---|---|---|
| id | uuid | no | gen |
| dataSourceId | uuid | yes | — (FK dropped; data preserved only, `:65-67`) |
| nameSingular | varchar | no | — |
| namePlural | varchar | no | — |
| labelSingular | varchar | no | — |
| labelPlural | varchar | no | — |
| description | text | yes | — |
| icon | varchar | yes | — |
| color | text | yes | — |
| openRecordIn | enum (`ObjectOpenRecordIn`) | no | `USER_CHOICE` |
| overrides | jsonb | yes | — |
| standardOverrides | jsonb | yes | — **declared in the entity but PHYSICALLY DROPPED in 2.20** |
| targetTableName | varchar | no | — (**deprecated**, always `'DEPRECATED'` for standard objects) |
| isCustom | boolean | no | false — **declared in the entity but PHYSICALLY DROPPED in 2.12** |
| isRemote | boolean | no | false |
| isActive | boolean | no | false |
| isSystem | boolean | no | false |
| isUIEditable | boolean | no | true |
| isUIReadOnly | boolean | no | false (superseded by isUIEditable) |
| isUICreatable | boolean | no | true |
| writability | enum (`MetadataWritability`) | no | `OPEN` |
| readability | enum (`MetadataReadability`) | no | `OPEN` |
| readabilityParentFieldUniversalIdentifiers | uuid[] | yes | — |
| sharingReach | enum (`ObjectSharingReach`) | no | `WORKSPACE` |
| disabledObjectPermissionActions | jsonb | yes | — |
| isAuditLogged | boolean | no | true |
| isSearchable | boolean | no | false |
| duplicateCriteria | jsonb | yes | — |
| shortcut | varchar | yes | — |
| labelIdentifierFieldMetadataId | uuid | yes | — (no FK, see TODO `:215`) |
| imageIdentifierFieldMetadataId | uuid | yes | — |
| isLabelSyncedWithName | boolean | no | false |
| createdAt / updatedAt | timestamptz | no | now() |

Indexes: `IDX_OBJECT_METADATA_APPLICATION_ID (applicationId)`;
UNIQUE `IDX_OBJECT_METADATA_NAME_SINGULAR_WORKSPACE_ID_UNIQUE (nameSingular, workspaceId)`;
UNIQUE `IDX_OBJECT_METADATA_NAME_PLURAL_WORKSPACE_ID_UNIQUE (namePlural, workspaceId)`.

Enum values:
* `ObjectOpenRecordIn` = SIDE_PANEL, RECORD_PAGE, USER_CHOICE (`packages/twenty-shared/src/types/ObjectOpenRecordIn.ts`)
* `MetadataWritability` = OPEN, APPLICATION, SYSTEM (`.../MetadataWritability.ts`)
* `MetadataReadability` = OPEN, PRIVATE, INHERITED, APPLICATION, SYSTEM (`.../MetadataReadability.ts`)
* `ObjectSharingReach` = ROLE_ACCESS, WORKSPACE (`.../ObjectSharingReach.ts`)

### 2.11 `core.fieldMetadata` — `srv/src/engine/metadata-modules/field-metadata/field-metadata.entity.ts`

| Column | Type | Null | Default |
|---|---|---|---|
| id | uuid | no | gen |
| objectMetadataId | uuid | no | — FK objectMetadata(id) CASCADE |
| type | varchar | no | — (FieldMetadataType as **varchar**, not enum) |
| name | varchar | no | — |
| label | varchar | no | — |
| defaultValue | jsonb | yes | — |
| description | text | yes | — |
| icon | varchar | yes | — |
| overrides | jsonb | yes | — |
| standardOverrides | jsonb | yes | — **declared in the entity but PHYSICALLY DROPPED in 2.20** |
| options | jsonb | yes | — (legacy option storage; superseded by `fieldMetadataOption` table) |
| settings | jsonb | yes | — |
| isCustom | boolean | no | false — **declared in the entity but PHYSICALLY DROPPED in 2.12** |
| isActive | boolean | no | false |
| isSystem | boolean | no | false |
| isSystemSideEffect | boolean | no | false |
| isUIEditable | boolean | no | true |
| writability | enum | no | `OPEN` |
| disabledFieldPermissionActions | jsonb | yes | — |
| isUIReadOnly | boolean | no | false |
| isNullable | boolean | yes | true |
| isSearchable | boolean | — | **not persisted** (derived) |
| isUnique | boolean | — | **not persisted** (derived from single-field UNIQUE IndexMetadata, `:186-187`) |
| isLabelSyncedWithName | boolean | no | false |
| isAuditLogged | boolean | no | true |
| relationTargetFieldMetadataId | uuid | yes | — |
| relationTargetObjectMetadataId | uuid | yes | — |
| morphId | uuid | yes | — |
| createdAt / updatedAt | timestamptz | no | now() |

CHECK `CHK_FIELD_METADATA_MORPH_RELATION_REQUIRES_MORPH_ID`:
`("type" != 'MORPH_RELATION') OR ("type" = 'MORPH_RELATION' AND "morphId" IS NOT NULL)` (`:54-57`).

Indexes: `IDX_FIELD_METADATA_APPLICATION_ID`; `IDX_FIELD_METADATA_RELATION_TARGET_FIELD_METADATA_ID`;
`IDX_FIELD_METADATA_RELATION_TARGET_OBJECT_METADATA_ID`;
UNIQUE `IDX_FIELD_METADATA_NAME_OBJECT_METADATA_ID_WORKSPACE_ID_UNIQUE (name,objectMetadataId,workspaceId)`;
`IDX_FIELD_METADATA_OBJECT_METADATA_ID_WORKSPACE_ID (objectMetadataId, workspaceId)`.

`FieldMetadataType` (varchar values) — `packages/twenty-shared/src/types/FieldMetadataType.ts`:
`ACTOR, ADDRESS, ARRAY, BOOLEAN, CURRENCY, DATE, DATE_TIME, EMAILS, FILES, FULL_NAME, LINKS,
MORPH_RELATION, MULTI_SELECT, NUMBER, NUMERIC, PHONES, POSITION, RATING, RAW_JSON, RELATION,
RICH_TEXT, SELECT, TEXT, TS_VECTOR, UUID`.

### 2.12 `core.fieldMetadataOption` — `srv/src/engine/metadata-modules/field-metadata-option/entities/field-metadata-option.entity.ts`

New in 2.56. Extends `OverridableEntity` (→ `workspaceId`, `universalIdentifier`, `applicationId`,
`overrides jsonb NULL`, `isActive bool default true`).
Columns: `id uuid PK`; `fieldMetadataId uuid NOT NULL` FK fieldMetadata CASCADE; `value varchar NOT NULL`
(the Postgres enum label); `label varchar NOT NULL`; `color varchar NULL`;
`position double precision NOT NULL default 0`; `createdAt/updatedAt`.
Indexes: `IDX_FIELD_METADATA_OPTION_APPLICATION_ID`;
UNIQUE `IDX_FIELD_METADATA_OPTION_FIELD_METADATA_ID_VALUE_UNIQUE (fieldMetadataId, value)`
**DEFERRABLE INITIALLY DEFERRED** (`:33-37`).

### 2.13 `core.indexMetadata` / `core.indexFieldMetadata`

`indexMetadata` (`srv/src/engine/metadata-modules/index-metadata/index-metadata.entity.ts`):
`id uuid PK`; `name varchar NOT NULL`; `objectMetadataId uuid NOT NULL` FK CASCADE;
`isCustom boolean NOT NULL default false` — **declared in the entity but PHYSICALLY DROPPED in 2.54**;
`isUnique boolean NOT NULL default false`;
`indexWhereClause text NULL`; `indexType enum NOT NULL default 'BTREE'` (`IndexType` = BTREE, GIN);
`isSystemSideEffect boolean NOT NULL default false`; `createdAt/updatedAt`; + Syncable columns.
UNIQUE `IDX_INDEX_METADATA_NAME_WORKSPACE_ID_OBJECT_METADATA_ID_UNIQUE (name,workspaceId,objectMetadataId)`;
`IDX_INDEX_METADATA_OBJECT_METADATA_ID_WORKSPACE_ID`; `IDX_INDEX_METADATA_APPLICATION_ID`.

`indexFieldMetadata` (`.../index-field-metadata.entity.ts`):
`id uuid PK`; `workspaceId uuid NOT NULL` FK workspace CASCADE; `indexMetadataId uuid NOT NULL` FK CASCADE;
`fieldMetadataId uuid NOT NULL` FK CASCADE; `order int NOT NULL`; `subFieldName text NULL`;
`createdAt/updatedAt`. Indexes `IDX_INDEX_FIELD_METADATA_INDEX_METADATA_ID`,
`IDX_INDEX_FIELD_METADATA_FIELD_METADATA_ID`, plus a plain `@Index()` on `workspaceId`.

### 2.14 `core.view` — `srv/src/engine/metadata-modules/view/entities/view.entity.ts`

Extends `OverridableEntity`. Columns: `id uuid PK`; `name text NOT NULL default ''`;
`objectMetadataId uuid NOT NULL` FK objectMetadata CASCADE; `type enum NOT NULL default 'TABLE'` (ViewType);
`key enum NULL default null` (ViewKey, only `INDEX`); `icon text NOT NULL`;
`position double precision NOT NULL default 0`; `isCompact boolean NOT NULL default false`;
`isCustom boolean NOT NULL default false` — **declared in the entity but PHYSICALLY DROPPED in 2.54**;
`openRecordIn enum NOT NULL default 'SIDE_PANEL'` (ViewOpenRecordIn);
`kanbanAggregateOperation enum NULL` (AggregateOperations);
`kanbanAggregateOperationFieldMetadataId uuid NULL` FK fieldMetadata CASCADE;
`calendarLayout enum NULL` (ViewCalendarLayout DAY/WEEK/MONTH);
`calendarFieldMetadataId uuid NULL` FK CASCADE; `calendarEndFieldMetadataId uuid NULL` FK **SET NULL**;
`mainGroupByFieldMetadataId uuid NULL` FK CASCADE; `shouldHideEmptyGroups bool NOT NULL default false`;
`shouldDisplayGroupsAsTabs bool NOT NULL default false`; `kanbanColumnWidth int NULL default null`;
`groupLoadLimit int NOT NULL default 8` (`DEFAULT_VIEW_GROUP_LOAD_LIMIT`);
`createdAt/updatedAt`; `deletedAt timestamptz NULL`; `anyFieldFilterValue text NULL default null`;
`visibility enum NOT NULL default 'WORKSPACE'` (ViewVisibility = WORKSPACE, UNLISTED);
`isSystemSideEffect bool NOT NULL default false`; `createdByUserWorkspaceId uuid NULL` FK SET NULL.

Indexes: `IDX_VIEW_APPLICATION_ID`; `IDX_VIEW_OBJECT_METADATA_ID_WORKSPACE_ID`;
`IDX_VIEW_VISIBILITY`; `IDX_VIEW_CALENDAR_FIELD_METADATA`; `IDX_VIEW_CALENDAR_END_FIELD_METADATA`;
`IDX_VIEW_KANBAN_FIELD_METADATA`; `IDX_VIEW_MAIN_GROUP_BY_FIELD_METADATA`;
`IDX_VIEW_CREATED_BY_USER_WORKSPACE`.
CHECK `CHK_VIEW_CALENDAR_INTEGRITY`: `("type" NOT IN ('CALENDAR','CALENDAR_WIDGET') OR ("calendarLayout" IS NOT NULL AND "calendarFieldMetadataId" IS NOT NULL))` (`:87-90`).

`ViewType` = TABLE, KANBAN, CALENDAR, LIST, FIELDS_WIDGET, TABLE_WIDGET, KANBAN_WIDGET, LIST_WIDGET,
CALENDAR_WIDGET (`packages/twenty-shared/src/types/ViewType.ts`).

### 2.15 `core.viewField` — `srv/src/engine/metadata-modules/view-field/entities/view-field.entity.ts`

Extends `OverridableEntity`. `id uuid PK`; `fieldMetadataId uuid NOT NULL` FK CASCADE;
`isVisible boolean NOT NULL default true`; `size int NOT NULL default 0`;
`position double precision NOT NULL default 0`; `aggregateOperation enum NULL`;
`viewId uuid NOT NULL` FK view CASCADE; `viewFieldGroupId uuid NULL` FK SET NULL;
`isSystemSideEffect bool NOT NULL default false`; `isCanonical bool NOT NULL default true`;
`relationTargetFieldMetadataId uuid NULL default null` FK CASCADE; `createdAt/updatedAt`; `deletedAt NULL`.

Indexes: `IDX_VIEW_FIELD_APPLICATION_ID`; `IDX_VIEW_FIELD_WORKSPACE_ID_VIEW_ID`;
`IDX_VIEW_FIELD_VIEW_ID`; `IDX_VIEW_FIELD_FIELD_METADATA_ID`; `IDX_VIEW_FIELD_VIEW_FIELD_GROUP_ID`;
`IDX_VIEW_FIELD_RELATION_TARGET_FIELD_METADATA_ID` (partial, NOT NULL);
UNIQUE `IDX_VIEW_FIELD_FIELD_METADATA_ID_VIEW_ID_UNIQUE (fieldMetadataId, viewId)`
WHERE `"deletedAt" IS NULL AND "isCanonical" = true`.

### 2.16 `core.viewFilter` — `srv/src/engine/metadata-modules/view-filter/entities/view-filter.entity.ts`

Extends `SyncableEntity` (NOT overridable). `id uuid PK`; `fieldMetadataId uuid NOT NULL` FK CASCADE;
`operand enum NOT NULL default 'CONTAINS'` (ViewFilterOperand); `value jsonb NOT NULL`;
`viewFilterGroupId uuid NULL` FK viewFilterGroup CASCADE; `positionInViewFilterGroup double precision NULL`;
`subFieldName text NULL default null`; `relationTargetFieldMetadataId uuid NULL default null` FK CASCADE;
`viewId uuid NOT NULL` FK view CASCADE; `createdAt/updatedAt`; `deletedAt NULL`.
Indexes: `IDX_VIEW_FILTER_WORKSPACE_ID_VIEW_ID`; `IDX_VIEW_FILTER_VIEW_ID`; `IDX_VIEW_FILTER_FIELD_METADATA_ID`;
`IDX_VIEW_FILTER_RELATION_TARGET_FIELD_METADATA_ID` (partial).

`ViewFilterOperand` values: IS, IS_NOT_NULL, IS_NOT, LESS_THAN_OR_EQUAL, GREATER_THAN_OR_EQUAL,
IS_BEFORE, IS_AFTER, CONTAINS, DOES_NOT_CONTAIN, IS_EMPTY, IS_NOT_EMPTY, IS_RELATIVE, IS_IN_PAST,
IS_IN_FUTURE, IS_TODAY, VECTOR_SEARCH (`packages/twenty-shared/src/types/ViewFilterOperand.ts`).

### 2.17 `core.viewSort` — `srv/src/engine/metadata-modules/view-sort/entities/view-sort.entity.ts`

Extends `SyncableEntity`. `id uuid PK`; `fieldMetadataId uuid NOT NULL` FK CASCADE;
`direction enum NOT NULL default 'ASC'` (ASC/DESC); `subFieldName varchar NULL`;
`relationTargetFieldMetadataId uuid NULL default null` FK **SET NULL**;
`viewId uuid NOT NULL` FK view CASCADE; `createdAt/updatedAt`; `deletedAt NULL`.
Indexes: `IDX_VIEW_SORT_WORKSPACE_ID_VIEW_ID`; `IDX_VIEW_SORT_VIEW_ID`; `IDX_VIEW_SORT_FIELD_METADATA_ID`;
`IDX_VIEW_SORT_RELATION_TARGET_FIELD_METADATA_ID` (partial);
UNIQUE `IDX_VIEW_SORT_FIELD_METADATA_ID_VIEW_ID_UNIQUE (fieldMetadataId, viewId)` WHERE `"deletedAt" IS NULL`.

### 2.18 `core.viewGroup`, `core.viewFilterGroup`, `core.viewFieldGroup`

* `viewGroup` (`.../view-group/entities/view-group.entity.ts`): `id`, `isVisible bool NOT NULL default true`,
  `fieldValue text NOT NULL`, `position double precision NOT NULL default 0`, `viewId uuid NOT NULL` FK CASCADE,
  `createdAt/updatedAt`, `deletedAt` + Syncable columns.
  Indexes `IDX_VIEW_GROUP_APPLICATION_ID`, `IDX_VIEW_GROUP_WORKSPACE_ID_VIEW_ID`, `IDX_VIEW_GROUP_VIEW_ID`.
* `viewFilterGroup` (`.../view-filter-group/entities/view-filter-group.entity.ts`):
  `id`, `parentViewFilterGroupId uuid NULL` FK self CASCADE, `logicalOperator enum NOT NULL default 'AND'`
  (AND/OR/NOT), `positionInViewFilterGroup double precision NULL`, `viewId uuid NOT NULL` FK CASCADE,
  `createdAt/updatedAt`, `deletedAt` + Syncable columns.
* `viewFieldGroup` (`.../view-field-group/entities/view-field-group.entity.ts`): extends `OverridableEntity`;
  `id`, `name text NOT NULL`, `position double precision NOT NULL default 0`,
  `isVisible bool NOT NULL default true`, `viewId uuid NOT NULL` FK CASCADE,
  `isSystemSideEffect bool NOT NULL default false`, `createdAt/updatedAt`, `deletedAt`.

### 2.19 `core.webhook` — `srv/src/engine/metadata-modules/webhook/entities/webhook.entity.ts`

Extends `SyncableEntity`. `id uuid PK`; `targetUrl varchar NOT NULL`;
`operations text[] NOT NULL default ARRAY['*.*']`; `description varchar NULL`;
`secret varchar NOT NULL`; `createdAt/updatedAt`; `deletedAt NULL`. Index `IDX_WEBHOOK_WORKSPACE_ID`.

### 2.20 `core.role` — `srv/src/engine/metadata-modules/role/role.entity.ts`

Extends `SyncableEntity`. `id uuid PK`; `label varchar NOT NULL`;
`canUpdateAllSettings bool NOT NULL default false`; `canAccessAllTools bool NOT NULL default false`;
`canAccessAllAgentChatChannels bool NOT NULL default false`;
`canReadAllObjectRecords bool NOT NULL default false`; `canCreateAllObjectRecords bool NOT NULL default false`;
`canUpdateAllObjectRecords bool NOT NULL default false`; `canSoftDeleteAllObjectRecords bool NOT NULL default false`;
`canDestroyAllObjectRecords bool NOT NULL default false`; `description text NULL`; `icon varchar NULL`;
`createdAt/updatedAt`; `isEditable bool NOT NULL default true`; `canBeAssignedToUsers bool NOT NULL default true`;
`canBeAssignedToAgents bool NOT NULL default true`; `canBeAssignedToApiKeys bool NOT NULL default true`;
`canBeAssignedToApplications bool NOT NULL default false`; `isSystemSideEffect bool NOT NULL default false`.
UNIQUE `IDX_ROLE_LABEL_WORKSPACE_ID_UNIQUE (label, workspaceId)`.

### 2.21 `core.roleTarget` — `srv/src/engine/metadata-modules/role-target/role-target.entity.ts`

Extends `SyncableEntity`. `id uuid PK`; `roleId uuid NOT NULL` FK role CASCADE;
`userWorkspaceId uuid NULL`; `isSystemSideEffect bool NOT NULL default false`;
`agentId uuid NULL` FK agent CASCADE; `apiKeyId uuid NULL` FK apiKey CASCADE;
`webhookId uuid NULL` FK webhook CASCADE; `targetApplicationId uuid NULL` FK application CASCADE;
`createdAt/updatedAt`.

CHECK `CHK_role_target_single_entity` (exactly one target):
`(("agentId" IS NOT NULL)::int + ("userWorkspaceId" IS NOT NULL)::int + ("apiKeyId" IS NOT NULL)::int + ("webhookId" IS NOT NULL)::int + ("targetApplicationId" IS NOT NULL)::int) = 1`.

Indexes: UNIQUE `IDX_ROLE_TARGET_UNIQUE_ASSIGNED_USER_WORKSPACE (workspaceId,userWorkspaceId)` WHERE `"isSystemSideEffect" = false`;
UNIQUE `IDX_ROLE_TARGET_UNIQUE_USER_WORKSPACE_ROLE (workspaceId,userWorkspaceId,roleId)`;
UNIQUE `IDX_ROLE_TARGET_UNIQUE_AGENT (workspaceId,agentId)`; UNIQUE `IDX_ROLE_TARGET_UNIQUE_API_KEY (workspaceId,apiKeyId)`;
UNIQUE `IDX_ROLE_TARGET_UNIQUE_WEBHOOK (workspaceId,webhookId)`;
UNIQUE `IDX_ROLE_TARGET_UNIQUE_TARGET_APPLICATION (workspaceId,targetApplicationId)`;
plus non-unique indexes on `userWorkspaceId,workspaceId`, `agentId`, `apiKeyId`, `webhookId`,
`targetApplicationId`, `roleId`.

### 2.22 Permission tables

* `core.objectPermission` (`srv/src/engine/metadata-modules/object-permission/object-permission.entity.ts`):
  `id`, `roleId uuid NOT NULL` FK CASCADE, `objectMetadataId uuid NOT NULL` FK CASCADE,
  `canReadObjectRecords bool NULL`, `canCreateObjectRecords bool NULL`, `canUpdateObjectRecords bool NULL`,
  `canSoftDeleteObjectRecords bool NULL`, `canDestroyObjectRecords bool NULL`, `createdAt/updatedAt` + Syncable.
  UNIQUE `IDX_OBJECT_PERMISSION_OBJECT_METADATA_ID_ROLE_ID_UNIQUE (objectMetadataId, roleId)`;
  `IDX_OBJECT_PERMISSION_WORKSPACE_ID_ROLE_ID`.
* `core.fieldPermission` (`.../field-permission/field-permission.entity.ts`): `id`, `roleId`,
  `objectMetadataId`, `fieldMetadataId`, `rowLevelPermissionPredicateGroupId uuid NULL`,
  `rowLevelPermissionRuleId uuid NULL`, `canReadFieldValue bool NULL`, `canUpdateFieldValue bool NULL`,
  `createdAt/updatedAt` + Syncable.
  Indexes: UNIQUE `(fieldMetadataId, roleId)` WHERE `rowLevelPermissionPredicateGroupId IS NULL`;
  UNIQUE `(fieldMetadataId, roleId, rowLevelPermissionPredicateGroupId)` WHERE NOT NULL;
  `IDX_FIELD_PERMISSION_WORKSPACE_ID_ROLE_ID`; `IDX_FIELD_PERMISSION_RULE_ID`.
* `core.permissionFlag` (`.../permission-flag/permission-flag.entity.ts`): `id`, `key varchar NOT NULL`,
  `label varchar NOT NULL`, `description text NULL`, `icon varchar NULL`,
  `permissionType varchar NOT NULL`, `createdAt/updatedAt` + Syncable.
  UNIQUE `IDX_PERMISSION_FLAG_KEY_WORKSPACE_ID_UNIQUE (key, workspaceId)`.
* `core.rolePermissionFlag` (`.../role-permission-flag/role-permission-flag.entity.ts`):
  `id`, `roleId uuid NOT NULL` FK CASCADE, `permissionFlagId uuid NOT NULL` FK CASCADE,
  `createdAt/updatedAt` + Syncable. (The legacy `flag varchar` column is declared in the entity but
  **PHYSICALLY DROPPED in 2.7** by `2-7-instance-command-fast-1779600000000-finalize-role-permission-flag-cutover.ts`.)
  UNIQUE `IDX_ROLE_PERMISSION_FLAG_PERMISSION_FLAG_ID_ROLE_ID_UNIQUE (permissionFlagId, roleId)`.
* `core.rowLevelPermissionPredicate`, `core.rowLevelPermissionPredicateGroup`, `core.rowLevelPermissionRule`
  (`srv/src/engine/metadata-modules/row-level-permission-predicate/entities/*`).

### 2.23 Other core tables (abridged: columns/types/nullability)

Full list of core entity tables (82) — from `srv/src/engine/**/**.entity.ts`:
`agent, agentChatChannel, agentChatChannelRole, apiKey, appToken, application, applicationAuthorization,
applicationRegistration, applicationRegistrationVariable, applicationRegistrationVersion, applicationTranslation,
approvedAccessDomain, billingArrangement, billingCreditGrant, billingCustomer, billingEntitlement, billingPrice,
billingProduct, billingSubscription, billingSubscriptionItem, calendarChannel, commandMenuItem, connectedAccount,
connectionProvider, connectionProviderOAuthClient, dataSource, deferredWorkspaceOperation, dpaAgreement,
emailingDomain, featureFlag, fieldMetadata, fieldMetadataOption, fieldPermission, file, frontComponent,
indexFieldMetadata, indexMetadata, keyValuePair, logicFunction, logicFunctionLayer, mcpServer, messageChannel,
messageFolder, messagingProvider, navigationMenuItem, objectMetadata, objectPermission, pageLayout,
pageLayoutTab, pageLayoutWidget, pendingWakeUp, permissionFlag, publicDomain, relatedRecordPath, role,
rolePermissionFlag, roleTarget, rowLevelPermissionPredicate, rowLevelPermissionPredicateGroup,
rowLevelPermissionRule, searchFieldMetadata, settingsMenuItem, signingKey, skill, timelineActivityType,
twoFactorAuthenticationMethod, unsubscribeTopic, upgradeMigration, usageLimit, user, userApplicationVariableValue,
userSession, userWorkspace, validationRule, view, viewField, viewFieldGroup, viewFilter, viewFilterGroup,
viewGroup, viewSort, webhook, workflow, workflowVersion, workspace, workspaceSSOIdentityProvider`.

Notable ones (column lists are complete; `NULL` = nullable, `DF` = default):

* **`core.upgradeMigration`** (`srv/src/engine/core-modules/upgrade/upgrade-migration.entity.ts`):
  `id uuid PK`; `name varchar NOT NULL`; `status varchar NOT NULL` (`'completed'|'failed'`);
  `attempt integer NOT NULL DF 1`; `executedByVersion varchar NOT NULL`; `errorMessage text NULL`;
  `isInitial boolean NOT NULL DF false`; `workspaceId uuid NULL` FK workspace CASCADE; `createdAt timestamptz`.
  UNIQUE `UQ_upgrade_migration_instance (name, attempt)` WHERE `"workspaceId" IS NULL`;
  UNIQUE `UQ_upgrade_migration_workspace (name, attempt, workspaceId)` WHERE `"workspaceId" IS NOT NULL`;
  `IDX_UPGRADE_MIGRATION_WORKSPACE_ID_NAME_ATTEMPT`; `IDX_UPGRADE_MIGRATION_WORKSPACE_ID_CREATED_AT`.
* **`core.userSession`** (`srv/src/engine/core-modules/user-session/user-session.entity.ts`):
  `id`; `tokenHash text` (UNIQUE `IDX_USER_SESSION_TOKEN_HASH_UNIQUE`); `userId uuid` FK CASCADE;
  `workspaceId uuid NULL`; `userWorkspaceId uuid NULL`; `authProvider text`; `isImpersonating bool DF false`;
  `impersonatorUserWorkspaceId`/`impersonatedUserWorkspaceId uuid NULL`; `userAgent`/`ipAddress`/`country`/
  `region`/`city text NULL`; `expiresAt timestamptz`; `lastActiveAt timestamptz`; `revokedAt timestamptz NULL`;
  `revokedReason text NULL`; `createdAt`. Explicit FK names: `FK_USER_SESSION_USER_ID`,
  `FK_USER_SESSION_WORKSPACE_ID`, `FK_USER_SESSION_USER_WORKSPACE_ID`.
* **`core.file`** (`srv/src/engine/core-modules/file/entities/file.entity.ts`): `id`; `workspaceId uuid NULL` FK CASCADE;
  `applicationId uuid` FK application **RESTRICT**; `applicationRegistrationId uuid NULL` FK CASCADE;
  `path varchar NOT NULL`; `size bigint NOT NULL`; `createdAt/updatedAt`; `deletedAt`;
  `isStaticAsset bool NOT NULL DF false`; `settings jsonb NULL`;
  `mimeType varchar NOT NULL DF 'application/octet-stream'`; `status enum NOT NULL DF 'UPLOADED'`;
  `contentSha256 varchar NULL`; `remoteOrigin jsonb NULL`.
  CHECKs `CHK_FILE_PENDING_MIME_OCTET_STREAM`, `CHK_FILE_WORKSPACE_ID_OR_APPLICATION_REGISTRATION_ID`,
  `CHK_FILE_WORKSPACE_ID_XOR_APPLICATION_REGISTRATION_ID`.
  UNIQUE `IDX_FILE_APPLICATION_ID_WORKSPACE_ID_PATH_UNIQUE (applicationId, workspaceId, path)`;
  UNIQUE `IDX_FILE_APPLICATION_REGISTRATION_ID_PATH_UNIQUE (applicationRegistrationId, path)`.
* **`core.signingKey`**: `id`; `publicKey varchar NOT NULL`; `privateKey varchar NULL` (encrypted, must match
  `enc:v2:%`); `isCurrent boolean NOT NULL DF false`; `revokedAt timestamptz NULL`; `createdAt/updatedAt`.
  Index `IDX_SIGNING_KEY_IS_CURRENT_UNIQUE (isCurrent)` WHERE `"isCurrent" = true`;
  CHECK `CHK_signingKey_privateKey_encrypted`.
* **`core.searchFieldMetadata`**: `id`; `objectMetadataId uuid NOT NULL` FK CASCADE;
  `fieldMetadataId uuid NOT NULL` FK CASCADE; `tsVectorFieldMetadataId uuid NOT NULL` FK CASCADE;
  `position float NOT NULL`; `isSystemSideEffect boolean NOT NULL DF true`; `createdAt/updatedAt` + Syncable.
  UNIQUE `IDX_SEARCH_FIELD_METADATA_FIELD_OBJECT_UNIQUE (fieldMetadataId, objectMetadataId)`.
* The remaining tables (`twoFactorAuthenticationMethod`, `approvedAccessDomain`, `publicDomain`,
  `emailingDomain`, `workspaceSSOIdentityProvider`, `unsubscribeTopic`, `agent*`, `application*`,
  `channel*`, `workflow*`, `pageLayout*`, `commandMenuItem`, `navigationMenuItem`, `frontComponent`,
  `logicFunction*`, `mcpServer`, `skill`, `validationRule`, `timelineActivityType`,
  `relatedRecordPath`, `deferredWorkspaceOperation`, `usageLimit`, `pendingWakeUp`, `billing*`) follow the
  same conventions — see their entity files.


### 2.24 Postgres type conventions in `core`

* Booleans are `boolean`, timestamps are `timestamptz`, PKs are `uuid` with `uuid_generate_v4()` default
  (the `uuid-ossp` extension, not `gen_random_uuid()`).
* TypeORM `enum` columns become **named Postgres enum types** `core."<table>_<column>_enum"`,
  e.g. `core."workspace_activationStatus_enum"`, `core."keyValuePair_type_enum"`,
  `core."twoFactorAuthenticationMethod_status_enum"`.
* `@PrimaryGeneratedColumn('uuid')` ⇒ `DEFAULT uuid_generate_v4()` (uuid-ossp).
* `@CreateDateColumn`/`@UpdateDateColumn` with `type: 'timestamptz'` ⇒ `DEFAULT now()`.
* `citext` is used only for `core.user.email`.

## 3. Per-workspace schemas (`workspace_<base36>`)

### 3.1 How a workspace schema and its tables get created

1. `WorkspaceService.activateWorkspace` (`srv/src/engine/core-modules/workspace/services/workspace.service.ts:687-789`)
   flips `activationStatus` `PENDING_CREATION → ONGOING_CREATION` (row-lock trick, `:691-703`),
   then calls `workspaceManagerService.init(...)` (`:735-738`).
2. `WorkspaceManagerService.init` (`srv/src/engine/workspace-manager/workspace-manager.service.ts:38-95`):
   1. `workspaceSchemaService.createWorkspaceDBSchema(workspaceId)` → `CREATE SCHEMA IF NOT EXISTS "workspace_<b36>"`.
   2. `workspaceRepository.update(workspaceId, { databaseSchema: schemaName })`.
   3. `applicationService.createTwentyStandardApplication({ workspaceId })`.
   4. `twentyStandardApplicationService.synchronizeTwentyStandardApplicationOrThrow({ workspaceId })`.
   5. `roleService.createWorkspaceCustomApplicationRole(...)` + `setupDefaultRoles(...)` (admin + member roles).
3. Synchronisation (`srv/src/engine/workspace-manager/twenty-standard-application/services/twenty-standard-application.service.ts:42-157`)
   computes the *desired* metadata maps for the twenty-standard application
   (`computeTwentyStandardApplicationAllFlatEntityMaps`) and diffs them against the current DB
   (`fromToAllFlatEntityMaps`), then runs `WorkspaceMigrationValidateBuildAndRunService.validateBuildAndRunWorkspaceMigrationFromTo`
   with `isSystemBuild: true`, `inferDeletionFromMissingEntities: true`.
4. The migration builder turns the diff into per-metadata actions
   (`create`/`update`/`delete` on `objectMetadata`, `fieldMetadata`, `indexMetadata`, `view*`, `role*`, …)
   and the runner executes them **twice**: once against the `core` metadata tables and once against the
   workspace schema (`executeForMetadata` vs `executeForWorkspaceSchema`).
   * `create-object-action-handler.service.ts:99-214`: inserts the `objectMetadata` row + `fieldMetadata` rows
     into `core`, then computes column definitions, creates the enum types, creates the table and its indexes.
   * `create-field-action-handler.service.ts:121-299`: `ALTER TABLE ... ADD COLUMN` + FK creation.
   * `create-index-action-handler.service.ts` → `indexManager.createIndex`.
5. Back in `activateWorkspace`: default feature flags are enabled, the `workspaceMember` row is created
   (`user-workspace.service.ts:161-216`), prefill data is inserted
   (`prefillCreatedWorkspaceRecords` → `prefill-companies/people/opportunities/dashboards/workflows`),
   and the upgrade cursor is initialised (`activateAndInitializeUpgradeState`, `:791-838`).
6. `activationStatus` becomes `ACTIVE` (has a subscription) or `CREATED` (self-hosted).

### 3.2 Table naming

* `computeTableName(nameSingular, isCustom)` → `isCustom ? '_' + nameSingular : nameSingular`
  (`srv/src/engine/utils/compute-table-name.util.ts:1-5`).
* `isCustom` = the object's `applicationUniversalIdentifier` is **not** the twenty-standard application
  (`srv/src/engine/utils/compute-object-target-table.util.ts:6-17`).
  The twenty-standard application universal identifier is
  `20202020-64aa-4b6f-b003-9c74b97cee20`
  (`packages/twenty-shared/src/application/constants/TwentyStandardApplicationUniversalIdentifier.ts:1`);
  the workspace-custom application gets a per-workspace uuid as its universal identifier
  (`srv/src/engine/core-modules/application/application.service.ts:586-597`).
* ⇒ standard objects use their bare singular name (`person`, `company`, `workspaceMember`, `note`, …);
  workspace-created custom objects are prefixed with `_` (`_myObject`).
* `objectMetadata.targetTableName` is **deprecated** and set to the literal `'DEPRECATED'` for standard
  objects (`srv/src/engine/workspace-manager/twenty-standard-application/utils/object-metadata/create-standard-object-flat-metadata.util.ts:131`).
  Never read it — derive the table name from `nameSingular` + application.

### 3.3 Column type mapping (metadata type → Postgres type)

`srv/src/engine/workspace-manager/workspace-migration/workspace-migration-runner/utils/field-metadata-type-to-column-type.util.ts:9-47`

| FieldMetadataType | Postgres column type |
|---|---|
| `TEXT`, `ARRAY`, legacy `RICH_TEXT`/`RICH_TEXT_V2` | `text` (isArray → `text[]`) |
| `UUID` | `uuid` |
| `NUMERIC` | `numeric` |
| `NUMBER`, `POSITION` | `float` (float8) |
| `BOOLEAN` | `boolean` |
| `DATE_TIME` | `timestamptz` |
| `DATE` | `date` |
| `RATING`, `SELECT`, `MULTI_SELECT` | `enum` → `"<schema>"."<table>_<column>_enum"` (MULTI_SELECT → `enum[]`) |
| `FILES`, `RAW_JSON` | `jsonb` |
| `TS_VECTOR` | `tsvector` (+ `GENERATED ALWAYS AS (...) STORED`) |
| composite types (`FULL_NAME`, `LINKS`, `CURRENCY`, `ADDRESS`, `ACTOR`, `EMAILS`, `PHONES`, `RICH_TEXT`) | flattened, never a column type |
| `RELATION`, `MORPH_RELATION` | no column; the *join* column is `uuid` |

Legacy alias handling: `isTextColumnType` also accepts the raw strings `'RICH_TEXT'`/`'RICH_TEXT_V2'`
(`.../utils/is-text-column-type.util.ts:5-12`) for workspaces that have not yet run the 1.20 migration.

### 3.4 Composite field flattening (the naming rule)

A composite field is **not** a column: it is expanded into one column per sub-property.
Column name = `<fieldName>` + `pascalCase(<subPropertyName>)`
(`srv/src/engine/metadata-modules/field-metadata/utils/compute-column-name.util.ts:42-62`;
`pascalCase = capitalize(camelCase(str))` — `packages/twenty-shared/src/utils/strings/pascalCase.ts`).

Composite definitions — `packages/twenty-shared/src/types/composite-types/*.composite-type.ts`
(registry: `composite-type-definitions.ts`):

| Composite type | Sub-properties (column suffix) | Column type per property |
|---|---|---|
| `FULL_NAME` | `firstName`, `lastName` | `text`, `text` |
| `LINKS` | `primaryLinkLabel`, `primaryLinkUrl`, `secondaryLinks` | `text`, `text`, `jsonb` |
| `CURRENCY` | `amountMicros`, `currencyCode` | `numeric`, `text` |
| `ADDRESS` | `addressStreet1`, `addressStreet2`, `addressCity`, `addressPostcode`, `addressState`, `addressCountry`, `addressLat`, `addressLng` | `text`×6, `numeric`×2 |
| `ACTOR` | `source` (SELECT enum), `workspaceMemberId` (uuid), `name` (text), `context` (jsonb) | enum, uuid, text, jsonb |
| `EMAILS` | `primaryEmail`, `additionalEmails` | `text`, `jsonb` |
| `PHONES` | `primaryPhoneNumber`, `primaryPhoneCountryCode`, `primaryPhoneCallingCode`, `additionalPhones` | `text`×3, `jsonb` |
| `RICH_TEXT` | `blocknote`, `markdown`, `tiptap` | `text`, `text`, `text` |

Worked examples (verified against the generated front-end mock snapshot
`packages/twenty-front/src/testing/mock-data/generated/metadata/objects/mock-objects-metadata.ts:13377-13384`):

* `person.name` (FULL_NAME) → `nameFirstName`, `nameLastName`
* `person.emails` (EMAILS) → `emailsPrimaryEmail`, `emailsAdditionalEmails`
* `person.phones` (PHONES) → `phonesPrimaryPhoneNumber`, `phonesPrimaryPhoneCountryCode`,
  `phonesPrimaryPhoneCallingCode`, `phonesAdditionalPhones`
* `person.linkedinLink` (LINKS) → `linkedinLinkPrimaryLinkLabel`, `linkedinLinkPrimaryLinkUrl`, `linkedinLinkSecondaryLinks`
* `company.address` (ADDRESS) → `addressAddressStreet1`, `addressAddressStreet2`, `addressAddressCity`,
  `addressAddressPostcode`, `addressAddressState`, `addressAddressCountry`, `addressAddressLat`, `addressAddressLng`
  (note the doubled `addressAddress…`: field name `address` + property `addressStreet1`)
* `opportunity.amount` (CURRENCY) → `amountAmountMicros`, `amountCurrencyCode`
* `note.bodyV2` (RICH_TEXT) → `bodyV2Blocknote`, `bodyV2Markdown`, `bodyV2Tiptap`
* `*.createdBy` / `*.updatedBy` (ACTOR) → `createdBySource`, `createdByWorkspaceMemberId`, `createdByName`, `createdByContext`

Important properties of the flattening:
* All composite sub-columns are **nullable** (`isNullable: true`) — "Required sub-fields are defaulted on
  read; rows are written without them" (`generate-column-definitions.util.ts:91-92`).
* The parent field's `isNullable` does **not** propagate to the sub-columns.
* Composite `SELECT` sub-properties (only `ACTOR.source`) create their own enum type
  `"<schema>"."<table>_<compositeColumn>_enum"`, e.g. `workspace_x.person_createdBySource_enum`
  (`workspace-schema-enum-operations.util.ts:98-150`).
* `duplicateCriteria` on `objectMetadata` stores **flattened column names**, not field names:
  person → `[["nameFirstName","nameLastName"],["linkedinLinkPrimaryLinkUrl"],["emailsPrimaryEmail"]]`
  (`create-standard-flat-object-metadata.util.ts:1329-1333`).
* Runtime read/write uses the same mapping:
  `srv/src/engine/twenty-orm/utils/format-data.util.ts:75-100` (read: composite → flattened keys),
  `srv/src/engine/twenty-orm/utils/get-field-metadata-id-to-column-names-map.util.ts:12-71` (write).

### 3.5 Relations

* A `RELATION`/`MORPH_RELATION` field has **no column of its own**. Only the owning side
  (`relationType = MANY_TO_ONE`) gets a join column named `<fieldName>Id`
  (`compute-morph-or-relation-field-join-column-name.util.ts:5-9`), type `uuid`, nullable.
* `ON DELETE` comes from `settings.onDelete` (`SET_NULL` → `SET NULL`, `CASCADE`, `RESTRICT`, `NO_ACTION`);
  default when unset is `CASCADE`
  (`.../action-handlers/field/services/utils/compute-foreign-key-on-delete.util.ts:38-42`,
  `.../workspace-migration/utils/convert-on-delete-action-to-on-delete.util.ts`).
* FK constraint name = TypeORM default naming strategy
  `namingStrategy.foreignKeyName(tableName, [columnName], '<schema>.<refTable>', ['id'])`
  (`srv/src/engine/twenty-orm/workspace-schema-manager/services/workspace-schema-foreign-key-manager.service.ts:75-88`).
  TypeORM 0.3.x default = `FK_` + sha1(`<table>_<column>`)[0:27] (node_modules not vendored → verify empirically).
* The reverse side (`ONE_TO_MANY`) has **no** FK column and no DB constraint — the FK lives on the other table.
* `MORPH_RELATION` "head" fields (a relation without a join column, e.g. the `attachments` relation on
  `person`) produce **zero** columns (`generate-column-definitions.util.ts:189-191`).

### 3.6 System columns on workspace tables

Most standard objects get these fields (from `compute-<object>-standard-flat-field-metadata.util.ts`).
The junction objects (`noteTarget`, `taskTarget`, `messageThreadTarget`, `calendarEventTarget`) only get
`id`, `createdAt`, `updatedAt`, `deletedAt`, `position`, `createdBy`, `updatedBy`, `searchVector` plus
their single parent relation:

| Field | Type | Null | Default |
|---|---|---|---|
| `id` | `uuid` | NOT NULL | `public.uuid_generate_v4()` — **PRIMARY KEY** |
| `createdAt` | `timestamptz` | NOT NULL | `now()` |
| `updatedAt` | `timestamptz` | NOT NULL | `now()` |
| `deletedAt` | `timestamptz` | NULL | — (soft delete) |
| `position` | `float` | NOT NULL | `0` |
| `createdBy` | ACTOR → 4 columns | NOT NULL | `{source:'MANUAL', name:'System', workspaceMemberId:null}` |
| `updatedBy` | ACTOR → 4 columns | NOT NULL | same |
| `searchVector` | `tsvector` | NULL | `GENERATED ALWAYS AS (to_tsvector('simple', …)) STORED` |

`PRIMARY KEY` is set because `isPrimary: flatFieldMetadata.name === 'id'`
(`generate-column-definitions.util.ts:174`).
`id`'s default comes from the function default value `'uuid'` → `public.uuid_generate_v4()`
(`serialize-function-default-value.util.ts:6-8`).
`createdAt`/`updatedAt` default `'now'` → `now()`.

### 3.7 `searchVector` (tsvector) and its GIN index

* The column is `tsvector` with `GENERATED ALWAYS AS (<expr>) STORED`
  (`generate-column-definitions.util.ts:101-117`, `build-sql-column-definition.util.ts:24-33`).
* The expression is built from the object's search fields
  (`srv/src/engine/workspace-manager/utils/get-ts-vector-column-expression.util.ts:20-135`):
  `to_tsvector('simple', <col1> || ' ' || <col2> || …)`, each column wrapped in
  `COALESCE(public.unaccent_immutable("<col>"), '')` (or `COALESCE("<col>"::text, '')` for UUID,
  `COALESCE("<col>", '')` for PHONES).
* Composite fields expand to their searchable sub-columns; `PHONES` and `LINKS` and `EMAILS` add extra
  derived expressions (international formats, `unaccent_immutable` of the JSON blob).
* Real generated expression for `person` (from the front mock snapshot
  `mock-objects-metadata.ts:13848`):
  ```
  to_tsvector('simple',
    COALESCE(public.unaccent_immutable("nameFirstName"), '') || ' ' ||
    COALESCE(public.unaccent_immutable("nameLastName"), '') || ' ' ||
    COALESCE(public.unaccent_immutable("emailsPrimaryEmail"), '') || ' ' ||
    COALESCE(public.unaccent_immutable(SPLIT_PART("emailsPrimaryEmail", '@', 2)), '') || ' ' || …
  )
  ```
* Every searchable standard object gets a GIN index named `searchVectorGinIndex`
  (physical name is the deterministic hash, see §3.9).

### 3.8 Enum types in the workspace schema

* Name = `<tableName>_<columnName>_enum`, sanitized to `[a-zA-Z0-9_]`
  (`srv/src/engine/workspace-manager/workspace-migration/utils/compute-postgres-enum-name.util.ts:8-13`).
* Created with `CREATE TYPE "<schema>"."<name>" AS ENUM (…)` **before** the table
  (`workspace-schema-enum-manager.service.ts:29-54`).
* Values = the option `value` strings of the field (`workspace-schema-enum-operations.util.ts:47-96`).
* The column type is then the schema-qualified enum name, and defaults are cast:
  `'NEW'::"workspace_x"."opportunity_stage_enum"` (`serialize-default-value.util.ts:56-59`).
* Standard option sets:
  * `opportunity.stage`: NEW(red), SCREENING(purple), MEETING(sky), PROPOSAL(turquoise), CUSTOMER(yellow);
    default `'NEW'` (`compute-opportunity-standard-flat-field-metadata.util.ts:224-265`).
  * `task.status`: TODO(0), IN_PROGRESS(1), DONE(2); default `'TODO'`.
  * `attachment.fileCategory`: ARCHIVE, AUDIO, IMAGE, PRESENTATION, SPREADSHEET, TEXT_DOCUMENT, VIDEO, OTHER;
    default `'OTHER'`.
  * `workspaceMember.dateFormat`: SYSTEM, MONTH_FIRST, DAY_FIRST, YEAR_FIRST (default SYSTEM);
    `timeFormat`: SYSTEM, HOUR_24, HOUR_12.
  * `messageParticipant.role`: FROM/TO/CC/BCC (default FROM).
  * `calendarEventParticipant.responseStatus`: NEEDS_ACTION, DECLINED, TENTATIVE, ACCEPTED.
  * `blocklist.scope`: WORKSPACE_MEMBER.
  * `workflowRun.status`: NOT_STARTED, RUNNING, COMPLETED, FAILED, ENQUEUED, STOPPING, STOPPED.
  * `calendarEvent`/`messageChannelMessageAssociation.direction`: INCOMING/OUTGOING.

### 3.9 Index naming (physical names are hashes)

`core.indexMetadata.name` and the physical Postgres index name are the **same** value, computed as
`generateDeterministicIndexName` (`srv/src/engine/metadata-modules/index-metadata/utils/generate-deterministic-index-name.ts:16-38`):

```
sha256( tableName + column1 + column2 + … [+ whereClause] )  ->  hex[0..27]
name = "IDX_" + [ "UNIQUE_" if unique ] + hex[0..27]
```

* `tableName` = the resolved workspace table name (with the `_` prefix for custom objects).
* Column names are the **physical** column names: a MANY_TO_ONE relation contributes `<name>Id`
  (`compute-morph-or-relation-field-join-column-name.util.ts`); a composite field without
  `subFieldName` expands to all sub-properties flagged `isIncludedInUniqueConstraint`
  (e.g. `emails` → `emailsPrimaryEmail`; `links` → `linksPrimaryLinkUrl`;
  `phones` → `phonesPrimaryPhoneNumber`, `phonesPrimaryPhoneCountryCode`, `phonesPrimaryPhoneCallingCode`)
  (`.../action-handlers/index/utils/index-action-handler.utils.ts:67-108`).
* The `WHERE` clause is part of the hash so a partial index can coexist with a non-partial one.
* The friendly `indexName` used in the builders (`companyIdIndex`, `emailsUniqueIndex`, …) is **not**
  the DB name — it only selects the fields. Only `indexMetadata.name` (hash) is stored.

### 3.10 Enterprise columns on record tables

Applied when the permission-table feature flag is on (`srv/src/engine/permissions/`):

| Column | Type | Notes |
|---|---|---|
| `_isPrivate` | `boolean NOT NULL DEFAULT false` | `internal/private-record/constants/is-private-column-definition.constant.ts`; partial index `IDX_PRIVATE_RECORD_<objectIdNoDashes>` on `(id) WHERE "_isPrivate"` |
| `_accessGroupIds` | `integer[]` | `internal/record-permission-repository/constants/access-group-column-names.constant.ts:2`; GIN index `IDX_ACCESS_GROUP_IDS_<objectIdNoDashes>` |
| `compactId` | `integer NOT NULL GENERATED ALWAYS AS IDENTITY` | only on the `accessGroup` object |

### 3.11 Standard objects — table + key columns

Table name = `nameSingular`. All of them also carry the system columns of §3.6.
Source: `srv/src/engine/workspace-manager/twenty-standard-application/utils/field-metadata/compute-<object>-standard-flat-field-metadata.util.ts`
and `.../utils/object-metadata/create-standard-flat-object-metadata.util.ts`.

**person** (`person`, plural `people`, label id `name`, image id `avatarFile`, shortcut `P`,
duplicateCriteria `[[nameFirstName,nameLastName],[linkedinLinkPrimaryLinkUrl],[emailsPrimaryEmail]]`):
`id`, `createdAt`, `updatedAt`, `deletedAt`, `position`, `searchVector`,
`name` FULL_NAME (nullable), `emails` EMAILS (nullable, **unique**), `linkedinLink` LINKS (nullable),
`jobTitle` TEXT (nullable), `phones` PHONES (nullable), `avatarUrl` TEXT (system, nullable, deprecated),
`avatarFile` FILES (system, nullable), `createdBy`/`updatedBy` ACTOR,
`company` RELATION→`companyId` (MANY_TO_ONE, SET NULL),
`pointOfContactForOpportunities` RELATION (ONE_TO_MANY),
`taskTargets`, `noteTargets`, `agentChatThreadTargets`, `calendarEventTargets`, `messageThreadTargets`,
`attachments`, `messageParticipants`, `calendarEventParticipants`, `timelineActivities`,
`listMemberships` — all RELATION ONE_TO_MANY.
Indexes: `companyIdIndex`, `emailsUniqueIndex` (UNIQUE on `emailsPrimaryEmail` WHERE `"deletedAt" IS NULL`),
`searchVectorGinIndex`.
Search fields: `name` (FULL_NAME), `emails` (EMAILS), `phones` (PHONES), `jobTitle` (TEXT).

**company** (`company`, plural `companies`, label id `name`, image id `domainName`, shortcut `C`,
duplicateCriteria `[[name],[domainNamePrimaryLinkUrl]]`):
`id`, `createdAt`, `updatedAt`, `deletedAt`, `position`, `searchVector`,
`name` TEXT (nullable), `domainName` LINKS (nullable, **unique**), `address` ADDRESS (nullable),
`linkedinLink` LINKS (nullable), `annualRevenue` CURRENCY (nullable),
`createdBy`/`updatedBy` ACTOR,
`people` RELATION→person (ONE_TO_MANY), `accountOwner` RELATION→workspaceMember (MANY_TO_ONE → `accountOwnerId`, SET NULL),
`taskTargets`, `noteTargets`, `agentChatThreadTargets`, `calendarEventTargets`, `messageThreadTargets`,
`opportunities`, `attachments`, `timelineActivities` (ONE_TO_MANY).
Indexes: `accountOwnerIdIndex`, `domainNameUniqueIndex` (UNIQUE on `domainNamePrimaryLinkUrl` WHERE `"deletedAt" IS NULL`),
`searchVectorGinIndex`.
Note: the historical `employees`, `xLink`, `idealCustomerProfile`, `addressOld` fields **no longer exist**.

**opportunity** (`opportunity`, plural `opportunities`, label id `name`, shortcut `O`):
`name` TEXT, `amount` CURRENCY, `closeDate` DATE_TIME, `stage` SELECT (NOT NULL, default `'NEW'`),
`pointOfContact` RELATION→person (MANY_TO_ONE → `pointOfContactId`, SET NULL),
`company` RELATION→company (MANY_TO_ONE → `companyId`, SET NULL),
`owner` RELATION→workspaceMember (MANY_TO_ONE → `ownerId`, SET NULL),
plus the standard side-effect relations. Indexes: `pointOfContactIdIndex`, `companyIdIndex`, `stageIndex`, `searchVectorGinIndex`.

**note** (`note`, plural `notes`, label id `title`, shortcut `N`):
`title` TEXT, `bodyV2` RICH_TEXT, `noteTargets` RELATION (ONE_TO_MANY),
`attachments`, `timelineActivities` (ONE_TO_MANY). Index: `searchVectorGinIndex`.

**task** (`task`, plural `tasks`, label id `title`, shortcut `T`):
`title` TEXT, `bodyV2` RICH_TEXT, `dueAt` DATE_TIME, `status` SELECT (default `'TODO'`),
`assignee` RELATION→workspaceMember (MANY_TO_ONE → `assigneeId`, SET NULL),
`taskTargets`, `attachments`, `timelineActivities`. Indexes: `assigneeIdIndex`, `searchVectorGinIndex`.

**attachment** (`attachment`, plural `attachments`, label id `name`):
`name` TEXT (system), `file` FILES (system), `fullPath` TEXT (system), `fileCategory` SELECT (system, NOT NULL,
default `'OTHER'`), and morph relations `targetTask`, `targetNote`, `targetPerson`, `targetCompany`,
`targetOpportunity`, `targetDashboard`, `targetAgentChatThread`, `targetMessage` — each MANY_TO_ONE with its
own `<name>Id` uuid column (`targetPersonId`, `targetCompanyId`, `targetOpportunityId`, `targetDashboardId`,
`targetAgentChatThreadId` are ON DELETE CASCADE; `targetTaskId`, `targetNoteId` are SET NULL).
Indexes: one per target (`taskIdIndex`, `noteIdIndex`, `personIdIndex`, `companyIdIndex`, `opportunityIdIndex`,
`dashboardIdIndex`, `agentChatThreadIdIndex`, `messageIdIndex`).

**noteTarget** (`noteTarget`): `note` RELATION→note (MANY_TO_ONE → `noteId`, CASCADE),
`targetPerson`/`targetCompany`/`targetOpportunity` morph relations with `<name>Id` columns (CASCADE).
**taskTarget** (`taskTarget`): `task` → `taskId` (CASCADE), `targetPerson`/`targetCompany`/`targetOpportunity`.
Both are junction tables with `createdBy`/`updatedBy` ACTOR + `position` + `searchVector`.

**workspaceMember** (`workspaceMember`, plural `workspaceMembers`, label id `name`, image id `avatarUrl`):
`name` FULL_NAME (NOT NULL), `colorScheme` TEXT (NOT NULL, default `'System'`), `uiScale` TEXT (NOT NULL, default `'Default'`),
`openRecordIn` TEXT (NOT NULL, default `'SIDE_PANEL'`), `locale` TEXT (NOT NULL, default `'en'`),
`avatarUrl` TEXT (nullable), `userEmail` TEXT (nullable, **unique**), `jobTitle` TEXT (nullable),
`calendarStartDay` NUMBER (NOT NULL, default 7), `userId` UUID (NOT NULL), `timeZone` TEXT (NOT NULL, default `'system'`),
`dateFormat` SELECT (NOT NULL, default `'SYSTEM'`), `timeFormat` SELECT (NOT NULL, default `'SYSTEM'`),
`numberFormat` SELECT (NOT NULL, default SYSTEM),
plus 14 ONE_TO_MANY relations (`assignedTasks`, `accountOwnerForCompanies`, `messageParticipants`, `blocklist`,
`calendarEventParticipants`, `timelineActivities`, `agentMessages`, `agentChatThreads`,
`agentChatThreadParticipants`, `agentMessageReactions`, `assignedAgentChatThreads`, `ownedOpportunities`, …).
Indexes: `userEmailUniqueIndex` (UNIQUE on `userEmail` WHERE `"deletedAt" IS NULL`), `searchVectorGinIndex`.
The `workspaceMember` row is created on activation with
`{name:{firstName,lastName}, colorScheme:'System', uiScale:'Default', openRecordIn:'SIDE_PANEL', userId, userEmail, avatarUrl, locale}`
(`srv/src/engine/core-modules/user-workspace/user-workspace.service.ts:191-203`).

**timelineActivity** (`timelineActivity`, plural `timelineActivities`, label id `linkedRecordCachedName`):
`happensAt` DATE_TIME (NOT NULL, default now), `timelineActivityTypeId` UUID (system),
`timelineActivityTypeSnapshot` RAW_JSON (system), `properties` RAW_JSON, `linkedRecordCachedName` TEXT (system),
`linkedRecordId` UUID (system), `linkedObjectMetadataId` UUID (system),
`workspaceMember` RELATION→workspaceMember (`workspaceMemberId`, CASCADE),
and morph targets `targetPerson`/`targetCompany`/`targetOpportunity`/`targetNote`/`targetTask`/
`targetWorkflowRun`/`targetDashboard`/`targetMessageList`/`targetMessageCampaign` with `<name>Id` columns.

**messageThread** (`messageThread`): `subject` TEXT, `messages` RELATION (ONE_TO_MANY),
`messageChannelMessageAssociations` RELATION (ONE_TO_MANY), `messageThreadTargets` RELATION (ONE_TO_MANY).
**message** (`message`): `headerMessageId` TEXT, `subject` TEXT, `text` TEXT, `receivedAt` DATE_TIME,
`messageThread` RELATION→messageThread (`messageThreadId`, CASCADE), `messageParticipants` (ONE_TO_MANY),
`messageChannelMessageAssociations` (ONE_TO_MANY), `messageCampaign` RELATION (`messageCampaignId`, SET NULL),
`isDraft` BOOLEAN (NOT NULL, default false), `attachments` (ONE_TO_MANY).
**messageParticipant** (`messageParticipant`): `role` SELECT (NOT NULL, default FROM), `handle` TEXT,
`displayName` TEXT, `message` RELATION (`messageId`, CASCADE), `person` RELATION (`personId`, SET NULL),
`workspaceMember` RELATION (`workspaceMemberId`, SET NULL), `messageCampaign` RELATION (`messageCampaignId`, SET NULL).
**messageThreadTarget** (`messageThreadTarget`): only `messageThread` → `messageThreadId` (CASCADE) + system columns.
**calendarEvent** (`calendarEvent`): `title` TEXT, `isCanceled` BOOLEAN (default false), `isFullDay` BOOLEAN (default false),
`startsAt`/`endsAt`/`externalCreatedAt`/`externalUpdatedAt` DATE_TIME, `description` TEXT, `location` TEXT,
`iCalUid` TEXT, `conferenceSolution` TEXT, `conferenceLink` LINKS,
`calendarChannelEventAssociations` (ONE_TO_MANY), `calendarEventParticipants` (ONE_TO_MANY),
`calendarEventTargets` (ONE_TO_MANY), `callRecordings` (ONE_TO_MANY).
**calendarEventParticipant**: `handle` TEXT, `displayName` TEXT, `isOrganizer` BOOLEAN (default false),
`responseStatus` SELECT (NOT NULL, default `'NEEDS_ACTION'`), `calendarEvent` → `calendarEventId` (CASCADE),
`person` → `personId` (SET NULL), `workspaceMember` → `workspaceMemberId`.
**dashboard** (`dashboard`): `title` TEXT, `pageLayoutId` UUID.
**workflowRun** (`workflowRun`): `name` TEXT, `enqueuedAt`/`startedAt`/`endedAt` DATE_TIME,
`status` SELECT (NOT NULL, default `'NOT_STARTED'`), `state` RAW_JSON (NOT NULL), `stepLogs` RAW_JSON (system),
`coreWorkflowId` UUID, `coreWorkflowVersionId` UUID.
**blocklist** (`blocklist`): `handle` TEXT, `scope` SELECT (system, NOT NULL, default WORKSPACE_MEMBER),
`workspaceMember` → `workspaceMemberId` (CASCADE).
**callRecording** (`callRecording`): `title` TEXT, `status` SELECT, `recordingRequestStatus` SELECT,
`applicationId` UUID, `externalBotId`/`externalRecordingId` TEXT, `startedAt`/`endedAt` DATE_TIME,
`video`/`audio` FILES, `transcript` RAW_JSON, `summary` RICH_TEXT, `calendarEvent` → `calendarEventId` (SET NULL).

Remaining standard objects (agent/AI + messaging infra): `agentChatThread`, `agentChatThreadTarget`,
`agentChatThreadParticipant`, `agentTurn`, `agentMessage`, `agentMessagePart`, `agentMessageReaction`,
`campaignDelivery`, `messageSuppression`, `messageCampaign`, `messageList`, `messageListMember`,
`messageChannelMessageAssociation`, `messageChannelMessageAssociationMessageFolder`, `shortLink`,
`accessGroup`, `recordShare`. Their field lists follow the same pattern; see the `compute-*.util.ts` files.

### 3.12 "favorite" no longer exists

Favorites are **navigation menu items** in `core.navigationMenuItem` with `type = 'RECORD'`
(`srv/src/engine/metadata-modules/navigation-menu-item/entities/navigation-menu-item.entity.ts`;
"MCP instruction: Favorites are navigation menu items. To favorite something, call
create_navigation_menu_item with scope: 'user'" — `srv/src/engine/api/mcp/utils/build-mcp-twenty-primitives-instruction-lines.util.ts:13`).
`navigationMenuItem` CHECK `CHK_navigation_menu_item_type_fields` enumerates
`FOLDER, OBJECT, VIEW, RECORD, LINK, PAGE_LAYOUT, AGENT_CHAT_CHANNEL, NATIVE_PAGE, WORKFLOW, AGENT, TAB, SECTION`.

## 4. Migration mechanism

### 4.1 Three layers

1. **Legacy TypeORM migrations (frozen)** —
   `srv/src/database/typeorm/core/legacy-typeorm-migrations-do-not-add/common/*.ts` (**182 files**) plus
   `billing/*.ts` (3 files, only loaded when `IS_BILLING_ENABLED=true`).
   Loaded by the datasource (`core.datasource.ts`, `migrations:` array) and executed by
   `dataSource.runMigrations({ transaction: 'each' })` inside `run-instance-commands`
   (`srv/src/database/commands/run-instance-commands.command.ts:170-184`).
   The bookkeeping table is `core._typeorm_migrations`; the first migration
   `1700140427984-setupMetadataTables.ts` creates `core._typeorm_generated_columns_and_materialized_views`
   and the original `core` tables.
   `srv/src/database/typeorm/core/legacy-typeorm-migrations-do-not-add/README.md` states the system is **frozen**:
   *"Do not add new files here."*
2. **Instance commands** — `srv/src/database/commands/upgrade-version-command/<version>/`.
   **263 fast** + **61 slow** files. Registered via `@RegisteredInstanceCommand(version, timestamp, {type:'slow'})`
   (`srv/src/engine/core-modules/upgrade/decorators/registered-instance-command.decorator.ts:20-33`)
   and listed in `instance-commands.constant.ts` (637 lines, auto-generated).
   * Fast: DDL only. Slow: `runDataMigration(dataSource)` first, then the same `up(queryRunner)`.
   * Each runs in **its own transaction** and records a row in `core.upgradeMigration`
     (`srv/src/engine/core-modules/upgrade/services/instance-command-runner.service.ts:31-107`).
   * Already-completed commands are skipped via `isLastAttemptCompleted` (`upgrade-migration.service.ts:33-49`).
   * `run-instance-commands` also runs the legacy TypeORM migrations first, then the whole sequence.
3. **Workspace commands** — same folder, `*-workspace-command-*.ts` (**369 files**).
   `@RegisteredWorkspaceCommand(version, timestamp)` + nest-commander `@Command`.
   They iterate every provisioned workspace (`activationStatus IN (CREATED, ACTIVE, SUSPENDED)`,
   ordered by `id ASC` — `srv/src/engine/workspace-manager/workspace-version/services/workspace-version.service.ts:27-52`,
   `packages/twenty-shared/src/workspace/constants/ProvisionedWorkspaceActivationStatuses.ts:3-8`)
   and mutate the per-workspace schema + metadata.

### 4.2 Commands to run

| Command | What it does |
|---|---|
| `node dist/database/scripts/setup-db.js` | creates `public` + `core` schemas, extensions (`uuid-ossp`, `unaccent`, `citext`), `public.unaccent_immutable` |
| `node dist/command/command.js run-instance-commands --force --include-slow` | legacy TypeORM migrations + all fast & slow instance commands (`srv/project.json:223-242`, `package.json:12`) |
| `node dist/command/command.js upgrade` | workspace commands for every provisioned workspace (`srv/src/database/commands/upgrade-version-command/upgrade.command.ts:27-31`); flags `-d/--dry-run`, `-v/--verbose`, `-w/--workspace-id` (repeatable), `--start-from-workspace-id`, `--workspace-count-limit` |

`nx database:init` = `setup-db` + `run-instance-commands --include-slow` (`srv/project.json:231-241`).

Execution order inside a version: **instance fast → instance slow → workspace commands**
(`docs/UPGRADE_COMMANDS.md`, "Execution Order"); the full sequence is built by
`UpgradeSequenceReaderService.getUpgradeSequence()` iterating
`TWENTY_CROSS_UPGRADE_SUPPORTED_VERSIONS` (`1.21.0` … `2.61.0`)
(`srv/src/engine/core-modules/upgrade/services/upgrade-sequence-reader.service.ts:39-60`).

### 4.3 What a fresh Rust server must reproduce

**Minimum viable (fresh, empty DB):**

1. `setup-db`: `CREATE SCHEMA core`, extensions `uuid-ossp`, `unaccent`, `citext`, function
   `public.unaccent_immutable(text)` (IMMUTABLE).
2. The **final** `core` schema as of 2.61.0 — i.e. the end state of the 182 legacy migrations **plus** the
   263 fast + 61 slow instance commands. Two ways to get it:
   * **(a) Recommended for byte-compatibility**: dump the schema from a real Twenty 2.61.0 instance
     (`pg_dump --schema-only --schema=core`) and ship it as the Rust baseline migration.
   * (b) Re-implement the ~324 instance commands as Rust migrations. This is the faithful path but is a lot
     of work; note that many are trivial `ALTER TABLE ... ADD COLUMN` with `IF NOT EXISTS`-style guards.
   Either way the **end state** must match: table names, column names/types/nullability/defaults,
   enum type names & values, index names (incl. the hash-based ones), FK names, CHECK constraints.
3. `core._typeorm_migrations` bookkeeping + a `core.upgradeMigration` row set consistent with "everything applied".
4. The twenty-standard application metadata rows (objectMetadata/fieldMetadata/indexMetadata/view*/role*/…)
   for each workspace — produced by the standard-metadata sync, not by migrations.
5. Per-workspace schema creation + tables for the standard objects (§3).

**Upgrade path (existing Twenty DB):** the Rust server must be able to read a DB produced by any
supported version (1.21.0+) and continue the upgrade sequence. That means implementing at least the
instance-command registry (with the same **names**, because `core.upgradeMigration.name` is the key)
and the workspace commands, or refusing to upgrade and documenting the supported floor.

Command names are the identity: they look like
`2.59.0_AddViewFieldIsCanonicalFastInstanceCommand_1794578763738`
and workspace commands `2.59.0_ExcludeDeletedRecordsFromUniqueFieldIndexesCommand_1794498818636`
(folder `upgrade-version-command/<version>/`, file `<version>-<kind>-<timestamp>-<slug>.ts`).

### 4.4 The `WasIntroducedInUpgrade` / `WasRemovedInUpgrade` / `WasRenamedInUpgrade` decorators

These mark entity properties with the upgrade command that introduced/removed/renamed them
(`srv/src/engine/core-modules/upgrade/decorators/*.decorator.ts`). They are **runtime metadata**, used to
decide whether a column may be read/written depending on how far the workspace has been upgraded
(`resolve-entity-shape-at-upgrade-cursor.util.ts`, `validate-upgrade-aware-entity-decorators.util.ts`).
A Rust port that ignores them will break on workspaces that have not yet run a given upgrade command.

## 5. Semantic traps for byte-compatibility

1. **Version pinning.** Root `package.json` says `0.2.1`; the real version is `2.61.0`
   (`twenty-current-version.constant.ts`). Pin to 2.61.0.
2. **Schema name = `workspace_` + base36(uuid)** — not the uuid with dashes.
   `uuidToBase36` strips `-`, parses the hex as a BigInt, `toString(36)`; result is **lowercase**.
   Implement with big integers (u128 does *not* fit a 128-bit UUID in 36^25 space — use a bigint/byte-array
   base conversion). `getWorkspaceSchemaName` (`workspace-datasource/utils/get-workspace-schema-name.util.ts:3-5`).
3. **`core` vs `public`.** Everything metadata is in `core`, not `public`. `public` only holds the
   `unaccent_immutable` helper.
4. **`citext` for `core.user.email`** — case-insensitive uniqueness; the app also lowercases on
   insert/update (`user.entity.ts:50-54`). A Rust port must lowercase too, or the citext index diverges.
5. **`uuid_generate_v4()` (uuid-ossp), not `gen_random_uuid()`** for `core` PKs; the workspace tables use
   `public.uuid_generate_v4()` as the `id` column default.
6. **TypeORM `enum` columns create real Postgres enum types** named `<table>_<column>_enum` in the owning
   schema. Changing a value requires `ALTER TYPE … ADD VALUE`; the workspace enum name embeds the
   *table* name (`person_stage_enum`), so renaming a table renames the enum.
7. **`fieldMetadata.type` is `varchar`, not a Postgres enum.** New types can be added without a migration.
8. **Composite fields never become a column.** They are flattened to `<field><PascalSub>` columns
   (§3.4) — and the flattened names leak into `objectMetadata.duplicateCriteria`, index column lists,
   tsvector expressions, and the GraphQL/REST response keys.
9. **`objectMetadata.targetTableName` is `'DEPRECATED'`.** Derive the table from `nameSingular`
   (+ `_` prefix for non-twenty-standard applications).
10. **Index names are hashes**, not the friendly `indexName` (§3.9). Two different metadata definitions
    that hash to the same 27 hex chars collide; the friendly names in the builders are irrelevant to the DB.
11. **FK names come from TypeORM's default naming strategy** (`FK_<sha1[0:27]>`), not from a Twenty constant.
12. **`searchVector` is a `STORED` generated column** whose expression embeds `public.unaccent_immutable(...)`
    ⇒ the function must be `IMMUTABLE` and exist before the table is created, and the expression must be
    byte-identical or Postgres will treat it as a different column definition on table rebuilds.
13. **`deletedAt` soft delete.** Every record table has it; unique indexes are partial
    (`WHERE "deletedAt" IS NULL`); reads filter on it. `viewFilter`/`viewSort`/`viewField` also have it.
14. **`isUnique` on `fieldMetadata` is not a column** — it is derived from a single-field UNIQUE
    `indexMetadata` (`field-metadata.entity.ts:186-187`). Do not add a column.
15. **`options` dual storage.** Since 2.56 options live in `core.fieldMetadataOption`
    (deferred-unique on `(fieldMetadataId, value)`), but `core.fieldMetadata.options` (jsonb) is still
    written and read as a fallback (`resolve-flat-field-metadata-options.util.ts`). Both must be kept in sync.
16. **`applicationId` / `universalIdentifier`** are on every `SyncableEntity` row; `universalIdentifier` is
    a uuid but for standard objects it is a **hard-coded constant**
    (`packages/twenty-shared/src/metadata/constants/standard-object-universal-identifiers.constant.ts`,
    e.g. person = `20202020-e674-48e5-a542-72570eee7213`, workspaceMember = `20202020-3319-4234-a34c-82d5c0e881a6`).
    Never regenerate them.
17. **`workspace.defaultRoleId` FK is not declared** on the entity (plain uuid column, `:373-374`) — there is
    no DB-level FK, but a CHECK requires it once the workspace is onboarded.
18. **`workspace.workspaceCustomApplicationId` is `NOT NULL` + `ON DELETE RESTRICT`.**
19. **Workspace `id` column is PRIMARY KEY, but `createdAt`/`updatedAt`/`deletedAt` are ordinary columns**,
    not TypeORM `@CreateDateColumn` — so the workspace tables are updated by application code, not by triggers.
20. **`position` is `float8`, not `int`.** `size` and `kanbanColumnWidth` and `groupLoadLimit` are `int`.
    `number`/`position` field types are `float` (float8); `numeric` only for CURRENCY.amountMicros and
    ADDRESS lat/lng.
21. **`DATE` columns are `date`**, and the pg driver is patched to return `date` as a **string**
    (`srv/src/database/pg/set-pg-date-type-parser.ts`, OID 1082). Rust must mirror that (no `NaiveDate`).
22. **`MULTI_SELECT` is `enum[]`**, `ARRAY` is `text[]`, `FILES`/`RAW_JSON` are `jsonb`.
23. **`ACTOR.source` is an enum** with values EMAIL, CALENDAR, WORKFLOW, AGENT, API, IMPORT, MANUAL, SYSTEM,
    WEBHOOK, APPLICATION (`packages/twenty-shared/src/types/composite-types/actor.composite-type.ts:10-21`).
24. **`core.upgradeMigration` is the upgrade ledger** — `name` + `attempt` are unique per scope; the
    `attempt` counter increments per retry; `isInitial` rows are *not* progress markers and are excluded
    from `getLastAttemptedCommandNameOrThrow` (`upgrade-migration.service.ts:186-188`).
25. **Soft-delete of workspaces**: `workspace.deletedAt` + `activationStatus`. Provisioned = CREATED, ACTIVE, SUSPENDED.
26. **Soft delete of users**: `core.user.deletedAt`, with `UQ_USER_EMAIL` partial on `"deletedAt" IS NULL`.
27. **The TypeORM entity is NOT the physical schema.** `synchronize: false` means TypeORM never validates
    the DB against the entity. Several columns declared in the entity were **physically dropped** by
    instance commands and therefore do **not** exist in a migrated DB:
    | Column | Dropped by |
    |---|---|
    | `core.objectMetadata.isCustom`, `core.fieldMetadata.isCustom` | `2-12-instance-command-fast-1780579070012-drop-is-custom-from-object-and-field-metadata.ts` |
    | `core.objectMetadata.standardOverrides`, `core.fieldMetadata.standardOverrides` | `2-20-instance-command-fast-1783511477234-drop-metadata-standard-overrides-column.ts` |
    | `core.view.isCustom`, `core.indexMetadata.isCustom`, `core.agent.isCustom`, `core.skill.isCustom`, `core.agentChatChannel.isCustom` | `2-54-instance-command-fast-1793139794914-drop-is-custom-from-agent-skill-view-index-and-agent-chat-channel.ts` |
    | `core.rolePermissionFlag.flag` | `2-7-instance-command-fast-1779600000000-finalize-role-permission-flag-cutover.ts` |
    Conversely, `core.objectMetadata.isUIReadOnly` / `core.fieldMetadata.isUIReadOnly` are **still present**
    on purpose (see the comment in `2-13-instance-command-fast-1781277453604-rename-is-ui-read-only-to-is-ui-editable.ts:19-25`:
    dropping them in the same release that stops using them breaks previous-release pods).
    ⇒ **Do not derive the Rust schema from the entities.** Dump it from a migrated 2.61.0 DB.

28. **Enum-ish `varchar` columns in `core` that carry a fixed domain**: `fieldMetadata.type`,
    `rolePermissionFlag.permissionType` (`permissionFlag.permissionType`), `workspace.locale`,
    `user.locale`, `application.sourceType`, `application.state`, `appToken.type` (text), `file.status` (enum).
29. **`workspaceMember` lives in the *workspace* schema**, but `core.userWorkspace` is the core-side
    membership row; both must exist for a workspace member.
30. **The `twenty-standard` application owns the standard objects**; the workspace "custom" application owns
    user-created objects and gets the `_` table prefix. `objectMetadata.applicationId` therefore determines
    the table name.
31. **`workspace.databaseSchema` is the source of truth** for the schema name; `core.dataSource` is
    deprecated but still populated — do not derive the schema name from `dataSource.schema`.
32. **`unaccent_immutable` must be created with the `public.` schema qualification** and be `IMMUTABLE`,
    otherwise `CREATE TABLE ... GENERATED ALWAYS AS` fails with "generation expression is not immutable".

## 6. Constraint & index naming in `core` (TypeORM hashes)

TypeORM generates constraint names by hashing, not by convention. Real names taken from
`legacy-typeorm-migrations-do-not-add/common/1700140427984-setupMetadataTables.ts:315`:

| Constraint | Name |
|---|---|
| `core.workspace` PK | `PK_ca86b6f9b3be5fe26d307d09b49` |
| `core.workspace` UNIQUE (`subdomain`) | `UQ_cba6255a24deb1fff07dd7351b8` |
| `core.workspace` UNIQUE (`customDomain`) | `UQ_900f0a3eb789159c26c8bcb39cd` |
| `core.user` PK | `PK_cace4a159ff9f2512dd42373760` |
| `core.userWorkspace` PK | `PK_222871f3641385e36e0b9f82aeb` |
| `core.keyValuePair` PK | `PK_c5a1ca828435d3eaf8f9361ed4b` |
| `core.appToken` PK | `PK_143bfe36c6284c6d3a52c94741f` |

Rule (TypeORM 0.3.x `DefaultNamingStrategy`): `PK_`/`UQ_`/`FK_`/`IDX_` + first 27 hex chars of the
sha1 of a normalized key. Named `@Index(...)`/`@Unique(...)`/`@Check(...)` decorators use the
**explicit** name instead (that is why `UQ_USER_EMAIL`, `IDX_WORKSPACE_ACTIVATION_STATUS`,
`CHK_VIEW_CALENDAR_INTEGRITY`, `CHK_role_target_single_entity` are literal).
⇒ Reproduce TypeORM's hashing exactly, or name constraints differently (works functionally, but breaks
byte-level schema diffing against a real Twenty DB).

Workspace-schema (per-workspace) tables do **not** use TypeORM naming for their indexes/FKs:
* indexes → the deterministic sha256 hash of §3.9 (`IDX_<27 hex>` / `IDX_UNIQUE_<27 hex>`);
* FKs → TypeORM's `foreignKeyName` (`FK_<sha1[0:27]>`), because the FK manager calls
  `queryRunner.connection.namingStrategy.foreignKeyName(...)` directly.

## 7. How to obtain a byte-exact baseline (recommended procedure)

1. Run the reference Twenty server at 2.61.0 against a scratch Postgres:
   `node dist/database/scripts/setup-db.js && node dist/command/command.js run-instance-commands --force --include-slow`.
2. `pg_dump --schema-only --no-owner --no-privileges --schema=core` → the `core` baseline for the Rust
   migration layer (plus `pg_dump --schema-only --schema=public` for the helper function/extensions).
3. Create one workspace through the API (sign-up) so a `workspace_<b36>` schema exists, then
   `pg_dump --schema-only --schema=workspace_<b36>` → the standard-object baseline.
4. `pg_dump --data-only --table=core.upgradeMigration` to capture the ledger rows.
5. Diff the Rust migrations against the dump until `pg_dump` output is identical.

Do **not** hand-write the `core` DDL from the entity list: §5 trap 27 shows the entities diverge from
the physical schema.

## 8. Known gaps / to verify next

* Exact TypeORM 0.3.31 `DefaultNamingStrategy` hash inputs (`node_modules` is not vendored in the
  reference checkout) — confirm against the dump produced in §7.
* `core` tables not yet enumerated column-by-column here (all 82 tables are listed in §2.23, but only
  ~25 have full column lists): `agent*`, `application*`, `calendarChannel`, `messageChannel`,
  `messagingProvider`, `connectedAccount`, `connectionProvider`, `workflow*`, `pageLayout*`,
  `commandMenuItem`, `navigationMenuItem`, `frontComponent`, `logicFunction*`, `mcpServer`, `skill`,
  `validationRule`, `timelineActivityType`, `relatedRecordPath`, `deferredWorkspaceOperation`,
  `usageLimit`, `pendingWakeUp`, `unsubscribeTopic`, `publicDomain`, `approvedAccessDomain`,
  `emailingDomain`, `workspaceSSOIdentityProvider`, `twoFactorAuthenticationMethod`, `dpaAgreement`,
  and the billing tables.
* The AI/agent standard objects (`agentChatThread`, `agentTurn`, `agentMessage`, `agentMessagePart`,
  `agentMessageReaction`, `agentChatThreadParticipant`, `campaignDelivery`, `messageSuppression`,
  `messageCampaign`, `messageList`, `messageListMember`, `shortLink`, `accessGroup`, `recordShare`)
  have per-field definitions in the `compute-*.util.ts` files but were not transcribed here.
* `srv/src/modules/**/standard-objects/*.workspace-entity.ts` are **TypeScript type classes only**
  (used by the ORM for typing); the metadata of record comes from
  `srv/src/engine/workspace-manager/twenty-standard-application/utils/**`. They still contain stale
  fields (`person.phone`, `company.addressOld`) that no longer exist in the metadata — ignore them.


