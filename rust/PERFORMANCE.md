# Performance

No performance result is current unless it names the binary/build, dataset, hardware,
concurrency, command and date. Debug builds are not comparable to release builds.

## Existing reference point (historical, re-measure)

`tools/parity-bench.py` previously measured on the 10-core host:

| Scenario | Rust release | Node 2.61.0 |
| --- | --- | --- |
| healthz throughput | 2,789 rps | 1,155 rps |
| healthz p99 | 6.5 ms | 68.8 ms |
| GraphQL auth-error throughput | 7,985 rps | 2,411 rps |
| GraphQL auth-error p99 | 2.1 ms | 10.1 ms |

These figures predate the current main-line work and are not a current gate.

## Required measurements

### Backend

- Release build and `tools/parity-bench.py` per API scenario.
- p50/p95/p99, throughput, error rate, CPU, RSS, DB time and pool saturation.
- N+1 query counts and `EXPLAIN (ANALYZE, BUFFERS)` for record/list/relation paths.
- Queue latency, worker throughput and retry behavior for email/webhooks.

### Frontend

- Release WASM byte size and initialization time.
- Long-list render count and input latency on the record index.
- Main-thread long tasks, layout shifts, redundant DOM mutations and memory growth.
- Animation frame timing for menus, dialogs, side panels, toasts and skeletons.
- Network waterfalls and duplicate requests on navigation.

## Current icon-slice concern

The complete 4,187-icon catalog is statically embedded in the frontend crate. This is
correct for completeness but may increase WASM size. Measure the release binary before
and after any lazy-loading or asset-splitting change; do not trade missing icons for a
smaller bundle.
