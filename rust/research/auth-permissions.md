# Twenty CRM — Auth, Permissions, Workspace Lifecycle, Config, Billing

Research doc for the Rust port (`rust-crm`). Reference source (read-only):
`C:\Users\tonminh123\Documents\crm-research\twenty` @ commit `51cff3a` (2026-11-18).

All paths below are relative to that repo. `packages/twenty-server/src/...` is abbreviated `srv/`.
Everything marked **Enterprise** lives behind the enterprise license (`/* @license Enterprise */`).

---

## 1. AUTH

### 1.1 Token types (the `type` claim)

`srv/src/engine/core-modules/auth/types/jwt-token-type.enum.ts:1-17` —
`ACCESS, REFRESH, WORKSPACE_AGNOSTIC, LOGIN, FILE, FILE_UPLOAD, API_KEY, REMOTE_SERVER,
KEY_ENCRYPTION_KEY, APPLICATION_ACCESS, APPLICATION_REFRESH, APP_OAUTH_STATE,
APPLICATION_REGISTRATION_GITHUB_CLAIM_STATE, APPROVED_ACCESS_DOMAIN, PLAYGROUND`.

Payload shapes (all extend `CommonPropertiesJwtPayload = { sub: string }`,
`types/common-properties-jwt-payload.type.ts:1-3`); each file is
`srv/src/engine/core-modules/auth/types/<name>.type.ts`:

- **ACCESS** — `sub, userId, workspaceId, workspaceMemberId?, userWorkspaceId, type, authProvider, isImpersonating?, impersonatorUserWorkspaceId?, impersonatedUserWorkspaceId?`
- **REFRESH** — `sub(=userId), userId, workspaceId?, jti?, authProvider?, targetedTokenType, isImpersonating?, impersonatorUserWorkspaceId?, impersonatedUserWorkspaceId?`
- **WORKSPACE_AGNOSTIC** — `sub(=userId), userId, authProvider, type` — **no workspaceId**
- **LOGIN** — `sub(=email), workspaceId, authProvider, impersonatorUserWorkspaceId?, jti?`
- **API_KEY** — `sub(=workspaceId), workspaceId, workspaceMemberId?, jti? (= apiKey.id)`
- **PLAYGROUND** — ACCESS minus the impersonation fields, `type=PLAYGROUND`
- **APPLICATION_ACCESS** — `sub(=applicationId), applicationId, workspaceId, userWorkspaceId?, userId?, agentId?, workspaceDeletionRequestTimestamp?`
- **APPLICATION_REFRESH** — `sub(=applicationId), applicationId, workspaceId, userWorkspaceId?, userId?, iat?`
- **FILE / FILE_UPLOAD** — `sub, workspaceId, fileId` (+ legacy FILE shape in `file-token-jwt-payload-legacy.type.ts`: `filename, workspaceMemberId?, noteBlockId?, attachmentId?, personId?`)
- **TRANSIENT** (`transient-token-jwt-payload.type.ts`) — `sub(=workspaceMemberId), userId, workspaceId, workspaceMemberId`, `type=LOGIN`
- **APP_OAUTH_STATE** — `workspaceId, connectionProviderId, userId, userWorkspaceId, scope, reconnectingConnectedAccountId, redirectLocation, flowSecretId`
- **APPROVED_ACCESS_DOMAIN** — `workspaceId, approvedAccessDomainId, domain`
- **APPLICATION_REGISTRATION_GITHUB_CLAIM_STATE** — `applicationRegistrationId, workspaceId, userId`

`AuthProviderEnum` (`srv/src/engine/core-modules/workspace/types/workspace.type.ts:1-7`):
`google | microsoft | password | sso | impersonation`.

### 1.2 Signing: algorithms, secret derivation, key rotation

**Algorithms** (`srv/src/engine/core-modules/jwt/constants/jwt-algorithm.constant.ts:3-12`):
`JWT_LEGACY_ALGORITHM = 'HS256'`, `JWT_ASYMMETRIC_ALGORITHM = 'ES256'`,
`JWT_SUPPORTED_VERIFY_ALGORITHMS = ['HS256','ES256']`.

**All tokens are signed with ES256** via `JwtWrapperService.signAsyncOrThrow`
(`srv/src/engine/core-modules/jwt/services/jwt-wrapper.service.ts:56-83`): key = current signing
key's private PEM, header `kid = signingKey.id`, `alg = ES256`.

**Legacy HS256 secret derivation** (verification of legacy tokens + fallback):
`jwt-wrapper.service.ts:207-216` — `sha256(APP_SECRET ‖ body ‖ type)` as lowercase hex, where
`body` = `payload.workspaceId` if present else `payload.userId`
(`extractAppSecretBody`, `:226-243`; schemas `:38-46`). **No separator** between the parts.

**Key resolution on verify** (`resolveVerificationKey`, `:102-146`): decode the header; if `kid`
is present and `alg === 'ES256'` → look up `signingKey.publicKey` by `kid` in the entity cache
(namespace `signingKeyPublicKey`, revoked keys excluded) → verify ES256. Otherwise → legacy path:
decode the payload, extract the body, derive the HS256 secret, verify HS256.

**API-key backward-compat** (`:160-186`): API keys created before 2025-12-12 were signed with
`ACCESS` type; if verification with the `API_KEY` secret fails, retry with
`generateAppSecret(ACCESS, body)`.

**Signing-key table** `core.signingKey` (`srv/src/engine/core-modules/jwt/entities/signing-key.entity.ts`):
`id uuid pk`, `publicKey varchar`, `privateKey varchar NULL` (encrypted; CHECK
`"privateKey" IS NULL OR "privateKey" LIKE 'enc:v2:%'`), `isCurrent boolean` (partial unique
index on `isCurrent = true`), `revokedAt timestamptz NULL`.

**Key generation** (`srv/src/engine/core-modules/jwt/services/jwt-key-manager.service.ts:224-236`):
`generateKeyPairSync('ec', { namedCurve: 'P-256' })`; private exported as **PKCS#8 PEM**, public as
**SPKI PEM**; private stored via `SecretEncryptionService.encryptVersioned` (`enc:v2:`).

**Rotation** (`jwt/services/signing-key-rotation.service.ts`): cron
`ROTATE_SIGNING_KEYS_CRON_PATTERN = '15 3 * * *'`
(`jwt/constants/rotate-signing-keys-cron-pattern.constant.ts:3`), job
`jwt/crons/jobs/rotate-signing-keys.cron.job.ts`. Rotation happens only when
`SIGNING_KEY_ROTATION_DAYS` is set and the current key is older; a valid enterprise plan is
required. Transaction: old row → `isCurrent=false, privateKey=null`; new row `isCurrent=true`.
Revoking nulls the private key and sets `revokedAt` (`jwt-key-manager.service.ts:117-152`).
The current key is cached in-process for 60 s (`CURRENT_SIGNING_KEY_LOCAL_TTL_MS`).

**Verification errors** (`jwt-wrapper.service.ts:200-221`): `TokenExpiredError` → "Token has
expired."/`UNAUTHENTICATED`; `JsonWebTokenError` → "Token invalid."/`UNAUTHENTICATED`; else
"Unknown token error."/`INVALID_INPUT`.

### 1.3 Password hashing

`srv/src/engine/core-modules/auth/auth.util.ts:11-23` —
`PASSWORD_REGEX = /^.{8,50}$/` (8–50 chars, no newline), `saltRounds = 10`,
`bcrypt.hash(password, 10)` / `bcrypt.compare`. Verification path
(`auth/utils/assert-user-password-is-valid-or-throw.util.ts:11-36`): empty `passwordHash` →
"Incorrect login method"; mismatch → "Wrong password."
Legacy symmetric helper in the same file (`auth.util.ts:25-52`): `encryptText`/`decryptText` =
**AES-256-CTR**, key `sha512(key).hex[0..32]`, random 16-byte IV prefix, base64 output.

### 1.4 Sign-up / sign-in flows (GraphQL)

All in `srv/src/engine/core-modules/auth/auth.resolver.ts` (1298 lines).

| Mutation | Guard | Purpose |
|---|---|---|
| `checkUserExists(email, captcha)` | `CaptchaGuard, Public, NoPermission` | `:213` — `exists`, `availableWorkspacesCount`, `isEmailVerified` |
| `getLoginTokenFromCredentials(email,password,origin)` | `CaptchaGuard, Public` | `:256` — validates the password, returns a **loginToken** only |
| `signIn(email,password)` | `CaptchaGuard, Public` | `:291` — workspace-agnostic token + refresh token + available workspaces |
| `verifyEmailAndGetLoginToken` / `verifyEmailAndGetWorkspaceAgnosticToken` | `Public` | `:325` / `:400` |
| `getAuthTokensFromOTP(loginToken,otp)` | `CaptchaGuard, Public` | `:450` — 2FA TOTP step |
| `getAuthTokensFromTwoFactorAuthenticationRecoveryCode` | `CaptchaGuard, Public` | `:496` |
| `signUp(email,password,locale,…)` | `CaptchaGuard, Public` | `:549` — sign-up **without** workspace |
| `signUpInWorkspace(…)` | `CaptchaGuard, Public` | `:~590` — sign-up into an invited workspace |
| `signUpInNewWorkspace(displayName,subdomain)` | `AccessPolicyGuard(AUTH_WRITE)`, `WorkspaceCreationCaptchaGuard` | `:~680` |
| `generateTransientToken` | `AccessPolicyGuard(AUTH_WRITE)` | `:~760` |
| `getAuthTokensFromLoginToken(loginToken,origin)` | `Public` | `:840` — **the login→tokens exchange** |
| `getAuthTokensFromSSOExchangeToken(ssoExchangeToken)` | `Public` | `:925` |
| `authorizeApp(…)` | `AccessPolicyGuard(AUTH_WRITE)` | `:~1090` — OAuth 2.1 code + PKCE |
| `renewToken(appToken)` | `Public` | `:~1105` — refresh rotation |
| `signOut(refreshToken?)` | `Public` | `:~1120` |
| `generateApiKeyToken` / `generatePlaygroundToken` | `AccessPolicyGuard(..., [API_KEYS_AND_WEBHOOKS(, ROLES)])` | `:~1150` / `:~1170` |
| `emailPasswordResetLink` / `updatePasswordViaResetToken` / `validatePasswordResetToken` | `CaptchaGuard, Public` / `Public` / `Public` | `:~1190`+ |

`AUTH_WRITE_ACCESS_POLICY` = `{ authPrincipals: { userSession: true, apiKey: false,
oauthClient: false, application: false }, requiredPermissionFlags: [] }` (`:139-146`);
`WORKSPACE_CREATION_ACCESS_POLICY` = the same with `oauthClient:{withUser:true,withoutUser:false}`
and `application:{withUser:true,withoutUser:false}` (`:129-138`).

**Canonical sign-in (two steps).**

1. `getLoginTokenFromCredentials` → `AuthService.validateLoginWithPasswordAndJoinWorkspaceIfInvited`
   (`auth/services/auth.service.ts:126-176`): the workspace is resolved from `origin`;
   `assertPasswordAuthAllowedOnWorkspaceOrThrow` rejects the workspace unless
   `isPasswordAuthEnabled` (or `isPasswordAuthBypassEnabled` **and** the user holds
   `PermissionFlagType.SSO_BYPASS`); **membership is checked before the password** (no user
   enumeration); a non-member needs a valid workspace invitation and is then auto-added;
   the password is bcrypt-verified; `checkIsEmailVerified` runs only when
   `IS_EMAIL_VERIFICATION_REQUIRED`. Result: a **LOGIN** JWT with `jti = randomUUID()`,
   `expiresIn = LOGIN_TOKEN_EXPIRES_IN` (15m).
