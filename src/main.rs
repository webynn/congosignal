mod actors;
mod agreement_discovery;
mod agreements;
mod ai_settings;
mod armed_groups;
mod arms;
mod authentication;
mod brief;
mod commodities;
mod control;
mod comparisons;
mod costs;
mod events;
mod evidence;
mod extract;
mod feeds;
mod fetch;
mod flights;
mod images;
mod intelligence;
mod notify;
mod password_auth;
mod quality;
mod radio;
mod reference;
mod research_briefs;
mod social;
mod store;
mod territory;
mod text;
mod voice;
mod watch;

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::{net::SocketAddr, time::Duration};

#[derive(Clone)]
struct App {
    db: SqlitePool,
    token: Option<String>,
}
struct ApiError(StatusCode, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error":self.1}))).into_response()
    }
}
impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        tracing::error!(error=%e,"API error");
        Self(StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
    }
}
impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        anyhow::Error::from(e).into()
    }
}
type ApiResult<T> = std::result::Result<T, ApiError>;
#[derive(Deserialize)]
struct Submit {
    url: String,
}
async fn submit(State(app): State<App>, Json(input): Json<Submit>) -> ApiResult<Response> {
    // Feed-only publishers cannot be fetched page by page; say so instead of queueing a failure.
    if let Ok(parsed) = url::Url::parse(&input.url)
        && let Some(answer) = feeds::submitted(&app.db, &parsed).await?
    {
        let code = if answer["status"] == "already_collected" {
            StatusCode::OK
        } else {
            StatusCode::UNPROCESSABLE_ENTITY
        };
        return Ok((code, Json(answer)).into_response());
    }
    let url =
        extract::seed(&input.url).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    let kind = if extract::is_article(&url) {
        "article"
    } else {
        "discovery"
    };
    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM jobs WHERE status IN ('pending','running')")
            .fetch_one(&app.db)
            .await?;
    if pending >= 500 {
        return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "queue full".into()));
    }
    let id = store::enqueue(&app.db, url.as_str(), kind).await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({"job_id":id,"status_url":format!("/v1/jobs/{id}")})),
    )
        .into_response())
}
async fn job(State(app): State<App>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let row = sqlx::query("SELECT * FROM jobs WHERE id=?")
        .bind(id)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "job not found".into()))?;
    Ok(Json(
        json!({"id":row.get::<String,_>("id"),"url":row.get::<String,_>("url"),
        "kind":row.get::<String,_>("kind"),"status":row.get::<String,_>("status"),
        "created_at":row.get::<String,_>("created_at"),"finished_at":row.get::<Option<String>,_>("finished_at"),
        "error":row.get::<Option<String>,_>("error"),
        "result":row.get::<Option<String>,_>("result").map(store::json).transpose()?}),
    ))
}
#[derive(Deserialize)]
struct Search {
    q: Option<String>,
    category: Option<String>,
    since: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}
