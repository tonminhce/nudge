import importlib.util
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
pc_spec = importlib.util.spec_from_file_location("pc", os.path.join(HERE, "parity-compare.py"))
pc = importlib.util.module_from_spec(pc_spec)
pc_spec.loader.exec_module(pc)
rp_spec = importlib.util.spec_from_file_location("rp", os.path.join(HERE, "record-parity.py"))
rp = importlib.util.module_from_spec(rp_spec)
rp_spec.loader.exec_module(rp)

node_tok, rust_tok, workspace = pc.fetch_parity_pair()
node = rp.Client(pc.NODE_BASE, node_tok, "node")
rust = rp.Client(pc.RUST_BASE, rust_tok, "rust")

reverse = (
    "query { companies(first: 2) { edges { node { name people(first: 2) { edges { node "
    "{ name { firstName } } } totalCount } } } } }"
)
_, nb = node.gql(reverse)
_, rb = rust.gql(reverse)
print("reverse/node", json.dumps(nb)[:300])
print("reverse/rust", json.dumps(rb)[:300])

_, cb = node.gql("query { companies(first: 2) { edges { node { id name } } } }")
edges = ((cb.get("data") or {}).get("companies") or {}).get("edges") or []
cid = edges[1]["node"]["id"] if len(edges) > 1 else (edges[0]["node"]["id"] if edges else "")
print("company_id", cid)

child_doc = (
    "mutation($d: CreatePersonInput!) { createPerson(data: $d) { id name { firstName } "
    "company { name } } }"
)
for label, company in [
    ("where", {"connect": {"where": {"id": cid}}}),
    ("bare", {"connect": {"id": cid}}),
]:
    for plane, client in [("node", node), ("rust", rust)]:
        variables = {
            "d": {
                "name": {"firstName": "Probe" + label.capitalize(), "lastName": "T"},
                "company": company,
            }
        }
        _, body = client.gql(child_doc, variables)
        print("child-%s/%s %s" % (label, plane, json.dumps(body)[:300]))
