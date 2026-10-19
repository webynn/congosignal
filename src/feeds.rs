//! Publisher sources collected as news articles, configured in config/news-feeds.json.
//! "feed": a publisher feed whose pages block automated access; items keep the publisher's title,
//! lede, author, section and time and link to the original. "feed_full_text" / "listing" /
//! "reliefweb_api": full article text where the publisher allows it (see fulltext.rs). Everything
//! is screened for DRC relevance (Great Lakes neighbours are kept as flagged regional context).
//! Conditional GET (ETag / Last-Modified) keeps feed polling cheap; raw items are kept as evidence.
mod fulltext;
mod newsdata;
use crate::extract::{Article, digest};
use crate::text::{self, Doc, Term};
use crate::{ApiResult, App, reference, store};
use anyhow::{Context, Result};
use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use std::sync::LazyLock;
use tokio::sync::Notify;
use url::Url;

#[derive(Deserialize, Serialize, Clone, Copy, Default, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
enum Mode {
    #[default]
    Feed,
    FeedFullText,
    Listing,
    ReliefwebApi,
    NewsdataApi,
}
#[derive(Deserialize, Serialize, Clone)]
pub struct Feed {
    pub id: String,
    pub name: String,
    url: String,
    site: String,
    language: String,
    coverage: String,
    access_note: String,
    #[serde(default)]
    mode: Mode,
    #[serde(default)]
    listing_paths: Vec<String>,
    /// Minimum seconds between requests to the publisher (robots Crawl-delay may raise it).
    #[serde(default)]
    page_delay_seconds: u64,
    /// Article pages fetched per poll at most.
    #[serde(default)]
    max_pages: usize,
    /// Per-source poll interval overriding FEED_POLL_SECONDS (still off when that is 0).
    #[serde(default)]
    poll_seconds: Option<u64>,
}
impl Feed {
    /// Why the source cannot run, if it needs configuration.
    fn not_configured(&self) -> Option<&'static str> {
        if self.mode == Mode::NewsdataApi && newsdata::api_key().is_none() {
            return Some("not configured: set NEWSDATA_API_KEY");
        }
        (self.mode == Mode::ReliefwebApi && fulltext::reliefweb_appname().is_none())
            .then_some(fulltext::RELIEFWEB_NOT_CONFIGURED)
    }
    /// Poll interval in seconds; FEED_POLL_SECONDS=0 turns every source off.
    fn every(&self) -> u64 {
        match poll_seconds() {
            0 => 0,
            n => self.poll_seconds.unwrap_or(n).clamp(300, 86400),
        }
    }
}
#[derive(Deserialize)]
struct Config {
    feeds: Vec<Feed>,
}
static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../config/news-feeds.json")).expect("valid news feeds")
});
pub fn feeds() -> &'static [Feed] {
    &CONFIG.feeds
}
static POLL_NOW: LazyLock<Notify> = LazyLock::new(Notify::new);
/// FEED_POLL_SECONDS: 0 disables; otherwise 300–86400 (default 1800).
fn poll_seconds() -> u64 {
    match std::env::var("FEED_POLL_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
    {
        Some(0) => 0,
        Some(n) => n.clamp(300, 86400),
        None => 1800,
    }
}

// ---------------------------------------------------------------------------------------------
// Relevance

const DRC_TERMS: &[&str] = &[
    "RDC",
    "RD Congo",
    "R.D. Congo",
    "Congo-Kinshasa",
    "République démocratique du Congo",
    "Democratic Republic of the Congo",
    "DRC",
    "Kinshasa",
    "Goma",
    "Bukavu",
    "Kivu",
    "Nord-Kivu",
    "Sud-Kivu",
    "Ituri",
    "Uvira",
    "Lubumbashi",
    "Kisangani",
    "Kolwezi",
    "Katanga",
    "Kasaï",
    "M23",
    "AFC/M23",
    "Alliance Fleuve Congo",
    "FARDC",
    "Wazalendo",
    "Monusco",
    "MONUSCO",
    "Tshisekedi",
    "Kabila",
    "Katumbi",
    "Fayulu",
    "Nangaa",
    "Suminwa",
    "Gécamines",
    "Rubaya",
];
const REGIONAL_TERMS: &[&str] = &[
    "Rwanda",
    "rwandais",
    "rwandaise",
    "Kagame",
    "Kigali",
    "Ouganda",
    "ougandais",
    "Museveni",
    "Muhoozi",
    "Kampala",
    "Burundi",
    "burundais",
    "Ndayishimiye",
    "Bujumbura",
    "Grands Lacs",
    "Great Lakes",
];
/// Labelled terms: (DRC, regional).
type TermSet = Vec<(String, Term)>;
static TERMS: LazyLock<(TermSet, TermSet)> = LazyLock::new(|| {
    let build = |list: &[&str]| {
        list.iter()
            .map(|t| (t.to_string(), text::term(t)))
            .collect()
    };
    (build(DRC_TERMS), build(REGIONAL_TERMS))
});
/// "drc" on explicit DRC terms or a recognized DRC place; "regional" for Great Lakes neighbours.
fn relevance(text: &str) -> Value {
    let doc = Doc::new(text);
    let hits = |terms: &[(String, Term)]| -> Vec<String> {
        terms
            .iter()
            .filter(|(_, t)| doc.has_term(t))
            .map(|(label, _)| label.clone())
            .collect()
    };
    let mut drc = hits(&TERMS.0);
    for p in crate::intelligence::place_names(text) {
        if !drc.contains(&p) {
            drc.push(p);
        }
    }
    let regional = hits(&TERMS.1);
    let scope = if !drc.is_empty() {
        "drc"
    } else if !regional.is_empty() {
        "regional"
    } else {
        "excluded"
    };
    let matched = [drc, regional].concat();
    json!({"scope":scope,"matched_terms":matched,"method":"feed_terms_v1","note":"Relevance for review, not verification"})
}

// ---------------------------------------------------------------------------------------------
// Parsing

#[derive(Debug, Default, Clone)]
struct Item {
    title: String,
    link: String,
    published: Option<String>,
    author: Option<String>,
    summary: String,
    categories: Vec<String>,
    raw: String,
}
fn strip_tags(s: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for c in s.chars() {
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
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&#8217;", "’")
        .replace("&rsquo;", "’")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn date(s: &str) -> Option<String> {
    let s = s.trim();
    DateTime::parse_from_rfc3339(s)
        .or_else(|_| DateTime::parse_from_rfc2822(s))
        .ok()
        .map(|d| d.with_timezone(&Utc).to_rfc3339())
}
fn parse_feed(xml: &str) -> Result<Vec<Item>> {
    let doc = roxmltree::Document::parse(xml).context("feed is not valid XML")?;
    let mut out = Vec::new();
    for n in doc
        .descendants()
        .filter(|n| matches!(n.tag_name().name(), "item" | "entry"))
    {
        let child = |name: &str| {
            n.children()
                .find(|c| c.tag_name().name() == name)
                .map(|c| c.text().unwrap_or("").trim().to_string())
                .filter(|s| !s.is_empty())
        };
        let link = child("link").or_else(|| {
            n.children()
                .filter(|c| c.tag_name().name() == "link")
                .find(|c| c.attribute("rel").is_none_or(|r| r == "alternate"))
                .and_then(|c| c.attribute("href"))
                .map(String::from)
        });
        let (Some(title), Some(link)) = (child("title"), link) else {
            continue;
        };
        out.push(Item {
            title: strip_tags(&title),
            link,
            published: ["pubDate", "published", "date", "updated"]
                .iter()
                .find_map(|k| child(k).and_then(|d| date(&d))),
            author: child("creator").or_else(|| {
                n.children()
                    .find(|c| c.tag_name().name() == "author")
                    .and_then(|a| a.children().find(|c| c.tag_name().name() == "name"))
                    .and_then(|c| c.text())
                    .map(|s| s.trim().to_string())
            }),
            summary: strip_tags(
                &child("description")
                    .or_else(|| child("summary"))
                    .unwrap_or_default(),
            ),
            categories: n
                .children()
                .filter(|c| c.tag_name().name() == "category")
                .filter_map(|c| c.text().or(c.attribute("term")))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            raw: xml[n.range()].to_string(),
        });
    }
    Ok(out)
}
/// Publisher link on the feed's own site, normalized for a stable identity.
fn canonical(feed: &Feed, link: &str) -> Result<Url> {
    let mut url = Url::parse(link.trim())?;
    anyhow::ensure!(
        url.scheme() == "https" && url.host_str() == Some(feed.site.as_str()),
        "item link outside {}",
        feed.site
    );
    url.set_query(None);
    url.set_fragment(None);
    Ok(url)
}
/// Section from the publisher URL path (e.g. /1851360/politique/slug/), mapped to the feed's
/// category vocabulary where it matches.
fn section(url: &Url, categories: &[String]) -> Option<String> {
    // Radio Okapi paths read /YYYY/MM/DD/actualite/<section>/slug.
    let seg = url
        .path_segments()?
        .find(|s| !s.is_empty() && !s.chars().all(|c| c.is_ascii_digit()) && *s != "actualite")?;
    let mapped = match seg {
        "politique" => "Politique",
        s if s.starts_with("economie") => "Économie",
        "societe" => "Société",
        "culture" | "culture-medias" => "Culture",
        "sport" | "sports" => "Sport",
        s if s.starts_with("opinion") || s == "tribune" => "Opinion",
        "sante" => "Santé",
        "securite" => "Sécurité",
        "justice" => "Justice",
        "revue-de-presse" => "Revue de presse",
        _ => return categories.first().cloned(),
    };
    Some(mapped.into())
}
fn to_article(feed: &Feed, item: &Item) -> Result<Option<Article>> {
    let url = canonical(feed, &item.link)?;
    let text = format!("{}. {}", item.title, item.summary);
    let relevance = relevance(&text);
    if relevance["scope"] == "excluded" {
        return Ok(None);
    }
    let paragraphs: Vec<String> = [item.summary.clone()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    let content = if paragraphs.is_empty() {
        item.title.clone()
    } else {
        paragraphs.join("\n\n")
    };
    Ok(Some(Article {
        id: digest(url.as_str().as_bytes()),
        source: feed.name.clone(),
        url: url.to_string(),
        fetched_url: feed.url.clone(),
        title: item.title.clone(),
        description: (!item.summary.is_empty()).then(|| item.summary.clone()),
        category: section(&url, &item.categories),
        author: item.author.clone(),
        author_method: item.author.as_ref().map(|_| "feed_metadata".into()),
        language: feed.language.clone(),
        published_at: item.published.clone(),
        modified_at: None,
        fetched_at: Utc::now().to_rfc3339(),
        content_sha256: digest(content.as_bytes()),
        html_sha256: digest(item.raw.as_bytes()),
        content,
        paragraphs,
        extractor_version: "feed-v1".into(),
        coverage: Some(feed.coverage.clone()),
        relevance: Some(relevance),
    }))
}

// ---------------------------------------------------------------------------------------------
// Polling

async fn previous_detail(db: &SqlitePool, feed: &Feed) -> Result<Value> {
    Ok(
        sqlx::query_scalar::<_, Option<String>>("SELECT detail FROM reference_sync WHERE id=?")
            .bind(format!("news-feed:{}", feed.id))
            .fetch_optional(db)
            .await?
            .flatten()
            .and_then(|d| serde_json::from_str(&d).ok())
            .unwrap_or_default(),
    )
}
async fn poll(
    db: &SqlitePool,
    client: &reqwest::Client,
    pages: &mut fulltext::Pages,
    feed: &Feed,
) -> Result<Value> {
    let previous = previous_detail(db, feed).await?;
    let full_text = feed.mode == Mode::FeedFullText;
    if full_text {
        // The feed lives on the publisher's host, so it shares the page pacing and robots policy.
        let delay = pages
            .permit(&Url::parse(&feed.url)?, feed.page_delay_seconds)
            .await?;
        fulltext::Pages::turn(&feed.site, delay).await;
    }
    let mut req = client.get(&feed.url);
    if let Some(etag) = previous["etag"].as_str() {
        req = req.header("If-None-Match", etag);
    }
    if let Some(lm) = previous["last_modified"].as_str() {
        req = req.header("If-Modified-Since", lm);
    }
    let mut r = req.send().await.context("feed connection failed")?;
    if r.status() == reqwest::StatusCode::NOT_MODIFIED {
        let mut detail = previous.clone();
        detail["not_modified"] = json!(true);
        return Ok(detail);
    }
    anyhow::ensure!(r.status().is_success(), "feed HTTP {}", r.status().as_u16());
    let header = |name: &str| {
        r.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(String::from)
    };
    let (etag, last_modified) = (header("etag"), header("last-modified"));
    let mut bytes = Vec::new();
    while let Some(chunk) = r.chunk().await? {
        bytes.extend_from_slice(&chunk);
        anyhow::ensure!(bytes.len() <= 5 << 20, "feed larger than 5 MB");
    }
    let items = parse_feed(&String::from_utf8_lossy(&bytes))?;
    anyhow::ensure!(!items.is_empty(), "feed returned no items");
    let (mut drc, mut regional, mut skipped, mut invalid) = (0, 0, 0, 0);
    let (mut fetched, mut full, mut pending, mut chars) = (0, 0, 0, 0usize);
    let (mut page_errors, mut stop) = (Vec::new(), false);
    let mut failed = fulltext::failures(&previous);
    for item in &items {
        match to_article(feed, item) {
            Ok(Some(mut a)) => {
                if a.relevance.as_ref().is_some_and(|r| r["scope"] == "drc") {
                    drc += 1;
                } else {
                    regional += 1;
                }
                let mut raw = item.raw.clone();
                if full_text {
                    // Never overwrite stored full text with the feed summary.
                    if fulltext::coverage_of(db, &a.id).await?.as_deref() == Some("full_text") {
                        full += 1;
                        continue;
                    }
                    a.coverage = Some("feed_summary".into());
                    let attempts = failed.get(&a.url).copied().unwrap_or(0);
                    if !stop && fetched < feed.max_pages && attempts < fulltext::MAX_ATTEMPTS {
                        fetched += 1;
                        let url = Url::parse(&a.url)?;
                        let page = match pages.html(&url, feed.page_delay_seconds).await {
                            Ok(html) => fulltext::extract_page(&html).map(|p| (p, html)),
                            Err(e) => Err(e),
                        };
                        match page {
                            Ok((p, html)) => {
                                let mut f =
                                    fulltext::page_article(feed, &url, p, &html, Some(item));
                                // Keep the feed's screening: the item was selected on it.
                                f.relevance = a.relevance.take();
                                raw = format!("{}\n{html}", item.raw);
                                a = f;
                                full += 1;
                                chars += a.content.chars().count();
                                failed.remove(&a.url);
                            }
                            Err(e) => {
                                stop = fulltext::throttled(&e);
                                *failed.entry(a.url.clone()).or_default() += 1;
                                page_errors.push(format!("{}: {e}", url.path()));
                            }
                        }
                    }
                    if a.coverage.as_deref() == Some("feed_summary") {
                        pending += 1;
                    }
                }
                store::save(db, &a, &raw).await?;
            }
            Ok(None) => skipped += 1,
            Err(_) => invalid += 1,
        }
    }
    let mut detail = json!({"items":items.len(),"drc":drc,"regional":regional,"skipped":skipped,"invalid":invalid,
              "etag":etag,"last_modified":last_modified,"not_modified":false});
    if full_text {
        let fresh = fetched - page_errors.len();
        detail["full_text"] = json!(full);
        detail["summary_only"] = json!(pending);
        detail["page_fetches"] = json!(fetched);
        detail["page_errors"] = json!(page_errors.len());
        detail["last_page_error"] = json!(page_errors.last());
        detail["avg_chars"] = json!((fresh > 0).then(|| chars / fresh));
        detail["page_failures"] = fulltext::failures_json(&failed);
    }
    Ok(detail)
}
/// Poll one source and record the outcome for GET /v1/feeds.
async fn run(
    db: &SqlitePool,
    client: &reqwest::Client,
    pages: &mut fulltext::Pages,
    feed: &Feed,
) -> Result<()> {
    let previous = previous_detail(db, feed).await?;
    let result = match feed.mode {
        Mode::Feed | Mode::FeedFullText => poll(db, client, pages, feed).await,
        Mode::Listing => fulltext::poll_listing(db, pages, feed, &previous).await,
        Mode::ReliefwebApi => fulltext::poll_reliefweb(db, client, feed).await,
        Mode::NewsdataApi => newsdata::poll(db, feed).await,
    };
    // A failed poll keeps the per-URL failure counts so broken pages stay bounded.
    let detail = result.as_ref().cloned().unwrap_or_else(
        |_| json!({"page_failures": previous.get("page_failures").cloned().unwrap_or_default()}),
    );
    let count = result
        .as_ref()
        .map(|d| (d["drc"].as_u64().unwrap_or(0) + d["regional"].as_u64().unwrap_or(0)) as usize)
        .map_err(|e| anyhow::anyhow!(e.to_string()));
    if let Err(e) = &count {
        tracing::warn!(feed = %feed.id, error = %e, "news feed poll failed");
    }
    reference::record(db, &format!("news-feed:{}", feed.id), &count, detail).await
}
pub async fn worker(db: SqlitePool) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent(reference::user_agent())
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()?;
    let mut pages = fulltext::Pages::new()?;
    loop {
        let every = poll_seconds();
        if every > 0 {
            for feed in feeds() {
                let id = format!("news-feed:{}", feed.id);
                if feed.not_configured().is_some() {
                    continue;
                }
                let every = feed.every();
                if reference::due(&db, &id, every as i64).await? {
                    run(&db, &client, &mut pages, feed).await?;
                }
            }
        }
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
            _ = POLL_NOW.notified() => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// API

pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let sync = reference::sync_rows(&app.db, "news-feed:").await?;
    let mut out = Vec::new();
    for f in feeds() {
        // ReliefWeb articles are credited "ReliefWeb · <organisation>".
        let (collected, full_text, latest): (i64, i64, Option<String>) = sqlx::query_as(
            "SELECT count(*), coalesce(sum(json_extract(data,'$.coverage')='full_text'),0), max(published_at)
             FROM articles WHERE json_extract(data,'$.source')=?1 OR json_extract(data,'$.source') LIKE ?1 || ' · %'",
        )
        .bind(&f.name)
        .fetch_one(&app.db)
        .await?;
        let mut sync = sync
            .iter()
            .find(|s| s["id"] == format!("news-feed:{}", f.id))
            .cloned();
        // Per-URL failure bookkeeping is internal.
        if let Some(d) = sync.as_mut().and_then(|s| s["detail"].as_object_mut()) {
            d.remove("page_failures");
        }
        out.push(json!({"id":f.id,"name":f.name,"url":f.url,"site":f.site,"coverage":f.coverage,"access_note":f.access_note,
            "mode":f.mode,"configured":f.not_configured().is_none(),"status_note":f.not_configured(),
            "poll_seconds":f.every(),
            "collected":collected,"full_text":full_text,"latest_published":latest,"sync":sync}));
    }
    Ok(Json(json!({"items":out,"poll_seconds":poll_seconds()})))
}
pub async fn refresh(State(app): State<App>) -> ApiResult<Json<Value>> {
    sqlx::query("UPDATE reference_sync SET checked_at=NULL WHERE id LIKE 'news-feed:%'")
        .execute(&app.db)
        .await?;
    POLL_NOW.notify_waiters();
    Ok(Json(json!({"queued":true,"enabled":poll_seconds()>0})))
}
/// A submitted URL from a feed-only publisher: report whether it is already collected.
pub async fn submitted(db: &SqlitePool, url: &Url) -> Result<Option<Value>> {
    let Some(feed) = feeds()
        .iter()
        .find(|f| url.host_str() == Some(f.site.as_str()))
    else {
        return Ok(None);
    };
    let mut canonical = url.clone();
    canonical.set_query(None);
    canonical.set_fragment(None);
    let id: Option<String> = sqlx::query_scalar("SELECT id FROM articles WHERE url=?")
        .bind(canonical.as_str())
        .fetch_optional(db)
        .await?;
    Ok(Some(match id {
        Some(id) => json!({"status":"already_collected","article_id":id,"source":feed.name}),
        None if feed.mode == Mode::Feed => {
            json!({"status":"feed_only","source":feed.name,"message":format!("{} is collected from its official feed every {} minutes; individual pages cannot be fetched automatically. {} New DRC articles appear automatically; older articles no longer in the feed are not available.", feed.name, poll_seconds() / 60, feed.access_note)})
        }
        None => {
            json!({"status":"collected_automatically","source":feed.name,"message":format!("{} is collected automatically every {} minutes with full text. {} Individual URLs are not queued; new DRC articles appear automatically.", feed.name, feed.every() / 60, feed.access_note)})
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    const RSS: &str = r#"<?xml version="1.0"?><rss xmlns:dc="http://purl.org/dc/elements/1.1/"><channel><title>JA</title>
      <item><title> RDC : Joseph Kabila prend la tête de la rébellion </title><link>https://www.jeuneafrique.com/1851360/politique/rdc-kabila/?utm=x</link><pubDate>2026-10-16T09:00:00+00:00</pubDate><dc:creator>Romain Gras</dc:creator><description><![CDATA[<p>L’AFC/M23 a été dissoute &amp; remplacée.</p>]]></description></item>
      <item><title>En Ouganda, Muhoozi étend son influence</title><link>https://www.jeuneafrique.com/1852330/politique/ouganda/</link><pubDate>Sun, 18 Oct 2026 12:15:00 +0000</pubDate><description>Depuis la réélection de Yoweri Museveni…</description></item>
      <item><title>Cameroun : ces femmes qui font l’économie</title><link>https://www.jeuneafrique.com/1850516/economie-entreprises/cameroun/</link><description>Banque, BTP, cacao.</description></item>
      <item><title>Congo-Brazzaville : Sassou en tournée</title><link>https://www.jeuneafrique.com/1/politique/brazza/</link><description>Le président congolais est à Pointe-Noire.</description></item>
      <item><title>Injected RDC link</title><link>https://evil.test/rdc</link></item>
    </channel></rss>"#;
    #[test]
    fn feed_items_are_screened_and_attributed() {
        let feed = feeds().iter().find(|f| f.id == "jeuneafrique").unwrap();
        let items = parse_feed(RSS).unwrap();
        assert_eq!(items.len(), 5);
        let a = to_article(feed, &items[0]).unwrap().unwrap();
        assert_eq!(a.source, "Jeune Afrique");
        assert_eq!(
            a.url,
            "https://www.jeuneafrique.com/1851360/politique/rdc-kabila/"
        );
        assert_eq!(a.title, "RDC : Joseph Kabila prend la tête de la rébellion");
        assert_eq!(a.content, "L’AFC/M23 a été dissoute & remplacée.");
        assert_eq!(a.category.as_deref(), Some("Politique"));
        assert_eq!(a.author.as_deref(), Some("Romain Gras"));
        assert_eq!(a.published_at.as_deref(), Some("2026-10-16T09:00:00+00:00"));
        assert_eq!(a.coverage.as_deref(), Some("feed_summary"));
        assert_eq!(a.relevance.as_ref().unwrap()["scope"], "drc");
        let uganda = to_article(feed, &items[1]).unwrap().unwrap();
        assert_eq!(uganda.relevance.unwrap()["scope"], "regional");
        assert_eq!(
            uganda.published_at.as_deref(),
            Some("2026-10-18T12:15:00+00:00")
        );
        assert!(
            to_article(feed, &items[2]).unwrap().is_none(),
            "unrelated country excluded"
        );
        assert!(
            to_article(feed, &items[3]).unwrap().is_none(),
            "Congo-Brazzaville is not the DRC"
        );
        assert!(
            to_article(feed, &items[4]).is_err(),
            "links outside the publisher are rejected"
        );
    }
    /// Manual live check against the real publishers (never run by default; honours all delays):
    /// LIVE_DB=sqlite://agent.db cargo test live_collect -- --ignored --nocapture
    #[tokio::test]
    #[ignore]
    async fn live_collect() {
        let db = store::open(&std::env::var("LIVE_DB").unwrap_or("sqlite::memory:".into()))
            .await
            .unwrap();
        reference::init(&db).await.unwrap();
        let client = reqwest::Client::builder()
            .user_agent(reference::user_agent())
            .build()
            .unwrap();
        let mut pages = fulltext::Pages::new().unwrap();
        for feed in feeds().iter().filter(|f| f.mode != Mode::Feed) {
            if let Some(note) = feed.not_configured() {
                println!("{}: {note}", feed.id);
                continue;
            }
            run(&db, &client, &mut pages, feed).await.unwrap();
            let row: (Option<String>, Option<String>, Option<String>) =
                sqlx::query_as("SELECT status, error, detail FROM reference_sync WHERE id=?")
                    .bind(format!("news-feed:{}", feed.id))
                    .fetch_one(&db)
                    .await
                    .unwrap();
            println!("{}: {row:?}", feed.id);
        }
    }
}
