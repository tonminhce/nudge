# Twenty CRM — Workers, Workflows & Integrations → Rust porting notes

Research source: `C:\Users\tonminh123\Documents\crm-research\twenty` (read-only).
Target: Rust (tokio, sqlx, redis).

All paths below are relative to the repo root `packages/twenty-server/src/...` unless stated otherwise.
"Twenty version" = the checked-out tree (bullmq `5.78.0`, ioredis `5.10.1`, `ai` SDK `7.0.126`, `twenty-server/package.json:89,123,84`).

---

## 1. Background processing

### 1.1 Architecture

| Concern | Twenty implementation | File |
|---|---|---|
| Queue abstraction | `MessageQueueDriver` interface (`add`, `bulkAdd`, `work`, `addCron`, `removeCron`, `register`, `getJobs`, `getInFlightJobs`, `removeJob`, `reconcileJobMemory`) | `engine/core-modules/message-queue/drivers/interfaces/message-queue-driver.interface.ts` |
| Prod driver | `BullMQDriver` (bullmq `Queue` + `Worker`) | `engine/core-modules/message-queue/drivers/bullmq.driver.ts` |
| Test/dev driver | `SyncDriver` (in-process, immediate execution) | `engine/core-modules/message-queue/drivers/sync.driver.ts` |
| DI wrapper | `MessageQueueService` — one instance per queue, injected via `@InjectMessageQueue(MessageQueue.x)` | `engine/core-modules/message-queue/services/message-queue.service.ts`, `decorators/message-queue.decorator.ts` |
| Decorator-based dispatch | `@Processor(queueNameOrOptions)` on a class + `@Process(jobName)` on a method; `MessageQueueExplorer` scans providers via Nest `DiscoveryService` and registers a worker per queue | `engine/core-modules/message-queue/decorators/processor.decorator.ts`, `decorators/process.decorator.ts`, `engine/core-modules/message-queue/message-queue.explorer.ts` |
| Worker bootstrap | `queue-worker/queue-worker.ts` → `NestFactory.createApplicationContext(QueueWorkerModule)` (no HTTP server) | `queue-worker/queue-worker.ts`, `queue-worker/queue-worker.module.ts` |
| Drain on shutdown | `MessageQueueWorkerDrainService`; `BullMQDriver.onModuleDestroy` closes queues then workers; `boundedShutdownDrain` queues get a bounded abort timer | `engine/core-modules/message-queue/services/message-queue-worker-drain.service.ts`, `bullmq.driver.ts:266-351` |
| Redis clients | `RedisClientService` — separate `ioredis` clients for queue (`REDIS_QUEUE_URL` or `REDIS_URL`) and general (`REDIS_URL`); `maxRetriesPerRequest: null` (required by bullmq) | `engine/core-modules/redis-client/redis-client.service.ts` |
| Redis prefix | `REDIS_QUEUE_PREFIX` default `'bull'`; passed to bullmq as `options.prefix` | `engine/core-modules/twenty-config/config-variables.ts:1676`, `message-queue.module-factory.ts` |
| General cache | `CacheStorageService` over `cache-manager` + `@keyv/redis`; namespaced keys `${keyPrefix}:${namespace}:${key}` | `engine/core-modules/cache-storage/services/cache-storage.service.ts`, `utils/get-cache-storage-key.util.ts` |

`MessageQueueDriverType` currently only has `BullMQ` and `Sync` wired (`message-queue.module-factory.ts:16` hardcodes `BullMQ`).

### 1.2 Queue names (29)

From `engine/core-modules/message-queue/message-queue.constants.ts`:

```
record-export-queue              workspace-data-export-queue    task-assigned-queue
messaging-queue                  webhook-queue                 cron-queue
email-queue                      campaign-queue                campaign-send-queue
campaign-engagement-queue        calendar-queue                connected-account-sync-webhook-queue
contact-creation-queue           billing-queue                 workspace-queue
workspace-destroy-queue          entity-events-to-db-queue     event-log-queue
workflow-queue                   delayed-jobs-queue            delete-cascade-queue
logic-function-queue             lambda-management-queue       application-lifecycle-hook-queue
application-upgrade-queue        trigger-queue                 ai-queue
ai-stream-queue                  access-group-queue
```

### 1.3 Per-queue worker configuration

`engine/core-modules/message-queue/message-queue-worker-config.constant.ts` is the single source of truth; every queue must declare `concurrency`, `lockDuration`, `maxStalledCount`, `boundedShutdownDrain` (compile error if a queue is added without it). Optional: `globalConcurrency` (across all worker processes), `globalLimiter` (`{max, durationMs}` → bullmq `limiter`), `groupLimiter` (per workspace/app group, applied as a **delay** at enqueue time, not a bullmq feature).

Notable values:
- `workflow-queue`: `concurrency: 1`, `groupLimiter: {max: 100, durationMs: 60_000}`.
- `ai-queue`: `concurrency: 10` (agents block on model I/O).
- `ai-stream-queue`: `concurrency: 20`, `lockDuration: 600_000` (10 min — a stream holds the lock for its whole life), `maxStalledCount: 0` (a stalled stream cannot be resumed), `boundedShutdownDrain: true`.
- `campaign-engagement-queue`: `concurrency: 50`; `campaign-queue`/`campaign-send-queue`: 10.
- `logic-function-queue`: `concurrency: 20`.
- `lambda-management-queue`: `concurrency: 2`, `globalLimiter: {max: 2, durationMs: 1_000}`.
- `record-export-queue`: `concurrency: 1`, `globalConcurrency: 2`, `maxStalledCount: 0`.
- `workspace-destroy-queue`: `globalConcurrency: 1`.

`WORKER_ENABLED_QUEUES` / `WORKER_EXCLUDED_QUEUES` (comma-separated env) select which queues a worker process serves (`message-queue.explorer.ts:63-108`, `utils/should-create-worker-for-queue.util.ts`).

### 1.4 Job payload / options model

`engine/core-modules/message-queue/drivers/interfaces/job-options.interface.ts`:

```ts
QueueJobScope  = { isSystemJob: true }                       // no workspace
               | { isWorkspaceJob: true, workspaceId }        // workspace scope
               | { workspaceId, applicationUniversalIdentifier } // application scope

QueueJobSettings = {
  id?: string;                       // job id prefix (dedup)
  doNotCheckDuplicatesBasedOnPrefix?: boolean;
  priority?: number;                 // default 5 (DEFAULT_JOB_PRIORITY)
  retryLimit?: number;               // → attempts = 1 + retryLimit
  backoff?: { strategy: 'fixed'|'exponential'; initialDelayMilliseconds; jitter? };
  delay?: number;                    // ms
  deduplication?: { id: string; keepLastIfActive?: boolean };
  broadcastTo?: { workspaceId; userWorkspaceId? };
  fairnessGroupKeys?: string[];      // extra fair-scheduling groups
  groupKey?: string;                 // group for the groupLimiter
}
```

Job ids are `buildQueueJobIdWithSuffix({jobIdPrefix, suffix: uuidv4()})` → `` `${prefix}-${uuid}` `` (`utils/build-queue-job-id-with-suffix.util.ts`), i.e. an explicit `id` is a **prefix**, not the final job id — dedup is checked by comparing prefixes against waiting job ids (`utils/is-queue-job-id-already-waiting.util.ts`).

BullMQ option mapping (`bullmq.driver.ts:621-647`): `attempts = 1 + retryLimit`, `backoff {type, delay, jitter}`, `removeOnComplete {age: 14400s, count: 1000}`, `removeOnFail {age: 604800s, count: 1000}` (`constants/queue-retention.constants.ts`).

Job handler contract (`interfaces/message-queue-job.interface.ts`): the handler receives `{ id, name, data, retryLimit, updateData(data), updateProgress(progress), abortSignal }`.

`@Processor` may declare `scope: Scope.REQUEST` — then the Nest explorer builds a request-scoped instance per job with `{ req: { workspaceId: job.data?.workspaceId } }` (`message-queue.explorer.ts:196-230`).

### 1.5 Fair scheduling (custom, on top of bullmq)

Twenty replaces bullmq's priority ordering with a **virtual-time fair queue** implemented in Lua:

- `drivers/fairness/fair-tag-script.constant.ts` — computes a virtual-time *ticket* per group of group-keys (workspace / application), reading clocks and weights.
- `drivers/fairness/fair-dispatch-script.constant.ts` — rebases job priorities in the job hash (`HSET job priority fairBase`) when the virtual clock advances.
- `drivers/fairness/fair-rebase-script.constant.ts` — bulk rebase of the priority set.
- `drivers/fairness/group-rate-pace-script.constant.ts` — paces enqueues per group: each group key has a "next slot" timestamp; a job's `delay` is pushed so the group's rate limit is respected.
- `drivers/fairness/fair-ticket-allocator.ts`, `fair-queue-scheduler.ts`, `group-rate-pacer.ts` — the TS drivers.
- Constants (`drivers/fairness/fair-priority.constant.ts`): `URGENT_FAIR_PRIORITY = 1`, `FAIR_TICKET_OFFSET = 2`, `FAIR_REBASE_THRESHOLD = 2**20`, `FAIR_REBASE_STEP = 2**19`, `FAIR_REBASE_CHUNK_SIZE = 1000`, `FAIR_REBASE_GRACE_MS = 1000`, `FAIR_CLOCK_TTL_MS = 24h`.
- Fair keys live under `${prefix}:${queueName}:fair:` → `vtime`, `base`, `weights`, `clock:<groupKey>` (`fair-ticket-allocator.ts:48-70`).
- Group keys: `system` | `workspace:<workspaceId>` | `workspace:<id>` + `application:<universalIdentifier>` (`utils/build-queue-job-group-keys.util.ts`, `utils/build-workspace-group-key.util.ts`, `utils/build-application-group-key.util.ts`).
- Pace slot key: `${workspaceId}:jobGroup:${groupKey}` (`utils/build-pace-slot-key.util.ts` — comment says the prefix must not change since 2.60).
- `IS_WORKER_GROUP_LIMITER_ENABLED` gates the group limiter (`config-variables.ts:1714`).

### 1.6 Job memory limiting (custom, on top of bullmq)

`drivers/memory/`:
- `job-memory-admit-script.constant.ts` — atomic admission. Keys per group key `i`: `KEYS[2i-1]` = "used" byte counter, `KEYS[2i]` = "jobs" hash, field `"<queue>:<jobId>"` → size in bytes. Shares of free Redis memory per group; the whole batch is refused if any group exceeds its limit. `HSETNX` prevents double charging.
- `job-memory-release-script.constant.ts` — releases one job's charge (idempotent).
- `job-memory-reconcile-script.constant.ts`, `job-memory-restore-script.constant.ts` — reconciliation of charges whose BullMQ job vanished.
- `jobs-exist-script.constant.ts` — batch existence check (`EXISTS` per job key).
- `job-size-limiter.ts` — per-job size ceiling `JOB_QUEUE_MAX_JOB_SIZE_BYTES` (default 5 MiB) → `MessageQueueExceptionCode.JOB_TOO_LARGE`, surfaced as bullmq `UnrecoverableError` (`bullmq.driver.ts:446-453`).
- `constants/job-memory-limit.constant.ts`: `NOT_CHARGED_QUEUES = [access-group, billing, delete-cascade, workspace-destroy]`, `JOB_MEMORY_RECONCILE_BATCH_SIZE = 200`, `JOB_MEMORY_RECONCILE_SUSPECTS_TTL_SECONDS = 3600`.
- On completion, oversized job payloads are replaced by `{memorySizeBytes, hash: sha256(json)}` (`bullmq.driver.ts:906-932`).

### 1.7 Cron jobs

All cron jobs run **on the `cron-queue`** and are registered with bullmq's `upsertJobScheduler`, not with `@Cron`:
- `addCron` → `queue.upsertJobScheduler(getJobKey({jobName, jobId}), repeat, {name, data, opts})`; scheduler id = `` `${jobName}.${jobId}` `` (`bullmq.driver.ts:533-574`, `utils/get-job-key.util.ts`).
- `removeCron` → `queue.removeJobScheduler(sameKey)`.
- Registration command: `database/commands/cron-register-all.command.ts` (`cron:register:all`) lists 36 commands and calls `command.run()` on each, with per-command feature flags (`MARKETPLACE_CATALOG_SYNC_CRON_ENABLED`, `IS_BILLING_ENABLED`, `SIGNING_KEY_ROTATION_DAYS`). (`CodeInterpreterSessionCleanupCronCommand` is imported there but registered by its own module.)
- Each cron command lives in `*.cron.command.ts` and enqueues via `messageQueueService.addCron({jobName: SomeJob.name, data, options: {repeat: {pattern}}})`; the actual processor is `*.cron.job.ts` with `@Processor(MessageQueue.cronQueue)` + `@Process(SomeJob.name)`.

