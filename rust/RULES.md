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
