<p align="center">
  <picture>
    <source
      srcset=".github/assets/logo-nudge-dark.svg"
      media="(prefers-color-scheme: dark)"
    >
    <source
      srcset=".github/assets/logo-nudge-light.svg"
      media="(prefers-color-scheme: light)"
    >
    <img
      src=".github/assets/logo-nudge-light.svg"
      alt="Nudge logo"
      width="360"
    >
  </picture>
</p>

<p align="center">
  <strong>The super lightweight CRM that feels instant.</strong><br>
  High performance. User friendly. Yours to host.
</p>

<p align="center">
  <a href="#status">Status</a> ·
  <a href="#features">Features</a> ·
  <a href="#why-nudge">Why Nudge</a> ·
  <a href="#quick-start">Quick start</a> ·
  <a href="#architecture">Architecture</a> ·
  <a href="#contributing">Contributing</a>
</p>

---

Nudge is a customer relationship manager built from scratch in Rust — for teams that
want their CRM to load before they finish blinking, run on a box the size of a paperback,
and stay completely in their own hands.

Most open-source CRMs are built on interpreted stacks that need gigabytes of RAM, a
handful of Node processes and a prayer under load. Nudge takes a different road: one
compiled server, a Postgres database, and a UI that ships as static assets. The result is
a CRM that starts in milliseconds, sips memory, and scales by adding cores instead of
mystery.

