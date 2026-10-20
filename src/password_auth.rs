//! Invite-only password authentication. Links and session tokens are stored hashed.
use crate::{
    ApiError, ApiResult, App,
    authentication::{actor, cookie_value, hash, random, valid_email},
};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::sync::{Arc, OnceLock};
const WORKSPACE: &str = "congo-signal";
static HASH_SLOTS: OnceLock<Arc<tokio::sync::Semaphore>> = OnceLock::new();
static DUMMY: OnceLock<String> = OnceLock::new();
pub fn enabled() -> bool {
    std::env::var("AUTH_MODE").as_deref() == Ok("password")
}
fn error(status: StatusCode, text: &str) -> ApiError {
    ApiError(status, text.into())
}
pub async fn init(db: &SqlitePool) -> anyhow::Result<()> {
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS password_credentials(user_id TEXT PRIMARY KEY,password_hash TEXT NOT NULL,updated_at INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS password_links(hash TEXT PRIMARY KEY,email TEXT NOT NULL,expires INTEGER NOT NULL,created_by TEXT NOT NULL);
    CREATE INDEX IF NOT EXISTS password_links_email ON password_links(email);
    CREATE TABLE IF NOT EXISTS password_attempts(bucket TEXT PRIMARY KEY,started INTEGER NOT NULL,attempts INTEGER NOT NULL);
    CREATE TABLE IF NOT EXISTS member_profiles(user_id TEXT PRIMARY KEY,name TEXT NOT NULL,organisation TEXT NOT NULL,purpose TEXT NOT NULL,created_at INTEGER NOT NULL);").execute(db).await?;
    if enabled() {
        let _ = DUMMY.set(hash_password(random()).await?);
    }
    Ok(())
}
fn guard(headers: &HeaderMap) -> ApiResult<()> {
    if !enabled() {
        return Err(error(
            StatusCode::NOT_FOUND,
            "Password login is not enabled",
        ));
    }
    let origin = std::env::var("APP_ORIGIN").unwrap_or_default();
    if origin.is_empty()
        || headers.get("origin").and_then(|v| v.to_str().ok()) != Some(origin.as_str())
    {
        return Err(error(StatusCode::FORBIDDEN, "Invalid request origin"));
    }
    Ok(())
}
async fn throttle(db: &SqlitePool, kind: &str, identifier: &str) -> ApiResult<()> {
    let now = chrono::Utc::now().timestamp();
    sqlx::query("DELETE FROM password_attempts WHERE started<?")
        .bind(now - 900)
        .execute(db)
        .await?;
    for (bucket, limit) in [
        (format!("global:{kind}"), 200),
        (format!("{kind}:{}", hash(identifier)), 10),
    ] {
        let n:i64=sqlx::query_scalar("INSERT INTO password_attempts VALUES(?,?,1) ON CONFLICT(bucket) DO UPDATE SET attempts=attempts+1 RETURNING attempts").bind(bucket).bind(now).fetch_one(db).await?;
        if n > limit {
            return Err(error(
                StatusCode::TOO_MANY_REQUESTS,
                "Too many attempts. Try again in 15 minutes.",
            ));
        }
    }
    Ok(())
}
async fn hash_password(password: String) -> anyhow::Result<String> {
    let permit = HASH_SLOTS
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(4)))
        .clone()
        .acquire_owned()
        .await?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let salt = SaltString::encode_b64(uuid::Uuid::new_v4().as_bytes())
            .map_err(|_| anyhow::anyhow!("Salt generation failed"))?;
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|_| anyhow::anyhow!("Password hashing failed"))
    })
    .await?
}
async fn check_password(password: String, encoded: String) -> anyhow::Result<bool> {
    let permit = HASH_SLOTS
        .get_or_init(|| Arc::new(tokio::sync::Semaphore::new(4)))
        .clone()
        .acquire_owned()
        .await?;
    Ok(tokio::task::spawn_blocking(move || {
        let _permit = permit;
        PasswordHash::new(&encoded).is_ok_and(|h| {
            Argon2::default()
                .verify_password(password.as_bytes(), &h)
                .is_ok()
        })
    })
    .await?)
}
fn response(value: Value) -> Response {
    let mut r = Json(value).into_response();
    r.headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    r
}
pub async fn page() -> Response {
    let mut r = Html(include_str!("../dashboard/login.html")).into_response();
    for (k, v) in [
        ("cache-control", "no-store"),
        ("referrer-policy", "no-referrer"),
        ("x-frame-options", "DENY"),
        (
            "content-security-policy",
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src https://fonts.gstatic.com; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'",
        ),
    ] {
        r.headers_mut().insert(k, v.parse().unwrap());
    }
    r
}
pub async fn script() -> Response {
    (
        [
            ("content-type", "text/javascript; charset=utf-8"),
            ("cache-control", "no-store"),
        ],
        include_str!("../dashboard/login.js"),
    )
        .into_response()
}
#[derive(Deserialize)]
pub struct Login {
    email: String,
    password: String,
}
pub async fn login(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<Login>,
) -> ApiResult<Response> {
    guard(&headers)?;
    let email = input.email.trim().to_lowercase();
    if email.len() > 254 || input.password.len() > 1024 {
        return Err(error(StatusCode::BAD_REQUEST, "Invalid credentials"));
    }
    throttle(&app.db, "login", &email).await?;
    let row=sqlx::query("SELECT m.user_id,c.password_hash FROM memberships m JOIN password_credentials c ON c.user_id=m.user_id WHERE m.workspace_id=? AND m.email=? AND m.active=1").bind(WORKSPACE).bind(&email).fetch_optional(&app.db).await?;
    let encoded = row
        .as_ref()
        .map(|r| r.get::<String, _>("password_hash"))
        .unwrap_or_else(|| DUMMY.get().cloned().unwrap_or_default());
    let valid = check_password(input.password, encoded.clone()).await?;
    if !valid || row.is_none() {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            "Email or password is incorrect",
        ));
    }
    let uid: String = row.unwrap().get("user_id");
    let mut tx = app.db.begin_with("BEGIN IMMEDIATE").await?;
    // Recheck after expensive hashing: a reset, revocation or role update may have happened.
    let active:i64=sqlx::query_scalar("SELECT count(*) FROM memberships m JOIN password_credentials c ON c.user_id=m.user_id WHERE m.user_id=? AND m.workspace_id=? AND m.active=1 AND c.password_hash=?").bind(&uid).bind(WORKSPACE).bind(encoded).fetch_one(&mut *tx).await?;
    if active != 1 {
        return Err(error(
            StatusCode::UNAUTHORIZED,
            "Email or password is incorrect",
        ));
    }
    let token = random();
    let csrf = random();
    let now = chrono::Utc::now().timestamp();
    sqlx::query("DELETE FROM auth_sessions WHERE expires<?")
        .bind(now)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO auth_sessions VALUES(?,?,?,?,?)")
        .bind(hash(&token))
        .bind(&uid)
        .bind(WORKSPACE)
        .bind(&csrf)
        .bind(now + 28800)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO audit_log(at,user_id,workspace_id,action,target) VALUES(?,?,?,'password_login','session')").bind(chrono::Utc::now().to_rfc3339()).bind(&uid).bind(WORKSPACE).execute(&mut *tx).await?;
    tx.commit().await?;
    let mut r = response(json!({"redirect":"/"}));
    r.headers_mut().insert(
        "set-cookie",
        cookie_value("congo_session", &token, 28800)
            .parse()
            .unwrap(),
    );
    Ok(r)
}
#[derive(Deserialize)]
pub struct Setup {
    token: String,
    password: String,
}
pub async fn setup(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<Setup>,
) -> ApiResult<Response> {
    guard(&headers)?;
    if input.token.len() != 64 || input.password.chars().count() < 12 || input.password.len() > 1024
    {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Use a valid link and a password of at least 12 characters (maximum 1024 bytes).",
        ));
    }
    throttle(&app.db, "setup", &input.token).await?;
    let token_hash = hash(&input.token);
    let now = chrono::Utc::now().timestamp();
    let exists: i64 =
        sqlx::query_scalar("SELECT count(*) FROM password_links WHERE hash=? AND expires>?")
            .bind(&token_hash)
            .bind(now)
            .fetch_one(&app.db)
            .await?;
    if exists != 1 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "This link expired or was already used. Ask your administrator for a new link.",
        ));
    }
    let encoded = hash_password(input.password).await?;
    let mut tx = app.db.begin_with("BEGIN IMMEDIATE").await?;
    let email: Option<String> =
        sqlx::query_scalar("DELETE FROM password_links WHERE hash=? AND expires>? RETURNING email")
            .bind(token_hash)
            .bind(chrono::Utc::now().timestamp())
            .fetch_optional(&mut *tx)
            .await?;
    let email = email.ok_or_else(|| {
        error(
            StatusCode::BAD_REQUEST,
            "This link expired or was already used",
        )
    })?;
    let member = sqlx::query(
        "SELECT user_id FROM memberships WHERE workspace_id=? AND email=? AND active=1",
    )
    .bind(WORKSPACE)
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| error(StatusCode::FORBIDDEN, "Invitation is no longer active"))?;
    // Preserve OIDC user IDs: bookmarks, notes, briefs and allowances remain attached.
    let uid = member
        .get::<Option<String>, _>("user_id")
        .unwrap_or_else(|| hash(&format!("password:{email}")));
    sqlx::query("INSERT OR IGNORE INTO auth_users VALUES(?,?,?)")
        .bind(&uid)
        .bind(format!("password:{email}"))
        .bind(&email)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE memberships SET user_id=? WHERE workspace_id=? AND email=?")
        .bind(&uid)
        .bind(WORKSPACE)
        .bind(&email)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO password_credentials VALUES(?,?,?) ON CONFLICT(user_id) DO UPDATE SET password_hash=excluded.password_hash,updated_at=excluded.updated_at").bind(&uid).bind(encoded).bind(now).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM auth_sessions WHERE user_id=?")
        .bind(&uid)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM password_links WHERE email=?")
        .bind(&email)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO audit_log(at,user_id,workspace_id,action,target) VALUES(?,?,?,'password_set','credentials')").bind(chrono::Utc::now().to_rfc3339()).bind(&uid).bind(WORKSPACE).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(response(json!({"password_set":true,"email":email})))
}
#[derive(Deserialize)]
pub struct Email {
    email: String,
}
pub async fn access_link(State(app): State<App>, Json(input): Json<Email>) -> ApiResult<Response> {
    if !enabled() {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Password login is not enabled",
        ));
    }
    let a = actor();
    let email = input.email.trim().to_lowercase();
    if !["owner", "administrator"].contains(&a.role.as_str()) || !valid_email(&email) {
        return Err(error(StatusCode::FORBIDDEN, "Not permitted"));
    }
    let mut tx = app.db.begin_with("BEGIN IMMEDIATE").await?;
    let target =
        sqlx::query("SELECT role FROM memberships WHERE email=? AND workspace_id=? AND active=1")
            .bind(&email)
            .bind(&a.workspace_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| error(StatusCode::NOT_FOUND, "Invite an active member first"))?;
    if a.role != "owner" && target.get::<String, _>("role") == "owner" {
        return Err(error(
            StatusCode::FORBIDDEN,
            "Only owners can reset owner access",
        ));
    }
    let token = random();
    let expires = chrono::Utc::now().timestamp() + 1800;
    sqlx::query("DELETE FROM password_links WHERE email=? OR expires<?")
        .bind(&email)
        .bind(chrono::Utc::now().timestamp())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO password_links VALUES(?,?,?,?)")
        .bind(hash(&token))
        .bind(&email)
        .bind(expires)
        .bind(&a.user_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO audit_log(at,user_id,workspace_id,action,target) VALUES(?,?,?,'password_link_created',?)").bind(chrono::Utc::now().to_rfc3339()).bind(&a.user_id).bind(&a.workspace_id).bind(&email).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(response(
        json!({"url":format!("{}/auth/login#setup={token}",std::env::var("APP_ORIGIN").unwrap_or_default()),"expires_at":expires}),
    ))
}
pub async fn reset_request(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<Email>,
) -> ApiResult<Response> {
    guard(&headers)?;
    if input.email.len() > 254 {
        return Err(error(StatusCode::BAD_REQUEST, "Invalid email"));
    }
    throttle(&app.db, "reset", &input.email.trim().to_lowercase()).await?;
    // No SMTP is configured. Recovery stays administrator-mediated, with no account enumeration.
    Ok(response(
        json!({"message":"Ask your workspace owner or administrator for a new password link. No email has been sent."}),
    ))
}

