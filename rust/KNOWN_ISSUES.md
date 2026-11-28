# Known issues

## Critical correctness/security

- Non-default workspace-member roles are not classified as limiting in
  `permissions/context.rs`; criteria-only collection reads can still pass where
  upstream denies them.
- Transactional email jobs are in-process tasks, not durable queue entries; process
  termination drops queued mail.
- Webhook delivery is not wired to record-event commit callbacks.
- Existing UI evidence includes stale or mis-route captures; do not cite it as current
  parity proof.

## Frontend gaps

- Dialog/Popover/Menu/Tabs primitives lack production focus, dismissal, keyboard and
  collision behavior.
- Record index is a simplified surrogate: fixed columns, fake checkbox spans, no named
  views/filter builder/groups/aggregates/drag-and-drop.
- Record show uses static fields and People-only editing.
- Command menu is global/static rather than contextual.
- Side panel is local state rather than a routed stack.
- Settings routes render placeholders.

## Data/cleanup requiring approval

- `twenty_ref` contains test workspaces/users/tokens created by E2E drivers.
- Two `ProbeWhere T` rows in workspace `workspace_2uyacdvjhehfx4qxnjecohfkd` block a
  clean `findMany-first2` measurement. Ask before deleting them.
- Do not drop the 42 Postgres databases without explicit approval.

## Product decisions

- Email brand strings use Nudge where upstream uses Twenty.
- Upstream builds some verification links as workspace URLs; the port uses the
  frontend base URL.

## Evidence hygiene

- `FEATURE-PARITY.md` still contains historical prose that can overstate completion;
  the concrete command results and current source are authoritative.
- `rust/HANDOFF.md` is an older 2026-11-26 snapshot. Prefer `PORTING_STATUS.md`,
  `UI_PARITY.md`, `BACKEND_PARITY.md` and the current `FEATURE-PARITY.md`.
- UI JSON captures omit candidate commit/build, fixture and verified auth state.
