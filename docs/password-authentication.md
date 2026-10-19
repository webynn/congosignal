# Built-in authentication

Set `AUTH_MODE=password`, `APP_ORIGIN` to the exact HTTPS origin, and `AUTH_OWNER_EMAIL` to the bootstrap owner email. No ZITADEL or SMTP connection is required. Local development can retain `AUTH_MODE=local` on loopback only.

Sign in at `/auth/login`. Passwords require at least 12 characters, are stored as salted Argon2id hashes, and are never returned. Login and setup endpoints require same-origin JSON requests and have persistent per-account/link and global attempt limits. Sessions are HttpOnly, SameSite=Lax, Secure on HTTPS, and expire after eight hours. Authenticated mutations retain the existing CSRF checks and database role checks.

In **Team & access**, invite the email, then choose **Setup / reset password**. Share the displayed link privately with that member. Each link expires after 30 minutes; generating another invalidates previous links. Setting a password consumes the link and revokes all of that user's sessions. Removing or updating a member invalidates outstanding links and sessions. Administrators cannot issue owner reset links.

Email delivery is not configured. Forgotten-password instructions direct members to the workspace owner/administrator; the UI never claims an email was sent. An owner locked out without another administrator must use operator recovery below.

## Operator recovery / first owner setup

Use a trusted server console, not a public HTTP endpoint. Generate a cryptographically random 32-byte hexadecimal token. Store only its SHA-256 hexadecimal digest in `password_links`, with the active member's normalized email, an expiry timestamp, and an operator identifier in `created_by`. Delete previous links for that email first. Deliver `APP_ORIGIN/auth/login#setup=TOKEN` privately to that user. Do not put raw tokens into logs or source control. A 24-hour expiry is permitted for the first operator-issued setup; normal UI links last 30 minutes. Never set or request the user's actual password.

Existing OIDC users retain their user IDs when setting a password, preserving private notes, saved articles, briefs, membership and spending allowances. Existing sessions can finish their eight-hour lifetime, or are revoked when the password is set. OIDC mode remains a rollback option but is not used in password mode.

## Validation

`python3 tests/password_auth_smoke.py` runs an isolated server and database. It covers the full setup/login/logout lifecycle, identity preservation, Argon2 hashes, CSRF, role restrictions, reset replay/concurrency, expired/revoked links and sessions, and throttling. It does not use real credentials or alter production data.
