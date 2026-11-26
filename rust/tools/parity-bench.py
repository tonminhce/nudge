import json
import statistics
import sys
import threading
import time
import urllib.error
import urllib.request

WORKERS = 8
REQUESTS_PER_WORKER = 250


def hammer(base, path, body=None, headers=None):
    url = base + path
    latencies = []
    errors = 0
    local = threading.local()

    def worker():
        nonlocal errors
        opener = urllib.request.build_opener()
        data = body.encode() if body is not None else None
        for _ in range(REQUESTS_PER_WORKER):
            started = time.perf_counter()
            try:
                req = urllib.request.Request(url, data=data, method="POST" if data else "GET")
                for key, value in (headers or {}).items():
                    req.add_header(key, value)
                with opener.open(req, timeout=30) as response:
                    response.read()
            except urllib.error.HTTPError as error:
                error.read()
            except Exception:
                errors += 1
            latencies.append(time.perf_counter() - started)

    threads = [threading.Thread(target=worker) for _ in range(WORKERS)]
    started = time.perf_counter()
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    wall = time.perf_counter() - started

    latencies.sort()
    total = len(latencies)
    return {
        "rps": round(total / wall, 1),
        "p50_ms": round(statistics.median(latencies) * 1000, 2),
        "p95_ms": round(latencies[int(total * 0.95)] * 1000, 2),
        "p99_ms": round(latencies[int(total * 0.99)] * 1000, 2),
        "errors": errors,
    }


def main():
    node_base = "http://127.0.0.1:3001"
    rust_base = "http://127.0.0.1:3000"
    graphql_body = json.dumps({"query": "{ __typename }"})

    scenarios = [
        ("healthz", "GET", "/healthz", None),
        (
            "graphql auth error",
            "POST",
            "/graphql",
            graphql_body,
        ),
    ]

    for name, method, path, body in scenarios:
        print(f"== {name} ({WORKERS} workers x {REQUESTS_PER_WORKER} requests)")
        for label, base in (("node", node_base), ("rust", rust_base)):
            result = hammer(base, path, body)
            print(
                f"  {label:5} {result['rps']:>8} rps   p50 {result['p50_ms']:>8} ms"
                f"   p95 {result['p95_ms']:>8} ms   p99 {result['p99_ms']:>8} ms"
                f"   errors {result['errors']}"
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
