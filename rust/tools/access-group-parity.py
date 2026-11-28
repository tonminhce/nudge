import argparse
import importlib.util
import json
import os
import re
import subprocess
import sys
import uuid

HERE = os.path.dirname(os.path.abspath(__file__))
SPEC = importlib.util.spec_from_file_location("parity_compare", os.path.join(HERE, "parity-compare.py"))
PARITY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PARITY)

STATE_PATH = os.environ.get("NUDGE_PARITY_STATE", "/tmp/nudge-record-parity-state.json")
DB_CONTAINER = os.environ.get("NUDGE_DB_CONTAINER", "infra-db-1")
DB_NAME = os.environ.get("NUDGE_DB_NAME", "twenty_ref")
REDIS_CONTAINER = os.environ.get("NUDGE_REDIS_CONTAINER", "infra-redis-1")
REDIS_DB = os.environ.get("NUDGE_REDIS_DB", "1")
REDIS_PREFIX = os.environ.get("NUDGE_REF_REDIS_PREFIX", "twentyref")
JOURNAL_PATH = os.environ.get("NUDGE_ACCESS_GROUP_JOURNAL", "/tmp/nudge-access-group-journal.json")

UUID_RE = re.compile(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$", re.I)

CACHE_KEYS = [
    "featureFlagsMap",
    "rolesPermissions",
    "flatRoleMaps",
    "flatRoleTargetMaps",
    "flatRowLevelPermissionRuleMaps",
    "flatObjectPermissionMaps",
    "flatFieldPermissionMaps",
    "flatPermissionFlagMaps",
    "flatRolePermissionFlagMaps",
    "rlsWorkspaceMemberRelationValueMaps",
]

COMPANY_QUERY = """
query parityCompanies($first: Int) {
  companies(first: $first) {
    edges { node { id name } }
    totalCount
  }
}
"""


def check_uuid(value, label):
    if not UUID_RE.match(str(value or "")):
        raise RuntimeError(f"{label} is not a uuid")
    return str(value)


def load_state():
    with open(STATE_PATH, encoding="utf-8") as handle:
        state = json.load(handle)
    for key in ("email", "password"):
        if not state.get(key):
            raise RuntimeError(f"{STATE_PATH} is missing {key}")
    return state


def sql(statement, label):
    result = subprocess.run(
        ["docker", "exec", DB_CONTAINER, "psql", "-U", "postgres", "-d", DB_NAME, "-Atc", statement],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if result.returncode != 0:
        raise RuntimeError(f"{label}: {result.stderr.strip()[:200]}")
    return [line for line in result.stdout.splitlines() if line != ""]


def quote(value):
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return str(value)
    return "'" + str(value).replace("'", "''") + "'"


def redis_flush(workspace_id):
    scan = subprocess.run(
        [
            "docker", "exec", REDIS_CONTAINER, "redis-cli", "-n", str(REDIS_DB),
            "--scan", "--pattern", f"*:{workspace_id}*",
        ],
        capture_output=True,
        text=True,
        timeout=60,
    )
    if scan.returncode != 0:
        raise RuntimeError(f"redis scan failed: {scan.stderr.strip()[:200]}")
    keys = [line.strip() for line in scan.stdout.splitlines() if line.strip()]
    for name in CACHE_KEYS:
        keys.append(f"{REDIS_PREFIX}:engine:workspace:{name}:{workspace_id}:hash")
        keys.append(f"{REDIS_PREFIX}:engine:workspace:{name}:{workspace_id}:data")
    keys = sorted(set(keys))
    deleted = 0
    for chunk in [keys[i:i + 60] for i in range(0, len(keys), 60)]:
        result = subprocess.run(
            ["docker", "exec", REDIS_CONTAINER, "redis-cli", "-n", str(REDIS_DB), "DEL", *chunk],
            capture_output=True,
            text=True,
            timeout=30,
        )
        if result.returncode != 0:
            raise RuntimeError(f"redis flush failed: {result.stderr.strip()[:200]}")
        deleted += sum(int(value) for value in result.stdout.split())
    return deleted


def nudge_login(state, base):
    workspace_id = state.get("workspaceId") or ""
    for _ in range(2):
        status, body = PARITY.request_raw(
            base,
            "POST",
            "/auth/login-token",
            json.dumps(
                {
                    "email": state["email"],
                    "password": state["password"],
                    "workspaceId": workspace_id,
                }
            ),
        )
        if status == 200:
            break
        contexts = sql(
            "select uw.\"workspaceId\" from core.\"userWorkspace\" uw "
            "join core.\"user\" u on u.id = uw.\"userId\" "
            f"where u.email = {quote(state['email'])} limit 1",
            "workspace lookup",
        )
        if not contexts:
            raise RuntimeError(f"login-token failed: status={status}")
        workspace_id = contexts[0]
    token_body = json.loads(body)
    status, body = PARITY.request_raw(
        base,
        "POST",
        "/auth/tokens",
        json.dumps({"loginToken": token_body["loginToken"], "workspaceId": workspace_id}),
    )
    if status != 200:
        raise RuntimeError(f"auth/tokens failed: status={status}")
    return json.loads(body)["accessToken"]


def create_account(prefix, names):
    email = f"{prefix}-{uuid.uuid4().hex[:24]}@nudge.test"
    password = "Ag-" + uuid.uuid4().hex
    status, body = PARITY.request_raw(
        PARITY.RUST_BASE,
        "POST",
        "/auth/sign-up",
        json.dumps({"email": email, "password": password}),
    )
    if status != 201:
        raise RuntimeError(f"sign-up failed: status={status} body={body[:160]}")
    payload = json.loads(body)
    token = payload["accessToken"]
    for name in names:
        mutation = (
            "mutation newCompany($name: String!) { createCompany(data: { name: $name }) { id name } }"
        )
        status, body = PARITY.graphql(
            PARITY.RUST_BASE, mutation, token, variables={"name": name}, path="/graphql"
        )
        created = json.loads(body)
        if not ((created.get("data") or {}).get("createCompany") or {}).get("id"):
            raise RuntimeError(f"company seed failed for {name}: {body[:160]}")
    state = {
        "email": email,
        "password": password,
        "workspaceId": payload["workspaceId"],
        "token": token,
        "names": list(names),
    }
    handle = os.open(STATE_PATH, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(handle, "w", encoding="utf-8") as stream:
        json.dump(state, stream)
    return state


def auth_pair():
    state = load_state()
    token = state.get("token")
    if token:
        probe = "query ctx { currentUser { id } }"
        status, body = PARITY.graphql(
            PARITY.NODE_BASE, probe, token, path="/metadata"
        )
        if status == 200 and json.loads(body).get("data"):
            return state, token
    if os.environ.get("TWENTY_REFERENCE_PASSWORD"):
        PARITY.REFERENCE_PASSWORD = os.environ["TWENTY_REFERENCE_PASSWORD"]
        try:
            return state, PARITY.fetch_reference_token()
        except RuntimeError:
            pass
    return state, nudge_login(state, PARITY.RUST_BASE)


def current_context(token):
    query = "query ctx { currentUser { id currentUserWorkspace { id } } }"
    status, body = PARITY.graphql(PARITY.NODE_BASE, query, token, path="/metadata")
    payload = json.loads(body)
    if status != 200 or payload.get("errors"):
        raise RuntimeError(f"currentUser failed: status={status} errors={payload.get('errors')}")
    user = payload["data"]["currentUser"]
    user_workspace_id = check_uuid(user["currentUserWorkspace"]["id"], "user workspace id")
    workspaces = sql(
        "select uw.\"workspaceId\", w.\"databaseSchema\" from core.\"userWorkspace\" uw "
        "join core.\"workspace\" w on w.id = uw.\"workspaceId\" "
        f"where uw.id = {quote(user_workspace_id)}",
        "user workspace lookup",
    )
    if not workspaces or "|" not in workspaces[0]:
        raise RuntimeError(f"user workspace {user_workspace_id} has no core row")
    workspace_id, schema = workspaces[0].split("|")
    return {
        "userId": check_uuid(user["id"], "user id"),
        "userWorkspaceId": user_workspace_id,
        "workspaceId": check_uuid(workspace_id, "workspace id"),
        "schema": schema,
    }


def company_metadata(ctx):
    rows = sql(
        "select o.id, f.id from core.\"objectMetadata\" o "
        "join core.\"fieldMetadata\" f on f.\"objectMetadataId\" = o.id "
        f"where o.\"workspaceId\" = {quote(ctx['workspaceId'])} "
        "and o.\"nameSingular\" = 'company' and f.name = 'name' limit 1",
        "company metadata lookup",
    )
    if not rows or "|" not in rows[0]:
        raise RuntimeError("no company/name metadata for this workspace")
    object_id, field_id = rows[0].split("|")
    return check_uuid(object_id, "company object id"), check_uuid(field_id, "company name field id")


def visible_companies(base, token, limit=200):
    status, body = PARITY.graphql(
        base, COMPANY_QUERY, token, variables={"first": limit}, path="/graphql"
    )
    payload = json.loads(body)
    if status != 200 or payload.get("errors"):
        raise RuntimeError(f"{base}: status={status} errors={payload.get('errors')}")
    collection = payload["data"].get("companies") or {}
    ids = sorted(edge["node"]["id"] for edge in collection.get("edges") or [])
    return ids, collection.get("totalCount")


def inventory(ctx):
    workspace_id = ctx["workspaceId"]
    flags = sql(
        "select key, value from core.\"featureFlag\" "
        f"where \"workspaceId\" = {quote(workspace_id)} order by key",
        "feature flags",
    )
    rules = sql(
        "select r.id, r.\"roleId\", r.\"canReadRecords\", "
        "(select count(*) from core.\"rowLevelPermissionPredicate\" p "
        "where p.\"rowLevelPermissionRuleId\" = r.id) "
        "from core.\"rowLevelPermissionRule\" r "
        f"where r.\"workspaceId\" = {quote(workspace_id)} order by r.\"createdAt\"",
        "rls rules",
    )
    targets = sql(
        "select rt.\"userWorkspaceId\", rt.\"roleId\", r.label from core.\"roleTarget\" rt "
        "join core.role r on r.id = rt.\"roleId\" "
        f"where rt.\"workspaceId\" = {quote(workspace_id)} order by r.label",
        "role targets",
    )
    groups = sql(
        f'select count(*) from "{ctx["schema"]}"."accessGroup"',
        "access groups",
    )
    stamped = sql(
        "select count(*) filter (where \"_accessGroupIds\" is not null), count(*) "
        f"from \"{ctx['schema']}\".company",
        "company access group arrays",
    )
    return {
        "flags": flags,
        "rules": rules,
        "targets": targets,
        "accessGroups": groups[0] if groups else "?",
        "stamped": stamped[0] if stamped else "?",
    }


def owner_target(ctx):
    rows = sql(
        "select rt.id, rt.\"roleId\", r.label, rt.\"userWorkspaceId\", rt.\"workspaceId\", "
        "coalesce(r.\"applicationId\", rt.\"applicationId\") "
        "from core.\"roleTarget\" rt join core.\"userWorkspace\" uw on uw.id = rt.\"userWorkspaceId\" "
        "join core.role r on r.id = rt.\"roleId\" "
        "where rt.\"userWorkspaceId\" is not null "
        f"and uw.\"userId\" = {quote(ctx['userId'])} "
        f"and uw.\"workspaceId\" = {quote(ctx['workspaceId'])} "
        "order by r.\"createdAt\" limit 1",
        "owner role target",
    )
    if not rows or "|" not in rows[0]:
        raise RuntimeError("the signed-in user has no role target in this workspace")
    parts = rows[0].split("|")
    target_id, role_id, label = parts[0], parts[1], parts[2]
    user_workspace_id = parts[3] if len(parts) > 3 else ""
    target_workspace_id = parts[4] if len(parts) > 4 else ""
    application_id = parts[5] if len(parts) > 5 else ""
    return {
        "targetId": check_uuid(target_id, "role target id"),
        "roleId": check_uuid(role_id, "role id"),
        "label": label,
        "userWorkspaceId": check_uuid(user_workspace_id, "target user workspace id")
        if user_workspace_id
        else None,
        "workspaceId": check_uuid(target_workspace_id, "target workspace id"),
        "applicationId": check_uuid(application_id, "role application id") if application_id else None,
    }


def write_journal(journal):
    handle = os.open(JOURNAL_PATH, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(handle, "w", encoding="utf-8") as stream:
        json.dump(journal, stream)


def read_journal():
    if not os.path.exists(JOURNAL_PATH):
        return None
    with open(JOURNAL_PATH, encoding="utf-8") as stream:
        return json.load(stream)


def provision(ctx, value, access_groups, predicate_field):
    existing = read_journal()
    if existing:
        raise RuntimeError(f"{JOURNAL_PATH} already holds a provisioned state; restore first")
    object_id, name_field_id = company_metadata(ctx)
    field_id = name_field_id if predicate_field == "name" else predicate_field
    workspace_id = ctx["workspaceId"]
    target = owner_target(ctx)
    new_role = str(uuid.uuid4())

    sql(
        "insert into core.role (id, \"universalIdentifier\", \"workspaceId\", label, description, icon, "
        "\"canUpdateAllSettings\", \"canAccessAllTools\", \"canReadAllObjectRecords\", "
        "\"canUpdateAllObjectRecords\", \"canSoftDeleteAllObjectRecords\", \"canDestroyAllObjectRecords\", "
        "\"canCreateAllObjectRecords\", \"isEditable\", \"canBeAssignedToUsers\", \"canBeAssignedToAgents\", "
        "\"canBeAssignedToApiKeys\", \"canBeAssignedToApplications\", \"isSystemSideEffect\", "
        "\"canAccessAllAgentChatChannels\", \"applicationId\", \"createdAt\", \"updatedAt\") "
        f"select {quote(new_role)}, gen_random_uuid(), \"workspaceId\", 'Parity RLS', "
        "'temporary parity probe role', 'IconLock', "
        "\"canUpdateAllSettings\", \"canAccessAllTools\", \"canReadAllObjectRecords\", "
        "\"canUpdateAllObjectRecords\", \"canSoftDeleteAllObjectRecords\", \"canDestroyAllObjectRecords\", "
        "\"canCreateAllObjectRecords\", true, \"canBeAssignedToUsers\", \"canBeAssignedToAgents\", "
        "\"canBeAssignedToApiKeys\", \"canBeAssignedToApplications\", false, "
        "\"canAccessAllAgentChatChannels\", \"applicationId\", now(), now() "
        f"from core.role where id = {quote(target['roleId'])}",
        "role insert",
    )
    copied = sql(
        "insert into core.\"objectPermission\" (id, \"universalIdentifier\", \"workspaceId\", \"roleId\", "
        "\"objectMetadataId\", \"canReadObjectRecords\", \"canUpdateObjectRecords\", "
        "\"canSoftDeleteObjectRecords\", \"canDestroyObjectRecords\", \"canCreateObjectRecords\", "
        "\"applicationId\", \"createdAt\", \"updatedAt\") "
        "select gen_random_uuid(), gen_random_uuid(), \"workspaceId\", "
        f"{quote(new_role)}, \"objectMetadataId\", \"canReadObjectRecords\", \"canUpdateObjectRecords\", "
        "\"canSoftDeleteObjectRecords\", \"canDestroyObjectRecords\", \"canCreateObjectRecords\", "
        f"\"applicationId\", now(), now() from core.\"objectPermission\" where \"roleId\" = {quote(target['roleId'])}",
        "object permission copy",
    )
    sql(
        "insert into core.\"objectPermission\" (id, \"universalIdentifier\", \"workspaceId\", \"roleId\", "
        "\"objectMetadataId\", \"canReadObjectRecords\", \"canUpdateObjectRecords\", "
        "\"canSoftDeleteObjectRecords\", \"canDestroyObjectRecords\", \"canCreateObjectRecords\", "
        "\"applicationId\", \"createdAt\", \"updatedAt\") "
        "select gen_random_uuid(), gen_random_uuid(), om.\"workspaceId\", "
        f"{quote(new_role)}, om.id, true, true, true, true, true, om.\"applicationId\", now(), now() "
        "from core.\"objectMetadata\" om "
        f"where om.\"workspaceId\" = {quote(workspace_id)} and om.\"isActive\" is true "
        "and not exists (select 1 from core.\"objectPermission\" op "
        f"where op.\"roleId\" = {quote(new_role)} and op.\"objectMetadataId\" = om.id)",
        "object permission grant",
    )
    granted = sql(
        "select count(*) from core.\"objectPermission\" where \"roleId\" = " + quote(new_role),
        "granted object permissions",
    )
    rule = str(uuid.uuid4())
    sql(
        "insert into core.\"rowLevelPermissionRule\" (id, \"universalIdentifier\", \"workspaceId\", "
        "\"roleId\", \"objectMetadataId\", \"canReadRecords\", \"canCreateRecords\", \"canUpdateRecords\", "
        "\"canSoftDeleteRecords\", \"canDestroyRecords\", \"applicationId\", \"createdAt\", \"updatedAt\") "
        f"values ({quote(rule)}, gen_random_uuid(), {quote(workspace_id)}, {quote(new_role)}, "
        f"{quote(object_id)}, true, false, false, false, false, {quote(target['applicationId'])}, now(), now())",
        "rule insert",
    )
    group = str(uuid.uuid4())
    sql(
        "insert into core.\"rowLevelPermissionPredicateGroup\" (id, \"universalIdentifier\", \"workspaceId\", "
        "\"roleId\", \"objectMetadataId\", \"rowLevelPermissionRuleId\", \"logicalOperator\", "
        "\"positionInRowLevelPermissionPredicateGroup\", \"canReadRecords\", \"canCreateRecords\", "
        "\"canUpdateRecords\", \"canSoftDeleteRecords\", \"canDestroyRecords\", \"applicationId\", "
        "\"createdAt\", \"updatedAt\") "
        f"values ({quote(group)}, gen_random_uuid(), {quote(workspace_id)}, {quote(new_role)}, "
        f"{quote(object_id)}, {quote(rule)}, 'AND', 0, true, false, false, false, false, "
        f"{quote(target['applicationId'])}, now(), now())",
        "predicate group insert",
    )
    sql(
        "insert into core.\"rowLevelPermissionPredicate\" (id, \"universalIdentifier\", \"workspaceId\", "
        "\"roleId\", \"rowLevelPermissionRuleId\", \"rowLevelPermissionPredicateGroupId\", "
        "\"objectMetadataId\", \"fieldMetadataId\", operand, value, "
        "\"positionInRowLevelPermissionPredicateGroup\", \"applicationId\", \"createdAt\", \"updatedAt\") "
        f"values (gen_random_uuid(), gen_random_uuid(), {quote(workspace_id)}, {quote(new_role)}, "
        f"{quote(rule)}, {quote(group)}, {quote(object_id)}, {quote(field_id)}, 'IS', "
        f"{quote(json.dumps(value))}::jsonb, 0, {quote(target['applicationId'])}, now(), now())",
        "predicate insert",
    )
    sql(
        f"update core.\"roleTarget\" set \"roleId\" = {quote(new_role)} where id = {quote(target['targetId'])}",
        "role target switch",
    )
    flags = []
    if access_groups:
        for key in ("IS_ACCESS_GROUPS_ENABLED",):
            sql(
                "insert into core.\"featureFlag\" (id, key, value, \"workspaceId\", \"createdAt\", \"updatedAt\") "
                f"values (gen_random_uuid(), {quote(key)}, true, {quote(workspace_id)}, now(), now()) "
                "on conflict (\"workspaceId\", key) do update set value = true, \"updatedAt\" = now()",
                "feature flag insert",
            )
            flags.append(key)

    journal = {
        "workspaceId": workspace_id,
        "schema": ctx["schema"],
        "roleId": new_role,
        "ruleId": rule,
        "groupId": group,
        "targetId": target["targetId"],
        "targetUserWorkspaceId": target["userWorkspaceId"],
        "targetWorkspaceId": target["workspaceId"],
        "targetApplicationId": target["applicationId"],
        "previousRoleId": target["roleId"],
        "previousRoleLabel": target["label"],
        "flags": flags,
        "value": value,
        "objectPermissionRows": granted,
    }
    write_journal(journal)
    flushed = redis_flush(workspace_id)
    return journal, flushed


def restore():
    journal = read_journal()
    if not journal:
        raise RuntimeError(f"no journal at {JOURNAL_PATH}")
    repoint = sql(
        "update core.\"roleTarget\" set \"roleId\" = "
        + quote(journal["previousRoleId"])
        + ", \"updatedAt\" = now() where id = "
        + quote(journal["targetId"]),
        "role target restore",
    )
    if not repoint or repoint[0] != "UPDATE 1":
        sql(
            "insert into core.\"roleTarget\" (id, \"workspaceId\", \"roleId\", \"userWorkspaceId\", "
            "\"universalIdentifier\", \"applicationId\", \"createdAt\", \"updatedAt\") values ("
            f"{quote(journal['targetId'])}, {quote(journal['targetWorkspaceId'])}, "
            f"{quote(journal['previousRoleId'])}, {quote(journal['targetUserWorkspaceId'])}, "
            "gen_random_uuid(), "
            f"{quote(journal['targetApplicationId'])}, now(), now()) "
            "on conflict (id) do update set \"roleId\" = excluded.\"roleId\", \"updatedAt\" = now()",
            "role target repair",
        )
    for statement, label in [
        (
            "delete from core.\"rowLevelPermissionPredicate\" where \"rowLevelPermissionRuleId\" = "
            + quote(journal["ruleId"]),
            "predicate delete",
        ),
        (
            "delete from core.\"rowLevelPermissionPredicateGroup\" where \"rowLevelPermissionRuleId\" = "
            + quote(journal["ruleId"]),
            "predicate group delete",
        ),
        (
            "delete from core.\"rowLevelPermissionRule\" where id = " + quote(journal["ruleId"]),
            "rule delete",
        ),
        (
            "delete from core.\"objectPermission\" where \"roleId\" = " + quote(journal["roleId"]),
            "object permission delete",
        ),
        (
            "delete from core.role where id = " + quote(journal["roleId"]),
            "role delete",
        ),
    ]:
        sql(statement, label)
    for key in journal.get("flags") or []:
        sql(
            "delete from core.\"featureFlag\" where \"workspaceId\" = "
            + quote(journal["workspaceId"])
            + " and key = "
            + quote(key),
            "feature flag delete",
        )
    flushed = redis_flush(journal["workspaceId"])
    os.unlink(JOURNAL_PATH)
    return flushed


def outcome(base, token, limit):
    try:
        ids, total = visible_companies(base, token, limit)
    except RuntimeError as error:
        text = str(error)
        if "PERMISSION_DENIED" in text:
            return ("denied", None)
        raise
    return ("visible", tuple(sorted(ids)))


def compare(ctx, token, nudge_base, expect_filtered, limit):
    ref = outcome(PARITY.NODE_BASE, token, limit)
    nudge = outcome(nudge_base, token, limit)
    print(f"reference :3001 {ref[0]} n={len(ref[1]) if ref[1] else 0}")
    print(f"nudge     {nudge_base} {nudge[0]} n={len(nudge[1]) if nudge[1] else 0}")
    if ref != nudge:
        if ref[1] and nudge[1]:
            for value in sorted(set(ref[1]) - set(nudge[1]))[:10]:
                print(f"  only in reference: {value}")
            for value in sorted(set(nudge[1]) - set(ref[1]))[:10]:
                print(f"  only in nudge:     {value}")
        print("FAIL: reference and nudge disagree on visibility")
        return 1
    if ref[0] == "denied":
        print("MATCH: both servers deny the collection read for this role")
        return 0
    if expect_filtered and len(ref[1]) == 0:
        print("WEAK: both servers hid every company, the rule may not be loaded")
        return 1
    print(f"MATCH: identical visible company sets ({len(ref[1])} records)")
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--provision", action="store_true")
    parser.add_argument("--restore", action="store_true")
    parser.add_argument("--access-groups", action="store_true")
    parser.add_argument("--value", default=None)
    parser.add_argument("--predicate-field", default="name")
    parser.add_argument("--nudge-base", default=PARITY.RUST_BASE)
    parser.add_argument("--limit", type=int, default=200)
    parser.add_argument("--flush-only", action="store_true")
    parser.add_argument("--signup", default=None)
    parser.add_argument("--names", default="Ag Visible,Ag Hidden One,Ag Hidden Two")
    args = parser.parse_args()

    if args.restore:
        flushed = restore()
        print(f"restored, flushed {flushed} reference cache keys")
        return 0

    if args.signup:
        create_account(args.signup, [name.strip() for name in args.names.split(",")])

    state, token = auth_pair()
    try:
        ctx = current_context(token)
    except RuntimeError as error:
        workspace_id = state.get("workspaceId")
        if not workspace_id:
            raise
        redis_flush(workspace_id)
        ctx = current_context(token)
    print(f"workspace={ctx['workspaceId']} schema={ctx['schema']}")

    if args.flush_only:
        print(f"flushed {redis_flush(ctx['workspaceId'])} reference cache keys")
        return 0

    for label, entries in inventory(ctx).items():
        print(f"{label}: {entries}")

    expect_filtered = False
    if args.provision:
        value = args.value or (state.get("names") or [None])[0]
        if not value:
            raise RuntimeError("--provision needs --value <company name>")
        journal, flushed = provision(ctx, value, args.access_groups, args.predicate_field)
        expect_filtered = True
        print(
            "provisioned role "
            f"{journal['roleId']} rule {journal['ruleId']} "
            f"(was {journal['previousRoleLabel']}), flushed {flushed} cache keys"
        )
        print("start a fresh nudge-server (NUDGE_NODE_PORT=3002) before comparing")

    return compare(ctx, token, args.nudge_base, expect_filtered, args.limit)


if __name__ == "__main__":
    sys.exit(main())
