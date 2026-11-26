AT_EMAIL="tokens-$SUFFIX@nudge.test"
AT_NEW_PASSWORD="NewPassword456"
AT_LOGIN_PASSWORD="$PASSWORD"
AT_SERVER_LOG="${ACCOUNT_TOKENS_SERVER_LOG:-}"
AT_HDRS="$(mktemp)"

at_capture() {
  local response
  response="$(curl -s -D "$AT_HDRS" -w $'\n%{http_code}' -X POST "$BASE$1" \
    -H 'content-type: application/json' -H 'x-twenty-session-cookie: 1' \
    -H "origin: $BASE" -d "$2")"
  STATUS="${response##*$'\n'}"
  BODY="${response%$'\n'*}"
}

at_db() {
  docker exec infra-db-1 psql -U postgres -d "${E2E_DATABASE_NAME:-default}" -tAc "$1" | tr -d '[:space:]'
}

at_absent() {
  [ "$(printf '%s' "$BODY" | grep -c "$1")" = "0" ] && echo 1 || echo 0
}

at_capture /auth/sign-up "{\"email\":\"$AT_EMAIL\",\"password\":\"$PASSWORD\",\"workspaceDisplayName\":\"Token HQ\",\"subdomain\":\"nudge-tokens-$SUFFIX\"}"
check "tokens: sign up reset user" 201 "$STATUS"
AT_OWN_WS_ID="$(extract workspaceId)"
AT_COOKIE="$(grep -o 'twenty-session=sess_[A-Za-z0-9_-]*' "$AT_HDRS" | head -1 | cut -d= -f2)"
check_value "  session cookie captured" "$([ -n "$AT_COOKIE" ] && echo 1 || echo 0)"

get /auth/me -H "cookie: twenty-session=$AT_COOKIE" -H "x-twenty-session-cookie: 1"
check "tokens: session live before password change" 200 "$STATUS"

post /auth/email-password-reset-link "{\"email\":\"$AT_EMAIL\"}"
check "tokens: reset link accepted" 200 "$STATUS"
check_value "  success true" "$(printf '%s' "$BODY" | grep -c '"success":true')"

post /auth/email-password-reset-link "{\"email\":\"$AT_EMAIL\"}"
check "tokens: reset link second" 200 "$STATUS"
post /auth/email-password-reset-link "{\"email\":\"$AT_EMAIL\"}"
check "tokens: reset link third" 200 "$STATUS"
post /auth/email-password-reset-link "{\"email\":\"$AT_EMAIL\"}"
check "tokens: reset link throttled" 429 "$STATUS"
check_value "  limit reached code" "$([ "$(error_code)" = "LIMIT_REACHED" ] && echo 1 || echo 0)"

sleep 1

AT_RESET_TOKEN=""
if [ -n "$AT_SERVER_LOG" ] && [ -f "$AT_SERVER_LOG" ]; then
  AT_RESET_TOKEN="$(grep -o 'reset-password/[0-9a-f]\{64\}' "$AT_SERVER_LOG" | tail -1 | cut -d/ -f2)"
fi

if [ -n "$AT_RESET_TOKEN" ]; then
  check_value "  reset token recovered from log" 1

  post /auth/validate-password-reset-token "{\"passwordResetToken\":\"$AT_RESET_TOKEN\"}"
  check "tokens: validate reset token" 200 "$STATUS"
  check_value "  hasPassword true" "$(printf '%s' "$BODY" | grep -c '"hasPassword":true')"
  check_value "  returns the email" "$(printf '%s' "$BODY" | grep -c "\"email\":\"$AT_EMAIL\"")"

  post /auth/update-password-via-reset-token "{\"passwordResetToken\":\"$AT_RESET_TOKEN\",\"newPassword\":\"$AT_NEW_PASSWORD\"}"
  check "tokens: update password via reset token" 200 "$STATUS"

  if [ "$STATUS" = "200" ]; then
    AT_LOGIN_PASSWORD="$AT_NEW_PASSWORD"
  fi

  get /auth/me -H "cookie: twenty-session=$AT_COOKIE" -H "x-twenty-session-cookie: 1"
  check "tokens: sessions revoked on password change" 401 "$STATUS"

  post /auth/login-token "{\"email\":\"$AT_EMAIL\",\"password\":\"$PASSWORD\",\"workspaceId\":\"$AT_OWN_WS_ID\"}"
  check "tokens: old password rejected" 401 "$STATUS"

  post /auth/login-token "{\"email\":\"$AT_EMAIL\",\"password\":\"$AT_NEW_PASSWORD\",\"workspaceId\":\"$AT_OWN_WS_ID\"}"
  check "tokens: new password accepted" 200 "$STATUS"

  post /auth/validate-password-reset-token "{\"passwordResetToken\":\"$AT_RESET_TOKEN\"}"
  check "tokens: reset token invalidated after use" 403 "$STATUS"
