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
  response="$(curl -s -w $'\n%{http_code}' -X POST "$BASE$1" \
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
check "signed out token rejected" 401 "$STATUS"

printf '\n%d checks, %d failures\n' "$results" "$failures"

[ "$failures" = "0" ]