Cron patterns (all `repeat: { pattern }`, cron-parser 5-field syntax):

| Job | Pattern |
|---|---|
| `WorkflowCronTriggerCronJob` | `* * * * *` |
| `WorkflowRunEnqueueCronJob` | `* * * * *` |
| `CronTriggerCronJob` (logic functions) | `* * * * *` |
| `AgentCronTriggerCronJob` | `* * * * *` |
| `PendingWakeUpSweepCronJob` | `*/5 * * * *` |
| `MessagingMessagesImportCronJob` | `*/1 * * * *` |
| `MessagingMessageListFetchCronJob` | `2-59/5 * * * *` |
| `MessagingOngoingStaleCronJob` | `0 * * * *` |
| `MessagingRelaunchFailedMessageChannelsCronJob` | `*/30 * * * *` |
| `MessagingMessageChannelSyncStatusMonitoringCronJob` | `2/10 * * * *` |
| `CalendarEventsImportCronJob` | `*/1 * * * *` |
| `CalendarEventListFetchCronJob` | `*/5 * * * *` |
| `CalendarOngoingStaleCronJob` | `0 * * * *` |
| `CalendarRelaunchFailedCalendarChannelsCronJob` | `*/30 * * * *` |
| `WorkflowHandleStaledRunsCronJob` | `*/10 * * * *` |
| `DeferredWorkspaceOperationRecoveryCronJob` | `*/10 * * * *` |
| `WorkflowCleanWorkflowRunsCronJob` | `0 */3 * * *` |
| `ReconcileCampaignStatsCronJob` | `15 * * * *` |
| `EmailingOngoingStaleCronJob` | `0 * * * *` |
| `WebhookSubscriptionRenewalCronJob` | `0 * * * *` |
| `EventLogCleanupCronJob`, `PendingFileCleanupCronJob`, `CheckEmailingDomainVerificationCronJob`, `CheckCustomDomainValidRecordsCronJob`, `CheckPublicDomainsValidRecordsCronJob`, `UsageLimitInstanceOverrideExpiryCronJob`, `CodeInterpreterSessionCleanupCronJob`, `MarketplaceCatalogSyncCronJob` | `0 * * * *` |
| `RotateSigningKeysCronJob` | `15 3 * * *` |
| `UserSessionCleanupCronJob` | `0 3 * * *` |
| `TrashCleanupCronJob` | `10 0 * * *` |
| `StaleRegistrationCleanupCronJob` | `30 2 * * *` |
| `BillingReminderCronJob` | `0 8 * * *` |
| `ApplicationRecurringChargeCronJob` | `0 5 * * *` |
| `EnterpriseKeyValidationCronJob` | `0 4 * * *` or a server-id-derived offset `${second} ${minute} ${hour} * * *` |
| `CleanOnboardingWorkspacesJob` | `30 * * * *` |
| `CleanSuspendedWorkspacesJob` | `0 * * * *` |
| `WorkspaceDataExportCleanupCronJob` | `30 * * * *` |

Cron **dispatch** dedup is separate from the cron scheduler: `engine/core-modules/cron/services/cron-trigger-deduplication.service.ts` computes `prev()` from the pattern with `cron-parser`, and only dispatches if `now - prev < 60_000`; the dedup lock key is `${keyPrefix}:${lastTriggerTimestamp}` in the `engine:lock` cache namespace with a 2-minute TTL.

### 1.8 Redis keys a Rust worker must know

BullMQ v5 key layout (bullmq internals, not present in Twenty source; prefix from `REDIS_QUEUE_PREFIX`, default `bull`):

```
bull:<queue>:<jobId>              HASH   job payload (name, data, opts, priority, progress, attemptsMade, ...)
bull:<queue>:wait                 LIST   waiting job ids (RPUSH by the worker, LPOP by the worker)
bull:<queue>:active               LIST
bull:<queue>:delayed              ZSET   score = timestamp when the job becomes due
bull:<queue>:completed            ZSET
bull:<queue>:failed               ZSET
bull:<queue>:paused               LIST
bull:<queue>:prioritized          ZSET   score = priority (Twenty writes custom `fairBase` into the job hash too)
bull:<queue>:waiting-children     ZSET
bull:<queue>:id                   STRING counter for auto-generated job ids
bull:<queue>:delayed-count,…      counters (bullmq v5 "shorthand" counters)
bull:<queue>:meta                 HASH   bullmq queue metadata (opts, version, paused)
bull:<queue>:meta-paused          STRING
bull:<queue>:stalled              ZSET
bull:<queue>:repeat               ZSET   repeatable-job scheduler keys
bull:<queue>:repeat:<key>         ZSET   bullmq 5 job-scheduler key (`upsertJobScheduler` id)
bull:<queue>:<schedulerId>        ZSET   scheduler definition
bull:<queue>:limiter:<...>        ZSET   bullmq global limiter keys
```

Twenty-specific keys on top of bullmq:

| Key | Type | Where |
|---|---|---|
| `bull:<queue>:fair:vtime`, `bull:<queue>:fair:base`, `bull:<queue>:fair:weights`, `bull:<queue>:fair:clock:<groupKey>` | STRING / HASH | `drivers/fairness/fair-ticket-allocator.ts:48-70` |
| `<groupKey>:used` and `<groupKey>:jobs` (job memory charge counters) | STRING / HASH | `drivers/memory/job-memory-admit-script.constant.ts` (keys built by `job-memory-limiter.ts`) |
| `<workspaceId>:jobGroup:<groupKey>` | STRING (ms timestamp) | `utils/build-pace-slot-key.util.ts` |
| `<namespace>:<key>` with optional `<keyPrefix>:` prefix | any | `engine/core-modules/cache-storage/utils/get-cache-storage-key.util.ts`; namespaces in `types/cache-storage-namespace.enum.ts` (`module:messaging`, `module:emailing`, `module:calendar`, `module:workflow`, `module:applications`, `engine:workspace`, `engine:lock`, …) |
| `messages-to-import:<workspaceId>:<channelId>`, `messages-to-import-total:…`, `messages-imported:…` | SET / counter | `modules/messaging/message-import-manager/services/messaging-message-list-fetch.service.ts:110-190` |
| `calendar-events-to-import:<workspaceId>:<calendarChannelId>` | SET | `modules/calendar/calendar-event-import-manager/services/calendar-fetch-events.service.ts:96` |
| `workflow-run-not-started-count:<workspaceId>`, `workflow-enqueue-running:<workspaceId>`, `workflow:execution-soft-throttle:<workspaceId>`, `workflow:execution-hard-throttle:<workspaceId>` | counter / lock | `modules/workflow/workflow-runner/workflow-run-queue/workspace-services/workflow-throttling.workspace-service.ts:150-180` |
| `workflow-cron:<workspaceId>:<workflowId>:<lastTriggerTimestamp>` | lock (TTL 120 s) | `engine/core-modules/cron/services/cron-trigger-deduplication.service.ts:38` |

### 1.9 Recommendation: do not re-implement BullMQ

BullMQ's wire format (the Lua scripts `addJob`, `moveToActive`, `moveToFinished`, `moveToDelayed`, `updateProgress`, `retryJob`, plus the `stalled` checker and the `repeat` scheduler) is large, versioned, and coupled to bullmq's own key names. Re-implementing it in Rust is high-risk.

Recommended Rust design:
1. **Own the queue, own the semantics.** Implement a Redis-backed queue in Rust with your own key namespace (e.g. `tq:<queue>:*`), your own Lua-free operations (`RPUSH`/`LPOP`/`BRPOPLPUSH`/`ZADD`/`ZPOPMIN`), your own at-least-once semantics, per-job lock (`SET key token NX PX`), visibility timeout, and dead-letter queue. Use `tokio::task::JoinSet` for the worker pool. Crates: `redis` (with `aio` + `tokio-comp`), `tokio`, `async-trait`, `serde`, `serde_json`, `thiserror`, `tracing`.
2. **Or use a Rust-native queue**: `apalis` (SQL/Redis backends, `tokio` support), `lapin` (AMQP, needs a broker), `sift`/`flume` + Postgres, or `pgmq` (Postgres extension) — `pgmq` maps well because Twenty already stores runs/jobs in Postgres (`workflowRun` table, `NOT_STARTED_RUNS_FIND_OPTIONS`).
3. **If coexistence with a live Twenty deployment is required** (rolling migration), speak bullmq only for the queues you must share and treat it as a compatibility adapter: write jobs with `jobId = "<prefix>-<uuid>"`, `priority`, `opts` JSON with `attempts`, `backoff`, `removeOnComplete`, `removeOnFail`, `broadcastTo`, and use `HINCRBY`-based `bull:<queue>:id` for auto ids. Because bullmq's worker uses Lua scripts for state transitions, the *safe* direction is: **let bullmq (or a JS sidecar) consume, let Rust produce**; or keep one queue implementation and drain the other.

**Porting difficulty: MEDIUM** (7/10). The queue semantics themselves are simple; the hard parts are the custom Lua (fair scheduling + memory admission), the multi-namespace cache layout, and the sheer number of job payload shapes.
**Best crates**: `redis` (0.27+, `aio`/tokio-comp), `tokio` (`tokio::time`, `tokio::sync::Semaphore`), `tokio-util` (`CancellationToken`, `TaskTracker`), `serde`/`serde_json`, `apalis` (if you want a ready-made queue), `sqlx` (for run state), `tracing`, `metrics`/`prometheus`.

---

## 2. The workflow engine

Two layers:
- **Core layer** (`engine/core-modules/workflow/`) — the *definition*: `WorkflowEntity` (table `core.workflow`) and `WorkflowVersionEntity` (table `core.workflowVersion`), the GraphQL resolvers, and the core <-> workspace sync service.
- **Workspace layer** (`modules/workflow/`) — the *runtime*: trigger dispatch, run creation, the executor, the step actions, and the run queue.

### 2.1 Data model

`engine/core-modules/workflow/entities/workflow.entity.ts`:

| Column | Type | Notes |
|---|---|---|
| `id` | uuid PK | |
| `name` | text null | |
| `workspaceId` | uuid | indexed (`IDX_WORKFLOW_WORKSPACE_ID`) |
| `applicationId` | uuid | indexed |
| `versionDefinitionHash` | text null | hash of the definition |
| `lastPublishedVersionId` | uuid null | workspace-side version |
| `workspaceWorkflowId` | uuid null | the legacy workspace workflow id |
| `lastPublishedCoreWorkflowVersionId` | uuid null | the *active* core version |
| `visibility` | enum | `WorkflowVisibility.WORKSPACE` default; `PRIVATE` hides the workflow *and* its versions/steps (`CoreWorkflowIdResolutionService`) |
| `isSystem` | bool | |
| `createdByUserWorkspaceId` | uuid null | |
| `jobConfig` | jsonb | was `{limiter:{max:100,durationMs:60_000}}`, dropped in upgrade 2-61 |

`engine/core-modules/workflow/entities/workflow-version.entity.ts`:

| Column | Type | Notes |
|---|---|---|
| `id` | uuid PK | |
| `triggers` | jsonb null | `WorkflowTrigger[]` — a workflow has exactly one trigger in practice (`triggers?.[0]`) |
| `steps` | jsonb null | `WorkflowAction[]` — the step graph |
| `status` | enum | `WorkflowVersionStatus`: `DRAFT`, `ACTIVE`, `DEACTIVATED`, `ARCHIVED` |
| `workflowId` / `coreWorkflowId` | uuid | `coreWorkflowId` is the FK to `core.workflow` (`onDelete: CASCADE`) |
| `workspaceWorkflowVersionId` | uuid null | |
| `isSystemSideEffect` | bool | |
| `applicationId` | uuid | inherited from `SyncableEntity` |

Unique partial indexes: one `ACTIVE` version per workflow per workspace; unique `workspaceWorkflowVersionId`.

**The run** is a workspace-scoped standard object, `modules/workflow/common/standard-objects/workflow-run.workspace-entity.ts`:
- `status`: `NOT_STARTED | RUNNING | COMPLETED | FAILED | ENQUEUED | STOPPING | STOPPED`.
- `state`: `{ flow: { trigger, steps }, stepInfos: Record<stepId, StepInfo>, workflowRunError? }` — **the whole flow graph is snapshotted into the run** at creation time (`workflowRunStateSchema`, `twenty-shared/src/workflow/schemas/workflow-run-state-schema.ts`). Past runs therefore keep the definition they executed.
- `StepInfo` = `{ status, result?, error?, retryAttempt?, wait?, threadId?, history?[] }`; `StepStatus` = `NOT_STARTED | RUNNING | SUCCESS | STOPPED | FAILED | FAILED_SAFELY | PENDING | SKIPPED` (`twenty-shared/src/workflow/types/WorkflowRunStateStepInfos.ts`).
- `enqueuedAt`, `createdAt`; a per-step log table holds tool I/O logs (`WorkflowRunStepLogWorkspaceService`).

