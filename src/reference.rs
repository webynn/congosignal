//! Shared plumbing for reference data (sanctions lists, Wikidata, Commons, Sketchfab) and the
//! searchable corpus (news, archived X posts, primary documents) used for mention extraction.
use crate::text::Doc;
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::collections::BTreeSet;
use std::sync::LazyLock;
use tokio::sync::Notify;

/// Wakes the reference workers for an immediate refresh.
pub static REFRESH: LazyLock<Notify> = LazyLock::new(Notify::new);

/// REFERENCE_SYNC=0 disables network refreshes (tests, offline use); mention scans still run.
pub fn sync_enabled() -> bool {
    std::env::var("REFERENCE_SYNC").as_deref() != Ok("0")
}
/// Wikimedia throttles generic agents; identify the tool and, when configured, an operator contact.
pub fn user_agent() -> String {
    let base = std::env::var("CRAWLER_USER_AGENT").unwrap_or_else(|_| "DrcOsintWorker/0.1".into());
    let contact = std::env::var("REFERENCE_CONTACT")
        .map(|c| format!("; contact: {c}"))
        .unwrap_or_default();
    format!("{base} (Congo Signal DRC open-source research{contact})")
}
/// True when an upstream asked us to slow down; callers stop the cycle instead of recording failures.
pub fn throttled(e: &anyhow::Error) -> bool {
    e.to_string().contains("HTTP 429")
}
pub fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(user_agent())
        .timeout(std::time::Duration::from_secs(180))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()?)
}
/// Next permitted Wikimedia request time, shared by every worker (Wikimedia rate-limits per client).
static WIKIMEDIA_SLOT: LazyLock<tokio::sync::Mutex<tokio::time::Instant>> =
    LazyLock::new(|| tokio::sync::Mutex::new(tokio::time::Instant::now()));
/// GET with a response size cap; errors never include upstream bodies. Wikimedia requests are
/// spaced out, and 429 responses are retried after the server's Retry-After (capped).
pub async fn get_bytes(client: &reqwest::Client, url: &str, cap: usize) -> Result<Vec<u8>> {
    let wikimedia = url.contains("wikidata.org") || url.contains("wikimedia.org");
    let mut attempt = 0;
    let mut r = loop {
        if wikimedia {
            let mut slot = WIKIMEDIA_SLOT.lock().await;
            tokio::time::sleep_until(*slot).await;
            *slot = tokio::time::Instant::now() + std::time::Duration::from_millis(1200);
        }
        let r = client.get(url).send().await.context("connection failed")?;
        if r.status().as_u16() != 429 || attempt >= 3 {
            break r;
        }
        attempt += 1;
        let wait = r
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(10)
            .min(60);
        tokio::time::sleep(std::time::Duration::from_secs(wait + attempt)).await;
    };
    anyhow::ensure!(r.status().is_success(), "HTTP {}", r.status().as_u16());
    let mut out = Vec::new();
    while let Some(chunk) = r.chunk().await? {
        out.extend_from_slice(&chunk);
        anyhow::ensure!(out.len() <= cap, "response larger than {} MB", cap >> 20);
    }
    Ok(out)
}
pub async fn get_json(client: &reqwest::Client, url: &str) -> Result<Value> {
    Ok(serde_json::from_slice(
        &get_bytes(client, url, 8 << 20).await?,
    )?)
}

pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS reference_sync (id TEXT PRIMARY KEY, checked_at TEXT, status TEXT, error TEXT, records INTEGER, detail TEXT);")
        .execute(db)
        .await?;
    Ok(())
}
pub async fn due(db: &SqlitePool, id: &str, every_seconds: i64) -> Result<bool> {
    let checked: Option<String> =
        sqlx::query_scalar("SELECT checked_at FROM reference_sync WHERE id=?")
            .bind(id)
            .fetch_optional(db)
            .await?
            .flatten();
    Ok(checked
        .and_then(|c| chrono::DateTime::parse_from_rfc3339(&c).ok())
        .is_none_or(|c| Utc::now().signed_duration_since(c).num_seconds() >= every_seconds))
}
pub async fn record(
    db: &SqlitePool,
    id: &str,
    result: &Result<usize>,
    detail: Value,
) -> Result<()> {
    let (status, error, records) = match result {
        Ok(n) => ("ok", None, Some(*n as i64)),
        Err(e) => ("failed", Some(e.to_string()), None),
    };
    // A failure keeps the previous record count so earlier data stays described.
    sqlx::query("INSERT INTO reference_sync(id,checked_at,status,error,records,detail) VALUES(?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET checked_at=excluded.checked_at,status=excluded.status,error=excluded.error,records=coalesce(excluded.records,records),detail=excluded.detail")
        .bind(id)
        .bind(Utc::now().to_rfc3339())
        .bind(status)
        .bind(error)
        .bind(records)
        .bind(detail.to_string())
        .execute(db)
        .await?;
    Ok(())
}
pub async fn sync_rows(db: &SqlitePool, prefix: &str) -> Result<Vec<Value>> {
    Ok(sqlx::query("SELECT * FROM reference_sync WHERE id LIKE ? ORDER BY id")
        .bind(format!("{prefix}%"))
        .fetch_all(db)
        .await?
        .iter()
        .map(|r| json!({"id":r.get::<String,_>("id"),"checked_at":r.get::<Option<String>,_>("checked_at"),"status":r.get::<Option<String>,_>("status"),"error":r.get::<Option<String>,_>("error"),"records":r.get::<Option<i64>,_>("records"),"detail":r.get::<Option<String>,_>("detail").and_then(|d|serde_json::from_str::<Value>(&d).ok())}))
        .collect())
}

// ---------------------------------------------------------------------------------------------
// Wikidata / Commons (CC0 data; Commons media carry their own licenses, recorded per image)

