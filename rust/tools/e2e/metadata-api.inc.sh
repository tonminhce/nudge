md_post() {
  local response
  response="$(curl -s -w $'\n%{http_code}' -X POST "$BASE/metadata" \
    -H 'content-type: application/json' "$@" -d "$MD_BODY")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

MD_BODY='{"query":"{ __typename }"}'
md_post
check "metadata: unauthenticated query" 200 "$STATUS"
check_value "  UNAUTHENTICATED code" "$(printf '%s' "$BODY" | grep -c '"code":"UNAUTHENTICATED"')"

MD_BODY='{"query":"query { objects(paging: { first: 100 }) { edges { node { id nameSingular } } totalCount } }"}'
md_post -H "authorization: Bearer $ACCESS_TOKEN"
check "metadata: objects query" 200 "$STATUS"
check_value "  returns person" "$(printf '%s' "$BODY" | grep -c '"person"')"

MD_BODY='{"query":"mutation { createOneObject(input: { object: { nameSingular: \"rocket\", namePlural: \"rockets\", labelSingular: \"Rocket\", labelPlural: \"Rockets\" } }) { id nameSingular } }"}'
md_post -H "authorization: Bearer $ACCESS_TOKEN"
check "metadata: createOneObject" 200 "$STATUS"
check_value "  no error payload" "$(at_absent '"errors"')"
MD_ROCKET_ID="$(printf '%s' "$BODY" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)"
check_value "  rocket id captured" "$([ -n "$MD_ROCKET_ID" ] && echo 1 || echo 0)"

MD_BODY='{"query":"query { objects(paging: { first: 100 }) { edges { node { nameSingular } } } }"}'
md_post -H "authorization: Bearer $ACCESS_TOKEN"
check_value "metadata: rocket listed" "$(printf '%s' "$BODY" | grep -c '"rocket"')"

MD_BODY='{"query":"mutation { createOneObject(input: { object: { nameSingular: \"1bad name\", namePlural: \"1bad names\", labelSingular: \"x\", labelPlural: \"x\" } }) { id } }"}'
md_post -H "authorization: Bearer $ACCESS_TOKEN"
check "metadata: invalid name rejected" 200 "$STATUS"
check_value "  validation error" "$(printf '%s' "$BODY" | grep -c '"errors"')"

MD_BODY='{"query":"mutation { deleteOneObject(input: { id: \"00000000-0000-0000-0000-000000000000\" }) { id } }"}'
md_post -H "authorization: Bearer $ACCESS_TOKEN"
check "metadata: delete unknown id" 200 "$STATUS"
check_value "  not-found error" "$(printf '%s' "$BODY" | grep -c '"errors"')"

MD_BODY="{\"query\":\"mutation { deleteOneObject(input: { id: \\\"$MD_ROCKET_ID\\\" }) { id nameSingular } }\"}"
md_post -H "authorization: Bearer $ACCESS_TOKEN"
check "metadata: deleteOneObject rocket" 200 "$STATUS"
check_value "  no error" "$(at_absent '"errors"')"

MD_BODY='{"query":"query { objects(paging: { first: 100 }) { edges { node { nameSingular } } } }"}'
md_post -H "authorization: Bearer $ACCESS_TOKEN"
check_value "metadata: rocket removed" "$(at_absent '"rocket"')"