**Status: under active development.** The data engine core is built and verified; the
product surfaces below are landing wave by wave — see the
[roadmap](#status). Star the repo to follow along, or jump in: the
[contributing guide](#contributing) explains where the code lives and what to build next.

## Features

Everything in one workspace, self-hosted, no per-seat fees, ever.

### CRM core
- **People, companies and deals** — the records your team lives in, with kanban pipelines,
  drag-and-drop and inline editing
- **Custom data model** — create your own objects and fields at runtime, no redeploy:
  25+ field types including composite fields (full name, emails, phones, links, address,
  currency), selects with options, ratings, relations and more
- **Views for every workflow** — tables, kanban boards, calendars and lists, with saved
  filters, sorts, group-by, compact mode and per-view visibility (workspace-wide or unlisted)
- **Favorites and navigation** — pin the records and folders your team actually uses
- **Activities** — notes, tasks with due dates and assignees, attachments, and a full
  timeline on every record
- **Merge and duplicates** — find duplicate records and merge them without losing history
- **CSV import/export** — bring your data, take your data

### Search and intelligence
- **Instant full-text search** — Postgres tsvector-powered, accent-insensitive, ranked
- **AI assistant and agents** — bring your own model (OpenAI, Anthropic, Google and more),
  chat with your CRM, let agents update records for you
- **Command menu** — everything is one keystroke away

### Automation
- **Workflows** — visual automation with triggers (record created/updated/deleted, cron
  schedule, webhook, manual) and actions (create/update/delete records, HTTP request,
  send email, run code, filter, iterate, AI agent step)
- **Webhooks** — signed HMAC delivery on every record event, with retries
- **Email and calendar sync** — Gmail, Microsoft, IMAP/SMTP and CalDAV, with contact
  auto-creation and participant matching

### Platform
- **GraphQL and REST APIs** — a dynamic GraphQL schema generated from your data model,
  cursor pagination, filtering, ordering, grouping and aggregates; a clean REST surface
  for everything else
- **API keys and roles** — granular permissions down to object-level, field-level and
  row-level rules; roles assignable to users, agents, API keys and applications
- **Multi-workspace** — one deployment, many isolated workspaces, each with its own schema
- **Authentication that scales up with you** — passwords, Google/Microsoft OAuth, OIDC,
  SAML, two-factor authentication, invitation links
- **Files** — local disk or S3-compatible storage
- **Desktop app** — the same UI in a native Tauri shell for Windows, macOS and Linux
- **Settings UI for everything** — the data model, views, roles, webhooks and integrations
  are all manageable from the app, no YAML required

## Why Nudge

### High performance
Rust gives Nudge properties no interpreted CRM can match:

| | Nudge | Typical Node/PHP CRM |
|---|---|---|
| Memory footprint | tens of MB idle | hundreds of MB to GBs |
| Cold start | milliseconds | seconds |
| Concurrency | async on tokio, all cores | process/thread gymnastics |
| Deployment | one compiled binary + Postgres | runtime + package manager + build chain |
| Tail latency | predictable, no GC pauses | dependent on JIT/GC mood |

### User friendly
- A fast, keyboard-first UI with a command menu, saved views and side panels
- Sensible defaults: create a workspace, invite your team, start typing — no setup wizard
  marathon
- The data model is editable live: add an object or field and the API, the views and the
  UI pick it up immediately

### Open source, for real
- **AGPL-3.0** — free forever, no feature paywalls, no seat counting
- **Self-host in minutes** — `docker compose` for Postgres and Redis, one binary for the
  server
- **Own your data** — a plain Postgres schema you can dump, inspect and migrate whenever
  you like
- **No telemetry by default**

## Quick start

Requires Docker and a Rust toolchain (1.85+).

```sh
git clone https://github.com/tonminhce/nudge.git
cd nudge

docker compose -f rust/infra/docker-compose.yml up -d   # Postgres 16 + Redis 7

cd rust
export PG_DATABASE_URL=postgres://postgres:postgres@127.0.0.1:5432/default
export REDIS_URL=redis://127.0.0.1:6379
export APP_SECRET=change-me-to-a-long-random-value

cargo run -p nudge-server
curl http://127.0.0.1:3000/healthz
```

## Architecture

```
┌───────────────┐    HTTP/GraphQL/REST    ┌──────────────────────┐
│  Leptos UI    │ ◄─────────────────────► │  Nudge server (Rust) │
│  (wasm, CSR)  │                         │  axum · tokio · sqlx │
└───────────────┘                         └──────────────────────┘
        │                                           │
        ▼ Tauri 2 shell (desktop)                   ▼
┌───────────────────────┐                  ┌────────────────────┐
│ Windows · macOS ·     │                  │ Postgres 16        │
│ Linux                 │                  │ Redis 7            │
└───────────────────────┘                  └────────────────────┘
```

- **`crates/core`** — shared primitives: configuration (environment-driven, sensible
  defaults), error types, identifiers
- **`crates/server`** — the API server: migrations, the metadata engine that turns your
  objects and fields into real Postgres schemas, and the GraphQL/REST surfaces
- **`crates/ui`** — the Leptos interface (landing with the UI wave)
- **`crates/desktop`** — the Tauri shell (landing with the desktop wave)

Multi-tenancy is schema-per-workspace: every workspace's records live in their own
Postgres schema, isolated at the database level, with a shared `core` schema for
metadata. Migrations are generated SQL verified against a reference dump, so upgrading
is deterministic — `rust/tools/verify-core-schema.sh` proves byte-level parity of the
schema on every run.

## Status

Nudge is developed in verified waves. Each wave must build, pass tests and run
end-to-end before the next begins.

- [x] **Wave 0** — research and architecture decisions
- [x] **Wave 1** — server foundation: config, Postgres/Redis, database migrations,
      health endpoint *(done, schema verified)*
- [x] **Wave 2** — authentication, workspaces, roles and permissions
      *(done: password auth, ES256 signing, session cookies with the CSRF origin
      check, API keys, 2FA, invitations, email verification, password reset,
      workspace lifecycle, impersonation and SSO; SAML signature verification is
      flag-gated off until a sound Rust path exists)*
- [ ] **Wave 3** — metadata engine: standard objects, live schema management
- [ ] **Wave 4** — GraphQL and REST record APIs
- [ ] **Wave 5** — the web UI and the Tauri desktop app
- [ ] **Wave 6** — files, webhooks, search, import/export, activities
- [ ] **Wave 7** — workflows, email/calendar sync, AI agents

The honest picture of what exists and what does not is tracked feature by
feature in [`rust/FEATURE-PARITY.md`](rust/FEATURE-PARITY.md): every area of the
upstream product, its size, its status here and the wave that closes it. Read
that before planning anything, and update it as waves land.

## Contributing

Issues and pull requests are welcome. The port knowledge base lives in
`rust/` (`MAP.md`, `DECISIONS.md`, `GLOSSARY.md`, `HANDOFF.md`) — read those before
proposing large changes, and keep in mind:

- `cargo fmt`, `cargo clippy -D warnings` and `cargo test` must pass
- no comments in code: if something needs explaining, it goes in the docs
- database migrations are append-only once released

## License

[AGPL-3.0](LICENSE)
