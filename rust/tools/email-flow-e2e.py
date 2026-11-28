import hashlib
import json
import os
import quopri
import re
import secrets
import subprocess
import sys
import time
import urllib.error
import urllib.request

BASE = os.environ.get("BASE", "http://127.0.0.1:3999")
SINK_LOG = os.environ.get("SMTP_SINK_LOG", "/tmp/smtp-sink.log")
SERVER_LOG = os.environ.get("SERVER_LOG", "")
VERIFICATION_REQUIRED = os.environ.get("EMAIL_VERIFICATION_REQUIRED", "0") == "1"
EXPECTED_FROM = os.environ.get("EMAIL_EXPECTED_FROM", "")
INVITE_FIRST_NAME = "Ada"
INVITE_LAST_NAME = "Tester"
FAILURES = []
TOKENS = []


def check(label, ok, detail=""):
    print(("ok   " if ok else "FAIL ") + label + (f" [{detail}]" if detail else ""))
    if not ok:
        FAILURES.append(label)


def psql(query):
    url = os.environ.get("E2E_DATABASE_URL")
    if url:
        argv = ["psql", url, "-tAc", query]
    else:
        argv = [
            "docker", "exec", os.environ.get("E2E_DATABASE_CONTAINER", "infra-db-1"),
            "psql", "-U", "postgres", "-d", os.environ.get("E2E_DATABASE_NAME", "twenty_ref"),
            "-tAc", query,
        ]
    done = subprocess.run(argv, capture_output=True, text=True)
    if done.returncode != 0:
        raise SystemExit("psql failed: " + done.stderr.strip()[:200])
    return done.stdout.strip()


def request(path, payload, token=None):
    req = urllib.request.Request(BASE + path, data=json.dumps(payload).encode(), method="POST")
    req.add_header("content-type", "application/json")
    if token:
        req.add_header("authorization", "Bearer " + token)
    try:
        with urllib.request.urlopen(req) as resp:
            return resp.status, resp.read().decode()
    except urllib.error.HTTPError as err:
        return err.code, err.read().decode()


def messages():
    if not os.path.exists(SINK_LOG):
        return []
    raw = open(SINK_LOG, "rb").read().decode("utf-8", "replace")
    return raw.split("CMD DATA\n=== MESSAGE ===\n")[1:]


def read(message):
    subject = re.search(r"^Subject: (.*)$", message, re.M).group(1)
    boundary = re.search(r'boundary="([^"]+)"', message).group(1)
    plain = ""
    for chunk in message.split("--" + boundary):
        if "Content-Type: text/plain" in chunk:
            payload = chunk.split("\n\n", 1)[1].rstrip("-\n").rstrip()
            plain = quopri.decodestring(payload.encode()).decode("utf-8", "replace")
    return subject, plain, message.split("\n\n", 1)[0]


def find_message(recipient, subject):
    for _ in range(20):
        for frame in messages():
            if f"To: {recipient}" in frame and read(frame)[0] == subject:
                return frame
        time.sleep(0.5)
    return None


def token_matches_app_token(email, token_type, plain_token):
    stored = psql(
        "SELECT t.value FROM core.\"appToken\" t JOIN core.\"user\" u ON u.id=t.\"userId\" "
        f"WHERE u.email='{email}' AND t.type='{token_type}' AND t.\"revokedAt\" IS NULL "
        "ORDER BY t.\"createdAt\" DESC LIMIT 1"
    )
    return hashlib.sha256(plain_token.encode()).hexdigest() == stored


suffix = secrets.token_hex(5)
email = f"e2e-{suffix}@nudge.test"
guest = f"guest-{suffix}@nudge.test"
password = "Aa1-" + secrets.token_urlsafe(12) + "#k"
new_password = "Bb2-" + secrets.token_urlsafe(12) + "#z"

status, body = request("/auth/sign-up", {
    "email": email,
    "password": password,
    "workspaceDisplayName": "Mailer HQ",
    "subdomain": f"mailer-{suffix}",
})
payload = json.loads(body) if body else {}
workspace_id = payload.get("workspaceId")
check("sign up returns 201", status == 201, f"{status} {body[:120]}")
check("sign up leaves isEmailVerified false",
      psql(f"SELECT \"isEmailVerified\" FROM core.\"user\" WHERE email='{email}'") == "f")

