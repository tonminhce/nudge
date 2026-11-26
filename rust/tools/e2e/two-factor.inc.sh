EMAIL="2fa-$SUFFIX@nudge.test"
SUBDOMAIN="nudge-2fa-$SUFFIX"
TFA_PASSWORD="SuperSecret123"

totp_code_at() {
  node -e '
    const [secret, offsetSteps] = process.argv.slice(1);
    const crypto = require("node:crypto");
    const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let bits = 0;
    let value = 0;
    const bytes = [];
    for (const ch of secret) {
      const index = alphabet.indexOf(ch.toUpperCase());
      if (index < 0) process.exit(1);
      value = (value << 5) | index;
      bits += 5;
      if (bits >= 8) {
        bytes.push((value >>> (bits - 8)) & 0xff);
        bits -= 8;
      }
    }
    const counter = Math.floor(Date.now() / 1000 / 30) - Number(offsetSteps || 0);
    const counterBuffer = Buffer.alloc(8);
    counterBuffer.writeBigUInt64BE(BigInt(counter));
    const digest = crypto.createHmac("sha1", Buffer.from(bytes)).update(counterBuffer).digest();
    const offset = digest[digest.length - 1] & 0xf;
    const binary =
      ((digest[offset] & 0x7f) << 24) |
      ((digest[offset + 1] & 0xff) << 16) |
      ((digest[offset + 2] & 0xff) << 8) |
      (digest[offset + 3] & 0xff);
    process.stdout.write(String(binary % 1000000).padStart(6, "0"));
  ' "$1" "${2:-0}"
}

CODE_PATTERN='[0-9A-Z]\{5\}-[0-9A-Z]\{5\}-[0-9A-Z]\{5\}-[0-9A-Z]\{5\}'

post /auth/sign-up "{\"email\":\"$EMAIL\",\"password\":\"$TFA_PASSWORD\",\"workspaceDisplayName\":\"2FA HQ\",\"subdomain\":\"$SUBDOMAIN\"}"
check "2fa sign up" 201 "$STATUS"
TFA_WORKSPACE="$(extract workspaceId)"
TFA_USER_ID="$(printf '%s' "$BODY" | grep -o '"user":{"id":"[^"]*"' | cut -d'"' -f6)"
check_value "  workspace present" "$([ -n "$TFA_WORKSPACE" ] && echo 1 || echo 0)"
check_value "  user id present" "$([ -n "$TFA_USER_ID" ] && echo 1 || echo 0)"

post /auth/login-token "{\"email\":\"$EMAIL\",\"password\":\"$TFA_PASSWORD\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "2fa login token" 200 "$STATUS"
TFA_LOGIN_TOKEN="$(extract loginToken)"

post /auth/otp/provision "{\"loginToken\":\"$TFA_LOGIN_TOKEN\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "otp provision" 200 "$STATUS"
TFA_URI="$(extract uri)"
TFA_SECRET="$(printf '%s' "$TFA_URI" | sed -n 's/.*secret=\([A-Za-z2-7]*\).*/\1/p')"
check_value "  otpauth uri present" "$([ -n "$TFA_URI" ] && echo 1 || echo 0)"
check_value "  uri carries totp params" "$(printf '%s' "$TFA_URI" | grep -c 'algorithm=SHA1&issuer=')"
check_value "  secret extracted" "$([ ${#TFA_SECRET} -eq 16 ] && echo 1 || echo 0)"

post /auth/otp/provision "{\"loginToken\":\"$TFA_LOGIN_TOKEN\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "otp provision reuses pending secret" 200 "$STATUS"
check_value "  same uri" "$([ "$(extract uri)" = "$TFA_URI" ] && echo 1 || echo 0)"

post /auth/tokens "{\"loginToken\":\"$TFA_LOGIN_TOKEN\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "sign in while only pending" 200 "$STATUS"
TFA_ACCESS="$(extract accessToken)"
check_value "  access token present" "$([ -n "$TFA_ACCESS" ] && echo 1 || echo 0)"

TFA_OTP="$(totp_code_at "$TFA_SECRET" 0)"
post /auth/otp/verify "{\"otp\":\"$TFA_OTP\"}" -H "authorization: Bearer $TFA_ACCESS"
check "otp verify enrolls" 200 "$STATUS"
TFA_METHOD_ID="$(extract twoFactorAuthenticationMethodId)"
BACKUP_CODES="$(printf '%s' "$BODY" | grep -o '"backupCodes":\[[^]]*\]')"
FIRST_BACKUP_CODE="$(printf '%s' "$BACKUP_CODES" | grep -o "\"$CODE_PATTERN\"" | head -1 | tr -d '"')"
SECOND_BACKUP_CODE="$(printf '%s' "$BACKUP_CODES" | grep -o "\"$CODE_PATTERN\"" | sed -n 2p | tr -d '"')"
check_value "  method id present" "$([ -n "$TFA_METHOD_ID" ] && echo 1 || echo 0)"
check_value "  ten backup codes" "$(printf '%s' "$BACKUP_CODES" | grep -o "\"$CODE_PATTERN\"" | wc -l | awk '{print ($1 == 10) ? 1 : 0}')"