### 2.2 Triggers

`modules/workflow/workflow-trigger/types/workflow-trigger.type.ts`:

```ts
enum WorkflowTriggerType { DATABASE_EVENT, MANUAL, CRON, WEBHOOK }
```

| Type | Settings |
|---|---|
| `DATABASE_EVENT` | `{ eventName: string }` (e.g. `"person.created"`) plus `outputSchema` |
| `MANUAL` | `{ objectType?, icon?, isPinned?, availability?: Global \| SingleRecord \| BulkRecords }` |
| `CRON` | `{ type: 'DAYS' \| 'HOURS' \| 'MINUTES' \| 'CUSTOM', schedule \| pattern }` -> converted to a cron pattern by `workflow-trigger/utils/compute-cron-pattern-from-schedule.ts` |
| `WEBHOOK` | GET: `{ httpMethod:'GET', authentication:'API_KEY'\|null }`; POST: `{ httpMethod:'POST', authentication, expectedBody, expectedOutputSchema? }` |

Every trigger also carries `outputSchema`, `nextStepIds?`, `position {x,y}`.

**Trigger -> run dispatch:**

1. *Database event*: `modules/workflow/workflow-trigger/automated-trigger/listeners/workflow-database-event-trigger.listener.ts` listens to `*.<action>`; for each cached trigger whose `settings.eventName` matches (via `computeTriggerEventNamesMatchingEvent`), it enriches the payload with relations, filters by watched fields (`filterEventsByUpdatedFields`), evaluates the trigger's own step filters (`evaluateStepFilters` with the event under the `trigger` context key), applies record-level permissions (`RecordEventPermissionsReadService.filterReadableEvents`), and enqueues a `WorkflowTriggerJob`.
2. *Cron*: `modules/workflow/workflow-trigger/automated-trigger/crons/jobs/workflow-cron-trigger-cron.job.ts` runs every minute (`* * * * *`), reads cached cron triggers from `CacheStorageNamespace.ModuleWorkflow`, checks the cron dedup service, and enqueues `WorkflowTriggerJob`.
3. *Manual*: GraphQL mutation -> `CoreWorkflowRunnerService.run(...)` directly.
4. *Webhook*: `engine/core-modules/workflow/controllers/workflow-trigger.controller.ts` — `POST|GET /webhooks/workflows/:workspaceId/:workflowId`, guarded by `PublicEndpointGuard`. It resolves the workflow, requires an `ACTIVE` version with a `WEBHOOK` trigger, and calls `CoreWorkflowRunnerService.run` synchronously (returns `{workflowName, success, workflowRunId}`).
5. *Logic functions / agents* have parallel (older) trigger machinery: `engine/core-modules/logic-function/logic-function-trigger/triggers/{cron,database-event,route}` and `engine/metadata-modules/ai/ai-agent-trigger/`.

`WorkflowTriggerJob` (`modules/workflow/workflow-trigger/jobs/workflow-trigger.job.ts`) resolves the *core* version (there is a legacy workspace-version dispatch path too), verifies `status === ACTIVE` and that the core version belongs to the workflow, then calls `CoreWorkflowRunnerService.run`.

Job payload: `WorkflowTriggerJobData = { workspaceId, workflowId, payload, coreWorkflowVersionId?, workspaceWorkflowVersionId? }`.

### 2.3 Run creation and enqueue

`modules/workflow/workflow-runner/services/core-workflow-runner.service.ts`:

- Resolves `workflowVersion` -> `trigger` -> `steps`; a missing definition throws `WorkflowRunException(WORKFLOW_RUN_INVALID)`.
- Billing: `billingUsageService.getSubscriptionInactiveReason`; hard throttle via `WorkflowThrottlingWorkspaceService.throttleOrThrowIfHardLimitReached`.
- Manual trigger -> run created directly as `ENQUEUED` and enqueued immediately.
- Other triggers -> run created as `NOT_STARTED`, the `not-started` counter is incremented, and a `WorkflowRunEnqueueJob` is queued on `workflow-queue` (`{workspaceId, isCacheMode:true}`).
- `WorkflowRunEnqueueWorkspaceService` (`workflow-run-queue/workspace-services/workflow-run-enqueue.workspace-service.ts`) then:
  - takes a **per-workspace lock** `workflow-enqueue-running:<workspaceId>` (TTL 60 s, `acquireLock`),
  - reads the number of `NOT_STARTED` runs (cached count or a DB count),
  - admits runs in batches of `QUERY_MAX_RECORDS` through a **soft token bucket** (`WORKFLOW_EXEC_SOFT_THROTTLE_LIMIT` / `_TTL`),
  - flips them to `ENQUEUED` with `enqueuedAt`, and enqueues one `RunWorkflowJob` per run.
- Job options: `buildRunWorkflowJobOptions` -> `{ id: workflowRunId, doNotCheckDuplicatesBasedOnPrefix: true, groupKey: 'workflow', ...scope }` — **the workflowRun id is the bullmq job id**, so a run is enqueued at most once.

Constants (`workflow-run-queue/constants/`): `NUMBER_OF_WORKFLOW_RUNS_TO_KEEP = 1000`, `RUNS_TO_CLEAN_THRESHOLD_DAYS = 14`, `STALED_RUNS_THRESHOLD_MS = 1h`, `STUCK_RUNNING_RUNS_THRESHOLD_MS = 1h`, `STUCK_STOPPING_RUNS_THRESHOLD_MS = 1h`.

### 2.4 Execution semantics

`modules/workflow/workflow-executor/workspace-services/workflow-executor.workspace-service.ts` is the heart.

- `executeFromSteps({stepIds, workflowRunId, workspaceId})` runs the given steps **in parallel** (`Promise.all`), then `computeWorkflowRunStatus`.
- `executeFromStep`:
  1. `shouldExecuteStep({step, steps, stepInfos, workflowRunStatus})` — false when the run is not `RUNNING`, when the step already started, when a parent step is not satisfied (`shouldExecuteChildStep`), or when it is an iterator step (iterator has its own gate `shouldExecuteIteratorStep`).
  2. `executeStep` -> sets the step to `RUNNING` -> `assertActionTypeEnabled` (feature flag per action type) -> `applicationWorkflowStepPermissionService.assertRunApplicationMayUseStepOrThrow` -> usage-quota admission (`UsageQuotaAdmissionService.assertUsageAllowed`, `UsageResourceType.WORKFLOW` / `UsageOperationType.WORKFLOW_EXECUTION`) -> `workflowAction.execute({currentStepId, steps, context, runInfo})`.
  3. On error: `isUserFacingWorkflowExecutorError` decides whether it is a user error (no Sentry); a system error increments `MetricsKeys.WorkflowRunSystemError`.
  4. **Retry**: if the action returned an error that is not a user error and `stepHasRetryAttemptsLeft`, the step is moved to retry and a *delayed* `RunWorkflowJob` is enqueued with `delay = STEP_RETRY_DELAYS_MS[attempt]` = `[1_000, 5_000, 15_000]` (`twenty-shared/src/workflow/constants/StepRetryDelaysMs.ts`), capped by the step's configured retry count and by the length of that array.
  5. **Fail-safe**: if the step failed and an enclosing iterator has `continueOnFailure`, or the step itself is marked `continueOnFailure`, the output is flagged `shouldFailSafely`.
  6. Billing: a successful step charges 100 micro-credits x 1 invocation (`chargeWorkflowNodeRun`).
- **Step output -> status mapping** (`processStepExecutionResult`):
  `wait` -> `PENDING`; `shouldEndWorkflowRun` -> `STOPPED`; `shouldRemainRunning` -> `RUNNING`; `shouldFailSafely` -> `FAILED_SAFELY`; `result` -> `SUCCESS`; `shouldSkipStepExecution` -> `SKIPPED`; else -> `FAILED`.
  A step that returns `wait` is **armed** before its status flips to `PENDING` (`workflowStepWaitWorkspaceService.arm`) so a racing event is deferred, not lost.
- **Run completion** (`computeWorkflowRunStatus`): `STOPPING` + no running steps -> `STOPPED`; `workflowShouldFail` -> `FAILED`; `workflowShouldKeepRunning` (any `PENDING`/`RUNNING` step, or a terminal step with unresolved children) -> stay `RUNNING`; otherwise `COMPLETED`.
- **Step budget**: after `MAX_EXECUTED_STEPS_COUNT = 20` executed steps, the executor re-enqueues a `RunWorkflowJob` with `lastExecutedStepId` to continue in a fresh job (`continueExecutionFromStepInAnotherJob`).

`RunWorkflowJob` (`modules/workflow/workflow-runner/jobs/run-workflow.job.ts`) handles four cases from its payload `{workflowRunId, workspaceId, lastExecutedStepId?, stepIdsToRetry?, awaitedStepOutput?}`:
- `awaitedStepOutput` -> claim the `PENDING` step atomically (`updateStepInfoIfPending`) and execute it with the handed-over output;
- `stepIdsToRetry` -> reset those steps to `NOT_STARTED` and re-execute;
- `lastExecutedStepId` -> compute next steps and continue;
- none -> start the run from `trigger.nextStepIds`.

Next-step computation (`getNextStepIdsToExecute`): iterator first (`getNextStepIdsForIterator`), then if/else (`getNextStepIdsForIfElse` -> union of all branch `nextStepIds`), else `step.nextStepIds`.
Skipping is **cascading**: `skipAndFailSafelyStepsThenContinue` marks children `SKIPPED`/`FAILED_SAFELY` and recurses.

Child/parent relationships (`utils/find-child-step-ids.util.ts`, `utils/find-parent-steps.util.ts`): children are `step.nextStepIds`, except for `IF_ELSE` where they are the union of the branch targets; an iterator's loop children are *not* parents.

### 2.5 Step types (actions)

`twenty-shared/src/workflow/types/WorkflowActionType.ts` (23 types) and `modules/workflow/workflow-executor/factories/workflow-action.factory.ts`:

| Type | Implementation | Notes |
|---|---|---|
| `CODE` | `workflow-actions/code/code.workflow-action.ts` | Inline JS/TS executed by the **logic-function engine** (`CodeStepRunnerService`) or a legacy logic function; only `settings.input.logicFunctionInput` goes through variable resolution (the code itself stays verbatim) |
| `LOGIC_FUNCTION` | `workflow-actions/logic-function/` | runs an app logic function |
| `MCP_TOOL` | `workflow-actions/mcp-tool/` | calls a tool on a configured MCP server; gated by `FeatureFlagKey.IS_MCP_SERVERS_ENABLED` |
| `SEND_EMAIL` / `DRAFT_EMAIL` | `workflow-actions/mail-sender/` | body is a JSON `EmailDocument`; variables resolved per string context; sender = connected account id or workspace member id |
| `SEND_CHAT_MESSAGE` | `workflow-actions/send-chat-message/` | gated by `IS_AI_CHAT_INBOX_ENABLED` |
| `CREATE_CALENDAR_EVENT` | `workflow-actions/create-calendar-event/` | |
| `CREATE_RECORD` / `UPDATE_RECORD` / `DELETE_RECORD` / `UPSERT_RECORD` / `FIND_RECORDS` / `PICK_RECORD` | `workflow-actions/record-crud/` | via `CreateRecordService` etc.; relation fields formatted, invalid fields filtered, TipTap rich text converted to Markdown |
| `FORM` | `workflow-actions/form/form.workflow-action.ts` | returns `{wait:{type:'CALLBACK'}}` — the run parks until a form response arrives |
| `FILTER` | `workflow-actions/filter/filter.workflow-action.ts` | evaluates `stepFilters`/`stepFilterGroups`; sets `shouldEndWorkflowRun: !matchesFilter` |
| `IF_ELSE` | `workflow-actions/if-else/` | branch union |
| `HTTP_REQUEST` | `workflow-actions/http-request/` | a `ToolBackedWorkflowAction` over `HttpTool`; SSRF-safe client; `outboundRequestSource: 'workflow-http'` |
| `AI_AGENT` | `workflow-actions/ai-agent/ai-agent.workflow-action.ts` | starts an agent run; the run is handed to the agent engine, which calls the step back with the outcome (`awaitedStepOutput`) |
| `CLASSIFY` | `workflow-actions/classify/` | |
| `ITERATOR` | `workflow-actions/iterator/iterator.workflow-action.ts` | loops `items` (array or JSON string) up to `MAX_ITERATIONS = 10000`; resets the loop's step infos each pass and pushes `history` |
| `EMPTY` | `workflow-actions/empty/` | no-op |
| `DELAY` | `workflow-actions/delay/delay.workflow-action.ts` | `SCHEDULED_DATE` or `DURATION` -> `{wait:{type:'TIME', resumeAt}}` |
| `WAIT_FOR_EVENT` | `workflow-actions/wait-for-event/` | `{wait:{type:'EVENT', eventName, recordId?, updatedFields?, expiresAt?}}` |

