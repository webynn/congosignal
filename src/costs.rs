//! Persistent admission control. Allocations are planning estimates, not provider invoices.
use crate::{ApiResult, App};
use anyhow::{Result, ensure};
use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::sync::OnceLock;
static DB: OnceLock<SqlitePool> = OnceLock::new();
pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS cost_limits(provider TEXT PRIMARY KEY,paused INTEGER NOT NULL DEFAULT 1,daily_requests INTEGER NOT NULL DEFAULT 0,daily_micros INTEGER NOT NULL DEFAULT 0,monthly_micros INTEGER NOT NULL DEFAULT 0,request_micros INTEGER NOT NULL DEFAULT 0);CREATE TABLE IF NOT EXISTS cost_events(id INTEGER PRIMARY KEY AUTOINCREMENT,provider TEXT NOT NULL,operation TEXT NOT NULL,created_at TEXT NOT NULL,day TEXT NOT NULL,month TEXT NOT NULL,allocated_micros INTEGER NOT NULL,status TEXT NOT NULL DEFAULT 'reserved_or_uncertain',usage TEXT); CREATE INDEX IF NOT EXISTS cost_events_period ON cost_events(provider,day,month);").execute(db).await?;
    let cols = sqlx::query("PRAGMA table_info(cost_events)")
        .fetch_all(db)
        .await?;
    for field in ["user_id", "workspace_id"] {
        if !cols.iter().any(|r| r.get::<String, _>("name") == field) {
            sqlx::query(&format!("ALTER TABLE cost_events ADD COLUMN {field} TEXT NOT NULL DEFAULT 'legacy-unattributed'")).execute(db).await?;
        }
    }
    for p in ["claude", "openai", "x"] {
        sqlx::query("INSERT OR IGNORE INTO cost_limits(provider) VALUES(?)")
            .bind(p)
            .execute(db)
            .await?;
    }
    let _ = DB.set(db.clone());
    Ok(())
}
pub async fn reserve(provider: &str, operation: &str) -> Result<i64> {
    reserve_db(
        DB.get()
            .ok_or_else(|| anyhow::anyhow!("Cost controls not initialized"))?,
        provider,
        operation,
    )
    .await
}
async fn reserve_db(db: &SqlitePool, provider: &str, operation: &str) -> Result<i64> {
    reserve_policy(db, provider, operation, crate::authentication::enabled()).await
}
async fn reserve_policy(
    db: &SqlitePool,
    provider: &str,
    operation: &str,
    enforce_membership: bool,
) -> Result<i64> {
    let now = chrono::Utc::now();
    let day = now.format("%Y-%m-%d").to_string();
    let month = now.format("%Y-%m").to_string();
    // Single write statement serializes budget checks and reservations across concurrent callers.
    let actor = crate::authentication::actor();
    let mut sql="INSERT INTO cost_events(provider,operation,created_at,day,month,allocated_micros,user_id,workspace_id) SELECT provider,?,?,?,?,request_micros,?,? FROM cost_limits WHERE provider=? AND paused=0 AND request_micros>0 AND (SELECT count(*) FROM cost_events WHERE provider=? AND day=?)<daily_requests AND (SELECT coalesce(sum(allocated_micros),0) FROM cost_events WHERE provider=? AND day=?)+request_micros<=daily_micros AND (SELECT coalesce(sum(allocated_micros),0) FROM cost_events WHERE provider=? AND month=?)+request_micros<=monthly_micros".to_string();
    if enforce_membership {
        sql.push_str(" AND EXISTS(SELECT 1 FROM memberships m WHERE m.user_id=? AND m.workspace_id=? AND m.active=1 AND m.role IN ('owner','administrator','analyst','reviewer','worker') AND (SELECT count(*) FROM cost_events WHERE user_id=m.user_id AND workspace_id=m.workspace_id AND day=?)<m.daily_requests AND (SELECT coalesce(sum(allocated_micros),0) FROM cost_events WHERE user_id=m.user_id AND workspace_id=m.workspace_id AND day=?)+request_micros<=m.daily_micros AND (SELECT coalesce(sum(allocated_micros),0) FROM cost_events WHERE user_id=m.user_id AND workspace_id=m.workspace_id AND month=?)+request_micros<=m.monthly_micros)");
    }
    let q = sqlx::query(&sql)
        .bind(operation)
        .bind(now.to_rfc3339())
        .bind(&day)
        .bind(&month)
        .bind(&actor.user_id)
        .bind(&actor.workspace_id)
        .bind(provider)
        .bind(provider)
        .bind(&day)
        .bind(provider)
        .bind(&day)
        .bind(provider)
        .bind(&month);
    let q = if enforce_membership {
        q.bind(&actor.user_id)
            .bind(&actor.workspace_id)
            .bind(&day)
            .bind(&day)
            .bind(&month)
    } else {
        q
    };
    let r = q.execute(db).await?;
    ensure!(
        r.rows_affected() == 1,
        "Cost controls: {provider} paused, unconfigured, or budget reached. Open Costs to configure limits."
    );
    Ok(r.last_insert_rowid())
}
pub async fn finish(id: i64, status: &str, usage: Value) -> Result<()> {
    let db = DB
        .get()
        .ok_or_else(|| anyhow::anyhow!("Cost controls not initialized"))?;
    sqlx::query("UPDATE cost_events SET status=?,usage=? WHERE id=?")
        .bind(status)
        .bind(usage.to_string())
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}
#[derive(Deserialize)]
pub struct Settings {
    provider: String,
    paused: bool,
    daily_requests: i64,
    daily_usd: f64,
    monthly_usd: f64,
    request_usd: f64,
}
pub async fn configure(State(app): State<App>, Json(s): Json<Settings>) -> ApiResult<Json<Value>> {
    if !["claude", "openai", "x"].contains(&s.provider.as_str())
        || !(0..=100000).contains(&s.daily_requests)
        || [s.daily_usd, s.monthly_usd, s.request_usd]
            .iter()
            .any(|n| !n.is_finite() || *n < 0. || *n > 100000.)
        || (!s.paused
            && (s.request_usd < 0.000001
                || s.daily_requests == 0
                || s.daily_usd < s.request_usd
                || s.monthly_usd < s.request_usd))
    {
        return Err(crate::ApiError(
            axum::http::StatusCode::BAD_REQUEST,
            "Provide valid USD allowances and a positive request cap before enabling.".into(),
        ));
    }
    sqlx::query("UPDATE cost_limits SET paused=?,daily_requests=?,daily_micros=?,monthly_micros=?,request_micros=? WHERE provider=?").bind(s.paused).bind(s.daily_requests).bind((s.daily_usd*1e6).round() as i64).bind((s.monthly_usd*1e6).round() as i64).bind((s.request_usd*1e6).round() as i64).bind(s.provider).execute(&app.db).await?;
    status(State(app)).await
}
pub async fn status(State(app): State<App>) -> ApiResult<Json<Value>> {
    let now = chrono::Utc::now();
    let day = now.format("%Y-%m-%d").to_string();
    let month = now.format("%Y-%m").to_string();
    let mut items = Vec::new();
    for r in sqlx::query("SELECT * FROM cost_limits ORDER BY provider")
        .fetch_all(&app.db)
        .await?
    {
        let p: String = r.get("provider");
        let usage=sqlx::query("SELECT count(*) n,coalesce(sum(allocated_micros),0) amount FROM cost_events WHERE provider=? AND day=?").bind(&p).bind(&day).fetch_one(&app.db).await?;
        let monthly:i64=sqlx::query_scalar("SELECT coalesce(sum(allocated_micros),0) FROM cost_events WHERE provider=? AND month=?").bind(&p).bind(&month).fetch_one(&app.db).await?;
        items.push(json!({"provider":p,"paused":r.get::<i64,_>("paused")==1,"daily_requests":r.get::<i64,_>("daily_requests"),"daily_usd":r.get::<i64,_>("daily_micros") as f64/1e6,"monthly_usd":r.get::<i64,_>("monthly_micros") as f64/1e6,"request_usd":r.get::<i64,_>("request_micros") as f64/1e6,"requests_today":usage.get::<i64,_>("n"),"allocated_today_usd":usage.get::<i64,_>("amount") as f64/1e6,"allocated_month_usd":monthly as f64/1e6}));
    }
    let mut events = Vec::new();
    for r in sqlx::query("SELECT * FROM cost_events ORDER BY id DESC LIMIT 100")
        .fetch_all(&app.db)
        .await?
    {
        events.push(json!({"user_id":r.get::<String,_>("user_id"),"workspace_id":r.get::<String,_>("workspace_id"),"provider":r.get::<String,_>("provider"),"operation":r.get::<String,_>("operation"),"at":r.get::<String,_>("created_at"),"status":r.get::<String,_>("status"),"allocated_usd":r.get::<i64,_>("allocated_micros") as f64/1e6,"usage":r.get::<Option<String>,_>("usage").and_then(|v|serde_json::from_str::<Value>(&v).ok())}));
    }
    Ok(Json(
        json!({"items":items,"events":events,"currency":"USD","timezone":"UTC","basis":"User-defined per-request allocations, not actual charges. Failed or uncertain requests retain their allocation. Historical spend, browser Mapbox, hosting and other clients are excluded."}),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn user_caps_apply_across_providers_and_revocation_blocks_jobs() {
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        crate::authentication::init(&db).await.unwrap();
        sqlx::query("UPDATE cost_limits SET paused=0,daily_requests=100,daily_micros=10000,monthly_micros=10000,request_micros=100").execute(&db).await.unwrap();
        sqlx::query("INSERT INTO memberships VALUES('congo-signal','a@example.org','alice','analyst',1,2,200,200)").execute(&db).await.unwrap();
        let a = crate::authentication::Actor {
            user_id: "alice".into(),
            workspace_id: "congo-signal".into(),
            role: "analyst".into(),
            email: "a@example.org".into(),
        };
        crate::authentication::ACTOR.scope(a,async {
            assert!(reserve_policy(&db,"claude","test",true).await.is_ok());
            assert!(reserve_policy(&db,"x","test",true).await.is_ok());
            assert!(reserve_policy(&db,"openai","test",true).await.is_err());
            sqlx::query("UPDATE memberships SET active=0,daily_requests=100,daily_micros=10000,monthly_micros=10000 WHERE user_id='alice'").execute(&db).await.unwrap();
            assert!(reserve_policy(&db,"openai","test",true).await.is_err());
        }).await;
        let user: String = sqlx::query_scalar("SELECT user_id FROM cost_events LIMIT 1")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(user, "alice");
    }
    #[tokio::test]
    async fn independent_limits_and_pause() {
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        for (requests, daily, monthly) in [(0, 1000, 1000), (10, 99, 1000), (10, 1000, 99)] {
            sqlx::query("UPDATE cost_limits SET paused=0,daily_requests=?,daily_micros=?,monthly_micros=?,request_micros=100 WHERE provider='claude'").bind(requests).bind(daily).bind(monthly).execute(&db).await.unwrap();
            assert!(reserve_db(&db, "claude", "test").await.is_err());
        }
        sqlx::query("UPDATE cost_limits SET paused=1,daily_requests=10,daily_micros=1000,monthly_micros=1000 WHERE provider='claude'").execute(&db).await.unwrap();
        assert!(reserve_db(&db, "claude", "test").await.is_err());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM cost_events")
                .fetch_one(&db)
                .await
                .unwrap(),
            0
        );
    }
    #[tokio::test]
    async fn persistent_atomic_caps() {
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        assert!(reserve_db(&db, "x", "test").await.is_err());
        sqlx::query("UPDATE cost_limits SET paused=0,daily_requests=2,daily_micros=200,monthly_micros=200,request_micros=100 WHERE provider='x'").execute(&db).await.unwrap();
        let (a, b, c) = tokio::join!(
            reserve_db(&db, "x", "test"),
            reserve_db(&db, "x", "test"),
            reserve_db(&db, "x", "test")
        );
        assert_eq!([a, b, c].iter().filter(|v| v.is_ok()).count(), 2);
        init(&db).await.unwrap();
        assert!(reserve_db(&db, "x", "test").await.is_err());
    }
}
