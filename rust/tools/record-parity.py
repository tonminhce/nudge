import argparse
import importlib.util
import json
import os
import secrets
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
CAPTURE_DIR = os.environ.get("PARITY_CAPTURE_DIR") or os.path.join(HERE, "e2e-capture")


def _load_parity_compare():
    spec = importlib.util.spec_from_file_location(
        "parity_compare", os.path.join(HERE, "parity-compare.py")
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


pc = _load_parity_compare()

DIFF_DEPTH = int(os.environ.get("PARITY_DIFF_DEPTH", "4"))
DETAIL_WIDTH = int(os.environ.get("PARITY_DETAIL_WIDTH", "60"))
_MISSING = object()


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


SEED_COMPANIES = ["Acme Corp", "Initech"]
SEED_PEOPLE = [
    ("Ivan", "Zhao", "Acme Corp", "CEO"),
    ("Alice", "Chen", "Initech", "Engineer"),
    ("Bob", "Diaz", None, None),
]


def seed_if_empty(client):
    _, body = client.gql("query { people(first: 1) { edges { node { id } } } }")
    edges = ((body.get("data") or {}).get("people") or {}).get("edges") or []
    if edges:
        return 0
    company_ids = {}
    for name in SEED_COMPANIES:
        _, created = client.gql(
            "mutation($n: String!) { createCompany(data: { name: $n }) { id name } }",
            {"n": name},
        )
        node = (created.get("data") or {}).get("createCompany") or {}
        if node.get("id"):
            company_ids[name] = node["id"]
    seeded = 0
    for first, last, company, position in SEED_PEOPLE:
        data = {"name": {"firstName": first, "lastName": last}}
        if company and company_ids.get(company):
            data["company"] = {"connect": {"id": company_ids[company]}}
            if position:
                data["position"] = position
        elif position:
            data["position"] = position
        _, created = client.gql(
            "mutation($d: CreatePersonInput!) { createPerson(data: $d) { id } }",
            {"d": data},
        )
        if ((created.get("data") or {}).get("createPerson") or {}).get("id"):
            seeded += 1
    return seeded


SEEDED_PERSON_NAMES = ["Ivan", "Alice", "Bob"]


def resolve_person_id(client):
    for first in SEEDED_PERSON_NAMES:
        _, body = client.gql(
            "query($n: String) { people(filter: { name: { firstName: { eq: $n } } }, first: 1) "
            "{ edges { node { id } } } }",
            {"n": first},
        )
        edges = ((body.get("data") or {}).get("people") or {}).get("edges") or []
        if edges:
            return edges[0]["node"]["id"]
    _, body = client.gql("query { people(first: 1) { edges { node { id } } } }")
    edges = ((body.get("data") or {}).get("people") or {}).get("edges") or []
    if not edges:
        raise RuntimeError(f"{client.label}: no people seeded")
    return edges[0]["node"]["id"]


def resolve_company_name(client):
    for name in SEED_COMPANIES:
        _, body = client.gql(
            "query($n: String) { companies(filter: { name: { eq: $n } }) "
            "{ edges { node { name } } } }",
            {"n": name},
        )
        edges = ((body.get("data") or {}).get("companies") or {}).get("edges") or []
        if edges:
            return name
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


def scenario_filter_operators(client):
    captures = {}
    _, or_body = client.gql(
        'query { people(filter: { or: [{ name: { firstName: { eq: "Alice" } } }, '
        '{ name: { firstName: { eq: "Bob" } } }] }, first: 3) '
        "{ edges { node { name { firstName } } } totalCount } }"
    )
    captures["or"] = or_body
    _, null_body = client.gql(
        'query { people(filter: { position: { is: "NULL" } }, first: 3) '
        "{ edges { node { name { firstName } position } } totalCount } }"
    )
    captures["isNull"] = null_body
    _, ilike = client.gql(
        'query { people(filter: { name: { ilike: "%ha%" } }, first: 3) '
        "{ edges { node { id } } } }"
    )
    captures["compositeIlike"] = ilike
    captures["compositeIlikeCodes"] = error_codes(ilike)
    return captures


def scenario_bad_cursor(client):
    _, body = client.gql(
        'query { people(first: 2, after: "garbage") { edges { node { id } } } }'
    )
    return {"badCursor": body, "codes": error_codes(body)}


def scenario_reverse_relation(client):
    _, body = client.gql(
        'query { companies(filter: { name: { eq: "Acme Corp" } }) { edges { node { name '
        "people { edges { node { name { firstName } } } totalCount } } } } }"
    )
    return {"reverse": body}


def scenario_order_nulls_first(client):
    _, body = client.gql(
        "query { people(first: 3, orderBy: [{ name: { lastName: AscNullsFirst } }]) "
        "{ edges { node { name { lastName } } cursor } pageInfo { hasNextPage endCursor } } }"
    )
    return {"nullsFirst": body}


def scenario_rest_detail(client):
    person_id = resolve_person_id(client)
    list_status, list_body = client.rest("/rest/people/%s" % person_id)
    bad_status, bad_body = client.rest("/rest/nope?limit=1")
    return {
        "restSingle": {"statusCode": list_status, "body": list_body},
        "restUnknownObject": {"statusCode": bad_status, "body": bad_body},
    }


MERGE_FIELDS = """
    id
    name
    domainName { primaryLinkLabel primaryLinkUrl secondaryLinks { label url } }
    linkedinLink { primaryLinkLabel primaryLinkUrl secondaryLinks { label url } }
    accountOwner { id }
    deletedAt
"""


def _detag(value, tag):
    if isinstance(value, dict):
        return {key: _detag(item, tag) for key, item in value.items()}
    if isinstance(value, list):
        return [_detag(item, tag) for item in value]
    if isinstance(value, str):
        return value.replace(tag, "<tag>")
    return value


def scenario_merge_many(client):
    tag = "p" + secrets.token_hex(5)
    captures = {}
    _, created = client.gql(
        "mutation CreateCompanies($data: [CreateCompanyInput!]!) "
        "{ createCompanies(data: $data) { %s } }" % MERGE_FIELDS,
        {
            "data": [
                {
                    "name": "MergePrimary" + tag,
                    "domainName": {
                        "primaryLinkUrl": "https://%s.test" % tag,
                        "secondaryLinks": [
                            {"url": "https://%s-a.test" % tag, "label": "A"}
                        ],
                    },
                    "linkedinLink": {
                        "primaryLinkUrl": "https://linkedin.com/%s-primary" % tag,
                        "primaryLinkLabel": "Primary",
                        "secondaryLinks": [
                            {"url": "https://linkedin.com/%s-p1" % tag, "label": "P1"}
                        ],
                    },
                },
                {
                    "name": "MergeSecondary" + tag,
                    "domainName": {
                        "primaryLinkUrl": "https://%s-b.test" % tag,
                        "secondaryLinks": [
                            {"url": "https://%s-b1.test" % tag, "label": "B1"}
                        ],
                    },
                    "linkedinLink": {
                        "primaryLinkUrl": "https://linkedin.com/%s-secondary" % tag,
                        "secondaryLinks": [
                            {"url": "https://linkedin.com/%s-p1" % tag, "label": "P1"},
                            {"url": "https://linkedin.com/%s-s1" % tag, "label": "S1"},
                        ],
                    },
                },
            ]
        },
    )
    companies = (created.get("data") or {}).get("createCompanies") or []
    captures["create"] = {"count": len(companies)}
    if len(companies) != 2:
        captures["abort"] = {"error": "create failed", "codes": error_codes(created)}
        return _detag(captures, tag)
    primary_id = companies[0]["id"]
    secondary_id = companies[1]["id"]
    if not primary_id or not secondary_id:
        captures["abort"] = {"error": "ids missing"}
        return _detag(captures, tag)
    _, person = client.gql(
        "mutation CreatePerson($data: CreatePersonInput!) { createPerson(data: $data) "
        "{ id name { firstName } company { name } } }",
        {
            "data": {
                "name": {"firstName": "MergeChild", "lastName": tag},
                "company": {"connect": {"id": secondary_id}},
            }
        },
    )
    captures["childBefore"] = person
    merge_document = (
        "mutation MergeCompanies($ids: [UUID!]!, $index: Int!, $dryRun: Boolean!) "
        "{ mergeCompanies(ids: $ids, conflictPriorityIndex: $index, dryRun: $dryRun) "
        "{ %s } }" % MERGE_FIELDS
    )
    _, dry = client.gql(
        merge_document,
        {"ids": [primary_id, secondary_id], "index": 0, "dryRun": True},
    )
    captures["dryRun"] = dry
    _, merged = client.gql(
        merge_document,
        {"ids": [primary_id, secondary_id], "index": 0, "dryRun": False},
    )
    captures["merge"] = merged
    child_id = ((person.get("data") or {}).get("createPerson") or {}).get("id") or ""
    captures["childAfter"] = client.gql(
        "query($id: UUID!) { person(filter: { id: { eq: $id } }) "
        "{ id name { firstName } company { name } } }",
        {"id": child_id},
    )[1]
    captures["secondaryAfter"] = client.gql(
        "query($id: UUID!) { company(filter: { id: { eq: $id } }) { id deletedAt } }",
        {"id": secondary_id},
    )[1]
    captures["singleId"] = client.gql(
        merge_document, {"ids": [primary_id], "index": 0, "dryRun": False}
    )[1]
    captures["badIndex"] = client.gql(
        merge_document,
        {"ids": [primary_id, secondary_id], "index": 7, "dryRun": False},
    )[1]
    captures["unknownId"] = client.gql(
        merge_document,
        {
            "ids": [primary_id, "00000000-0000-4000-8000-000000000000"],
            "index": 0,
            "dryRun": False,
        },
    )[1]
    client.gql(
        "mutation($id: UUID!) { destroyCompany(id: $id) { id } }", {"id": primary_id}
    )
    client.gql(
        "mutation($id: UUID!) { destroyPerson(id: $id) { id } }",
        {"id": child_id},
    )
    return _detag(captures, tag)


SCENARIOS = [
    ("findMany-first2", scenario_find_many),
    ("findOne", scenario_find_one),
    ("company-filter", scenario_company_filter),
    ("orderBy-pagination", scenario_order_pagination),
    ("mutation-roundtrip", scenario_mutation_round_trip),
    ("merge-many", scenario_merge_many),
    ("error-bad-filter", scenario_error_filter),
    ("rest-findMany", scenario_rest_find_many),
    ("filter-operators", scenario_filter_operators),
    ("bad-cursor", scenario_bad_cursor),
    ("reverse-relation", scenario_reverse_relation),
    ("order-nulls-first", scenario_order_nulls_first),
    ("rest-detail", scenario_rest_detail),
]


def is_gap(status, body):
    if status == 404:
        return True
    if isinstance(body, dict) and body.get("message", "").startswith("Cannot "):
        return True
    codes = error_codes(body) if isinstance(body, dict) else []
    return "UNAUTHENTICATED" in codes and not (body.get("data") if isinstance(body, dict) else None)


ABSENT_CODES = {
    "GRAPHQL_VALIDATION_FAILED",
    "GRAPHQL_PARSE_FAILED",
    "FEATURE_NOT_SUPPORTED",
    "METHOD_NOT_ALLOWED",
    "NOT_FOUND",
}

ABSENT_MARKERS = ("Cannot ", "does not exist", "Unknown type", "Cannot query field")


def missing_feature(body):
    if not isinstance(body, dict):
        return None
    if body.get("statusCode") in (404, 501):
        return "rest %s" % body["statusCode"]
    for err in body.get("errors") or []:
        ext = err.get("extensions") or {}
        code = ext.get("code")
        message = err.get("message") or ""
        if code in ABSENT_CODES:
            return code
        if any(marker in message for marker in ABSENT_MARKERS):
            return code or "unknown_field"
    return None


def compare_capture(node_val, rust_val, rust_reachable):
    if not rust_reachable:
        return "GAP", "record endpoint not implemented on Rust server"
    if rust_val is _MISSING:
        return "GAP", "Rust scenario never answered this capture"
    node_norm = pc.normalize_json(node_val)
    rust_norm = pc.normalize_json(rust_val)
    if node_norm == rust_norm:
        return "MATCH", ""
    diffs = pc.diff_paths(node_norm, rust_norm)
    preview = "; ".join(
        "%s: node=%s rust=%s" % (p, json.dumps(l)[:48], json.dumps(r)[:48])
        for p, l, r in diffs[:DIFF_DEPTH]
    )
    if len(diffs) > DIFF_DEPTH:
        preview += "; +%d more" % (len(diffs) - DIFF_DEPTH)
    hint = missing_feature(rust_val)
    if hint:
        preview = "[absent: %s] %s" % (hint, preview)
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
    absent_body = {
        "errors": [{"message": "Cannot query field \"company\" on type \"Person\"", "extensions": {"code": "GRAPHQL_VALIDATION_FAILED"}}]
    }
    other_body = {"errors": [{"message": "boom", "extensions": {"code": "INTERNAL"}}]}
    print("selftest missing_feature validation error -> %s" % missing_feature(absent_body))
    print("selftest missing_feature internal error -> %s" % missing_feature(other_body))
    print("selftest missing_feature rest 404 -> %s" % missing_feature({"statusCode": 404, "message": "Cannot .."}))
    if missing_feature(absent_body) != "GRAPHQL_VALIDATION_FAILED":
        failures += 1
    if missing_feature(other_body) is not None:
        failures += 1
    absent_diff = compare_capture({"data": {"x": 1}}, absent_body, True)
    print("selftest absent diff keeps DIFF but hints -> %s" % absent_diff[0])
    if absent_diff[0] != "DIFF" or not absent_diff[1].startswith("[absent:"):
        failures += 1
    unanswered = compare_capture({"data": {"x": 1}}, _MISSING, True)
    print("selftest unanswered capture -> %s %s" % unanswered)
    if unanswered[0] != "GAP":
        failures += 1
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
    rust_token = os.environ.get("RUST_TOKEN")
    node_token = os.environ.get("NODE_TOKEN")
    if node_token and rust_token:
        node.token = node_token
    else:
        try:
            pair_node, pair_rust, workspace_id = pc.fetch_parity_pair()
            node.token = node_token or pair_node
            rust_token = rust_token or pair_rust
            print("parity workspace shared by both servers: %s" % workspace_id)
        except Exception as error:
            print("node auth failed: %s" % error)
            return 2
    rust_reachable = False
    rust = Client(pc.RUST_BASE, rust_token, "rust")
    try:
        probe_status, probe_body = rust.gql("query { __typename }")
        rust_reachable = not is_gap(probe_status, probe_body)
    except Exception:
        pass

    if not args.offline:
        try:
            seeded = seed_if_empty(node)
            if seeded:
                print("seeded %d rows into the shared parity workspace" % seeded)
        except Exception as error:
            print("seeding failed: %s" % error)

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
    absent = 0
    for name, fn in scenarios:
        node_caps = run_node(node, name, fn)[0] if not args.offline else load_fixture(name)
        if node_caps is None:
            print(row % (name, "-", "ERROR", "no fixture; run without --offline first"))
            failures += 1
            continue
        crash = None
        try:
            rust_caps = {} if not rust_reachable else fn(rust)
        except Exception as error:
            crash = "%s: %s" % (type(error).__name__, error)
            rust_caps = {}
        for key in sorted(set(node_caps) | set(rust_caps)):
            if not rust_reachable:
                status, detail = "GAP", "record plane not implemented"
                gaps += 1
            elif crash:
                status, detail = "ERROR", "rust scenario raised: %s" % crash
            elif key not in rust_caps:
                status, detail = "GAP", "rust never answered this capture"
                gaps += 1
            elif key not in node_caps:
                status, detail = "DIFF", "rust-only capture: %s" % json.dumps(
                    rust_caps[key]
                )[:48]
            else:
                status, detail = compare_capture(node_caps[key], rust_caps[key], True)
            if status == "DIFF" and detail.startswith("[absent:"):
                absent += 1
            if status in ("DIFF", "ERROR"):
                failures += 1
            print(row % (name, key, status, detail[:DETAIL_WIDTH]))

    label = "Rust record plane reachable" if rust_reachable else "Rust record plane NOT reachable"
    print("\n%s | %d gaps | %d diffs" % (label, gaps, failures))
    print("GAP means the record plane never answered; DIFF means it answered differently from 2.61.0.")
    print("%d of the diffs name a field, column or endpoint the Rust plane does not implement." % absent)
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