2. `getAuthTokensFromLoginToken` (`:840-925`): `verifyLoginToken` (signature, `type===LOGIN`,
   valid `authProvider`); `validateWorkspaceAccess(origin, tokenWorkspaceId)` (the token's
   workspace must equal the origin workspace); `validateUserAccess` (user exists, email verified
   when required, a `userWorkspace` row exists); the impersonation branch when
   `authProvider === Impersonation` (see §1.12); otherwise
   `validateRegularAuthentication` → the 2FA requirement check
   (`TwoFactorAuthenticationService.validateTwoFactorAuthenticationRequirement`): throws
   `TWO_FACTOR_AUTHENTICATION_VERIFICATION_REQUIRED` when the user already has a *verified*
   method (→ the client must call `getAuthTokensFromOTP`), and
   `TWO_FACTOR_AUTHENTICATION_PROVISION_REQUIRED` when
   `workspace.isTwoFactorAuthenticationEnforced`; then `consumeLoginTokenOrThrow` — **single
   use** via `cacheStorage.setIfAbsent('login-token:consumed:<jti>', …, LOGIN_TOKEN_EXPIRES_IN)`
   (`auth/constants/consumed-login-token-cache-key-prefix.constant.ts:3`); then
   `AuthService.verify(email, workspaceId, authProvider)` (`:390-455`) → access + refresh tokens;
   finally `userSessionService.issueSessionForTokenPair(...)` sets the session cookie.

`AuthService.verify` clears `user.passwordHash` in memory, then
`accessTokenService.generateAccessToken({userId, workspaceId, authProvider})` and
`refreshTokenService.generateRefreshToken({userId, workspaceId, authProvider,
targetedTokenType: ACCESS})`.

### 1.5 Access / refresh / workspace-agnostic token issuance

**Access** (`auth/token/services/access-token.service.ts:130-170`): `expiresIn =
ACCESS_TOKEN_EXPIRES_IN` (30m); `resolveTokenSubject(userId, workspaceId)` loads the user,
workspace and `userWorkspace` and, for a provisioned workspace, the `workspaceMember` row
(via a system auth context) to fill `workspaceMemberId`; payload `sub=user.id, userId,
workspaceId, workspaceMemberId, userWorkspaceId, type=ACCESS, authProvider, isImpersonating,
impersonatorUserWorkspaceId, impersonatedUserWorkspaceId`; signed ES256, **no `jti`**.

**Refresh** (`auth/token/services/refresh-token.service.ts:115-155`): `expiresIn =
REFRESH_TOKEN_EXPIRES_IN` (60d; impersonation refresh tokens use a hardcoded `'1d'`); a
`core.appToken` row is created (`type=REFRESH_TOKEN`); the JWT carries `sub=userId,
jti = appToken.id`, `targetedTokenType`, `workspaceId?`, `authProvider`, impersonation fields and
is signed with `jwtid = appToken.id`. `verifyRefreshToken` (`:34-113`) requires
`type===REFRESH`, `jti`, `sub`, an existing `appToken` and an existing user; a revoked token is
still renewable **only** when `context.revokedReason` is unset **and**
`revokedAt > now - REFRESH_TOKEN_REUSE_GRACE_PERIOD` (1m)
(`auth/utils/is-revoked-refresh-token-still-renewable.util.ts`).
`RenewTokenService.generateTokensFromRefreshToken` (`token/services/renew-token.service.ts:38-121`)
takes a **shared** advisory lock `pg_advisory_xact_lock_shared(hashtextextended('user-authentication:<userId>',0))`
(`auth/utils/acquire-user-authentication-lock.util.ts:9-27`), re-reads the token under lock,
refuses when `context.revokedReason` is set, revokes the old token and issues a new one, then
issues a new access token (or a workspace-agnostic token when
`targetedTokenType === WORKSPACE_AGNOSTIC`).

**Workspace-agnostic** (`token/services/workspace-agnostic-token.service.ts:29-72`):
`expiresIn = WORKSPACE_AGNOSTIC_TOKEN_EXPIRES_IN` (30m); payload `sub=user.id, userId,
authProvider, type=WORKSPACE_AGNOSTIC` (no workspace).

**Transient** (`token/services/transient-token.service.ts:22-54`): `type=LOGIN` payload with
`sub=workspaceMemberId`, `expiresIn = SHORT_TERM_TOKEN_EXPIRES_IN` (5m).

**Login** (`token/services/login-token.service.ts:38-70`): `type=LOGIN`, `sub=email`,
`workspaceId`, `authProvider`, `impersonatorUserWorkspaceId?`, `jti = randomUUID()`,
`expiresIn = LOGIN_TOKEN_EXPIRES_IN` (15m).

### 1.6 Email verification

`auth/token/services/email-verification-token.service.ts` — plain token =
**`crypto.randomBytes(32).toString('hex')`** (64 hex chars), stored as **`sha256(plainToken)` hex**
in `core.appToken` (`type=EMAIL_VERIFICATION_TOKEN`, `value=hashed`, `context.email`,
`expiresAt`). `validateEmailVerificationTokenOrThrow` re-hashes, looks up by `value`+type, checks
`expiresAt`, checks `context.email === email`, and refuses when an **already verified** user has
that email. `expiresIn = EMAIL_VERIFICATION_TOKEN_EXPIRES_IN` (1h). Resend cooldown 1 minute
(`email-verification/services/email-verification.service.ts`,
`deleteExistingTokensAfterCooldownOrThrow`). Triggers: `SIGN_UP | EMAIL_UPDATE`
(`email-verification/email-verification.constants.ts:1-4`). The email is only sent when
`IS_EMAIL_VERIFICATION_REQUIRED=true`.

### 1.7 Password reset

`auth/services/reset-password.service.ts` — plain reset token =
**`crypto.randomBytes(32).toString('hex')`**, stored as **`sha256(plainResetToken)` hex** in
`core.appToken` (`type=PASSWORD_RESET_TOKEN`); `expiresIn = PASSWORD_RESET_TOKEN_EXPIRES_IN`
(**5m**); `rotatePasswordResetToken` revokes all previous reset rows for the user then inserts
the new one (**one live reset token per user**); `validatePasswordResetToken` looks up by
sha256 + type with `expiresAt > now` and `revokedAt IS NULL`.
`updatePasswordViaResetToken` → `AuthService.updatePassword(userId, newPassword)`
(`auth.service.ts:697+`): re-validates `PASSWORD_REGEX`, re-hashes with bcrypt(10), **revokes all
refresh tokens of the user**, **revokes all sessions** (reason `PASSWORD_CHANGED`), and sends a
"password changed" email. Throttle: `emailPasswordResetLink` uses a token bucket, max **3** per
**15 min** per email (`auth.resolver.ts:157-160`).

### 1.8 Secret encryption (`enc:v2` envelope) — required for stored secrets

`srv/src/engine/core-modules/secret-encryption/`:
- Envelope **`enc:v2:<keyId>:<payloadBase64>`** (`constants/secret-encryption.constant.ts`,
  `utils/format-secret-encryption-envelope-v2.util.ts`).
- `keyId` = **first 8 hex chars of `sha256(rawKey)`** (`utils/compute-encryption-key-id.util.ts`).
- Payload = base64 of `IV(12) ‖ ciphertext ‖ authTag(16)`, **AES-256-GCM**.
- Derived key = `hkdfSync('sha256', ikm=rawKey, salt=32 zero bytes,
  info='twenty:enc:v2:'+(workspaceId ?? 'instance'), 32)` (`utils/derive-gcm-key.util.ts`).
- Primary key = `ENCRYPTION_KEY` if set else `APP_SECRET`; `FALLBACK_ENCRYPTION_KEY` is used for
  decryption by keyId (`utils/resolve-encryption-keys-or-throw.util.ts`).
- Legacy path: **AES-256-CTR**, key `sha512(rawKey).hex[0..32]`, 16-byte IV prefix, base64,
  no integrity tag (`utils/encrypt-aes-ctr.util.ts`).
- Instance HMAC (session cookies): `hkdfSync('sha256', rawKey, zeros,
  'twenty:hmac:v1:session-cookie', 32)` plus legacy `sha256(APP_SECRET + 'SESSION_STORE_SECRET')`
  hex (`utils/derive-instance-hmac-key.util.ts`, `utils/resolve-session-cookie-secrets.util.ts`).

### 1.9 Session cookies (session auth)

- `core.userSession` (`user-session/user-session.entity.ts`): `id uuid`, `tokenHash text` (unique),
  `userId`, `workspaceId?`, `userWorkspaceId?`, `authProvider`, `isImpersonating`,
  `impersonatorUserWorkspaceId?`, `impersonatedUserWorkspaceId?`, `userAgent`, `ipAddress`,
  `country`, `region`, `city`, `expiresAt`, `lastActiveAt`, `revokedAt?`, `revokedReason?`.
- Token = **`sess_` + `crypto.randomBytes(32).toString('base64url')`**
  (`user-session/utils/generate-user-session-token.util.ts`), stored as **`sha256(token)` hex**
  (`utils/hash-user-session-token.util.ts`).
- Cookie names (`user-session/constants/`): secure **`__Host-twenty-session`**, insecure
  `twenty-session`; impersonator `__Host-twenty-impersonator-session` /
  `twenty-impersonator-session`.
- Cookie options (`user-session/services/user-session-cookie.service.ts:74-82`): `httpOnly: true`,
  `secure: isSecureCookieDeployment(...)`, `sameSite: AUTH_COOKIE_SAME_SITE` (default `lax`),
  `path: '/'`. `isSecureCookieDeployment` = `SERVER_URL` is https **or** `sameSite === 'none'`.
- The cookie is only read/written when the request may carry credentials:
  `resolveSessionCookieAccess` → `opted-in` (opt-in header present) | `navigation`
  (`Sec-Fetch-Mode: navigate`) | `denied`.
- Lifetimes: absolute `SESSION_ABSOLUTE_LIFETIME` (180d), idle `SESSION_IDLE_TIMEOUT` (30d);
  impersonation sessions use a hardcoded `1d`. `lastActiveAt` is touched at most every
  `min(5min, idleTimeout/2)`. Session cache TTL 60 s, Redis namespace `engine:auth-session`.
- **Session tokens are cookie-only**: a `sess_` token as a Bearer token is rejected
  (`access-token.service.ts:207-213`).
- Revocation reasons: `USER_SIGN_OUT, USER_REVOKED, SUPERSEDED, PASSWORD_CHANGED,
  IMPERSONATION_ENDED, TWO_FACTOR_AUTHENTICATION_RESET`. Cleanup cron `0 3 * * *`; ended sessions
  retained 30 days.
- CSRF (`srv/src/engine/middlewares/cookie-session-csrf.middleware.ts`): unsafe methods (not
  GET/HEAD/OPTIONS) with a cookie session require an allowed `Origin`; failure → 403
  `CSRF_ORIGIN_MISMATCH`. Bearer-token requests skip the check.

### 1.10 SSO / OAuth

**Google** (`auth/strategies/google.auth.strategy.ts`): clientID `AUTH_GOOGLE_CLIENT_ID`,
clientSecret `AUTH_GOOGLE_CLIENT_SECRET`, callbackURL `AUTH_GOOGLE_CALLBACK_URL`,
`scope:['email','profile']`; `state` = JSON `{workspaceInviteHash, workspaceId,
billingCheckoutSessionState, action, locale, returnToPath}`. Only an email with
`verified === true` is accepted, else `EMAIL_NOT_VERIFIED`.

**Microsoft** (`auth/strategies/microsoft.auth.strategy.ts`): `AUTH_MICROSOFT_*`,
`tenant:'common'`, `scope:['User.Read']`; email from `userPrincipalName`.

**Social sign-in handler** (`AuthService.signInUpWithSocialSso`, `auth.service.ts:~990-1105`):
email lowercased. With **no** workspace & no invite hash → create/reuse the user (verified),
issue an **SSO exchange token** (opaque `crypto.randomBytes(32).toString('hex')`, stored as
`sha256(hex)` in `core.appToken` `type=SSO_EXCHANGE_TOKEN`, `expiresIn =
SHORT_TERM_TOKEN_EXPIRES_IN` 5m, single-use via `DELETE … WHERE revokedAt IS NULL AND
deletedAt IS NULL`), redirect to `<base>/welcome#ssoExchangeToken=<token>` (fragment keeps it out
of logs/Referer). With a workspace/invite → resolve workspace + invitation, check access,
sign in/up, create the SSO connected account (feature-flagged), issue a **login token**, redirect
to `/verify?loginToken=…`.

**Enterprise OIDC/SAML** (`srv/src/engine/core-modules/sso/`):
- IdP row `core.workspaceSSOIdentityProvider` (`sso/workspace-sso-identity-provider.entity.ts`):
  `type` OIDC|SAML, `issuer`, `clientID`, `clientSecret`, `ssoURL`, `certificate`, `fingerprint`,
  `status` Active|Inactive|Error.
