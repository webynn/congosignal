//! Delayed province-level historical reporting, never a live control boundary layer.
use crate::{
    ApiError, ApiResult, App,
    extract::{Article, digest},
    text,
};
use axum::{Json, extract::State, http::StatusCode};
use chrono::{Duration, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
pub const PROVINCES: [(&str, f64, f64); 3] = [
    ("Nord-Kivu", 28.7, -0.6),
    ("Sud-Kivu", 28.4, -3.0),
    ("Ituri", 29.1, 1.5),
];
fn cutoff() -> NaiveDate {
    Utc::now().date_naive() - Duration::days(30)
}
fn historical(s: &str) -> bool {
    s.get(..10)
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .is_some_and(|d| d <= cutoff())
}
pub async fn init(db: &SqlitePool) -> anyhow::Result<()> {
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS historical_claims(id TEXT PRIMARY KEY,data TEXT NOT NULL); CREATE TABLE IF NOT EXISTS historical_reviews(seq INTEGER PRIMARY KEY AUTOINCREMENT,claim_id TEXT NOT NULL,created_at TEXT NOT NULL,data TEXT NOT NULL);").execute(db).await?;
    Ok(())
}
fn candidates(source: Value, body: &str) -> Vec<Value> {
    let Some(date) = source["published_at"].as_str() else {
        return vec![];
    };
    if !historical(date) {
        return vec![];
    }
    let mut out = Vec::new();
    // Paragraphs are candidates only: no event date, actor control or credibility is inferred.
    for paragraph in body
        .split('\n')
        .map(str::trim)
        .filter(|p| p.chars().count() >= 60 && p.chars().count() <= 2500)
    {
        let t = text::fold(paragraph);
        if !["m23", "fardc", "afc/"].iter().any(|x| t.contains(x))
            || ![
                "control", "controle", "presence", "occup", "retrait", "withdraw", "pris ",
                "prise ", "captur",
            ]
            .iter()
            .any(|x| t.contains(x))
        {
            continue;
        }
        for (province, _, _) in PROVINCES {
            let english = match province {
                "Nord-Kivu" => "north kivu",
                "Sud-Kivu" => "south kivu",
                _ => "ituri",
            };
            if !t.contains(english)
                && !t.contains(&province.to_lowercase())
                && !t.replace(' ', "-").contains(&province.to_lowercase())
            {
                continue;
            }
            let id =
                digest(format!("history-v1:{}:{province}:{paragraph}", source["id"]).as_bytes());
            out.push(json!({"id":id,"province":province,"source":source,"quote":paragraph,"status":"unreviewed","discovered_at":Utc::now().to_rfc3339()}));
        }
    }
    out
}
async fn discover(db: &SqlitePool) -> anyhow::Result<usize> {
    let mut found = Vec::new();
    let rows: Vec<String> = sqlx::query_scalar("SELECT data FROM articles")
        .fetch_all(db)
        .await?;
    for raw in rows {
        let a: Article = serde_json::from_str(&raw)?;
        found.extend(candidates(json!({"id":a.id,"kind":"article","title":a.title,"publisher":a.source,"url":a.url,"published_at":a.published_at}),&a.content));
    }
    let docs: Vec<String> = sqlx::query_scalar("SELECT data FROM evidence_snapshots")
        .fetch_all(db)
        .await?;
    for raw in docs {
        let d: Value = serde_json::from_str(&raw)?;
        for p in d["pages"].as_array().into_iter().flatten() {
            found.extend(candidates(json!({"id":d["id"],"kind":"document","title":d["source"]["title"],"publisher":d["source"]["publisher"],"url":d["source"]["url"],"published_at":d["source"]["reference_date"],"page":p["page"]}),p["text"].as_str().unwrap_or("")));
        }
    }
    let mut count = 0;
    for c in found {
        count += sqlx::query("INSERT OR IGNORE INTO historical_claims VALUES(?,?)")
            .bind(c["id"].as_str())
            .bind(c.to_string())
            .execute(db)
            .await?
            .rows_affected() as usize;
    }
    Ok(count)
}
pub async fn worker(db: SqlitePool) -> anyhow::Result<()> {
    loop {
        if let Err(e) = discover(&db).await {
            tracing::warn!(error=%e,"historical discovery failed");
        }
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }
}
pub async fn refresh(State(app): State<App>) -> ApiResult<Json<Value>> {
    Ok(Json(json!({"added":discover(&app.db).await?})))
}
pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let rows: Vec<String> =
        sqlx::query_scalar("SELECT data FROM historical_claims ORDER BY rowid DESC")
            .fetch_all(&app.db)
            .await?;
    let reviews = sqlx::query("SELECT seq,data FROM historical_reviews ORDER BY seq")
        .fetch_all(&app.db)
        .await?;
    let mut latest = std::collections::HashMap::new();
    let mut history = Vec::new();
    for r in reviews {
        let mut v: Value =
            serde_json::from_str(r.get::<&str, _>("data")).map_err(anyhow::Error::from)?;
        v["revision"] = json!(r.get::<i64, _>("seq"));
        latest.insert(v["claim_id"].as_str().unwrap_or("").to_owned(), v.clone());
        history.push(v);
    }
    let mut seen = std::collections::HashSet::new();
    let items: Vec<Value> = rows
        .into_iter()
        .filter_map(|r| serde_json::from_str::<Value>(&r).ok())
        .map(|mut c| {
            c["review"] = latest
                .get(c["id"].as_str().unwrap_or(""))
                .cloned()
                .unwrap_or(Value::Null);
            c
        })
        .filter(|c| {
            !c["review"].is_null()
                || seen.insert(format!(
                    "{}:{}:{}",
                    c["source"]["url"], c["province"], c["quote"]
                ))
        })
        .collect();
    let observations: Value =
        serde_json::from_str(include_str!("../dashboard/geology/historical-ipis.json"))
            .map_err(anyhow::Error::from)?;
    Ok(Json(
        json!({"observations":observations,"items":items,"history":history,"cutoff":cutoff().to_string(),"delay_days":30,"provinces":PROVINCES.iter().map(|(n,x,y)|json!({"name":n,"lon":x,"lat":y})).collect::<Vec<_>>()}),
    ))
}
#[derive(Deserialize)]
pub struct Review {
    claim_id: String,
    status: String,
    event_date: String,
    actor: String,
    assessment: String,
    note: String,
}
fn validate(r: &Review) -> anyhow::Result<()> {
    anyhow::ensure!(
        ["reviewed", "disputed", "rejected"].contains(&r.status.as_str()),
        "Invalid review status"
    );
    anyhow::ensure!(
        historical(&r.event_date) && r.event_date.len() == 10,
        "Use a confirmed event date at least 30 days old"
    );
    anyhow::ensure!(
        ["fardc", "afc_m23", "both", "unknown"].contains(&r.actor.as_str()),
        "Invalid actor"
    );
    anyhow::ensure!(
        ["presence", "reported_control", "contested", "unknown"].contains(&r.assessment.as_str()),
        "Invalid assessment"
    );
    anyhow::ensure!(
        (20..=2000).contains(&r.note.trim().chars().count()),
        "Explain the evidence and uncertainty in 20–2000 characters"
    );
    Ok(())
}
pub async fn review(State(app): State<App>, Json(r): Json<Review>) -> ApiResult<Json<Value>> {
    validate(&r).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    let raw: Option<String> = sqlx::query_scalar("SELECT data FROM historical_claims WHERE id=?")
        .bind(&r.claim_id)
        .fetch_optional(&app.db)
        .await?;
    let c: Value = serde_json::from_str(
        &raw.ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "Claim not found".into()))?,
    )
    .map_err(anyhow::Error::from)?;
    if !historical(c["source"]["published_at"].as_str().unwrap_or("")) {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Source is not eligible for historical publication".into(),
        ));
    }
    let published = c["source"]["published_at"]
        .as_str()
        .and_then(|d| d.get(..10))
        .unwrap_or("");
    if r.event_date.as_str() > published {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Event date cannot be after the source publication date".into(),
        ));
    }
    let actor = crate::authentication::actor();
    let out = json!({"claim_id":r.claim_id,"province":c["province"],"status":r.status,"event_date":r.event_date,"actor":r.actor,"assessment":r.assessment,"note":r.note.trim(),"reviewer":actor.user_id,"created_at":Utc::now().to_rfc3339()});
    sqlx::query("INSERT INTO historical_reviews(claim_id,created_at,data) VALUES(?,?,?)")
        .bind(&r.claim_id)
        .bind(out["created_at"].as_str())
        .bind(out.to_string())
        .execute(&app.db)
        .await?;
    Ok(Json(out))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dates_and_review_gate() {
        assert!(!historical(&Utc::now().date_naive().to_string()));
        assert!(!historical("invalid"));
        assert!(historical("2022-01-01"));
        let r = Review {
            claim_id: "x".into(),
            status: "reviewed".into(),
            event_date: Utc::now().date_naive().to_string(),
            actor: "fardc".into(),
            assessment: "presence".into(),
            note: "Evidence checked against source text".into(),
        };
        assert!(validate(&r).is_err());
    }
    #[test]
    fn discovery_is_historical_and_does_not_infer_control() {
        let s = json!({"id":"a","published_at":"2022-01-01"});
        let text = "Le rapport documente la présence des FARDC au Nord-Kivu, sans établir le contrôle de toute la province.";
        let c = candidates(s, text);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0]["status"], "unreviewed");
        assert!(c[0].get("actor").is_none());
        assert!(candidates(json!({"published_at":Utc::now().to_rfc3339()}), text).is_empty());
    }
}
