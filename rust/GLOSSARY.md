# Glossary

Twenty's domain terms and the names the Rust port uses for them. One term, one
name: units must use the Rust name below so the port stays consistent.

| Twenty term | Meaning | Rust name |
|---|---|---|
| workspace | A tenant. Owns a Postgres schema `workspace_<base36(uuid)>`. | `workspace`, `WorkspaceId` |
| workspace member | A user's membership in a workspace; carries the roles. | `user_workspace` table, `UserWorkspace` |
| object metadata | The definition of a record type (People, Companies, custom objects). | `object_metadata` table, `ObjectMetadata` |
| field metadata | The definition of one field on an object. | `field_metadata` table, `FieldMetadata` |
| standard object | An object owned by the built-in `twenty-standard` application. Its table is the bare singular name. | `StandardObject` |
| custom object | An object created in a workspace. Its table is `_` + singular name. | `CustomObject` |
| twenty-standard application | The built-in application id `20202020-64aa-4b6f-b003-9c74b97cee20`. | `TWENTY_STANDARD_APPLICATION_ID` |
| composite field | A field whose value is a struct (FULL_NAME, EMAILS, CURRENCY, ADDRESS, ...). Flattened to one column per sub-property. | `CompositeField`, `flattened_column_name` |
| flattened column name | `<fieldName>` + `PascalCase(subProperty)`, e.g. `name` -> `nameFirstName`. | `flattened_column_name` |
| view | A saved arrangement of fields, filters, sorts and groups for one object. | `view` table, `View` |
| view field | One column of a view. | `view_field` table, `ViewField` |
| view filter / filter group | A filter on a view, and the AND/OR/NOT group it belongs to. | `ViewFilter`, `ViewFilterGroup` |
| view sort | The ordering of a view. | `ViewSort` |
| role | A set of permissions, assignable to users, agents, API keys, webhooks or applications. | `role` table, `Role` |
| role target | The assignment of a role to exactly one principal. | `role_target` table, `RoleTarget` |
| permission flag | A named capability (27 of them) a role may hold. | `permission_flag` table, `PermissionFlag` |
| object permission | Per-role, per-object read/create/update/delete rights. | `object_permission` table |
| field permission | Per-role, per-field read/update rights. | `field_permission` table |
| row level permission | A predicate that narrows which records a role may read. | `RowLevelPermission` |
| API key | A workspace scoped key; only its metadata is stored, never the key itself. | `api_key` table, `ApiKey` |
| app token | A single use or refreshable token (refresh, password reset, invitation, SSO exchange, ...). | `app_token` table, `AppToken` |
| signing key | The current ES256 key pair that signs the server's JWTs. | `signing_key` table, `SigningKey` |
| user session | A browser session cookie backed by a hashed token. | `user_session` table, `UserSession` |
| feature flag | A boolean switch scoped to a workspace. | `feature_flag` table, `FeatureFlag` |
| key value pair | Generic per-user / per-workspace / per-application settings. | `key_value_pair` table, `KeyValuePair` |
| navigation menu item | The sidebar tree; favourites are rows of it. | `navigation_menu_item` table (wave 3) |
| workspace schema | The Postgres schema holding a workspace's records, `workspace_<base36(uuid)>`. | `workspace_schema_name` |
| core schema | The Postgres schema holding instance-wide and metadata tables. | `core` |
| find many / find one | The record queries, in GraphQL and REST. | `find_many`, `find_one` |
| group by | Grouping records by one field, with aggregates. | `group_by` |
| keyset cursor | The pagination cursor: base64url JSON plus an `id` tiebreaker. | `Cursor` |
| query hook | A pre/post hook attached to an object and method, e.g. actor stamping. | `QueryHook` |
| workspace activation | The flow that creates the workspace schema, the standard objects and the default roles. | `activate_workspace` |
| app | An installable application (manifest, logic functions, front components). | `application` table |
| workflow | An automation: a trigger, steps and actions, with versioned snapshots. | `workflow` table (wave 7) |
| BullMQ | The Node job queue the original uses. The Rust port uses its own queue (see ADR-007). | `queue` |
