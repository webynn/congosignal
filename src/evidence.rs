//! Curated primary-source evidence. Kept separate from live news and map events.
use crate::{ApiError, ApiResult, App, extract::digest};
use anyhow::{Result, bail, ensure};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::{
    collections::HashMap,
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::io::AsyncWriteExt;
use url::Url;
#[derive(Deserialize, Serialize, Clone)]
pub struct Source {
    id: String,
    publisher: String,
    title: String,
    url: String,
    fetch_url: String,
    kind: String,
    format: String,
    symbol: Option<String>,
    language: String,
    reference_date: Option<String>,
    date_kind: String,
    pages: Vec<usize>,
    images: Vec<usize>,
    expected: String,
    notes: String,
    enabled: bool,
}
fn sources() -> Vec<Source> {
    serde_json::from_str(include_str!("../config/evidence-sources.json"))
        .expect("valid curated sources")
}
pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS evidence_sources(id TEXT PRIMARY KEY,status TEXT NOT NULL DEFAULT 'not_requested',checked_at TEXT,last_success_at TEXT,error TEXT);CREATE TABLE IF NOT EXISTS evidence_snapshots(id TEXT PRIMARY KEY,source_id TEXT NOT NULL,sha256 TEXT NOT NULL,fetched_at TEXT NOT NULL,fetched_url TEXT NOT NULL,media_type TEXT NOT NULL,data TEXT NOT NULL,UNIQUE(source_id,sha256));CREATE INDEX IF NOT EXISTS evidence_source_snapshots ON evidence_snapshots(source_id,fetched_at);").execute(db).await?;
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS evidence_registry(id TEXT PRIMARY KEY,data TEXT NOT NULL);CREATE TABLE IF NOT EXISTS evidence_briefs(id TEXT PRIMARY KEY,status TEXT NOT NULL,data TEXT,error TEXT);").execute(db).await?;
    let cols = sqlx::query("PRAGMA table_info(evidence_briefs)")
        .fetch_all(db)
        .await?;
    if !cols
        .iter()
        .any(|r| r.get::<String, _>("name") == "requested_by")
    {
        sqlx::query("ALTER TABLE evidence_briefs ADD COLUMN requested_by TEXT")
            .execute(db)
            .await?;
    }
    for s in sources() {
        sqlx::query("INSERT OR IGNORE INTO evidence_sources(id) VALUES(?)")
            .bind(&s.id)
            .execute(db)
            .await?;
    }
    for s in sources() {
        sqlx::query("INSERT INTO evidence_registry VALUES(?,?) ON CONFLICT(id) DO UPDATE SET data=excluded.data").bind(&s.id).bind(serde_json::to_string(&s)?).execute(db).await?;
    }
    sqlx::query("UPDATE evidence_briefs SET status='pending' WHERE status='running'")
        .execute(db)
        .await?;
    sqlx::query("UPDATE evidence_sources SET status='pending' WHERE status='running'")
        .execute(db)
        .await?;
    Ok(())
}
pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let mut items = Vec::new();
    for s in registry(&app.db).await? {
        let r = sqlx::query("SELECT * FROM evidence_sources WHERE id=?")
            .bind(&s.id)
            .fetch_one(&app.db)
            .await?;
        let mut v = serde_json::to_value(&s).map_err(anyhow::Error::from)?;
        v["status"] = json!(if !s.enabled {
            "reference_only".to_string()
        } else {
            r.get::<String, _>("status")
        });
        v["checked_at"] = json!(r.get::<Option<String>, _>("checked_at"));
        v["last_success_at"] = json!(r.get::<Option<String>, _>("last_success_at"));
        v["error"] = json!(r.get::<Option<String>, _>("error"));
        items.push(v);
    }
    Ok(Json(
        json!({"items":items,"schedule_seconds":poll_seconds(),"scope":"primary_documents","claim_status":"source_attributed_not_independently_verified"}),
    ))
}
pub async fn refresh(
    State(app): State<App>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let s = registry(&app.db)
        .await?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "unknown source".into()))?;
    if !s.enabled {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "reference-only source; automated retrieval not configured".into(),
        ));
    }
    sqlx::query("UPDATE evidence_sources SET status='pending',error=NULL WHERE id=? AND status NOT IN ('pending','running')").bind(&id).execute(&app.db).await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({"source_id":id,"status_url":"/v1/sources"})),
    ))
}
#[derive(Deserialize)]
pub struct Search {
    q: Option<String>,
    source_id: Option<String>,
    page: Option<usize>,
    limit: Option<usize>,
    offset: Option<usize>,
}
pub async fn documents(State(app): State<App>, Query(q): Query<Search>) -> ApiResult<Json<Value>> {
    let rows=sqlx::query("SELECT e.id,e.source_id,e.fetched_at,e.data FROM evidence_snapshots e WHERE e.rowid=(SELECT max(s.rowid) FROM evidence_snapshots s WHERE s.source_id=e.source_id) ORDER BY e.fetched_at DESC").fetch_all(&app.db).await?;
    let needle = q.q.unwrap_or_default().to_lowercase();
    let mut items = Vec::new();
    for r in rows {
        let sid: String = r.get("source_id");
        if q.source_id.as_ref().is_some_and(|v| v != &sid) {
            continue;
        }
        let d: Value =
            serde_json::from_str(r.get::<&str, _>("data")).map_err(anyhow::Error::from)?;
        for page in d["pages"].as_array().unwrap_or(&Vec::new()) {
            let text = page["text"].as_str().unwrap_or("");
            if q.page
                .is_some_and(|n| page["page"].as_u64() != Some(n as u64))
                || !text.to_lowercase().contains(&needle)
            {
                continue;
            }
            items.push(json!({"document_id":r.get::<String,_>("id"),"source_id":sid,"title":d["source"]["title"],"publisher":d["source"]["publisher"],"url":d["source"]["url"],"reference_date":d["source"]["reference_date"],"fetched_at":r.get::<String,_>("fetched_at"),"page":page["page"],"text":text,"claim_status":"source_attributed_not_independently_verified"}));
        }
    }
    let total = items.len();
    let results: Vec<_> = items
        .into_iter()
        .skip(q.offset.unwrap_or(0))
        .take(q.limit.unwrap_or(20).clamp(1, 100))
        .collect();
    Ok(Json(json!({"total":total,"items":results})))
}
pub async fn document(State(app): State<App>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let raw: Option<String> = sqlx::query_scalar("SELECT data FROM evidence_snapshots WHERE id=?")
        .bind(id)
        .fetch_optional(&app.db)
        .await?;
    let raw = raw.ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "document not found".into()))?;
    Ok(Json(
        serde_json::from_str(&raw).map_err(anyhow::Error::from)?,
    ))
}
fn allowed(u: &Url) -> bool {
    u.scheme() == "https"
        && u.port_or_known_default() == Some(443)
        && u.username().is_empty()
        && u.password().is_none()
        && matches!(
            u.host_str(),
            Some(
                "congomines.org"
                    | "wpstorage7ceb827ed5.blob.core.windows.net"
                    | "economie.gouv.cd"
                    | "www.apcsc.gouv.cd"
                    | "documents.un.org"
                    | "digitallibrary.un.org"
                    | "docs.un.org"
                    | "main.un.org"
                    | "peacekeeping.un.org"
                    | "www.hrw.org"
                    | "www.mod.gov.rw"
                    | "www.iiss.org"
            )
        )
}
pub(crate) struct Fetcher {
    client: reqwest::Client,
    robots: HashMap<String, (String, Duration)>,
    last: Instant,
}
impl Fetcher {
    pub(crate) fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(90))
                .user_agent("DrcOsintWorker/0.2")
                .build()?,
            robots: HashMap::new(),
            last: Instant::now() - Duration::from_secs(5),
        })
    }
    async fn wait(&mut self, delay: Duration) {
        tokio::time::sleep(delay.saturating_sub(self.last.elapsed())).await;
        self.last = Instant::now()
    }
    async fn policy(&mut self, u: &Url) -> Result<Duration> {
        let host = u.host_str().unwrap().to_string();
        if !self.robots.contains_key(&host) {
            self.wait(Duration::from_secs(2)).await;
            let r = self
                .client
                .get(format!("https://{host}/robots.txt"))
                .send()
                .await?;
            let status = r.status();
            let text = if status == 404 {
                String::new()
            } else {
                ensure!(
                    status == 200,
                    "robots policy unavailable (HTTP {})",
                    status.as_u16()
                );
                let bytes = bounded(r, 512000).await?;
                let text = String::from_utf8(bytes)?;
                ensure!(
                    !text.trim_start().starts_with('<'),
                    "robots policy returned HTML instead of rules"
                );
                text
            };
            let mut secs = 2f64;
            for line in text.lines() {
                if let Some((k, v)) = line.split_once(':')
                    && k.trim().eq_ignore_ascii_case("crawl-delay")
                    && let Ok(n) = v.trim().parse::<f64>()
                    && n.is_finite()
                {
                    secs = secs.max(n)
                }
            }
            ensure!(secs <= 300., "crawl delay too long");
            self.robots
                .insert(host.clone(), (text, Duration::from_secs_f64(secs)));
        }
        let (rules, delay) = self.robots.get(&host).unwrap();
        ensure!(
            robotstxt::DefaultMatcher::default().one_agent_allowed_by_robots(
                rules,
                "DrcOsintWorker",
                u.as_str()
            ),
            "robots policy disallows document"
        );
        Ok(*delay)
    }
    pub(crate) async fn get(&mut self, input: &str) -> Result<(Url, Vec<u8>, String)> {
        let mut u = Url::parse(input)?;
        for _ in 0..6 {
            ensure!(allowed(&u), "source origin is not allowlisted");
            let delay = self.policy(&u).await?;
            self.wait(delay).await;
            let r = self.client.get(u.clone()).send().await?;
            if r.status().is_redirection() {
                let loc = r
                    .headers()
                    .get("location")
                    .ok_or_else(|| anyhow::anyhow!("redirect missing location"))?
                    .to_str()?;
                u = u.join(loc)?;
                continue;
            }
            ensure!(
                r.status() == 200,
                "source returned HTTP {}",
                r.status().as_u16()
            );
            let ty = r
                .headers()
                .get("content-type")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("")
                .to_string();
            let bytes = bounded(r, 40_000_000).await?;
            return Ok((u, bytes, ty));
        }
        bail!("too many source redirects")
    }
}
async fn bounded(mut r: reqwest::Response, max: usize) -> Result<Vec<u8>> {
    ensure!(
        r.content_length().unwrap_or(0) <= max as u64,
        "source exceeds size limit"
    );
    let mut b = Vec::new();
    while let Some(c) = r.chunk().await? {
        ensure!(b.len() + c.len() <= max, "source exceeds size limit");
        b.extend_from_slice(&c)
    }
    Ok(b)
}
fn html_data(s: &Source, bytes: &[u8], url: &Url) -> Result<(Vec<Value>, Vec<Value>)> {
    let raw = std::str::from_utf8(bytes)?;
    let h = scraper::Html::parse_document(raw);
    let selector = if s.publisher == "Rwanda Ministry of Defence" && s.kind != "discovery" {
        ".news-text-wrap"
    } else if s.publisher == "Human Rights Watch" && s.kind != "discovery" {
        ".article-body"
    } else {
        "main, article, .main-container, #main-content"
    };
    let root = h
        .select(&scraper::Selector::parse(selector).unwrap())
        .next()
        .ok_or_else(|| anyhow::anyhow!("source content container missing"))?;
    let text = root
        .select(&scraper::Selector::parse("h1,h2,h3,p,li,td").unwrap())
        .map(|e| {
            e.text()
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    ensure!(
        text.len() > 150 && text.to_lowercase().contains(&s.expected.to_lowercase()),
        "source content validation failed"
    );
    let mut links = Vec::new();
    for e in root.select(&scraper::Selector::parse("a[href],iframe[src]").unwrap()) {
        if let Some(href) = e.value().attr("href").or_else(|| e.value().attr("src"))
            && let Ok(u) = url.join(href)
            && u.scheme() == "https"
        {
            links.push(json!({"url":u.as_str(),"label":e.text().collect::<Vec<_>>().join(" ")}));
        }
    }
    links.truncate(500);
    Ok((vec![json!({"page":null,"text":text})], links))
}
async fn pdf_data(bytes: Vec<u8>, s: &Source) -> Result<Vec<Value>> {
    ensure!(
        bytes.starts_with(b"%PDF-"),
        "expected a PDF, not an HTML wrapper"
    );
    let bin = std::env::var("PDFTOTEXT_BIN").unwrap_or_else(|_| "pdftotext".into());
    let mut child = tokio::process::Command::new(bin)
        .args(["-layout", "-", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| {
            anyhow::anyhow!(
                "PDF extraction unavailable: install Poppler or configure PDFTOTEXT_BIN"
            )
        })?;
    let mut stdin = child.stdin.take().unwrap();
    let writer = tokio::spawn(async move { stdin.write_all(&bytes).await });
    let out = tokio::time::timeout(Duration::from_secs(120), child.wait_with_output()).await??;
    writer.await??;
    ensure!(out.status.success(), "PDF extraction failed");
    ensure!(out.stdout.len() < 30_000_000, "PDF text too large");
    let text = String::from_utf8(out.stdout)?;
    pdf_pages(&text, s)
}
fn pdf_pages(text: &str, s: &Source) -> Result<Vec<Value>> {
    let first = text.split('\x0c').next().unwrap_or("");
    ensure!(
        first.contains(&s.expected),
        "PDF identity does not match registered document"
    );
    let mut pages: Vec<_> = text
        .split('\x0c')
        .enumerate()
        .map(|(i, p)| json!({"page":i+1,"text":p.trim()}))
        .collect();
    if pages.last().is_some_and(|p| p["text"] == "") {
        pages.pop();
    }
    ensure!(!pages.is_empty(), "PDF has no extractable pages");
    Ok(pages)
}
async fn collect(f: &mut Fetcher, db: &SqlitePool, s: &Source) -> Result<()> {
    let (url, bytes, ty) = f.get(&s.fetch_url).await?;
    if s.kind == "discovery" || s.kind == "catalogue" || s.kind == "statistics_catalogue" {
        discover(db, s, &bytes, &url).await?;
    }
    let mut s = s.clone();
    if s.id.starts_with("discovered-") && s.format == "html" {
        let h = scraper::Html::parse_document(std::str::from_utf8(&bytes)?);
        if let Some(el) = h.select(&scraper::Selector::parse("h1").unwrap()).next() {
            let title = el.text().collect::<Vec<_>>().join(" ").trim().to_string();
            if !title.is_empty() {
                s.title = title;
            }
        }
        s.reference_date = h
            .select(
                &scraper::Selector::parse(
                    "meta[property='article:published_time'], time[datetime]",
                )
                .unwrap(),
            )
            .next()
            .and_then(|e| {
                e.value()
                    .attr("content")
                    .or_else(|| e.value().attr("datetime"))
            })
            .map(str::to_owned);
        s.date_kind = "publisher_metadata_or_unknown".into();
    }
    let sha = digest(&bytes);
    let (pages, links) = if s.format == "pdf" {
        (pdf_data(bytes.clone(), &s).await?, Vec::new())
    } else {
        ensure!(ty.contains("text/html"), "expected HTML");
        html_data(&s, &bytes, &url)?
    };
    let id = digest(format!("{}:{sha}", s.id).as_bytes());
    let now = chrono::Utc::now().to_rfc3339();
    let content = pages
        .iter()
        .filter_map(|p| p["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let relevance = relevance(&content);
    let data = json!({"relevance":relevance,"id":id,"source":s,"fetched_url":url.as_str(),"fetched_at":now,"sha256":sha,"media_type":ty,"pages":pages,"links":links,"extraction":"pdf-layout-text-or-html-v1","claim_status":"source_attributed_not_independently_verified","screenshot_match":"references_registered; inspect cited pages to verify wording"});
    let dir = std::env::var("EVIDENCE_ARCHIVE_DIR").unwrap_or_else(|_| "data/evidence".into());
    tokio::fs::create_dir_all(&dir).await?;
    let dest = std::path::Path::new(&dir).join(format!(
        "{id}.{}",
        if s.format == "pdf" { "pdf" } else { "html" }
    ));
    tokio::fs::write(dest, &bytes).await?;
    sqlx::query("INSERT OR IGNORE INTO evidence_snapshots(id,source_id,sha256,fetched_at,fetched_url,media_type,data) VALUES(?,?,?,?,?,?,?)").bind(id).bind(&s.id).bind(sha).bind(&now).bind(url.as_str()).bind(ty).bind(data.to_string()).execute(db).await?;
    sqlx::query("UPDATE evidence_sources SET status='ready',checked_at=?,last_success_at=?,error=NULL WHERE id=?").bind(&now).bind(&now).bind(&s.id).execute(db).await?;
    Ok(())
}
pub async fn worker(db: SqlitePool) -> Result<()> {
    let mut f = Fetcher::new()?;
    let mut scheduled = Instant::now() - Duration::from_secs(86400);
    loop {
        let interval = poll_seconds();
        if interval > 0 && scheduled.elapsed().as_secs() >= 60 {
            f.robots.clear();
            schedule(&db, interval).await?;
            scheduled = Instant::now();
        }
        run_brief(&db).await?;
        let id: Option<String> = sqlx::query_scalar(
            "SELECT id FROM evidence_sources WHERE status='pending' ORDER BY id LIMIT 1",
        )
        .fetch_optional(&db)
        .await?;
        let Some(id) = id else {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        };
        sqlx::query("UPDATE evidence_sources SET status='running' WHERE id=?")
            .bind(&id)
            .execute(&db)
            .await?;
        if let Some(s) = registry(&db).await?.into_iter().find(|s| s.id == id)
            && let Err(e) = collect(&mut f, &db, &s).await
        {
            sqlx::query(
                "UPDATE evidence_sources SET status='failed',checked_at=?,error=? WHERE id=?",
            )
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(e.to_string())
            .bind(id)
            .execute(&db)
            .await?;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pdf_identity_and_page_numbers() {
        let s = sources().remove(0);
        let pages = pdf_pages("S/2022/967 Cover\x0cSecond page\x0c", &s).unwrap();
        assert_eq!(pages.len(), 2);
        assert_eq!(pages[1]["page"], 2);
        assert_eq!(pages[1]["text"], "Second page");
        assert!(pdf_pages("Wrong cover\x0cS/2022/967 cited in another report", &s).is_err());
    }
    #[test]
    fn origins() {
        assert!(allowed(
            &Url::parse("https://documents.un.org/test").unwrap()
        ));
        for u in [
            "http://documents.un.org/test",
            "https://documents.un.org.evil.com/test",
            "https://127.0.0.1/test",
            "https://documents.un.org:8443/test",
            "https://user@documents.un.org/test",
        ] {
            assert!(!allowed(&Url::parse(u).unwrap()));
        }
    }
    #[test]
    fn catalogue() {
        let ss = sources();
        assert_eq!(ss.len(), 14);
        let unique: std::collections::HashSet<_> = ss.iter().map(|s| &s.id).collect();
        assert_eq!(unique.len(), ss.len());
        for s in ss.iter().filter(|s| s.enabled) {
            assert!(allowed(&Url::parse(&s.fetch_url).unwrap()))
        }
    }
    #[test]
    fn html_identity() {
        let s = sources()
            .into_iter()
            .find(|s| s.id == "rdf-car-2023")
            .unwrap();
        assert!(
            html_data(
                &s,
                b"<main>Access denied</main>",
                &Url::parse(&s.url).unwrap()
            )
            .is_err()
        );
    }
}

fn poll_seconds() -> u64 {
    std::env::var("EVIDENCE_POLL_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(|n| if n == 0 { 0 } else { n.clamp(3600, 604800) })
        .unwrap_or(21600)
}
async fn registry(db: &SqlitePool) -> Result<Vec<Source>> {
    let rows: Vec<String> = sqlx::query_scalar("SELECT data FROM evidence_registry ORDER BY id")
        .fetch_all(db)
        .await?;
    rows.into_iter()
        .map(|r| Ok(serde_json::from_str(&r)?))
        .collect()
}
async fn schedule(db: &SqlitePool, interval: u64) -> Result<()> {
    let cutoff = (chrono::Utc::now() - chrono::Duration::seconds(interval as i64)).to_rfc3339();
    for s in registry(db).await?.into_iter().filter(|s| s.enabled) {
        sqlx::query("UPDATE evidence_sources SET status='pending',error=NULL WHERE id=? AND status NOT IN ('pending','running') AND (checked_at IS NULL OR checked_at < ?)").bind(s.id).bind(&cutoff).execute(db).await?;
    }
    Ok(())
}
fn relevance(text: &str) -> Value {
    let normal = text
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), " ");
    let norm = format!(
        " {} ",
        normal.split_whitespace().collect::<Vec<_>>().join(" ")
    );
    let direct = [
        "drc",
        "rdc",
        "democratic republic of congo",
        "democratic republic of the congo",
        "république démocratique du congo",
        "congolese",
        "congolais",
        "kinshasa",
        "goma",
        "bukavu",
        "ituri",
        "kivu",
        "m23",
        "monusco",
        "fardc",
    ];
    let hits: Vec<_> = direct
        .into_iter()
        .filter(|t| norm.contains(&format!(" {t} ")))
        .collect();
    let scope = if !hits.is_empty() {
        "drc"
    } else if ["rwanda", "rwandan", "burundi", "uganda", "peacekeeping"]
        .iter()
        .any(|t| norm.contains(&format!(" {t} ")))
    {
        "regional"
    } else {
        "excluded"
    };
    json!({"scope":scope,"matched_terms":hits,"method":"explicit_terms_v1","note":"Relevance for review, not verification or incident geolocation"})
}
fn candidate(parent: &Source, url: &Url, label: &str) -> Option<Source> {
    if !allowed(url) {
        return None;
    }
    let host = url.host_str()?;
    let origin = Url::parse(&parent.url).ok()?;
    if origin.host_str() != Some(host)
        && !(origin.host_str() == Some("main.un.org")
            && ["docs.un.org", "digitallibrary.un.org"].contains(&host))
    {
        return None;
    }
    let path = url.path();
    let valid = match host {
        "www.hrw.org" => {
            path.starts_with("/news/")
                || path.starts_with("/report/")
                || path.starts_with("/world-report/")
        }
        "www.mod.gov.rw" => path.starts_with("/docs-media/news-detail/"),
        "www.iiss.org" => {
            path.starts_with("/publications/strategic-comments/")
                && path.trim_matches('/').split('/').count() > 3
        }
        "peacekeeping.un.org" => {
            path.starts_with("/en/document-library/") || path.ends_with(".pdf")
        }
        "docs.un.org" => path.contains("/S/") || path.contains("/s/"),
        "digitallibrary.un.org" => path.ends_with("-EN.pdf"),
        _ => false,
    };
    if !valid {
        return None;
    }
    let mut u = url.clone();
    u.set_query(None);
    u.set_fragment(None);
    let mut source = parent.clone();
    source.id = format!("discovered-{}", digest(u.as_str().as_bytes()));
    source.url = u.to_string();
    source.fetch_url = u.to_string();
    source.title = if label.trim().is_empty() {
        path.into()
    } else {
        label.trim().into()
    };
    source.kind = match host {
        "www.mod.gov.rw" => "government_statement",
        "www.hrw.org" => "ngo_report",
        "www.iiss.org" => "research_analysis",
        _ => "official_report",
    }
    .into();
    source.format = if path.ends_with(".pdf") {
        "pdf"
    } else {
        "html"
    }
    .into();
    source.expected = String::new();
    source.symbol = None;
    if host == "docs.un.org" {
        let parts: Vec<_> = path.trim_matches('/').split('/').collect();
        let i = parts.iter().position(|p| p.eq_ignore_ascii_case("s"))?;
        if parts.len() != i + 3
            || parts[i + 1].len() != 4
            || !parts[i + 1..]
                .iter()
                .all(|p| p.chars().all(|c| c.is_ascii_digit()))
        {
            return None;
        }
        let symbol = format!("S/{}/{}", parts[i + 1], parts[i + 2]);
        source.fetch_url =
            format!("https://documents.un.org/api/symbol/access?s={symbol}&l=en&t=pdf");
        source.format = "pdf".into();
        source.expected = symbol.clone();
        source.symbol = Some(symbol);
    }
    source.reference_date = None;
    source.pages.clear();
    source.images.clear();
    source.notes = format!(
        "Discovered from {}. Automated relevance screening; source claims remain unverified.",
        parent.url
    );
    source.enabled = true;
    Some(source)
}
async fn discover(db: &SqlitePool, parent: &Source, bytes: &[u8], base: &Url) -> Result<()> {
    let candidates = {
        let h = scraper::Html::parse_document(std::str::from_utf8(bytes)?);
        let mut seen = std::collections::HashSet::new();
        h.select(
            &scraper::Selector::parse(
                "main a[href], article a[href], .main-container a[href], .news a[href]",
            )
            .unwrap(),
        )
        .filter_map(|e| {
            let u = base.join(e.value().attr("href")?).ok()?;
            let s = candidate(parent, &u, &e.text().collect::<Vec<_>>().join(" "))?;
            if seen.insert(s.url.clone()) {
                Some(s)
            } else {
                None
            }
        })
        .take(30)
        .collect::<Vec<_>>()
    };
    for s in candidates {
        let existing = registry(db)
            .await?
            .iter()
            .any(|old| old.url == s.url || (old.symbol.is_some() && old.symbol == s.symbol));
        if existing {
            continue;
        }
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM evidence_registry WHERE id LIKE 'discovered-%'",
        )
        .fetch_one(db)
        .await?;
        if count >= 500 {
            break;
        }
        sqlx::query("INSERT OR IGNORE INTO evidence_registry VALUES(?,?)")
            .bind(&s.id)
            .bind(serde_json::to_string(&s)?)
            .execute(db)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO evidence_sources(id,status) VALUES(?,'pending')")
            .bind(&s.id)
            .execute(db)
            .await?;
    }
    Ok(())
}
#[derive(Deserialize)]
pub struct Feed {
    pub scope: Option<String>,
    pub q: Option<String>,
}
pub async fn feed(State(app): State<App>, Query(q): Query<Feed>) -> ApiResult<Json<Value>> {
    let rows=sqlx::query("SELECT e.id,e.data,b.status AS brief_status,b.data AS brief_data,b.error AS brief_error FROM evidence_snapshots e LEFT JOIN evidence_briefs b ON b.id=e.id WHERE e.rowid=(SELECT max(x.rowid) FROM evidence_snapshots x WHERE x.source_id=e.source_id) ORDER BY e.fetched_at DESC").fetch_all(&app.db).await?;
    let mut items = Vec::new();
    for r in rows {
        let d: Value = serde_json::from_str(r.get("data")).map_err(anyhow::Error::from)?;
        if ["discovery", "catalogue", "statistics_catalogue"]
            .contains(&d["source"]["kind"].as_str().unwrap_or(""))
        {
            continue;
        }
        let content = d["pages"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let rel = relevance(&content);
        if rel["scope"] == "excluded"
            || (q.scope.as_deref() != Some("regional") && rel["scope"] != "drc")
        {
            continue;
        }
        if q.q.as_ref().is_some_and(|s| {
            !format!("{} {content}", d["source"]["title"])
                .to_lowercase()
                .contains(&s.to_lowercase())
        }) {
            continue;
        }
        let b = r
            .get::<Option<String>, _>("brief_data")
            .and_then(|s| serde_json::from_str::<Value>(&s).ok());
        items.push(json!({"id":r.get::<String,_>("id"),"source":d["source"],"fetched_at":d["fetched_at"],"relevance":rel,"search_text":content,"excerpt":content.chars().take(400).collect::<String>(),"brief":{"status":r.get::<Option<String>,_>("brief_status").unwrap_or("not_requested".into()),"data":b,"error":r.get::<Option<String>,_>("brief_error")}}));
    }
    Ok(Json(
        json!({"items":items,"schedule_seconds":poll_seconds()}),
    ))
}
pub async fn analyze(
    State(app): State<App>,
    Path(id): Path<String>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let exists: i64 = sqlx::query_scalar("SELECT count(*) FROM evidence_snapshots WHERE id=?")
        .bind(&id)
        .fetch_one(&app.db)
        .await?;
    if exists == 0 {
        return Err(ApiError(StatusCode::NOT_FOUND, "document not found".into()));
    }
    sqlx::query("INSERT INTO evidence_briefs(id,status,requested_by) VALUES(?,'pending',?) ON CONFLICT(id) DO UPDATE SET status='pending',error=NULL,requested_by=excluded.requested_by WHERE status='failed'").bind(&id).bind(serde_json::to_string(&crate::authentication::actor()).map_err(anyhow::Error::from)?).execute(&app.db).await?;
    Ok((StatusCode::ACCEPTED, Json(json!({"id":id}))))
}
async fn run_brief(db: &SqlitePool) -> Result<()> {
    let row=sqlx::query("SELECT b.id,e.data,b.requested_by FROM evidence_briefs b JOIN evidence_snapshots e ON e.id=b.id WHERE b.status='pending' LIMIT 1").fetch_optional(db).await?;
    let Some(r) = row else { return Ok(()) };
    let id: String = r.get("id");
    sqlx::query("UPDATE evidence_briefs SET status='running' WHERE id=?")
        .bind(&id)
        .execute(db)
        .await?;
    let result = async {
        let data: Value = serde_json::from_str(r.get("data"))?;
        crate::authentication::ACTOR
            .scope(
                r.get::<Option<String>, _>("requested_by")
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_else(crate::authentication::actor),
                crate::intelligence::evidence_brief(&data),
            )
            .await
    }
    .await;
    match result {
        Ok(v) => {
            sqlx::query(
                "UPDATE evidence_briefs SET status='completed',data=?,error=NULL WHERE id=?",
            )
            .bind(v.to_string())
            .bind(id)
            .execute(db)
            .await?;
        }
        Err(_) => {
            sqlx::query("UPDATE evidence_briefs SET status='failed',error='Brief generation failed; check AI configuration or retry' WHERE id=?").bind(id).execute(db).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod discovery_tests {
    use super::*;
    #[test]
    fn relevance_boundaries() {
        assert_eq!(
            relevance("Democratic Republic of the Congo: Goma")["scope"],
            "drc"
        );
        assert_eq!(
            relevance("Republic of Congo, Brazzaville")["scope"],
            "excluded"
        );
        assert_eq!(
            relevance("Rwanda trains soldiers in CAR")["scope"],
            "regional"
        );
        assert_eq!(relevance("gomarket and M230")["scope"], "excluded");
        assert_eq!(
            relevance("République démocratique du Congo")["scope"],
            "drc"
        );
    }
    #[tokio::test]
    async fn discovery_dedup_schedule_and_disabled() {
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        let parent = sources()
            .into_iter()
            .find(|s| s.id == "hrw-drc-discovery")
            .unwrap();
        let html=b"<main><a href='/news/2026/10/18/drc-test'>DRC report</a><a href='/news/2026/10/18/drc-test#top'>Duplicate</a><a href='https://evil.example/news/x'>Wrong host</a><a href='/donate'>Donate</a></main>";
        discover(&db, &parent, html, &Url::parse(&parent.url).unwrap())
            .await
            .unwrap();
        discover(&db, &parent, html, &Url::parse(&parent.url).unwrap())
            .await
            .unwrap();
        assert_eq!(registry(&db).await.unwrap().len(), 15);
        sqlx::query(
            "UPDATE evidence_sources SET status='ready',checked_at=? WHERE id='hrw-rwanda-2026'",
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&db)
        .await
        .unwrap();
        schedule(&db, 21600).await.unwrap();
        let status: String =
            sqlx::query_scalar("SELECT status FROM evidence_sources WHERE id='hrw-rwanda-2026'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(status, "ready");
        let disabled: String =
            sqlx::query_scalar("SELECT status FROM evidence_sources WHERE id='hrw-kanombe-thread'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(disabled, "not_requested");
    }
}

/// Serve only a validated archived PDF, never arbitrary files or remote URLs.
pub async fn pdf(
    State(app): State<App>,
    Path(id): Path<String>,
) -> ApiResult<axum::response::Response> {
    use axum::response::IntoResponse;
    if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(ApiError(StatusCode::NOT_FOUND, "PDF not found".into()));
    }
    let row = sqlx::query("SELECT sha256,data FROM evidence_snapshots WHERE id=?")
        .bind(&id)
        .fetch_optional(&app.db)
        .await?;
    let row = row.ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "PDF not found".into()))?;
    let d: Value = serde_json::from_str(row.get("data")).map_err(anyhow::Error::from)?;
    if d["source"]["format"] != "pdf" {
        return Err(ApiError(
            StatusCode::NOT_FOUND,
            "This publication is not a PDF".into(),
        ));
    }
    let dir = std::env::var("EVIDENCE_ARCHIVE_DIR").unwrap_or_else(|_| "data/evidence".into());
    let file = std::path::Path::new(&dir).join(format!("{id}.pdf"));
    let metadata = tokio::fs::metadata(&file)
        .await
        .map_err(|_| ApiError(StatusCode::NOT_FOUND, "Archived PDF is unavailable".into()))?;
    if metadata.len() > 40_000_000 {
        return Err(ApiError(
            StatusCode::PAYLOAD_TOO_LARGE,
            "PDF exceeds size limit".into(),
        ));
    }
    let bytes = tokio::fs::read(file).await.map_err(anyhow::Error::from)?;
    if !bytes.starts_with(b"%PDF-") || digest(&bytes) != row.get::<String, _>("sha256") {
        return Err(ApiError(
            StatusCode::CONFLICT,
            "Archived PDF integrity check failed".into(),
        ));
    }
    Ok((
        [
            ("content-type", "application/pdf"),
            ("content-disposition", "inline; filename=report.pdf"),
            ("cache-control", "private, no-store"),
            ("x-content-type-options", "nosniff"),
        ],
        bytes,
    )
        .into_response())
}
