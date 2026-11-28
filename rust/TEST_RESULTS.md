# Test results

Record the command, commit/build, environment and result. A green suite is evidence only
for the behavior it actually exercises.

## Baseline workspace gate

- Command: `bash tools/test-workspace.sh`
- Environment: `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_DEV_INCREMENTAL=false`
- Result: 1,181 passed, 1 ignored, 0 failed.
- Log: `/tmp/email-durability-baseline.log`.
- Note: this baseline predates the current icon slice.

## Current icon-slice gate

- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed
  (log `/tmp/icon-clippy.log`).
- `bash tools/test-workspace.sh`: 1,187 passed, 1 ignored, 0 failed
  (log `/tmp/icon-workspace-gate.log`).

## Icon slice

- `cargo check -p nudge-front-ui --all-targets`: passed.
- `cargo test -p nudge-front-ui icons::tests -- --test-threads=1`:
  22 passed, 0 failed.
- `cargo check -p nudge-front-core -p nudge-front-app --target wasm32-unknown-unknown`:
  passed.
- `cargo fmt --all --check`: passed at the time of writing.
- `bash tools/check-icon-catalog.sh`: both generated catalogs reproduce exactly after
  regeneration and `rustfmt`.
- Gallery SSR tests now render under `Owner::default()` because the complete custom
  catalog uses a keyed `<For>`; all five gallery tests pass.
- Browser comparison of the local gallery against the built reference:
  - sampled Tabler/custom address-book DOM: canonical match;
  - 40 exported custom icons: path geometry and computed styles match;
  - three special icons: canonical SVG match;
  - five tree-shaken custom assets: pinned source/raw SVG match.

## Differential baselines recorded in FEATURE-PARITY.md

- `tools/record-parity.py`: 0 gaps / 0 diffs.
- `tools/access-group-parity.py`: MATCH for the visible-company fixture.
- `tools/metadata-parity.sh`: object/field/option/index/search-field counts match.
- `tools/permissions-parity.py`: 29 historical `workspace.permissionFlags` differences
  were addressed by the 12-flag synchronization; re-run before treating this as current.

## Not yet run for the current slice

- Release frontend build and WASM size comparison.
- Full browser interaction/visual regression for non-icon components.