const REGION_QIDS: &[&str] = &["Q974", "Q1037", "Q1036", "Q967"];
const REGION_WORDS: &[&str] = &[
    "congo",
    "rwanda",
    "uganda",
    "burundi",
    "m23",
    "fdlr",
    "kivu",
    "ituri",
    "congolese",
    "rwandan",
];
fn claim_ids(e: &Value, prop: &str) -> Vec<String> {
    e.pointer(&format!("/claims/{prop}"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| {
            c.pointer("/mainsnak/datavalue/value/id")
                .and_then(Value::as_str)
        })
        .map(String::from)
        .collect()
}
fn claim_string(e: &Value, prop: &str) -> Option<String> {
    e.pointer(&format!("/claims/{prop}/0/mainsnak/datavalue/value"))
        .and_then(|v| {
            v.as_str()
                .map(String::from)
                .or_else(|| v.get("time").and_then(Value::as_str).map(String::from))
        })
}
fn qualifier_time(c: &Value, prop: &str) -> Option<String> {
    c.pointer(&format!("/qualifiers/{prop}/0/datavalue/value/time"))
        .and_then(Value::as_str)
        .map(|t| t.trim_start_matches('+').chars().take(10).collect())
}
fn time_value(t: Option<String>) -> Option<String> {
    // "+1971-12-25T00:00:00Z" -> "1971-12-25"; month/day of 00 mean unknown precision.
    t.map(|t| {
        t.trim_start_matches('+')
            .chars()
            .take(10)
            .collect::<String>()
            .replace("-00", "")
    })
}
fn strip_html(s: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for c in s.chars() {
        match c {
            '<' => tag = true,
            '>' => tag = false,
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
/// Image thumbnail plus license and author from Commons extmetadata.
async fn commons_image(client: &reqwest::Client, file: &str) -> Result<Value> {
    let url = format!(
        "https://commons.wikimedia.org/w/api.php?action=query&format=json&prop=imageinfo&iiprop=url|extmetadata&iiurlwidth=480&titles={}",
        urlencode(&format!("File:{file}"))
    );
    let body = get_json(client, &url).await?;
    let info = body
        .pointer("/query/pages")
        .and_then(Value::as_object)
        .and_then(|p| p.values().next())
        .and_then(|p| p.pointer("/imageinfo/0"))
        .context("no Commons image info")?;
    let meta = |k: &str| {
        info.pointer(&format!("/extmetadata/{k}/value"))
            .and_then(Value::as_str)
            .map(strip_html)
    };
    Ok(
        json!({"thumb":info["thumburl"],"page":info["descriptionurl"],"file":file,"license":meta("LicenseShortName"),"license_url":meta("LicenseUrl"),"artist":meta("Artist"),"credit":meta("Credit")}),
    )
}
pub fn urlencode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
/// Resolve an English Wikipedia title (redirects followed) to a Wikidata entity.
pub async fn entity_by_title(client: &reqwest::Client, title: &str) -> Result<Option<Value>> {
    let url = format!(
        "https://www.wikidata.org/w/api.php?action=wbgetentities&format=json&sites=enwiki&redirects=yes&props=labels|descriptions|claims|sitelinks&languages=en|fr&sitefilter=enwiki|frwiki&titles={}",
        urlencode(title)
    );
    let body = get_json(client, &url).await?;
    Ok(body["entities"]
        .as_object()
        .and_then(|m| m.values().find(|e| e.get("missing").is_none()).cloned()))
}
pub fn is_human(e: &Value) -> bool {
    claim_ids(e, "P31").iter().any(|q| q == "Q5")
}
/// Guards name-only matches against same-named people elsewhere.
pub fn regional(e: &Value) -> bool {
    let desc = format!(
        "{} {}",
        e.pointer("/descriptions/en/value")
            .and_then(Value::as_str)
            .unwrap_or(""),
        e.pointer("/descriptions/fr/value")
            .and_then(Value::as_str)
            .unwrap_or("")
    )
    .to_lowercase();
    claim_ids(e, "P27")
        .iter()
        .any(|q| REGION_QIDS.contains(&q.as_str()))
        || REGION_WORDS.iter().any(|w| desc.contains(w))
}
async fn labels(client: &reqwest::Client, ids: &BTreeSet<String>) -> Result<Value> {
    let mut out = serde_json::Map::new();
    let ids: Vec<&String> = ids.iter().collect();
    for chunk in ids.chunks(50) {
        let url = format!(
            "https://www.wikidata.org/w/api.php?action=wbgetentities&format=json&props=labels&languages=en|fr&ids={}",
            chunk
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("|")
        );
        let body = get_json(client, &url).await?;
        for (id, e) in body["entities"].as_object().into_iter().flatten() {
            out.insert(id.clone(), json!({"en":e.pointer("/labels/en/value"),"fr":e.pointer("/labels/fr/value").or(e.pointer("/labels/en/value"))}));
        }
    }
    Ok(Value::Object(out))
}
/// Flatten a Wikidata entity into the profile fields the dashboard shows.
pub async fn summarize(client: &reqwest::Client, e: &Value, method: &str) -> Result<Value> {
    let props = [
        "P27", "P19", "P39", "P102", "P241", "P410", "P463", "P1416", "P106", "P495", "P176",
        "P137", "P159", "P488", "P17",
    ];
    let mut wanted = BTreeSet::new();
    for p in props {
        wanted.extend(claim_ids(e, p));
    }
    let names = labels(client, &wanted).await?;
    let list = |p: &str| -> Vec<Value> {
        claim_ids(e, p)
            .iter()
            .map(|q| json!({"id":q,"label":names.get(q)}))
            .collect()
    };
    let positions: Vec<Value> = e
        .pointer("/claims/P39")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| {
            let q = c.pointer("/mainsnak/datavalue/value/id")?.as_str()?;
            Some(json!({"id":q,"label":names.get(q),"start":qualifier_time(c,"P580"),"end":qualifier_time(c,"P582")}))
        })
        .collect();
    let image_file = claim_string(e, "P18").or_else(|| claim_string(e, "P154"));
    let image = match image_file {
        Some(f) => commons_image(client, &f).await.ok(),
        None => None,
    };
    let id = e["id"].as_str().unwrap_or_default();
    Ok(json!({
        "qid": id, "url": format!("https://www.wikidata.org/wiki/{id}"), "match": method,
        "label": {"en": e.pointer("/labels/en/value"), "fr": e.pointer("/labels/fr/value")},
        "description": {"en": e.pointer("/descriptions/en/value"), "fr": e.pointer("/descriptions/fr/value")},
        "wikipedia": {"en": e.pointer("/sitelinks/enwiki/title").and_then(Value::as_str).map(|t| format!("https://en.wikipedia.org/wiki/{}", t.replace(' ', "_"))),
                      "fr": e.pointer("/sitelinks/frwiki/title").and_then(Value::as_str).map(|t| format!("https://fr.wikipedia.org/wiki/{}", t.replace(' ', "_")))},
        "image": image,
        "born": time_value(claim_string(e, "P569")), "inception": time_value(claim_string(e, "P571")),
        "service_entry": time_value(claim_string(e, "P729")),
        "birthplace": list("P19"), "citizenship": list("P27"), "positions": positions,
        "parties": list("P102"), "military_branch": list("P241"), "rank": list("P410"),
        "member_of": list("P463"), "affiliations": list("P1416"), "occupations": list("P106"),
        "origin": list("P495"), "manufacturer": list("P176"), "operators": list("P137"),
        "headquarters": list("P159"), "leaders": list("P488"), "country": list("P17"),
        "x_username": claim_string(e, "P2002"), "website": claim_string(e, "P856"),
    }))
}

// ---------------------------------------------------------------------------------------------
// Corpus

pub struct CorpusDoc {
    pub kind: &'static str,
    pub id: String,
    pub page: Option<i64>,
    pub title: String,
    pub url: String,
    pub at: Option<String>,
    pub doc: Doc,
}
/// News articles, archived X posts and each page of the latest primary-document snapshots.
pub async fn corpus(db: &SqlitePool) -> Result<Vec<CorpusDoc>> {
    let mut out = Vec::new();
    for r in sqlx::query("SELECT id,url,title,content,published_at FROM articles")
        .fetch_all(db)
        .await?
    {
        let title: String = r.get("title");
        out.push(CorpusDoc {
            kind: "report",
            id: r.get("id"),
            page: None,
            doc: Doc::new(&format!("{title}.\n{}", r.get::<String, _>("content"))),
            title,
            url: r.get("url"),
            at: r.get("published_at"),
        });
    }
    let posts = sqlx::query("SELECT p.id,p.text,p.created_at,p.data,a.handle FROM x_posts p JOIN x_accounts a ON a.id=p.author_id")
        .fetch_all(db)
        .await
        .unwrap_or_default();
    for r in posts {
        let data: Value = serde_json::from_str(&r.get::<String, _>("data")).unwrap_or_default();
        out.push(CorpusDoc {
            kind: "post",
            id: r.get("id"),
            page: None,
            title: format!("@{}", r.get::<String, _>("handle")),
            url: data["url"].as_str().unwrap_or_default().to_string(),
            at: r.get("created_at"),
            doc: Doc::new(&r.get::<String, _>("text")),
        });
    }
    let docs = sqlx::query("SELECT e.id,e.data,r.data AS source FROM evidence_snapshots e JOIN evidence_registry r ON r.id=e.source_id WHERE e.rowid=(SELECT max(x.rowid) FROM evidence_snapshots x WHERE x.source_id=e.source_id)")
        .fetch_all(db)
        .await
        .unwrap_or_default();
    for r in docs {
        let data: Value = serde_json::from_str(&r.get::<String, _>("data")).unwrap_or_default();
        let source: Value = serde_json::from_str(&r.get::<String, _>("source")).unwrap_or_default();
        for p in data["pages"].as_array().into_iter().flatten() {
            out.push(CorpusDoc {
                kind: "document",
                id: r.get("id"),
                page: p["page"].as_i64(),
                title: source["title"].as_str().unwrap_or_default().to_string(),
                url: source["url"].as_str().unwrap_or_default().to_string(),
                at: source["reference_date"]
                    .as_str()
                    .map(String::from)
                    .or_else(|| data["fetched_at"].as_str().map(String::from)),
                doc: Doc::new(p["text"].as_str().unwrap_or_default()),
            });
        }
    }
    Ok(out)
}
