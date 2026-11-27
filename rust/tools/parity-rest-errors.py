import json
import os
import re
import sys
import urllib.error
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import importlib.util

here = os.path.dirname(os.path.abspath(__file__))
spec = importlib.util.spec_from_file_location(
    "parity_compare", os.path.join(here, "parity-compare.py")
)
pc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pc)

TOKEN = os.environ.get("RUST_TOKEN", "")
if not TOKEN:
    raise SystemExit("RUST_TOKEN env is required")

CASES = [
    ("GET", "/rest/people?limit=bogus"),
    ("GET", "/rest/people?filter=bogus"),
    ("GET", "/rest/people?depth=2"),
    ("GET", "/rest/people?order_by=bogus"),
    ("GET", "/rest/people?starting_after=bogus"),
    ("GET", "/rest/people?ending_before=bogus&starting_after=bogus"),
    ("GET", "/rest/people?fields=name,companyId"),
    ("GET", "/rest/people?fields=name,bogusField"),
    ("GET", "/rest/people?fields="),
    ("GET", "/rest/people/bogus-uuid"),
    ("GET", "/rest/bogus-object"),
    ("GET", "/rest/people/11111111-1111-1111-1111-111111111111"),
    ("GET", "/rest/people/too/many/segments"),
    ("PATCH", "/rest/people"),
    ("DELETE", "/rest/people?filter=%7B%7D"),
    ("GET", "/rest/people?group_by=bogus&limit=1"),
    ("GET", "/rest/people?fields=company&limit=1"),
    ("GET", "/rest/people?fields=company&depth=1&limit=1"),
]


def fetch(base, method, path):
    req = urllib.request.Request(base + path, method=method)
    req.add_header("content-type", "application/json")
    req.add_header("authorization", f"Bearer {TOKEN}")
    try:
        with urllib.request.urlopen(req, timeout=60) as response:
            return response.status, json.loads(response.read().decode())
    except urllib.error.HTTPError as error:
        raw = error.read().decode()
        try:
            return error.code, json.loads(raw)
        except Exception:
            return error.code, raw


def deep_diff(path, node, rust, out):
    if isinstance(node, dict) and isinstance(rust, dict):
        for key in sorted(set(node) | set(rust)):
            deep_diff(
                f"{path}.{key}",
                node.get(key, "<missing>"),
                rust.get(key, "<missing>"),
                out,
            )
    elif isinstance(node, list) and isinstance(rust, list):
        if len(node) != len(rust):
            out.append(f"{path}: len node={len(node)} rust={len(rust)}")
        else:
            for index, (left, right) in enumerate(zip(node, rust)):
                deep_diff(f"{path}[{index}]", left, right, out)
    else:
        normalized_node = pc.normalize(json.dumps(node))
        normalized_rust = pc.normalize(json.dumps(rust))
        if normalized_node != normalized_rust:
            out.append(
                f"{path}: node={normalized_node[:80]} rust={normalized_rust[:80]}"
            )


failures = 0
for method, path in CASES:
    node_status, node_body = fetch(pc.NODE_BASE, method, path)
    rust_status, rust_body = fetch(pc.RUST_BASE, method, path)
    diffs = []
    if node_status != rust_status:
        diffs.append(f"status: node={node_status} rust={rust_status}")
    deep_diff("<root>", node_body, rust_body, diffs)
    label = "MATCH" if not diffs else "DIFF "
    print(f"{label} {method} {path}")
    for diff in diffs[:6]:
        print(f"  {diff}")
    failures += bool(diffs)

print(f"\n{failures} differences")
sys.exit(1 if failures else 0)