async fn articles(State(app): State<App>, Query(s): Query<Search>) -> ApiResult<Json<Value>> {
    let limit = s.limit.unwrap_or(20).clamp(1, 100);
    let offset = s.offset.unwrap_or(0).max(0);
    let since = s
        .since
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(&s)
                .map(|d| d.with_timezone(&chrono::Utc).to_rfc3339())
                .map_err(|_| ApiError(StatusCode::BAD_REQUEST, "since must be RFC3339".into()))
        })
        .transpose()?;
    let q = s.q.map(|q| q.to_lowercase());
    let rows: Vec<String> = sqlx::query_scalar(
        r#"SELECT data FROM articles WHERE
        (? IS NULL OR instr(lower(title || ' ' || content), ?) > 0)
        AND (? IS NULL OR category = ?) AND (? IS NULL OR published_at >= ?)
        ORDER BY published_at DESC, id LIMIT ? OFFSET ?"#,
    )
    .bind(&q)
    .bind(&q)
    .bind(&s.category)
    .bind(&s.category)
    .bind(&since)
    .bind(&since)
    .bind(limit)
    .bind(offset)
    .fetch_all(&app.db)
    .await?;
    let items = rows
        .into_iter()
        .map(store::json)
        .collect::<Result<Vec<_>>>()?;
    Ok(Json(json!({"items":items,"limit":limit,"offset":offset})))
}
async fn article(State(app): State<App>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let row = sqlx::query("SELECT data,first_seen_at,last_seen_at FROM articles WHERE id=?")
        .bind(id)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "article not found".into()))?;
    let mut data = store::json(row.get("data"))?;
    data["first_seen_at"] = json!(row.get::<String, _>("first_seen_at"));
    data["last_seen_at"] = json!(row.get::<String, _>("last_seen_at"));
    Ok(Json(data))
}
async fn versions(State(app): State<App>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT data FROM versions WHERE article_id=? ORDER BY fetched_at DESC LIMIT 100",
    )
    .bind(id)
    .fetch_all(&app.db)
    .await?;
    Ok(Json(
        json!({"items":rows.into_iter().map(store::json).collect::<Result<Vec<_>>>()?}),
    ))
}
async fn stats(State(app): State<App>) -> ApiResult<Json<Value>> {
    let rows = sqlx::query(
        "SELECT category,count(*) AS count FROM articles GROUP BY category ORDER BY count DESC",
    )
    .fetch_all(&app.db)
    .await?;
    let jobs = sqlx::query("SELECT status,count(*) AS count FROM jobs GROUP BY status")
        .fetch_all(&app.db)
        .await?;
    Ok(Json(
        json!({"categories":rows.into_iter().map(|r| json!({"category":r.get::<Option<String>,_>("category"),"count":r.get::<i64,_>("count")})).collect::<Vec<_>>(),
        "jobs":jobs.into_iter().map(|r| json!({"status":r.get::<String,_>("status"),"count":r.get::<i64,_>("count")})).collect::<Vec<_>>()}),
    ))
}
async fn process(
    db: &SqlitePool,
    fetcher: &mut fetch::Fetcher,
    url: &str,
    kind: &str,
) -> Result<Value> {
    let (fetched, html) = fetcher.get(&extract::seed(url)?).await?;
    if kind == "article" {
        let article = extract::extract(&html, &fetched)?;
        store::save(db, &article, &html).await?;
        Ok(json!({"article_id":article.id,"url":article.url}))
    } else {
        let links = extract::discover(&html, &fetched, 50);
        let mut ids = Vec::new();
        let mut skipped = 0;
        for link in links {
            if fetched.query_pairs().any(|(k, _)| k == "page") {
                let exists: i64 = sqlx::query_scalar("SELECT count(*) FROM articles WHERE url=?")
                    .bind(&link)
                    .fetch_one(db)
                    .await?;
                if exists > 0 {
                    skipped += 1;
                    continue;
                }
            }
            let pending: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM jobs WHERE status IN ('pending','running')",
            )
            .fetch_one(db)
            .await?;
            if pending >= 500 {
                break;
            }
            ids.push(store::enqueue(db, &link, "article").await?);
        }
        Ok(json!({"article_jobs":ids,"already_collected":skipped}))
    }
}
async fn worker(db: SqlitePool) -> Result<()> {
    let mut fetcher = fetch::Fetcher::new()?;
    // One process owns a database. Interrupted jobs are retried on restart.
    sqlx::query("UPDATE jobs SET status='pending' WHERE status='running'")
        .execute(&db)
        .await?;
    let poll_seconds = std::env::var("POLL_SECONDS")
        .ok()
        .map(|s| s.parse::<u64>())
        .transpose()?;
    anyhow::ensure!(
        poll_seconds.is_none_or(|s| s >= 60),
        "POLL_SECONDS must be at least 60"
    );
    let mut next_poll = tokio::time::Instant::now();
    loop {
        if let Some(seconds) = poll_seconds
            && tokio::time::Instant::now() >= next_poll
        {
            let pending: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM jobs WHERE status IN ('pending','running')",
            )
            .fetch_one(&db)
            .await?;
            if pending < 400 {
                store::enqueue(&db, "https://actualite.cd/", "discovery").await?;
            }
            next_poll = tokio::time::Instant::now() + Duration::from_secs(seconds);
        }
        let row = sqlx::query(
            "SELECT id,url,kind FROM jobs WHERE status='pending' ORDER BY created_at,id LIMIT 1",
        )
        .fetch_optional(&db)
        .await?;
        let Some(row) = row else {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        };
        let id: String = row.get("id");
        sqlx::query("UPDATE jobs SET status='running' WHERE id=?")
            .bind(&id)
            .execute(&db)
            .await?;
        let result = process(
            &db,
            &mut fetcher,
            &row.get::<String, _>("url"),
            &row.get::<String, _>("kind"),
        )
        .await;
        let (status, result, error) = match result {
            Ok(v) => ("completed", Some(v.to_string()), None),
            Err(e) => {
                tracing::warn!(job_id=%id,error=%e,"job failed");
                ("failed", None, Some(e.to_string()))
            }
        };
        sqlx::query("UPDATE jobs SET status=?,result=?,error=?,finished_at=? WHERE id=?")
            .bind(status)
            .bind(result)
            .bind(error)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(id)
            .execute(&db)
            .await?;
    }
}
#[tokio::main]
async fn main() -> Result<()> {
    if std::env::var("LOAD_DOTENV").as_deref() != Ok("0") {
        dotenvy::dotenv().ok();
    }
    ai_settings::init()?;
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let bind: SocketAddr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse()?;
    let token = std::env::var("API_TOKEN").ok().filter(|t| !t.is_empty());
    anyhow::ensure!(
        bind.ip().is_loopback() || authentication::enabled(),
        "Public binding requires AUTH_MODE=oidc and configured ZITADEL"
    );
    let db = store::open(
        &std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/osint.db".into()),
    )
    .await?;
    intelligence::init(&db).await?;
    comparisons::init(&db).await?;
    research_briefs::init(&db).await?;
    authentication::init(&db).await?;
    evidence::init(&db).await?;
    territory::init(&db).await?;
    agreements::init(&db).await?;
    costs::init(&db).await?;
    agreement_discovery::init(&db).await?;
    social::init(&db).await?;
    reference::init(&db).await?;
    actors::init(&db).await?;
    armed_groups::init(&db).await?;
    arms::init(&db).await?;
    images::init(&db).await?;
    commodities::init(&db).await?;
    radio::init(&db).await?;
    flights::init(&db).await?;
    watch::init(&db).await?;
    events::init(&db).await?;
    control::init(&db).await?;
    let app = App {
        db: db.clone(),
        token,
    };
    let router = Router::new()
        .route("/v1/auth/me", get(authentication::me))
        .route("/v1/team/access-link", post(password_auth::access_link))
        .route("/v1/auth/logout", post(authentication::logout))
        .route(
            "/v1/team",
            get(authentication::team).post(authentication::change_member),
        )
        .route("/v1/watches", get(watch::list).post(watch::save))
        .route("/v1/watches/test", post(watch::test))
        .route("/v1/watches/telegram-chats", get(watch::telegram_chats))
        .route("/v1/watches/{id}", axum::routing::delete(watch::remove))
        .route("/v1/watches/{id}/results", get(watch::results))
        .route("/v1/alerts", get(watch::alerts))
        .route("/v1/alerts/read", post(watch::mark_read))
        .route("/v1/events", get(events::list))
        .route("/v1/control", get(control::get).post(control::update))
        .route("/v1/control/dismiss", post(control::dismiss))
        .route("/v1/brief", get(brief::get))
        .route("/v1/jobs", post(submit))
        .route("/v1/jobs/{id}", get(job))
        .route("/v1/articles", get(articles))
        .route("/v1/articles/{id}", get(article))
        .route("/v1/articles/{id}/versions", get(versions))
        .route("/v1/stats", get(stats))
        .route("/v1/config", get(intelligence::config))
        .route("/v1/social", get(social::feed))
        .route("/v1/social/posts", get(social::posts))
        .route("/v1/social/topics/{id}/sample", post(social::sample))
        .route("/v1/social/accounts", get(social::accounts))
        .route("/v1/social/accounts/{handle}", get(social::account))
        .route(
            "/v1/social/accounts/{handle}/history",
            post(social::history),
        )
        .route("/v1/feeds", get(feeds::list))
        .route("/v1/feeds/refresh", post(feeds::refresh))
        .route("/v1/armed-groups", get(armed_groups::list))
        .route("/v1/armed-groups/review", post(armed_groups::review))
        .route(
            "/v1/research-briefs",
            get(research_briefs::list).post(research_briefs::create),
        )
        .route("/v1/research-briefs/{id}", get(research_briefs::get))
        .route("/v1/quality", get(quality::report))
        .route("/v1/actors", get(actors::list))
        .route("/v1/actors/refresh", post(actors::refresh))
        .route("/v1/actors/{id}", get(actors::detail))
        .route("/v1/images", get(images::status))
        .route("/v1/images/{kind}/{id}/reject", post(images::reject))
        .route("/v1/flights", get(flights::list))
        .route(
            "/v1/settings/ai",
            get(ai_settings::get).post(ai_settings::save),
        )
        .route("/v1/settings/ai/check", post(ai_settings::check))
        .route("/v1/voice/status", get(voice::status))
        .route("/v1/voice/options", get(voice::options))
        .route("/v1/voice/chat", post(voice::converse))
        .route("/v1/voice/token", post(voice::token))
        .route("/v1/radio", get(radio::list))
        .route("/v1/radio/{id}/listen", post(radio::listen))
        .route("/v1/commodities", get(commodities::list))
        .route("/v1/geology/unit", get(commodities::geology_unit))
        .route("/v1/arms", get(arms::list))
        .route("/v1/arms/{id}", get(arms::detail))
        .route("/v1/costs", get(costs::status).post(costs::configure))
        .route("/v1/agreements", get(agreements::list))
        .route(
            "/v1/agreements/discovery/refresh",
            post(agreement_discovery::refresh),
        )
        .route("/v1/agreements/{id}", get(agreements::detail))
        .route("/v1/agreements/{id}/analyze", post(agreements::analyze))
        .route("/v1/agreements/{id}/review", post(agreements::review))
        .route(
            "/v1/agreements/{id}/documents/{doc}/pdf",
            get(agreements::pdf),
        )
        .route("/v1/sources", get(evidence::list))
        .route("/v1/sources/{id}/refresh", post(evidence::refresh))
        .route("/v1/evidence", get(evidence::documents))
        .route("/v1/evidence-feed", get(evidence::feed))
        .route("/v1/evidence/{id}/analyze", post(evidence::analyze))
        .route("/v1/evidence/{id}", get(evidence::document))
        .route("/v1/evidence/{id}/pdf", get(evidence::pdf))
        .route("/v1/history-control", get(territory::list))
        .route("/v1/history-control/refresh", post(territory::refresh))
        .route("/v1/history-control/review", post(territory::review))
        .route("/v1/comparisons", get(comparisons::list))
        .route("/v1/comparisons/analyze", post(comparisons::analyze))
        .route("/v1/intelligence", get(intelligence::feed))
        .route("/v1/intelligence/status", get(intelligence::status))
        .route("/v1/intelligence/analyze", post(intelligence::analyze))
        .route("/v1/intelligence/{id}", get(intelligence::detail))
        .route("/v1/intelligence/{id}/notes", post(intelligence::save_note))
        .route_layer(middleware::from_fn_with_state(
            app.clone(),
            authentication::middleware,
        ))
        .route("/auth/config", get(authentication::config))
        .route("/auth/login", get(authentication::login))
        .route("/auth/callback", get(authentication::callback))
        .route("/auth/password/login", post(password_auth::login))
        .route("/auth/password/setup", post(password_auth::setup))
        .route(
            "/auth/password/reset-request",
            post(password_auth::reset_request),
        )
        .route("/auth/password.js", get(password_auth::script))
        .route("/ready", get(quality::ready))
        .with_state(app)
        .route("/health", get(|| async { Json(json!({"status":"ok"})) }))
        .route(
            "/",
            get(|| async { axum::response::Html(include_str!("../dashboard/index.html")) }),
        )
        .route(
            "/voice.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../dashboard/voice.js"),
                )
            }),
        )
        .route(
            "/app.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../dashboard/app.js"),
                )
            }),
        )
        .route(
            "/styles.css",
            get(|| async {
                (
                    [("content-type", "text/css; charset=utf-8")],
                    include_str!("../dashboard/styles.css"),
                )
            }),
        )
        .route(
            "/v1/geology/sites",
            get(|| async {
                (
                    [("content-type", "application/geo+json")],
                    include_str!("../dashboard/geology/sites.geo.json"),
                )
            }),
        )
        .route(
            "/geology/congo-1931.jpg",
            get(|| async {
                (
                    [("content-type", "image/jpeg")],
                    include_bytes!("../dashboard/geology/congo-1931.jpg").as_slice(),
                )
            }),
        )
        .route(
            "/drc.geo.json",
            get(|| async {
                (
                    [("content-type", "application/json")],
                    include_str!("../dashboard/drc.geo.json"),
                )
            }),
        )
        .route(
            "/geo/territories.geo.json",
            get(|| async {
                (
                    [("content-type", "application/json"), ("cache-control", "public, max-age=86400")],
                    include_str!("../dashboard/geo/territories.geo.json"),
                )
            }),
        )
        .route(
            "/geology/provinces.geo.json",
            get(|| async {
                (
                    [("content-type", "application/json")],
                    include_str!("../dashboard/geology/provinces.geo.json"),
                )
            }),
        )
        // gzip/brotli for JSON and the dashboard: the report list alone is ~1 MB uncompressed.
        .layer(tower_http::compression::CompressionLayer::new());
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind,"API worker listening");
    tokio::select! {
        result = worker(db.clone()) => result?,
        result = intelligence::worker(db.clone()) => result?,
        result = evidence::worker(db.clone()) => result?,
        result = territory::worker(db.clone()) => result?,
        result = agreement_discovery::worker(db.clone()) => result?,
        result = social::worker(db.clone()) => result?,
        result = actors::worker(db.clone()) => result?,
        result = arms::worker(db.clone()) => result?,
        result = feeds::worker(db.clone()) => result?,
        result = images::worker(db.clone()) => result?,
        result = commodities::worker(db.clone()) => result?,
        result = flights::worker(db.clone()) => result?,
        result = watch::worker(db.clone()) => result?,
        result = events::worker(db.clone()) => result?,
        result = radio::worker(db) => result?,
        result = axum::serve(listener,router).with_graceful_shutdown(shutdown_signal()) => result?,
    }
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("SIGTERM handler");
        tokio::select! { _=tokio::signal::ctrl_c()=>{}, _=term.recv()=>{} }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
    tracing::info!("Graceful shutdown requested");
}
