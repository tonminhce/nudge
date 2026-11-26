sso_hdrs="$(mktemp)"
SSO_SUFFIX="SSO-$SUFFIX"
SSO_SERVER_BASE="${SERVER_URL:-http://localhost:3000}"

sso_get_redirect() {
  local response
  response="$(curl -s -D "$sso_hdrs" -o /dev/null -w '%{http_code}' "$BASE$1")"
  STATUS="${response##*$'\n'}"
  SSO_LOCATION="$(grep -i '^location:' "$sso_hdrs" | head -1 | sed 's/^[Ll]ocation:[ ]*//' | tr -d '\r')"
}

sso_post_form() {
  local response
  response="$(curl -s -w $'\n%{http_code}' -X POST "$BASE$1" \
    -H 'content-type: application/x-www-form-urlencoded' -d "$2")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

printf '\n[sso]\n'

if [ -n "${AUTH_GOOGLE_CLIENT_ID:-}" ]; then
  sso_get_redirect /auth/google
  check "google start redirects" 302 "$STATUS"
  check_value "  google authorize endpoint" "$(printf '%s' "$SSO_LOCATION" | grep -c 'https://accounts.google.com/o/oauth2/v2/auth?')"
  check_value "  google client id" "$(printf '%s' "$SSO_LOCATION" | grep -c 'client_id=')"
  check_value "  google callback redirect uri" "$(printf '%s' "$SSO_LOCATION" | grep -c 'redirect_uri=')"
  check_value "  google email profile scope" "$(printf '%s' "$SSO_LOCATION" | grep -c 'scope=email%20profile')"
  check_value "  json state param" "$(printf '%s' "$SSO_LOCATION" | grep -c 'state=%7B')"
  check_value "  select account prompt" "$(printf '%s' "$SSO_LOCATION" | grep -c 'prompt=select_account')"

  sso_get_redirect "/auth/google/redirect?error=access_denied&state=%7B%7D"
  check "google access denied redirect" 302 "$STATUS"
  check_value "  access denied message" "$(printf '%s' "$SSO_LOCATION" | grep -c 'Google%20OAuth%20access%20denied')"
else
  sso_get_redirect /auth/google
  check "google start redirects when disabled" 302 "$STATUS"
  check_value "  google disabled message" "$(printf '%s' "$SSO_LOCATION" | grep -c 'Google%20auth%20is%20not%20enabled')"

  sso_get_redirect /auth/google/redirect
  check "google redirect redirects when disabled" 302 "$STATUS"
  check_value "  google disabled message on redirect" "$(printf '%s' "$SSO_LOCATION" | grep -c 'Google%20auth%20is%20not%20enabled')"
fi

if [ -n "${AUTH_MICROSOFT_CLIENT_ID:-}" ]; then
  sso_get_redirect /auth/microsoft
  check "microsoft start redirects" 302 "$STATUS"
  check_value "  microsoft common tenant" "$(printf '%s' "$SSO_LOCATION" | grep -c 'https://login.microsoftonline.com/common/oauth2/v2.0/authorize?')"
  check_value "  microsoft user.read scope" "$(printf '%s' "$SSO_LOCATION" | grep -c 'scope=User.Read')"
else
  sso_get_redirect /auth/microsoft
  check "microsoft start redirects when disabled" 302 "$STATUS"
  check_value "  microsoft disabled message" "$(printf '%s' "$SSO_LOCATION" | grep -c 'Microsoft%20auth%20is%20not%20enabled')"
fi

SSO_UNKNOWN_IDP="$(uuidgen)"

sso_get_redirect "/auth/oidc/login/$SSO_UNKNOWN_IDP"
check "oidc login unknown idp redirects" 302 "$STATUS"
check_value "  oidc unknown idp message" "$(printf '%s' "$SSO_LOCATION" | grep -c 'errorMessage=Identity%20provider%20not%20found')"

sso_get_redirect "/auth/oidc/login/not-a-uuid"
check "oidc login malformed id redirects" 302 "$STATUS"
check_value "  oidc malformed id message" "$(printf '%s' "$SSO_LOCATION" | grep -c 'Identity%20provider%20not%20found')"

sso_get_redirect "/auth/oidc/callback"
check "oidc callback without state redirects" 302 "$STATUS"
check_value "  invalid state message" "$(printf '%s' "$SSO_LOCATION" | grep -c 'Invalid%20state')"

sso_get_redirect "/auth/oidc/callback?state=neverissued"
check "oidc callback unknown state redirects" 302 "$STATUS"
check_value "  unknown state message" "$(printf '%s' "$SSO_LOCATION" | grep -c 'Invalid%20state')"

get "/auth/saml/metadata/11111111-2222-3333-4444-555555555555"
check "saml metadata renders" 200 "$STATUS"
check_value "  entity id is the login url" "$(printf '%s' "$BODY" | grep -c "entityID=\"$SSO_SERVER_BASE/auth/saml/login/11111111-2222-3333-4444-555555555555\"")"
check_value "  wants assertions signed" "$(printf '%s' "$BODY" | grep -c 'WantAssertionsSigned="true"')"
check_value "  acs is the callback url" "$(printf '%s' "$BODY" | grep -c "Location=\"$SSO_SERVER_BASE/auth/saml/callback/11111111-2222-3333-4444-555555555555\"")"

