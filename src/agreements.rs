//! Evidence-linked agreement research; draft assessments never masquerade as outcomes.
use crate::{ApiError, ApiResult, App};
use anyhow::{Result, ensure};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
pub const METHOD_VERSION: &str = "agreements-v2-passages";
static ANALYSIS_GATE: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);
pub fn catalogue() -> Value {
    serde_json::from_str(include_str!("../config/agreements.json")).expect("agreement catalogue")
}
async fn lookup(db: &SqlitePool, id: &str) -> Result<Option<Value>> {
    if let Some(v) = catalogue()["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == id)
    {
        return Ok(Some(v.clone()));
    }
    let raw = sqlx::query_scalar::<_, String>("SELECT data FROM agreement_discoveries WHERE id=?")
        .bind(id)
        .fetch_optional(db)
        .await?;
    Ok(raw.map(|s| serde_json::from_str(&s)).transpose()?)
}
pub fn fingerprint(record: &Value) -> String {
    crate::extract::digest(format!("{}:{}", METHOD_VERSION, record).as_bytes())
}
pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::query("CREATE TABLE IF NOT EXISTS agreement_assessments(id TEXT PRIMARY KEY,status TEXT NOT NULL,data TEXT,error TEXT,reviewed INTEGER NOT NULL DEFAULT 0,updated_at TEXT NOT NULL)").execute(db).await?;
    sqlx::query("UPDATE agreement_assessments SET status='failed',error='Worker restarted during analysis' WHERE status='running'").execute(db).await?;
    for record in catalogue()["items"].as_array().unwrap() {
        let id = record["id"].as_str().unwrap();
        if let Some(raw) = sqlx::query_scalar::<_, String>(
            "SELECT data FROM agreement_assessments WHERE id=? AND status='completed'",
        )
        .bind(id)
        .fetch_optional(db)
        .await?
        {
            let old: Value = serde_json::from_str(&raw)?;
            if old["source_fingerprint"] != fingerprint(record) {
                sqlx::query("UPDATE agreement_assessments SET status='stale',reviewed=0,error='Source or method changed; reassessment required' WHERE id=?").bind(id).execute(db).await?;
            }
        }
    }
    Ok(())
}
async fn assessment(db: &SqlitePool, id: &str) -> Result<Value> {
    let r = sqlx::query("SELECT * FROM agreement_assessments WHERE id=?")
        .bind(id)
        .fetch_optional(db)
        .await?;
    Ok(match r {
        Some(r) => {
            json!({"status":r.get::<String,_>("status"),"data":r.get::<Option<String>,_>("data").map(|s|serde_json::from_str::<Value>(&s)).transpose()?,"error":r.get::<Option<String>,_>("error"),"reviewed":r.get::<i64,_>("reviewed")==1,"updated_at":r.get::<String,_>("updated_at")})
        }
        None => json!({"status":"not_requested","reviewed":false}),
    })
}
pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let mut c = catalogue();
    c["items"]
        .as_array_mut()
        .unwrap()
        .extend(crate::agreement_discovery::records(&app.db).await?);
    c["discovery"] = crate::agreement_discovery::status(&app.db).await?;
    for i in c["items"].as_array_mut().unwrap() {
        i["assessment"] = assessment(&app.db, i["id"].as_str().unwrap()).await?;
        for d in i["documents"].as_array_mut().unwrap() {
            let n = d["text"].as_str().unwrap_or("").chars().count();
            d.as_object_mut().unwrap().remove("text");
            d.as_object_mut().unwrap().remove("pages");
            d["characters"] = json!(n);
        }
    }
    Ok(Json(c))
}
pub async fn detail(State(app): State<App>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let mut v = lookup(&app.db, &id)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "Unknown agreement".into()))?;
    v["assessment"] = assessment(&app.db, &id).await?;
    Ok(Json(v))
}
pub async fn pdf(
    State(app): State<App>,
    Path((id, doc)): Path<(String, String)>,
) -> ApiResult<([(axum::http::HeaderName, &'static str); 2], Vec<u8>)> {
    let v = lookup(&app.db, &id)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "Unknown agreement".into()))?;
    ensure_doc(&v, &doc).map_err(|_| ApiError(StatusCode::NOT_FOUND, "Unknown PDF".into()))?;
    let bytes = tokio::fs::read(format!("data/agreements/{doc}.pdf"))
        .await
        .map_err(|_| {
            ApiError(
                StatusCode::NOT_FOUND,
                "PDF archive not installed on this server; use source link".into(),
            )
        })?;
    Ok((
        [
            (axum::http::header::CONTENT_TYPE, "application/pdf"),
            (axum::http::header::CONTENT_DISPOSITION, "inline"),
        ],
        bytes,
    ))
}
fn ensure_doc(v: &Value, id: &str) -> Result<()> {
    ensure!(
        v["documents"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["id"] == id && d["format"] == "pdf"),
        "unknown document"
    );
    Ok(())
}
pub async fn analyze(
    State(app): State<App>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let v = lookup(&app.db, &id)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "Unknown agreement".into()))?;
    if !v["documents"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["status"] == "available")
    {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "No readable document; OCR or collection required".into(),
        ));
    }
    let permit = ANALYSIS_GATE.try_acquire().map_err(|_| {
        ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "An agreement analysis is already running".into(),
        )
    })?;
    if assessment(&app.db, &id).await?["status"] == "completed" {
        return Ok((
            StatusCode::OK,
            Json(json!({"status":"completed","cached":true})),
        ));
    }
    sqlx::query("INSERT INTO agreement_assessments(id,status,updated_at) VALUES(?,'running',?) ON CONFLICT(id) DO UPDATE SET status='running',error=NULL,reviewed=0,updated_at=excluded.updated_at").bind(&id).bind(chrono::Utc::now().to_rfc3339()).execute(&app.db).await?;
    let requester = crate::authentication::actor();
    tokio::spawn(async move {
        let _permit = permit;
        let result = crate::authentication::ACTOR
            .scope(requester, crate::intelligence::agreement_assessment(&v))
            .await;
        let (status, data, error) = match result {
            Ok(data) => ("completed", Some(data.to_string()), None),
            Err(e) => ("failed", None, Some(format!("{e:#}"))),
        };
        let _ = sqlx::query(
            "UPDATE agreement_assessments SET status=?,data=?,error=?,updated_at=? WHERE id=?",
        )
        .bind(status)
        .bind(data)
        .bind(error)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(id)
        .execute(&app.db)
        .await;
    });
    Ok((StatusCode::ACCEPTED, Json(json!({"status":"running"}))))
}
pub async fn review(State(app): State<App>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let r = sqlx::query(
        "UPDATE agreement_assessments SET reviewed=1 WHERE id=? AND status='completed'",
    )
    .bind(&id)
    .execute(&app.db)
    .await?;
    if r.rows_affected() == 0 {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "No completed draft to review".into(),
        ));
    }
    Ok(Json(assessment(&app.db, &id).await?))
}
pub fn validate_analysis(out: &mut Value, input: &Value) -> Result<()> {
    for section in ["design", "outcomes"] {
        let rows = out[section].as_array_mut().ok_or_else(|| {
            anyhow::anyhow!("AI returned no valid {section} array; retry analysis")
        })?;
        ensure!(rows.len() == 5, "Expected five dimensions");
        let mut seen = std::collections::HashSet::new();
        for row in rows {
            let dim = row["dimension"].as_str().unwrap_or("");
            ensure!(
                [
                    "security",
                    "revenue",
                    "services",
                    "sovereignty",
                    "accountability"
                ]
                .contains(&dim)
                    && seen.insert(dim.to_string()),
                "Invalid or duplicate dimension"
            );
            let doc = input["documents"]
                .as_array()
                .unwrap()
                .iter()
                .find(|d| d["id"] == row["document_id"]);
            // Quotes come from supplied passages, never from model-authored strings.
            if row.get("passage_id").is_some() {
                let passage = doc
                    .and_then(|d| d["passages"].as_array())
                    .and_then(|ps| ps.iter().find(|p| p["id"] == row["passage_id"]));
                row["quote"] = json!(passage.and_then(|p| p["text"].as_str()).unwrap_or(""));
            }
            let quote = row["quote"].as_str().unwrap_or("").to_string();
            let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
            let verified = doc.is_some_and(|d| {
                quote.len() >= 15 && norm(d["text"].as_str().unwrap_or("")).contains(&norm(&quote))
            });
            row["quote_verified"] = json!(verified);
            if !verified && !quote.is_empty() {
                row["quote"] = json!("");
                row["validation_note"] = json!("Citation withheld: no matching source passage");
            }
            if row["score"].is_null() {
                continue;
            }
            ensure!(
                row["score"].as_i64().is_some_and(|n| (-2..=2).contains(&n)),
                "Invalid score"
            );
            let doc = input["documents"]
                .as_array()
                .unwrap()
                .iter()
                .find(|d| d["id"] == row["document_id"]);
            let quote = row["quote"].as_str().unwrap_or("").to_string();
            let norm = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
            let supported = doc.is_some_and(|d| {
                quote.len() >= 15
                    && norm(d["text"].as_str().unwrap_or("")).contains(&norm(&quote))
                    && d["status"] == "available"
                    && (if section == "outcomes" {
                        d["role"] == "implementation_report"
                            && input["date"]
                                .as_str()
                                .zip(d["report_date"].as_str())
                                .is_some_and(|(signed, reported)| reported >= signed)
                    } else {
                        d["role"] == "agreement"
                    })
            });
            if !supported {
                row["score"] = Value::Null;
                row["validation_note"] =
                    json!("Score withheld: quotation or outcome evidence could not be verified");
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn citations_use_source_passages_and_reject_unknown_ids() {
        let quote = "We will build roads and deliver services.";
        let rows: Vec<_> = ["security", "revenue", "services", "sovereignty", "accountability"]
            .iter().map(|d| json!({"dimension":d,"score":1,"document_id":"contract","passage_id":"p1","quote":"Invented quotation"})).collect();
        let input = json!({"documents":[{"id":"contract","role":"agreement","status":"available","text":quote,"passages":[{"id":"p1","text":quote}]}]});
        let mut out = json!({"design":rows,"outcomes":rows});
        out["design"][1]["passage_id"] = json!("nonexistent");
        validate_analysis(&mut out, &input).unwrap();
        assert_eq!(out["design"][0]["quote"], quote);
        assert_eq!(out["design"][0]["quote_verified"], true);
        assert_eq!(out["design"][0]["score"], 1);
        assert!(out["design"][1]["score"].is_null());
        assert_eq!(out["design"][1]["quote"], "");
    }
    #[tokio::test]
    async fn changed_source_invalidates_review() {
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        let c = catalogue();
        let id = c["items"][0]["id"].as_str().unwrap();
        sqlx::query("INSERT INTO agreement_assessments VALUES(?,'completed','{\"source_fingerprint\":\"old\"}',NULL,1,'2026-01-01')").bind(id).execute(&db).await.unwrap();
        init(&db).await.unwrap();
        let a = assessment(&db, id).await.unwrap();
        assert_eq!(a["status"], "stale");
        assert_eq!(a["reviewed"], false);
    }
    #[test]
    fn historical_report_cannot_score_later_amendment() {
        let rows:Vec<_>=["security","revenue","services","sovereignty","accountability"].iter().map(|d|json!({"dimension":d,"score":1,"document_id":"report","quote":"Road construction was partly completed."})).collect();
        let input = json!({"date":"2024-03-14","documents":[{"id":"report","role":"implementation_report","status":"available","report_date":"2021-09","text":"Road construction was partly completed."}]});
        let mut out = json!({"design":rows,"outcomes":rows});
        validate_analysis(&mut out, &input).unwrap();
        assert!(out["outcomes"][0]["score"].is_null());
    }
    #[test]
    fn catalogue_unique_and_readable() {
        let c = catalogue();
        let mut ids = std::collections::HashSet::new();
        for i in c["items"].as_array().unwrap() {
            assert!(ids.insert(i["id"].as_str().unwrap()));
            assert!(!i["documents"].as_array().unwrap().is_empty());
            assert!(ensure_doc(i, "../../.env").is_err());
        }
    }
    #[test]
    fn promises_cannot_be_outcome_evidence() {
        let dims = [
            "security",
            "revenue",
            "services",
            "sovereignty",
            "accountability",
        ];
        let rows:Vec<_>=dims.iter().map(|d|json!({"dimension":d,"score":2,"document_id":"contract","quote":"We will build roads and deliver services."})).collect();
        let input = json!({"documents":[{"id":"contract","role":"agreement","status":"available","text":"We will build roads and deliver services."}]});
        let mut out = json!({"design":rows,"outcomes":rows});
        validate_analysis(&mut out, &input).unwrap();
        assert_eq!(out["design"][0]["score"], 2);
        assert!(out["outcomes"][0]["score"].is_null());
    }
}