`ToolBackedWorkflowAction` (`workflow-actions/tool-backed/tool-backed.workflow-action.ts`) is the shared base: `preprocessInput` -> `resolveInput` (variable resolution) -> `postprocessInput` -> `tool.execute(input, ctx)` -> persist a step log -> `{result}` or `{error}`.

### 2.6 Variable resolution

`twenty-shared/src/utils/variable-resolver.ts`:

- Pattern `{{path}}` (regex `/\{\{([^{}]+)\}\}/g`).
- `resolveInput(input, context)` walks strings / arrays / objects (keys too).
- A **lone** `{{path}}` keeps its native type (`evalFromContext`); embedded tokens are stringified (`resolveStringTemplate`).
- `twenty-shared/src/utils/evalFromContext.ts` is a **Handlebars-compatible re-implementation** (Handlebars is CommonJS and cannot be tree-shaken from the browser bundle). It reproduces Handlebars quirks on purpose: own properties only, `@root`/`this`/`./` prefixes, `[bracket]` literal segments, `../` rejected, keywords (`true/false/null/undefined`) and numbers as literals, trailing numeric/keyword segment resolves to `undefined`, `@root.` short-circuits on falsy while other paths short-circuit on `null|undefined`, values JSON round-tripped (Dates -> ISO strings, `NaN`/functions dropped).
- The **context** is `{ [stepId]: stepResult }` for every step that has a result — `getWorkflowRunContext(stepInfos)` (`twenty-shared/src/workflow/utils/getWorkflowRunContext.ts`). The trigger's payload is exposed under the reserved step id `trigger` (`TRIGGER_STEP_ID = 'trigger'`, `twenty-shared/src/workflow/constants/TriggerStepId.ts`).
- Path escaping helpers: `twenty-shared/src/workflow/utils/variable-path.util.ts` (`parseVariablePath`, `escapePathSegment` — bracket notation for keys with spaces/dots).

### 2.7 Waiting / pausing

Two mechanisms:
1. **`pendingWakeUp`** table (`core.pendingWakeUp`, `engine/core-modules/pending-wake-up/entities/pending-wake-up.entity.ts`) — durable waits: condition `{type:'TIME'|'EVENT'|'ANSWER', ...}`, `resumeAt`, unique `(ownerType, ownerId, ownerKey)`. A row is **deleted to claim it**, so a wake-up resolves exactly once. `PendingWakeUpService.arm` upserts and enqueues `ResumePendingWakeUpJob` on `delayed-jobs-queue`; `PendingWakeUpSweepCronJob` (`*/5 * * * *`) re-arms time-based waits that outlived their job.
2. **`wait` on the step** — `workflowStepWaitSchema` = `CALLBACK` | the `pendingWakeUp` conditions. The step sits in `PENDING` until resolved; the resolution re-enqueues `RunWorkflowJob` with `awaitedStepOutput`.

### 2.8 Staleness and cleanup

- `WorkflowHandleStaledRunsCronJob` (`*/10 * * * *`) -> `workflow-handle-staled-runs.job` on `workflow-queue`: marks runs stuck `RUNNING`/`STOPPING` beyond 1 h as failed (`workflow-handle-staled-runs.workspace-service.ts`).
- `WorkflowCleanWorkflowRunsCronJob` (`0 */3 * * *`) -> keeps the newest 1000 runs per workflow, deletes runs older than 14 days.

**Porting difficulty: MEDIUM-HIGH** (7/10).
The engine is a faithful, well-factored state machine; the traps are (a) the Handlebars-compatible variable resolver (must be ported byte-for-byte, including quirks — it is a self-contained ~250-line file, easy to port literally), (b) the step-status / wait / fail-safe / retry matrix, (c) the parallel step execution with "claim" semantics, (d) the two trigger systems (core + legacy workspace) which both must be ported or one dropped.

**Best crates**: `serde`/`serde_json` for the JSONB `triggers`/`steps`/`state` (typed structs mirroring `twenty-shared/src/workflow/schemas/*`), `sqlx` for `core.workflow` / `core.workflowVersion` / `workflowRun` / `pendingWakeUp`, `tokio` for the parallel step execution (`JoinSet`), `async-trait`, `thiserror`, `regex` (variable pattern), `chrono` (time), `cron` (cron-parser parity — note cron-parser accepts 5- and 6-field patterns), `tracing`. MCP tool calls and code steps need an outbound sandbox: `reqwest` (HTTP), `tokio::process` (child processes for the local logic-function driver).

---

## 3. Messaging and calendar sync

### 3.1 Providers and entities

`twenty-shared/src/types/ConnectedAccountProvider.ts`: `google`, `microsoft`, `imap_smtp_caldav`, `oidc`, `saml`, `email_group`, `app`.
`twenty-shared/src/types/MessageChannelType.ts`: `EMAIL`, `SMS`, `EMAIL_GROUP`, `APP`.
`twenty-shared/src/types/CalendarChannelSyncStage.ts` / `MessageChannelSyncStage.ts` mirror each other.

`core.connectedAccount` (`engine/metadata-modules/connected-account/entities/connected-account.entity.ts`): `handle`, `externalAccountId`, `provider`, `accessToken`/`refreshToken` (**encrypted**, DB check `LIKE 'enc:v2:%'`), `accessTokenExpiresAt`, `authFailedAt`/`authFailedReason`, `handleAliases[]`, `scopes[]`, `connectionParameters` (JSONB, IMAP/SMTP/CALDAV passwords encrypted — DB check constraint), `userWorkspaceId`, `connectionProviderId`.

`core.messageChannel` (`engine/metadata-modules/message-channel/entities/message-channel.entity.ts`): `handle`, `displayName`, `type`, `visibility`, `isContactAutoCreationEnabled`, `contactAutoCreationPolicy` (`SENT_AND_RECEIVED|SENT|NONE`), `messageFolderImportPolicy` (`ALL_FOLDERS|SELECTED_FOLDERS`), `excludeNonProfessionalEmails`, `excludeGroupEmails`, `isSyncEnabled`, `syncStage`, `syncStatus`, `syncCursor`, `syncStageStartedAt`, `throttleFailureCount`, `throttleRetryAfter`, `syncedAt`, `webhookSubscriptionStatus`, `webhookSubscriptionExternalId`.

Workspace standard objects (`modules/messaging/common/standard-objects/`): `message`, `messageParticipant`, `messageThread`, `messageThreadTarget`, `messageChannelMessageAssociation`, `messageChannelMessageAssociationMessageFolder`.
`modules/calendar/common/standard-objects/`: `calendarEvent`, `calendarEventParticipant`, `calendarEventTarget`, `calendarChannelEventAssociation`.

### 3.2 Messaging import pipeline (two-phase)

**Phase A — message list fetch** (finds *which* messages changed):
- Cron `MessagingMessageListFetchCronJob` (`2-59/5 * * * *`, on `cron-queue`) selects channels with `isSyncEnabled`, `syncStage = MESSAGE_LIST_FETCH_PENDING`, `type ∈ POLLED_MESSAGE_CHANNEL_TYPES` (= `[EMAIL]` only; push channels would be scheduled forever — `twenty-shared/src/constants/PolledMessageChannelTypes.ts`), in ACTIVE workspaces. Channels that are throttled (`isThrottled`) or whose webhook subscription is active and whose last sync is fresh are skipped.
- It marks the channels scheduled and enqueues `MessagingMessageListFetchJob` on `messaging-queue` (payload `{messageChannelId, workspaceId}`).
- `MessagingMessageListFetchService` (`message-import-manager/services/messaging-message-list-fetch.service.ts`):
  1. processes pending group-email and folder actions,
  2. `syncMessageFoldersService.syncMessageFolders`,
  3. `messagingGetMessageListService.getMessageLists` -> per-folder list of `messageExternalIds` + `nextSyncCursor`,
  4. clears the cache keys `messages-to-import:{ws}:{channelId}`, `messages-to-import-total:…`, `messages-imported:…`,
  5. chunks external ids by 200, subtracts the ids already present in `messageChannelMessageAssociation`, and `SADD`s the remainder into `messages-to-import:{ws}:{channelId}` (TTL 1 week).
- **Driver specifics**
  - *Gmail* (`drivers/gmail/services/gmail-get-message-list.service.ts`): `gmail.users.messages.list` with `q = computeGmailExcludeSearchFilter(...)` (excluded labels/categories), paginated; the sync cursor is the **`historyId`** of the first message (`users.messages.get`). The incremental path uses `GmailGetHistoryService` -> `users.history.list` with `historyTypes = [messageAdded, messageDeleted, labelAdded, labelRemoved]`.
  - *Microsoft* (`drivers/microsoft/services/microsoft-get-message-list.service.ts`): Graph **delta** queries per folder, batched 20 folders per `/$batch` request, `Prefer: odata.maxpagesize=999, IdType="ImmutableId"`; per-folder `syncCursor`.
  - *IMAP* (`drivers/imap/services/imap-get-message-list.service.ts`, `imapflow` 2.0.2): per-mailbox UID ranges; the sync cursor is a serialized mailbox state (`createSyncCursor`/`parseSyncCursor`); missing mailboxes are skipped rather than failing the whole channel.

**Phase B — message import** (fetches and persists the messages):
- Cron `MessagingMessagesImportCronJob` (`*/1 * * * *`) -> `MessagingMessagesImportJob` on `messaging-queue`.
- `MessagingMessagesImportService.processMessageBatchImport`:
  1. `SPOP`s up to `MESSAGING_MESSAGES_GET_BATCH_SIZE` (400) ids from `messages-to-import:{ws}:{channelId}` (`cacheStorage.setPop`),
  2. `messagingGetMessagesService.getMessages` -> raw provider messages,
  3. maps external folder ids to internal `messageFolder` ids,
  4. resolves the connected account's `handleAliases` (`email-alias-manager`),
  5. loads the blocklist for the workspace member,
  6. `filterEmails(channelHandle, aliases, messages, blocklistHandles, excludeGroupEmails, isInternalImportEnabled)` drops internal mail, blocklisted senders, group emails and unsubscribes,
  7. `messagingSaveMessagesAndEnqueueContactCreationService.saveMessagesAndEnqueueContactCreation` persists everything in **one workspace transaction**, then
  8. marks the channel `MESSAGES_IMPORT_PENDING` again (more batches) or `markAsMessageSyncCompleted` (last batch) and resets `throttleFailureCount`.

**Persistence** (`messaging-save-messages-and-enqueue-contact-creation.service.ts`) — inside one transaction:
- `messageService.saveMessagesWithinTransaction` -> `message` + `messageThread` + `messageChannelMessageAssociation`;
- `messageFolderAssociationService.saveMessageFolderAssociations` (mode `replace` refreshes folder links; moved associations are skipped);
- `messageParticipantService.saveMessageParticipants`;
- `messageAttachmentService.saveMessageAttachments` and `saveMessageRemoteAttachments` (remote attachments are fetched lazily).
After the transaction:
- **contact creation** — participants flagged `shouldCreateContact` are enqueued as `CreateCompanyAndContactJob` on `contact-creation-queue` (payload `{workspaceId, connectedAccount, contactsToCreate, source: FieldActorSource.EMAIL}`) when `messageChannel.isContactAutoCreationEnabled`.
  `shouldCreateContact` is false for drafts, for the connected account itself/its aliases, for non-professional emails (when excluded), and for group emails (when excluded). The policy is `SENT_AND_RECEIVED` or `SENT` (the latter requires the message to have been sent by the connected account).
- **participant matching** — `messageParticipantService.matchMessageParticipants(..., matchWith: 'workspaceMemberAndPerson')` for email channels; `APP` channels use `targetsOnly` or `personOnly` with the declared `personHandleField` (never matching a non-email handle as an email).

**Inbound email** (a separate driver, `drivers/inbound-email/`): messages arrive by e-mail to a per-account address; sources are `resend-inbound-email-message-source.service.ts` and `ses-s3-inbound-email-message-source.service.ts` (S3 + SES inbound). `MessagingInboundEmailImportJob` imports them; the local part is prefixed (`inbound-email-local-part-prefix.constant.ts`) with random bytes.

**Outbound**: `MessagingMessageOutboundService.sendMessage/createDraft/sendDraft` dispatches on `connectedAccount.provider` to Gmail / Microsoft Graph / IMAP-SMTP / email-group drivers (`modules/messaging/message-outbound-manager/drivers/*`).

**Sync webhooks (push)**:
- `modules/connected-account-sync-webhooks/connected-account-sync-webhooks.controller.ts`:
  - `POST /webhooks/google/messaging` — Google Pub/Sub push payload; `POST /webhooks/google/calendar` — Google Calendar channel notification (`x-goog-channel-id`, `x-goog-resource-state`, `x-goog-channel-token`).
  - `POST /webhooks/microsoft/messaging` and `/webhooks/microsoft/calendar` — Graph notification; the `validationToken` query param is echoed back as `text/plain` (HTML-escaped) for the Graph validation handshake.