else
  check_value "  hashed reset happy path needs ACCOUNT_TOKENS_SERVER_LOG" 1
fi

post /auth/validate-password-reset-token "{\"passwordResetToken\":\"$(printf 'a%.0s' {1..64})\"}"
check "tokens: unknown reset token rejected" 403 "$STATUS"
check_value "  forbidden code" "$([ "$(error_code)" = "FORBIDDEN_EXCEPTION" ] && echo 1 || echo 0)"

post /auth/update-password-via-reset-token "{\"passwordResetToken\":\"$(printf 'b%.0s' {1..64})\",\"newPassword\":\"$AT_NEW_PASSWORD\"}"
check "tokens: update with unknown token rejected" 403 "$STATUS"

post /auth/verify-email "{\"emailVerificationToken\":\"$(printf 'c%.0s' {1..64})\",\"email\":\"ghost-$SUFFIX@nudge.test\"}"
check "tokens: unknown verification token rejected" 403 "$STATUS"
check_value "  invalid token code" "$([ "$(error_code)" = "INVALID_TOKEN" ] && echo 1 || echo 0)"

post /auth/verify-email "{\"emailVerificationToken\":\"$(printf 'd%.0s' {1..64})\",\"email\":\"$AT_EMAIL\"}"
check "tokens: verify for verified email rejected" 400 "$STATUS"
check_value "  already verified code" "$([ "$(error_code)" = "EMAIL_ALREADY_VERIFIED" ] && echo 1 || echo 0)"

post /auth/resend-email-verification-token "{\"email\":\"$AT_EMAIL\"}"
check "tokens: resend verification gated" 400 "$STATUS"
check_value "  gated code" "$([ "$(error_code)" = "EMAIL_VERIFICATION_NOT_REQUIRED" ] || [ "$(error_code)" = "EMAIL_ALREADY_VERIFIED" ] && echo 1 || echo 0)"

post /auth/workspace-invite-hash-valid "{\"inviteHash\":\"not-a-hash-$SUFFIX\"}"
check "tokens: unknown invite hash invalid" 200 "$STATUS"
check_value "  isValid false" "$(printf '%s' "$BODY" | grep -c '"isValid":false')"

AT_INVITER_EMAIL="inviter-$SUFFIX@nudge.test"
post /auth/sign-up "{\"email\":\"$AT_INVITER_EMAIL\",\"password\":\"$PASSWORD\",\"workspaceDisplayName\":\"Invite HQ\",\"subdomain\":\"nudge-inv-$SUFFIX\"}"
check "tokens: inviter signed up" 201 "$STATUS"
AT_INVITER_TOKEN="$(extract accessToken)"
AT_INV_WS_ID="$(extract workspaceId)"
check_value "  inviter token present" "$([ -n "$AT_INVITER_TOKEN" ] && echo 1 || echo 0)"

AT_INVITE_HASH="$(at_db "SELECT \"inviteHash\" FROM core.\"workspace\" WHERE id = '$AT_INV_WS_ID'")"
check_value "  invite hash present" "$([ -n "$AT_INVITE_HASH" ] && echo 1 || echo 0)"

post /auth/workspace-invite-hash-valid "{\"inviteHash\":\"$AT_INVITE_HASH\"}"
check "tokens: invite hash valid" 200 "$STATUS"
check_value "  isValid true" "$(printf '%s' "$BODY" | grep -c '"isValid":true')"

post /auth/workspace-from-invite-hash "{\"inviteHash\":\"$AT_INVITE_HASH\"}"
check "tokens: workspace from invite hash" 200 "$STATUS"
check_value "  returns the workspace id" "$(printf '%s' "$BODY" | grep -c "\"id\":\"$AT_INV_WS_ID\"")"

