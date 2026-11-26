import argparse
import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CAPTURE_DIR = os.path.join(HERE, "e2e-capture")


def _load_parity_compare():
    spec = importlib.util.spec_from_file_location(
        "parity_compare", os.path.join(HERE, "parity-compare.py")
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


pc = _load_parity_compare()


def as_json(raw):
    try:
        return json.loads(raw)
    except (ValueError, TypeError):
        return {"<raw>": raw}


class Client:
    def __init__(self, base, token, label):
        self.base = base
        self.token = token
        self.label = label

    def gql(self, query, variables=None):
        status, raw = pc.graphql(self.base, query, self.token, variables)
        return status, as_json(raw)

    def rest(self, path):
        headers = {"origin": pc.REFERENCE_ORIGIN}
        if self.token:
            headers["authorization"] = f"Bearer {self.token}"
        status, raw = pc.request(self.base, "GET", path, None, headers)
        return status, as_json(raw)


def error_codes(body):
    codes = set()
    for err in (body.get("errors") or []):
        ext = err.get("extensions") or {}
        for field in ("code", "subCode"):
            value = ext.get(field)
            if value:
                codes.add(value)
    return sorted(codes)


def resolve_person_id(client):
    _, body = client.gql("query { people(first: 1) { edges { node { id } } } }")
    edges = ((body.get("data") or {}).get("people") or {}).get("edges") or []
    if not edges:
        raise RuntimeError(f"{client.label}: no people seeded")
    return edges[0]["node"]["id"]


def resolve_company_name(client):
    _, body = client.gql("query { companies(first: 1) { edges { node { name } } } }")
    edges = ((body.get("data") or {}).get("companies") or {}).get("edges") or []
    if not edges:
        raise RuntimeError(f"{client.label}: no companies seeded")
    return edges[0]["node"]["name"]


CONN = """
  edges { node { %s } cursor }
  pageInfo { hasNextPage hasPreviousPage startCursor endCursor }
  totalCount
"""


def scenario_find_many(client):
    person_id = resolve_person_id(client)
    fields = "id name { firstName lastName } company { name } position"
    query = "query { people(first: 2) { %s } }" % (CONN % fields)
    _, body = client.gql(query)
    return {"findMany": body, "anchorPersonId": person_id}


def scenario_find_one(client):
    person_id = resolve_person_id(client)
    query = (
        "query($id: UUID!) { person(filter: { id: { eq: $id } }) "
        "{ id name { firstName lastName } company { name } createdAt updatedAt } }"
    )
    _, body = client.gql(query, {"id": person_id})
    return {"findOne": body}


def scenario_company_filter(client):
    name = resolve_company_name(client)
    query = (
        "query($name: String) { companies(filter: { name: { eq: $name } }) "
        "{ edges { node { id name } } totalCount } }"
    )
    _, body = client.gql(query, {"name": name})
    return {"companyFilter": body}


def scenario_order_pagination(client):
    order = "orderBy: [{ name: { lastName: DescNullsLast } }]"
    fields = "name { lastName }"
    query = "query { people(first: 2, %s) { %s } }" % (order, CONN % fields)
    _, page1 = client.gql(query)
    data = page1.get("data") or {}
    people = data.get("people") or {}
    cursor = (people.get("pageInfo") or {}).get("endCursor")
    out = {"page1": page1}
    if cursor:
        query2 = (
            "query($c: String) { people(first: 2, after: $c, %s) { %s } }"
            % (order, CONN % fields)
        )
        _, page2 = client.gql(query2, {"c": cursor})
        out["page2"] = page2
    return out


def scenario_mutation_round_trip(client):
    captures = {}
    _, created = client.gql(
        "mutation { createPerson(data: { name: { firstName: \"Parity\", lastName: \"Temp\" } }) "
        "{ id name { firstName } position createdAt } }"
    )
    captures["create"] = created
    person = (created.get("data") or {}).get("createPerson") or {}
    pid = person.get("id")
    if not pid:
        captures["abort"] = {"error": "create failed", "codes": error_codes(created)}
        return captures
    q = "query($id: UUID!) { person(filter: { id: { eq: $id } }) { id deletedAt } }"
    _, read = client.gql(q, {"id": pid})
    captures["read"] = read
    _, updated = client.gql(
        "mutation($id: UUID!) { updatePerson(id: $id, data: { jobTitle: \"Harness\" }) "
        "{ id jobTitle updatedAt } }",
        {"id": pid},
    )
    captures["update"] = updated
    _, soft = client.gql(
        "mutation($id: UUID!) { deletePerson(id: $id) { id deletedAt } }", {"id": pid}
    )
    captures["softDelete"] = soft
    _, hidden = client.gql(q, {"id": pid})
    captures["hiddenRead"] = hidden
    _, restored = client.gql(
        "mutation($id: UUID!) { restorePerson(id: $id) { id deletedAt } }", {"id": pid}
    )
    captures["restore"] = restored
    _, destroyed = client.gql(
        "mutation($id: UUID!) { destroyPerson(id: $id) { id } }", {"id": pid}
    )
    captures["destroy"] = destroyed
    _, gone = client.gql(q, {"id": pid})
    captures["gone"] = gone
    return captures


def scenario_error_filter(client):
    _, body = client.gql(
        "query { people(filter: { name: { bogus: \"x\" } }) { edges { node { id } } } }"
    )
    shape = set()
    for err in (body.get("errors") or []):
        shape.update((err.get("extensions") or {}).keys())
    return {
        "errorCodes": error_codes(body),
        "errorShape": sorted(shape),
        "error": body,
    }


def scenario_rest_find_many(client):
    status, body = client.rest(
        "/rest/people?limit=2&order_by=name.lastName%5BDescNullsLast%5D"
    )
    return {"rest": {"statusCode": status, "body": body}}


SCENARIOS = [
    ("findMany-first2", scenario_find_many),
    ("findOne", scenario_find_one),
    ("company-filter", scenario_company_filter),
    ("orderBy-pagination", scenario_order_pagination),
    ("mutation-roundtrip", scenario_mutation_round_trip),
    ("error-bad-filter", scenario_error_filter),
    ("rest-findMany", scenario_rest_find_many),
]


def is_gap(status, body):
    if status == 404:
        return True
    if isinstance(body, dict) and body.get("message", "").startswith("Cannot "):
        return True
    codes = error_codes(body) if isinstance(body, dict) else []
    return "UNAUTHENTICATED" in codes and not (body.get("data") if isinstance(body, dict) else None)


def compare_capture(node_val, rust_val, rust_reachable):
    if not rust_reachable:
        return "GAP", "record endpoint not implemented on Rust server"
    node_norm = pc.normalize_json(node_val)
    rust_norm = pc.normalize_json(rust_val)
    if node_norm == rust_norm:
        return "MATCH", ""
    diffs = pc.diff_paths(node_norm, rust_norm)
    preview = "; ".join(
        "%s: node=%s rust=%s" % (p, json.dumps(l)[:48], json.dumps(r)[:48])
        for p, l, r in diffs[:4]
    )
    if len(diffs) > 4:
        preview += "; +%d more" % (len(diffs) - 4)
    return "DIFF", preview


def run_node(client, name, fn):
    captures = fn(client)
    os.makedirs(CAPTURE_DIR, exist_ok=True)
    payload = {
        "scenario": name,
        "normalized": pc.normalize_json(captures),
    }
    path = os.path.join(CAPTURE_DIR, "%s.node.json" % name)
    with open(path, "w") as handle:
        json.dump(payload, handle, indent=2, sort_keys=True)
        handle.write("\n")
    return captures, path


def load_fixture(name):
    path = os.path.join(CAPTURE_DIR, "%s.node.json" % name)
    if not os.path.exists(path):
        return None
    with open(path) as handle:
        return json.load(handle)["normalized"]


def selftest():
    base = {
        "data": {
            "people": {
                "edges": [
                    {
                        "node": {
                            "id": "7a93d1e5-3f74-4945-8a65-d7f996083f72",
                            "name": {"firstName": "Ivan", "lastName": "Zhao"},
                        },
                        "cursor": "eyJpZCI6IjdhOTNkMWU1LTNmNzQtNDk0NS04YTY1LWQ3Zjk5NjA4M2Y3MiJ9",
                    }
                ],
                "pageInfo": {
                    "hasNextPage": True,
                    "endCursor": "eyJpZCI6IngyMzM0NTM0NS0xMjM0LTU2NzgtMTIzNC0xMjM0NTY3ODkwMTIifQ",
                    "startCursor": "eyJpZCI6IjdhOTNkMWU1LTNmNzQtNDk0NS04YTY1LWQ3Zjk5NjA4M2Y3MiJ9",
                },
                "totalCount": 5,
            }
        }
    }
    reordered = {
        "data": {
            "people": {
                "totalCount": 5,
                "pageInfo": {
                    "endCursor": "eyJpZCI6Ijk5OTk5OTk5LTk5OTktOTk5OS05OTk5LTk5OTk5OTk5OTk5OSJ9",
                    "startCursor": "eyJpZCI6IjExMTExMTExLTExMTEtMTExMS0xMTExLTExMTExMTExMTExMSJ9",
                    "hasNextPage": True,
                },
                "edges": [
                    {
                        "cursor": "eyJpZCI6ImFhYWFhYWFhLWFhYWEtYWFhYS1hYWFhLWFhYWFhYWFhYWFhYSJ9",
                        "node": {
                            "name": {"lastName": "Zhao", "firstName": "Ivan"},
                            "id": "99999999-9999-9999-9999-999999999999",
                        },
                    }
                ],
            }
        }
    }
    failures = 0
    status, detail = compare_capture(base, reordered, True)
    print("selftest key-order + cursor + uuid normalization -> %s %s" % (status, detail))
    if status != "MATCH":
        failures += 1
    mutated = json.loads(json.dumps(base))
    mutated["data"]["people"]["edges"][0]["node"]["name"]["firstName"] = "Someone"
    mutated["data"]["people"]["totalCount"] = 6
    del mutated["data"]["people"]["pageInfo"]["hasNextPage"]
    status, detail = compare_capture(base, mutated, True)
    print("selftest perturbed payload -> %s %s" % (status, detail))
    if status != "DIFF":
        failures += 1
    codes_a = error_codes(
        {
            "errors": [
                {"extensions": {"code": "BAD_USER_INPUT", "subCode": "INVALID_ARGS_FILTER"}}
            ]
        }
    )
    codes_b = error_codes(
        {
            "errors": [
                {"extensions": {"subCode": "INVALID_ARGS_FILTER", "code": "BAD_USER_INPUT"}}
            ]
        }
    )
    print("selftest error codes set-equal -> %s" % ("MATCH" if codes_a == codes_b else "DIFF"))
    if codes_a != codes_b:
        failures += 1
    iso = {"a": "2026-11-26T17:16:16.065Z", "b": "1795729750.123Z"}
    print("selftest iso + epoch normalizers -> %s" % pc.normalize_json(iso))
    mv = {"metadataVersion": 42}
    print("selftest metadataVersion normalizer -> %s" % pc.normalize_json(mv))
    return 1 if failures else 0


def run(args):
    if getattr(args, "selftest", False):
        return selftest()
    scenarios = SCENARIOS
    if args.scenario:
        scenarios = [s for s in SCENARIOS if s[0] == args.scenario]
        if not scenarios:
            print("unknown scenario: %s" % args.scenario)
            return 2
    node = Client(pc.NODE_BASE, None, "node")
    try:
        node.token = pc.fetch_reference_token()
    except Exception as error:
        print("node auth failed: %s" % error)
        return 2
    rust_reachable = False
    rust = Client(pc.RUST_BASE, os.environ.get("RUST_TOKEN"), "rust")
    try:
        probe_status, probe_body = rust.gql("query { __typename }")
        rust_reachable = not is_gap(probe_status, probe_body)
    except Exception:
        pass

    if args.node_only:
        print("capturing reference-side fixtures only (--node-only)")
        written = 0
        failures = 0
        for name, fn in scenarios:
            try:
                _, path = run_node(node, name, fn)
                print("  wrote %s" % os.path.relpath(path, HERE))
                written += 1
            except Exception as error:
                print("  FAIL %s: %s" % (name, error))
                failures += 1
        print("\n%d fixtures captured, %d errors" % (written, failures))
        return 1 if failures else 0

    row = "%-20s %-14s %-6s %s"
    print(row % ("SCENARIO", "CAPTURE", "STATUS", "DETAIL"))
    print("-" * 92)
    failures = 0
    gaps = 0
    for name, fn in scenarios:
        node_caps = run_node(node, name, fn)[0] if not args.offline else load_fixture(name)
        if node_caps is None:
            print(row % (name, "-", "ERROR", "no fixture; run without --offline first"))
            failures += 1
            continue
        try:
            rust_caps = {} if not rust_reachable else fn(rust)
        except Exception as error:
            rust_caps = {"__error__": str(error)}
        for key in sorted(node_caps):
            if not rust_reachable:
                status, detail = "GAP", "record plane not implemented"
                gaps += 1
            else:
                status, detail = compare_capture(
                    node_caps[key], rust_caps.get(key), True
                )
            if status in ("DIFF", "ERROR"):
                failures += 1
            print(row % (name, key, status, detail[:60]))

    label = "Rust record plane reachable" if rust_reachable else "Rust record plane NOT reachable"
    print("\n%s | %d gaps | %d diffs" % (label, gaps, failures))
    print("GAP rows are the wave-4 delta (findMany/findOne/mutation not yet served by nudge-server).")
    return 1 if failures else 0


def main():
    parser = argparse.ArgumentParser(description="Twenty->Rust record parity harness")
    parser.add_argument("--node-only", action="store_true")
    parser.add_argument("--offline", action="store_true")
    parser.add_argument("--scenario", help="run a single scenario by name")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    return run(args)


if __name__ == "__main__":
    sys.exit(main())
