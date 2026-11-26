import json
import sys
import time
import urllib.request

BASE = "http://127.0.0.1:3001/metadata"
ORIGIN = "http://refhq.localhost:3001"
EMAIL = f"owner-{int(time.time())}@refhq.test"
OUT_DIR = sys.argv[1] if len(sys.argv) > 1 else "/tmp"


def gql(query, token=None, origin=None):
    headers = {"content-type": "application/json"}
    if token:
        headers["authorization"] = f"Bearer {token}"
    if origin:
        headers["origin"] = origin
    req = urllib.request.Request(
        BASE, data=json.dumps({"query": query}).encode(), headers=headers
    )
    body = json.load(urllib.request.urlopen(req))
    if body.get("errors"):
        raise SystemExit(f"graphql error: {json.dumps(body)[:300]}")
    return body["data"]


r = gql(
    f'mutation signUp {{ signUp(email: "{EMAIL}", password: "ReferenceSecret123", locale: "en") '
    "{ tokens { accessOrWorkspaceAgnosticToken { token } } } }"
)
agnostic = r["signUp"]["tokens"]["accessOrWorkspaceAgnosticToken"]["token"]
print("signed up", EMAIL)

r2 = gql(
    "mutation nw { signUpInNewWorkspace(input: { displayName: \"Reference HQ\", subdomain: \"refhq\" }) "
    "{ loginToken { token } workspace { id } } }",
    agnostic,
)
ws_id = r2["signUpInNewWorkspace"]["workspace"]["id"]
lt = r2["signUpInNewWorkspace"]["loginToken"]["token"]
print("workspace", ws_id)

r3 = gql(
    f'mutation ex {{ getAuthTokensFromLoginToken(loginToken: "{lt}", origin: "{ORIGIN}") '
    "{ tokens { accessOrWorkspaceAgnosticToken { token } } } }"
)
access = r3["getAuthTokensFromLoginToken"]["tokens"]["accessOrWorkspaceAgnosticToken"]["token"]

r4 = gql("mutation act { activateWorkspace(data: {}) { id } }", access, origin=ORIGIN)
print("activated", json.dumps(r4)[:150])
open(f"{OUT_DIR}/ws-id.txt", "w").write(ws_id)
open(f"{OUT_DIR}/ref-access.txt", "w").write(access)
print("SEED_OK")
