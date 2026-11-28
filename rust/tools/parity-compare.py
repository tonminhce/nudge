import json
import os
import re
import secrets
import subprocess
import sys
import time
import urllib.error
import urllib.request

NODE_BASE = "http://127.0.0.1:3001"
RUST_BASE = "http://127.0.0.1:3000"
REFERENCE_ORIGIN = "http://refhq.localhost:3001"
REFERENCE_EMAIL = "owner-1795713371@refhq.test"
REFERENCE_PASSWORD = os.environ.get("TWENTY_REFERENCE_PASSWORD")

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


def status_code(base, path):
    try:
        with urllib.request.urlopen(base + path, timeout=30) as response:
            return response.status
    except urllib.error.HTTPError as error:
        return error.code
    except (urllib.error.URLError, TimeoutError):
        return 0


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
    if not REFERENCE_PASSWORD:
        raise RuntimeError("TWENTY_REFERENCE_PASSWORD is required")
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
    ("POST", "/nonexistent-page", '{ "query": "x" }'),
]

BUNDLE_REL_RE = re.compile(r"modulepreload|preload|prefetch|module|preconnect|dns-prefetch", re.I)
ASSET_RE = re.compile(r"\.(?:css|ico|jpeg|jpg|png|svg|webp)$", re.I)
BRAND_RE = re.compile(r"(?i)\bnudge\b|\btwenty\b")


def tag_attrs(fragment):
    return dict(re.findall(r'([a-zA-Z-]+)\s*=\s*"([^"]*)"', fragment))


def brand_value(value):
    value = BRAND_RE.sub("<brand>", value)
    return "<asset>" if ASSET_RE.search(value.split("?")[0]) else value


def shell_contract(html):
    nodes = []
    root = re.search(r'<html\b((?:[^>"]|"[^"]*")*)>', html, re.I)
    if root:
        attrs = " ".join(
            f"{name}={brand_value(value)}"
            for name, value in sorted(tag_attrs(root.group(1)).items())
        )
        nodes.append(f"html:{attrs}")
    stylesheets = 0
    for tag in re.finditer(r'<(meta|link|noscript|div)\b((?:[^>"]|"[^"]*")*)>', html, re.I):
        name = tag.group(1).lower()
        attrs = tag_attrs(tag.group(2))
        rel = attrs.get("rel", "")
        if name == "link" and BUNDLE_REL_RE.fullmatch(rel):
            continue
        if name == "link" and rel == "stylesheet":
            stylesheets += 1
            continue
        if name == "div":
            if attrs.get("id") == "root":
                nodes.append("div:id=root")
            continue
        key = (
            attrs.get("name")
            or attrs.get("property")
            or rel
            or attrs.get("charset")
            or ""
        )
        value = attrs.get("content") or attrs.get("href") or ""
        node = f"{name}:{key}" if key else name
        if value:
            node += "=" + brand_value(value)
        nodes.append(node)
    if stylesheets:
        nodes.append("link:stylesheet=<bundle>")
    title = re.search(r"<title>([^<]*)</title>", html, re.I)
    if title:
        nodes.append("title:" + brand_value(title.group(1).strip()))
    noscript = re.search(r"<noscript>([^<]*)</noscript>", html, re.I)
    if noscript:
        nodes.append("noscript:" + noscript.group(1).strip())
    return nodes


def shell_report():
    node_status, node_body = request(NODE_BASE, "GET", "/nonexistent-page")
    rust_status, rust_body = request(RUST_BASE, "GET", "/nonexistent-page")
    node_nodes = shell_contract(node_body)
    rust_nodes = shell_contract(rust_body)
    same = node_status == rust_status == 200 and node_nodes == rust_nodes
    print(f"{'MATCH' if same else 'DIFF '} GET shell head contract")
    if same:
        return 0
    for index in range(max(len(node_nodes), len(rust_nodes))):
        left = node_nodes[index] if index < len(node_nodes) else "<missing>"
        right = rust_nodes[index] if index < len(rust_nodes) else "<missing>"
        if left != right:
            print(f"  node[{index}]: {left}")
            print(f"  rust[{index}]: {right}")
    return 1


def manifest_report():
    node_status, node_body = request(NODE_BASE, "GET", "/manifest.json")
    rust_status, rust_body = request(RUST_BASE, "GET", "/manifest.json")
    if node_status != 200 or rust_status != 200:
        print(f"DIFF  GET /manifest.json node={node_status} rust={rust_status}")
        return 1
    node = json.loads(node_body)
    rust = json.loads(rust_body)
    problems = []
    if sorted(node) != sorted(rust):
        problems.append(f"keys node={sorted(node)} rust={sorted(rust)}")
    for key in ("display", "start_url", "theme_color", "background_color"):
        if node.get(key) != rust.get(key):
            problems.append(f"{key} node={node.get(key)} rust={rust.get(key)}")
    for key in ("short_name", "name"):
        if BRAND_RE.sub("<brand>", str(node.get(key, ""))) != BRAND_RE.sub(
            "<brand>", str(rust.get(key, ""))
        ):
            problems.append(f"{key} node={node.get(key)} rust={rust.get(key)}")
    node_fields = {tuple(sorted(icon)) for icon in node.get("icons", [])}
    rust_fields = {tuple(sorted(icon)) for icon in rust.get("icons", [])}
    if node_fields != rust_fields:
        problems.append(f"icon fields node={node_fields} rust={rust_fields}")
    for base, document in ((RUST_BASE, rust), (NODE_BASE, node)):
        for icon in document.get("icons", []):
            status = status_code(base, "/" + icon["src"].lstrip("/"))
            if status != 200:
                problems.append(f"{base} {icon['src']} -> {status}")
    same = not problems
    print(f"{'MATCH' if same else 'DIFF '} GET /manifest.json asset contract")
    for problem in problems:
        print(f"  {problem}")
    return 0 if same else 1


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
    failures += shell_report()
    failures += manifest_report()

    if os.environ.get("NUDGE_PARITY_SIGNUP") == "1":
        stamp = str(int(time.time()))
        email = f"parity-{stamp}@refhq.test"
        password = "Parity-" + secrets.token_urlsafe(18)
        node_body = json.dumps(
            {
                "query": (
                    "mutation signUp { signUp(email: %s, password: %s, locale: \"en\") "
                    "{ tokens { accessOrWorkspaceAgnosticToken { token } } } }"
                    % (json.dumps(email), json.dumps(password))
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
                    "password": password,
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