- Subscription lifecycle: `modules/connected-account/webhook-subscription-manager/` — `create-webhook-subscription.job`, `renew-webhook-subscription.job`, `revoke-webhook-subscription.job`, `sync-workspace-webhook-subscriptions.job` (all on `webhook-queue`); the `WebhookSubscriptionRenewalCronJob` (`0 * * * *`) renews expiring subscriptions in batches, with a buffer and spacing (`WEBHOOK_SUBSCRIPTION_RENEWAL_*` constants) and a claim TTL (`WEBHOOK_SUBSCRIPTION_CLAIM_STALE_MS`).

**Staleness / relaunch**: `MessagingOngoingStaleCronJob` (`0 * * * *`) finds channels stuck in an ONGOING stage beyond `MESSAGING_IMPORT_ONGOING_SYNC_TIMEOUT` (30 min) and fails them; `MessagingRelaunchFailedMessageChannelsCronJob` (`*/30 * * * *`) restarts failed channels. Throttling: `MESSAGING_THROTTLE_DURATION = 60_000`, `MESSAGING_THROTTLE_MAX_ATTEMPTS = 5`.

### 3.3 Calendar sync

Same two-phase shape (`modules/calendar/calendar-event-import-manager/`):
- `CalendarEventListFetchCronJob` (`*/5 * * * *`) -> `CalendarEventListFetchJob` (`calendar-queue`) -> `CalendarFetchEventsService.fetchCalendarEvents`:
  - `getCalendarEventsService.getCalendarEvents({connectedAccount, syncCursor, recurringEventHorizon})` -> `{calendarEventIds, calendarEventIdsToDelete, recurringEventExternalIdsBeyondHorizon, nextSyncCursor}`;
  - deletes the `calendarChannelEventAssociation` rows of removed events and cleans orphaned events (`calendarEventCleanerService.deleteOrphanedCalendarEvents`);
  - `SADD`s the ids to import into `calendar-events-to-import:{ws}:{calendarChannelId}`;
  - stores `nextSyncCursor` on the channel.
- `CalendarEventsImportCronJob` (`*/1 * * * *`) -> `CalendarEventsImportJob` -> `calendar-import-events.service.ts` / `calendar-save-events.service.ts` (batch size `CALENDAR_EVENT_IMPORT_BATCH_SIZE = 100`).
- Drivers: `google-calendar/` (Google Calendar API), `microsoft-calendar/` (Graph), `caldav/` (`tsdav` 2.2.1 — `caldav-fetch-events.service.ts` uses RFC4791 `sync-token` and `REPORT`; `caldav-sync-cursor.type.ts` is a JSON cursor; `parse-ical-event.util.ts` parses iCalendar).
- Recurring events: `CALENDAR_RECURRING_EVENT_HORIZON_MS = 90 days`; instances beyond the horizon are expanded through `CalendarRecurringEventWakeUpService` (a `pendingWakeUp` with owner type `CALENDAR_RECURRING_EVENT`).
- Creation (outbound): `modules/calendar/calendar-event-creation-manager/drivers/{caldav,google-calendar,microsoft-calendar}`.

**IMAP/SMTP/CalDAV connection** (`engine/core-modules/imap-smtp-caldav-connection/`): `connectionParameters` schema with `IMAP`, `SMTP`, `CALDAV` sections, each `{host, port, username, password, secure}`; `EmailConnectionSecurity` enum; TLS options built by `build-imap-tls-options.util.ts` / `build-smtp-tls-options.util.ts`; a validator service tests the connection before saving (`imap-smtp-caldav-connection-validator.service.ts`).

### 3.4 Participant matching (people / companies / members)

`modules/match-participant/match-participant.service.ts` is shared by messaging and calendar.

- `matchWith` modes: `workspaceMemberOnly`, `personOnly`, `workspaceMemberAndPerson`, `targetsOnly`.
- People are matched by **primary email first, then additional emails**, case-insensitively (`utils/find-person-by-primary-or-additional-email.ts`); for app channels with a declared `personHandleField`, by canonical handle (`person-handle-field/utils/find-person-by-canonical-handle.util.ts`).
- Workspace members are matched by `userEmail` only.
- Participants are processed in chunks of 200; only rows whose `personId`/`workspaceMemberId` actually changed are updated.
- `ParticipantTargetReconciliationService.reconcileParticipantTargets` maintains the "targets" (company) relations derived from the person.
- Re-matching is event-driven: `modules/match-participant/listeners/participant-target-person.listener.ts` and `modules/messaging/message-participant-manager/listeners/message-participant-{person,workspace-member}.listener.ts` enqueue `MessageParticipantMatchParticipantJob` (`messaging-queue`) / `CalendarEventParticipantMatchParticipantJob` (`calendar-queue`) when a person's emails or a member's email change.

**Contact/company creation** (`modules/contact-creation-manager/`): `CreateCompanyAndContactJob` (`contact-creation-queue`) -> `CreateCompanyAndPersonService`:
- dedupes contacts (`getUniqueContactsAndHandles`), filters out contacts that are self or workspace members (`filter-out-contacts-that-belong-to-self-or-workspace-members.util.ts`),
- derives the company from the email domain (`get-domain-name-from-handle.util.ts`, `get-company-name-from-domain-name.util.ts`) and the person's name from the local part / display name (`get-first-name-and-last-name-from-handle-and-display-name.util.ts`, `get-parsed-name-from-email-local-part.util.ts`),
- creates or **restores** (soft-deleted) people, and creates companies for work domains (`isWorkDomain`).

### 3.5 Timeline

`modules/timeline/` — the activity feed.
- `UpsertTimelineActivityFromInternalEvent` (`entity-events-to-db-queue`) consumes `WorkspaceEventBatch<ObjectRecordNonDestructiveEvent>` and calls `timelineActivityService.upsertEvents`.
- `TimelineActivityRoutingPlanService.shouldProcessEvent` decides whether an object/action produces a timeline activity.
- `timeline-activity-type-cache.service.ts` resolves the activity type from the rules; `timeline/utils/resolve-timeline-activity-{type,rule-action,routing}.util.ts` implement the resolution.
- Fields that are not audit-logged are excluded (`exclude-non-audit-logged-fields-from-events.util.ts`), restricted fields are omitted (`omit-restricted-fields-from-timeline-activity-event.util.ts`), and linked records get a cached name (`resolve-linked-record-cached-name.util.ts`).
- `build-timeline-activity-merge-key.util.ts` + `build-linked-timeline-activity-happens-at-sync-updates.util.ts` dedupe/merge activities; `LinkedTimelineActivityDeletionJob` removes them when the source record is deleted.

**Porting difficulty: HIGH** (8/10) for messaging, **MEDIUM-HIGH** (7/10) for calendar.
The hard parts are the provider APIs (Gmail history vs Graph delta vs IMAP UID), the two-phase cache-backed pipeline with throttling and stale detection, the MIME/HTML quote-stripping utilities (`strip-quoted-history.util.ts`, `remove-caret-quoted-fragments.util.ts`, `describe-caret-quoting.util.ts` — Gmail-style caret quoting), the encrypted credential storage, and the transactional save + contact creation + participant matching chain.
**Best crates**: `reqwest` (HTTP/2 + oauth), `async-trait`, `serde`/`serde_json`, `sqlx` (transactions), `tokio`, `mail-parser` (MIME parsing — the TS side uses `mailparser`), `html5ever`/`scraper` (HTML → text, quote stripping), `chrono`, `uuid`, `sha2`/`hmac`/`base64` (signature + encryption), `aes-gcm` (the `enc:v2:` credential format), `tokio-rustls`/`rustls`, `quick-xml` (iCalendar), `ical` (no mature crate — parse `.ics` with `quick-xml` + `chrono` or use `icalendar`/`ical-parser`), `imap` (async IMAP — `imap` crate is sync, prefer `async-imap` or drive it over `tokio::process`), `lettre` (SMTP), `msgraph-sdk` (community) or raw `reqwest` against Graph, `google-apis-*` (community) or raw `reqwest`.

---

## 4. AI: agents, chat, tools, model providers

### 4.1 Model providers

`engine/metadata-modules/ai/ai-models/`:
- `ai-providers.json` — the catalog of providers: `openai` (`@ai-sdk/openai`), `anthropic` (`@ai-sdk/anthropic`), `google` (`@ai-sdk/google`), `mistral` (`@ai-sdk/mistral`), `xai` (`@ai-sdk/xai`), `typesafe-ai` (`@ai-sdk/typesafe-ai`).
- `ai-models.json` — per-model metadata: `inputCostPerMillionTokens`, `outputCostPerMillionTokens`, `cachedInputCostPerMillionTokens`, `cacheCreationCostPerMillionTokens`, `longContextCost {thresholdTokens, …}`, `contextWindowTokens`, `maxOutputTokens`, `modalities` (`image`, `pdf`), `supportsReasoning`, plus a `benchmark` block (`intelligenceIndex`, `outputTokensPerSecond`, `costPerTask`, `effort`, `measuredAt`).
- `ai-evaluation-models.json`, `ai-model-benchmarks.json`, `ai-self-host-spec.json`.
- `ai-sdk-package.constant.ts` — the full set of SDK packages: `openai`, `anthropic`, `google`, `mistral`, `xai`, `amazon-bedrock`, `openai-compatible`, `azure`, `typesafe-ai`.
- `sdk-provider-factory.service.ts` — builds a provider instance per provider name (cached in a `Map`), choosing the factory by `config.npm`; Bedrock supports API key or role credentials (`fromNodeProviderChain`), Azure and OpenAI-compatible are supported. Gemini gets a middleware (`sanitize-gemini-tool-result-refs.middleware.ts`).
- `ai-model-registry.service.ts` — `RegisteredAiModel {modelId, sdkPackage, model, supportsReasoning, effort, providerName, modelsDevName}`; keeps separate registries for language, **transcription** and **evaluation** models (transcription models never reach the chat picker or token costing).
- `ai-model-preferences.service.ts`, `provider-config.service.ts`, `workspace-ai-data-policy.service.ts` (`isAiModelCompliantWithDataPolicy`, `NO_AI_DATA_POLICY`).
- `models-dev-catalog.service.ts` pulls the models.dev catalog; `buildCompositeModelId` / `parseAiModelVariantId` encode tier+effort into the model id (`AUTO_SELECT_WORKSPACE_DEFAULT_MODEL_ID` = auto-select).

### 4.2 The step loop (the actual agent loop)

`engine/metadata-modules/ai/ai-agent-execution/services/agent-step-loop.service.ts` — wraps the Vercel AI SDK's `streamText` with:
- a `ToolSet`, `toolChoice: 'auto' | 'required'`, optional `output` schema, `abortSignal`, and a `StopCondition`;
- **pausing tools**: `pausing-tools/` — `ask-question`, `ask-questions`, `request-form`, `propose-tool-call`; a pausing tool ends the turn and the run is *suspended* (`AgentRunSuspensionService`) until the awaited input arrives;
- **tool repair**: `ai-agent/utils/repair-tool-call.util.ts` fixes malformed tool calls;
- **billing** per step (`AiBillingService`, `StepDebitResult`), incl. web-search operations;
- **metrics** per scope (`chat` vs `agent`) — success/failure counts, duration histograms, output-token histograms;
- **cache breakpoints** (`provider-options.util.ts`: `getCacheProviderOptions`, `injectCacheBreakpoint`) for Anthropic prompt caching;
- stop conditions and `NoOutputGeneratedError` are treated as normal loop ends.

### 4.3 Agent runs

`ai-agent-execution/services/agent-runner.service.ts`:
- `start(input)` enqueues `RunAgentJob` on `ai-queue` with a generated `turnId`; `runInRequest(...)` runs it in-request for API calls that wait.
- A run can be **suspended** and later **continued** (`ContinueAgentRunJob`), up to `AGENT_RUN_MAX_CONTINUATIONS = 50`; a run that waits is resumed through `pendingWakeUp` (`AgentRunPendingWakeUpHandlerService`).
- `RUN_AGENT_MAX_WAIT_MS = 15 * 60_000`; settled runs are reconciled every `RUN_AGENT_SETTLED_RUN_RECONCILE_INTERVAL_MS = 10_000` with a TTL of `RUN_AGENT_SETTLED_RUN_TTL_SECONDS = 3_600` (pub/sub can drop notices during a reconnect, so the stored settlement is re-read).
- `DeliverAgentRunOutcomeJob` (`ai-queue`) delivers the outcome back to the caller (a workflow step, an API call, a chat thread).
- Agent entities: `ai-agent/entities/agent.entity.ts` — `name`, `label`, `icon`, `avatarColor`/`avatarShape`, `description`, `prompt`, `modelId` (default auto-select), `responseFormat` (JSONB), `modelConfiguration` (JSONB), `isSystem`, `isCustom` (removed in 2-54), `deletedAt`; `OverridableEntity` lets an app override label/icon/prompt/model.
- Agent triggers (`twenty-shared/src/application/agentTriggerType.ts`): `DATABASE_EVENT` (settings `{eventName, updatedFields?, batchMode?, condition?, conditionBindings?}`) and `CRON` (`{pattern}`); the cron trigger runs on `AgentCronTriggerCronJob` (`* * * * *`).
- The `AI_AGENT` workflow step (`modules/workflow/workflow-executor/workflow-actions/ai-agent/ai-agent.workflow-action.ts`) validates the config (`getWorkflowAiAgentStepConfigIssues`), resolves the prompt through the variable resolver, requires the recipient to be a workspace member (uuid), and hands the run to the agent engine, which later calls the step back via `awaitedStepOutput`.

