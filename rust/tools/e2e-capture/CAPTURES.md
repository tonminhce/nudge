# Captured reference fixtures

Every `*.node.json` file here is the normalized JSON body captured live from the
Twenty 2.61.0 reference server (`http://127.0.0.1:3001`, workspace `refhq`,
DB `twenty_ref`) by `tools/record-parity.py --node-only`. Values were rewritten
by the normalizers in `tools/parity-compare.py` (`normalize_json`): UUIDs to
`<uuid>`, ISO timestamps to `<iso>`, JWT-shaped strings to `<jwt>`, connection
cursor strings (keys `cursor`, `endCursor`, `startCursor`, `after`, `before`,
`starting_after`, `ending_before`) to `<cursor>`, and any `metadataVersion` key
to `<mv>`. Re-running the capture regenerates these files.

The reference upstream path prefix for all citations below is
`packages/twenty-server/src/` at tag `twenty/v2.61.0`.

## findMany-first2.node.json

Command: `python3 -I rust/tools/record-parity.py --node-only --scenario findMany-first2`

Query issued to the reference:

    query { people(first: 2) { edges { node { id name { firstName lastName } company { name } position } cursor } pageInfo { hasNextPage hasPreviousPage startCursor endCursor } totalCount } }

Wire-format contract:

- Resolver name `people` = `camelCase(namePlural)` for `findMany`:
  `engine/utils/get-resolver-name.util.ts:12-14`.
- findMany args `first,last,offset,before,after,filter,orderBy`:
  `engine/api/graphql/workspace-schema-builder/utils/get-resolver-args.util.ts:19-50`.
- Connection shape `edges { node cursor }` + `pageInfo`:
  `engine/api/graphql/workspace-schema-builder/graphql-type-generators/object-types/connection-gql-object-type.generator.ts:74-86`;
  `totalCount` on the connection:
  `engine/api/graphql/workspace-schema-builder/utils/get-available-aggregations-from-object-fields.util.ts:218`.
- `PageInfo { startCursor endCursor hasNextPage hasPreviousPage }`:
  `engine/api/graphql/workspace-schema-builder/graphql-types/object/page-into.object-type.ts:5-12`.
- Cursor payload is base64url(JSON) with the order-by leaves plus `id`:
  `engine/api/common/utils/cursors.util.ts:85-95` (encode),
  `engine/api/common/utils/cursors.util.ts:24-34` (decode). A default-order
  cursor decodes to exactly `{"id":"<uuid>"}`, which is why cursors differ per
  record and must normalize to `<cursor>`.

## findOne.node.json

Command: `python3 -I rust/tools/record-parity.py --node-only --scenario findOne`

Query issued (variables `{"id": <anchor person id from findMany-first2>}`):

    query($id: UUID!) { person(filter: { id: { eq: $id } }) { id name { firstName lastName } company { name } createdAt updatedAt } }

Wire-format contract:

- Resolver name `person` = `camelCase(nameSingular)` for `findOne`:
  `engine/utils/get-resolver-name.util.ts:15-16`.
- findOne takes a single required `filter` arg (no `id` arg):
  `engine/api/graphql/workspace-schema-builder/utils/get-resolver-args.util.ts:51-58`.
- Composite `FULL_NAME` reads back as `{ firstName lastName }` object
  (`company` M2O read as relation object): flattening contract in
  `research/workspace-migrations.md §5.4` mirrored by `research/query-layer.md §3.10`.
- A missing id is `RECORD_NOT_FOUND` mapped to `code: NOT_FOUND` with
  `subCode: RECORD_NOT_FOUND`:
  `engine/api/common/common-query-runners/utils/common-query-runner-to-graphql-api-exception-handler.util.ts:18-19`
  over `engine/api/common/common-query-runners/errors/common-query-runner.exception.ts:7`.

## company-filter.node.json

Command: `python3 -I rust/tools/record-parity.py --node-only --scenario company-filter`

Query issued (variables `{"name": <first company name>}`):

    query($name: String) { companies(filter: { name: { eq: $name } }) { edges { node { id name } } totalCount } }

Wire-format contract:

- Filter arg kind on findMany:
  `engine/api/graphql/workspace-schema-builder/utils/get-resolver-args.util.ts:41-44`.
- Operator set incl. `eq` on the record DSL
  (`TSFilter`/`StringFilter`): `research/query-layer.md §3.1` citing
  `engine/api/common/common-query-runners/common-find-query/utils/compute-where-condition-parts.ts`.
- Unknown sub-field on a composite filter type is `INVALID_ARGS_FILTER` mapped
  to `code: BAD_USER_INPUT`:
  `engine/api/common/common-query-runners/utils/common-query-runner-to-graphql-api-exception-handler.util.ts:25,36`.

## orderBy-pagination.node.json

Command: `python3 -I rust/tools/record-parity.py --node-only --scenario orderBy-pagination`

Queries issued:

    query { people(first: 2, orderBy: [{ name: { lastName: DescNullsLast } }]) { edges { node { name { lastName } } cursor } pageInfo { hasNextPage hasPreviousPage startCursor endCursor } totalCount } }
    query($c: String) { people(first: 2, after: $c, orderBy: [{ name: { lastName: DescNullsLast } }]) { ... same shape ... } }

Wire-format contract:

- `orderBy` is an array input on findMany:
  `engine/api/graphql/workspace-schema-builder/utils/get-resolver-args.util.ts:45-49`.
