WL_EMAIL="ws-$SUFFIX@nudge.test"
WL_SUBDOMAIN="nudge-ws-$SUFFIX"
WL_ORIGIN="http://localhost:3000"

post /auth/sign-up "{\"email\":\"$WL_EMAIL\",\"password\":\"$PASSWORD\",\"workspaceDisplayName\":\"WL HQ\",\"subdomain\":\"$WL_SUBDOMAIN\"}"
check "wl admin sign up" 201 "$STATUS"
WL_TOKEN="$(extract accessToken)"
WL_USER_ID="$(printf '%s' "$BODY" | grep -o '"user":{"id":"[^"]*"' | cut -d'"' -f6)"
WL_FIRST_WORKSPACE="$(extract workspaceId)"
check_value "  wl admin token" "$([ -n "$WL_TOKEN" ] && echo 1 || echo 0)"
check_value "  wl admin user id" "$([ -n "$WL_USER_ID" ] && echo 1 || echo 0)"

post /auth/workspace "{\"displayName\":\"WL Second\",\"subdomain\":\"$WL_SUBDOMAIN-2\"}" -H "authorization: Bearer $WL_TOKEN"
check "create second workspace" 201 "$STATUS"
WL_LOGIN_TOKEN="$(extract loginToken)"
WL_SECOND_WORKSPACE="$(printf '%s' "$BODY" | grep -o '"workspace":{"id":"[^"]*"' | cut -d'"' -f6)"
check_value "  workspace login token" "$([ -n "$WL_LOGIN_TOKEN" ] && echo 1 || echo 0)"
check_value "  second workspace id" "$([ -n "$WL_SECOND_WORKSPACE" ] && echo 1 || echo 0)"
check_value "  workspace ids differ" "$([ "$WL_SECOND_WORKSPACE" != "$WL_FIRST_WORKSPACE" ] && echo 1 || echo 0)"

post /auth/workspace "{\"displayName\":\"WL Second\",\"subdomain\":\"$WL_SUBDOMAIN-2\"}" -H "authorization: Bearer $WL_TOKEN"
check "second workspace subdomain taken" 400 "$STATUS"
check_value "  taken error code" "$([ "$(error_code)" = "SUBDOMAIN_ALREADY_TAKEN" ] && echo 1 || echo 0)"

post /auth/workspace "{\"displayName\":\"WL Bad\",\"subdomain\":\"api-reserved\"}" -H "authorization: Bearer $WL_TOKEN"
check "reserved subdomain rejected" 400 "$STATUS"
check_value "  invalid error code" "$([ "$(error_code)" = "SUBDOMAIN_NOT_VALID" ] && echo 1 || echo 0)"

post /auth/workspace "{\"subdomain\":\"$WL_SUBDOMAIN-3\"}" -H "authorization: Bearer $WL_TOKEN"
check "workspace name required" 400 "$STATUS"

post /auth/workspace "{\"displayName\":\"WL Second\"}"
check "workspace creation needs auth" 401 "$STATUS"

post /auth/tokens "{\"loginToken\":\"$WL_LOGIN_TOKEN\",\"workspaceId\":\"$WL_SECOND_WORKSPACE\"}"
check "exchange tokens for second workspace" 200 "$STATUS"
WL_SECOND_TOKEN="$(extract accessToken)"
check_value "  second workspace token" "$([ -n "$WL_SECOND_TOKEN" ] && echo 1 || echo 0)"

get /auth/me -H "authorization: Bearer $WL_SECOND_TOKEN"
check "me shows the second workspace" 200 "$STATUS"
check_value "  second workspace in me" "$(printf '%s' "$BODY" | grep -c "\"id\":\"$WL_SECOND_WORKSPACE\"")"

get /auth/roles -H "authorization: Bearer $WL_SECOND_TOKEN"
check "roles empty before activation" 200 "$STATUS"
check_value "  no roles yet" "$(printf '%s' "$BODY" | grep -c '"roles":\[\]')"

post /auth/workspace/activate "{}" -H "authorization: Bearer $WL_SECOND_TOKEN"
check "activate second workspace" 200 "$STATUS"
check_value "  activation status" "$(printf '%s' "$BODY" | grep -c '"activationStatus":"CREATED"')"

post /auth/workspace/activate "{}" -H "authorization: Bearer $WL_SECOND_TOKEN"
check "activation is idempotent" 200 "$STATUS"

get /auth/roles -H "authorization: Bearer $WL_SECOND_TOKEN"
check "roles after activation" 200 "$STATUS"
check_value "  admin role seeded" "$(printf '%s' "$BODY" | grep -c '"label":"Admin"')"
check_value "  member role seeded" "$(printf '%s' "$BODY" | grep -c '"label":"Member"')"
check_value "  custom application role seeded" "$(printf '%s' "$BODY" | grep -c '"label":"Workspace Custom Application"')"

WL_OTHER_EMAIL="ws-other-$SUFFIX@nudge.test"
WL_OTHER_SUBDOMAIN="nudge-ws-other-$SUFFIX"

post /auth/sign-up "{\"email\":\"$WL_OTHER_EMAIL\",\"password\":\"$PASSWORD\",\"workspaceDisplayName\":\"WL Other\",\"subdomain\":\"$WL_OTHER_SUBDOMAIN\"}"
check "wl other user sign up" 201 "$STATUS"
WL_OTHER_TOKEN="$(extract accessToken)"
WL_OTHER_USER="$(printf '%s' "$BODY" | grep -o '"user":{"id":"[^"]*"' | cut -d'"' -f6)"
WL_OTHER_WORKSPACE="$(extract workspaceId)"
check_value "  other user present" "$([ -n "$WL_OTHER_USER" ] && echo 1 || echo 0)"

post /auth/impersonate "{\"workspaceId\":\"$WL_SECOND_WORKSPACE\",\"userId\":\"$WL_OTHER_USER\"}"
check "impersonation start needs auth" 401 "$STATUS"

post /auth/impersonate "{\"workspaceId\":\"$WL_SECOND_WORKSPACE\",\"userId\":\"$WL_OTHER_USER\"}" -H "authorization: Bearer $WL_TOKEN"
check "impersonation without the admin role denied" 403 "$STATUS"
check_value "  permission denied code" "$([ "$(error_code)" = "PERMISSION_DENIED" ] && echo 1 || echo 0)"

post /auth/impersonate "{\"workspaceId\":\"$WL_SECOND_WORKSPACE\",\"userId\":\"$(uuidgen)\"}" -H "authorization: Bearer $WL_SECOND_TOKEN"
check "unknown target user workspace" 500 "$STATUS"
check_value "  target missing code" "$([ "$(error_code)" = "USER_WORKSPACE_NOT_FOUND" ] && echo 1 || echo 0)"

post /auth/impersonate "{\"workspaceId\":\"$WL_SECOND_WORKSPACE\",\"userId\":\"$WL_USER_ID\"}" -H "authorization: Bearer $WL_SECOND_TOKEN"
check "admin context reaches the self check" 403 "$STATUS"
check_value "  self impersonation message" "$(printf '%s' "$BODY" | grep -c 'cannot impersonate themselves')"

post /auth/impersonate "{\"workspaceId\":\"$WL_OTHER_WORKSPACE\",\"userId\":\"$WL_OTHER_USER\"}" -H "authorization: Bearer $WL_SECOND_TOKEN"
check "server level impersonation denied" 403 "$STATUS"
check_value "  server level message" "$(printf '%s' "$BODY" | grep -c 'Server level impersonation not allowed')"

post /auth/impersonate "{\"workspaceId\":\"$WL_OTHER_WORKSPACE\",\"userId\":\"$WL_OTHER_USER\"}" -H "authorization: Bearer $WL_OTHER_TOKEN"
check "non admin cannot impersonate" 403 "$STATUS"
check_value "  non admin code" "$([ "$(error_code)" = "PERMISSION_DENIED" ] && echo 1 || echo 0)"

post /auth/stop-impersonation "{}"
check "stop impersonation needs auth" 401 "$STATUS"

post /auth/stop-impersonation "{}" -H "authorization: Bearer $WL_SECOND_TOKEN"
check "stop without impersonation denied" 403 "$STATUS"
check_value "  not impersonating message" "$(printf '%s' "$BODY" | grep -c 'Not currently impersonating')"

post /auth/impersonation/tokens "{\"loginToken\":\"garbage\",\"workspaceId\":\"$WL_SECOND_WORKSPACE\"}"
check "impersonation exchange rejects garbage" 401 "$STATUS"

get /auth/roles -H "authorization: Bearer $WL_SECOND_TOKEN"
WL_ADMIN_ROLE_ID="$(printf '%s' "$BODY" | grep -o '{"id":"[^"]*","label":"Admin"' | cut -d'"' -f4)"
plus_one_year="$(date -u -v+1y "+%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || date -u -d "+1 year" "+%Y-%m-%dT%H:%M:%SZ")"

post /auth/api-keys "{\"name\":\"wl key\",\"expiresAt\":\"$plus_one_year\",\"roleId\":\"$WL_ADMIN_ROLE_ID\"}" -H "authorization: Bearer $WL_SECOND_TOKEN"
check "wl api key created" 201 "$STATUS"
WL_API_KEY_ID="$(printf '%s' "$BODY" | grep -o '"apiKey":{"id":"[^"]*"' | cut -d'"' -f6)"

post /auth/api-keys/token "{\"apiKeyId\":\"$WL_API_KEY_ID\"}" -H "authorization: Bearer $WL_SECOND_TOKEN"
check "wl api key token minted" 200 "$STATUS"
WL_API_KEY_TOKEN="$(extract token)"

post /auth/impersonate "{\"workspaceId\":\"$WL_SECOND_WORKSPACE\",\"userId\":\"$WL_USER_ID\"}" -H "authorization: Bearer $WL_API_KEY_TOKEN"
check "api keys cannot impersonate" 403 "$STATUS"
check_value "  api key message" "$(printf '%s' "$BODY" | grep -c "Can't impersonate user via api key")"

WL_COOKIE_EMAIL="ws-cookie-$SUFFIX@nudge.test"
WL_COOKIE_SUBDOMAIN="nudge-ws-cookie-$SUFFIX"

post_cookie_flow /auth/sign-up "{\"email\":\"$WL_COOKIE_EMAIL\",\"password\":\"$PASSWORD\",\"workspaceDisplayName\":\"WL Cookie\",\"subdomain\":\"$WL_COOKIE_SUBDOMAIN\"}" -H "origin: $WL_ORIGIN"
check "wl cookie sign up" 201 "$STATUS"
WL_SESSION_COOKIE="$(grep -o 'twenty-session=sess_[A-Za-z0-9_-]*' "$HDRS" | head -1 | cut -d= -f2)"
check_value "  wl cookie session issued" "$([ -n "$WL_SESSION_COOKIE" ] && echo 1 || echo 0)"

post_cookie_flow /auth/workspace "{\"displayName\":\"WL Cookie Second\",\"subdomain\":\"$WL_COOKIE_SUBDOMAIN-2\"}" -H "cookie: twenty-session=$WL_SESSION_COOKIE" -H "origin: $WL_ORIGIN"
check "cookie session creates a workspace" 201 "$STATUS"
WL_COOKIE_LOGIN_TOKEN="$(extract loginToken)"
WL_COOKIE_WORKSPACE="$(printf '%s' "$BODY" | grep -o '"workspace":{"id":"[^"]*"' | cut -d'"' -f6)"
check_value "  cookie flow login token" "$([ -n "$WL_COOKIE_LOGIN_TOKEN" ] && echo 1 || echo 0)"

post_cookie_flow /auth/tokens "{\"loginToken\":\"$WL_COOKIE_LOGIN_TOKEN\",\"workspaceId\":\"$WL_COOKIE_WORKSPACE\"}" -H "cookie: twenty-session=$WL_SESSION_COOKIE" -H "origin: $WL_ORIGIN"
check "cookie flow exchange into second workspace" 200 "$STATUS"
WL_COOKIE_NEW_SESSION="$(grep -o 'twenty-session=sess_[A-Za-z0-9_-]*' "$HDRS" | head -1 | cut -d= -f2)"
check_value "  replacement cookie issued" "$([ -n "$WL_COOKIE_NEW_SESSION" ] && echo 1 || echo 0)"

get_cookie_flow /auth/me -H "cookie: twenty-session=$WL_COOKIE_NEW_SESSION" -H "$SESSION_OPT_IN"
check "cookie me shows the second workspace" 200 "$STATUS"
check_value "  cookie second workspace in me" "$(printf '%s' "$BODY" | grep -c "\"id\":\"$WL_COOKIE_WORKSPACE\"")"

post_cookie_flow /auth/stop-impersonation "{}" -H "cookie: twenty-session=$WL_COOKIE_NEW_SESSION" -H "origin: $WL_ORIGIN"
check "cookie stop without impersonation denied" 403 "$STATUS"
check_value "  cookie not impersonating message" "$(printf '%s' "$BODY" | grep -c 'Not currently impersonating')"
