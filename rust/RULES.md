# Rules

Behaviours the user has corrected. Every unit must keep each one. Add a line the moment the user
corrects a behaviour, so the next unit and the next session cannot make the same mistake again.

Rules are added with:

```sh
rust-anything rules <repo> --add "<the rule, in one sentence>"
```

## Corrections

### R-001: No comments in code
- **Statement:** No comments in code. This covers every source file the port writes: Rust, SQL and
  shell. No `//`, `///`, `//!` or `--` comments, and no comment blocks.
- **Why:** The user set it as a hard rule on 2026-11-26. The code has to read on its own; the
  reasoning belongs in the documentation, not in the source.
- **How to apply:** Write the code without comments. Put the explanation in `DECISIONS.md` (an ADR),
  `GLOSSARY.md`, `MAP.md` or `HANDOFF.md` instead. This overrides the `rust-anything` convention of
  marking infallible states with an `// INFALLIBLE:` comment: do not add such a comment, and do not
  let one survive a rewrite. Documentation files (`.md`) keep their prose; `.env.example` and the
  generated SQL baseline keep no comments either.

### R-002: The auth flows are verified against the upstream behaviour, not guessed
- **Statement:** Every auth behaviour is checked against the upstream implementation before it is
  considered done. The three ways to check: the JWT vectors in
  `crates/server/src/auth/jwt_service.rs` (signed by Node, see
  `tools/generate-jwt-test-vectors.mjs`), the bit-compatibility checklist in
  `research/auth-permissions.md` section 6, and the end-to-end script `tools/auth-e2e.sh`.
- **Why:** The upstream uses several details that look like bugs but are the contract: the legacy
  HS256 secret has no separator between its parts, the refresh token stays usable for one minute
  after it is rotated, the login token is single use, and the two factor table is keyed by
  `userWorkspaceId` and not by `userId`.
- **How to apply:** When a flow disagrees with `research/auth-permissions.md`, the research wins.
  When the research is silent, read the upstream source before writing code, and add a test.

- DB-gated tests must run core migrations through workspace_migrations::ddl::test_support::migrate_core (pins search_path to public, advisory-locked, fails loudly); never sqlx::migrate! on the AppState pool and never swallow the result with .ok()? — the pool search_path makes sqlx re-apply migrations against a split _sqlx_migrations ledger and .ok()? turns every DB-gated test into a silent skip.
- core.objectMetadata.targetTableName stores the literal 'DEPRECATED' upstream at 2.61.0; the physical workspace table name must be derived from nameSingular + application universal identifier (compute_object_target_table), never read from that column.

### R-003: Frontend parity is exact and measured live
- **Statement:** Frontend UI must match the live Twenty CRM v2.61.0 reference exactly; verify every surface with side-by-side browser screenshots and computed styles, with zero tolerated visual differences.
- **Why:** The user requires the port to look and behave like the real product, not an approximation.
- **How to apply:** Use `tools/ui-parity-harness.mjs` from the persistent Qoder Browser `node_repl`. For every page, component and interaction, capture the reference and Nudge at the same viewport and state with `captureSurface`, then run `diffSurfaces` and `formatDiff`. Compare DOM structure, geometry, computed layout/spacing/color/typography, SVG markup/stroke, focus/disabled state, and animation timing/keyframes. The default tolerance is zero; only explicit branding/text substitutions may be ignored. Save paired screenshots and the JSON diff as evidence, exercise the real pointer/keyboard path, and do not mark the UI unit done until the diff is empty and the interaction matches. Build release assets with `tools/build-frontend.sh` so the exact upstream fonts are copied into `dist/fonts`.

### R-004: The backend scope is every Twenty feature
- **Statement:** Backend must reimplement every Twenty CRM feature, not only the daily-driver surface; keep the full wave-6 and wave-7 scope and prove behavior with live reference parity.
- **Why:** The user explicitly rejected a reduced scope.
- **How to apply:** Track every feature in `FEATURE-PARITY.md`; do not mark the migration complete while any applicable row remains todo or part.
