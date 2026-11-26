# Record parity harness (wave-4 tooling)

This directory holds the committed golden fixtures captured from the live
Twenty 2.61.0 reference server by the record-parity harness. The harness itself
is the pair of scripts next to it:

- `../parity-compare.py` — bootstrap healthz/fallback comparison plus the shared
  normalizers and a key-order-insensitive JSON diff used by everything else.
- `../record-parity.py` — the parameterized record scenario runner
  (findMany, findOne, filter, sort + cursor pagination, a
  create → read → update → soft-delete → restore → destroy round-trip, an
  error-code case, and one REST find-many case) against both servers.

## Running

Requirements: the Node reference on `http://127.0.0.1:3001` (container
`twenty-ref`, seeded workspace `refhq`), the Rust server on
`http://127.0.0.1:3000` (`nudge-server`), and Python 3 stdlib only.

    cd <repo>/rust/worktrees/parity-harness/rust

    python3 -I tools/parity-compare.py                 # bootstrap cases (healthz etc.)
    python3 -I tools/record-parity.py --node-only      # re-capture the golden fixtures
    python3 -I tools/record-parity.py                  # full comparison (node live, rust live)
    python3 -I tools/record-parity.py --offline        # compare rust live against committed fixtures only
    python3 -I tools/record-parity.py --selftest       # verify normalizers + JSON diff logic offline
    python3 -I tools/parity-compare.py --records       # same as record-parity.py (delegates)

The reference token is minted per run via the seed-script flow: `signIn` (which
returns a per-workspace `loginToken` for the already-seeded owner, so sign-up is
never re-run), then `getAuthTokensFromLoginToken` with the `refhq.localhost:3001`
origin. If the Rust record plane later needs its own credential, pass it in the
`RUST_TOKEN` environment variable; today the Rust server exposes no record
endpoints at all and `RUST_TOKEN` is unused.

## Current status vs the Rust server

The live `nudge-server` build serves `/healthz`, `/auth/*`, and the 404
fallback; `POST /graphql`, `GET /rest/*`, and `POST /metadata` all return the
fallback 404 (`{"message":"Cannot POST /graphql",...}`). The harness probes
this first and renders every Rust column as `GAP` ("record plane not
implemented") instead of emitting misleading diffs. That gap map is the wave-4
work: `crates/server/src/query/entry/graphql.rs` + `entry/rest.rs` and their
runners per `research/query-layer.md §10/§12 (U3–U7)`. Once those land, the
same runs flip `GAP` rows to `MATCH`/`DIFF` automatically.

## Normalizers (in `parity-compare.py`)

Applied to both sides before any comparison; a diff on normalized values is a
real wire-contract difference, anything else is noise:

- `<uuid>` — any UUID string (record ids differ per instance).
- `<iso>` — `YYYY-MM-DDTHH:MM:SS(.fff)Z` timestamps (wall-clock noise).
- `<epoch>` — `NNNNNNNNNN(.NNN)Z` epoch-style stamps (kept from the original script).
- `<jwt>` — ES256-shaped tokens (auth flows, `loginToken`/`accessToken`).
- `<cursor>` — **new**: strings under keys `cursor`, `endCursor`,
  `startCursor`, `after`, `before`, `starting_after`, `ending_before`.
  Connection cursors are base64url(JSON) payloads embedding record ids and
  order-leaf values (upstream `engine/api/common/utils/cursors.util.ts:85-95`
  at tag `twenty/v2.61.0`), so their bytes legitimately differ between two
  databases; pagination semantics are asserted on page contents + `pageInfo`
  instead.
- `<mv>` — **new**: any `metadataVersion` value (Redis-stored per-workspace
  counter, upstream `engine/api/rest/core/handlers/rest-api-base.handler.ts:181-197`;
  never two servers agree on it).
- JSON diff is key-order-insensitive: comparison happens on recursively
  normalized structures (dicts compared as maps, lists positionally), and
  `diff_paths` reports leaf-level mismatches as `path: node=X rust=Y`.
- Error comparison: `extensions.code` and `extensions.subCode` are collected
  across all errors into a set (`INVALID_ARGS_FILTER` + `BAD_USER_INPUT` must
  match as *sets*, not ordered lists — a second error entry with reversed key
  order still passes); `message` text is stored in fixtures but compared
  best-effort only (ADR-005 / `research/query-layer.md §10`).

## Fixture files

One per scenario, captured live from the reference and normalized as above:

- `findMany-first2.node.json` — connection shape + `<cursor>` normalization.
- `findOne.node.json` — singular resolver with required `filter:{id:{eq:}}`.
- `company-filter.node.json` — equality filter on companies.
- `orderBy-pagination.node.json` — composite-subfield orderBy + `after` walk.
- `mutation-roundtrip.node.json` — create/read/update/soft-delete/hidden-read/
  restore/destroy/gone-read (the record is destroyed again by the scenario, so
  the reference DB stays at its seeded state; `destroy` is the last step).
- `error-bad-filter.node.json` — bad composite operator: codes/shape envelope.
- `rest-findMany.node.json` — REST envelope `{data.people[], totalCount, pageInfo}`.

Each file's exact request and its upstream file:line contract are in
`CAPTURES.md` (all citations against tag `twenty/v2.61.0`).

## Deviations

- `mutation-roundtrip` issues its GraphQL via string-interpolated ids (not
  variables) for the update/delete/restore/destroy steps; upstream accepts
  both, and the harness normalizes ids to `<uuid>` anyway.
- The harness writes/reads only its own `tools/e2e-capture/` directory; it
  never mutates the reference database beyond what a round-trip scenario
  needs, and its round-trip cleans up after itself (verified count of
  leftover harness rows: 0).
