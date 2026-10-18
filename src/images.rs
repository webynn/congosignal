//! Web-search images for profiles and equipment that have no Wikidata/Commons image (and, for
//! equipment, no 3D model). OpenAI web search proposes an image and its source page; the server
//! then verifies independently that the URL serves a real raster image and that the source page
//! names the subject. Only links are stored. Nothing is generated: a photo of a real person must
//! come from a real page that identifies them. Analysts can reject any image.
use crate::text::{self, Doc};
use crate::{ApiError, ApiResult, App, reference};
use anyhow::{Context, Result};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

/// Something that needs an image: a profile or an equipment type.
pub struct Subject {
    pub kind: &'static str,
    pub id: String,
    pub name: String,
    /// Disambiguating context for the search (role, affiliation, nationality, birth year).
    pub context: String,
    /// Names the source page must contain.
    pub names: Vec<String>,
    pub person: bool,
    pub priority: i64,
}

fn api_key() -> Option<String> {
    std::env::var("OPENAI_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
}
fn enabled() -> bool {
    api_key().is_some()
        && reference::sync_enabled()
        && std::env::var("IMAGE_SEARCH_ENABLED").as_deref() != Ok("0")
}
fn model() -> String {
    std::env::var("OPENAI_IMAGE_MODEL").unwrap_or_else(|_| "gpt-5-mini".into())
}
fn daily_limit() -> i64 {
    std::env::var("IMAGE_SEARCH_DAILY_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(40)
        .clamp(0, 500)
}
// Test-only override; the API key is sent to this origin.
fn api_url() -> String {
    std::env::var("OPENAI_API_URL").unwrap_or_else(|_| "https://api.openai.com/v1/responses".into())
}

pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql(
        r#"
        CREATE TABLE IF NOT EXISTS web_images (kind TEXT NOT NULL, subject_id TEXT NOT NULL, status TEXT NOT NULL,
          image_url TEXT, source_url TEXT, source_title TEXT, caption TEXT, checks TEXT, model TEXT,
          searched_at TEXT NOT NULL, error TEXT, PRIMARY KEY(kind, subject_id));
        CREATE TABLE IF NOT EXISTS web_image_requests (id INTEGER PRIMARY KEY AUTOINCREMENT, day TEXT NOT NULL);
        "#,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Shown images by subject id: verified (name on source page) or unverified (page unreachable).
pub async fn found(db: &SqlitePool, kind: &str) -> Result<HashMap<String, Value>> {
    Ok(sqlx::query("SELECT * FROM web_images WHERE kind=? AND status IN ('verified','unverified')")
        .bind(kind)
        .fetch_all(db)
        .await?
        .iter()
        .map(|r| {
            (
                r.get::<String, _>("subject_id"),
                json!({"thumb":r.get::<Option<String>,_>("image_url"),"page":r.get::<Option<String>,_>("source_url"),
                       "source":"web_search","status":r.get::<String,_>("status"),
                       "source_title":r.get::<Option<String>,_>("source_title"),"caption":r.get::<Option<String>,_>("caption"),
                       "checks":r.get::<Option<String>,_>("checks").and_then(|c| serde_json::from_str::<Value>(&c).ok()),
                       "searched_at":r.get::<String,_>("searched_at")}),
            )
        })
        .collect())
}

// ---------------------------------------------------------------------------------------------
// Search

fn schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["found","image_url","source_page_url","source_title","caption"],
        "properties":{"found":{"type":"boolean"},"image_url":{"type":["string","null"]},"source_page_url":{"type":["string","null"]},
                      "source_title":{"type":["string","null"]},"caption":{"type":["string","null"]}}})
}
/// The model's JSON answer from a Responses API body.
fn output_json(body: &Value) -> Result<Value> {
    let text = body["output"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|o| o["type"] == "message")
        .flat_map(|o| o["content"].as_array().into_iter().flatten())
        .filter(|c| c["type"] == "output_text")
        .filter_map(|c| c["text"].as_str())
        .collect::<String>();
    let start = text.find('{').context("no JSON in model output")?;
    let end = text.rfind('}').context("no JSON in model output")?;
    Ok(serde_json::from_str(&text[start..=end])?)
}
async fn ask(client: &reqwest::Client, key: &str, s: &Subject) -> Result<Value> {
    let instructions = if s.person {
        "You find existing public photographs of named public figures for an open-source research tool about the Democratic Republic of the Congo. \
         Search the web. Return a DIRECT image file URL (jpg, png or webp) that appears on a page which names this exact person, preferably Wikimedia Commons, a government or UN page, or a reputable news outlet. \
         The person must be the subject of the photo. If several people share the name, use the context to pick the right one. \
         If you are not confident the photo shows this exact person, return found=false. Never return logos, flags, illustrations, maps, or AI-generated images. Never invent URLs."
    } else {
        "You find existing public images of military equipment types or organizations for an open-source research tool about the Democratic Republic of the Congo. \
         Search the web. Return a DIRECT image file URL (jpg, png or webp) from a page that names the subject, preferably Wikimedia Commons, a government page or a reputable outlet. \
         If unsure, return found=false. Never return AI-generated images. Never invent URLs."
    };
    let request = json!({
        "model": model(),
        "tools": [{"type": "web_search"}],
        "instructions": instructions,
        "input": format!("Subject: {}\nContext: {}", s.name, s.context),
        "text": {"format": {"type": "json_schema", "name": "image_result", "strict": true, "schema": schema()}},
    });
    let r = client
        .post(api_url())
        .bearer_auth(key)
        .json(&request)
        .send()
        .await
        .context("OpenAI connection failed")?;
    let status = r.status();
    let body: Value = r.json().await.unwrap_or_default();
    if !status.is_success() {
        let msg: String = body
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("")
            .chars()
            .take(200)
            .collect();
        anyhow::bail!("OpenAI HTTP {}: {msg}", status.as_u16());
    }
    output_json(&body)
}

