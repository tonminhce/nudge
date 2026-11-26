#!/usr/bin/env bash
set -uo pipefail

BASE="${BASE:-http://127.0.0.1:3000}"
SUFFIX="${SUFFIX:-$(date +%s)}"
EMAIL="${EMAIL:-founder-$SUFFIX@nudge.test}"
PASSWORD="${PASSWORD:-SuperSecret123}"
SUBDOMAIN="${SUBDOMAIN:-nudge-hq-$SUFFIX}"
EXPECT_ROTATED_REUSE="${EXPECT_ROTATED_REUSE:-200}"

results=0
failures=0
STATUS=""
BODY=""

post() {
  local response
  response="$(curl -s -w $'\n%{http_code}' -X POST "$BASE$1" "${@:3}" \
    -H 'content-type: application/json' -d "$2")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

get() {
  local response
  response="$(curl -s -w $'\n%{http_code}' "$BASE$1" "${@:2}")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

extract() {
  printf '%s' "$BODY" | grep -o "\"$1\":\"[^\"]*\"" | head -1 | cut -d'"' -f4
}

error_code() {
  printf '%s' "$BODY" | grep -o '"error":"[^"]*"' | head -1 | cut -d'"' -f4
}

check() {
  local label="$1" expected="$2" actual="$3"

  if [ "$actual" = "$expected" ]; then
    printf 'PASS  %-34s HTTP %s\n' "$label" "$actual"
    results=$((results + 1))
  else
    printf 'FAIL  %-34s HTTP %s (expected %s)\n' "$label" "$actual" "$expected"
    failures=$((failures + 1))
  fi
}

check_value() {
  local label="$1" condition="$2"

  if [ "$condition" = "1" ]; then
    printf 'PASS  %-34s\n' "$label"
    results=$((results + 1))
  else
    printf 'FAIL  %-34s\n' "$label"
    failures=$((failures + 1))
  fi
}

printf 'health: %s\n\n' "$(curl -s "$BASE/healthz")"

post /auth/sign-up "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\",\"workspaceDisplayName\":\"Nudge HQ\",\"subdomain\":\"$SUBDOMAIN\"}"
check "sign up" 201 "$STATUS"
WORKSPACE_ID="$(extract workspaceId)"
ACCESS_TOKEN="$(extract accessToken)"
check_value "  workspace id present" "$([ -n "$WORKSPACE_ID" ] && echo 1 || echo 0)"
check_value "  access token present" "$([ -n "$ACCESS_TOKEN" ] && echo 1 || echo 0)"

post /auth/check-user "{\"email\":\"$EMAIL\"}"
check "check user exists" 200 "$STATUS"
check_value "  exists is true" "$(printf '%s' "$BODY" | grep -c '"exists":true')"

post /auth/sign-up "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\",\"subdomain\":\"$SUBDOMAIN-again\"}"
check "duplicate sign up rejected" 403 "$STATUS"
check_value "  error code" "$([ "$(error_code)" = "USER_ALREADY_EXISTS" ] && echo 1 || echo 0)"

get /auth/me -H "authorization: Bearer $ACCESS_TOKEN"
check "me with access token" 200 "$STATUS"
check_value "  returns the user" "$(printf '%s' "$BODY" | grep -c "\"email\":\"$EMAIL\"")"

get /auth/me
check "me without token" 401 "$STATUS"

get /auth/me -H "authorization: Bearer ${ACCESS_TOKEN}x"
check "me with tampered token" 401 "$STATUS"

post /auth/login-token "{\"email\":\"$EMAIL\",\"password\":\"WrongPass123\",\"workspaceId\":\"$WORKSPACE_ID\"}"
check "login token wrong password" 401 "$STATUS"

post /auth/login-token "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\",\"workspaceId\":\"$WORKSPACE_ID\"}"
check "login token with right password" 200 "$STATUS"
LOGIN_TOKEN="$(extract loginToken)"
check_value "  login token present" "$([ -n "$LOGIN_TOKEN" ] && echo 1 || echo 0)"

post /auth/tokens "{\"loginToken\":\"$LOGIN_TOKEN\",\"workspaceId\":\"$WORKSPACE_ID\"}"
check "exchange login token" 200 "$STATUS"
REFRESH_TOKEN="$(extract refreshToken)"
check_value "  refresh token present" "$([ -n "$REFRESH_TOKEN" ] && echo 1 || echo 0)"

post /auth/tokens "{\"loginToken\":\"$LOGIN_TOKEN\",\"workspaceId\":\"$WORKSPACE_ID\"}"
check "login token is single use" 401 "$STATUS"

post /auth/token "{\"refreshToken\":\"$REFRESH_TOKEN\"}"
check "renew refresh token" 200 "$STATUS"
NEW_REFRESH_TOKEN="$(extract refreshToken)"
check_value "  new refresh token present" "$([ -n "$NEW_REFRESH_TOKEN" ] && echo 1 || echo 0)"

post /auth/token "{\"refreshToken\":\"$REFRESH_TOKEN\"}"
check "rotated refresh token" "$EXPECT_ROTATED_REUSE" "$STATUS"

post /auth/token "{\"refreshToken\":\"$NEW_REFRESH_TOKEN\"}"
check "new refresh token works" 200 "$STATUS"

post /auth/sign-out "{\"refreshToken\":\"$NEW_REFRESH_TOKEN\"}"
check "sign out" 200 "$STATUS"

post /auth/token "{\"refreshToken\":\"$NEW_REFRESH_TOKEN\"}"
check "signed out token beyond grace" "$EXPECT_ROTATED_REUSE" "$STATUS"

HDRS="$(mktemp)"
COOKIE_EMAIL="cookies-$SUFFIX@nudge.test"
COOKIE_SUBDOMAIN="nudge-cookies-$SUFFIX"

post_cookie_flow() {
  local path="$1" body="$2"
  shift 2
  local response
  response="$(curl -s -D "$HDRS" -w $'\n%{http_code}' -X POST "$BASE$path" \
    -H 'content-type: application/json' -H "$SESSION_OPT_IN" "$@" -d "$body")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

get_cookie_flow() {
  local path="$1"
  shift
  local response
  response="$(curl -s -w $'\n%{http_code}' "$BASE$path" "$@" )"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

SESSION_OPT_IN="x-twenty-session-cookie: 1"

post_cookie_flow /auth/sign-up "{\"email\":\"$COOKIE_EMAIL\",\"password\":\"$PASSWORD\",\"workspaceDisplayName\":\"Cookie HQ\",\"subdomain\":\"$COOKIE_SUBDOMAIN\"}" \
  -H "$SESSION_OPT_IN"
check "cookie sign up" 201 "$STATUS"
SESSION_COOKIE="$(grep -o 'twenty-session=sess_[A-Za-z0-9_-]*' "$HDRS" | head -1 | cut -d= -f2)"
check_value "  session cookie issued" "$([ -n "$SESSION_COOKIE" ] && echo 1 || echo 0)"

get_cookie_flow /auth/me -H "cookie: twenty-session=$SESSION_COOKIE" -H "$SESSION_OPT_IN"
check "me with session cookie" 200 "$STATUS"
check_value "  returns the cookie user" "$(printf '%s' "$BODY" | grep -c "\"email\":\"$COOKIE_EMAIL\"")"

get_cookie_flow /auth/me -H "cookie: twenty-session=$SESSION_COOKIE"
check "me with cookie without opt-in" 401 "$STATUS"

get_cookie_flow /auth/me -H "authorization: Bearer $SESSION_COOKIE" -H "$SESSION_OPT_IN"
check "session token as bearer rejected" 401 "$STATUS"
check_value "  cookie-only message" "$(printf '%s' "$BODY" | grep -c 'only accepted from the session cookie')"

post_cookie_flow /auth/token "{\"refreshToken\":\"garbage\"}" -H "cookie: twenty-session=$SESSION_COOKIE" \
  -H "$SESSION_OPT_IN" -H "origin: http://evil.example"
check "csrf foreign origin blocked" 403 "$STATUS"
check_value "  csrf error code" "$([ "$(error_code)" = "CSRF_ORIGIN_MISMATCH" ] && echo 1 || echo 0)"

post_cookie_flow /auth/token "{\"refreshToken\":\"garbage\"}" -H "cookie: twenty-session=$SESSION_COOKIE" \
  -H "$SESSION_OPT_IN"
check "csrf missing origin fails closed" 403 "$STATUS"

get_cookie_flow /auth/me -H "cookie: twenty-session=$SESSION_COOKIE" -H "$SESSION_OPT_IN"
COOKIE_WORKSPACE_ID="$(printf '%s' "$BODY" | grep -o '"workspace":{"id":"[^"]*"' | cut -d'"' -f6)"

post_cookie_flow /auth/login-token "{\"email\":\"$COOKIE_EMAIL\",\"password\":\"$PASSWORD\",\"workspaceId\":\"$COOKIE_WORKSPACE_ID\"}" \
  -H "cookie: twenty-session=$SESSION_COOKIE" -H "$SESSION_OPT_IN" -H "origin: http://127.0.0.1:3000"
check "login token with allowed origin" 200 "$STATUS"
COOKIE_LOGIN_TOKEN="$(extract loginToken)"

post_cookie_flow /auth/tokens "{\"loginToken\":\"$COOKIE_LOGIN_TOKEN\",\"workspaceId\":\"$COOKIE_WORKSPACE_ID\"}" \
  -H "cookie: twenty-session=$SESSION_COOKIE" -H "$SESSION_OPT_IN" -H "origin: http://127.0.0.1:3000"
check "exchange with allowed origin" 200 "$STATUS"
NEW_SESSION_COOKIE="$(grep -o 'twenty-session=sess_[A-Za-z0-9_-]*' "$HDRS" | head -1 | cut -d= -f2)"
check_value "  replacement cookie issued" "$([ -n "$NEW_SESSION_COOKIE" ] && echo 1 || echo 0)"
check_value "  replacement differs" "$([ "$NEW_SESSION_COOKIE" != "$SESSION_COOKIE" ] && echo 1 || echo 0)"

get_cookie_flow /auth/me -H "cookie: twenty-session=$SESSION_COOKIE" -H "$SESSION_OPT_IN"
check "old session superseded" 401 "$STATUS"

get_cookie_flow /auth/me -H "cookie: twenty-session=$NEW_SESSION_COOKIE" -H "$SESSION_OPT_IN"
check "replacement session works" 200 "$STATUS"

post_cookie_flow /auth/login-token "{\"email\":\"$COOKIE_EMAIL\",\"password\":\"$PASSWORD\",\"workspaceId\":\"$COOKIE_WORKSPACE_ID\"}" \
  -H "cookie: twenty-session=$NEW_SESSION_COOKIE" -H "$SESSION_OPT_IN" -H "origin: http://127.0.0.1:3000"
check "login token for bridge flow" 200 "$STATUS"
COOKIE_LOGIN_TOKEN="$(extract loginToken)"

post_cookie_flow /auth/tokens "{\"loginToken\":\"$COOKIE_LOGIN_TOKEN\",\"workspaceId\":\"$COOKIE_WORKSPACE_ID\"}" \
  -H "cookie: twenty-session=$NEW_SESSION_COOKIE" -H "$SESSION_OPT_IN" \
  -H "origin: http://127.0.0.1:3000"
check "bridge exchange" 200 "$STATUS"
COOKIE_REFRESH_TOKEN="$(extract refreshToken)"
NEW_SESSION_COOKIE="$(grep -o 'twenty-session=sess_[A-Za-z0-9_-]*' "$HDRS" | head -1 | cut -d= -f2)"
check_value "  bridge cookie captured" "$([ -n "$NEW_SESSION_COOKIE" ] && echo 1 || echo 0)"

post_cookie_flow /auth/token "{\"refreshToken\":\"$COOKIE_REFRESH_TOKEN\"}" \
  -H "cookie: twenty-session=$NEW_SESSION_COOKIE" -H "$SESSION_OPT_IN" \
  -H "origin: http://127.0.0.1:3000"
check "renewal with presented session" 200 "$STATUS"
check_value "  bridge reuses the session" "$([ -z "$(grep -o 'twenty-session=sess_' "$HDRS" | head -1)" ] && echo 1 || echo 0)"

get_cookie_flow /auth/me -H "cookie: twenty-session=$NEW_SESSION_COOKIE" -H "$SESSION_OPT_IN"
check "reused session still active" 200 "$STATUS"

post_cookie_flow /auth/sign-out "{}" -H "cookie: twenty-session=$NEW_SESSION_COOKIE" -H "$SESSION_OPT_IN" \
  -H "origin: http://127.0.0.1:3000"
check "cookie sign out" 200 "$STATUS"
check_value "  cookie cleared" "$([ "$(grep -c 'Expires=Thu, 01 Jan 1970 00:00:00 GMT' "$HDRS")" -ge 1 ] && echo 1 || echo 0)"

get_cookie_flow /auth/me -H "cookie: twenty-session=$NEW_SESSION_COOKIE" -H "$SESSION_OPT_IN"
check "signed out session rejected" 401 "$STATUS"

rm -f "$HDRS"

for section in "$(dirname "$0")"/e2e/*.inc.sh; do
  [ -f "$section" ] && source "$section"
done

plus_one_year="$(date -u -v+1y "+%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u -d "+1 year" "+%Y-%m-%dT%H:%M:%SZ")"

get /auth/roles -H "authorization: Bearer $ACCESS_TOKEN"
check "roles listed" 200 "$STATUS"
ADMIN_ROLE_ID="$(printf '%s' "$BODY" | grep -o '{"id":"[^"]*","label":"Admin"' | cut -d'"' -f4)"
check_value "  admin role found" "$([ -n "$ADMIN_ROLE_ID" ] && echo 1 || echo 0)"

post /auth/api-keys "{\"name\":\"e2e key\",\"expiresAt\":\"$plus_one_year\",\"roleId\":\"$ADMIN_ROLE_ID\"}" \
  -H "authorization: Bearer $ACCESS_TOKEN"
check "api key created" 201 "$STATUS"
API_KEY_ID="$(printf '%s' "$BODY" | grep -o '"apiKey":{"id":"[^"]*"' | cut -d'"' -f6)"
check_value "  key id present" "$([ -n "$API_KEY_ID" ] && echo 1 || echo 0)"

post /auth/api-keys/token "{\"apiKeyId\":\"$API_KEY_ID\"}" -H "authorization: Bearer $ACCESS_TOKEN"
check "api key token minted" 200 "$STATUS"
API_KEY_TOKEN="$(extract token)"
check_value "  token present" "$([ -n "$API_KEY_TOKEN" ] && echo 1 || echo 0)"

get /auth/me -H "authorization: Bearer $API_KEY_TOKEN"
check "me rejects api key context" 401 "$STATUS"

get /auth/roles -H "authorization: Bearer $API_KEY_TOKEN"
check "roles with api key token" 200 "$STATUS"

post /auth/api-keys/revoke "{\"id\":\"$API_KEY_ID\"}" -H "authorization: Bearer $ACCESS_TOKEN"
check "api key revoked" 200 "$STATUS"
check_value "  revokedAt set" "$(printf '%s' "$BODY" | grep -c '"revokedAt":"[^n]')"

get /auth/roles -H "authorization: Bearer $API_KEY_TOKEN"
check "revoked api key rejected" 403 "$STATUS"
check_value "  revoked message" "$(printf '%s' "$BODY" | grep -c 'This API Key is revoked')"

post /auth/api-keys "{\"name\":\"expired key\",\"expiresAt\":\"2020-01-01T00:00:00Z\",\"roleId\":\"$ADMIN_ROLE_ID\"}" \
  -H "authorization: Bearer $ACCESS_TOKEN"
check "expired key created" 201 "$STATUS"
EXPIRED_KEY_ID="$(printf '%s' "$BODY" | grep -o '"apiKey":{"id":"[^"]*"' | cut -d'"' -f6)"

post /auth/api-keys/token "{\"apiKeyId\":\"$EXPIRED_KEY_ID\"}" -H "authorization: Bearer $ACCESS_TOKEN"
check "minting for expired key rejected" 403 "$STATUS"
check_value "  expired message" "$(printf '%s' "$BODY" | grep -c 'This API Key has expired')"

in_two_seconds="$(date -u -v+2S "+%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u -d "+2 seconds" "+%Y-%m-%dT%H:%M:%SZ")"
post /auth/api-keys "{\"name\":\"short lived key\",\"expiresAt\":\"$in_two_seconds\",\"roleId\":\"$ADMIN_ROLE_ID\"}" \
  -H "authorization: Bearer $ACCESS_TOKEN"
check "short lived key created" 201 "$STATUS"
SHORT_KEY_ID="$(printf '%s' "$BODY" | grep -o '"apiKey":{"id":"[^"]*"' | cut -d'"' -f6)"

post /auth/api-keys/token "{\"apiKeyId\":\"$SHORT_KEY_ID\"}" -H "authorization: Bearer $ACCESS_TOKEN"
check "short lived token minted" 200 "$STATUS"
SHORT_TOKEN="$(extract token)"

get /auth/roles -H "authorization: Bearer $SHORT_TOKEN"
check "short lived token works" 200 "$STATUS"

sleep 3

get /auth/roles -H "authorization: Bearer $SHORT_TOKEN"
check "expired api key rejected" 403 "$STATUS"
check_value "  expired message" "$(printf '%s' "$BODY" | grep -c 'This API Key is expired')"

post /auth/api-keys "{\"name\":\"bad role key\",\"expiresAt\":\"$plus_one_year\",\"roleId\":\"$(uuidgen)\"}" \
  -H "authorization: Bearer $ACCESS_TOKEN"
check "api key with unknown role rejected" 400 "$STATUS"

post /auth/api-keys "{\"name\":\"e2e key\",\"expiresAt\":\"$plus_one_year\",\"roleId\":\"$ADMIN_ROLE_ID\"}"
check "api key creation needs auth" 401 "$STATUS"

printf '\n%d checks, %d failures\n' "$results" "$failures"

[ "$failures" = "0" ]