### 4.4 AI chat (the inbox)

`engine/metadata-modules/ai/ai-chat/`:
- Standard objects (`ai-history/standard-objects/`): `agentChatThread`, `agentChatThreadParticipant`, `agentChatThreadTarget`, `agentMessage`, `agentMessagePart`, `agentMessageReaction`, `agentTurn`.
- Streaming: `StreamAgentChatJob` on `ai-stream-queue` (concurrency 20, `lockDuration` 10 min, `maxStalledCount: 0`). It **claims** the thread (`AgentChatStreamClaimService`) — a job whose `streamId` is no longer the thread's `activeStreamId` exits immediately — then runs the turn (`AgentTurnStreamService.runClaimed`) with the job's `abortSignal`.
- `FlushQueuedAgentChatMessagesJob` (`ai-stream-queue`) sends messages that arrived while a run held the conversation.
- Snooze jobs: `EndAgentChatChannelSnoozeJob`, `EndAgentChatThreadSnoozeJob` (on `delayed-jobs-queue`).
- `RewriteAgentChatChannelGrantsJob` (`workspace-queue`) recomputes channel grants.
- Assistant policy services: `agent-chat-reply-policy.service.ts`, `agent-chat-assistant-participation.service.ts`, `agent-chat-reply-continuation-policy.service.ts`, `agent-chat-turn-preflight.service.ts`.
- Chat tools: `ai-chat/tools/` — `ask-question`, `request-form`, `react-to-message`, `attach-conversation-to-records`, `complete-workspace-setup`.
- System prompt: `system-prompt-builder.service.ts`; title generation: `agent-title-generation.service.ts`; message pruning: `message-pruning.service.ts`.

### 4.5 Tool system

Two layers:
- **Defined tools** (`engine/core-modules/tool/tools/`): `calendar-tool`, `code-interpreter-tool`, `email-tool`, `file-upload-tool`, `http-tool`, `output-navigation-tool`, `read-file-tool`, `search-help-center-tool`, `send-email-tool`, `share-record-tool`. Each implements `Tool = {description, inputSchema, execute(input, ctx), flag?}` (`engine/core-modules/tool/types/tool.type.ts`).
- **Providers** (`engine/core-modules/tool-provider/providers/`): `action-tool`, `database-tool`, `workflow-tool`, `dashboard-tool`, `front-component-tool`, `logic-function-tool`, `mcp-tool`. Each has a `category` (`ToolCategory`), `isAvailable(context)`, `generateDescriptors(context, options)`, `executeStaticTool(...)` and/or `executeMcpServerTool(...)`.
- `ToolRegistryService` (`tool-provider/services/tool-registry.service.ts`) merges provider descriptors + `DefinedToolRegistryService` descriptors, resolves schemas lazily per category, and dispatches execution (`ToolExecutorService`); outputs are normalized and spilled (`ToolOutputSpillService`) when large.
- Meta-tools (`tool-provider/tools/`): `get-tool-catalog`, `learn-tools`, `execute-tool`, `load-skill` — the agent first learns the catalog, then executes.
- Excluded from agent runs: `AGENT_RUN_EXCLUDED_TOOL_NAMES = ['search_help_center', 'create_file_upload', 'complete_file_upload', 'share_record']`.

### 4.6 Other AI surfaces
- `ai-generate-text/controllers/ai-generate-text.controller.ts` — deprecated `POST /rest/ai/generate-text` (guarded by `PermissionFlagType.AI`); the replacement is the `generate` GraphQL mutation or `ai.generate` in a logic function.
- `ai-front-component/` — compiles React front components in a browser-like builder (`front-component-compiler.service.ts`).
- `ai-evaluation/`, `ai-billing/` (per-step debits, web-search operation types), `ai-workspace-stats/`, `ai-history/`.

**Porting difficulty: HIGH** (8/10).
The agent loop, the AI-SDK provider abstraction, the streaming/claim protocol and the billing/metrics bookkeeping are all intricate; the Vercel AI SDK has **no Rust equivalent**, so you must re-implement streaming tool-calling, provider quirks (Gemini tool-result sanitising, Anthropic cache breakpoints, reasoning-effort variants), and the pause/resume protocol.

**Best crates**: `reqwest` + `eventsource-stream` or `reqwest-eventsource` (SSE), `async-trait`, `serde`/`serde_json`, `tokio` (`JoinSet`, `CancellationToken`), `futures` (`StreamExt`), `tokio-util`, `sqlx`, `uuid`, `chrono`, `dashmap` (the model registry cache), `tracing`, `opentelemetry` (telemetry parity).
Provider clients: `async-openai` (OpenAI-compatible), `anthropic` (official Rust SDK), `async-google-cloud`/`google-generative-ai`, `mistralai`, `xai-sdk` (or raw `reqwest` for all of them). MCP: `rmcp` (the official Rust MCP SDK) — it matches the `@modelcontextprotocol/sdk` 1.32.2 used by Twenty.

---

## 5. File storage, email, webhooks, MCP, serverless apps / logic functions

### 5.1 File storage

**Drivers** (`engine/core-modules/file-storage/`):
- `StorageDriverType = 'LOCAL' | 'S_3'` (`interfaces/file-storage.interface.ts`); `FileStorageDriverFactory` picks by `STORAGE_TYPE` and caches by a config hash (`file-storage-driver.factory.ts`).
- `drivers/local.driver.ts` — filesystem under `STORAGE_LOCAL_PATH` (default `.local-storage`); guards path traversal with `realpathSync` + `startsWith(root + sep)`; the ETag substitute is `${ino}-${size}-${mtimeMs}`.
- `drivers/s3.driver.ts` — `@aws-sdk/client-s3` + `@aws-sdk/lib-storage` (multipart `Upload`) + `@aws-sdk/s3-request-presigner`; separate `s3Client` and `metadataClient` with different timeouts (`constants/s3-client-timeouts.constant.ts`: request 5 min, connection 30 s, max sockets 200; metadata request 15 s, connection 5 s, 2 attempts, 50 sockets); `forcePathStyle: true`; credentials from `STORAGE_S3_ACCESS_KEY_ID`/`SECRET` or the node provider chain.
- `drivers/validated-storage.driver.ts` — a decorator that calls `assertStoragePathIsSafe` on **every** operation.
- Path safety: `utils/is-safe-relative-path.util.ts` (rejects empty, NUL, absolute, backslashes, `..` segments after normalize), `utils/validate-file-path.util.ts` (relative + per-segment safety + extension required + extension allow-list), `utils/validate-storage-path-is-within-workspace-or-throw.util.ts`.
- `StorageDriver` interface (`drivers/interfaces/storage-driver.interface.ts`): `readFile` (with `ByteRange`), `readFilePrefix`, `writeFile`, `writeFileStream`, `getFileMetadata`, `downloadFile`, `delete`, `move` (with `ifMatchChecksum`), `copy`, `checkFileExists`, `checkFolderExists`, `getPresignedUrl`, `getPresignedUploadUrl`.

**Upload flow** (`engine/core-modules/file/file-upload/`):
- `FileUploadService.createFileUpload` -> validates the folder is in `DIRECT_UPLOAD_FILE_FOLDERS`, validates the size, rejects SVG over `MAX_SANITIZABLE_SVG_BYTES`, creates the `FileEntity` row with status `PENDING` and mime `application/octet-stream` (the DB has a CHECK that a PENDING file has that mime type), then returns either a presigned PUT URL (S3) or a signed token for the streaming endpoint.
- `FileUploadController` `PUT /file-upload/:id` streams the body straight to the storage driver (never buffered) when presigned upload is unavailable (`FileUploadTokenGuard`).
- `FileUploadCompletionService` sniffs the real content type, sanitizes SVGs, moves the file out of quarantine and flips the status to `ACTIVE`.
- Folders (`file/constants/supported-file-folders.constant.ts`): `CorePicture`, `FilesField`, `Workflow`, `AgentChat`, `EmailAttachment`, `EmailImage`, `AppTarball`, `Dpa`, `WorkspaceDataExport`. `AppTarball` and `CorePicture` have dedicated, permission-gated completions.
- Signed download URLs: `file/file-url/file-url.service.ts` signs a JWT (`JwtTokenTypeEnum`) bound to `{workspaceId, fileFolder, fileId}`; `FileController` streams the file after `FileByIdGuard` + `CustomPermissionGuard`.
- `file-remote-content/` stores remote attachments (e.g. a link-only attachment) and `file-email-attachment/` handles mail attachments.
- Cleanup: `PendingFileCleanupCronJob` (`0 * * * *`) removes abandoned PENDING uploads.
- Quotas: `UsageLimitStockService` charges storage stock per byte/file (`UsageResourceType.STORAGE`, `UsageOperationType.STORAGE_FILE`), released via `releaseStorageStock`.

### 5.2 Email sending (SMTP)

`engine/core-modules/email/`:
- `EmailService.send(sendMailOptions)` enqueues `EmailSenderJob` on `email-queue` with `retryLimit: 3` and `EMAIL_SEND_RETRY_BACKOFF = {strategy:'exponential', initialDelayMilliseconds: 5_000, jitter: 0.5}` (`constants/email-send-retry-backoff.constant.ts`); the job is `{isSystemJob: true}` (no workspace scope).
- `EmailSenderJob` -> `EmailSenderService` -> `EmailDriverFactory.getCurrentDriver()`.
- Drivers: `drivers/smtp.driver.ts` (`nodemailer` `createTransport({pool: true})`), `drivers/logger.driver.ts` (dev). `EMAIL_DRIVER` = `SMTP | LOGGER`; SMTP config: `EMAIL_SMTP_HOST`, `EMAIL_SMTP_PORT` (587), `EMAIL_SMTP_USER`, `EMAIL_SMTP_PASSWORD`, `EMAIL_SMTP_NO_TLS`, `EMAIL_SMTP_NAME` (`email-driver.factory.ts`).
- Body compilation: `utils/compile-outbound-email-content.util.ts` — the body is a JSON `EmailDocument`; `reactMarkupFromJSON` + `render` (React Email, `twenty-emails` package) produce HTML, then `sanitizeOutboundEmailHtml`, then `toPlainText(html)` for the text part. `getFullHtmlEmailIfWholeBody` short-circuits when the document is already a full HTML email.
- Recipients: `utils/parse-comma-separated-emails.util.ts`, `utils/get-email-recipients-error.util.ts`; bindings: `utils/resolve-email-document-bindings.util.ts`.
- Sending domains / verification: `engine/core-modules/emailing-domain/` (SPF/DKIM/DMARC verification, `CheckEmailingDomainVerificationCronJob` hourly).
- Campaigns (bulk email) have their own queues: `campaign-queue`, `campaign-send-queue`, `campaign-engagement-queue` (`modules/emailing/jobs/*`: `materialize-campaign`, `send-campaign-email`, `send-campaign-email-batch`, `record-campaign-engagement`, `refresh-campaign-stats`, `reconcile-workspace-campaign-stats`, `recover-workspace-stale-campaigns`).

### 5.3 Outbound webhooks (delivery + signature)