post /auth/workspace-from-invite-hash "{\"inviteHash\":\"not-a-hash-$SUFFIX\"}"
check "tokens: workspace from unknown hash rejected" 400 "$STATUS"
check_value "  workspace does not exist" "$(printf '%s' "$BODY" | grep -c 'Workspace does not exist.')"

post /auth/workspace-invitations "{\"emails\":[\"$AT_EMAIL\"]}"
check "tokens: invitations need auth" 401 "$STATUS"

get /auth/workspace-invitations
check "tokens: invitations list needs auth" 401 "$STATUS"

post /auth/workspace-invitations "{\"emails\":[\"$AT_EMAIL\"]}" -H "authorization: Bearer $AT_INVITER_TOKEN"
check "tokens: invitation sent" 200 "$STATUS"
check_value "  success true" "$(printf '%s' "$BODY" | grep -c '"success":true')"
check_value "  result carries the email" "$(printf '%s' "$BODY" | grep -c "\"email\":\"$AT_EMAIL\"")"

AT_INVITE_TOKEN="$(at_db "SELECT value FROM core.\"appToken\" WHERE type = 'INVITATION_TOKEN' AND \"workspaceId\" = '$AT_INV_WS_ID' AND context->>'email' = '$AT_EMAIL'")"
check_value "  invitation token stored verbatim" "$([ ${#AT_INVITE_TOKEN} -eq 64 ] && echo 1 || echo 0)"

get /auth/workspace-invitations -H "authorization: Bearer $AT_INVITER_TOKEN"
check "tokens: invitations listed" 200 "$STATUS"
check_value "  one invitation" "$(printf '%s' "$BODY" | grep -c '"invitations":\[{')"
check_value "  token value not leaked" "$(at_absent '"value"')"

post /auth/workspace-invitations "{\"emails\":[\"$AT_EMAIL\"]}" -H "authorization: Bearer $AT_INVITER_TOKEN"
check "tokens: duplicate invitation reported" 200 "$STATUS"
check_value "  success false" "$(printf '%s' "$BODY" | grep -c '"success":false')"
check_value "  already invited error" "$(printf '%s' "$BODY" | grep -c 'already invited')"

post /auth/workspace-invitations "{\"emails\":[\"$AT_INVITER_EMAIL\"]}" -H "authorization: Bearer $AT_INVITER_TOKEN"
check "tokens: member invite rejected" 200 "$STATUS"
check_value "  already in workspace error" "$(printf '%s' "$BODY" | grep -c 'already in the workspace')"

post /auth/workspace-invitations "{\"emails\":[\"bad-email\"]}" -H "authorization: Bearer $AT_INVITER_TOKEN"
check "tokens: malformed invitee rejected" 400 "$STATUS"

post /auth/workspace-invitations "{\"emails\":[\"x-$SUFFIX@nudge.test\",\"x-$SUFFIX@nudge.test\"]}" -H "authorization: Bearer $AT_INVITER_TOKEN"
check "tokens: duplicate invitees rejected" 400 "$STATUS"

post /auth/workspace-invitations "{\"emails\":[\"y-$SUFFIX@nudge.test\"],\"roleId\":\"$(uuidgen)\"}" -H "authorization: Bearer $AT_INVITER_TOKEN"
check "tokens: unknown role rejected" 400 "$STATUS"

post /auth/workspace-invitations/accept "{\"invitationToken\":\"$(printf 'e%.0s' {1..64})\",\"email\":\"$AT_EMAIL\"}"
check "tokens: accept unknown invitation rejected" 403 "$STATUS"

post /auth/workspace-invitations/accept "{\"invitationToken\":\"$AT_INVITE_TOKEN\",\"email\":\"other-$SUFFIX@nudge.test\"}"
check "tokens: accept with wrong email rejected" 403 "$STATUS"
check_value "  email mismatch message" "$(printf '%s' "$BODY" | grep -c 'Email does not match the invitation')"

post /auth/workspace-invitations/accept "{\"invitationToken\":\"$AT_INVITE_TOKEN\",\"email\":\"$AT_EMAIL\"}"
check "tokens: invitation accepted" 200 "$STATUS"
check_value "  joins the inviting workspace" "$(printf '%s' "$BODY" | grep -c "\"id\":\"$AT_INV_WS_ID\"")"

post /auth/workspace-invitations/accept "{\"invitationToken\":\"$AT_INVITE_TOKEN\",\"email\":\"$AT_EMAIL\"}"
check "tokens: accepted invitation is single use" 403 "$STATUS"