// ---------------------------------------------------------------------------------------------
// Independent verification

fn https(u: &str) -> Option<url::Url> {
    url::Url::parse(u.trim())
        .ok()
        .filter(|u| u.scheme() == "https" && u.host_str().is_some())
}
/// Raster image of a plausible size; SVG is refused (it can carry script).
async fn image_ok(client: &reqwest::Client, url: &url::Url) -> Result<String> {
    let mut r = client
        .get(url.clone())
        .send()
        .await
        .context("image unreachable")?;
    anyhow::ensure!(
        r.status().is_success(),
        "image HTTP {}",
        r.status().as_u16()
    );
    let kind = r
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    anyhow::ensure!(
        ["image/jpeg", "image/png", "image/webp", "image/gif"]
            .iter()
            .any(|t| kind.starts_with(t)),
        "not a raster image ({kind})"
    );
    let mut size = 0;
    while let Some(chunk) = r.chunk().await? {
        size += chunk.len();
        anyhow::ensure!(size <= 15 << 20, "image larger than 15 MB");
    }
    anyhow::ensure!(size >= 4096, "image too small ({size} bytes)");
    Ok(kind)
}
/// Whether text names the subject: a full name variant, or for people at least two name tokens
/// including the surname (handles "MAKENGA, Sultani").
fn names_subject(text: &str, s: &Subject) -> bool {
    let doc = Doc::new(text);
    if s.names.iter().any(|n| doc.has_term(&text::term(n))) {
        return true;
    }
    let toks = text::tokens(&s.name);
    s.person
        && toks.len() >= 2
        && doc.has(&[toks[toks.len() - 1].clone()])
        && toks.iter().filter(|t| doc.has(&[(*t).clone()])).count() >= 2
}
fn strip(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let mut tag = false;
    for c in html.chars() {
        match c {
            '<' => tag = true,
            '>' => {
                tag = false;
                out.push(' ');
            }
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    out
}
async fn verify(client: &reqwest::Client, s: &Subject, answer: &Value) -> (String, Value) {
    let image = answer["image_url"].as_str().and_then(https);
    let page = answer["source_page_url"].as_str().and_then(https);
    let Some(image) = image else {
        return (
            "rejected_auto".into(),
            json!({"image":"missing or not https"}),
        );
    };
    let image_check = image_ok(client, &image).await;
    if let Err(e) = &image_check {
        return ("rejected_auto".into(), json!({"image":e.to_string()}));
    }
    let Some(page) = page else {
        return (
            "unverified".into(),
            json!({"image":"ok","page":"no source page given"}),
        );
    };
    let page_body = async {
        let r = client.get(page.clone()).send().await?;
        anyhow::ensure!(r.status().is_success(), "page HTTP {}", r.status().as_u16());
        let bytes = r.bytes().await?;
        anyhow::ensure!(bytes.len() <= 8 << 20, "page too large");
        Ok::<String, anyhow::Error>(String::from_utf8_lossy(&bytes).into_owned())
    }
    .await;
    match page_body {
        Err(e) => (
            "unverified".into(),
            json!({"image":"ok","page":e.to_string()}),
        ),
        Ok(html) => {
            let named = names_subject(&strip(&html), s);
            let file = image.path().rsplit('/').next().unwrap_or("").to_string();
            let on_page = !file.is_empty() && html.contains(&file);
            let checks =
                json!({"image":"ok","page":"ok","names_subject":named,"image_on_page":on_page});
            if named || (!s.person && on_page) {
                ("verified".into(), checks)
            } else {
                ("rejected_auto".into(), checks)
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Worker

async fn due(db: &SqlitePool, kind: &str, id: &str) -> Result<bool> {
    let row =
        sqlx::query("SELECT status, searched_at FROM web_images WHERE kind=? AND subject_id=?")
            .bind(kind)
            .bind(id)
            .fetch_optional(db)
            .await?;
    let Some(r) = row else { return Ok(true) };
    let status: String = r.get("status");
    let age = chrono::DateTime::parse_from_rfc3339(&r.get::<String, _>("searched_at"))
        .map(|t| Utc::now().signed_duration_since(t))
        .unwrap_or(Duration::days(999));
    Ok(match status.as_str() {
        "verified" | "unverified" | "rejected" => false,
        "error" => age > Duration::days(1),
        _ => age > Duration::days(30),
    })
}
async fn reserve(db: &SqlitePool) -> Result<bool> {
    let day = Utc::now().format("%Y-%m-%d").to_string();
    Ok(sqlx::query("INSERT INTO web_image_requests(day) SELECT ? WHERE (SELECT count(*) FROM web_image_requests WHERE day=?)<?")
        .bind(&day)
        .bind(&day)
        .bind(daily_limit())
        .execute(db)
        .await?
        .rows_affected()
        == 1)
}
async fn store(
    db: &SqlitePool,
    s: &Subject,
    status: &str,
    answer: &Value,
    checks: Value,
    error: Option<String>,
) -> Result<()> {
    sqlx::query("INSERT INTO web_images(kind,subject_id,status,image_url,source_url,source_title,caption,checks,model,searched_at,error) VALUES(?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(kind,subject_id) DO UPDATE SET status=excluded.status,image_url=excluded.image_url,source_url=excluded.source_url,source_title=excluded.source_title,caption=excluded.caption,checks=excluded.checks,model=excluded.model,searched_at=excluded.searched_at,error=excluded.error")
        .bind(s.kind)
        .bind(&s.id)
        .bind(status)
        .bind(answer["image_url"].as_str())
        .bind(answer["source_page_url"].as_str())
        .bind(answer["source_title"].as_str())
        .bind(answer["caption"].as_str())
        .bind(checks.to_string())
        .bind(model())
        .bind(Utc::now().to_rfc3339())
        .bind(error)
        .execute(db)
        .await?;
    Ok(())
}
async fn cycle(
    db: &SqlitePool,
    openai: &reqwest::Client,
    web: &reqwest::Client,
    key: &str,
) -> Result<()> {
    let mut subjects = crate::actors::image_subjects(db).await?;
    subjects.extend(crate::arms::image_subjects(db).await?);
    subjects.sort_by_key(|s| std::cmp::Reverse(s.priority));
    for s in subjects {
        if !due(db, s.kind, &s.id).await? {
            continue;
        }
        if !reserve(db).await? {
            break;
        }
        match ask(openai, key, &s).await {
            Ok(answer) if answer["found"] == true => {
                let (status, checks) = verify(web, &s, &answer).await;
                store(db, &s, &status, &answer, checks, None).await?;
            }
            Ok(answer) => store(db, &s, "none", &answer, json!({}), None).await?,
            Err(e) => {
                tracing::warn!(subject = %s.id, error = %e, "image search failed");
                store(db, &s, "error", &json!({}), json!({}), Some(e.to_string())).await?;
                if e.to_string().contains("HTTP 401") || e.to_string().contains("HTTP 429") {
                    break;
                }
            }
        }
    }
    Ok(())
}
pub async fn worker(db: SqlitePool) -> Result<()> {
    let openai = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .build()?;
    let web = reqwest::Client::builder()
        .user_agent(reference::user_agent())
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;
    loop {
        if enabled()
            && let Some(key) = api_key()
            && let Err(e) = cycle(&db, &openai, &web, &key).await
        {
            tracing::warn!(error = %e, "image search cycle failed");
        }
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}

// ---------------------------------------------------------------------------------------------
// API

pub async fn reject(
    State(app): State<App>,
    Path((kind, id)): Path<(String, String)>,
) -> ApiResult<Json<Value>> {
    if !matches!(kind.as_str(), "actor" | "equipment") {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "kind must be actor or equipment".into(),
        ));
    }
    let n = sqlx::query(
        "UPDATE web_images SET status='rejected', searched_at=? WHERE kind=? AND subject_id=?",
    )
    .bind(Utc::now().to_rfc3339())
    .bind(&kind)
    .bind(&id)
    .execute(&app.db)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "no web image for this subject".into(),
        ));
    }
    Ok(Json(json!({"rejected":true})))
}
pub async fn status(State(app): State<App>) -> ApiResult<Json<Value>> {
    let day = Utc::now().format("%Y-%m-%d").to_string();
    let used: i64 = sqlx::query_scalar("SELECT count(*) FROM web_image_requests WHERE day=?")
        .bind(day)
        .fetch_one(&app.db)
        .await?;
    let counts: Vec<Value> = sqlx::query("SELECT kind, status, count(*) AS n FROM web_images GROUP BY kind, status")
        .fetch_all(&app.db)
        .await?
        .iter()
        .map(|r| json!({"kind":r.get::<String,_>("kind"),"status":r.get::<String,_>("status"),"count":r.get::<i64,_>("n")}))
        .collect();
    Ok(Json(
        json!({"enabled":enabled(),"configured":api_key().is_some(),"model":model(),"requests_today":used,"daily_limit":daily_limit(),"counts":counts}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn makenga() -> Subject {
        Subject {
            kind: "actor",
            id: "sultani-makenga".into(),
            name: "Sultani Makenga".into(),
            context: String::new(),
            names: vec!["Sultani Makenga".into()],
            person: true,
            priority: 0,
        }
    }
    #[test]
    fn responses_output_is_parsed() {
        let body = json!({"output":[{"type":"web_search_call"},{"type":"message","content":[{"type":"output_text","text":"{\"found\":true,\"image_url\":\"https://x.org/a.jpg\",\"source_page_url\":\"https://x.org/p\",\"source_title\":null,\"caption\":null}"}]}]});
        assert_eq!(
            output_json(&body).unwrap()["image_url"],
            "https://x.org/a.jpg"
        );
        assert!(output_json(&json!({"output":[]})).is_err());
    }
    #[test]
    fn source_page_must_name_the_person() {
        let s = makenga();
        assert!(names_subject("Le général Sultani Makenga à Rutshuru", &s));
        assert!(names_subject("MAKENGA, Sultani (UN list)", &s));
        assert!(!names_subject("Le chef du M23 à Rutshuru", &s));
        assert!(
            !names_subject("Makenga est un nom courant", &s),
            "surname alone is not enough"
        );
        assert!(https("http://x.org/a.jpg").is_none());
        assert!(https("javascript:alert(1)").is_none());
    }
}
