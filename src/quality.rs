//! Read-only collection diagnostics; completeness is not an accuracy score.
use crate::{
    ApiResult, App,
    extract::{Article, digest},
    intelligence, text,
};
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::Row;
use std::collections::BTreeMap;

pub async fn ready(State(app): State<App>) -> Response {
    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='articles'",
        )
        .fetch_one(&app.db),
    )
    .await
    {
        Ok(Ok(1)) => (StatusCode::OK, Json(json!({"status":"ready"}))).into_response(),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"status":"not_ready"})),
        )
            .into_response(),
    }
}
fn content_key(a: &Article) -> Option<String> {
    let normalized = text::tokens(&a.content).join(" ");
    (normalized.len() >= 100).then(|| digest(normalized.as_bytes()))
}
pub async fn report(State(app): State<App>) -> ApiResult<Json<Value>> {
    let raw: Vec<String> =
        sqlx::query_scalar("SELECT data FROM articles ORDER BY published_at DESC")
            .fetch_all(&app.db)
            .await?;
    let mut pubs = BTreeMap::<String, usize>::new();
    let mut duplicates = BTreeMap::<String, Vec<Value>>::new();
    let mut issues = Vec::new();
    let mut missing_date = 0;
    let mut future_date = 0;
    let mut missing_author = 0;
    let mut summary_only = 0;
    let mut previews = BTreeMap::<String, usize>::new();
    let mut malformed = 0;
    for raw in &raw {
        let Ok(a) = serde_json::from_str::<Article>(raw) else {
            malformed += 1;
            continue;
        };
        *pubs
            .entry(a.source.trim_start_matches("NewsData.io · ").to_owned())
            .or_default() += 1;
        let date = a
            .published_at
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok());
        let mut reasons = Vec::new();
        if date.is_none() {
            missing_date += 1;
            reasons.push("missing_date");
        }
        if date.is_some_and(|d| d > Utc::now() + chrono::Duration::hours(1)) {
            future_date += 1;
            reasons.push("future_date");
        }
        if a.author.as_deref().is_none_or(|s| s.trim().is_empty()) {
            missing_author += 1;
        }
        if matches!(a.coverage.as_deref(), Some("feed_summary" | "api_summary")) {
            summary_only += 1;
        }
        if let Some(key) = content_key(&a) {
            duplicates
                .entry(key)
                .or_default()
                .push(json!({"id":a.id,"title":a.title,"source":a.source,"url":a.url}));
        }
        if a.language.starts_with("en") {
            *previews.entry("source_english".into()).or_default() += 1;
        } else {
            let preview = intelligence::enrichment(&app.db, &a, "preview").await?;
            let brief = intelligence::enrichment(&app.db, &a, "brief").await?;
            let status = if brief["status"] == "completed" || preview["status"] == "completed" {
                "cached"
            } else {
                preview["status"].as_str().unwrap_or("not_requested")
            };
            *previews.entry(status.into()).or_default() += 1;
            if status == "failed" {
                reasons.push("preview_failed");
            }
        }
        if !reasons.is_empty() && issues.len() < 50 {
            issues.push(json!({"id":a.id,"title":a.title,"source":a.source,"reasons":reasons}));
        }
    }
    let duplicates: Vec<Value> = duplicates
        .into_values()
        .filter(|v| v.len() > 1)
        .map(|v| json!({"articles":v}))
        .collect();
    let feeds = crate::feeds::list(State(app.clone())).await?.0;
    let feed_health:Vec<Value>=feeds["items"].as_array().into_iter().flatten().map(|f|{
        let interval=f["poll_seconds"].as_i64().unwrap_or(0);
        let checked=f["sync"]["checked_at"].as_str().and_then(|s|DateTime::parse_from_rfc3339(s).ok());
        let overdue=interval>0&&checked.is_none_or(|d|(Utc::now()-d.with_timezone(&Utc)).num_seconds()>interval*2+600);
        let status=if f["configured"]!=true{"not_configured"}else if interval==0{"paused"}else if f["sync"]["status"]=="failed"{"failed"}else if overdue{"overdue"}else{"checked"};
        json!({"name":f["name"],"status":status,"checked_at":f["sync"]["checked_at"],"poll_seconds":interval,"collected":f["collected"]})
    }).collect();
    let queues = sqlx::query("SELECT status,count(*) AS n FROM jobs GROUP BY status")
        .fetch_all(&app.db)
        .await?
        .iter()
        .map(|r| json!({"status":r.get::<String,_>("status"),"count":r.get::<i64,_>("n")}))
        .collect::<Vec<_>>();
    let failed_documents: i64 =
        sqlx::query_scalar("SELECT count(*) FROM evidence_sources WHERE status='failed'")
            .fetch_one(&app.db)
            .await?;
    Ok(Json(
        json!({"generated_at":Utc::now().to_rfc3339(),"auth_mode":if crate::authentication::enabled(){"oidc"}else{"local"},"articles":raw.len(),"publishers":pubs,"missing_date":missing_date,"future_date":future_date,"missing_author":missing_author,"summary_only":summary_only,"malformed_records":malformed,"english_previews":previews,"duplicate_groups":duplicates,"issues":issues,"feeds":feed_health,"jobs":queues,"failed_documents":failed_documents,"accuracy_evaluated":false,"notice":"Completeness and processing checks only. Duplicate text may be syndication, not independent corroboration. Translation accuracy and entity attribution require human sample review."}),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn readiness_requires_initialized_database() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        let app = App {
            db: db.clone(),
            token: None,
        };
        assert_eq!(
            ready(State(app.clone())).await.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
        sqlx::query("CREATE TABLE articles(id TEXT)")
            .execute(&db)
            .await
            .unwrap();
        assert_eq!(ready(State(app)).await.status(), StatusCode::OK);
    }
}