- OIDC: `openid-client` discovery, **PKCE S256** (`code_challenge_method:'S256'`,
  `usePKCE:true`), scope `openid email profile`, `response_types:['code']`. Id-token algorithm
  resolution (`sso/utils/resolve-id-token-signing-algorithm.util.ts`): prefer `RS256`, else the
  first **signed** advertised algorithm (`none` filtered out). Email = `userinfo.email ?? userinfo.upn`.
- SAML (`auth/strategies/saml.auth.strategy.ts`): `wantAssertionsSigned:true`,
  `wantAuthnResponseSigned:false`, `disableRequestedAuthnContext:true`, `signatureAlgorithm:'sha256'`;
  IdP cert = base64 body of the stored certificate; assertion email =
  `profile.email ?? profile.mail ?? profile.nameID`; replay protection records the assertion `ID`
  + max bearer `NotOnOrAfter` in Redis (`saml-assertion:consumed:<idpId>:<assertionId>`).
- Endpoints (`auth/controllers/sso-auth.controller.ts`): `GET /auth/saml/metadata/:identityProviderId`,
  `GET /auth/oidc/login/:identityProviderId`, `GET /auth/saml/login/:identityProviderId`,
  `GET /auth/oidc/callback`, `POST /auth/saml/callback/:identityProviderId`. Callback URL =
  `<SERVER_URL>/auth/<type>/callback[/<idpId>]`, issuer URL = `<SERVER_URL>/auth/<type>/login/<idpId>`.
- SSO requires the `SSO` billing entitlement.

**OAuth 2.1 / MCP app authorization** (`AuthService.generateAuthorizationCode`,
`auth.service.ts:~540-700`): PKCE mandatory for public clients; the redirect URI must be
registered (or a loopback host when none is); scope validation; `resource` must be served by a
known origin; authorization code = `crypto.randomBytes(42).toString('hex')` stored as **sha256
hex** in `core.appToken` `type=AUTHORIZATION_CODE` with **5 min** expiry.

### 1.11 Impersonation

`srv/src/engine/core-modules/impersonation/` — levels `server` (target in another workspace) and
`workspace` (`services/impersonation-authorization.service.ts` `getImpersonationLevel`).
**Server level** requires `impersonator.user.canImpersonate === true`,
`targetWorkspace.allowImpersonation === true`, and the impersonator's workspace to be the
server-admin workspace when `SERVER_ADMIN_WORKSPACE_ID` is set
(`utils/is-server-admin-access-allowed-from-workspace.util.ts`); 2FA is mandatory unless
`NODE_ENV=development`. **Workspace level** requires the impersonator's role to hold
`PermissionFlagType.IMPERSONATE` in the target workspace
(`SettingPermissionsService.userWorkspaceHasPermissionFlag`). Escalation guard: a target with
admin privileges may only be impersonated by another admin
(`utils/user-can-server-impersonate-target.util.ts`,
`utils/user-can-workspace-impersonate-target.util.ts`). Denial reasons
(`constants/impersonation-denial-by-reason.constant.ts`):
`SERVER_LEVEL_NOT_ALLOWED, SERVER_LEVEL_2FA_PROVISION_REQUIRED,
SERVER_LEVEL_2FA_VERIFICATION_REQUIRED, WORKSPACE_LEVEL_NOT_ALLOWED, TARGET_HAS_ADMIN_PRIVILEGES`.
Flow: `ImpersonationService.impersonate` → a **LOGIN** token with `authProvider=Impersonation`
and `impersonatorUserWorkspaceId`; the client exchanges it via `getAuthTokensFromLoginToken` →
`AuthService.generateImpersonationAccessTokenAndRefreshToken` → access token with
`isImpersonating=true`, `impersonatorUserWorkspaceId`, `impersonatedUserWorkspaceId`,
`authProvider=Impersonation`; the refresh token lifetime is `1d`. On sign-in while impersonating
the impersonator's cookie is **parked** in the impersonator cookie and restored by
`stopImpersonation`. Every attempt is logged as a workspace event `IMPERSONATION_EVENT`.

### 1.12 2FA (TOTP)

`srv/src/engine/core-modules/two-factor-authentication/` — strategy `TOTP` only
(`strategies/otp/totp/totp.strategy.ts`). **TOTP parameters**
(`strategies/otp/totp/constants/totp.strategy.constants.ts`): `algorithm sha1`, `digits 6`,
`encoding hex`, `window 1`, `step 30` (RFC 6238 defaults, otplib-compatible). The secret
(`authenticator.generateSecret()`) is stored **encrypted** in
`core.twoFactorAuthenticationMethod.secret` (CHECK `secret LIKE 'enc:v2:%'`), `status`
PENDING|VERIFIED, `strategy` TOTP, unique `(userWorkspaceId, strategy)`. Provisioning URI =
`keyuri(userEmail, 'Twenty' | 'Twenty - <workspaceDisplayName>')`. A pending enrollment may be
reused within 1 h (`PENDING_METHOD_REUSE_WINDOW_MS`). **Replay protection**: used OTPs are cached
under `two-factor-authentication-used-otp:<methodId>:<token>` for
`(2*window+2)*step*1000` ms = **2 min** (`constants/two-factor-authentication-used-otp.constant.ts`).
**Backup codes**: 10 codes (`TWO_FACTOR_AUTHENTICATION_BACKUP_CODE_COUNT`), each 20 chars from
`0123456789ABCDEFGHJKMNPQRSTVWXYZ` grouped in 5s with `-`, stored as
`sha256(code.toUpperCase().replace(/[^0-9A-Z]/g,''))` hex
(`utils/hash-two-factor-authentication-recovery-code.util.ts`), `expiresAt = 9999-12-31`.
**Recovery code** (admin-issued for another member): same alphabet/format and hash, stored in
`core.appToken` `type=TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE`, `expiresIn =
TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE_EXPIRES_IN` (1h), one pending per (user, workspace).
Rate limits: OTP 10/15 min per (user, workspace); recovery-code redemption 5/15 min per
userWorkspace; recovery-code issuance 10/1 h per actor
(`two-factor-authentication/constants/*.constant.ts`). Step-up auth reuses the OTP path.

### 1.13 API keys

`srv/src/engine/core-modules/api-key/` — `core.apiKey`: `id uuid`, `name`, `expiresAt timestamptz`,
`revokedAt?`, `workspaceId`. A key has **exactly one role**, stored via `core.roleTarget` with
`targetMetadataForeignKey='apiKeyId'` (`api-key/services/api-key-role.service.ts`); the role must
have `canBeAssignedToApiKeys = true`. Token minting (`api-key/services/api-key.service.ts:161-196`):
`signAsyncOrThrow({ sub: workspaceId, type: API_KEY, workspaceId }, { expiresIn, jwtid: apiKeyId })`
— `expiresIn` = `'100y'` when there is no expiry, else `(expiresAt-now)/1000` seconds.
Validation (`auth/strategies/jwt.auth.strategy.ts:74-114`): the workspace comes from `payload.sub`,
the key is looked up by `payload.jti` in the workspace cache `apiKeyMap`; revoked → 403 "This API
Key is revoked"; expired → 403 "This API Key is expired". Legacy payloads (no `type`, no
`workspaceId`) are also accepted (`isLegacyApiKeyPayload`).

### 1.14 Token/auth context dispatch

`auth/strategies/jwt.auth.strategy.ts` `dispatch()`: `API_KEY` (+legacy) → api-key context;
`WORKSPACE_AGNOSTIC` → user-only context; `ACCESS`/`PLAYGROUND` → user + workspace +
workspaceMember context (impersonation validated when `isImpersonating === true` and
`type===ACCESS`); `APPLICATION_ACCESS` → application context (plus user when `userId` and
`userWorkspaceId` are present); anything else → `INVALID_JWT_TOKEN_TYPE`.

### 1.15 Server-admin identity assertion (Enterprise)

`srv/src/engine/core-modules/auth/identity-assertion/` — an identity-aware proxy injects a signed
JWT into `SERVER_ADMIN_IDENTITY_ASSERTION_HEADER`; the server verifies it against
`SERVER_ADMIN_IDENTITY_ASSERTION_JWKS_URL` with issuer `SERVER_ADMIN_IDENTITY_ASSERTION_ISSUER`
and audience `SERVER_ADMIN_IDENTITY_ASSERTION_AUDIENCE` (algorithms in
`constants/identity-assertion-algorithms.constant.ts`; JWKS cache constants
`IDENTITY_ASSERTION_SIGNING_KEYS_MAX_AGE_MS` / `..._MIN_REFETCH_INTERVAL_MS` /
`..._FETCH_TIMEOUT_MS`).

---

## 2. AUTHORIZATION

### 2.1 Roles

`core.role` (`srv/src/engine/metadata-modules/role/role.entity.ts`): `label`, `description`,
`icon`, `isEditable`, `isSystemSideEffect`, `canUpdateAllSettings`, `canAccessAllTools`,
`canAccessAllAgentChatChannels`, `canReadAllObjectRecords`, `canCreateAllObjectRecords`,
`canUpdateAllObjectRecords`, `canSoftDeleteAllObjectRecords`, `canDestroyAllObjectRecords`,
`canBeAssignedToUsers`, `canBeAssignedToAgents`, `canBeAssignedToApiKeys`,
`canBeAssignedToApplications`; unique `(label, workspaceId)`.

- **Admin** — universal id `20202020-02c2-43f2-b94d-cab1f2b532eb`
  (`srv/src/engine/workspace-manager/twenty-standard-application/constants/standard-role.constant.ts`),
  built in `.../utils/role-metadata/create-standard-flat-role-metadata.util.ts`:
  `canUpdateAllSettings=true, canAccessAllTools=true, canAccessAllAgentChatChannels=true,
  canReadAllObjectRecords=true, canCreateAllObjectRecords=true, canUpdateAllObjectRecords=true,
  canSoftDeleteAllObjectRecords=true, canDestroyAllObjectRecords=true,
  canBeAssignedToUsers=true, canBeAssignedToAgents=false, canBeAssignedToApiKeys=true, isEditable=false`.
- **Member** — universal id `20202020-a8a6-47ae-be79-e3601fa46a8e`
  (`packages/twenty-shared/src/constants/MemberRoleUniversalIdentifier.ts`), created by
  `RoleService.createMemberRole` (`metadata-modules/role/role.service.ts:494-521`):
  `canUpdateAllSettings=false, canAccessAllTools=true, canAccessAllAgentChatChannels=true,
  canReadAllObjectRecords=true, canUpdateAllObjectRecords=true, canSoftDeleteAllObjectRecords=true,
  canDestroyAllObjectRecords=true, canBeAssignedToUsers=true, canBeAssignedToAgents=false,
  canBeAssignedToApiKeys=false` (create not granted); becomes `workspace.defaultRoleId`.
- **Guest** — `RoleService.createGuestRole` (`role.service.ts:588+`): read-only records.
- **Workspace custom application role** — `RoleService.createWorkspaceCustomApplicationRole`
  (`role.service.ts:525+`), `canUpdateAllSettings=false, canAccessAllTools=false`.

**Role targets** — `core.roleTarget`
(`srv/src/engine/metadata-modules/role-target/role-target.entity.ts`): exactly one of
`userWorkspaceId | agentId | apiKeyId | webhookId | targetApplicationId` (CHECK
`CHK_role_target_single_entity`) plus `roleId`, `isSystemSideEffect`; unique
`(workspaceId, userWorkspaceId)` when `isSystemSideEffect = false`.

### 2.2 Permission flags (settings-level)

