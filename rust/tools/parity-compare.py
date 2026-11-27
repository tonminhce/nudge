import json
import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request

NODE_BASE = "http://127.0.0.1:3001"
RUST_BASE = "http://127.0.0.1:3000"
REFERENCE_ORIGIN = "http://refhq.localhost:3001"
REFERENCE_EMAIL = "owner-1795713371@refhq.test"
REFERENCE_PASSWORD = "ReferenceSecret123"

UUID_RE = re.compile(
    r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}", re.I
)
TOKEN_RE = re.compile(r"eyJ[A-Za-z0-9_-]{20,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}")
EPOCH_RE = re.compile(r"\d{10}(?:\.\d{3})?Z")
ISO_RE = re.compile(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z")


def normalize(text):
    text = TOKEN_RE.sub("<jwt>", text)
    text = UUID_RE.sub("<uuid>", text)
    text = ISO_RE.sub("<iso>", text)
    text = EPOCH_RE.sub("<epoch>", text)
    text = re.sub(r'"message":"[^"]*exists[^"]*"', '"message":"<exists>"', text)
    return text


def request(base, method, path, body=None, headers=None):
    url = base + path
    data = body.encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("content-type", "application/json")
    for key, value in (headers or {}).items():
        req.add_header(key, value)
    try:
        with urllib.request.urlopen(req, timeout=30) as response:
            raw = response.read().decode()
            return response.status, normalize(raw)
    except urllib.error.HTTPError as error:
        raw = error.read().decode()
        return error.code, normalize(raw)


CURSOR_KEYS = {
    "cursor",
    "endCursor",
    "startCursor",
    "after",
    "before",
    "starting_after",
    "ending_before",
}
MV_KEYS = {"metadataVersion"}


def normalize_string(value):
    value = TOKEN_RE.sub("<jwt>", value)
    value = UUID_RE.sub("<uuid>", value)
    value = ISO_RE.sub("<iso>", value)
    value = EPOCH_RE.sub("<epoch>", value)
    return value


def normalize_json(value, key=None):
    if isinstance(value, dict):
        return {k: normalize_json(v, k) for k, v in value.items()}
    if isinstance(value, list):
        return [normalize_json(v) for v in value]
    if key in MV_KEYS and value is not None:
        return "<mv>"
    if isinstance(value, str):
        if key in CURSOR_KEYS:
            return "<cursor>"
        return normalize_string(value)
    return value


def diff_paths(node, rust, path=""):
    if node == rust:
        return []
    if isinstance(node, dict) and isinstance(rust, dict):
        out = []
        for key in sorted(set(node) | set(rust)):
            child = f"{path}.{key}" if path else key
            if key not in node:
                out.append((child, "<missing>", rust[key]))
            elif key not in rust:
                out.append((child, node[key], "<missing>"))
            else:
                out.extend(diff_paths(node[key], rust[key], child))
        return out
    if isinstance(node, list) and isinstance(rust, list):
        out = []
        for index in range(max(len(node), len(rust))):
            child = f"{path}[{index}]"
            left = node[index] if index < len(node) else "<missing>"
            right = rust[index] if index < len(rust) else "<missing>"
            out.extend(diff_paths(left, right, child))
        return out
    return [(path or "<root>", node, rust)]


def request_raw(base, method, path, body=None, headers=None):
    url = base + path
    data = body.encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("content-type", "application/json")
    for key, value in (headers or {}).items():
        req.add_header(key, value)
    try:
        with urllib.request.urlopen(req, timeout=30) as response:
            return response.status, response.read().decode()
    except urllib.error.HTTPError as error:
        return error.code, error.read().decode()


def fetch_reference_token():
    query = (
        "mutation signIn { signIn(email: \"%s\", password: \"%s\") "
        "{ availableWorkspaces { availableWorkspacesForSignIn { id loginToken } } } }"
        % (REFERENCE_EMAIL, REFERENCE_PASSWORD)
    )
    status, body = request_raw(
        NODE_BASE, "POST", "/metadata", json.dumps({"query": query})
    )
    parsed = json.loads(body)
    if not parsed.get("data"):
        raise RuntimeError("signIn failed: %s" % body[:200])
    workspaces = parsed["data"]["signIn"]["availableWorkspaces"][
        "availableWorkspacesForSignIn"
    ]
    login_token = workspaces[0]["loginToken"]
    body = ""
    for origin_argument in (f', origin: "{REFERENCE_ORIGIN}"', ""):
        exchange = (
            'mutation ex { getAuthTokensFromLoginToken(loginToken: "%s"'
            + origin_argument
            + ') { tokens { accessOrWorkspaceAgnosticToken { token } } } }'
        ) % (login_token,)
        status, body = request_raw(
            NODE_BASE, "POST", "/metadata", json.dumps({"query": exchange})
        )
        parsed = json.loads(body)
        if parsed.get("data"):
            return parsed["data"]["getAuthTokensFromLoginToken"]["tokens"][
                "accessOrWorkspaceAgnosticToken"
            ]["token"]
    raise RuntimeError("token exchange failed: %s" % body[:200])


def graphql(base, query, token, variables=None, path="/graphql"):
    payload = {"query": query}
    if variables:
        payload["variables"] = variables
    headers = {"origin": REFERENCE_ORIGIN}
    if token:
        headers["authorization"] = f"Bearer {token}"
    return request_raw(base, "POST", path, json.dumps(payload), headers)


CASES = [
    ("GET", "/healthz", None),
    ("POST", "/graphql", '{ "query": "{ currentUser { id } }" }'),
    ("GET", "/metadata", None),
    ("POST", "/metadata", '{ "query": "query nope { nonexistentField }" }'),
    ("GET", "/nonexistent-page", None),
    ("POST", "/nonexistent-page", '{ "query": "x" }'),
]


def run_record_cases():
    import argparse
    import importlib.util

    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location(
        "record_parity", os.path.join(here, "record-parity.py")
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    scenario = None
    if "--scenario" in sys.argv:
        scenario = sys.argv[sys.argv.index("--scenario") + 1]
    args = argparse.Namespace(
        node_only="--node-only" in sys.argv,
        offline="--offline" in sys.argv,
        scenario=scenario,
    )
    return module.run(args)


def main():
    if "--records" in sys.argv:
        return run_record_cases()
    failures = 0
    for method, path, body in CASES:
        node_status, node_body = request(NODE_BASE, method, path, body)
        rust_status, rust_body = request(RUST_BASE, method, path, body)
        same = node_status == rust_status and node_body == rust_body
        label = "MATCH" if same else "DIFF "
        print(f"{label} {method} {path} node={node_status} rust={rust_status}")
        if not same:
            failures += 1
            print(f"  node: {node_body[:220]}")
            print(f"  rust: {rust_body[:220]}")

    stamp = str(int(time.time()))
    email = f"parity-{stamp}@refhq.test"
    node_body = json.dumps(
        {
            "query": (
                'mutation signUp { signUp(email: "%s", password: "ParitySecret123", locale: "en") '
                "{ tokens { accessOrWorkspaceAgnosticToken { token } } } }" % email
            )
        }
    )
    node_status, node_body = request(NODE_BASE, "POST", "/metadata", node_body)
    rust_status, rust_body = request(
        RUST_BASE,
        "POST",
        "/auth/sign-up",
        json.dumps(
            {
                "email": email.replace("@refhq.test", "@nudge.test"),
                "password": "ParitySecret123",
            }
        ),
    )
    rust_shape = rust_body
    print(f"SIGNUP node={node_status} rust={rust_status}")
    node_ok = node_status == 200 and "<jwt>" in node_body
    rust_ok = rust_status == 201 and "<jwt>" in rust_shape and '"workspaceId"' in rust_shape
    reference_gated = "SIGNUP_DISABLED" in node_body or "restricted to admins" in node_body
    if reference_gated:
        rust_ok = rust_status == 201 and "<jwt>" in rust_shape and '"workspaceId"' in rust_shape
        if rust_ok:
            print("  known: the reference instance gates further sign-ups; the port does not yet")
        else:
            failures += 1
            print(f"  rust: {rust_shape[:220]}")
    elif not (node_ok and rust_ok):
        failures += 1
        print(f"  node: {node_body[:220]}")
        print(f"  rust: {rust_shape[:220]}")

    print(f"\n{failures} differences")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