get /auth/otp/backup-codes/status -H "authorization: Bearer $TFA_ACCESS"
check "backup code status" 200 "$STATUS"
check_value "  ten remaining" "$(printf '%s' "$BODY" | grep -c '"remainingBackupCodeCount":10')"

post /auth/login-token "{\"email\":\"$EMAIL\",\"password\":\"$TFA_PASSWORD\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "login token after enrollment" 200 "$STATUS"
TFA_LOGIN_TOKEN="$(extract loginToken)"

post /auth/tokens "{\"loginToken\":\"$TFA_LOGIN_TOKEN\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "exchange requires otp" 401 "$STATUS"
check_value "  verification required code" "$([ "$(error_code)" = "TWO_FACTOR_AUTHENTICATION_VERIFICATION_REQUIRED" ] && echo 1 || echo 0)"

FUTURE_OTP="$(totp_code_at "$TFA_SECRET" 3)"
post /auth/otp/tokens "{\"loginToken\":\"$TFA_LOGIN_TOKEN\",\"otp\":\"$FUTURE_OTP\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "wrong otp rejected" 400 "$STATUS"
check_value "  invalid otp code" "$([ "$(error_code)" = "INVALID_OTP" ] && echo 1 || echo 0)"
check_value "  login token survives failed otp" "$([ -n "$TFA_LOGIN_TOKEN" ] && echo 1 || echo 0)"

post /auth/otp/tokens "{\"loginToken\":\"$TFA_LOGIN_TOKEN\",\"otp\":\"$FUTURE_OTP\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "second wrong otp still invalid not throttled" 400 "$STATUS"

TFA_OTP="$(totp_code_at "$TFA_SECRET" -1)"
post /auth/otp/tokens "{\"loginToken\":\"$TFA_LOGIN_TOKEN\",\"otp\":\"$TFA_OTP\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "next step otp issues tokens" 200 "$STATUS"
check_value "  access token issued" "$([ -n "$(extract accessToken)" ] && echo 1 || echo 0)"

post /auth/otp/tokens "{\"loginToken\":\"$TFA_LOGIN_TOKEN\",\"otp\":\"$TFA_OTP\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "otp replay rejected" 400 "$STATUS"
check_value "  replay is invalid otp" "$([ "$(error_code)" = "INVALID_OTP" ] && echo 1 || echo 0)"

post /auth/login-token "{\"email\":\"$EMAIL\",\"password\":\"$TFA_PASSWORD\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
TFA_LOGIN_TOKEN="$(extract loginToken)"
post /auth/otp/recovery/tokens "{\"loginToken\":\"$TFA_LOGIN_TOKEN\",\"recoveryCode\":\"$FIRST_BACKUP_CODE\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "backup code redeems login" 200 "$STATUS"
RECOVERY_ACCESS="$(extract accessToken)"
check_value "  tokens from backup code" "$([ -n "$RECOVERY_ACCESS" ] && echo 1 || echo 0)"
check_value "  no provisioning uri" "$(printf '%s' "$BODY" | grep -c '"provisioningUri":null')"

