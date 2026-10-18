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

/// OPENAI_API_KEY, or OPENAI_KEY as an alias.
/// Unix time until which searches pause after an OpenAI auth or credit/rate error.
static PAUSED_UNTIL: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
fn api_key() -> Option<String> {
    ["OPENAI_API_KEY", "OPENAI_KEY"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .map(|k| k.trim().trim_matches('"').to_string())
        .find(|k| !k.is_empty())
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
                       "license":r.get::<Option<String>,_>("checks").and_then(|c| serde_json::from_str::<Value>(&c).ok()).and_then(|c| c.pointer("/commons/license").cloned()),
                       "license_url":r.get::<Option<String>,_>("checks").and_then(|c| serde_json::from_str::<Value>(&c).ok()).and_then(|c| c.pointer("/commons/license_url").cloned()),
                       "artist":r.get::<Option<String>,_>("checks").and_then(|c| serde_json::from_str::<Value>(&c).ok()).and_then(|c| c.pointer("/commons/artist").cloned()),
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
    } else if s.kind == "actor" {
        "You find the official emblem, logo or flag of a named organization (armed group, army, party, company or mission) for an open-source research tool about the Democratic Republic of the Congo. \
         Search the web, preferring Wikimedia Commons. Return a DIRECT image file URL whose file name or caption names the organization. \
         Never return portraits of individuals, photos of other forces, or unrelated scenes; if no emblem, logo or flag exists, return found=false. Never invent URLs."
    } else {
        "You find an existing photograph of a military equipment type for an open-source research tool about the Democratic Republic of the Congo. \
         Search the web, preferring Wikimedia Commons. Return a DIRECT image file URL whose file name or caption names this equipment type. \
         If unsure, return found=false. Never return AI-generated images, diagrams of other systems, or invented URLs."
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
/// Upstream throttling or outage: retry later instead of rejecting the candidate.
#[derive(Debug)]
struct Transient(String);
impl std::fmt::Display for Transient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Transient {}
/// Commons file name for an upload.wikimedia.org URL (original or /thumb/ variant).
fn commons_file(url: &url::Url) -> Option<String> {
    if url.host_str() != Some("upload.wikimedia.org") {
        return None;
    }
    let segs: Vec<&str> = url.path_segments()?.collect();
    let name = match segs.iter().position(|s| *s == "thumb") {
        Some(i) => segs.get(i + 3)?,
        None => segs.last()?,
    };
    url::form_urlencoded::parse(name.as_bytes())
        .map(|(k, _)| k.into_owned())
        .next()
        .filter(|n| !n.is_empty())
}
fn wikimedia(url: &url::Url) -> bool {
    url.host_str()
        .is_some_and(|h| h.ends_with("wikimedia.org") || h.ends_with("wikipedia.org"))
}
/// Raster image of a plausible size; SVG is refused (it can carry script).
async fn image_ok(client: &reqwest::Client, url: &url::Url) -> Result<String> {
    if wikimedia(url) {
        reference::wikimedia_turn().await;
    }
    let mut r = client
        .get(url.clone())
        .send()
        .await
        .map_err(|e| Transient(format!("image unreachable: {e}")))?;
    let code = r.status().as_u16();
    if code == 429 || r.status().is_server_error() {
        return Err(Transient(format!("image HTTP {code}")).into());
    }
    anyhow::ensure!(r.status().is_success(), "image HTTP {code}");
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
/// Verify a candidate. Commons images are checked through the Commons API (no file download),
/// which also records license and author; `answer` is updated to the Commons thumbnail and page.
async fn verify(client: &reqwest::Client, s: &Subject, answer: &mut Value) -> (String, Value) {
    let image = answer["image_url"].as_str().and_then(https);
    let page = answer["source_page_url"].as_str().and_then(https);
    let Some(image) = image else {
        return (
            "rejected_auto".into(),
            json!({"image":"missing or not https"}),
        );
    };
    // Organizations and equipment must be identified by the image itself (file name, caption, title).
    let path: String = url::form_urlencoded::parse(image.path().replace('_', " ").as_bytes())
        .map(|(k, _)| k.into_owned())
        .collect();
    let meta = format!(
        "{} . {} . {}",
        answer["caption"].as_str().unwrap_or(""),
        answer["source_title"].as_str().unwrap_or(""),
        path
    );
    let meta_names = names_subject(&meta, s);
    if !s.person {
        let folded = text::fold(&meta);
        let emblem = [
            "logo",
            "emblem",
            "embleme",
            "flag",
            "drapeau",
            "insignia",
            "seal",
            "coat of arms",
            "armoiries",
            "badge",
        ]
        .iter()
        .any(|w| folded.contains(w));
        let reason = if !meta_names {
            Some("image file name or caption does not name the subject")
        } else if s.kind == "actor" && !emblem {
            Some("organizations only use an emblem, logo or flag")
        } else {
            None
        };
        if let Some(reason) = reason {
            return (
                "rejected_auto".into(),
                json!({"image":"not checked","reason":reason}),
            );
        }
    }
    if let Some(file) = commons_file(&image) {
        return match reference::commons_image(client, &file).await {
            Ok(info) if info["thumb"].is_string() => {
                answer["image_url"] = info["thumb"].clone();
                answer["source_page_url"] = info["page"].clone();
                let named = meta_names
                    || names_subject(
                        &format!("{file} {}", info["credit"].as_str().unwrap_or("")),
                        s,
                    );
                let checks =
                    json!({"image":"ok (Commons)","meta_names_subject":named,"commons":info});
                (if named { "verified" } else { "unverified" }.into(), checks)
            }
            Ok(_) => (
                "rejected_auto".into(),
                json!({"image":"not found on Commons"}),
            ),
            Err(e) => ("error".into(), json!({"image":e.to_string()})),
        };
    }
    if let Err(e) = image_ok(client, &image).await {
        let status = if e.is::<Transient>() {
            "error"
        } else {
            "rejected_auto"
        };
        return (status.into(), json!({"image":e.to_string()}));
    }
    let named_status = |named: bool| if named { "verified" } else { "unverified" }.to_string();
    let Some(page) = page else {
        return (
            named_status(meta_names),
            json!({"image":"ok","page":"no source page given","meta_names_subject":meta_names}),
        );
    };
    if wikimedia(&page) {
        reference::wikimedia_turn().await;
    }
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
            named_status(meta_names),
            json!({"image":"ok","page":e.to_string(),"meta_names_subject":meta_names}),
        ),
        Ok(html) => {
            let named = names_subject(&strip(&html), s);
            let file = image.path().rsplit('/').next().unwrap_or("").to_string();
            let on_page = !file.is_empty() && html.contains(&file);
            let checks = json!({"image":"ok","page":"ok","names_subject":named,"meta_names_subject":meta_names,"image_on_page":on_page});
            if named || meta_names {
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
            Ok(mut answer) if answer["found"] == true => {
                let (status, checks) = verify(web, &s, &mut answer).await;
                store(db, &s, &status, &answer, checks, None).await?;
            }
            Ok(answer) => store(db, &s, "none", &answer, json!({}), None).await?,
            Err(e) => {
                tracing::warn!(subject = %s.id, error = %e, "image search failed");
                let account = ["HTTP 401", "HTTP 403", "HTTP 429"]
                    .iter()
                    .any(|c| e.to_string().contains(c));
                if account {
                    // Credit, auth or rate problems are not the subject's fault: pause, don't mark it.
                    PAUSED_UNTIL.store(
                        Utc::now().timestamp() + 3600,
                        std::sync::atomic::Ordering::Relaxed,
                    );
                    break;
                }
                store(db, &s, "error", &json!({}), json!({}), Some(e.to_string())).await?;
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
            && Utc::now().timestamp() >= PAUSED_UNTIL.load(std::sync::atomic::Ordering::Relaxed)
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
fn paused_until() -> Option<String> {
    let t = PAUSED_UNTIL.load(std::sync::atomic::Ordering::Relaxed);
    (t > Utc::now().timestamp())
        .then(|| chrono::DateTime::from_timestamp(t, 0).map(|d| d.to_rfc3339()))
        .flatten()
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
        json!({"paused_until":paused_until(),"enabled":enabled(),"configured":api_key().is_some(),"model":model(),"requests_today":used,"daily_limit":daily_limit(),"counts":counts}),
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
    #[tokio::test]
    async fn organizations_need_a_named_emblem() {
        let web = reqwest::Client::new();
        let adf = Subject {
            kind: "actor",
            id: "adf".into(),
            name: "Allied Democratic Forces (ADF)".into(),
            context: String::new(),
            names: vec!["ADF".into(), "Allied Democratic Forces".into()],
            person: false,
            priority: 0,
        };
        let scene = json!({"image_url":"https://upload.wikimedia.org/x/Joint_MONUSCO-FARDC_operation_against_ADF_in_Beni.jpg","caption":"UN soldier during an operation against the Allied Democratic Forces (ADF)","source_title":null,"source_page_url":null});
        assert_eq!(
            verify(&web, &adf, &mut scene.clone()).await.0,
            "rejected_auto"
        );
        let rdf = Subject {
            kind: "actor",
            id: "rdf".into(),
            name: "Rwanda Defence Force (RDF)".into(),
            context: String::new(),
            names: vec!["Rwanda Defence Force".into()],
            person: false,
            priority: 0,
        };
        let portrait = json!({"image_url":"https://example.org/Corneille_Nangaa.jpg","caption":"Corneille Nangaa","source_title":null,"source_page_url":null});
        assert_eq!(
            verify(&web, &rdf, &mut portrait.clone()).await.0,
            "rejected_auto"
        );
        let thumb = url::Url::parse("https://upload.wikimedia.org/wikipedia/commons/thumb/b/b2/Rwanda_Defence_Force_emblem.png/480px-Rwanda_Defence_Force_emblem.png").unwrap();
        assert_eq!(
            commons_file(&thumb).as_deref(),
            Some("Rwanda_Defence_Force_emblem.png")
        );
        let original = url::Url::parse(
            "https://upload.wikimedia.org/wikipedia/commons/7/7c/Kisangani%2C_RD_Congo.jpg?utm=x",
        )
        .unwrap();
        assert_eq!(
            commons_file(&original).as_deref(),
            Some("Kisangani,_RD_Congo.jpg")
        );
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
