# Next-agent prompt — finish the Twenty 2.61.0 → Nudge migration

MISSION: Finish the production-ready Rust port of Twenty CRM 2.61.0 into Nudge, autonomously, until every applicable gate passes. Do NOT declare completion while any row in `rust/FEATURE-PARITY.md` still reads **part** or **todo** (26 part / 44 done at the 2026-11-28 baseline).

## Repo and state

- Git root: `/Users/tonminh-mac/Documents/GitHub/nudge`; Rust workspace: `rust/`. Pathspecs need `rust/`.
- Work on `main`. Baseline: `25f31a9`; prior lanes merged, worktrees pruned. The handoff records 987 tests passed; re-measure before relying on that number.
- Infra containers: `twenty-ref` (twentycrm/twenty:v2.61.0, port 3001, db `twenty_ref` on `infra-db-1:5433`) and `infra-redis-1` (port 6380). Nothing else is running: no app server, no SMTP sink.
- Current branch tips with commits ahead of `main`: `rls-criteria-deny` (2), `record-defects-f26efe19` (2), `research/permissions` (1). Inspect before assuming they are merged.

## Read first, in order

1. `rust/RULES.md` — R-001 through R-005, non-negotiable.
2. `rust/PORTING_STATUS.md`, `rust/UI_PARITY.md`, `rust/BACKEND_PARITY.md` and
   `rust/KNOWN_ISSUES.md` — current status, evidence and open blockers.
3. `rust/HANDOFF-2026-11-28.md` — §4 gates/traps, §5 upstream facts, §6 open work, §7 safety.
4. `rust/FEATURE-PARITY.md` — the "What is left, in order" section is the backlog, in priority order.
5. `rust/MAP.md` and `rust/HANDOFF.md` — wave map. Load the `rust-anything` skill and follow `whole-app.md`: claim a unit in MAP.md, port it, and never mark a unit done without its gate passing.

## Work loop, repeated per unit

1. Read upstream at tag `twenty/v2.61.0` (or the built bundle in `twenty-ref` at `/app/packages/twenty-server/dist/...`) before writing code. BusyBox `grep` has no `--include`. R-002: behaviour is verified against upstream, never guessed.
2. Implement in Rust with exclusive file ownership. If fanning out subagents (~5 concurrent), each gets its own worktree and disjoint files; exactly one cargo build per worktree, with `CARGO_PROFILE_DEV_DEBUG=0`.
3. Gate: `bash tools/test-workspace.sh` (R-005: never point DB-gated tests at the shared parity database). Frontend lanes also need `cargo check -p nudge-front-core -p nudge-front-app --target wasm32-unknown-unknown`.
4. Prove against the live reference, not only unit tests: `tools/record-parity.py`, `tools/permissions-parity.py`, `tools/access-group-parity.py`, `python3 -I tools/parity-compare.py`, `tools/metadata-parity.sh`, `tools/prove-attnum-parity.sh`, `tools/email-template-parity.cjs` (inside `twenty-ref`), `tools/email-flow-e2e.py` plus `tools/smtp-sink.py`, `tools/ui-parity-harness.mjs` (R-003: zero visual diff, fresh captures only).
5. Update the relevant `FEATURE-PARITY.md` row and `HANDOFF-2026-11-28.md` with measured numbers and `file:line` references. New commits only; push to `origin/main` once the gate is green.

## Open work, in priority order

1. **Channel-visibility redaction (#36).** `messageThread { messages { … } }` returns the redacted list where 2.61.0 returns nulls (`crates/server/src/query/visibility.rs:269`). `merge.rs:118`, `duplicates.rs:62`, `group_by` are unwired; `search.rs:39-50` keeps its own object list; REST was exercised only through the shared runner. Close the four remaining read-shape diffs and re-run the oracle set.
2. **RLS criteria on the record plane (#37/#47).** Deny collection reads for a non-default role carrying a read-criteria rule; keep the all-records role unfiltered. Branch `rls-criteria-deny` is two commits ahead and was never merged: harvest it. `research/permissions.md` §2.11 has the measured pair and harness.
3. **Permissions P4–P5.** Read `IS_PERMISSION_TABLES_ENABLED` before permission DDL; extend access-group treatment to non-company objects. Reconcile the 29 `workspace.permissionFlags` diffs (15 reference vs 23 Nudge). `research/permissions` is one commit ahead.
4. **Record-plane diffs.** Last measurement: 13 total (9 merge-many, 2 orderBy-pagination, 1 findMany-first2, 1 rest-findMany) plus unmerged `record-defects-f26efe19`. #48 needs the two `ProbeWhere T` rows removed in `workspace_2uyacdvjhehfx4qxnjecohfkd`; ask before deleting.
5. **Frontend exact parity (#16/#25/#29/#43).** Runtime `document.title`, workspace favicon, production Dialog/Popover/Menu/Tabs contracts, record index/show, contextual command menu, routed side panel, settings leaves — measured live per R-003, never reusing stale ledgers.
6. **Email/webhook slice (#38/#49).** BullMQ-equivalent durability for the transactional queue versus in-process `tokio::spawn`; live webhook delivery parity against the reference container.
7. **#40 file module/storage drivers**, then wave 6 (search/import/export, timeline, favorites, notes/tasks) and wave 7 (queues, workflows, messaging/calendar sync, AI agents, MCP, billing).
8. **Performance gate (#25).** Release build, `tools/parity-bench.py`, record rps and p99 per scenario against the reference. Earlier figures: Rust 2789 rps / p99 6.5 ms vs Node 1155 rps / p99 68.8 ms on healthz; re-measure after each wave. Debug builds are not comparable.

## Rules

- **R-001:** no comments in code (Rust, SQL, shell, `.env.example`). This overrides the rust-anything `// INFALLIBLE:` convention. Prose belongs in `.md` docs.
- **R-002:** verify auth behaviour against upstream; `research/auth-permissions.md` wins over guesses.
- **R-003:** UI parity is exact and measured live; default tolerance zero; only explicit branding/text substitutions may be ignored.
- **R-004:** scope is every Twenty feature; never shrink the backlog to what is convenient.
- **R-005:** full suite only through `bash tools/test-workspace.sh` on a scratch database.
- Silence lints with `#[expect(lint, reason = "...")]`, never `#[allow]`. No `unwrap`/`expect`/`panic!` in a function whose upstream returns an error.
- Migrations are append-only once applied. Environment-variable names stay identical to the Node server.
- **Secrets:** never print or commit credentials or login logs (`provider_config.json`, `credentials.json`, local router DB). Extract `APP_SECRET` programmatically without echoing it; keep credential handles in `/tmp` mode 0600 and delete them at run end. Never type credentials into a browser — hand that step to the user.
- **Destructive/shared actions need explicit user approval at that action:** deleting rows in `twenty_ref`, dropping any of the 42 Postgres databases, removing containers you did not create, `DROP SCHEMA` on a shared database, force-push, `--no-verify`, amending published commits. Everything else — refactors, tests, merges, commits, pushing green work to `main` — do without waiting.
- Kill every server, sink and background process you started before stopping, and say so.
- Scratch scripts live in `/tmp`, not in the repo.
- Report honestly: record `file:line` and measured numbers. Never write "fixed/verified/PASS" as a substitute for evidence, and never claim completion early.