`engine/metadata-modules/webhook/`:
- `WebhookEntity` (`core.webhook`): `targetUrl`, `operations: string[]` (default `['*.*']`), `description`, `secret` (NOT NULL), `workspaceId`.
- Matching: `utils/find-webhooks-matching-event-name.util.ts` splits the event name `object.action` and matches against `computeWebhookOperationsToMatch(...)` (so `person.*` matches `person.created`, `*.*` matches everything).
- Pipeline: DB event -> `EntityEventsToDbListener` -> `webhook-queue` `CallWebhookJobsJob` -> per-webhook permission filtering (`RecordEventPermissionsReadService.filterReadableEvents` with the webhook as the auth principal) -> rate limiting (`WebhookRateLimitService.admitWebhookEventsWithinRateLimit`, a token bucket per workspace under `UsageResourceType.WEBHOOK` / `UsageOperationType.WEBHOOK_CALL`) -> events are chunked by 20 into `CallWebhookJob` jobs.
- Payload per event (`utils/transform-event-batch-to-webhook-events.ts` -> `CallWebhookJobData`): `{targetUrl, eventName, objectMetadata:{id,nameSingular}, workspaceId, webhookId, eventDate, userId?, workspaceMemberId?, applicationId?, record, updatedFields?, secret}`.
- **Signature** (`jobs/call-webhook.job.ts`):
  ```
  X-Twenty-Webhook-Timestamp: Date.now().toString()
  X-Twenty-Webhook-Signature: hex( HMAC-SHA256(secret, `${timestamp}:${JSON.stringify(payloadWithoutSecret)}`) )
  X-Twenty-Webhook-Nonce: hex(16 random bytes)
  ```
  The `secret` is stripped from the body before signing and before sending. The request is `POST` with `Content-Type: application/json`, timeout 5 s, response streamed and immediately destroyed. SSRF protection comes from `SecureHttpClientService` (private/internal IPs blocked -> `error: 'Webhook URL resolves to a private/internal IP address'`).
- Metadata webhooks: `jobs/call-webhook-jobs-for-metadata.job.ts` (workspace metadata events).
- Responses are recorded as events (`WEBHOOK_RESPONSE_EVENT`) and metrics (`JobWebhookCallCompleted`, `JobWebhookCallThrottled`).

### 5.4 MCP server (Twenty acting as an MCP server)

`engine/api/mcp/`:
- `controllers/mcp-core.controller.ts` — `POST {ApiPath.Mcp}` (JSON-RPC 2.0). Guards: `McpAuthGuard`, `AccessPolicyGuard` (accepts user sessions incl. impersonated/playground, API keys, OAuth clients, applications with/without a user, and agents), `WorkspaceNotSuspendedGuard`. If the body has no `id` it is a **notification** -> HTTP 202 with no body; if `Accept` includes `text/event-stream` the response is an **SSE stream**; otherwise a plain JSON-RPC response.
- `services/mcp-protocol.service.ts` — implements `initialize` (returns `protocolVersion`, `capabilities {tools, resources, prompts}` with `listChanged: false`, `serverInfo`, and generated `instructions`), `tools/list`, `tools/call`. Tool annotations distinguish read-only/open-world vs closed-world vs destructive (`constants/mcp-*-tool-annotations.constant.ts`).
- Tool set comes from the `ToolRegistryService` (section 4.5) plus MCP-specific tools: `tools/generic-record-tools.tool.ts`, `tools/list-object-metadata-names.tool.ts`, `tools/list-skills.tool.ts`, plus the meta-tools `get_tool_catalog`, `learn_tools`, `execute_tool`, `load_skill`.
- `toolMode` comes from the `?mode=` query param (`utils/parse-mcp-tool-mode.util.ts`); `MCP_GENERIC_MODE_LISTED_TOOL_CATEGORIES` restricts what is listed in generic mode.
- Constants: `constants/mcp-protocol-version.const.ts`, `mcp-server-info.const.ts`, `mcp-methods.const.ts`, `json-rpc-error-code.const.ts`.

**Twenty as an MCP *client*** (agents calling third-party MCP servers): `engine/core-modules/tool-provider/providers/mcp-tool.provider.ts` + `engine/metadata-modules/mcp-server/services/{mcp-server-client,mcp-server-tool-sync,mcp-server-tool-call}.service.ts`; the manifest is `McpServerManifest {name, label, url, connectionProvider?}` (`twenty-shared/src/application/mcpServerManifestType.ts`); tools are synced on demand (`CATALOG_TOOL_SYNC_TIMEOUT_MS = 5_000`) and destructive tools must be proposed and approved (`PROPOSE_TOOL_CALL_TOOL_NAME`).

### 5.5 Logic functions (serverless code execution)

`engine/core-modules/logic-function/`:
- `logic-function-drivers/interfaces/logic-function-driver.interface.ts` — `execute`, `transpile`, `buildDependencies`, `installPrebuiltBundle`, `getInstalledBundleChecksum`, `delete`, `deleteApplicationResources`, `executeHttpRequest`.
- `LogicFunctionDriverType = DISABLED | LAMBDA | LOCAL` (`LOGIC_FUNCTION_TYPE` env).
  - **LOCAL** (`drivers/local/local.driver.ts`): esbuild bundles the TS/JS source (`platform: node`, `format: esm`, `target: esnext`, `bundle: true`, `packages: 'external'`) into a temp dir, installs dependencies with yarn (`constants/yarn-install/`), and runs it as a **child process** (`LocalChildProcessRunnerService`). Layers are cached (`LocalLayerManagerService`, `LocalPrebuiltBundleService`).
  - **LAMBDA** (`drivers/lambda/lambda.driver.ts`): `@aws-sdk/client-lambda` — creates functions, publishes layers (zip uploaded to S3), invokes them, parses `LogType` results, and maps timeouts (`buildLogicFunctionTimeoutResult`).
  - **DISABLED**: HTTP egress only.
- The executor: `logic-function-executor/logic-function-executor.service.ts` — `LogicFunctionExecuteResult {data, duration, billedDurationMs, logs, status, error?}`; throttled by `logic-function-execution-throttle` (`LOGIC_FUNCTION_EXEC_THROTTLE_LIMIT = 1000`, `LOGIC_FUNCTION_EXEC_THROTTLE_TTL = 60_000`).
- Triggers: `logic-function-trigger/triggers/{cron,database-event,route}` — the route trigger (`triggers/route/route-trigger.service.ts`) matches a path against the function's `httpRouteTriggerSettings.path`/`httpMethod`/`isAuthRequired`/`forwardedRequestHeaders`, resolves the workspace by host or token, and runs the function inline.
- Jobs: `logic-function-trigger/jobs/logic-function-trigger.job.ts` on `logic-function-queue` (concurrency 20, retry backoff `LOGIC_FUNCTION_QUEUE_RETRY_BACKOFF = {exponential, 1_000 ms, jitter 0.5}`); `application-lifecycle-hook.job.ts` on `application-lifecycle-hook-queue` (pre/post install/uninstall hooks); `logic-function-prebuilt-warm-up/jobs/warm-up-logic-function.job.ts` on `lambda-management-queue`.
- `logic-function-trigger/logic-function-job-runner.service.ts` — `LOGIC_FUNCTION_APPLICATION_RETRY_LIMIT = 3`; the retry count is persisted in the job data (`applicationRetryCount`) so a redelivered job does not restart the retry budget; disabled/suspended/not-found/usage-refused/oversized-dependency errors are swallowed (they must not retry).

### 5.6 Applications / the app runtime (twenty-sdk apps)

- The **manifest** (`twenty-shared/src/application/manifestType.ts`): `{application, objects[], fields[], indexes?, logicFunctions[], frontComponents[], permissionFlags[], roles[], skills[], agents[], connectionProviders?, messagingProviders?, mcpServers[], publicAssets[], sourceFiles?, views[], viewFields[], navigationMenuItems[], pageLayouts[], pageLayoutTabs[], pageLayoutWidgets[], commandMenuItems[], timelineActivityTypes[], settingsMenuItems[], validationRules?, relatedRecordPaths?, translations?, workflows?}`.
- `LogicFunctionManifest` (`logicFunctionManifestType.ts`): `{name?, description?, timeoutSeconds?, cronTriggerSettings?, eventTriggerSettings?, httpRouteTriggerSettings?, serverRouteTriggerSettings?, toolTriggerSettings?, workflowActionTriggerSettings?, sourceHandlerPath, sourceFiles?, builtHandlerPath, builtHandlerChecksum, handlerName}`.
- `AgentTriggerManifest` (`agentTriggerType.ts`): `EVENT` (with `EventTriggerManifestSettings`), legacy `DATABASE_EVENT`, or `CRON`.
- The SDK (`packages/twenty-sdk/src/sdk/`): `define/` helpers (`define-application`, `define-logic-function`, `define-workflow`, `define-mcp-server`, `define-agents`, `define-connection-providers`, `define-front-component`, `define-objects`, `define-fields`, `define-roles`, `define-skills`, `define-messaging-providers`, …); runtime helpers under `sdk/logic-function/` (`agents/`, `ai/`, `connections/`, `errors/`, `jobs/`, `key-value/`, `messaging/`, `response.ts`, `timeline/`, `utils/`); `sdk/inbox/`; `sdk/billing/`.
- **App package delivery** (`engine/core-modules/application/application-package/` + `application-registration/`): the app tarball is uploaded to `FileFolder.AppTarball`, extracted securely (`utils/extract-tarball-securely.util.ts`), dependencies (`package.json` + `yarn.lock`) are stored under `FileFolder.Dependencies` and checksum-verified (`compute-application-package-checksums.util.ts`, `assert-matching-dependency-checksums-or-throw.util.ts`); the standard dependency set and the yarn engine are copied in (`constants/standard-application-dependencies/`, `constants/yarn-engine/`).
- The SDK client is generated per app: `engine/core-modules/sdk-client/jobs/generate-sdk-client.job.ts` (`workspace-queue`) + `sdk-client-archive.service.ts`; the SDK archive is a Lambda layer (`sdk-client-archive.service.ts`).
- App lifecycle jobs: `application-install/jobs/trigger-install-application.job.ts`, `trigger-uninstall-application.job.ts`, `application-upgrade/jobs/trigger-upgrade-application.job.ts`, `application/jobs/upgrade-applications.job.ts` (`application-upgrade-queue`), `pre-installed-apps/jobs/install-pre-installed-apps.job.ts` (`workspace-queue`).
- The manifest is applied to the workspace as *metadata* (objects, fields, views, roles, permission flags); the workspace engine then serves the app's workflows, agents, logic functions and MCP servers.
- App **jobs**: `twenty-shared/src/application/enqueueJobType.ts` — `enqueueJob` / `enqueueJobs` from a logic function, returning `{enqueued, logicFunctionUniversalIdentifier, jobId, enqueuedJobsCount, jobIds, jobs:[{jobId,sizeBytes}], availableBytes}`; `JobStatusState` = `COMPLETED|FAILED|ACTIVE|WAITING|DELAYED|PRIORITIZED|WAITING_CHILDREN`.

**Porting difficulty: HIGH** (8/10) overall for section 5, with per-item ratings:

| Item | Difficulty | Notes / best crates |
|---|---|---|
| File storage | MEDIUM (5/10) | `aws-sdk-s3` (Rust) or `rusty-s3`, `object_store` (unified API over S3/local), `tokio-util::io::StreamReader`, `sha2`, `mime_guess`, `sanitize-filename`, `tempfile`. Port the path-safety validators literally. |
| SMTP email | LOW-MEDIUM (4/10) | `lettre` (SMTP, with `lettre_tokio`), `tokio-rustls`. The React Email body compilation must be replaced (port `twenty-emails` templates or pre-render to HTML). |
| Outbound webhooks | LOW-MEDIUM (4/10) | `reqwest`, `hmac` + `sha2` + `hex` for the signature, `tokio::time` for the 5 s timeout, `ipnet` for the SSRF IP checks. Signature must match byte-for-byte: `HMAC-SHA256(secret, "{timestamp}:{JSON.stringify(payload)}")`. |
| MCP server | MEDIUM-HIGH (6/10) | `rmcp` (official Rust MCP SDK) implements JSON-RPC + SSE for you; `axum` for the HTTP layer; `serde_json` for the DTOs. |
| Logic functions (serverless) | VERY HIGH (9/10) | The product is *JavaScript execution* — porting it means shipping a JS runtime (`boa_engine`, `rquickjs`/`quickjs` via `rquickjs`, `deno_core`, or a Node sidecar). esbuild-equivalent bundling: `oxc`/`swc`/`esbuild-rs`; zipping: `zip`; AWS Lambda: `aws-sdk-lambda`; child processes: `tokio::process`. The pragmatic Rust answer is to keep a Node sidecar and drive it over HTTP/gRPC. |
| App runtime / SDK | HIGH (8/10) | The manifest model ports cleanly (`serde`), but the whole app ecosystem (manifest schema, tarball delivery, dependency installation, generated SDK client, per-app Lambda functions) is JS-centric. |

---

## 6. The event / listener system in detail (expands §1)

### 6.1 Production of database events

Events are produced by Twenty's **workspace ORM**, not by Postgres triggers:

- `engine/twenty-orm/repository/workspace-repository.ts` — `emitCreateEvents` (after insert) and `emitMutationEvents` (after update / soft-delete / restore / destroy).
- The action mapping (`workspace-repository.ts:99-107`):
  ```
  insert          -> [CREATED, UPSERTED]
  update          -> [UPDATED, UPSERTED]
  soft-delete     -> [DELETED]
  restore         -> [RESTORED]
  delete (destroy)-> [DESTROYED]
  ```
  (the code emits `CREATED`+`UPSERTED` for inserts, `UPDATED`+`UPSERTED` for updates — see `emitCreateEvents` at line 1402 and `MUTATION_EVENT_ACTIONS_BY_KIND` at line 99.)