sso_get_redirect "/auth/saml/login/$SSO_UNKNOWN_IDP"
check "saml login unknown idp redirects" 302 "$STATUS"
check_value "  saml unknown idp message" "$(printf '%s' "$SSO_LOCATION" | grep -c 'Identity%20provider%20not%20found')"

sso_post_form "/auth/saml/callback/$SSO_UNKNOWN_IDP" "SAMLResponse=garbage"
if [ "${AUTH_SAML_ACCEPT_UNVERIFIED_ASSERTIONS:-false}" = "true" ]; then
  check "saml callback without signature gate processes" 302 "$STATUS"
else
  check "saml callback gated by default" 403 "$STATUS"
  check_value "  saml gate error code" "$([ "$(error_code)" = "SSO_AUTH_FAILED" ] && echo 1 || echo 0)"
fi

SSO_EXCHANGE_TOKEN="$(openssl rand -hex 32)"
SSO_EXCHANGE_HASH="$(printf '%s' "$SSO_EXCHANGE_TOKEN" | shasum -a 256 | cut -d' ' -f1)"
SSO_USER_ID="$(uuidgen)"
SSO_EXCHANGE_EMAIL="$SSO_SUFFIX-exchange@nudge.test"

docker exec infra-db-1 psql -U postgres -d "${E2E_DATABASE_NAME:-default}" -q \
  -c "INSERT INTO core.\"user\" (id, email, \"isEmailVerified\") VALUES ('$SSO_USER_ID', '$SSO_EXCHANGE_EMAIL', true)" \
  -c "INSERT INTO core.\"appToken\" (\"userId\", type, value, \"expiresAt\", context) VALUES ('$SSO_USER_ID', 'SSO_EXCHANGE_TOKEN', '$SSO_EXCHANGE_HASH', now() + interval '5 minutes', '{\"authProvider\":\"google\"}')"

post /auth/sso/exchange-token "{\"ssoExchangeToken\":\"$SSO_EXCHANGE_TOKEN\"}"
check "sso exchange token redeems" 200 "$STATUS"
SSO_AGNOSTIC_TOKEN="$(extract accessToken)"
check_value "  workspace agnostic token present" "$([ -n "$SSO_AGNOSTIC_TOKEN" ] && echo 1 || echo 0)"

post /auth/sso/exchange-token "{\"ssoExchangeToken\":\"$SSO_EXCHANGE_TOKEN\"}"
check "sso exchange token is single use" 400 "$STATUS"
check_value "  opaque invalid token error" "$([ "$(error_code)" = "INVALID_INPUT" ] && echo 1 || echo 0)"

post /auth/sso/exchange-token "{\"ssoExchangeToken\":\"$(printf 'a%.0s' $(seq 1 64))\"}"
check "unknown sso exchange token rejected" 400 "$STATUS"

get /auth/me -H "authorization: Bearer $SSO_AGNOSTIC_TOKEN"
check "sso session token resolves the user" 200 "$STATUS"
check_value "  exchange user email" "$(printf '%s' "$BODY" | grep -c "$SSO_EXCHANGE_EMAIL")"

rm -f "$sso_hdrs"