- Directions are the nulls-qualified enum (`AscNullsFirst`, `AscNullsLast`,
  `DescNullsFirst`, `DescNullsLast`); a bare `Asc` is rejected
  ("requires a direction value"): `engine/api/utils/resolve-order-by-leaves.utils.ts:300,325`;
  direction enum in `twenty-shared` `ObjectRecordGroupBy.ts:4-9`. Composite
  fields require subfield ordering: `resolve-order-by-leaves.utils.ts:300`.
- Cursor continuation carries the order-leaf values (`name.lastName`) plus `id`
  inside the base64url payload: `engine/api/common/utils/cursors.util.ts:55-90`,
  which is the coupling documented in `research/query-layer.md §13` risk 1.
  The fixture normalizes every cursor to `<cursor>`; parity of pagination is
  asserted on page contents + `hasPreviousPage`, not on cursor bytes.

## mutation-roundtrip.node.json

Command: `python3 -I rust/tools/record-parity.py --node-only --scenario mutation-roundtrip`

Sequence issued (each a separate POST to `/graphql`; the created id is threaded
through subsequent calls; the record is destroyed at the end so the reference DB
returns to its seeded state):

    mutation { createPerson(data: { name: { firstName: "Parity", lastName: "Temp" } }) { id name { firstName } position createdAt } }
    query  ($id) { person(filter: { id: { eq: $id } }) { id deletedAt } }
    mutation ($id) { updatePerson(id: $id, data: { jobTitle: "Harness" }) { id jobTitle updatedAt } }
    mutation ($id) { deletePerson(id: $id) { id deletedAt } }          # soft delete
    query  ($id) { person(filter: { id: { eq: $id } }) { id deletedAt } }   # expects NOT_FOUND/RECORD_NOT_FOUND
    mutation ($id) { restorePerson(id: $id) { id deletedAt } }
    mutation ($id) { destroyPerson(id: $id) { id } }
    query  ($id) { person(filter: { id: { eq: $id } }) { id } }       # expects NOT_FOUND/RECORD_NOT_FOUND

Wire-format contract:

- Mutation resolver names `create<Person>`/`update<Person>`/`delete<Person>`/
  `restore<Person>`/`destroy<Person>`:
  `engine/utils/get-resolver-name.util.ts:20-43`.
- createOne arg is `data` (required):
  `engine/api/graphql/workspace-schema-builder/utils/get-resolver-args.util.ts:78-92`;
  updateOne/deleteOne/destroyOne take `id` + (`data` for update):
  `get-resolver-args.util.ts:93-100,118-129`.
- Soft delete sets `deletedAt`; subsequent `findOne` hides the record and
  throws `RECORD_NOT_FOUND` → `code: NOT_FOUND`:
  `engine/api/common/common-query-runners/utils/common-query-runner-to-graphql-api-exception-handler.util.ts:18-19`,
  `research/query-layer.md §4.1`. The captured `position: 0` on create is the
  `undefined → treated as first → min-1` backfill: the five seeded people occupy
  positions 1..5, so the new first record lands at `1 - 1 = 0` (verified against
  the reference DB). Contract in
  `engine/core-modules/record-position/services/record-position.service.ts`
  (`buildRecordPosition:32-41` first bucket = `minPosition - index - 1`, empty
  table → 1; `:43-50` last bucket; `:86-90` undefined position joins the first
  bucket when backfill is on), summarized at
  `research/query-layer.md §4.5`.

## error-bad-filter.node.json

Command: `python3 -I rust/tools/record-parity.py --node-only --scenario error-bad-filter`

Query issued:

    query { people(filter: { name: { bogus: "x" } }) { edges { node { id } } } }

Wire-format contract:

- Record-plane domain errors are returned as HTTP 200 with `errors[].extensions`:
  `engine/middlewares/middleware.service.ts:103-116` (`statusCode = 200` at :103).
- `extensions.code` is the GraphQL `ErrorCode` (`BAD_USER_INPUT` here) and
  `extensions.subCode` the domain code (`INVALID_ARGS_FILTER`):
  `engine/core-modules/graphql/utils/graphql-errors.util.ts:21-38` plus the
  mapping at
  `engine/api/common/common-query-runners/utils/common-query-runner-to-graphql-api-exception-handler.util.ts:20-36`.
- The harness compares `errorCodes` (set of `code` + `subCode` across all
  errors, sorted → order-insensitive) and `errorShape` (the set of
  `extensions` keys); the message text is captured but compared best-effort
  only (ADR-005).

## rest-findMany.node.json

Command: `python3 -I rust/tools/record-parity.py --node-only --scenario rest-findMany`

Request issued: `GET /rest/people?limit=2&order_by=name.lastName%5BDescNullsLast%5D`

Wire-format contract:

- REST find-many envelope `{ data: { <plural>: [...] }, totalCount, pageInfo }`:
  `engine/api/rest/core/handlers/rest-api-find-many.handler.ts:91-99`;
  `FormatResult` interface `engine/api/rest/core/handlers/rest-api-base.handler.ts:40-46`.
- Resource path is the plural name (`/rest/people`):
  `engine/api/rest/core/utils/parse-core-path.utils.ts:7-45` (per
  `research/query-layer.md §7.2`).
- `order_by` values must be nulls-qualified or the request fails 400 with
  `{ statusCode, error, messages }`:
  `engine/api/rest/input-request-parsers/order-by-parser-utils/parse-order-by-rest-request.util.ts`
  (error text asserted in its `__tests__` spec :83-91); REST envelope
  `research/query-layer.md §7.3` / `graphql-rest-api.md §5.2`.
