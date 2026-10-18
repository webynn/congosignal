//! Invite-only OIDC sessions. App roles are read from SQLite on every request.
use crate::{ApiError, ApiResult, App};
use anyhow::{Result, ensure};
use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use std::sync::OnceLock;
const WORKSPACE: &str = "congo-signal";
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Actor {
    pub user_id: String,
    pub workspace_id: String,
    pub role: String,
    pub email: String,
}
tokio::task_local! {pub static ACTOR:Actor;}
pub fn actor() -> Actor {
    ACTOR.try_with(Clone::clone).unwrap_or(Actor {
        user_id: "worker".into(),
        workspace_id: WORKSPACE.into(),
        role: "worker".into(),
        email: String::new(),
    })
}
pub fn enabled() -> bool {
    std::env::var("AUTH_MODE").as_deref() == Ok("oidc")
}
#[derive(Clone)]
struct Oidc {
    issuer: String,
    client_id: String,
    origin: String,
    authorize: String,
    token: String,
    jwks: String,
}
static OIDC: OnceLock<Oidc> = OnceLock::new();
fn random() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
fn hash(s: &str) -> String {
    crate::extract::digest(s.as_bytes())
}
fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(20))
        .build()?)
}
async fn read_json(mut r: reqwest::Response) -> Result<Value> {
    ensure!(
        r.status().is_success(),
        "Identity provider returned HTTP {}",
        r.status()
    );
    let mut b = Vec::new();
    while let Some(c) = r.chunk().await? {
        ensure!(
            b.len() + c.len() <= 1_000_000,
            "Identity response too large"
        );
        b.extend_from_slice(&c);
    }
    Ok(serde_json::from_slice(&b)?)
}
fn endpoint(s: &str) -> Result<String> {
    let u = url::Url::parse(s)?;
    ensure!(
        u.scheme() == "https" && u.username().is_empty() && u.password().is_none(),
        "Identity endpoints require HTTPS"
    );
    Ok(s.into())
}
pub async fn init(db: &SqlitePool) -> Result<()> {
    ensure!(
        ["local", "oidc"].contains(
            &std::env::var("AUTH_MODE")
                .unwrap_or_else(|_| "local".into())
                .as_str()
        ),
        "AUTH_MODE must be local or oidc"
    );
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS workspaces(id TEXT PRIMARY KEY,name TEXT NOT NULL);INSERT OR IGNORE INTO workspaces VALUES('congo-signal','Congo Signal');
 CREATE TABLE IF NOT EXISTS auth_users(id TEXT PRIMARY KEY,subject TEXT UNIQUE NOT NULL,email TEXT NOT NULL);
 CREATE TABLE IF NOT EXISTS memberships(workspace_id TEXT NOT NULL,email TEXT NOT NULL,user_id TEXT,role TEXT NOT NULL,active INTEGER NOT NULL DEFAULT 1,daily_requests INTEGER NOT NULL DEFAULT 0,daily_micros INTEGER NOT NULL DEFAULT 0,monthly_micros INTEGER NOT NULL DEFAULT 0,PRIMARY KEY(workspace_id,email));
 CREATE UNIQUE INDEX IF NOT EXISTS membership_identity ON memberships(workspace_id,user_id) WHERE user_id IS NOT NULL;
 CREATE TABLE IF NOT EXISTS auth_sessions(hash TEXT PRIMARY KEY,user_id TEXT NOT NULL,workspace_id TEXT NOT NULL,csrf TEXT NOT NULL,expires INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS auth_states(hash TEXT PRIMARY KEY,verifier TEXT NOT NULL,nonce TEXT NOT NULL,expires INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS audit_log(id INTEGER PRIMARY KEY AUTOINCREMENT,at TEXT NOT NULL,user_id TEXT NOT NULL,workspace_id TEXT NOT NULL,action TEXT NOT NULL,target TEXT NOT NULL);
 CREATE TABLE IF NOT EXISTS private_notes(workspace_id TEXT NOT NULL,user_id TEXT NOT NULL,article_id TEXT NOT NULL,saved INTEGER NOT NULL,review TEXT NOT NULL,note TEXT NOT NULL,updated_at TEXT NOT NULL,PRIMARY KEY(workspace_id,user_id,article_id));").execute(db).await?;
    // Existing shared notes are preserved in their original table until initial owner migration.
    for (email, id, role) in [
        ("local-owner", "local-owner", "owner"),
        ("worker", "worker", "worker"),
    ] {
        sqlx::query(
            "INSERT OR IGNORE INTO memberships(workspace_id,email,user_id,role) VALUES(?,?,?,?)",
        )
        .bind(WORKSPACE)
        .bind(email)
        .bind(id)
        .bind(role)
        .execute(db)
        .await?;
    }
    let has_notes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='analyst_notes'",
    )
    .fetch_one(db)
    .await?;
    if has_notes > 0 {
        sqlx::query("INSERT OR IGNORE INTO private_notes SELECT 'congo-signal','local-owner',article_id,saved,review,note,updated_at FROM analyst_notes").execute(db).await?;
    }
    if enabled() {
        let issuer = endpoint(std::env::var("OIDC_ISSUER")?.trim_end_matches('/'))?;
        let client_id = std::env::var("OIDC_CLIENT_ID")?;
        ensure!(!client_id.is_empty(), "OIDC client ID required");
        let origin = std::env::var("APP_ORIGIN")?
            .trim_end_matches('/')
            .to_string();
        let u = url::Url::parse(&origin)?;
        ensure!(
            u.origin().ascii_serialization() == origin
                && (u.scheme() == "https"
                    || (u.scheme() == "http"
                        && [Some("localhost"), Some("127.0.0.1")].contains(&u.host_str()))),
            "APP_ORIGIN must be an HTTPS origin (HTTP allowed only for local development)"
        );
        let d = read_json(
            client()?
                .get(format!("{issuer}/.well-known/openid-configuration"))
                .send()
                .await?,
        )
        .await?;
        ensure!(d["issuer"] == issuer, "Discovery issuer mismatch");
        let c = Oidc {
            issuer,
            client_id,
            origin,
            authorize: endpoint(d["authorization_endpoint"].as_str().unwrap_or(""))?,
            token: endpoint(d["token_endpoint"].as_str().unwrap_or(""))?,
            jwks: endpoint(d["jwks_uri"].as_str().unwrap_or(""))?,
        };
        let _ = OIDC.set(c);
        let owner = std::env::var("AUTH_OWNER_EMAIL")?.trim().to_lowercase();
        ensure!(
            valid_email(&owner),
            "AUTH_OWNER_EMAIL required for bootstrap"
        );
        sqlx::query("INSERT INTO memberships(workspace_id,email,role) SELECT ?,?,'owner' WHERE NOT EXISTS(SELECT 1 FROM memberships WHERE workspace_id=? AND role='owner' AND email!='local-owner')").bind(WORKSPACE).bind(owner).bind(WORKSPACE).execute(db).await?;
    }
    Ok(())
}
fn cookie(headers: &HeaderMap, key: &str) -> Option<String> {
    headers
        .get("cookie")?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|v| {
            let (k, v) = v.trim().split_once('=')?;
            (k == key).then(|| v.to_string())
        })
}
fn cookie_value(name: &str, value: &str, age: u32) -> String {
    let secure = OIDC.get().is_some_and(|c| c.origin.starts_with("https://"));
    format!(
        "{name}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={age}{}",
        if secure { "; Secure" } else { "" }
    )
}
fn denied(code: StatusCode, msg: &str) -> Response {
    ApiError(code, msg.into()).into_response()
}
fn permit(role: &str, method: &str, path: &str) -> bool {
    if path == "/v1/auth/me" || path == "/v1/auth/logout" {
        return true;
    }
    if path.starts_with("/v1/team") {
        return ["owner", "administrator"].contains(&role);
    }
    if path == "/v1/costs" || path.starts_with("/v1/settings/ai") {
        return role == "owner";
    }
    if method == "GET" {
        return true;
    }
    if role == "owner" {
        return true;
    }
    if path == "/v1/voice/token" || path == "/v1/voice/chat" {
        return ["administrator", "analyst", "reviewer"].contains(&role);
    }
    if path.ends_with("/notes")
        || path.ends_with("/analyze")
        || path.ends_with("/history")
        || path.ends_with("/sample")
    {
        return ["administrator", "analyst", "reviewer"].contains(&role);
    }
    if path.ends_with("/review") {
        return ["administrator", "reviewer"].contains(&role);
    }
    if path.ends_with("/refresh") || path == "/v1/jobs" || path.ends_with("/reject") {
        return role == "administrator" || role == "worker";
    }
    false
}
async fn session(db: &SqlitePool, h: &HeaderMap) -> Result<Option<(Actor, String)>> {
    let Some(t) = cookie(h, "congo_session") else {
        return Ok(None);
    };
    let r=sqlx::query("SELECT s.user_id,s.workspace_id,s.csrf,m.role,m.email FROM auth_sessions s JOIN memberships m ON m.user_id=s.user_id AND m.workspace_id=s.workspace_id WHERE s.hash=? AND s.expires>? AND m.active=1").bind(hash(&t)).bind(chrono::Utc::now().timestamp()).fetch_optional(db).await?;
    Ok(r.map(|r| {
        (
            Actor {
                user_id: r.get("user_id"),
                workspace_id: r.get("workspace_id"),
                role: r.get("role"),
                email: r.get("email"),
            },
            r.get("csrf"),
        )
    }))
}
fn valid_csrf(headers: &HeaderMap, origin: &str, csrf: &str) -> bool {
    !csrf.is_empty()
        && !origin.is_empty()
        && headers.get("origin").and_then(|v| v.to_str().ok()) == Some(origin)
        && headers.get("x-csrf-token").and_then(|v| v.to_str().ok()) == Some(csrf)
}
pub async fn middleware(
    State(app): State<App>,
    req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let (a, csrf) = if enabled() {
        match session(&app.db, req.headers()).await {
            Ok(Some(v)) => v,
            Ok(None) => return denied(StatusCode::UNAUTHORIZED, "Sign in required"),
            Err(_) => return denied(StatusCode::INTERNAL_SERVER_ERROR, "Session lookup failed"),
        }
    } else {
        if let Some(token) = &app.token {
            if req
                .headers()
                .get("authorization")
                .and_then(|s| s.to_str().ok())
                != Some(format!("Bearer {token}").as_str())
            {
                return denied(StatusCode::UNAUTHORIZED, "bearer token required");
            }
        }
        (
            Actor {
                user_id: "local-owner".into(),
                workspace_id: WORKSPACE.into(),
                role: "owner".into(),
                email: "local-owner".into(),
            },
            String::new(),
        )
    };
    if !permit(&a.role, req.method().as_str(), req.uri().path()) {
        return denied(
            StatusCode::FORBIDDEN,
            "Your workspace role does not permit this action",
        );
    }
    let mutation = req.method() != axum::http::Method::GET;
    if enabled() && mutation {
        let origin = OIDC.get().map(|c| c.origin.as_str()).unwrap_or("");
        if !valid_csrf(req.headers(), origin, &csrf) {
            return denied(StatusCode::FORBIDDEN, "Invalid origin or CSRF token");
        }
    }
    let path = req.uri().path().to_string();
    let audit_actor = a.clone();
    let response = ACTOR.scope(a, next.run(req)).await;
    if mutation && response.status().is_success() {
        if let Err(e) = audit(&app.db, &audit_actor, "api_mutation", &path).await {
            tracing::error!(error=%e,"Failed to persist mutation audit");
        }
    }
    let mut response = response;
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response
}
async fn audit(db: &SqlitePool, a: &Actor, action: &str, target: &str) -> Result<()> {
    sqlx::query("INSERT INTO audit_log(at,user_id,workspace_id,action,target) VALUES(?,?,?,?,?)")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&a.user_id)
        .bind(&a.workspace_id)
        .bind(action)
        .bind(target)
        .execute(db)
        .await?;
    Ok(())
}
pub async fn config() -> Json<Value> {
    Json(
        json!({"mode":if enabled(){"oidc"}else{"local"},"configured":OIDC.get().is_some(),"login_url":"/auth/login"}),
    )
}
pub async fn me(State(app): State<App>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let csrf = if enabled() {
        session(&app.db, &headers).await?.map(|x| x.1)
    } else {
        None
    };
    Ok(Json(
        json!({"user":actor(),"csrf":csrf,"mode":if enabled(){"oidc"}else{"local"}}),
    ))
}
pub async fn login(State(app): State<App>) -> ApiResult<Response> {
    let c = OIDC.get().ok_or_else(|| {
        ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "ZITADEL is not configured".into(),
        )
    })?;
    let state = random();
    let verifier = random();
    let nonce = random();
    let now = chrono::Utc::now().timestamp();
    sqlx::query("DELETE FROM auth_states WHERE expires<?")
        .bind(now)
        .execute(&app.db)
        .await?;
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM auth_states")
        .fetch_one(&app.db)
        .await?;
    if n >= 1000 {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many login attempts".into(),
        ));
    }
    sqlx::query("INSERT INTO auth_states VALUES(?,?,?,?)")
        .bind(hash(&state))
        .bind(&verifier)
        .bind(&nonce)
        .bind(now + 600)
        .execute(&app.db)
        .await?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut u = url::Url::parse(&c.authorize).map_err(anyhow::Error::from)?;
    u.query_pairs_mut().extend_pairs([
        ("client_id", c.client_id.as_str()),
        ("redirect_uri", &format!("{}/auth/callback", c.origin)),
        ("response_type", "code"),
        ("scope", "openid email profile"),
        ("state", &state),
        ("nonce", &nonce),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
    ]);
    let mut r = Redirect::to(u.as_str()).into_response();
    r.headers_mut().insert(
        "set-cookie",
        cookie_value("congo_login", &state, 600).parse().unwrap(),
    );
    r.headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    Ok(r)
}
#[derive(Deserialize)]
pub struct Callback {
    state: String,
    code: String,
}
#[derive(Clone, Deserialize)]
struct Claims {
    sub: String,
    email: String,
    email_verified: bool,
    nonce: String,
    azp: Option<String>,
    aud: Value,
}
fn verify_identity(
    raw: &str,
    key: &jsonwebtoken::DecodingKey,
    issuer: &str,
    client_id: &str,
    nonce: &str,
) -> Result<Claims> {
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.set_audience(&[client_id]);
    validation.set_issuer(&[issuer]);
    validation.validate_nbf = true;
    validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    let claims = jsonwebtoken::decode::<Claims>(raw, key, &validation)?.claims;
    ensure!(
        claims.nonce == nonce && claims.email_verified && !claims.sub.is_empty(),
        "Unverified identity"
    );
    ensure!(
        claims.azp.as_deref().is_none_or(|v| v == client_id)
            && (!claims.aud.as_array().is_some_and(|a| a.len() > 1)
                || claims.azp.as_deref() == Some(client_id)),
        "Authorized party mismatch"
    );
    Ok(claims)
}
pub async fn callback(
    State(app): State<App>,
    headers: HeaderMap,
    Query(q): Query<Callback>,
) -> ApiResult<Response> {
    let c = OIDC
        .get()
        .ok_or_else(|| ApiError(StatusCode::SERVICE_UNAVAILABLE, "Login unavailable".into()))?;
    if cookie(&headers, "congo_login").as_deref() != Some(q.state.as_str()) {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Login state mismatch".into(),
        ));
    }
    let r =
        sqlx::query("DELETE FROM auth_states WHERE hash=? AND expires>? RETURNING verifier,nonce")
            .bind(hash(&q.state))
            .bind(chrono::Utc::now().timestamp())
            .fetch_optional(&app.db)
            .await?
            .ok_or_else(|| {
                ApiError(
                    StatusCode::BAD_REQUEST,
                    "Login expired or already used".into(),
                )
            })?;
    let verifier: String = r.get("verifier");
    let nonce: String = r.get("nonce");
    let token = read_json(
        client()?
            .post(&c.token)
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", &c.client_id),
                ("redirect_uri", &format!("{}/auth/callback", c.origin)),
                ("code", &q.code),
                ("code_verifier", &verifier),
            ])
            .send()
            .await
            .map_err(anyhow::Error::from)?,
    )
    .await?;
    let raw = token["id_token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing ID token"))?;
    let header = jsonwebtoken::decode_header(raw).map_err(anyhow::Error::from)?;
    if !(header.alg == jsonwebtoken::Algorithm::RS256) {
        return Err(ApiError(
            StatusCode::UNAUTHORIZED,
            "Unsupported identity signature".into(),
        ));
    }
    let jwks: jsonwebtoken::jwk::JwkSet = serde_json::from_value(
        read_json(
            client()?
                .get(&c.jwks)
                .send()
                .await
                .map_err(anyhow::Error::from)?,
        )
        .await?,
    )
    .map_err(anyhow::Error::from)?;
    let key = jwks
        .find(header.kid.as_deref().unwrap_or(""))
        .ok_or_else(|| anyhow::anyhow!("Unknown identity key"))?;
    let claims = verify_identity(
        raw,
        &jsonwebtoken::DecodingKey::from_jwk(key).map_err(anyhow::Error::from)?,
        &c.issuer,
        &c.client_id,
        &nonce,
    )
    .map_err(|_| ApiError(StatusCode::UNAUTHORIZED, "Invalid identity token".into()))?;
    let email = claims.email.trim().to_lowercase();
    let subject = format!("{}|{}", c.issuer, claims.sub);
    let uid = hash(&subject);
    let mut tx = app.db.begin().await?;
    let member = sqlx::query(
        "SELECT role,user_id FROM memberships WHERE workspace_id=? AND email=? AND active=1",
    )
    .bind(WORKSPACE)
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        ApiError(
            StatusCode::FORBIDDEN,
            "This email has not been invited to Congo Signal".into(),
        )
    })?;
    if !(member
        .get::<Option<String>, _>("user_id")
        .is_none_or(|v| v == uid))
    {
        return Err(ApiError(
            StatusCode::UNAUTHORIZED,
            "Invitation is bound to another identity".into(),
        ));
    }
    sqlx::query(
        "INSERT INTO auth_users VALUES(?,?,?) ON CONFLICT(id) DO UPDATE SET email=excluded.email",
    )
    .bind(&uid)
    .bind(subject)
    .bind(&email)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE memberships SET user_id=? WHERE workspace_id=? AND email=?")
        .bind(&uid)
        .bind(WORKSPACE)
        .bind(&email)
        .execute(&mut *tx)
        .await?;
    if std::env::var("AUTH_OWNER_EMAIL")
        .ok()
        .is_some_and(|e| e.trim().eq_ignore_ascii_case(&email))
    {
        sqlx::query("INSERT OR IGNORE INTO private_notes SELECT ?,?,article_id,saved,review,note,updated_at FROM private_notes WHERE workspace_id=? AND user_id='local-owner'").bind(WORKSPACE).bind(&uid).bind(WORKSPACE).execute(&mut *tx).await?;
        sqlx::query("INSERT OR IGNORE INTO private_notes SELECT ?,?,article_id,saved,review,note,updated_at FROM analyst_notes").bind(WORKSPACE).bind(&uid).execute(&mut *tx).await?;
    }
    let session = random();
    let csrf = random();
    sqlx::query("INSERT INTO auth_sessions VALUES(?,?,?,?,?)")
        .bind(hash(&session))
        .bind(&uid)
        .bind(WORKSPACE)
        .bind(csrf)
        .bind(chrono::Utc::now().timestamp() + 28800)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let a = Actor {
        user_id: uid,
        workspace_id: WORKSPACE.into(),
        role: member.get("role"),
        email,
    };
    audit(&app.db, &a, "login", "session").await?;
    let mut response = Redirect::to("/").into_response();
    response.headers_mut().append(
        "set-cookie",
        cookie_value("congo_session", &session, 28800)
            .parse()
            .unwrap(),
    );
    response.headers_mut().append(
        "set-cookie",
        cookie_value("congo_login", "", 0).parse().unwrap(),
    );
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert("referrer-policy", "no-referrer".parse().unwrap());
    Ok(response)
}
pub async fn logout(State(app): State<App>, h: HeaderMap) -> ApiResult<Response> {
    if let Some(t) = cookie(&h, "congo_session") {
        sqlx::query("DELETE FROM auth_sessions WHERE hash=?")
            .bind(hash(&t))
            .execute(&app.db)
            .await?;
    }
    let mut r = Json(json!({"logged_out":true})).into_response();
    r.headers_mut().insert(
        "set-cookie",
        cookie_value("congo_session", "", 0).parse().unwrap(),
    );
    Ok(r)
}
fn valid_email(s: &str) -> bool {
    s.len() < 254 && s.contains('@') && !s.chars().any(char::is_whitespace)
}
#[derive(Deserialize)]
pub struct Member {
    email: String,
    role: String,
    active: bool,
    daily_requests: i64,
    daily_usd: f64,
    monthly_usd: f64,
}
pub async fn change_member(
    State(app): State<App>,
    Json(m): Json<Member>,
) -> ApiResult<Json<Value>> {
    let a = actor();
    let email = m.email.trim().to_lowercase();
    if !(valid_email(&email) || email == "worker")
        || ![
            "owner",
            "administrator",
            "analyst",
            "reviewer",
            "viewer",
            "worker",
        ]
        .contains(&m.role.as_str())
        || (email == "worker") != (m.role == "worker")
        || !(0..=100000).contains(&m.daily_requests)
        || [m.daily_usd, m.monthly_usd]
            .iter()
            .any(|v| !v.is_finite() || *v < 0. || *v > 100000.)
    {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Invalid member settings".into(),
        ));
    }
    let mut tx = app.db.begin_with("BEGIN IMMEDIATE").await?;
    let old=sqlx::query("SELECT role,daily_requests,daily_micros,monthly_micros FROM memberships WHERE workspace_id=? AND email=?").bind(&a.workspace_id).bind(&email).fetch_optional(&mut *tx).await?;
    if a.role != "owner"
        && (m.role == "owner"
            || old
                .as_ref()
                .is_some_and(|r| r.get::<String, _>("role") == "owner")
            || m.daily_requests != old.as_ref().map_or(0, |r| r.get("daily_requests"))
            || (m.daily_usd * 1e6).round() as i64
                != old.as_ref().map_or(0, |r| r.get("daily_micros"))
            || (m.monthly_usd * 1e6).round() as i64
                != old.as_ref().map_or(0, |r| r.get("monthly_micros")))
    {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "Only owners can manage owners or spending allowances".into(),
        ));
    }
    if old
        .as_ref()
        .is_some_and(|r| r.get::<String, _>("role") == "owner")
        && (!m.active || m.role != "owner")
    {
        let n:i64=sqlx::query_scalar("SELECT count(*) FROM memberships WHERE workspace_id=? AND role='owner' AND active=1 AND email!='local-owner'").bind(&a.workspace_id).fetch_one(&mut *tx).await?;
        if n <= 1 {
            return Err(ApiError(
                StatusCode::CONFLICT,
                "Cannot remove the last owner".into(),
            ));
        }
    }
    sqlx::query("INSERT INTO memberships(workspace_id,email,role,active,daily_requests,daily_micros,monthly_micros) VALUES(?,?,?,?,?,?,?) ON CONFLICT(workspace_id,email) DO UPDATE SET role=excluded.role,active=excluded.active,daily_requests=excluded.daily_requests,daily_micros=excluded.daily_micros,monthly_micros=excluded.monthly_micros").bind(&a.workspace_id).bind(&email).bind(m.role).bind(m.active).bind(m.daily_requests).bind((m.daily_usd*1e6).round() as i64).bind((m.monthly_usd*1e6).round() as i64).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM auth_sessions WHERE workspace_id=? AND user_id IN (SELECT user_id FROM memberships WHERE workspace_id=? AND email=?)").bind(&a.workspace_id).bind(&a.workspace_id).bind(&email).execute(&mut *tx).await?;
    tx.commit().await?;
    audit(&app.db, &a, "member_changed", &email).await?;
    team(State(app)).await
}
pub async fn team(State(app): State<App>) -> ApiResult<Json<Value>> {
    let a = actor();
    let mut members = Vec::new();
    for r in sqlx::query(
        "SELECT * FROM memberships WHERE workspace_id=? AND email!='local-owner' ORDER BY email",
    )
    .bind(&a.workspace_id)
    .fetch_all(&app.db)
    .await?
    {
        members.push(json!({"email":r.get::<String,_>("email"),"user_id":r.get::<Option<String>,_>("user_id"),"role":r.get::<String,_>("role"),"active":r.get::<i64,_>("active")==1,"daily_requests":r.get::<i64,_>("daily_requests"),"daily_usd":r.get::<i64,_>("daily_micros") as f64/1e6,"monthly_usd":r.get::<i64,_>("monthly_micros") as f64/1e6}));
    }
    let mut audit = Vec::new();
    for r in sqlx::query("SELECT * FROM audit_log WHERE workspace_id=? ORDER BY id DESC LIMIT 100")
        .bind(&a.workspace_id)
        .fetch_all(&app.db)
        .await?
    {
        audit.push(json!({"at":r.get::<String,_>("at"),"user_id":r.get::<String,_>("user_id"),"action":r.get::<String,_>("action"),"target":r.get::<String,_>("target")}));
    }
    Ok(Json(
        json!({"workspace_id":a.workspace_id,"members":members,"audit":audit}),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csrf_requires_both_expected_origin_and_session_token() {
        let mut h = HeaderMap::new();
        assert!(!valid_csrf(&h, "https://app.example", "token"));
        h.insert("origin", "https://evil.example".parse().unwrap());
        h.insert("x-csrf-token", "token".parse().unwrap());
        assert!(!valid_csrf(&h, "https://app.example", "token"));
        h.insert("origin", "https://app.example".parse().unwrap());
        assert!(valid_csrf(&h, "https://app.example", "token"));
        assert!(!valid_csrf(&h, "https://app.example", "another-session"));
    }
    #[test]
    fn identity_tokens_require_signature_issuer_audience_nonce_and_verified_email() {
        let key = jsonwebtoken::EncodingKey::from_rsa_pem(include_bytes!(
            "../tests/fixtures/oidc-test-private.pem"
        ))
        .unwrap();
        let public = jsonwebtoken::DecodingKey::from_rsa_pem(include_bytes!(
            "../tests/fixtures/oidc-test-public.pem"
        ))
        .unwrap();
        let claims = json!({"sub":"test","iss":"https://identity.example","aud":"client","exp":chrono::Utc::now().timestamp()+600,"nonce":"nonce","email":"test@example.org","email_verified":true});
        let sign = |c: &Value| {
            jsonwebtoken::encode(
                &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256),
                c,
                &key,
            )
            .unwrap()
        };
        assert!(
            verify_identity(
                &sign(&claims),
                &public,
                "https://identity.example",
                "client",
                "nonce"
            )
            .is_ok()
        );
        for (field, value) in [
            ("iss", json!("https://evil.example")),
            ("aud", json!("other")),
            ("nonce", json!("other")),
            ("email_verified", json!(false)),
            ("exp", json!(0)),
            ("azp", json!("other")),
        ] {
            let mut bad = claims.clone();
            bad[field] = value;
            assert!(
                verify_identity(
                    &sign(&bad),
                    &public,
                    "https://identity.example",
                    "client",
                    "nonce"
                )
                .is_err(),
                "{field}"
            );
        }
        let mut tampered = sign(&claims);
        tampered.push('x');
        assert!(
            verify_identity(
                &tampered,
                &public,
                "https://identity.example",
                "client",
                "nonce"
            )
            .is_err()
        );
    }
    #[tokio::test]
    async fn last_owner_and_privilege_escalation_are_blocked() {
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        sqlx::query("INSERT INTO memberships(workspace_id,email,user_id,role) VALUES('congo-signal','owner@example.org','o','owner')").execute(&db).await.unwrap();
        let app = App {
            db: db.clone(),
            token: None,
        };
        let owner = Actor {
            user_id: "o".into(),
            workspace_id: WORKSPACE.into(),
            role: "owner".into(),
            email: "owner@example.org".into(),
        };
        let input = Member {
            email: "owner@example.org".into(),
            role: "viewer".into(),
            active: true,
            daily_requests: 0,
            daily_usd: 0.,
            monthly_usd: 0.,
        };
        assert!(
            ACTOR
                .scope(
                    owner.clone(),
                    change_member(State(app.clone()), Json(input))
                )
                .await
                .is_err()
        );
        let admin = Actor {
            role: "administrator".into(),
            ..owner
        };
        let input = Member {
            email: "new@example.org".into(),
            role: "owner".into(),
            active: true,
            daily_requests: 0,
            daily_usd: 0.,
            monthly_usd: 0.,
        };
        assert!(
            ACTOR
                .scope(admin, change_member(State(app), Json(input)))
                .await
                .is_err()
        );
    }
    #[test]
    fn roles() {
        for role in ["administrator", "analyst", "reviewer", "viewer", "worker"] {
            assert!(!permit(role, "GET", "/v1/settings/ai"));
            assert!(!permit(role, "POST", "/v1/settings/ai"));
            assert!(!permit(role, "POST", "/v1/settings/ai/check"));
        }
        assert!(permit("owner", "POST", "/v1/settings/ai"));

        assert!(!permit("viewer", "POST", "/v1/intelligence/analyze"));
        assert!(permit("analyst", "POST", "/v1/intelligence/analyze"));
        assert!(!permit("analyst", "POST", "/v1/agreements/x/review"));
        assert!(permit("reviewer", "POST", "/v1/agreements/x/review"));
        assert!(!permit("administrator", "POST", "/v1/costs"));
        assert!(!permit("analyst", "GET", "/v1/team"));
        assert!(!permit("viewer", "POST", "/v1/unknown"));
        assert!(!permit("viewer", "POST", "/v1/voice/token"));
        assert!(permit("analyst", "POST", "/v1/voice/token"));
    }
    #[tokio::test]
    async fn expired_and_revoked_sessions() {
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        sqlx::query("INSERT INTO auth_sessions VALUES(?,'local-owner','congo-signal','csrf',?)")
            .bind(hash("test"))
            .bind(chrono::Utc::now().timestamp() + 100)
            .execute(&db)
            .await
            .unwrap();
        let mut h = HeaderMap::new();
        h.insert("cookie", "congo_session=test".parse().unwrap());
        assert!(session(&db, &h).await.unwrap().is_some());
        sqlx::query("UPDATE memberships SET active=0")
            .execute(&db)
            .await
            .unwrap();
        assert!(session(&db, &h).await.unwrap().is_none());
        sqlx::query("UPDATE memberships SET active=1")
            .execute(&db)
            .await
            .unwrap();
        sqlx::query("UPDATE auth_sessions SET expires=0")
            .execute(&db)
            .await
            .unwrap();
        assert!(session(&db, &h).await.unwrap().is_none());
    }
}
