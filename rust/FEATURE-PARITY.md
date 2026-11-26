# Feature parity with the upstream CRM

What Nudge has, what it does not, and what it takes to close the gap. Status is measured, not
estimated: every "done" row has a test or a verification script behind it.

Legend: **done** = implemented and verified · **part** = partially implemented · **todo** = not
started. The wave numbers are the ones in [MAP.md](MAP.md).

## Summary

| Area | Upstream size | Nudge today |
|---|---|---|
| A. Data model and metadata engine | ~15k LOC server, 5.8k LOC UI | todo |
| B. Record APIs (GraphQL, REST, query runner) | ~30k LOC server | todo |
| C. Auth and access control | ~12k LOC server | **part** (password auth done) |
| D. Web interface (Leptos) | ~330k LOC React | todo |
| E. Automations and integrations | ~60k LOC server | todo |
| F. Platform and operations | ~10k LOC | **part** |

Roughly 3 of about 90 feature areas are done. The rest is the port.

## A. Data model and metadata engine

| Feature | Status | Wave | Notes |
|---|---|---|---|
| `core` schema, byte-identical to the reference | **done** | 1 | 80 tables, 57 enums, 368 indexes, 389 constraints; 0 differences |
| Workspace schema per tenant (`workspace_<base36>`) | **done** | 1 | created on sign-up |
| Object and field metadata CRUD | todo | 3 | the metadata API is the contract for everything below |
| Standard objects (person, company, opportunity, note, task, attachment, timeline, workspace member) | todo | 3 | |
| Runtime DDL: create and alter workspace tables from metadata | todo | 3 | the metadata to DDL migration runner |
| Composite fields (full name, emails, phones, links, address, currency) and their flattening rule | todo | 3 | `<field><SubProperty>` column names |
| Field types: 25+ (select with options, rating, relation, morph relation, raw json, ts vector, ...) | todo | 3 | |
| Index metadata and physical index creation (hashed names) | todo | 3 | |
| Search vectors and the GIN index | todo | 3 | |
| Validation rules | todo | 5 | 3.9k LOC upstream |
| Custom fields and objects edited from the UI | todo | 5 | |
| Data model settings screens | todo | 5 | 5.8k LOC upstream |

## B. Record APIs

| Feature | Status | Wave | Notes |
|---|---|---|---|
| `findMany` with filter, sort, keyset pagination, `totalCount` | todo | 4 | the shared query runner |
| `findOne`, `findDuplicates`, `mergeMany` | todo | 4 | |
| `createOne`/`createMany`, `updateOne`/`updateMany` | todo | 4 | |
| `deleteOne` (soft), `destroyOne`, `restoreMany` | todo | 4 | |
| `groupBy` and aggregates | todo | 4 | 12 aggregate operations |
| Full text search | todo | 4 | |
| Record permissions, sharing, private records | todo | 4 | |
| Export records | todo | 6 | |
| REST API with the upstream shape | todo | 4 | |
| Dynamic GraphQL schema generated from metadata | todo | 4 | the upstream generates it at runtime |
| Live updates over SSE (`OnEventSubscription`) | todo | 4 | |
| Pre and post query hooks (actor stamping, timeline snapshots) | todo | 4 | |

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
| Workspace creation without a sign-up | todo | 2 | |
| API keys | todo | 2 | one role per key |
| Session cookies and the CSRF origin check | todo | 2 | |
| Two factor authentication (TOTP, backup and recovery codes) | todo | 2 | |
| SSO: Google, Microsoft, OIDC, SAML | todo | 2 | |
| Impersonation (server and workspace level) | todo | 2 | |
| Invitations | todo | 2 | tokens stored verbatim |
| Email verification and password reset | todo | 2 | |
| Role, object, field and row level permission enforcement | todo | 2 | 27 permission flags |

## D. Web interface (Leptos)

Nothing in the interface is built yet. The upstream is about 330k lines of React across 12,700
files; the Leptos port is estimated at roughly 40k lines of Rust for the daily driver surface.

| Feature | Status | Wave | Notes |
|---|---|---|---|
| Design system: tokens, primitives, icons | todo | 5 | 25 hues, 13 step scale, 618 primitives, 503 components |
| Application shell, navigation, command menu | todo | 5 | |
| Record table: virtualised, drag and drop, groups, aggregates | todo | 5 | 101k LOC upstream, the largest area |
| Record kanban board | todo | 5 | |
| Record calendar and list views | todo | 5 | |
| Record show page and per type field display and input | todo | 5 | |
| Views, filters, sorts, group by | todo | 5 | 27 view operations |
| Side panel (57 kinds) | todo | 5 | |
| Settings pages | todo | 5 | 86k LOC upstream |
| Auth and onboarding screens | todo | 5 | 7 onboarding steps |
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
| File storage: local disk and S3 | todo | 6 | |
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

1. Finish C (the remaining auth and the permission enforcement) — it is the last piece of the
   identity foundation and the record APIs depend on it.
2. A, the metadata engine — without it there are no records to serve.
3. B, the record APIs — the surface every client uses.
4. D, the interface — the largest area by lines of code.
5. E and F, the automations and the platform.

The upstream is roughly 1.8M lines across 25,000 files. A faithful port is a multi-session
program; the waves in [MAP.md](MAP.md) are ordered so that every wave leaves a usable server.