post /auth/login-token "{\"email\":\"$EMAIL\",\"password\":\"$TFA_PASSWORD\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
TFA_LOGIN_TOKEN="$(extract loginToken)"
post /auth/otp/recovery/tokens "{\"loginToken\":\"$TFA_LOGIN_TOKEN\",\"recoveryCode\":\"$FIRST_BACKUP_CODE\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "backup code works exactly once" 400 "$STATUS"
check_value "  invalid recovery code" "$([ "$(error_code)" = "INVALID_RECOVERY_CODE" ] && echo 1 || echo 0)"

TFA_OTP="$(totp_code_at "$TFA_SECRET" 0)"
post /auth/otp/recovery/generate "{\"userId\":\"$TFA_USER_ID\",\"otp\":\"$TFA_OTP\"}" -H "authorization: Bearer $RECOVERY_ACCESS"
check "recovery code for self rejected" 403 "$STATUS"
check_value "  target not allowed" "$([ "$(error_code)" = "RECOVERY_CODE_TARGET_NOT_ALLOWED" ] && echo 1 || echo 0)"

sleep $(( 61 - $(date +%s) % 30 ))
TFA_OTP="$(totp_code_at "$TFA_SECRET" 0)"
post /auth/otp/backup-codes "{\"otp\":\"$TFA_OTP\"}" -H "authorization: Bearer $RECOVERY_ACCESS"
check "backup codes regenerated" 200 "$STATUS"
NEW_FIRST_CODE="$(printf '%s' "$BODY" | grep -o "\"$CODE_PATTERN\"" | head -1 | tr -d '"')"
check_value "  fresh code differs" "$([ -n "$NEW_FIRST_CODE" ] && [ "$NEW_FIRST_CODE" != "$FIRST_BACKUP_CODE" ] && echo 1 || echo 0)"

get /auth/otp/backup-codes/status -H "authorization: Bearer $RECOVERY_ACCESS"
check_value "  fresh set counts ten" "$(printf '%s' "$BODY" | grep -c '"remainingBackupCodeCount":10')"

post /auth/login-token "{\"email\":\"$EMAIL\",\"password\":\"$TFA_PASSWORD\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
TFA_LOGIN_TOKEN="$(extract loginToken)"
post /auth/otp/recovery/tokens "{\"loginToken\":\"$TFA_LOGIN_TOKEN\",\"recoveryCode\":\"$SECOND_BACKUP_CODE\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "revoked old code rejected" 400 "$STATUS"

post /auth/otp/recovery/tokens "{\"loginToken\":\"$TFA_LOGIN_TOKEN\",\"recoveryCode\":\"totally-wrong-code\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "garbage recovery code rejected" 400 "$STATUS"
check_value "  invalid recovery code" "$([ "$(error_code)" = "INVALID_RECOVERY_CODE" ] && echo 1 || echo 0)"

post /auth/otp/method/delete "{\"twoFactorAuthenticationMethodId\":\"$TFA_METHOD_ID\"}" -H "authorization: Bearer $RECOVERY_ACCESS"
check "2fa method disabled" 200 "$STATUS"
check_value "  success true" "$(printf '%s' "$BODY" | grep -c '"success":true')"

get /auth/otp/backup-codes/status -H "authorization: Bearer $RECOVERY_ACCESS"
check_value "  backup codes gone" "$(printf '%s' "$BODY" | grep -c '"remainingBackupCodeCount":0')"

post /auth/login-token "{\"email\":\"$EMAIL\",\"password\":\"$TFA_PASSWORD\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
TFA_LOGIN_TOKEN="$(extract loginToken)"
post /auth/tokens "{\"loginToken\":\"$TFA_LOGIN_TOKEN\",\"workspaceId\":\"$TFA_WORKSPACE\"}"
check "exchange works after disable" 200 "$STATUS"
TFA_ACCESS="$(extract accessToken)"

post /auth/otp/verify "{\"otp\":\"000000\"}" -H "authorization: Bearer $RECOVERY_ACCESS"
check "verify without method rejected" 403 "$STATUS"
check_value "  method not found" "$([ "$(error_code)" = "INVALID_CONFIGURATION" ] && echo 1 || echo 0)"

post /auth/otp/provision-user "" -H "authorization: Bearer $TFA_ACCESS"
check "authenticated provisioning" 200 "$STATUS"
check_value "  uri present" "$([ -n "$(extract uri)" ] && echo 1 || echo 0)"
