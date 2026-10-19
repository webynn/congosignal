//! Public reporting directory. Mentions never automatically establish activity.
use crate::{
    ApiError, ApiResult, App,
    extract::{Article, digest},
    text,
};
use axum::{Json, extract::State, http::StatusCode};
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};

fn catalogue() -> Value {
    serde_json::from_str(include_str!("../config/armed-groups.json")).expect("group catalogue")
}
pub async fn init(db: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query("CREATE TABLE IF NOT EXISTS group_activity_reviews(seq INTEGER PRIMARY KEY AUTOINCREMENT, data TEXT NOT NULL)").execute(db).await?;
    Ok(())
}
fn day(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.get(..10)?, "%Y-%m-%d").ok()
}
#[cfg(test)]
fn matches(body: &str, terms: &[Value]) -> bool {
    let doc = text::Doc::new(body);
    terms
        .iter()
        .filter_map(Value::as_str)
        .any(|s| doc.has_term(&text::term(s)))
}
async fn evidence(db: &SqlitePool) -> anyhow::Result<Vec<Value>> {
    let cat = catalogue();
    let groups = cat["groups"].as_array().unwrap();
    let mut sources = Vec::new();
    for raw in sqlx::query_scalar::<_, String>("SELECT data FROM articles")
        .fetch_all(db)
        .await?
    {
        let a: Article = serde_json::from_str(&raw)?;
        sources.push((json!({"id":a.id,"title":a.title,"publisher":a.source,"url":a.url,"published_at":a.published_at,"kind":"article"}),format!("{}\n{}\n{}",a.title,a.description.unwrap_or_default(),a.content)));
    }
    for raw in sqlx::query_scalar::<_, String>("SELECT data FROM evidence_snapshots")
        .fetch_all(db)
        .await?
    {
        let d: Value = serde_json::from_str(&raw)?;
        for p in d["pages"].as_array().into_iter().flatten() {
            sources.push((json!({"id":d["id"],"title":d["source"]["title"],"publisher":d["source"]["publisher"],"url":d["source"]["url"],"published_at":d["source"]["reference_date"],"kind":"document","page":p["page"]}),p["text"].as_str().unwrap_or("").to_owned()));
        }
    }
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let compiled: Vec<_> = groups
        .iter()
        .map(|g| {
            (
                g,
                g["terms"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(Value::as_str)
                    .map(text::term)
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    for (source, body) in sources {
        let mut counts = HashMap::<String, usize>::new();
        for passage in body
            .split('\n')
            .map(str::trim)
            .filter(|p| !p.is_empty() && p.chars().count() <= 3000)
        {
            let doc = text::Doc::new(passage);
            for (g, terms) in &compiled {
                let gid = g["id"].as_str().unwrap();
                if counts.get(gid).copied().unwrap_or(0) >= 3
                    || !terms.iter().any(|t| doc.has_term(t))
                {
                    continue;
                }
                *counts.entry(gid.to_owned()).or_default() += 1;
                let id = digest(
                    format!(
                        "{}:{}:{}:{}",
                        g["id"], source["url"], source["page"], passage
                    )
                    .as_bytes(),
                );
                if seen.insert(id.clone()) {
                    out.push(json!({"id":id,"group_id":g["id"],"source":source,"quote":passage}));
                }
            }
        }
    }
    out.sort_by(|a, b| {
        b["source"]["published_at"]
            .as_str()
            .cmp(&a["source"]["published_at"].as_str())
    });
    Ok(out)
}
async fn reviews(db: &SqlitePool) -> anyhow::Result<Vec<Value>> {
    sqlx::query_scalar::<_, String>("SELECT data FROM group_activity_reviews ORDER BY seq")
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|s| Ok(serde_json::from_str(&s)?))
        .collect()
}
fn status(events: &[Value], now: NaiveDate) -> Value {
    let latest = events
        .iter()
        .filter(|e| {
            ["reported", "corroborated", "disputed"].contains(&e["decision"].as_str().unwrap_or(""))
        })
        .filter_map(|e| day(e["event_date"].as_str()?).map(|d| (d, e)))
        .max_by_key(|(d, _)| *d);
    match latest {
        None => json!({"code":"insufficient_evidence","last_event_date":null}),
        Some((date, e)) => {
            let age = (now - date).num_days();
            let code = if e["decision"] == "disputed" {
                "disputed"
            } else if ["dissolution", "merger"].contains(&e["activity"].as_str().unwrap_or("")) {
                "organizational_change_reported"
            } else if age <= 30 {
                "activity_reported_30d"
            } else if age <= 90 {
                "activity_reported_90d"
            } else {
                "historical_evidence"
            };
            json!({"code":code,"last_event_date":date.to_string(),"activity":e["activity"],"decision":e["decision"]})
        }
    }
}
pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let cat = catalogue();
    let all = evidence(&app.db).await?;
    let history = reviews(&app.db).await?;
    let mut latest = HashMap::new();
    for r in &history {
        latest.insert(r["evidence_ids"][0].as_str().unwrap_or(""), r.clone());
    }
    let items: Vec<Value> = cat["groups"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| {
            let mut g = g.clone();
            let mentions: Vec<Value> = all
                .iter()
                .filter(|e| e["group_id"] == g["id"])
                .cloned()
                .collect();
            let events: Vec<Value> = latest
                .values()
                .filter(|e| e["group_id"] == g["id"])
                .cloned()
                .collect();
            g["status"] = status(&events, Utc::now().date_naive());
            g["source_count"] = json!(
                mentions
                    .iter()
                    .filter_map(|e| e["source"]["url"].as_str())
                    .collect::<HashSet<_>>()
                    .len()
            );
            g["last_publication_date"] = mentions
                .first()
                .map(|e| e["source"]["published_at"].clone())
                .unwrap_or(Value::Null);
            g["evidence"] = json!(mentions);
            g["events"] = json!(events);
            g
        })
        .collect();
    Ok(Json(
        json!({"items":items,"history":history,"generated_at":Utc::now().to_rfc3339(),"notice":cat["notice"],"method":"Word-boundary matching of archived news and documents. Activity statuses require an attributed, dated reviewer assessment. Recent publication is not recent activity. No tactical locations or inferred inactivity."}),
    ))
}
#[derive(Deserialize)]
pub struct Review {
    group_id: String,
    evidence_ids: Vec<String>,
    event_date: String,
    activity: String,
    decision: String,
    note: String,
    #[serde(default)]
    independent_sources: bool,
}
fn validate(r: &Review) -> anyhow::Result<()> {
    anyhow::ensure!(
        !r.evidence_ids.is_empty() && r.evidence_ids.len() <= 5,
        "Select 1–5 passages"
    );
    anyhow::ensure!(
        r.evidence_ids.iter().collect::<HashSet<_>>().len() == r.evidence_ids.len(),
        "Duplicate evidence"
    );
    anyhow::ensure!(
        r.event_date.len() == 10
            && day(&r.event_date).is_some_and(|d| d <= Utc::now().date_naive()),
        "Confirm a valid event date, not a future date"
    );
    anyhow::ensure!(
        [
            "violence",
            "recruitment",
            "statement",
            "negotiation",
            "demobilisation",
            "dissolution",
            "merger",
            "other"
        ]
        .contains(&r.activity.as_str()),
        "Invalid activity type"
    );
    anyhow::ensure!(
        ["reported", "corroborated", "disputed", "rejected"].contains(&r.decision.as_str()),
        "Invalid decision"
    );
    anyhow::ensure!(
        (30..=3000).contains(&r.note.trim().chars().count()),
        "Explain event date, attribution and uncertainty in 30–3000 characters"
    );
    anyhow::ensure!(
        r.decision != "corroborated" || (r.independent_sources && r.evidence_ids.len() >= 2),
        "Corroboration requires two independent reports and reviewer attestation"
    );
    Ok(())
}
pub async fn review(State(app): State<App>, Json(r): Json<Review>) -> ApiResult<Json<Value>> {
    validate(&r).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
    let all = evidence(&app.db).await?;
    let mut selected = Vec::new();
    for id in &r.evidence_ids {
        let e = all
            .iter()
            .find(|e| e["id"] == *id && e["group_id"] == r.group_id)
            .ok_or_else(|| {
                ApiError(
                    StatusCode::BAD_REQUEST,
                    "Evidence missing or belongs to another group".into(),
                )
            })?;
        if let Some(p) = e["source"]["published_at"].as_str().and_then(day)
            && day(&r.event_date).unwrap() > p
        {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                "Event date is after source publication".into(),
            ));
        }
        selected.push(e.clone());
    }
    if r.decision == "corroborated" {
        let urls: HashSet<_> = selected
            .iter()
            .filter_map(|e| e["source"]["url"].as_str())
            .collect();
        let publishers: HashSet<_> = selected
            .iter()
            .filter_map(|e| e["source"]["publisher"].as_str())
            .map(|p| text::fold(p.trim().trim_start_matches("NewsData.io · ")))
            .collect();
        if urls.len() < 2 || publishers.len() < 2 {
            return Err(ApiError(StatusCode::BAD_REQUEST,"Use distinct reports from different publishers; verify they are independent, not syndicated copies".into()));
        }
    }
    let out = json!({"group_id":r.group_id,"evidence_ids":r.evidence_ids,"evidence":selected,"event_date":r.event_date,"activity":r.activity,"decision":r.decision,"note":r.note.trim(),"independent_sources":r.independent_sources,"reviewer":crate::authentication::actor().user_id,"reviewed_at":Utc::now().to_rfc3339()});
    sqlx::query("INSERT INTO group_activity_reviews(data) VALUES(?)")
        .bind(out.to_string())
        .execute(&app.db)
        .await?;
    Ok(Json(out))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matching_requires_words() {
        let t = vec![json!("ADF")];
        assert!(matches("The ADF was mentioned", &t));
        assert!(!matches("ADFREE report", &t));
    }
    #[test]
    fn no_mentions_are_not_inactivity() {
        assert_eq!(
            status(&[], Utc::now().date_naive())["code"],
            "insufficient_evidence"
        );
    }
    #[test]
    fn publication_does_not_make_old_event_current() {
        let r = json!({"event_date":"2020-01-01","decision":"reported","activity":"violence","published_at":Utc::now().to_rfc3339()});
        assert_eq!(
            status(&[r], Utc::now().date_naive())["code"],
            "historical_evidence"
        );
    }
    #[test]
    fn corroboration_requires_attestation() {
        let r = Review {
            group_id: "a".into(),
            evidence_ids: vec!["a".into()],
            event_date: "2020-01-01".into(),
            activity: "statement".into(),
            decision: "corroborated".into(),
            note: "A sufficiently detailed explanation of source attribution".into(),
            independent_sources: false,
        };
        assert!(validate(&r).is_err());
    }
}