- `engine/twenty-orm/utils/format-twenty-orm-event-to-database-batch-event.util.ts` builds the batch event from `recordsBefore`/`recordsAfter` + the flat field metadata (it computes `updatedFields` and the per-field `diff`).
- The emission can be suppressed with `shouldSkipEventEmission` (used by internal writes, e.g. the timeline activity creation).
- `engine/twenty-orm/datasource/workspace-data-source.ts:96-98` wires the emitter into the data source.

### 6.2 Event shapes

`twenty-shared/src/database-events/`:
- `object-record.base.event.ts`: `{recordId, userId?, userWorkspaceId?, workspaceMemberId?, properties: {updatedFields?, before?, after?, diff?}}`.
- `ObjectRecordCreateEvent` — `{after}`; `ObjectRecordUpdateEvent` — `{updatedFields, diff, before, after}`; `ObjectRecordDeleteEvent` — `{before, after, updatedFields, diff}`; `ObjectRecordDestroyEvent` — `{before}`; `ObjectRecordRestoreEvent` — extends create with `{before, after, updatedFields, diff}`; `ObjectRecordUpsertEvent`.
- `ObjectRecordDiff<T> = {[K in keyof T]: {before: T[K]; after: T[K]}}`.

### 6.3 The emitter and the bus

`engine/workspace-event-emitter/workspace-event-emitter.ts`:
- `emitDatabaseBatchEvent({objectMetadataNameSingular, action, events, objectMetadata, workspaceId})` builds the **event name** `` `${objectNameSingular}.${action}` `` (`utils/compute-event-name.ts`), wraps it into a `WorkspaceEventBatch {name, workspaceId, objectMetadata, events}` and emits it on Nest's `EventEmitter2`.
- `emitCustomBatchEvent(eventName, events, workspaceId)` for non-DB events.
- `DatabaseEventAction` enum (`engine/api/graphql/graphql-query-runner/enums/database-event-action.ts`): `created|updated|deleted|destroyed|restored|upserted`.

### 6.4 Listeners

`@OnDatabaseBatchEvent(object, action)` (`engine/api/graphql/graphql-query-runner/decorators/on-database-batch-event.decorator.ts`) is a thin wrapper over `@OnEvent(`${object}.${action}`)`. A listener registered for `'*'` receives every object for that action.

Registered listeners (from the `@OnDatabaseBatchEvent` scan):

| Listener | File | Purpose |
|---|---|---|
| `EntityEventsToDbListener` | `engine/api/graphql/workspace-query-runner/listeners/entity-events-to-db.listener.ts` | the **fan-out hub** (see below) |
| `WorkflowDatabaseEventTriggerListener` | `modules/workflow/workflow-trigger/automated-trigger/listeners/workflow-database-event-trigger.listener.ts` | workflow database-event triggers |
| `PendingWakeUpDatabaseEventListener` | `engine/core-modules/pending-wake-up/listeners/pending-wake-up-database-event.listener.ts` | wakes workflow steps waiting on an event |
| `BillingWorkspaceMemberListener` | `engine/core-modules/billing/listeners/billing-workspace-member.listener.ts` | seat billing |
| `GlobalWorkspaceMemberListener` | `engine/core-modules/user/services/global-workspace-member.listener.ts` | member sync |
| `FilesFieldDeletionListener` | `engine/core-modules/file/files-field/listeners/files-field-deletion.listener.ts` | file cleanup |
| `NavigationMenuItemDeletionListener` | `engine/metadata-modules/navigation-menu-item/listeners/…` | menu cleanup |
| `AccessGroupSyncListener` | `engine/permissions/internal/access-group/listeners/access-group-sync.listener.ts` | access groups |
| `AgentChatThreadLifecycleListener` | `engine/metadata-modules/ai/ai-chat/listeners/…` | AI chat threads |
| `MessageParticipantPersonListener` / `…WorkspaceMemberListener` | `modules/messaging/message-participant-manager/listeners/…` | participant re-matching |
| `MessageBlocklistListener` | `modules/messaging/blocklist-manager/listeners/…` | blocklist re-import |
| `MessagingMessageCleanerMessageListener` | `modules/messaging/message-cleaner/listeners/…` | cleanup |
| `CalendarBlocklistListener`, `CalendarEventCleanerCalendarEventListener`, `CalendarEventParticipant*Listener` | `modules/calendar/.../listeners/…` | calendar equivalents |
| `ParticipantTargetPersonListener` | `modules/match-participant/listeners/…` | company/target reconciliation |
| `WorkspaceMemberAvatarFileDeletionListener` | `modules/workspace-member/listeners/…` | avatar cleanup |

### 6.5 The fan-out hub

`EntityEventsToDbListener.handleEvent(batchEvent, action)` (`entity-events-to-db.listener.ts`) does, in parallel (`Promise.all`):

1. **Timeline activities** — if the action is not `DESTROYED` and `timelineActivityRoutingPlanService.shouldProcessEvent(...)` is true, enqueue `UpsertTimelineActivityFromInternalEvent` on `entity-events-to-db-queue` (retryLimit 1).
2. **Audit log** — if `objectMetadata.isAuditLogged` and the event-log sink is enabled, enqueue `CreateEventLogFromInternalEvent` on `event-log-queue` (retryLimit 1).
3. **Webhooks** — if any webhook matches the event name, enqueue `CallWebhookJobsJob` on `webhook-queue` (retryLimit 3).
4. **Logic functions** — if any logic function is triggered by the event name, enqueue `CallDatabaseEventTriggerJobsJob` on `trigger-queue` (retryLimit 3).
5. **Agents** — if any agent has a matching database-event trigger, enqueue `CallAgentDatabaseEventTriggersJob` on `trigger-queue` (retryLimit 3).
6. **Subscriptions** — `objectRecordEventPublisher.publish(batchEvent)` (GraphQL live subscriptions over `graphql-redis-subscriptions`).

Special case: events on the `timelineActivity` object itself are **not** turned into timeline activities (they are published straight to subscriptions).

Each match is looked up from the **workspace cache** (`WorkspaceCacheService.getOrRecompute(workspaceId, ['flatWebhookMaps'|'flatLogicFunctionMaps'|'flatAgentMaps'|'workflowAutomatedTriggerMaps'])`); a cache failure is treated as "there is a match" (fail-open, so an event is never silently dropped).

### 6.6 Trigger matching rules

- `utils/is-database-event-trigger-listening-to-event.util.ts` -> `computeTriggerEventNamesMatchingEvent(eventName)` expands `person.created` into the set of trigger event names that should match it (e.g. `person.created`, `person.*`, `*.*`).
- `utils/filter-events-by-updated-fields.util.ts` narrows the events of a batch to those that touched the watched fields.
- `utils/load-database-event-condition-related-records.util.ts` + `services/database-event-trigger-condition.service.ts` load the records a condition references.

**Porting difficulty: MEDIUM** (6/10).
The event system is small and clean, but in Rust the equivalent must be a **transactional outbox**: Twenty emits events in-process after the ORM write (so a crash between the DB write and the event loses the event). A Rust port should write the event rows into an `outbox` table *inside the same transaction* as the mutation and have a relay publish them to the queue — this is strictly better than the TS behaviour and removes the `shouldSkipEventEmission` foot-gun.

**Best crates**: `sqlx` (transaction + `FOR UPDATE SKIP LOCKED` outbox polling), `tokio` + `tokio::sync::broadcast`/`mpsc` for in-process fan-out, `serde`/`serde_json` for the batch payloads, `redis` (pub/sub for the GraphQL subscription bridge), `tracing` for the per-listener tracing spans, `async-trait` for the listener trait.

---

## 7. Consolidated porting plan

### 7.1 Difficulty ranking

| # | Area | Difficulty | Why |
|---|---|---|---|
| 1 | Background processing (queue infra) | **7/10** | BullMQ wire format + two custom Lua subsystems (fair scheduling, memory admission) + 29 queues |
| 2 | Workflow engine | **7/10** | step status machine, waits, retries, Handlebars-compatible resolver |
| 3 | Workflow triggers & listeners | **6/10** | four trigger kinds + the fan-out hub + outbox semantics |
| 4 | Messaging sync (Gmail/MS/IMAP) | **8/10** | provider APIs, MIME/quote stripping, two-phase pipeline, transactional save |
| 5 | Calendar sync (Google/MS/CalDAV) | **7/10** | recurrence expansion, iCalendar parsing, sync tokens |
| 6 | Participant matching + contact creation | **6/10** | email/handle matching rules, person/company creation |
| 7 | Timeline | **6/10** | rule-driven activity types, field exclusion, merge keys |
| 8 | AI agents / chat / tools | **8/10** | no Rust AI-SDK equivalent; streaming, tool-calling, pause/resume |
| 9 | File storage | **5/10** | well-bounded; path safety must be ported literally |
| 10 | Email (SMTP) | **4/10** | `lettre`; the React Email body compilation is the real work |
| 11 | Outbound webhooks | **4/10** | small; signature must match byte-for-byte |
| 12 | MCP server | **6/10** | `rmcp` gives you JSON-RPC + SSE |
| 13 | Logic functions / app runtime | **9/10** | requires shipping a JS runtime |

### 7.2 Crate map

| Need | Crates |
|---|---|
| Async runtime | `tokio` (full), `tokio-util` (`CancellationToken`, `TaskTracker`, `StreamReader`), `futures` |
| HTTP client | `reqwest` (rustls, http2, multipart), `reqwest-eventsource` / `eventsource-stream` (SSE) |
| HTTP server | `axum` (+ `tower`, `tower-http`) |
| Redis | `redis` (aio + tokio-comp), `deadpool-redis` (pooling) |
| Postgres | `sqlx` (postgres, chrono, json, uuid, migrate) |
| Serialization | `serde`, `serde_json`, `serde_with` |
| Errors | `thiserror` (libraries), `anyhow` (binaries) |
| Time | `chrono`, `time` |
| Cron | `cron` (matches `cron-parser` for 5/6-field patterns) |
| Tracing / metrics | `tracing`, `tracing-subscriber`, `opentelemetry`, `prometheus` |
| Crypto | `sha2`, `hmac`, `hex`, `base64`, `aes-gcm` (for the `enc:v2:` credential format), `argon2`, `jsonwebtoken`/`jsonwebtoken-rs` |
| Validation | `validator`, `regex` |
| Object storage | `object_store`, `aws-sdk-s3`, `rusty-s3` |
| Email | `lettre` (+ `lettre_tokio`), `mail-parser` (MIME), `html5ever`/`scraper` (HTML -> text) |
| MCP | `rmcp` |
| JS runtime (logic functions) | `rquickjs` (QuickJS), `boa_engine`, `deno_core` — or keep a Node sidecar |
| Bundling (logic functions) | `oxc`, `swc` (Rust), `esbuild-rs`; zipping with `zip` |
| AWS Lambda | `aws-sdk-lambda` |
| GraphQL | `async-graphql` (if the GraphQL API is also ported) |

### 7.3 Suggested port order

1. **File storage + email + webhooks** (lowest risk, no schema coupling) — establishes the `StorageDriver` / `Tool` / `HttpClient` traits.
2. **Queue infrastructure** — a Rust-native queue with your own key namespace; keep the `MessageQueueService` shape (one service per queue, `add` / `work` / `addCron`).
3. **Workflow engine** — port `evalFromContext` and `resolveInput` **literally** first (they are self-contained and behavioural parity is testable), then the step executor, then the triggers.
4. **Event outbox** — write events into Postgres in the same transaction; relay them to the queue.
5. **Messaging / calendar** — port the provider drivers behind a `MessageImportDriver` / `CalendarImportDriver` trait.
6. **AI** — last: build a thin provider abstraction and re-implement the step loop.

### 7.4 Things to keep verbatim (do not redesign)

- `twenty-shared/src/utils/evalFromContext.ts` and `utils/variable-resolver.ts` — Handlebars-compatible semantics are load-bearing.
- `twenty-shared/src/workflow/constants/StepRetryDelaysMs.ts` (`[1_000, 5_000, 15_000]`).
- The webhook signature recipe: `HMAC-SHA256(secret, "{timestamp}:{JSON.stringify(payload)}")` + `X-Twenty-Webhook-Timestamp` / `-Signature` / `-Nonce`.
- The bullmq job-id convention `` `${prefix}-${uuid}` `` if coexistence is required.
- The fair-scheduling constants (`URGENT_FAIR_PRIORITY = 1`, `FAIR_TICKET_OFFSET = 2`) and the pace-slot key prefix `${workspaceId}:jobGroup:${groupKey}` (the comment says the prefix must not change since 2.60).
- `run-workflow-job-name.ts` (`RUN_WORKFLOW_JOB_NAME = 'RunWorkflowJob'`) and the `groupKey: 'workflow'` convention.
