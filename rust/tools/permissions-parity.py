import importlib.util
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
SPEC = importlib.util.spec_from_file_location("parity_compare", os.path.join(HERE, "parity-compare.py"))
PARITY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PARITY)

QUERY = """
{
  currentUser {
    id
    currentUserWorkspace {
      id
      permissionFlags
      objectRecordsPermissions {
        objectMetadataId
        canCreateAllRecords
        canReadAllRecords
        canUpdateAllRecords
        canSoftDeleteAllRecords
        canDestroyAllRecords
        unreadableFields
        uneditableFields
        canReadSomeRecords
      }
    }
  }
}
"""


def fetch(base, token):
    status, body = PARITY.graphql(base, QUERY, token, path="/metadata")
    payload = json.loads(body)
    if status != 200 or payload.get("errors"):
        raise RuntimeError(f"{base}: status={status} errors={payload.get('errors')}")
    return payload["data"]["currentUser"]


def normalize(user):
    workspace = user["currentUserWorkspace"]
    objects = sorted(workspace["objectRecordsPermissions"], key=lambda item: item["objectMetadataId"])
    for item in objects:
        item["unreadableFields"] = sorted(item["unreadableFields"])
        item["uneditableFields"] = sorted(item["uneditableFields"])
    return {
        "id": user["id"],
        "workspace": {
            "id": workspace["id"],
            "permissionFlags": sorted(workspace["permissionFlags"]),
            "objectRecordsPermissions": objects,
        },
    }


def main():
    token = PARITY.fetch_reference_token()
    reference = normalize(fetch(PARITY.NODE_BASE, token))
    nudge = normalize(fetch(PARITY.RUST_BASE, token))
    differences = PARITY.diff_paths(reference, nudge)
    if differences:
        for path, expected, actual in differences[:50]:
            print(f"{path}: reference={expected!r} nudge={actual!r}")
        print(f"FAIL: {len(differences)} permission differences")
        return 1
    print(
        "MATCH: currentUserWorkspace permissions "
        f"({len(reference['workspace']['permissionFlags'])} flags, "
        f"{len(reference['workspace']['objectRecordsPermissions'])} objects)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