`packages/twenty-shared/src/constants/PermissionFlagType.ts:1-29`:
`API_KEYS_AND_WEBHOOKS, WORKSPACE, WORKSPACE_MEMBERS, ROLES, DATA_MODEL, SECURITY, WORKFLOWS,
IMPERSONATE, SSO_BYPASS, APPLICATIONS, MARKETPLACE_APPS, LAYOUTS, BILLING, AI_SETTINGS, AI,
VIEWS, UPLOAD_FILE, DOWNLOAD_FILE, SEND_EMAIL_TOOL, CREATE_CALENDAR_EVENT_TOOL, HTTP_REQUEST_TOOL,
CODE_INTERPRETER_TOOL, IMPORT_CSV, EXPORT_CSV, CONNECTED_ACCOUNTS, PROFILE_INFORMATION`.
Table `core.permissionFlag` (`key`, `label`, `description`, `icon`, `permissionType`), unique
`(key, workspaceId)`; link table `core.rolePermissionFlag` (`roleId`, `permissionFlagId`),
unique pair. **Grant rule**
(`metadata-modules/flat-role/utils/is-permission-flag-granted-to-flat-role.util.ts`): a flag is
granted if the role has **role-wide access** (`hasRoleWideAccessToPermissionFlag`, e.g.
`canUpdateAllSettings` ⇒ all settings flags) **or** an explicit `rolePermissionFlag` row.
**Check** (`srv/src/engine/permissions/setting-permissions.service.ts` `hasPermissionFlag`):
a system auth context ⇒ always `true`; otherwise the main principal's roles must grant the flag
**and** every limiting role (agent) must also grant it.

### 2.3 Object-level, field-level and row-level permissions

- **Object permissions** — `core.objectPermission`
  (`srv/src/engine/metadata-modules/object-permission/object-permission.entity.ts`), unique
  `(objectMetadataId, roleId)`: `canReadObjectRecords`, `canCreateObjectRecords`,
  `canUpdateObjectRecords`, `canSoftDeleteObjectRecords`, `canDestroyObjectRecords` (nullable).
- **Field permissions** — `core.fieldPermission`
  (`.../object-permission/field-permission/field-permission.entity.ts`), unique
  `(fieldMetadataId, roleId)` when `rowLevelPermissionPredicateGroupId IS NULL` and
  `(fieldMetadataId, roleId, rowLevelPermissionPredicateGroupId)` otherwise:
  `canReadFieldValue`, `canUpdateFieldValue`; optional link to an RLS rule / predicate group.
- **Row-level rules (RLS)** — `core.rowLevelPermissionRule`
  (`srv/src/engine/metadata-modules/row-level-permission-predicate/entities/row-level-permission-rule.entity.ts`):
  `(roleId, objectMetadataId)` + `canReadRecords, canCreateRecords, canUpdateRecords,
  canSoftDeleteRecords, canDestroyRecords`.
- **Predicate groups** — `core.rowLevelPermissionPredicateGroup`: nested groups
  (`parentRowLevelPermissionPredicateGroupId`), `logicalOperator` AND|OR,
  `positionInRowLevelPermissionPredicateGroup`, per-operation grant flags, link to a rule.
- **Predicates** — `core.rowLevelPermissionPredicate`: `fieldMetadataId`,
  `relationFieldMetadataId?`, `operand` (CONTAINS default; `RowLevelPermissionPredicateOperand`),
  `value jsonb`, `subFieldName?`, `workspaceMemberFieldMetadataId?` +
  `workspaceMemberSubFieldName?` (compare against the acting workspace member), group + rule
  links, position.

### 2.4 Multi-role combination semantics

`srv/src/engine/permissions/internal/role/utils/` — **Union** across the principal's own roles
(`compute-permission-union.util.ts`): an "all records" flag is granted if **some** role grants
it; unreadable/uneditable field lists are **unioned**; row-level rules are concatenated.
**Intersection** with limiting roles (`compute-permission-intersection.util.ts`): "all records"
requires **every** limiting role to grant it; field lists are unioned; each rule's per-op grants
are ANDed with "other roles reach all records". A **runtime limiting role**
(`restrict-records-permissions-to-runtime-limiting-role.util.ts`) narrows further (AI runs).
**Create follows update** when the feature flag `IS_CREATE_RECORDS_PERMISSION_ENABLED` is off
(`get-enforced-operation-type.util.ts`, `follow-update-when-create-permission-is-disabled.util.ts`).
A role missing from the cache grants nothing (`setting-permissions.service.ts`).

### 2.5 Principals

`srv/src/engine/permissions/internal/principal/types/permissions-principal.type.ts`: main principal
∈ `workspaceMember | apiKey | agent | webhook | application | role`; limiting principal = `agent`;
application principal = `application | customApplication`. `canAuthContextHoldRoles` = everything
except `system` and `pendingActivationUser`
(`internal/principal/utils/can-auth-context-hold-roles.util.ts`). Role resolution per principal
type: `internal/principal/utils/resolve-permissions-principals-with-roles.util.ts`.

### 2.6 Enforcement in the query runner

