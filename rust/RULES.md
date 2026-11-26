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

