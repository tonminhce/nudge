IMP_HDRS="$(mktemp)"
IMP_SUFFIX="imp-$SUFFIX"
IMP_TARGET_EMAIL="target-$IMP_SUFFIX@nudge.test"
IMP_TARGET_SUBDOMAIN="nudge-imp-$IMP_SUFFIX"
IMP_ORIGIN="http://127.0.0.1:3000"

imp_post() {
  local response
  response="$(curl -s -D "$IMP_HDRS" -w $'\n%{http_code}' -X POST "$BASE$1" \
    -H 'content-type: application/json' -H "$SESSION_OPT_IN" "${@:3}" -d "$2")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

imp_get() {
  local response
  response="$(curl -s -w $'\n%{http_code}' "$BASE$1" "${@:2}")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

imp_founder_can_impersonate() {
  docker exec infra-db-1 psql -U postgres -d "${E2E_DATABASE_NAME:-default}" -tAc \
    "SELECT \"canImpersonate\" FROM core.\"user\" WHERE email = '$IMP_ADMIN_EMAIL'"
}

post /auth/sign-up "{\"email\":\"$IMP_TARGET_EMAIL\",\"password\":\"$PASSWORD\",\"subdomain\":\"$IMP_TARGET_SUBDOMAIN\"}"
check "impersonation target signed up" 201 "$STATUS"
IMP_TARGET_ID="$(printf '%s' "$BODY" | grep -o '"user":{"id":"[^"]*"' | cut -d'"' -f6)"
IMP_TARGET_WORKSPACE="$(printf '%s' "$BODY" | grep -o '"workspaceId":"[^"]*"' | cut -d'"' -f4)"
check_value "  target workspace id present" "$([ -n "$IMP_TARGET_WORKSPACE" ] && echo 1 || echo 0)"

IMP_ADMIN_EMAIL="$(docker exec infra-db-1 psql -U postgres -d "${E2E_DATABASE_NAME:-default}" -tAc \
  "SELECT email FROM core.\"user\" WHERE \"canImpersonate\" IS TRUE ORDER BY \"createdAt\" ASC LIMIT 1")"
IMP_ADMIN_WORKSPACE="$(docker exec infra-db-1 psql -U postgres -d "${E2E_DATABASE_NAME:-default}" -tAc \
  "SELECT w.id FROM core.\"userWorkspace\" uw \
   JOIN core.\"workspace\" w ON w.id = uw.\"workspaceId\" \
   JOIN core.\"user\" u ON u.id = uw.\"userId\" \
   WHERE u.email = '$IMP_ADMIN_EMAIL' ORDER BY w.\"createdAt\" ASC LIMIT 1")"
check_value "server admin exists" "$([ -n "$IMP_ADMIN_EMAIL" ] && echo 1 || echo 0)" 1
check_value "  admin holds canImpersonate" \
  "$([ "$(imp_founder_can_impersonate)" = "t" ] && echo 1 || echo 0)" 1
check_value "  admin workspace resolved" "$([ -n "$IMP_ADMIN_WORKSPACE" ] && echo 1 || echo 0)" 1

imp_post /auth/login-token "{\"email\":\"$IMP_ADMIN_EMAIL\",\"password\":\"$PASSWORD\",\"workspaceId\":\"$IMP_ADMIN_WORKSPACE\"}" \
  -H "origin: $IMP_ORIGIN"
check "admin login token for session" 200 "$STATUS"
IMP_LOGIN="$(extract loginToken)"

imp_post /auth/tokens "{\"loginToken\":\"$IMP_LOGIN\",\"workspaceId\":\"$IMP_ADMIN_WORKSPACE\"}"
check "admin session established" 200 "$STATUS"
IMP_FOUNDER_SESSION="$(grep -o 'twenty-session=sess_[A-Za-z0-9_-]*' "$IMP_HDRS" | head -1 | cut -d= -f2)"
IMP_ADMIN_TOKEN="$(extract accessToken)"
check_value "  admin session cookie" "$([ -n "$IMP_FOUNDER_SESSION" ] && echo 1 || echo 0)"

post /auth/impersonate "{\"workspaceId\":\"$IMP_TARGET_WORKSPACE\",\"userId\":\"$IMP_TARGET_ID\"}" \
  -H "authorization: Bearer $IMP_ADMIN_TOKEN"
check "founder impersonates target cross workspace" 200 "$STATUS"
IMP_LOGIN_TOKEN="$(extract loginToken)"

imp_post /auth/impersonation/tokens "{\"loginToken\":\"$IMP_LOGIN_TOKEN\",\"workspaceId\":\"$IMP_TARGET_WORKSPACE\"}" \
  -H "cookie: twenty-session=$IMP_FOUNDER_SESSION" -H "origin: $IMP_ORIGIN"
check "impersonation token exchange" 200 "$STATUS"
IMP_SESSION_COOKIE="$(grep -o 'twenty-session=sess_[A-Za-z0-9_-]*' "$IMP_HDRS" | head -1 | cut -d= -f2)"
IMP_PARKED_COOKIE="$(grep -o 'twenty-impersonator-session=sess_[A-Za-z0-9_-]*' "$IMP_HDRS" | head -1 | cut -d= -f2)"
check_value "  impersonation session issued" "$([ -n "$IMP_SESSION_COOKIE" ] && echo 1 || echo 0)"
check_value "  founder session parked" "$([ -n "$IMP_PARKED_COOKIE" ] && echo 1 || echo 0)"
check_value "  parked carries the admin session" \
  "$([ "$IMP_PARKED_COOKIE" = "$IMP_FOUNDER_SESSION" ] && echo 1 || echo 0)" 1

imp_get /auth/me -H "cookie: twenty-session=$IMP_SESSION_COOKIE" -H "$SESSION_OPT_IN"
check "me as impersonated target" 200 "$STATUS"
check_value "  target email served" "$(printf '%s' "$BODY" | grep -c "\"email\":\"$IMP_TARGET_EMAIL\"")" 1
check_value "  impersonation flagged" "$(printf '%s' "$BODY" | grep -c '"isImpersonating":true')" 1

IMP_STOP_RESPONSE="$(curl -s -D "$IMP_HDRS" -w $'\n%{http_code}' -X POST "$BASE/auth/stop-impersonation" \
  -H 'content-type: application/json' -d '{}' \
  -H "cookie: twenty-session=$IMP_SESSION_COOKIE; twenty-impersonator-session=$IMP_PARKED_COOKIE" \
  -H "$SESSION_OPT_IN" -H "origin: $IMP_ORIGIN")"
STATUS="${IMP_STOP_RESPONSE##*$'\n'}"
BODY="$(printf '%s' "$IMP_STOP_RESPONSE" | sed '$d')"
check "stop impersonation" 200 "$STATUS"
check_value "  restore reported" "$(printf '%s' "$BODY" | grep -c '"canRestoreImpersonatorSession":true')" 1
IMP_RESTORED_COOKIE="$(grep -o 'twenty-session=sess_[A-Za-z0-9_-]*' "$IMP_HDRS" | head -1 | cut -d= -f2)"
check_value "  founder session restored" "$([ "$IMP_RESTORED_COOKIE" = "$IMP_FOUNDER_SESSION" ] && echo 1 || echo 0)"
check_value "  impersonator cookies cleared" \
  "$([ "$(grep -c 'twenty-impersonator-session=;' "$IMP_HDRS")" -eq 2 ] && echo 1 || echo 0)" 1

imp_get /auth/me -H "cookie: twenty-session=$IMP_RESTORED_COOKIE" -H "$SESSION_OPT_IN"
check "me after restore" 200 "$STATUS"
check_value "  admin identity back" "$(printf '%s' "$BODY" | grep -c "\"email\":\"$IMP_ADMIN_EMAIL\"")" 1
check_value "  impersonation cleared" "$(printf '%s' "$BODY" | grep -c '"isImpersonating":false')" 1

imp_get /auth/me -H "cookie: twenty-session=$IMP_SESSION_COOKIE" -H "$SESSION_OPT_IN"
check "impersonation session revoked" 401 "$STATUS"

rm -f "$IMP_HDRS"