post /auth/login-token "{\"email\":\"$AT_EMAIL\",\"password\":\"$AT_LOGIN_PASSWORD\",\"workspaceId\":\"$AT_INV_WS_ID\"}"
check "tokens: invited user can log in" 200 "$STATUS"
check_value "  login token issued" "$([ -n "$(extract loginToken)" ] && echo 1 || echo 0)"

AT_INVITEE_EMAIL="cookies-$SUFFIX@nudge.test"
post /auth/workspace-invitations "{\"emails\":[\"$AT_INVITEE_EMAIL\"]}" -H "authorization: Bearer $AT_INVITER_TOKEN"
check "tokens: second invitation sent" 200 "$STATUS"

AT_SECOND_ID="$(at_db "SELECT id FROM core.\"appToken\" WHERE type = 'INVITATION_TOKEN' AND \"workspaceId\" = '$AT_INV_WS_ID' AND context->>'email' = '$AT_INVITEE_EMAIL'")"
AT_SECOND_TOKEN="$(at_db "SELECT value FROM core.\"appToken\" WHERE type = 'INVITATION_TOKEN' AND \"workspaceId\" = '$AT_INV_WS_ID' AND context->>'email' = '$AT_INVITEE_EMAIL'")"

post /auth/workspace-invitations/resend "{\"appTokenId\":\"$AT_SECOND_ID\"}" -H "authorization: Bearer $AT_INVITER_TOKEN"
check "tokens: invitation resent" 200 "$STATUS"
check_value "  resend success" "$(printf '%s' "$BODY" | grep -c '"success":true')"

AT_SECOND_TOKEN_NEW="$(at_db "SELECT value FROM core.\"appToken\" WHERE type = 'INVITATION_TOKEN' AND \"workspaceId\" = '$AT_INV_WS_ID' AND context->>'email' = '$AT_INVITEE_EMAIL'")"
check_value "  resend rotated the token" "$([ -n "$AT_SECOND_TOKEN_NEW" ] && [ "$AT_SECOND_TOKEN_NEW" != "$AT_SECOND_TOKEN" ] && echo 1 || echo 0)"

post /auth/workspace-invitations/accept "{\"invitationToken\":\"$AT_SECOND_TOKEN\",\"email\":\"$AT_INVITEE_EMAIL\"}"
check "tokens: old token rejected after resend" 403 "$STATUS"

post /auth/workspace-invitations/accept "{\"invitationToken\":\"$AT_SECOND_TOKEN_NEW\",\"email\":\"$AT_INVITEE_EMAIL\"}"
check "tokens: new token accepted" 200 "$STATUS"

post /auth/login-token "{\"email\":\"$AT_INVITEE_EMAIL\",\"password\":\"$PASSWORD\",\"workspaceId\":\"$AT_INV_WS_ID\"}"
check "tokens: resent invitee can log in" 200 "$STATUS"

post /auth/workspace-invitations "{\"emails\":[\"third-$SUFFIX@nudge.test\"]}" -H "authorization: Bearer $AT_INVITER_TOKEN"
check "tokens: third invitation sent" 200 "$STATUS"

AT_THIRD_ID="$(at_db "SELECT id FROM core.\"appToken\" WHERE type = 'INVITATION_TOKEN' AND \"workspaceId\" = '$AT_INV_WS_ID' AND context->>'email' = 'third-$SUFFIX@nudge.test'")"

post /auth/workspace-invitations/delete "{\"appTokenId\":\"$AT_THIRD_ID\"}" -H "authorization: Bearer $AT_INVITER_TOKEN"
check "tokens: invitation revoked" 200 "$STATUS"
check_value "  success string" "$(printf '%s' "$BODY" | grep -c '"success"')"

post /auth/workspace-invitations/delete "{\"appTokenId\":\"$AT_THIRD_ID\"}" -H "authorization: Bearer $AT_INVITER_TOKEN"
check "tokens: revoking twice errors" 200 "$STATUS"
check_value "  error string" "$(printf '%s' "$BODY" | grep -c '"error"')"

get /auth/workspace-invitations -H "authorization: Bearer $AT_INVITER_TOKEN"
check_value "  deleted invitation gone" "$(at_absent "third-$SUFFIX@nudge.test")"

rm -f "$AT_HDRS"
