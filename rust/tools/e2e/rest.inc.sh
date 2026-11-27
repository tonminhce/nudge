rest_get() {
  local response
  response="$(curl --globoff -s -w $'\n%{http_code}' "$BASE$REST_PATH" \
    -H "authorization: Bearer $ACCESS_TOKEN")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

rest_post() {
  local response
  response="$(curl -s -w $'\n%{http_code}' -X POST "$BASE$REST_PATH" \
    -H 'content-type: application/json' -H "authorization: Bearer $ACCESS_TOKEN" \
    -d "$REST_BODY")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

rest_patch() {
  local response
  response="$(curl -s -w $'\n%{http_code}' -X PATCH "$BASE$REST_PATH" \
    -H 'content-type: application/json' -H "authorization: Bearer $ACCESS_TOKEN" \
    -d "$REST_BODY")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

rest_delete() {
  local response
  response="$(curl -s -w $'\n%{http_code}' -X DELETE "$BASE$REST_PATH" \
    -H 'content-type: application/json' -H "authorization: Bearer $ACCESS_TOKEN" \
    -d "${REST_BODY:-{}}")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

REST_PATH='/rest/people?limit=2'
UNAUTH_RESPONSE="$(curl -s -w $'\n%{http_code}' "$BASE$REST_PATH")"
STATUS="${UNAUTH_RESPONSE##*$'\n'}"
BODY="${UNAUTH_RESPONSE%$'\n'*}"
check "rest: unauthenticated list" 401 "$STATUS"

REST_PATH='/rest/people?limit=2&filter=name.firstName[ilike]:%a%&order_by=name.firstName[AscNullsLast]&depth=1'
rest_get
check "rest: filtered ordered list" 200 "$STATUS"
check_value "  data.people envelope" "$(printf '%s' "$BODY" | grep -c '"people":\[')"
check_value "  totalCount present" "$(printf '%s' "$BODY" | grep -c '"totalCount"')"
check_value "  pageInfo present" "$(printf '%s' "$BODY" | grep -c '"pageInfo"')"

REST_BODY='{"data":{"name":{"firstName":"Rest","lastName":"E2E"},"jobTitle":"Rest Engineer"}}'
REST_PATH='/rest/people'
rest_post
check "rest: createOne person" 201 "$STATUS"
check_value "  createPerson envelope" "$(printf '%s' "$BODY" | grep -c '"createPerson"')"
REST_PERSON_ID="$(printf '%s' "$BODY" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)"
check_value "  person id captured" "$([ -n "$REST_PERSON_ID" ] && echo 1 || echo 0)"

REST_PATH="/rest/people/$REST_PERSON_ID"
rest_get
check "rest: findOne person" 200 "$STATUS"
check_value "  data.person envelope" "$(printf '%s' "$BODY" | grep -c '"person"')"

REST_BODY='{"data":{"jobTitle":"Rest Lead"}}'
rest_patch
check "rest: updateOne person" 200 "$STATUS"
check_value "  updatePerson envelope" "$(printf '%s' "$BODY" | grep -c '"updatePerson"')"
check_value "  updated job title" "$(printf '%s' "$BODY" | grep -c '"Rest Lead"')"

REST_PATH="/rest/people/$REST_PERSON_ID"
REST_BODY=''
rest_delete
check "rest: deleteOne person" 200 "$STATUS"
check_value "  deletePerson envelope" "$(printf '%s' "$BODY" | grep -c '"deletePerson"')"
rest_get
check "rest: soft-deleted person hidden" 404 "$STATUS"

REST_BODY='{"data":{"name":{"firstName":"Hard","lastName":"Delete"},"jobTitle":"Destroy Me"}}'
REST_PATH='/rest/people'
rest_post
check "rest: create destroy target" 201 "$STATUS"
REST_DESTROY_ID="$(printf '%s' "$BODY" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)"
REST_PATH="/rest/people/$REST_DESTROY_ID?soft_delete=false"
REST_BODY=''
rest_delete
check "rest: destroyOne person" 200 "$STATUS"
check_value "  destroyPerson envelope" "$(printf '%s' "$BODY" | grep -c '"destroyPerson"')"

REST_BODY='{"data":{"name":{"firstName":"Restore","lastName":"Me"},"jobTitle":"Restore Me"}}'
REST_PATH='/rest/people'
rest_post
check "rest: create restore target" 201 "$STATUS"
REST_RESTORE_ID="$(printf '%s' "$BODY" | grep -o '"id":"[^"]*"' | head -1 | cut -d'"' -f4)"
REST_BODY="{\"id\":\"$REST_RESTORE_ID\"}"
REST_PATH='/rest/people/restore'
rest_patch
check "rest: restore person" 200 "$STATUS"
check_value "  restorePerson envelope" "$(printf '%s' "$BODY" | grep -c '"restorePerson"')"

REST_BODY='{"data":{"name":{"firstName":"Group","lastName":"By"},"jobTitle":"Group Runner"}}'
REST_PATH='/rest/people'
rest_post
check "rest: create groupBy target" 201 "$STATUS"

REST_PATH='/rest/people/groupBy?group_by=%5B%7B%22jobTitle%22%3Atrue%7D%5D&aggregate=%5B%22countNotEmptyId%22%5D&limit=10'
rest_get
check "rest: people groupBy" 200 "$STATUS"
check_value "  dimension envelope" "$(printf '%s' "$BODY" | grep -c '"groupByDimensionValues"')"
check_value "  totalCount per group" "$(printf '%s' "$BODY" | grep -c '"totalCount"')"

REST_PATH='/rest/unknownObjects?limit=1'
rest_get
check "rest: unknown object rejected" 400 "$STATUS"
check_value "  unknown object message" "$(printf '%s' "$BODY" | grep -c "not found")"

REST_PATH='/rest/people?limit=bad'
rest_get
check "rest: invalid limit rejected" 400 "$STATUS"
check_value "  limit error code" "$(printf '%s' "$BODY" | grep -c '"INVALID_LIMIT_QUERY_PARAM"')"

REST_PATH='/rest/people?filter=broken'
rest_get
check "rest: invalid filter rejected" 400 "$STATUS"
check_value "  filter error code" "$(printf '%s' "$BODY" | grep -c '"INVALID_FILTER_QUERY_PARAM"')"

REST_BODY='{}'
REST_PATH='/rest/people'
rest_delete
check "rest: bulk delete empty filter rejected" 400 "$STATUS"
check_value "  mandatory filter code" "$(printf '%s' "$BODY" | grep -c '"BAD_REQUEST"')"
