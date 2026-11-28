# Porting status

Reference: Twenty CRM `twenty/v2.61.0`, commit
`4bac2940dcdcbf4bb4c974b628cd5e1b0a2e37e0`.

Status vocabulary: `NOT_STARTED`, `IN_PROGRESS`, `IMPLEMENTED`, `VERIFIED`, `BLOCKED`.
`VERIFIED` requires current evidence; stale screenshots and inherited counts do not qualify.

## Current baseline

- Workspace: `/Users/tonminh-mac/Documents/GitHub/nudge/rust`.
- Main-line baseline before the current icon slice: `aeb37cc`.
- Feature matrix: [FEATURE-PARITY.md](FEATURE-PARITY.md). At the baseline it contains
  44 `done` rows and 26 `part` rows; the matrix is authoritative over this summary.
- Full workspace gate at the baseline: 1,181 passed, 1 ignored, 0 failed
  (`bash tools/test-workspace.sh`, log `/tmp/email-durability-baseline.log`).
- Record differential: 0 gaps / 0 diffs across 37 captures and 13 scenarios
  (`tools/record-parity.py`, recorded in FEATURE-PARITY.md).
- Metadata differential: 37 objects, 636 fields, 65 options, 123 indexes and
  34 search fields match the reference (`tools/metadata-parity.sh`).

## Frontend

| Unit | Status | Evidence / next action |
| --- | --- | --- |
| Complete Tabler catalog | VERIFIED | 4,187 definitions regenerated from the reference bundle; byte-identical to the previous catalog plus the upstream `data-twenty-icon` flag. Browser DOM matches for sampled icons. |
| Complete custom SVG catalog | VERIFIED | 46/46 assets pinned to the 2.61.0 commit; defaults, `IconLockCustom`, SVGR data attribute and illustration wrapper corrected. Built-reference browser differential covers 40 exported components plus three special icons; five tree-shaken assets match pinned source/raw SVG. |
| Runtime title and favicon | IMPLEMENTED | `crates/front-app/src/head.rs`; old `head-*` captures are not current certificates. |
| Dialog/Popover/Menu/Tabs | IN_PROGRESS | Render primitives exist; focus, dismissal, keyboard and collision contracts remain. |
| Record index/show | IN_PROGRESS | Live data paths exist; metadata columns, selection, named views, filters and generic editors remain. |
| Command menu and routed side panel | IN_PROGRESS | Global search exists; contextual commands, panel route/stack and resize remain. |
| Settings leaves | NOT_STARTED | Current routes render placeholders. |
| Full UI/behavior/performance parity | IN_PROGRESS | See [UI_PARITY.md](UI_PARITY.md). |

## Backend

| Unit | Status | Evidence / next action |
| --- | --- | --- |
| Core schema and standard objects | VERIFIED | `tools/verify-core-schema.sh`, `tools/prove-attnum-parity.sh`, `tools/metadata-parity.sh`. |
| Record REST/GraphQL differential | VERIFIED | `tools/record-parity.py`: 0 gaps / 0 diffs at the baseline. |
| Permissions and channel visibility | IN_PROGRESS | Main read paths are wired; non-default workspace-member criteria classification remains. |
| Transactional email templates | VERIFIED | Golden SHA-256 table plus `tools/email-template-parity.cjs`. |
| Transactional email delivery | IN_PROGRESS | Live SMTP flow exists; BullMQ-equivalent durability does not. |
| Webhook delivery | IN_PROGRESS | Dispatcher exists; record-event enqueue path is not wired. |
| File module | IN_PROGRESS | Remaining storage/upload behavior is open. |
| Wave 6/7 feature surface | IN_PROGRESS | See [BACKEND_PARITY.md](BACKEND_PARITY.md). |

## Next ordered work

1. Commit the verified icon/design-token slice and its license notices.
2. Production overlay/navigation contracts: Dialog, Popover, Menu, Tabs.
3. Shared metadata/context substrate, then record index/show.
4. Contextual command menu and routed side panel.
5. Settings leaves.
6. In parallel where file ownership permits: email queue durability, webhook delivery,
   non-default-member RLS criteria, file module, then wave 6/7.

## Operating rules

- Read [RULES.md](RULES.md) before changing code.
- Use [PROMPT-NEXT-AGENT.md](PROMPT-NEXT-AGENT.md) when transferring the work.
- No comments in code. Prose belongs in Markdown.
- Commit and push green, independently verified slices on `main`.
- Never delete reference/test database rows or drop databases without explicit approval.