// ---------------------------------------------------------------------------------------------
// Self-registration for the analyst edition. New accounts get the read-only `viewer` role: they
// can use every analyst view and their own watchlists, but cannot trigger paid AI, X or voice
// requests until an owner raises their role on the Team page. SIGNUP_ENABLED=0 turns it off.

pub fn signup_enabled() -> bool {
    enabled() && std::env::var("SIGNUP_ENABLED").as_deref() != Ok("0")
}
pub const PURPOSES: [&str; 7] = [
    "research",
    "journalism",
    "humanitarian",
    "government",
    "business",
    "education",
    "other",
];
#[derive(Deserialize)]
pub struct Register {
    name: String,
    email: String,
    password: String,
    #[serde(default)]
    organisation: String,
    #[serde(default)]
    purpose: String,
    #[serde(default)]
    accept: bool,
}
pub async fn register(
    State(app): State<App>,
    headers: HeaderMap,
    Json(input): Json<Register>,
) -> ApiResult<Response> {
    guard(&headers)?;
    if !signup_enabled() {
        return Err(error(
            StatusCode::FORBIDDEN,
            "Registration is closed. Ask an administrator for an invitation.",
        ));
    }
    let email = input.email.trim().to_lowercase();
    let name = input.name.trim().to_string();
    let organisation = input
        .organisation
        .trim()
        .chars()
        .take(120)
        .collect::<String>();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(error(StatusCode::BAD_REQUEST, "Enter your name"));
    }
    if !valid_email(&email) {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Enter a valid email address",
        ));
    }
    if input.password.chars().count() < 12 || input.password.len() > 1024 {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Choose a password of at least 12 characters",
        ));
    }
    if !PURPOSES.contains(&input.purpose.as_str()) {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Tell us how you will use the analyst edition",
        ));
    }
    if !input.accept {
        return Err(error(
            StatusCode::BAD_REQUEST,
            "Accept the terms of use to continue",
        ));
    }
    throttle(&app.db, "register", &email).await?;
    let encoded = hash_password(input.password).await?;
    let uid = hash(&format!("password:{email}"));
    let now = chrono::Utc::now().timestamp();
    let mut tx = app.db.begin_with("BEGIN IMMEDIATE").await?;
    let exists: i64 =
        sqlx::query_scalar("SELECT count(*) FROM memberships WHERE workspace_id=? AND email=?")
            .bind(WORKSPACE)
            .bind(&email)
            .fetch_one(&mut *tx)
            .await?;
    if exists > 0 {
        return Err(error(
            StatusCode::CONFLICT,
            "An account with this email already exists. Sign in instead.",
        ));
    }
    sqlx::query("INSERT OR IGNORE INTO auth_users VALUES(?,?,?)")
        .bind(&uid)
        .bind(format!("password:{email}"))
        .bind(&email)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO memberships(workspace_id,email,user_id,role,active) VALUES(?,?,?,'viewer',1)",
    )
    .bind(WORKSPACE)
    .bind(&email)
    .bind(&uid)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO password_credentials VALUES(?,?,?) ON CONFLICT(user_id) DO UPDATE SET password_hash=excluded.password_hash,updated_at=excluded.updated_at").bind(&uid).bind(&encoded).bind(now).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO member_profiles VALUES(?,?,?,?,?) ON CONFLICT(user_id) DO UPDATE SET name=excluded.name,organisation=excluded.organisation,purpose=excluded.purpose")
        .bind(&uid).bind(&name).bind(&organisation).bind(&input.purpose).bind(now).execute(&mut *tx).await?;
    let token = random();
    let csrf = random();
    sqlx::query("INSERT INTO auth_sessions VALUES(?,?,?,?,?)")
        .bind(hash(&token))
        .bind(&uid)
        .bind(WORKSPACE)
        .bind(&csrf)
        .bind(now + 28800)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO audit_log(at,user_id,workspace_id,action,target) VALUES(?,?,?,'self_registration','viewer')").bind(chrono::Utc::now().to_rfc3339()).bind(&uid).bind(WORKSPACE).execute(&mut *tx).await?;
    tx.commit().await?;
    let mut r = response(json!({"redirect":"/#briefing"}));
    r.headers_mut().insert(
        "set-cookie",
        cookie_value("congo_session", &token, 28800)
            .parse()
            .unwrap(),
    );
    Ok(r)
}