if VERIFICATION_REQUIRED:
    frame = find_message(email, "Welcome to Nudge: Please Confirm Your Email")
    check("sign-up queued the verification email", frame is not None)
    token = None
    if frame:
        _, plain, headers = read(frame)
        if EXPECTED_FROM:
            check("verification from header matches the configured sender",
                  "From: " + EXPECTED_FROM in headers, headers.splitlines()[1])
        match = re.search(r"emailVerificationToken=([0-9a-f]{32,})", plain)
        check("verification link carries a token", match is not None)
        if match:
            token = match.group(1)
            TOKENS.append(token)
            check("verification token matches the stored hash",
                  token_matches_app_token(email, "EMAIL_VERIFICATION_TOKEN", token))

    if token:
        status, body = request("/auth/login-token", {
            "email": email, "password": password, "workspaceId": workspace_id,
        })
        check("login refused while the email is unverified",
              status == 403 and "EMAIL_NOT_VERIFIED" in body, f"{status} {body[:160]}")

        status, body = request("/auth/verify-email", {
            "emailVerificationToken": token, "email": email, "workspaceId": workspace_id,
        })
        check("verify email accepts the emailed token", status == 200, f"{status} {body[:160]}")
        check("verification flips isEmailVerified",
              psql(f"SELECT \"isEmailVerified\" FROM core.\"user\" WHERE email='{email}'") == "t")
else:
    status, body = request("/auth/resend-email-verification-token", {"email": email})
    check("resend refused when verification is not required",
          status == 400 and "EMAIL_VERIFICATION_NOT_REQUIRED" in body, f"{status} {body[:120]}")
    check("sign-up sent no verification email when not required",
          find_message(email, "Welcome to Nudge: Please Confirm Your Email") is None)

status, body = request("/auth/login-token", {
    "email": email, "password": password, "workspaceId": workspace_id,
})
check("login with the seeded password", status == 200, f"{status} {body[:160]}")

access = None
if status == 200:
    status, body = request("/auth/tokens", {
        "loginToken": json.loads(body).get("loginToken"), "workspaceId": workspace_id,
    })
    check("auth tokens issued", status == 200, f"{status} {body[:120]}")
    access = json.loads(body).get("accessToken") if status == 200 else None

status, body = request("/auth/email-password-reset-link", {"email": email})
check("password reset link accepted", status == 200, f"{status} {body[:120]}")
frame = find_message(email, "Action Needed to Reset Password")
check("reset email reached the sink", frame is not None)

match = None
if frame:
    _, plain, _ = read(frame)
    check("reset copy states the upstream five minute window",
          "valid for the next 5 minutes" in plain)
    match = re.search(r"reset-password/([0-9a-f]{32,})", plain)
    check("reset link carries a token", match is not None)

if match:
    reset_token = match.group(1)
    TOKENS.append(reset_token)
    check("reset token matches the stored hash",
          token_matches_app_token(email, "PASSWORD_RESET_TOKEN", reset_token))
    old_hash = psql(f"SELECT \"passwordHash\" FROM core.\"user\" WHERE email='{email}'")
    status, body = request("/auth/update-password-via-reset-token", {
        "passwordResetToken": reset_token, "newPassword": new_password,
    })
    check("password update accepts the reset token", status == 200, f"{status} {body[:160]}")
    check("password hash rotated", bool(old_hash)
          and old_hash != psql(f"SELECT \"passwordHash\" FROM core.\"user\" WHERE email='{email}'"))
    check("password changed email reached the sink",
          find_message(email, "Your Password Has Been Successfully Changed") is not None)
    status, _ = request("/auth/validate-password-reset-token", {"passwordResetToken": reset_token})
    check("reset token is single use", status == 403, f"{status}")
    status, _ = request("/auth/update-password-via-reset-token", {
        "passwordResetToken": reset_token, "newPassword": password,
    })
    check("reset token replay refused", status == 403, f"{status}")

    status, body = request("/auth/login-token", {
        "email": email, "password": new_password, "workspaceId": workspace_id,
    })
    check("login with the new password", status == 200, f"{status} {body[:160]}")
    if status == 200:
        status, body = request("/auth/tokens", {
            "loginToken": json.loads(body).get("loginToken"), "workspaceId": workspace_id,
        })
        if status == 200:
            access = json.loads(body).get("accessToken")

psql(f"UPDATE core.\"user\" SET \"firstName\"='{INVITE_FIRST_NAME}', "
     f"\"lastName\"='{INVITE_LAST_NAME}' WHERE email='{email}'")

if access:
    status, body = request("/auth/workspace-invitations", {"emails": [guest]}, access)
    check("invitation request accepted", status == 200, f"{status} {body[:160]}")
    frame = find_message(guest, "Join your team on Nudge")
    check("invitation email reached the sink", frame is not None)
    if frame:
        headers = read(frame)[2]
        from_header = [line for line in headers.splitlines() if line.lower().startswith("from:")][0]
        check("invitation from header carries the sender name",
              f"{INVITE_FIRST_NAME} {INVITE_LAST_NAME} (via Nudge)" in from_header, from_header)
else:
    check("invitation flow needs an access token", False, "no access token available")

if SERVER_LOG and TOKENS:
    log = open(SERVER_LOG).read()
    check("no emailed token written to the server log", not any(token in log for token in TOKENS))

print("messages captured: " + str(len(messages())))
if FAILURES:
    print(f"failed checks: {len(FAILURES)} -> " + ", ".join(FAILURES))
    sys.exit(1)
print("all email flow checks passed")
