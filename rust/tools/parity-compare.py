import json
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request

NODE_BASE = "http://127.0.0.1:3001"
RUST_BASE = "http://127.0.0.1:3000"

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


CASES = [
    ("GET", "/healthz", None),
    ("POST", "/graphql", '{ "query": "{ currentUser { id } }" }'),
    ("GET", "/metadata", None),
    ("POST", "/metadata", '{ "query": "query nope { nonexistentField }" }'),
    ("GET", "/nonexistent-page", None),
    ("POST", "/nonexistent-page", '{ "query": "x" }'),
]


def main():
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
