# Backend parity

Reference: Twenty CRM `twenty/v2.61.0`. API success alone is not parity evidence;
compare normalized responses, database state, errors and side effects.

## Verified baselines

- Core schema: `tools/verify-core-schema.sh`.
- Standard-object physical order: `tools/prove-attnum-parity.sh` (39 tables).
- Metadata rows: `tools/metadata-parity.sh` (37 objects, 636 fields, 65 options,
  123 indexes, 34 search fields).
- Record differential: `tools/record-parity.py` (0 gaps / 0 diffs across 37 captures
  and 13 scenarios at the baseline).
- Access-group read set: `tools/access-group-parity.py` reports MATCH for the
  two-record visible-company fixture.
- Permission flags: synchronized to the upstream 12-flag set; the 37
  `objectRecordsPermissions` entries and `unreadableFields` match.

## Open backend units

### Permissions

- A non-default role assigned to a workspace member is still loaded as a main role
  in `permissions/context.rs`; upstream classifies it as limiting for criteria reads.
- Kernel behavior for limiting roles is already present. Branch `rls-criteria-deny`
  is production-equivalent to main and must not be merged wholesale because its tests
  and fixtures are stale.
- Required proof: non-default criteria-only member is denied collection reads with
  `FORBIDDEN` / `PERMISSION_DENIED`; paired all-records role returns all rows; REST
  returns 403; cold/invalidated caches, findOne, aggregate/group-by and nested reads
  are covered.

### Transactional email queue

Upstream `email-queue` uses BullMQ with job `EmailSenderJob`, four total attempts,
5,000 ms exponential backoff with jitter 0.5, completed retention 14,400 s / 1,000,
failed retention 604,800 s / 1,000, concurrency 1 and a 30,000 ms lock.

Local `email/service.rs` still spawns in-process delivery, so process termination drops
the task. The next unit should use Redis-backed durable jobs, one SMTP attempt per job,
lease recovery/fencing, bounded shutdown, and the same retry/retention semantics.
Password-reset enqueue is detached upstream; verification, password-changed and
invitation enqueue are awaited. There is no upstream transaction spanning token
persistence and `Queue.add`.

### Webhook delivery

Upstream enqueues after commit, fans out through `webhook-queue`, matches
`object.created/updated/deleted/destroyed/restored` plus wildcard operations, sends a
flat JSON POST with a 5,000 ms timeout, and signs
`${timestamp}:${JSON.stringify(payloadWithoutSecret)}` using HMAC-SHA256. Network and
non-2xx failures are caught and returned as `success:false`; they do not trigger HTTP
retry. The local dispatcher exists but the record-event call site is not wired.

### File module

Remaining upload/storage behavior is tracked by task #40.

### Wave 6/7

Search/import/export, timeline, favorites, notes/tasks, queues, workflows,
messaging/calendar sync, AI agents, MCP and billing remain in scope. Do not reduce the
backend scope to the daily-driver surface.

## Safety constraints

- Never print or commit credentials or auth logs.
- Never delete rows in `twenty_ref` or drop any of the 42 Postgres databases without
  explicit approval.
- Keep migrations append-only after they have run.
- DB-gated tests run through `bash tools/test-workspace.sh` on an isolated database.
