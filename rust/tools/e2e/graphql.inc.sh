gql_post() {
  local response
  response="$(curl -s -w $'\n%{http_code}' -X POST "$BASE/graphql" \
    -H 'content-type: application/json' -H "authorization: Bearer $ACCESS_TOKEN" \
    -d "$GQL_BODY")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

GQL_BODY='{"query":"{ __typename }"}'
UNAUTH_RESPONSE="$(curl -s -w $'\n%{http_code}' -X POST "$BASE/graphql" \
  -H 'content-type: application/json' -d "$GQL_BODY")"
STATUS="${UNAUTH_RESPONSE##*$'\n'}"
BODY="${UNAUTH_RESPONSE%$'\n'*}"
check "graphql: unauthenticated query" 200 "$STATUS"
check_value "  UNAUTHENTICATED code" "$(printf '%s' "$BODY" | grep -c '"code":"UNAUTHENTICATED"')"

GQL_BODY='{"query":"{ __schema { queryType { name } mutationType { name } } }"}'
gql_post
check "graphql: introspection roots" 200 "$STATUS"
check_value "  query root" "$(printf '%s' "$BODY" | grep -c '"queryType":{"name":"Query"}')"
check_value "  mutation root" "$(printf '%s' "$BODY" | grep -c '"mutationType":{"name":"Mutation"}')"

GQL_BODY='{"query":"{ people(first: 2) { edges { node { id name { firstName lastName } } cursor } pageInfo { hasNextPage hasPreviousPage } totalCount } }"}'
gql_post
check "graphql: people connection" 200 "$STATUS"
check_value "  edges shape" "$(printf '%s' "$BODY" | grep -c '"edges"')"
check_value "  total count" "$(printf '%s' "$BODY" | grep -c '"totalCount"')"

GQL_BODY='{"query":"mutation { createCompany(data: {name: \"GraphQL E2E\", domainName: {primaryLinkUrl: \"https://graphql-e2e.test\"}}) { id name domainName { primaryLinkUrl } createdBy { source } } }"}'
gql_post
check "graphql: createCompany" 200 "$STATUS"
check_value "  no errors" "$(at_absent '"errors"')"
check_value "  manual actor" "$(printf '%s' "$BODY" | grep -c '"source":"MANUAL"')"
GQL_COMPANY_ID="$(printf '%s' "$BODY" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)"
check_value "  company id captured" "$([ -n "$GQL_COMPANY_ID" ] && echo 1 || echo 0)"

GQL_BODY="{\"query\":\"mutation { updateCompany(id: \\\"$GQL_COMPANY_ID\\\", data: {name: \\\"GraphQL E2E Updated\\\"}) { id name } }\"}"
gql_post
check "graphql: updateCompany" 200 "$STATUS"
check_value "  updated name" "$(printf '%s' "$BODY" | grep -c 'GraphQL E2E Updated')"

GQL_BODY="{\"query\":\"mutation { deleteCompany(id: \\\"$GQL_COMPANY_ID\\\") { id name } }\"}"
gql_post
check "graphql: deleteCompany" 200 "$STATUS"
check_value "  returns trashed record" "$(printf '%s' "$BODY" | grep -c 'GraphQL E2E Updated')"

GQL_BODY="{\"query\":\"query { company(filter: {id: {eq: \\\"$GQL_COMPANY_ID\\\"}}) { id name } }\"}"
gql_post
check "graphql: deleted company hidden" 200 "$STATUS"
check_value "  record not found" "$(printf '%s' "$BODY" | grep -c '"code":"NOT_FOUND"')"

GQL_BODY='{"query":"{ people { totalCount } currentUser { id } }"}'
gql_post
check "graphql: core workspace mix rejected" 200 "$STATUS"
check_value "  BAD_USER_INPUT code" "$(printf '%s' "$BODY" | grep -c '"code":"BAD_USER_INPUT"')"
check_value "  mix subCode" "$(printf '%s' "$BODY" | grep -c '"subCode":"CORE_WORKSPACE_MIX"')"