`srv/src/engine/twenty-orm/repository/workspace-repository.ts` (the ORM's WorkspaceRepository):

| Hook | Where |
|---|---|
| `shouldBypassPermissionChecks` option (system / internal writes) | `:133-134`; checked at `:168`, `:1040`, `:1489`, `:1564`, `:1768`, `:1789`, `:1892`, `:1954`, `:1998` |
| `validateWriteIsPermitted` → `recordPermissions.assertCanRunOperationOrThrow` | `:1760-1783` |
| `validateRLSPredicatesForUpdatedRecords` → `assertUpdatesKeepRecordAccessOrThrow` | `:1785-1806` |
| `validateQueryIsPermitted` → per-alias `assertCanRunOperationOrThrow({operationType:'read'})` | `:1948-1992` |
| `applyRowLevelPermissionPredicates` → SQL conditions per alias (main + joins) | `:1994-2075` |
| `nullifyRlsRuleUnreadableFields` → `recordFieldPermissions.buildUnreadableFieldsSQLConditions` (NULLs the columns instead of filtering the row) | `:1888-1947` |
| created records must match RLS → `assertCreatedRecordsMatchRowLevelPermissionsOrThrow` | `:1040-1047` |

Field-level enforcement on writes:
`srv/src/engine/permissions/internal/role/utils/validate-operation-is-permitted-or-throw.util.ts` —
for `read`/`destroy`/`softDelete` it checks `unreadableFields` against the **selected columns**;
for `create`/`update` it also checks `uneditableFields` against the **updated columns** (columns
that are RLS-predicate fields are exempt on create).

### 2.7 Endpoint-level access policy

`srv/src/engine/guards/access-policy.guard.ts` + `guards/access-policy/access-policy.service.ts`.
`AccessPolicy = { authPrincipals, requiredPermissionFlags, requiredFeatureFlags? }`
(`guards/types/access-policy.type.ts`). `authPrincipals`
(`guards/types/auth-principal-guard-config.type.ts`):
`{ userSession: boolean | {standard, impersonated, playground, workspaceAgnostic},
apiKey: boolean, oauthClient: boolean | {withUser, withoutUser},
application: boolean | {withUser, withoutUser, withAgent?} }`.
Principal classification (`guards/utils/classify-auth-principal.util.ts`) maps the token type +
request contents to a variant; `is-auth-principal-variant-accepted.util.ts` compares it with the
policy. Other guards: `PublicEndpointGuard` and `NoPermissionGuard` (both always true — explicit
markers), `WorkspaceNotSuspendedGuard` (throws `WORKSPACE_SUSPENDED` unless
`@AllowSuspendedWorkspace()`), `JwtAuthGuard` (REST), `ImpersonatePermissionGuard`,
`CustomPermissionGuard`, `AdminPanelGuard`, `BillingEnabled/DisabledGuard`.

### 2.8 Request pipeline

`srv/src/engine/main.ts` + `srv/src/engine/app.module.ts`:
1. `express-session` is installed (`main.ts:77`) with `getSessionStorageOptions(...)`
   (`core-modules/session-storage/session-storage.module-factory.ts`) — used by the OIDC/SAML
   passport strategies (`sessionKey`).
2. `CookieSessionCsrfMiddleware` runs **before** any middleware that authenticates from the cookie.
3. `RestCoreMiddleware` (`hydrateRestRequest`) / `GraphQLHydrateRequestFromTokenMiddleware`
   (`hydrateGraphqlRequest`) call `AccessTokenService.validateTokenByRequest`
   (`auth/token/services/access-token.service.ts:186-232`): the Bearer token first (a `sess_`
   token as Bearer is rejected), else the session cookie; a missing token → `FORBIDDEN_EXCEPTION`.
   The result is bound onto the request (`user`, `workspace`, `apiKey`, `application`, `agentId`,
   `workspaceMember`, `userWorkspaceId`, `tokenType`, `impersonationContext`). GraphQL also
   refuses `agentId` tokens ("This token is only accepted by MCP").

---

## 3. WORKSPACE LIFECYCLE

### 3.1 Activation status and the workspace row

`packages/twenty-shared/src/workspace/types/WorkspaceActivationStatus.ts`:
`ONGOING_CREATION, PENDING_CREATION, CREATED, ACTIVE, INACTIVE, SUSPENDED`; provisioned statuses
`CREATED, ACTIVE, SUSPENDED`
(`packages/twenty-shared/src/workspace/constants/ProvisionedWorkspaceActivationStatuses.ts`);
suspended = `SUSPENDED | INACTIVE` and not soft-deleted
(`core-modules/workspace/utils/is-workspace-suspended.util.ts`).
`core.workspace` (`core-modules/workspace/workspace.entity.ts`): `id`, `displayName`, `subdomain`
(unique), `customDomain?` (unique), `inviteHash?`, `databaseSchema?`, `defaultRoleId?`,
`activationStatus` (enum, default `INACTIVE`), `suspendedAt?`, `deletedAt?` (soft delete),
`allowImpersonation` (true), `isPublicInviteLinkEnabled` (true), `isGoogleAuthEnabled` (true),
`isMicrosoftAuthEnabled` (true), `isPasswordAuthEnabled` (true),
`isPasswordAuthBypassEnabled`/`isGoogleAuthBypassEnabled`/`isMicrosoftAuthBypassEnabled` (false),
`isTwoFactorAuthenticationEnforced` (false), `isCustomDomainEnabled`, `trashRetentionDays` (14),
`eventLogRetentionDays`, `workspaceDiscoverability` (PUBLIC default), `editableProfileFields`
(`{email,profilePicture,firstName,lastName}`). DB CHECKs: `onboarded_workspace_requires_default_role`
and `workspace_requires_database_schema` — outside `PENDING_CREATION`/`ONGOING_CREATION` a
workspace must have a `defaultRoleId` and a non-empty `databaseSchema`.

### 3.2 Creation

`SignInUpService.signUpOnNewWorkspace` (`auth/services/sign-in-up.service.ts:~620-820`), in one
DB transaction: (1) `assertWorkspaceCreationAllowed` — sign-up enabled
(`IS_MULTIWORKSPACE_ENABLED` or workspaceCount === 0); workspace count
`< MAX_WORKSPACES_WITHOUT_ENTERPRISE_KEY` (**5**,
`auth/constants/max-workspaces-without-organization-key.constant.ts`) unless the enterprise plan is
valid; if `IS_WORKSPACE_CREATION_LIMITED_TO_SERVER_ADMINS` (default **true**) only a
`canAccessFullAdminPanel` user may create a workspace; (2) `displayName` required, `subdomain`
validated via `SubdomainManagerService` (auto-generated from the work-email domain otherwise);
(3) insert `core.workspace` with `activationStatus=PENDING_CREATION`, `inviteHash=v4()`,
`id=v4()`, `workspaceCustomApplicationId=v4()`; (4) create the workspace custom application;
(5) create the user (if new) with `canImpersonate`/`canAccessFullAdminPanel` = "there is no server
admin yet" (the first user becomes the server admin); (6) create `core.userWorkspace`; (7) set the
onboarding flags; (8) `billingService.ensureBillingCustomer` when billing is enabled; (9) optional
logo inferred from the work-email domain. `signUpWithoutWorkspace` creates only the user — allowed
when `getSignUpWithoutWorkspaceDecision` returns `allowed` or `requiresDestination` with a
provisioned destination (invitation / available workspace / approved access domain)
(`auth/utils/get-sign-up-without-workspace-decision.util.ts`,
`auth/utils/has-provisioned-sign-up-destination.util.ts`).

### 3.3 Activation

`WorkspaceService.activateWorkspace` (`core-modules/workspace/services/workspace.service.ts:687-790`):
compare-and-set `PENDING_CREATION → ONGOING_CREATION`; an `ONGOING_CREATION` older than **5 min**
(`WORKSPACE_ACTIVATION_STALE_LOCK_TIMEOUT_MS`) is reclaimed; already `ACTIVE`/`CREATED` ⇒ return
the workspace (idempotent retry), else throw "Workspace is already being created". Then
`WorkspaceManagerService.init` (`srv/src/engine/workspace-manager/workspace-manager.service.ts`):
create the workspace DB schema and store it in `workspace.databaseSchema`;
`ApplicationService.createTwentyStandardApplication` +
`synchronizeTwentyStandardApplicationOrThrow` (standard objects/fields/page layouts/roles/flags);
`RoleService.createWorkspaceCustomApplicationRole`; `setupDefaultRoles` (assign the **Admin** role
to the creating user, ensure the **Member** role exists, set `workspace.defaultRoleId`). Then
`FeatureFlagService.enableFeatureFlags(DEFAULT_FEATURE_FLAGS, workspaceId)`;
`UserWorkspaceService.createWorkspaceMember`; `prefillCreatedWorkspaceRecords` (companies, people,
workflows, opportunities, dashboards, workflow command menu items —
`workspace-manager/standard-objects-prefill-data/utils/prefill-*.util.ts`);
`activateAndInitializeUpgradeState` (sets `activationStatus = ACTIVE` when the workspace has a
subscription, else `CREATED`, and marks the initial migration cursor); enqueue pre-installed apps
and SDK client generation. On failure the status rolls back to `PENDING_CREATION`.

### 3.4 Suspension / reactivation / deletion

`core-modules/workspace/services/workspace.service.ts` — `suspendWorkspace(id)`:
`activationStatus=SUSPENDED`, `suspendedAt=now()`, emits `workspace.suspended`.
`reactivateWorkspace(id)`: from `SUSPENDED`/`CREATED` to `ACTIVE`, `suspendedAt=null`, emits
`workspace.reactivated`. `deleteWorkspace(id, softDelete=false)`: **soft delete** cancels the
Stripe subscription (if billing), soft-deletes every `userWorkspace` and the user if it was their
last workspace, `workspaceRepository.softDelete({id, deletedAt: IsNull()})`, emits
`workspace.softDeleted`, enqueues the application uninstall; **hard delete** runs under a Postgres
advisory lock (`getWorkspaceApplicationUninstallLockName(workspace.id)`): delete the user
workspaces/users, assert the subscription is cancelled, run the pending application-uninstall
hooks, delete the runtime resources and the syncable metadata entities, **drop the workspace DB
schema**, flush the workspace cache, enqueue the file-folder and emailing-domain cleanup jobs,
delete the custom DNS hostname, then `workspaceRepository.delete(id)`.
Guards: `assertNotRemovingLastServerAdminOrThrow` /
`assertWorkspaceDeletionKeepsServerAdminOrThrow`. Inactivity thresholds:
`WORKSPACE_INACTIVE_DAYS_BEFORE_NOTIFICATION=7`, `..._SOFT_DELETION=14`, `..._DELETION=21`,
`MAX_NUMBER_OF_WORKSPACES_DELETED_PER_EXECUTION=5`.

### 3.5 Workspace member, picker, member creation

- `core.userWorkspace` (`core-modules/user-workspace/user-workspace.entity.ts`): `id uuid`,
  `userId`, `workspaceId`, `defaultAvatarUrl?`, `locale` (default `en`), `deletedAt?`, unique
  `(userId, workspaceId)` where `deletedAt IS NULL`; cascades to the 2FA methods. The
  `WorkspaceMember` standard object is the workspace-scoped mirror of a user; its id is what access
  tokens carry in `workspaceMemberId`.
- **Picker** — `UserWorkspaceService.findAvailableWorkspacesByEmail(email)` (`:381-451`) returns
  `availableWorkspacesForSignIn` (memberships, excluding `workspaceDiscoverability = HIDDEN`; enum
  `PUBLIC | MEMBERS_AND_INVITEES | HIDDEN`,
  `core-modules/workspace/types/workspace-discoverability.type.ts`) and
  `availableWorkspacesForSignUp` (approved-access-domain matches — only when
  `IS_EMAIL_VERIFICATION_REQUIRED` — plus invitations).
- **Adding a member** — `addUserToWorkspaceIfUserNotInWorkspace` (`:168-235`): role = invitation
  `roleId` else `workspace.defaultRoleId` (must be assignable to users) →
  `billingSeatService.addMemberWithinSeatLimit` creates `core.userWorkspace` + the
  `workspaceMember` row (`name{firstName,lastName}`, `colorScheme='System'`, `uiScale='Default'`,
  `openRecordIn=SIDE_PANEL`, `userId`, `userEmail`, `avatarUrl`, `locale`) →
  `roleAssignmentWriteService.assignRoleToUserWorkspaces` writes `core.roleTarget` → the
  invitation is invalidated → the onboarding "create profile" flag is set.

### 3.6 Feature flags

`core.featureFlag` (`core-modules/feature-flag/feature-flag.entity.ts`): `key text`,
`value boolean`, unique `(key, workspaceId)`. Enum `FeatureFlagKey`
(`packages/twenty-shared/src/types/FeatureFlagKey.ts:1-25`):
`IS_AI_CHAT_COMPONENT_BUILDER_ENABLED, IS_CONFIGURABLE_SEARCH_FIELDS_ENABLED,
IS_REST_METADATA_API_NEW_FORMAT_DIRECT, IS_RECORD_LEVEL_SHARING_ENABLED,
IS_CALENDAR_EVENT_SHARED_ACROSS_CHANNELS_ENABLED, IS_AI_CHAT_INBOX_ENABLED,
IS_AGENT_CHAT_DIRECT_MESSAGE_IDENTITY_ENABLED, IS_MCP_SERVERS_ENABLED,
IS_RELATED_RECORD_PATHS_ENABLED, IS_ROW_LEVEL_PERMISSION_RELATION_HOP_ENABLED,
IS_GETTING_STARTED_ENABLED, IS_MESSAGE_ATTACHMENT_ENABLED, IS_AI_DATA_POLICY_ENABLED,
IS_CREATE_RECORDS_PERMISSION_ENABLED, IS_ACCESS_GROUPS_ENABLED, IS_PRIVATE_RECORDS_ENABLED,
IS_PERMISSION_TABLES_ENABLED, IS_RECORD_GROUP_TABS_ENABLED,
IS_CALENDAR_INTERNAL_EVENTS_FILTER_ENABLED, IS_LOOKUP_COLUMNS_ENABLED,
IS_INTERNAL_IMPORT_COLUMN_ENABLED, IS_CALENDAR_RECURRING_EVENT_HORIZON_ENABLED,
IS_CUSTOMIZABLE_SIDEBAR_ENABLED`. Enabled at activation:
`IS_REST_METADATA_API_NEW_FORMAT_DIRECT, IS_AGENT_CHAT_DIRECT_MESSAGE_IDENTITY_ENABLED,
IS_MESSAGE_ATTACHMENT_ENABLED, IS_GETTING_STARTED_ENABLED, IS_INTERNAL_IMPORT_COLUMN_ENABLED`
(`workspace-manager/workspace-migration/constant/default-feature-flags.ts`).

### 3.7 Dev seeds

`srv/src/engine/workspace-manager/dev-seeder/` (dev-seeder commands / E2E only):
- **Workspaces** (`dev-seeder/core/constants/seeder-workspaces.constant.ts`):
  `SEED_APPLE_WORKSPACE_ID = 20202020-1c25-4d02-bf25-6aeccf7ea419` (overridable with
  `E2E_SEED_WORKSPACE_ID`), `SEED_YCOMBINATOR_WORKSPACE_ID = 3b8e6458-5fc1-4e63-8563-008ccddaa6db`,
  `SEED_EMPTY_WORKSPACE_3_ID`, `SEED_EMPTY_WORKSPACE_4_ID`; seeded with
  `activationStatus=PENDING_CREATION`, `inviteHash='apple.dev-invite-hash'` / `'yc.dev-invite-hash'`.
- **Users** (`dev-seeder/core/utils/seed-users.util.ts`): tim@apple.dev, jony.ive@apple.dev,
  phil.schiler@apple.dev, jane.austen@apple.dev, scott.forstall@apple.dev — all with
  **passwordHash `$2b$10$3LwXjJRtLsfx4hLuuXhxt.3mWgismTiZFCZSG3z9kDrSfsrBl0fT6`** (bcrypt cost 10)
  and `isEmailVerified: true`; the first four also have `canImpersonate=true,
  canAccessFullAdminPanel=true`. Fixed ids `USER_DATA_SEED_IDS` = `20202020-e6b5-4680-8a32-b8209737156b`
  (JANE), `20202020-9e3b-46d4-a556-88b9ddc2b034` (TIM), `20202020-3957-4908-9c36-2929a23f8357` (JONY),
  `20202020-7169-42cf-bc47-1cfef15264b8` (PHIL), `20202020-1111-4a01-8001-000000000001` (SCOTT).
- **UserWorkspaces** (`dev-seeder/core/utils/seed-user-workspaces.util.ts`, `USER_WORKSPACE_DATA_SEED_IDS`).
- **Permissions** (`dev-seeder/core/services/dev-seeder-permissions.service.ts`): Admin role → Tim
  in "light" mode / Jane in full mode; Member role → the others; plus a **Guest** role (Phil), an
  **Object-restricted** role (Tim — no read on Rockets, no update on Pets) and an
  **Impersonate-only** role (Scott — only `PermissionFlagType.IMPERSONATE`);
  `initMinimalPermissionsAndActivateWorkspace` sets `defaultRoleId` and `activationStatus = ACTIVE`.
- Other helpers: `seed-api-keys.util.ts`, `seed-feature-flags.util.ts`,
  `seed-two-factor-authentication-methods.util.ts`, `seed-server-id.util.ts`,
  `seed-metadata-entities.util.ts`, `seed-page-layouts.util.ts`, `seed-emailing-domains.util.ts`,
  `seed-unsubscribe-topics.util.ts`, `seed-agents.util.ts`, `seed-workspace.util.ts`; data seeds
  in `dev-seeder/data/constants/*-data-seeds.constant.ts`.

---

## 4. CONFIG (environment variables)

Source: `srv/src/engine/core-modules/twenty-config/config-variables.ts` (2886 lines) — a class
whose properties are filled from `process.env`
(`twenty-config/drivers/environment-config.driver.ts`). A property **without** a default is
required; with a default it is optional. `isEnvOnly: true` ⇒ env only (not the admin panel/DB);
`isSensitive: true` ⇒ masked in the admin panel. Config may also live in the DB when
`IS_CONFIG_VARIABLES_IN_DB_ENABLED=true` (default). Groups
(`twenty-config/enums/config-variables-group.enum.ts`): `SERVER_CONFIG, RATE_LIMITING,
STORAGE_CONFIG, GOOGLE_AUTH, MICROSOFT_AUTH, EMAIL_SETTINGS, LOGGING, ADVANCED_SETTINGS,
BILLING_CONFIG, CAPTCHA_CONFIG, CLOUDFLARE_CONFIG, LLM, LOGIC_FUNCTION_CONFIG,
CODE_INTERPRETER_CONFIG, SSL, SUPPORT_CHAT_CONFIG, ANALYTICS_CONFIG, TOKENS_DURATION,
AWS_SES_SETTINGS, RESEND_SETTINGS`.

### 4.1 Auth / tokens / cookies

`AUTH_PASSWORD_ENABLED=true`, `SIGN_IN_PREFILLED=false`, `IS_EMAIL_VERIFICATION_REQUIRED=false`, `ACCESS_TOKEN_EXPIRES_IN=30m`, `WORKSPACE_AGNOSTIC_TOKEN_EXPIRES_IN=30m`, `REFRESH_TOKEN_EXPIRES_IN=60d`, `REFRESH_TOKEN_REUSE_GRACE_PERIOD=1m`, `SESSION_ABSOLUTE_LIFETIME=180d`, `SESSION_IDLE_TIMEOUT=30d`, `WORKSPACE_AUTO_LOGIN_WINDOW=10m` (`DEFAULT_WORKSPACE_AUTO_LOGIN_WINDOW`), `AUTH_COOKIE_SAME_SITE=lax` (`lax|strict|none`), `AUTH_COOKIE_ALLOWED_ORIGINS=''`, `LOGIN_TOKEN_EXPIRES_IN=15m`, `FILE_TOKEN_EXPIRES_IN=1d`, `INVITATION_TOKEN_EXPIRES_IN=30d`, `SHORT_TERM_TOKEN_EXPIRES_IN=5m`, `APPLICATION_ACCESS_TOKEN_EXPIRES_IN=30m`, `APPLICATION_REFRESH_TOKEN_EXPIRES_IN=60d`, `PLAYGROUND_TOKEN_EXPIRES_IN=2h`, `EMAIL_VERIFICATION_TOKEN_EXPIRES_IN=1h`, `PASSWORD_RESET_TOKEN_EXPIRES_IN=5m`, `TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE_EXPIRES_IN=1h` (the token durations are in group `TOKENS_DURATION`); `AUTH_GOOGLE_ENABLED=false`, `AUTH_GOOGLE_CLIENT_ID`, `AUTH_GOOGLE_CLIENT_SECRET`, `AUTH_GOOGLE_CALLBACK_URL` (**required when `AUTH_GOOGLE_ENABLED`**), `AUTH_GOOGLE_APIS_CALLBACK_URL` (required); `AUTH_MICROSOFT_ENABLED=false`, `AUTH_MICROSOFT_CLIENT_ID`, `AUTH_MICROSOFT_CLIENT_SECRET`, `AUTH_MICROSOFT_CALLBACK_URL`, `AUTH_MICROSOFT_APIS_CALLBACK_URL` (**required when `AUTH_MICROSOFT_ENABLED`**); `APP_SECRET` (**required**, sensitive, env-only), `ENCRYPTION_KEY` (optional, falls back to `APP_SECRET`), `FALLBACK_ENCRYPTION_KEY` (optional), `SIGNING_KEY_ROTATION_DAYS` (unset ⇒ no auto-rotation), `IS_WORKSPACE_CREATION_LIMITED_TO_SERVER_ADMINS=true`, `IS_MULTIWORKSPACE_ENABLED=false`, `SERVER_ADMIN_WORKSPACE_ID` (unset, uuid, env-only), `SERVER_ADMIN_IDENTITY_ASSERTION_HEADER` / `_JWKS_URL` / `_ISSUER` / `_AUDIENCE` (unset, env-only).

### 4.2 Server / DB / Redis / queues

`NODE_ENV=production`, `NODE_PORT=3000`, `SERVER_URL=http://localhost:3000`, `SERVER_INTERNAL_URL` (unset), `SERVER_KEEP_ALIVE_TIMEOUT_MS=65000`, `AI_STREAM_SHUTDOWN_DRAIN_MS=300000`, `TRUST_PROXY=loopback, linklocal, uniquelocal`, `SERVER_ID` (required, uuid v4 seeded in the DB), `PUBLIC_DOMAIN_URL` (required), `FRONTEND_URL` (required), `FRONTEND_INDEX_URL` (unset, https only), `DEFAULT_SUBDOMAIN=app`, `ADDITIONAL_RESERVED_SUBDOMAINS=[]`, `ENTERPRISE_INSTANCE_TYPE=PRODUCTION`, `IS_CONFIG_VARIABLES_IN_DB_ENABLED=true`, `LOGIC_FUNCTION_LEGACY_ROUTE_CUTOFF`, `DPA_DEPLOYMENT_REGION=EU` (env-only); `PG_DATABASE_URL` (required), `PG_DATABASE_REPLICA_URL` (optional), `PG_SSL_ALLOW_SELF_SIGNED=false`, `PG_POOL_MAX_CONNECTIONS=10`, `PG_POOL_IDLE_TIMEOUT_MS=600000`, `PG_POOL_ALLOW_EXIT_ON_IDLE=true`, `PG_DATABASE_PRIMARY_TIMEOUT_MS=10000`, `PG_DATABASE_REPLICA_TIMEOUT_MS=10000`, `SEARCH_ILIKE_FALLBACK_TIMEOUT_MS=2000`; `REDIS_URL` (required), `REDIS_CACHE_PREFIX=''`, `REDIS_QUEUE_URL` (optional), `REDIS_QUEUE_PREFIX=bull`, `CACHE_STORAGE_TTL=3600*24*7`, `WORKER_ENABLED_QUEUES=[]`, `WORKER_EXCLUDED_QUEUES=[]`, `IS_REDIS_MEMORY_FAIRNESS_ENABLED=false`, `IS_WORKER_GROUP_LIMITER_ENABLED=false`, `JOB_QUEUE_MAX_JOB_SIZE_BYTES=5242880`, `JOB_QUEUE_MEMORY_SHARE_BY_WORKSPACE_ID=0.3`, `JOB_QUEUE_MEMORY_SHARE_BY_APPLICATION_UNIVERSAL_IDENTIFIER=0.4`, `JOB_QUEUE_MEMORY_RESERVE=0.1`.

### 4.3 Rate limiting / quotas (see §5)

`MUTATION_MAXIMUM_AFFECTED_RECORDS=100`, `API_RATE_LIMITING_SHORT_TTL_IN_MS=1000`,
`API_RATE_LIMITING_SHORT_LIMIT=100`, `API_RATE_LIMITING_LONG_TTL_IN_MS=60000`,
`API_RATE_LIMITING_LONG_LIMIT=100`, `APPLICATION_API_RATE_LIMITING_TTL_IN_MS=60000`,
`APPLICATION_API_RATE_LIMITING_LIMIT=500`, `EMAIL_SEND_RATE_LIMITING_TTL_IN_MS=10000`, `EMAIL_SEND_RATE_LIMITING_LIMIT=100`, `EMAIL_SEND_WORKSPACE_DAILY_LIMIT=1000`, `EMAIL_SEND_WORKSPACE_RATE_LIMITING_TTL_IN_MS=10000`, `EMAIL_SEND_WORKSPACE_RATE_LIMITING_LIMIT=100`, `WEBHOOK_CALL_RATE_LIMITING_TTL_IN_MS=60000`, `WEBHOOK_CALL_RATE_LIMITING_LIMIT=60`, `APPLICATION_JOB_ENQUEUE_RATE_LIMITING_TTL_IN_MS=60000`, `APPLICATION_JOB_ENQUEUE_RATE_LIMITING_LIMIT=500`, `APPLICATION_REGISTRATION_JOB_ENQUEUE_RATE_LIMITING_LIMIT=2000`, `INVITATION_SENDING_BY_WORKSPACE_THROTTLE_TTL_IN_MS=604800000`, `INVITATION_SENDING_BY_WORKSPACE_THROTTLE_LIMIT=500`, `INVITATION_SENDING_BY_EMAIL_THROTTLE_TTL_IN_MS=604800000`, `INVITATION_SENDING_BY_EMAIL_THROTTLE_LIMIT=10`, `WORKFLOW_EXEC_SOFT_THROTTLE_LIMIT=100`, `WORKFLOW_EXEC_SOFT_THROTTLE_TTL=60000`, `WORKFLOW_EXEC_HARD_THROTTLE_LIMIT=5000`, `WORKFLOW_EXEC_HARD_THROTTLE_TTL=3600000`, `GRAPHQL_MAX_FIELDS=2000`, `GRAPHQL_MAX_ROOT_RESOLVERS=20`, `COMMON_QUERY_COMPLEXITY_LIMIT=2000`.

### 4.4 Billing (see §5)

`IS_BILLING_ENABLED=false`, `BILLING_PLAN_REQUIRED_LINK`, `BILLING_PLANS={}`, `BILLING_FREE_TRIAL_WITH_CREDIT_CARD_DURATION_IN_DAYS=30`, `BILLING_FREE_TRIAL_WITHOUT_CREDIT_CARD_DURATION_IN_DAYS=7`, `BILLING_TRIAL_LIMIT_EXEMPT_EMAIL_DOMAINS=[]`, `BILLING_FREE_WORKFLOW_CREDITS_FOR_TRIAL_PERIOD_WITHOUT_CREDIT_CARD=500000`, `BILLING_FREE_WORKFLOW_CREDITS_FOR_TRIAL_PERIOD_WITH_CREDIT_CARD=2500000`, `AI_USER_CREDITS_LIMIT` (unset), `USAGE_LIMIT_DEFAULTS_BY_PLAN={}`, `BILLING_TRIAL_WITHOUT_CREDIT_CARD_REMINDER_DAYS_BEFORE=1`, `BILLING_TRIAL_WITH_CREDIT_CARD_REMINDER_DAYS_BEFORE=7`, `BILLING_SUBSCRIPTION_RENEWAL_REMINDER_DAYS_BEFORE=7`, `BILLING_STRIPE_API_KEY`, `BILLING_STRIPE_WEBHOOK_SECRET`, `BILLING_STRIPE_PUBLISHABLE_KEY`, `BILLING_USAGE_CAP_CLICKHOUSE_ENABLED=false`, `BILLING_ROLLOVER_TOTAL_CAP_MULTIPLIER=2`, `BILLING_MAX_ADMIN_CREDIT_GRANT_MICRO=1000000000`, `ONBOARDING_IMPORT_CONTACTS_CREDITS_REWARD=1000000`, `ONBOARDING_INVITE_TEAM_MAX_INVITES=5`, `ONBOARDING_INVITE_TEAM_CREDITS_REWARD_PER_USER=500000`, `ONBOARDING_ENRICHMENT_CREDIT_REWARD_TIERS={}`.

### 4.5 Email / storage / captcha / misc

Email: `EMAIL_FROM_ADDRESS='noreply@yourdomain.com'`, `EMAIL_FROM_NAME='Felix from Twenty'`, `EMAIL_DRIVER=LOGGER` (`LOGGER|SMTP|AWS_SES|RESEND`), `EMAIL_SMTP_HOST` (required), `EMAIL_SMTP_PORT=587`, `EMAIL_SMTP_NAME`, `EMAIL_SMTP_USER`/`EMAIL_SMTP_PASSWORD` (sensitive), `EMAIL_SMTP_NO_TLS=false`. Storage: `STORAGE_TYPE=LOCAL`, `STORAGE_LOCAL_PATH='.local-storage'`, `STORAGE_S3_REGION/NAME/ENDPOINT/ACCESS_KEY_ID/SECRET_ACCESS_KEY`, `STORAGE_S3_PRESIGNED_URL_ENABLED=false`, `STORAGE_S3_PRESIGNED_URL_BASE`, `STORAGE_S3_PRESIGNED_URL_EXPIRES_IN=900`, `MAX_TARBALL_UPLOAD_SIZE_BYTES=104857600`, `WORKSPACE_STORAGE_LIMIT_BYTES` (unset), `WORKSPACE_RECORD_LIMIT` (unset). Captcha: `CAPTCHA_DRIVER` (unset), `CAPTCHA_SITE_KEY`, `CAPTCHA_SECRET_KEY`, `CAPTCHA_MIN_SCORE=0`, `IS_WORKSPACE_CREATION_CAPTCHA_ENABLED=false`. Toggles: `IS_FEATURE_FLAG_MANAGEMENT_ENABLED=false`, `IS_CONNECTED_ACCOUNT_WEBHOOK_SUBSCRIPTION_ENABLED=false`, `IS_IMAP_SMTP_CALDAV_ENABLED=true`, `IS_IMAP_SMTP_CALDAV_CONNECTION_TEST_ENABLED=true`, `ALLOW_REQUESTS_TO_TWENTY_ICONS=true`, `OUTBOUND_HTTP_ALLOWED_INTERNAL_HOSTS=[]`, `OUTBOUND_HTTP_SAFE_MODE_ENABLED=true`, `IMAP_SMTP_ALLOW_SELF_SIGNED=false`, `WORKSPACE_SCHEMA_DDL_LOCKED=false`, `SHOULD_SEED_STANDARD_RECORD_PAGE_LAYOUTS=true` (deprecated), `MARKETPLACE_CATALOG_SYNC_CRON_ENABLED=true`, `IS_ATTACHMENT_PREVIEW_ENABLED=true`, `IS_MAPS_AND_ADDRESS_AUTOCOMPLETE_ENABLED=false`, `IS_ONBOARDING_AI_CHAT_ENABLED=false`, `IS_OPENAI_REASONING_SUMMARY_ENABLED=true`, `AI_SIGNED_FILE_URLS_ENABLED=true`, `EMAILING_DOMAIN_DRIVER=LOG`. Observability: `LOGGER_DRIVER=CONSOLE`, `LOG_LEVELS=['log','error','warn','performance']`, `LOGGER_IS_BUFFER_ENABLED=true`, `API_ACCESS_LOG_ENABLED=false`, `TYPEORM_LOGGING=['error']`, `METER_DRIVER=[]`, `METER_EXPORT_INTERVAL_MS=30000`, `METER_TEMPORALITY=Delta`, `SENTRY_DSN`, `SENTRY_FRONT_DSN`, `SENTRY_ENVIRONMENT`, `SENTRY_TRACES_SAMPLE_RATE=0.1`, `SENTRY_FRONT_TRACES_SAMPLE_RATE=0.1`, `SENTRY_PROFILES_SAMPLE_RATE=0.01`, `EVENT_SINKS=['clickhouse']`, `ANALYTICS_ENABLED=false`, `CLICKHOUSE_URL`, `USAGE_ROLLUP_FLUSH_INTERVAL_MS=60000`, `TELEMETRY_ENABLED=true`, `HEALTH_METRICS_TIME_WINDOW_IN_MINUTES=5`, `APP_VERSION` (unset). Enterprise / registry / AI: `ENTERPRISE_KEY`, `ENTERPRISE_VALIDITY_TOKEN`, `ENTERPRISE_API_URL`, `APP_REGISTRY_URL`, `APP_REGISTRY_CDN_URL`, `APP_CLAIM_GITHUB_CLIENT_ID/SECRET`, `APP_REGISTRY_TOKEN`, `OPENAI_API_KEY`/`ANTHROPIC_API_KEY`/`GOOGLE_API_KEY`/`XAI_API_KEY`/`GROQ_API_KEY`/`MISTRAL_API_KEY`/`TYPESAFE_AI_API_KEY` (optional), `AI_PROVIDERS={}`, `GOOGLE_MAP_API_KEY`, `PEOPLE_DATA_LABS_API_KEY`, `AWS_SES_*`, `RESEND_API_KEY`, `RESEND_WEBHOOK_SIGNING_SECRET`, `RESEND_DOMAIN_REGION`, `INBOUND_EMAIL_DOMAIN`, `SES_SNS_TOPIC_ARN_ALLOWLIST`, `SES_OUTBOUND_SNS_TOPIC_ARN`, `TATAMI_SES_WEBHOOK_URL`, `TATAMI_SNS_TOPIC_ARN`, `CLOUDFLARE_API_KEY`, `CLOUDFLARE_ZONE_ID`, `CLOUDFLARE_PUBLIC_DOMAIN_ZONE_ID`, `CLOUDFLARE_WEBHOOK_SECRET`, `CLOUDFLARE_DCV_DELEGATION_ID`, `IS_CLOUDFLARE_GEOLOCATION_ENABLED=false`, `SUPPORT_DRIVER=NONE`, `SUPPORT_FRONT_CHAT_ID`, `SUPPORT_FRONT_HMAC_KEY`, `SSL_KEY_PATH`, `SSL_CERT_PATH`.

---

## 5. BILLING, USAGE LIMITS, RATE LIMITING

### 5.1 Stripe (Enterprise)

`srv/src/engine/core-modules/billing/` — plans `PRO | ENTERPRISE | PRO_V2 | ORGANIZATION_V2`
(`billing/enums/billing-plan-key.enum.ts`). Entitlements
(`billing/enums/billing-entitlement-key.enum.ts`): `SSO, CUSTOM_DOMAIN, RLS, RECORD_SHARING,
AUDIT_LOGS, USAGE_LIMIT, VALIDATION_RULES, CUSTOM_ROLES`; `CUSTOM_ROLES` is granted even without
billing (`billing/constants/entitlement-keys-granted-without-billing.constant.ts`). Webhook
endpoint `POST /webhooks/stripe`
(`core-modules/billing-webhook/billing-webhook.controller.ts`): the signature is verified with
`BILLING_STRIPE_WEBHOOK_SECRET` over the **raw body**; events dispatch to the customer / product /
price / subscription / subscription-schedule / invoice / entitlement services. Subscription
statuses `Active, Trialing` activate a workspace
(`billing/constants/workspace-activating-subscription-statuses.constant.ts`); `Active, Trialing,
PastDue, Unpaid, Incomplete, Paused` are "not ended"
(`billing/constants/not-ended-subscription-statuses.constant.ts`);
`MAX_TRIAL_WORKSPACES_PER_USER=2`. `BillingSeatService.isWorkspaceAtSeatLimit(workspaceId)` blocks
new members with `BILLING_SEAT_LIMIT_REACHED` ("This workspace has used all its seats. Contact
your workspace admin to add more." — `auth/services/sign-in-up.service.ts`). Credits are integers
in **microCredits** (1 credit = 1 000 000 microCredits).

### 5.2 Usage limits

`srv/src/engine/core-modules/usage-limit/` — a generic quota/speed/stock framework. Definitions
in `usage-limit/constants/usage-limit-definitions.constant.ts` (`USAGE_LIMIT_DEFINITIONS`), keyed by
`UsageResourceType` (`ALL, API, AI, WORKFLOW, APP, STORAGE, …`) and limit kind
(`quota | speed | stock`). Spender types: `workspace | userWorkspace | apiKey | application |
agent`. Units: `CREDIT | REQUEST | TOKEN | INVOCATION | BYTE | FILE | USER_CREDIT`. Period units:
`second | minute | hour | day | week | month` (calendar-anchored variants in
`utils/get-calendar-day|month|week-period.util.ts`). Default speed limits are wired straight to
the config variables: API/apiKey/perWorkspace/second → `API_RATE_LIMITING_SHORT_*` and
`API_RATE_LIMITING_LONG_*`; API/application/crossWorkspace → `APPLICATION_API_RATE_LIMITING_*`.
The AI quota default is `userWorkspace` / `USER_CREDIT` / `week` from `AI_USER_CREDITS_LIMIT`
(overridable per plan through `USAGE_LIMIT_DEFAULTS_BY_PLAN`). Storage stock defaults come from
`WORKSPACE_STORAGE_LIMIT_BYTES` / `WORKSPACE_RECORD_LIMIT`. Per-workspace overrides live in
`core.usageLimit` (`usage-limit/usage-limit.entity.ts`); instance overrides in
`usage-limit/instance-override/`. Services: `usage-limit-speed.service.ts`,
`usage-limit-quota.service.ts` + `usage-quota-admission.service.ts`, `usage-limit-stock.service.ts`
+ `workspace-record-stock.service.ts`, `usage-period.service.ts`, `usage-limits-cache.service.ts`.
Exhaustion raises `UsageLimitHttpException` with `Retry-After` headers
(`utils/build-rate-limit-response-headers.util.ts`).

### 5.3 Rate limiting / throttling

Two independent mechanisms.
**(1) Token-bucket throttler** (`core-modules/throttler/throttler.service.ts`): a Redis Lua script
`throttler:try-consume-token-buckets`
(`throttler/constants/try-consume-token-buckets-script.constant.ts`) implements a multi-bucket
token bucket; each bucket is a Redis **hash** `token-bucket:<key>` with fields `t` (tokens) and
`ts` (last refill ms); the refill rate is `refill / windowMs` per ms capped at `burst`;
`PEXPIRE` = `ceil(burst / refill * windowMs)` so a key never expires before it would have refilled.
`tokenBucketThrottleOrThrow(key, tokens, maxTokens, windowMs)` throws `ThrottlerException`
(`LIMIT_REACHED`, HTTP **429**, "Rate limit reached. Please try again later.") when the bucket
cannot admit the request; namespace `engine:workspace`. Callers: the password-reset email
(3/15 min per email), the 2FA OTP (10/15 min), recovery-code issuance (10/1 h) and redemption
(5/15 min), invitation sending (10/7 d per email, 500/7 d per workspace), the OAuth token
endpoint (60/60 s per IP), and the workflow soft/hard throttles.
**(2) Usage-limit speed buckets** (§5.2), enforced on the Common/Rest API query runner
(`api/common/common-query-runners/common-base-query-runner.service.ts:621`).
GraphQL cost limits (`GRAPHQL_MAX_FIELDS`, `GRAPHQL_MAX_ROOT_RESOLVERS`,
`COMMON_QUERY_COMPLEXITY_LIMIT`) are separate and apply to GraphQL only.

### 5.4 Billing seats & entitlements

`BillingSeatService.isWorkspaceAtSeatLimit(workspaceId)` is checked **before** a new member is
created (`auth/services/sign-in-up.service.ts`); exceeding it raises
`BillingException(BILLING_SEAT_LIMIT_REACHED)`. `BillingService.hasEntitlement(workspaceId,
BillingEntitlementKey)` gates SSO, custom domains, RLS, record sharing, audit logs, usage limits,
validation rules and custom roles (`billing/services/billing.service.ts`).
`BillingService.hasWorkspaceAnySubscription(workspaceId)` decides `ACTIVE` vs `CREATED`
activation status (§3.3). `BILLING_MAX_ADMIN_CREDIT_GRANT_MICRO` caps a single admin grant at
1e9 microCredits (1000 credits); `BILLING_ROLLOVER_TOTAL_CAP_MULTIPLIER=2` caps the credits a
workspace may hold at 2× the plan allowance.

---

## 6. BIT-COMPATIBILITY CHECKLIST (a Rust server must accept existing Twenty data)

| Item | Exact algorithm / params | Reference |
|---|---|---|
| Password hash | **bcrypt, cost 10** (`$2b$` prefix) | `core-modules/auth/auth.util.ts:11-23` |
| Password policy | regex `^.{8,50}$` | same |
| Legacy JWT secret | `sha256(APP_SECRET ‖ body ‖ type).hex`, body = `workspaceId` else `userId`, type = the raw enum string | `core-modules/jwt/services/jwt-wrapper.service.ts:207-216` |
| Current JWT alg | **ES256** (P-256), header `kid` = `core.signingKey.id` | `jwt-wrapper.service.ts:56-83`, `jwt-key-manager.service.ts:224-236` |
| Signing key encoding | private = **PKCS#8 PEM**, public = **SPKI PEM** | `jwt-key-manager.service.ts:224-236` |
| Refresh token id | `jti` = `core.appToken.id` (uuid) | `auth/token/services/refresh-token.service.ts:140-152` |
| Login token id | `jti` = `randomUUID()` | `auth/token/services/login-token.service.ts:57` |
| Email-verification token | `randomBytes(32).hex`; stored `sha256(hex).hex` | `auth/token/services/email-verification-token.service.ts:39-57` |
| Password-reset token | `randomBytes(32).hex`; stored `sha256(hex).hex` | `auth/services/reset-password.service.ts:100-150` |
| SSO exchange token | `randomBytes(32).hex`; stored `sha256(hex).hex` | `auth/token/services/sso-exchange-token.service.ts:22-45` |
| OAuth authorization code | `randomBytes(42).hex`; stored `sha256(hex).hex`, 5 min | `auth/services/auth.service.ts:~640` |
| **Workspace invitation token** | `randomBytes(32).hex`; stored **verbatim** (not hashed) | `core-modules/workspace-invitation/services/workspace-invitation.service.ts:465-497` |
| Session token | `sess_` + `randomBytes(32).base64url`; stored `sha256(token).hex` | `user-session/utils/generate-user-session-token.util.ts`, `hash-user-session-token.util.ts` |
| 2FA backup/recovery code | 20 chars from `0123456789ABCDEFGHJKMNPQRSTVWXYZ`, grouped `XXXXX-XXXXX-XXXXX-XXXXX`; stored `sha256(code.toUpperCase().replace(/[^0-9A-Z]/g,'')).hex` | `two-factor-authentication/constants/*`, `utils/hash-two-factor-authentication-recovery-code.util.ts` |
| TOTP | RFC 6238, sha1, 6 digits, 30 s step, window 1, hex encoding | `two-factor-authentication/strategies/otp/totp/constants/totp.strategy.constants.ts` |
| Encrypted secrets (`enc:v2:`) | `enc:v2:<8-hex keyId>:<base64 IV(12)‖ct‖tag(16)>`; AES-256-GCM; key = HKDF-SHA256(rawKey, salt=32×0, info=`twenty:enc:v2:<workspaceId\|instance>`, 32) | `core-modules/secret-encryption/` |
| Legacy encrypted secrets | AES-256-CTR, key `sha512(rawKey).hex[0..32]`, 16-byte IV prefix, base64 | `secret-encryption/utils/encrypt-aes-ctr.util.ts` |
| Legacy `encryptText` helper | AES-256-CTR, key `sha512(key).hex[0..32]`, 16-byte IV prefix, base64 | `core-modules/auth/auth.util.ts:25-52` |
| Session cookie names | `__Host-twenty-session` (https) / `twenty-session` (http); impersonator `__Host-twenty-impersonator-session` / `twenty-impersonator-session` | `user-session/constants/` |
| Cookie flags | `httpOnly`, `secure` iff `SERVER_URL` is https or `sameSite=none`, `sameSite=AUTH_COOKIE_SAME_SITE`, `path=/` | `user-session/services/user-session-cookie.service.ts:74-82` |
| Redis cache namespaces | `engine:workspace`, `engine:core-entity`, `engine:auth-session`, `engine:usage-limit`, … | `cache-storage/types/cache-storage-namespace.enum.ts` |
| express-session store prefix | `engine:session:` | `core-modules/session-storage/session-storage.module-factory.ts` |
| Login-token single-use key | `login-token:consumed:<jti>` | `auth/constants/consumed-login-token-cache-key-prefix.constant.ts` |
| SAML replay key | `saml-assertion:consumed:<idpId>:<assertionId>` | `sso/constants/consumed-saml-assertion-cache-key-prefix.constant.ts` |
| Used-OTP key | `two-factor-authentication-used-otp:<methodId>:<token>` | `two-factor-authentication/utils/build-two-factor-authentication-used-otp-key.util.ts` |
| Throttler key | `token-bucket:<key>` | `throttler/constants/token-bucket-throttle-key-prefix.constant.ts` |
| Speed-limit bucket key | `{scope}:speed:{resourceType}:{operationType}:{spenderType}:{spenderId\|'-'}:{windowSeconds}` | `usage-limit/utils/build-speed-bucket-key.util.ts` |
| Invitation throttle key | `invitation-resending-workspace:throttler:<emailOrWorkspaceId>` | `workspace-invitation/utils/build-invitation-sending-throttle-key.util.ts` |
| OAuth throttle key | `oauth:<ip>` | `application/application-oauth/controllers/oauth-token.controller.ts` |
| Admin role id | `20202020-02c2-43f2-b94d-cab1f2b532eb` | `workspace-manager/twenty-standard-application/constants/standard-role.constant.ts` |
| Member role id | `20202020-a8a6-47ae-be79-e3601fa46a8e` | `packages/twenty-shared/src/constants/MemberRoleUniversalIdentifier.ts` |
| Dev-seed password hash | `$2b$10$3LwXjJRtLsfx4hLuuXhxt.3mWgismTiZFCZSG3z9kDrSfsrBl0fT6` | `workspace-manager/dev-seeder/core/utils/seed-users.util.ts` |

---

## 7. APPENDIX

### 7.1 `core.appToken` — the generic token table

`srv/src/engine/core-modules/app-token/app-token.entity.ts:21-34` — `AppTokenType`:
`REFRESH_TOKEN, CODE_CHALLENGE, AUTHORIZATION_CODE, PASSWORD_RESET_TOKEN, INVITATION_TOKEN,
ONBOARDING_INVITATION_TOKEN, EMAIL_VERIFICATION_TOKEN, ENTERPRISE_VALIDITY_TOKEN,
SSO_EXCHANGE_TOKEN, TWO_FACTOR_AUTHENTICATION_RECOVERY_CODE, TWO_FACTOR_AUTHENTICATION_BACKUP_CODE`.
Entity: `id uuid`, `userId?`, `workspaceId?`, `type`, `value text`, `expiresAt`, `deletedAt?`,
`revokedAt?`, `context jsonb` (`{ email?, roleId?, redirectUri?, clientId?, codeChallenge?, scope?,
resource?, authProvider?, revokedReason?, issuedByUserId?, twoFactorAuthenticationMethodId? }`);
`context.email` is lowercased on insert/update. Unique index
`IDX_APP_TOKEN_TYPE_VALUE_SSO_EXCHANGE_UNIQUE` on `(type, value)` for live SSO exchange tokens;
`IDX_APP_TOKEN_RECOVERY_CODE_PENDING_UNIQUE` on `(userId, workspaceId)` for a pending 2FA
recovery code.

### 7.2 Workspace invitations

`srv/src/engine/core-modules/workspace-invitation/services/workspace-invitation.service.ts` —
token = **`crypto.randomBytes(32).toString('hex')` stored verbatim** in `core.appToken.value`,
type `INVITATION_TOKEN` / `ONBOARDING_INVITATION_TOKEN`, `context = { email, roleId? }`,
`expiresAt = now + INVITATION_TOKEN_EXPIRES_IN` (30d) (`:465-497`). Validation: the token must
exist, `context.email` must equal the sign-in email (case-insensitive), and it must not be
expired. Invitation link = `<workspaceUrl>/invite/:workspaceInviteHash`. Throttling (`:500-540`):
per **email** 10/7 days and per **workspace** 500/7 days, key
`invitation-resending-workspace:throttler:<emailOrWorkspaceId>`. Public invite links use
`workspace.inviteHash` (uuid v4), gated by `workspace.isPublicInviteLinkEnabled`.

### 7.3 Frontend paths, error codes, captcha, admin panel

- **AppPath** (`packages/twenty-shared/src/types/AppPath.ts:1-45`): `Verify=/verify`,
  `VerifyEmail=/verify-email`, `SignInUp=/welcome`, `Invite=/invite/:workspaceInviteHash`,
  `ResetPassword=/reset-password/:passwordResetToken`, `WorkspaceActivation=/workspace-activation`,
  `CreateProfile=/create/profile`, `Authorize=/authorize`, `PlanRequired=/plan-required`,
  `InviteTeam=/invite-team`, `BookCall=/book-call`, `GettingStarted=/getting-started`, `Dpa=/dpa`.
- **Auth error codes / HTTP status** (`core-modules/auth/auth.exception.ts`): codes
  `USER_NOT_FOUND, USER_WORKSPACE_NOT_FOUND, EMAIL_NOT_VERIFIED, CLIENT_NOT_FOUND,
  WORKSPACE_NOT_FOUND, WORKSPACE_SUSPENDED, APPLICATION_NOT_FOUND, INVALID_INPUT,
  FORBIDDEN_EXCEPTION, IDENTITY_ASSERTION_REQUIRED, INSUFFICIENT_SCOPES, UNAUTHENTICATED,
  SESSION_SUPERSEDED, APPLICATION_REFRESH_TOKEN_INVALID_OR_EXPIRED, INVALID_DATA,
  OAUTH_ACCESS_DENIED, SSO_AUTH_FAILED, USE_SSO_AUTH, SIGNUP_DISABLED, GOOGLE_API_AUTH_DISABLED,
  MICROSOFT_API_AUTH_DISABLED, MISSING_ENVIRONMENT_VARIABLE, ENTERPRISE_VALIDITY_TOKEN_NOT_VALID,
  INVALID_JWT_TOKEN_TYPE, TWO_FACTOR_AUTHENTICATION_PROVISION_REQUIRED,
  TWO_FACTOR_AUTHENTICATION_VERIFICATION_REQUIRED, USER_ALREADY_EXISTS` + the common codes.
  Status: 400 `INVALID_INPUT`; 401 `UNAUTHENTICATED, SESSION_SUPERSEDED, USER_NOT_FOUND,
  WORKSPACE_NOT_FOUND, APPLICATION_NOT_FOUND, INVALID_DATA`, both 2FA codes,
  `APPLICATION_REFRESH_TOKEN_INVALID_OR_EXPIRED`; 403 `FORBIDDEN_EXCEPTION,
  IDENTITY_ASSERTION_REQUIRED, WORKSPACE_SUSPENDED, INSUFFICIENT_SCOPES, OAUTH_ACCESS_DENIED,
  SSO_AUTH_FAILED, USE_SSO_AUTH, SIGNUP_DISABLED, EMAIL_NOT_VERIFIED, INVALID_JWT_TOKEN_TYPE,
  USER_ALREADY_EXISTS, MISSING_ENVIRONMENT_VARIABLE`; 404 `CLIENT_NOT_FOUND`; 500
  `INTERNAL_SERVER_ERROR, USER_WORKSPACE_NOT_FOUND`.
- **Captcha** (`core-modules/captcha/captcha.guard.ts`): the token is read from the GraphQL arg
  `captchaToken` (or `input.captchaToken`); failure throws `CaptchaException` "Invalid Captcha,
  please try another device". `WorkspaceCreationCaptchaGuard` only enforces it when
  `IS_WORKSPACE_CREATION_CAPTCHA_ENABLED=true`.
- **Admin panel** (`core-modules/admin-panel/`, Enterprise): guarded by `AdminPanelGuard`
  (`srv/src/engine/guards/admin-panel.guard.ts`), which requires
  `user.canAccessFullAdminPanel === true` and, when `SERVER_ADMIN_WORKSPACE_ID` is set, that the
  request comes from that workspace; the identity-assertion service additionally requires the
  proxy-signed header JWT for requests into the server-admin workspace.

### 7.4 OAuth application token endpoint (Enterprise, `/oauth`)

`srv/src/engine/core-modules/application/application-oauth/controllers/oauth-token.controller.ts` —
`POST /oauth/token` (grants `authorization_code`, `client_credentials`, `refresh_token`),
`POST /oauth/revoke` (RFC 7009, always 200), `POST /oauth/introspect`. Rate limit **60 req / 60 s
per IP** (token bucket key `oauth:<ip>`); `invalid_client` → 401, other errors → 400, rate limit →
429 `rate_limit_exceeded`; responses set `Cache-Control: no-store`, `Pragma: no-cache`.
Code exchange (`application/application-oauth/oauth.service.ts` `exchangeAuthorizationCode`): the
code is sha256-hashed to find the `appToken`; a **reused or concurrently redeemed** code is
rejected; `redirect_uri`, `resource` and `client_id` must match the stored `context`; PKCE S256 is
verified as `base64url(sha256(code_verifier)) == code_challenge`; the consume is an atomic
`UPDATE … WHERE revokedAt IS NULL`. Response:
`{ access_token, token_type: 'Bearer', expires_in, refresh_token?, scope }`.
`client_credentials` requires **exactly one** workspace installation of the registration. Dynamic
client registration + client-id metadata documents:
`application/application-oauth/controllers/oauth-registration.controller.ts`,
`oauth-discovery.controller.ts` (grants `authorization_code, refresh_token`).

### 7.5 Express session, speed-limit buckets, signing-key cache

- **express-session** (`core-modules/session-storage/session-storage.module-factory.ts`): Redis
  store with key prefix **`engine:session:`**; `secret` = the array from
  `resolveSessionCookieSecretsOrThrow` (HKDF instance-HMAC for `ENCRYPTION_KEY`/`APP_SECRET`, the
  fallback key, plus the legacy `sha256(APP_SECRET + 'SESSION_STORE_SECRET')` hex); cookie
  `secure` iff `SERVER_URL` starts with `https`, `httpOnly`, `sameSite='lax'`, `maxAge = 30 min`;
  `proxy: true`, `resave: false`, `saveUninitialized: false`.
- **Speed-limit buckets** (`api/common/common-query-runners/common-base-query-runner.service.ts:621`
  calls `usageLimitSpeedService.consumeOrThrow`): key format
  `{scope}:speed:{resourceType}:{operationType}:{spenderType}:{spenderId|'-'}:{windowSeconds}`
  (`usage-limit/utils/build-speed-bucket-key.util.ts`) where `scope` = the workspace id for
  `perWorkspace` counters and the literal `server` for `crossWorkspace` counters.
- **Signing-key cache**: `jwt/services/signing-key-verify-counter.service.ts` counts verifications
  per `kid` and per legacy algorithm; `jwt/services/signing-key-entity-cache-provider.service.ts`
  caches the **public** key under the `signingKeyPublicKey` entity-cache namespace, excluding
  revoked keys.
