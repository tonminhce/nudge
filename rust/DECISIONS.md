# Decisions (ADRs)

Source of truth for the port. Original (reference only): C:\Users\tonminh123\Documents\crm-research\twenty
Output (this repo): C:\Users\tonminh123\Downloads\rust-crm

## ADR-001: Scope = maximum parity
Status: accepted (2026-11-26)
Context: User chose "Maximum parity" (every feature in the monorepo).
Decision: Target every feature, delivered in waves (see MAP.md). Each wave must build, pass tests and run end to end before the next starts.
Consequences: Multi-session effort (~1.8M LOC reference). Waves are ordered so every wave ships a usable app.

## ADR-002: Architecture = Postgres server + thin Tauri client
Status: accepted (2026-11-26)
Decision: axum server (crate `server`) on PostgreSQL (sqlx), Redis for cache/pubsub/queues; Tauri 2 desktop shell hosts the Leptos UI and talks to the server (URL configurable).
Consequences: Needs Postgres + Redis (docker compose provided). Tauri does not embed the backend.

## ADR-003: UI = Leptos (pure Rust), built with trunk, wrapped by Tauri
Status: accepted (2026-11-26)
Decision: Re-implement Twenty's UX (table, kanban, record page, settings, command menu) in Leptos; no reuse of twenty-front React code.
Consequences: UI parity is the largest cost centre; it is ported by feature area.

## ADR-004: Compatibility = full GraphQL + DB compatible
Status: accepted (2026-11-26)
Decision: Same Postgres layout (core schema + one schema per workspace `workspace_<id>`), same GraphQL schema on /graphql and /metadata (generated at runtime from metadata, as the original does), same REST /rest endpoints, so a live Twenty database and Twenty clients/SDK/Zapier can point at the Rust server.
Consequences: Use async-graphql dynamic schema; migrations must be byte-compatible with the original TypeORM migrations; research docs in rust/research/ pin the exact contracts.

## ADR-005: Equivalence bar
Status: accepted (2026-11-26)
Decision: GraphQL responses equal after JSON normalization (key order, float formatting); error `extensions.code` and HTTP status equal, message text best effort. Recorded cases replayed against both servers over the same database.

## ADR-006: The `core` schema is reproduced by versioned SQL migrations in the Rust server
Status: accepted (2026-11-26)
Context: Twenty's TypeORM entities are not the physical schema: upgrade commands drop columns (`isCustom`, `standardOverrides`), rename columns and change defaults. The end state after 324 instance commands is the contract, and the constraint and index names are TypeORM hashes.
Decision: The Rust server owns the `core` schema through sqlx migrations in `crates/server/migrations/core/`, written against the physical schema of 2.61.0 (see `research/data-model.md`). The authoritative check is a `pg_dump --schema-only` of a migrated 2.61.0 database; the migrations are diffed against it once the `twentycrm/twenty:2.61.0` image is available.
Consequences: A Rust server can create its own database from scratch, and can also attach to a database a Node server migrated, provided the remaining tables are added. Column and constraint names must match the original exactly; the migrations are append-only, never rewritten after they have run.

## ADR-007: The Rust server uses its own job queue instead of BullMQ
Status: accepted (2026-11-26), applies to wave 7
Context: The original runs 29 BullMQ queues on Redis, with two custom Lua subsystems (fair virtual-time scheduling, job-memory admission). Reproducing BullMQ's Redis key layout in Rust would tie the Rust worker to BullMQ internals without gaining anything.
Decision: The Rust server implements its own queue (Redis or Postgres backed, chosen in wave 7) with the same queue names and job payloads. The Node server and the Rust server do not share a queue.
Consequences: A cutover runs one worker or the other, never both against the same Redis. Job payloads stay compatible so the same handlers can be ported.

## ADR-008: Leptos UI, rendered client side, wrapped by Tauri 2
Status: accepted (2026-11-26)
Context: The original front end is 12,700 React files built with Vite. A pure Rust port (ADR-003) re-implements the UX in Leptos and ships it as static assets.
Decision: The UI is a Leptos CSR app built with `trunk`; Tauri 2 serves the built assets and hosts the shell. The UI talks to the server over the same HTTP API any other client uses, so the server stays usable without the desktop app.
Consequences: The desktop app needs the server running. `trunk` and `cargo-tauri` must be installed before wave 5.

## ADR-009: The baseline migration is generated from the live reference dump
Status: accepted (2026-11-26)
Context: Hand-writing 80 tables' DDL produced small but real mismatches (enum type name casing, default spellings, nullability, default values of auth booleans). The user approved pulling and running `twentycrm/twenty:v2.61.0`, whose migration produces the authoritative schema in database `twenty_ref`.
Decision: `crates/server/migrations/core/20260101000100_core_baseline.sql` is generated by `tools/extract-core-baseline.sh` from `baseline/core-2.61.0.sql` (a `pg_dump --schema-only --schema=core` of the reference), minus the TypeORM bookkeeping tables and with one statement fixup under `tools/fixups/` because `pg_get_constraintdef` does not round-trip. `tools/verify-core-schema.sh` proves parity (0 differences on columns, enums, indexes, constraints, functions).
Consequences: The schema cannot drift silently: any change must go through the dump + script, and the verify script must pass. The dump files are committed as evidence. When Twenty is re-pinned to a newer version, redo the dump, regenerate, and re-verify.

## ADR-010: No comments in code
Status: accepted (2026-11-26)
Context: The user set a hard rule banning comments in code.
Decision: No comments in any source the port writes: Rust (including doc comments), SQL and shell. Reasoning lives in the documentation files (DECISIONS.md, MAP.md, GLOSSARY.md, HANDOFF.md, research/). Recorded as rule R-001 in RULES.md.
Consequences: `gate`'s `// INFALLIBLE:` convention is replaced by trusting the recorded cases; code must be self-explanatory; the generated baseline strips the dump's comment lines.


