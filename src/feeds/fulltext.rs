//! Full-text news sources: article pages fetched from publishers that allow it (Radio Okapi via its
//! feed, 7sur7.cd via its section listings) and humanitarian reports from the ReliefWeb API.
//! Page access is robots.txt-checked per host, paced at max(configured delay, Crawl-delay), bounded
//! per poll, and stops for the poll when a host answers 429/503. Bot challenges are never bypassed:
//! a challenge page fails extraction and is recorded as a page error.
use super::{Feed, relevance};
use crate::extract::{Article, digest, selector};
use crate::store;
use anyhow::{Context, Result, bail, ensure};
use chrono::{DateTime, Utc};
use scraper::{ElementRef, Html};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::time::Instant;
use url::Url;

// ---------------------------------------------------------------------------------------------
// Polite page fetching

/// Next permitted request time per host, shared by every poll in the process.
static SLOTS: LazyLock<tokio::sync::Mutex<HashMap<String, Instant>>> =
    LazyLock::new(Default::default);
const AGENT: &str = "DrcOsintWorker";

pub(super) struct Pages {
    client: reqwest::Client,
    /// host → (fetched at, robots.txt body, Crawl-delay seconds)
    robots: HashMap<String, (Instant, String, f64)>,
}
/// Error text callers use to stop fetching from a host for the rest of the poll.
const THROTTLED: &str = "throttled by publisher";
pub(super) fn throttled(e: &anyhow::Error) -> bool {
    e.to_string().contains(THROTTLED)
}
fn crawl_delay(rules: &str) -> f64 {
    // Conservatively honour any Crawl-delay directive, even from other groups (as fetch.rs does).
    rules
        .lines()
        .filter_map(|l| l.split('#').next()?.split_once(':'))
        .filter(|(k, _)| k.trim().eq_ignore_ascii_case("crawl-delay"))
        .filter_map(|(_, v)| v.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite() && *n >= 0.)
        .fold(0., f64::max)
}
impl Pages {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent(crate::reference::user_agent())
                .timeout(Duration::from_secs(30))
                // Follow only same-host https redirects; the final URL is robots-checked again.
                .redirect(reqwest::redirect::Policy::custom(|a| {
                    let same = a.previous().first().and_then(|u| u.host_str())
                        == a.url().host_str()
                        && a.url().scheme() == "https";
                    if same && a.previous().len() < 4 {
                        a.follow()
                    } else {
                        a.stop()
                    }
                }))
                .build()?,
            robots: HashMap::new(),
        })
    }
    /// Wait for the host's slot, then reserve the next one `delay` later.
    pub(super) async fn turn(host: &str, delay: Duration) {
        let at = {
            let mut slots = SLOTS.lock().await;
            let at = slots
                .get(host)
                .copied()
                .unwrap_or_else(Instant::now)
                .max(Instant::now());
            slots.insert(host.to_string(), at + delay);
            at
        };
        tokio::time::sleep_until(at).await;
    }
    async fn cooldown(host: &str, seconds: u64) {
        SLOTS.lock().await.insert(
            host.to_string(),
            Instant::now() + Duration::from_secs(seconds),
        );
    }
    /// Robots policy for the URL's host (cached for an hour); returns the delay to use.
    pub async fn permit(&mut self, url: &Url, min_delay: u64) -> Result<Duration> {
        let host = url.host_str().context("URL without host")?.to_string();
        if self
            .robots
            .get(&host)
            .is_none_or(|(at, _, _)| at.elapsed() > Duration::from_secs(3600))
        {
            Self::turn(&host, Duration::from_secs(min_delay)).await;
            let r = self
                .client
                .get(format!("https://{host}/robots.txt"))
                .send()
                .await
                .context("robots.txt connection failed")?;
            let rules = match r.status().as_u16() {
                // No robots.txt means no restrictions.
                404 | 410 => String::new(),
                s if (200..300).contains(&s) => r.text().await?.chars().take(512_000).collect(),
                s => bail!("robots.txt HTTP {s}; not fetching pages"),
            };
            let delay = crawl_delay(&rules);
            ensure!(delay <= 300., "crawl delay exceeds supported limit");
            self.robots
                .insert(host.clone(), (Instant::now(), rules, delay));
        }
        let (_, rules, delay) = &self.robots[&host];
        ensure!(
            robotstxt::DefaultMatcher::default().one_agent_allowed_by_robots(
                rules,
                AGENT,
                url.as_str()
            ),
            "robots.txt disallows {}",
            url.path()
        );
        Ok(Duration::from_secs_f64(delay.max(min_delay as f64)))
    }
    /// Robots-checked, paced GET of an HTML page (5 MB cap).
    pub async fn html(&mut self, url: &Url, min_delay: u64) -> Result<String> {
        let delay = self.permit(url, min_delay).await?;
        let host = url.host_str().unwrap_or_default().to_string();
        Self::turn(&host, delay).await;
        let mut r = self.client.get(url.clone()).send().await?;
        let status = r.status().as_u16();
        if status == 429 || status == 503 {
            let wait = r
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(900)
                .clamp(60, 3600);
            Self::cooldown(&host, wait).await;
            bail!("{THROTTLED} (HTTP {status}); pausing {host} for {wait} s");
        }
        ensure!(r.status().is_success(), "page HTTP {status}");
        if r.url() != url {
            let (_, rules, _) = &self.robots[&host];
            ensure!(
                robotstxt::DefaultMatcher::default().one_agent_allowed_by_robots(
                    rules,
                    AGENT,
                    r.url().as_str()
                ),
                "redirect target disallowed by robots.txt"
            );
        }
        ensure!(
            r.headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.contains("text/html")),
            "expected an HTML page"
        );
        let mut bytes = Vec::new();
        while let Some(chunk) = r.chunk().await? {
            bytes.extend_from_slice(&chunk);
            ensure!(bytes.len() <= 5 << 20, "page larger than 5 MB");
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

// ---------------------------------------------------------------------------------------------
// Article pages

#[derive(Debug, Default)]
pub(super) struct Page {
    pub title: String,
    pub description: Option<String>,
    pub section: Option<String>,
    pub author: Option<(String, &'static str)>,
    pub published: Option<String>,
    pub modified: Option<String>,
    pub paragraphs: Vec<String>,
}
fn clean(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn page_date(s: &str) -> Option<String> {
    let s = s.trim();
    DateTime::parse_from_rfc3339(s)
        .or_else(|_| DateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%z"))
        .ok()
        .map(|d| d.with_timezone(&Utc).to_rfc3339())
}
/// Hidden blocks inside article bodies carry injected spam links on some Drupal sites.
fn hidden(el: ElementRef) -> bool {
    std::iter::once(*el)
        .chain(el.ancestors())
        .filter_map(|n| n.value().as_element())
        .any(|e| {
            e.attr("hidden").is_some()
                || e.attr("style").is_some_and(|s| {
                    s.replace(' ', "")
                        .to_ascii_lowercase()
                        .contains("display:none")
                })
        })
}
/// JSON-LD NewsArticle/Article objects, including inside @graph.
fn json_ld(doc: &Html) -> Vec<Value> {
    let mut out = Vec::new();
    for s in doc.select(&selector("script[type='application/ld+json']")) {
        let Ok(v) = serde_json::from_str::<Value>(&s.text().collect::<String>()) else {
            continue;
        };
        let mut stack = vec![v];
        while let Some(v) = stack.pop() {
            match v {
                Value::Array(a) => stack.extend(a),
                Value::Object(ref o) => {
                    if let Some(g) = o.get("@graph") {
                        stack.push(g.clone());
                    }
                    if o.contains_key("articleBody") || o.contains_key("datePublished") {
                        out.push(v);
                    }
                }
                _ => {}
            }
        }
    }
    out
}
const BODY_SELECTORS: &[&str] = &[
    "article .field--name-body",
    "article .field-name-body",
    ".field--name-body[property='schema:text']",
    ".field-name-body",
    "[itemprop='articleBody']",
];
/// Extract a Drupal-style article page: JSON-LD articleBody first, then body-field selectors.
pub(super) fn extract_page(html: &str) -> Result<Page> {
    let doc = Html::parse_document(html);
    let meta = |key: &str| -> Option<String> {
        doc.select(&selector("meta"))
            .find(|n| {
                n.value().attr("property") == Some(key) || n.value().attr("name") == Some(key)
            })
            .and_then(|n| n.value().attr("content"))
            .map(clean)
            .filter(|s| !s.is_empty())
    };
    let ld = json_ld(&doc);
    let ld_str = |key: &str| ld.iter().find_map(|v| v[key].as_str().map(clean));
    let mut paragraphs: Vec<String> = ld
        .iter()
        .find_map(|v| v["articleBody"].as_str().map(String::from))
        .map(|b| {
            // JSON-LD bodies are plain text, sometimes still carrying markup.
            let text = if b.contains('<') {
                Html::parse_fragment(&b).root_element().text().collect()
            } else {
                b
            };
            text.split('\n')
                .map(clean)
                .filter(|p| !p.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let mut body_el = None;
    if paragraphs.concat().len() < 200 {
        paragraphs.clear();
        for sel in BODY_SELECTORS {
            let Some(body) = doc.select(&selector(sel)).next() else {
                continue;
            };
            let found: Vec<String> = body
                .select(&selector("p, h2, h3, h4, li"))
                .filter(|p| !hidden(*p))
                .map(|p| clean(&p.text().collect::<String>()))
                .filter(|p| !p.is_empty())
                .collect();
            if found.concat().len() >= 200 {
                paragraphs = found;
                body_el = Some(body);
                break;
            }
        }
    }
    ensure!(
        paragraphs.concat().len() >= 200,
        "article body missing or too short; the page layout may have changed or the page is a challenge"
    );
    let title = meta("og:title")
        .or_else(|| ld_str("headline"))
        .or_else(|| {
            doc.select(&selector("h1"))
                .next()
                .map(|n| clean(&n.text().collect::<String>()))
        })
        .unwrap_or_default();
    ensure!(!title.is_empty(), "article title missing");
    let time_el = doc
        .select(&selector("article time[datetime], time[datetime]"))
        .next()
        .and_then(|t| t.value().attr("datetime"))
        .map(String::from);
    let published = meta("article:published_time")
        .or_else(|| ld_str("datePublished"))
        .or(time_el)
        .and_then(|s| page_date(&s));
    let modified = meta("article:modified_time")
        .or_else(|| meta("og:updated_time"))
        .or_else(|| ld_str("dateModified"))
        .and_then(|s| page_date(&s));
    let ld_author = ld.iter().find_map(|v| {
        let a = &v["author"];
        a.as_str()
            .or_else(|| a["name"].as_str())
            .or_else(|| a[0]["name"].as_str())
            .map(clean)
    });
    // Drupal newsrooms often sign with a final bold paragraph ("Joël Losinu, à Komanda").
    let byline = body_el.and_then(|b| {
        let p = b
            .select(&selector("p"))
            .filter(|p| !hidden(*p) && !clean(&p.text().collect::<String>()).is_empty())
            .last()?;
        let t = clean(&p.text().collect::<String>());
        let bold = clean(
            &p.select(&selector("strong, b"))
                .next()?
                .text()
                .collect::<String>(),
        );
        let name = t.split(',').next()?.trim().to_string();
        (t == bold
            && t.chars().count() < 70
            && (2..=5).contains(&name.split_whitespace().count())
            && !t.contains([':', '.', '!', '?']))
        .then_some(name)
    });
    let author = meta("author")
        .map(|a| (a, "metadata"))
        .or_else(|| ld_author.map(|a| (a, "json_ld")))
        .or_else(|| byline.map(|a| (a, "final_bold_paragraph_heuristic")));
    Ok(Page {
        title,
        description: meta("og:description").or_else(|| meta("description")),
        section: meta("article:section"),
        author,
        published,
        modified,
        paragraphs,
    })
}
const MAX_AGE_DAYS: i64 = 14;
fn url_date(url: &Url) -> Option<chrono::NaiveDate> {
    let parts: Vec<_> = url.path().trim_matches('/').splitn(4, '/').collect();
    chrono::NaiveDate::parse_from_str(&parts.get(..3)?.join("-"), "%Y-%m-%d").ok()
}
/// Dated article URL on the publisher's host: /YYYY/MM/DD/slug[/...], with Drupal's /index.php
/// alias, query and fragment removed so one article has one identity.
pub(super) fn article_url(site: &str, href: &str, base: &Url) -> Option<Url> {
    let mut url = base.join(href.trim()).ok()?;
    if url.scheme() != "https"
        || url.host_str() != Some(site)
        || url.port().is_some()
        || !url.username().is_empty()
    {
        return None;
    }
    if let Some(p) = url.path().strip_prefix("/index.php").map(String::from) {
        url.set_path(&p);
    }
    url.set_query(None);
    url.set_fragment(None);
    let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
    (parts.len() >= 4
        && parts[0].len() == 4
        && parts.last().is_some_and(|s| !s.is_empty())
        && chrono::NaiveDate::parse_from_str(&parts[..3].join("-"), "%Y-%m-%d").is_ok())
    .then_some(url)
}
pub(super) fn discover(html: &str, base: &Url, site: &str) -> Vec<Url> {
    let doc = Html::parse_document(html);
    let mut out: Vec<Url> = Vec::new();
    for a in doc.select(&selector("a[href]")) {
        if hidden(a) {
            continue;
        }
        if let Some(u) = a
            .value()
            .attr("href")
            .and_then(|h| article_url(site, h, base))
            && !out.contains(&u)
        {
            out.push(u);
        }
    }
    out
}
/// Full-text article from a fetched page; feed metadata (lede, author, time) fills page gaps.
pub(super) fn page_article(
    feed: &Feed,
    url: &Url,
    page: Page,
    html: &str,
    feed_item: Option<&super::Item>,
) -> Article {
    let content = page.paragraphs.join("\n\n");
    let description = page.description.or_else(|| {
        feed_item
            .map(|i| i.summary.clone())
            .filter(|s| !s.is_empty())
    });
    let (author, author_method) = match (page.author, feed_item.and_then(|i| i.author.clone())) {
        (Some((a, m)), _) => (Some(a), Some(m.to_string())),
        (None, Some(a)) => (Some(a), Some("feed_metadata".into())),
        _ => (None, None),
    };
    let relevance = relevance(&format!(
        "{}. {}. {}",
        page.title,
        description.clone().unwrap_or_default(),
        content
    ));
    Article {
        id: digest(url.as_str().as_bytes()),
        source: feed.name.clone(),
        url: url.to_string(),
        fetched_url: url.to_string(),
        title: page.title,
        description,
        category: page.section.or_else(|| super::section(url, &[])),
        author,
        author_method,
        language: feed.language.clone(),
        published_at: page
            .published
            .or_else(|| feed_item.and_then(|i| i.published.clone())),
        modified_at: page.modified,
        fetched_at: Utc::now().to_rfc3339(),
        content_sha256: digest(content.as_bytes()),
        html_sha256: digest(html.as_bytes()),
        content,
        paragraphs: page.paragraphs,
        extractor_version: "page-v1".into(),
        coverage: Some("full_text".into()),
        relevance: Some(relevance),
    }
}
pub(super) async fn coverage_of(db: &SqlitePool, id: &str) -> Result<Option<String>> {
    Ok(sqlx::query_scalar::<_, Option<String>>(
        "SELECT coalesce(json_extract(data,'$.coverage'),'') FROM articles WHERE id=?",
    )
    .bind(id)
    .fetch_optional(db)
    .await?
    .flatten())
}
/// Bounded per-URL failure counts carried in the sync detail so broken pages are not refetched
/// forever; after three failures (or an exclusion) a URL is skipped.
pub(super) fn failures(previous: &Value) -> HashMap<String, u64> {
    previous["page_failures"]
        .as_object()
        .map(|o| {
            o.iter()
                .map(|(k, v)| (k.clone(), v.as_u64().unwrap_or(0)))
                .collect()
        })
        .unwrap_or_default()
}
pub(super) fn failures_json(f: &HashMap<String, u64>) -> Value {
    // Keep the most-failed entries when trimming; order does not otherwise matter.
    let mut v: Vec<_> = f.iter().collect();
    v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    Value::Object(
        v.into_iter()
            .take(300)
            .map(|(k, n)| (k.clone(), json!(n)))
            .collect(),
    )
}
pub(super) const MAX_ATTEMPTS: u64 = 3;

/// Listing mode: discover dated articles on the homepage and section pages, fetch new ones.
pub(super) async fn poll_listing(
    db: &SqlitePool,
    pages: &mut Pages,
    feed: &Feed,
    previous: &Value,
) -> Result<Value> {
    let base = Url::parse(&feed.url)?;
    let mut candidates: Vec<Url> = Vec::new();
    let mut listing_errors = Vec::new();
    let listings = std::iter::once(base.clone())
        .chain(feed.listing_paths.iter().filter_map(|p| base.join(p).ok()))
        .collect::<Vec<_>>();
    for listing in &listings {
        match pages.html(listing, feed.page_delay_seconds).await {
            Ok(html) => {
                for u in discover(&html, listing, &feed.site) {
                    if !candidates.contains(&u) {
                        candidates.push(u);
                    }
                }
            }
            Err(e) if throttled(&e) => return Err(e),
            Err(e) => listing_errors.push(format!("{}: {e}", listing.path())),
        }
    }
    ensure!(
        !candidates.is_empty(),
        "no article links found ({})",
        listing_errors.join("; ")
    );
    let mut failed = failures(previous);
    let (mut drc, mut regional, mut skipped, mut known, mut errors, mut fetched) =
        (0, 0, 0, 0, 0, 0);
    let (mut last_error, mut stale) = (None, 0);
    let mut chars = 0usize;
    for url in &candidates {
        if coverage_of(db, &digest(url.as_str().as_bytes()))
            .await?
            .is_some()
        {
            known += 1;
            continue;
        }
        if failed.get(url.as_str()).is_some_and(|n| *n >= MAX_ATTEMPTS) || fetched >= feed.max_pages
        {
            continue;
        }
        // Sidebars link years-old tenders and evergreen pages; only recent news is collected.
        if url_date(url).is_none_or(|d| (Utc::now().date_naive() - d).num_days() > MAX_AGE_DAYS) {
            stale += 1;
            continue;
        }
        fetched += 1;
        let result = match pages.html(url, feed.page_delay_seconds).await {
            Ok(html) => extract_page(&html).map(|p| (p, html)),
            Err(e) => Err(e),
        };
        match result {
            Ok((page, html)) => {
                let a = page_article(feed, url, page, &html, None);
                match a
                    .relevance
                    .as_ref()
                    .map(|r| r["scope"].as_str().unwrap_or(""))
                {
                    Some("drc") => drc += 1,
                    Some("regional") => regional += 1,
                    _ => {
                        // Remember exclusions so the page is not fetched again.
                        skipped += 1;
                        failed.insert(url.to_string(), MAX_ATTEMPTS);
                        continue;
                    }
                }
                chars += a.content.chars().count();
                store::save(db, &a, &html).await?;
                failed.remove(url.as_str());
            }
            Err(e) => {
                let stop = throttled(&e);
                errors += 1;
                *failed.entry(url.to_string()).or_default() += 1;
                last_error = Some(format!("{}: {e}", url.path()));
                if stop {
                    break;
                }
            }
        }
    }
    let saved = drc + regional;
    Ok(
        json!({"items":candidates.len(),"drc":drc,"regional":regional,"skipped":skipped,"known":known,"stale":stale,
        "full_text":saved,"avg_chars":(saved > 0).then(|| chars / saved),"page_fetches":fetched,
        "page_errors":errors,"last_page_error":last_error,"listing_errors":listing_errors,
        "page_failures":failures_json(&failed),"not_modified":false}),
    )
}
// ---------------------------------------------------------------------------------------------
// ReliefWeb API

pub(super) const RELIEFWEB_NOT_CONFIGURED: &str =
    "not configured: request an appname at https://apidoc.reliefweb.int/parameters#appname";
pub(super) fn reliefweb_appname() -> Option<String> {
    std::env::var("RELIEFWEB_APPNAME")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
/// Recent reports whose primary country is the DRC, newest first.
pub(super) fn reliefweb_query(since: DateTime<Utc>) -> Value {
    json!({
        "limit": 50,
        "sort": ["date.created:desc"],
        "filter": {"operator": "AND", "conditions": [
            {"field": "primary_country.iso3", "value": "cod"},
            {"field": "date.created", "value": {"from": since.format("%Y-%m-%dT%H:%M:%S+00:00").to_string()}}
        ]},
        "fields": {"include": ["title", "body", "body-html", "url", "url_alias", "date.created",
            "date.original", "date.changed", "source.name", "source.shortname", "language.code",
            "format.name", "theme.name", "disaster.name", "origin", "headline.summary"]}
    })
}
fn names(v: &Value) -> Vec<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|x| x["name"].as_str().map(String::from))
        .collect()
}
/// Paragraphs from ReliefWeb's body-html (preferred) or Markdown body.
fn report_paragraphs(f: &Value) -> Vec<String> {
    if let Some(h) = f["body-html"].as_str() {
        let frag = Html::parse_fragment(h);
        let ps: Vec<String> = frag
            .select(&selector("p, h1, h2, h3, h4, li, td"))
            .map(|p| clean(&p.text().collect::<String>()))
            .filter(|p| !p.is_empty())
            .collect();
        if !ps.is_empty() {
            return ps;
        }
    }
    f["body"]
        .as_str()
        .unwrap_or("")
        .split("\n\n")
        .map(|p| clean(p.trim_start_matches(['#', '*', '-', '>', ' '])))
        .filter(|p| !p.is_empty())
        .collect()
}
/// One API report → Article; None when the report has no body text (attachment-only).
pub(super) fn report_article(feed: &Feed, item: &Value) -> Option<Article> {
    let f = &item["fields"];
    let title = clean(f["title"].as_str()?);
    let url = f["url_alias"].as_str().or(f["url"].as_str())?;
    let url = Url::parse(url).ok()?;
    if url.scheme() != "https" || url.host_str() != Some(feed.site.as_str()) {
        return None;
    }
    let paragraphs = report_paragraphs(f);
    let content = paragraphs.join("\n\n");
    if content.chars().count() < 80 {
        return None;
    }
    let sources = names(&f["source"]);
    let date = |k: &str| f["date"][k].as_str().and_then(page_date);
    Some(Article {
        id: digest(url.as_str().as_bytes()),
        source: format!(
            "{} · {}",
            feed.name,
            sources.first().map(String::as_str).unwrap_or("unknown")
        ),
        url: url.to_string(),
        fetched_url: feed.url.clone(),
        title,
        description: f["headline"]["summary"].as_str().map(clean),
        category: names(&f["format"]).into_iter().next(),
        author: None,
        author_method: None,
        language: f["language"][0]["code"]
            .as_str()
            .unwrap_or(&feed.language)
            .to_string(),
        published_at: date("original").or_else(|| date("created")),
        modified_at: date("changed"),
        fetched_at: Utc::now().to_rfc3339(),
        content_sha256: digest(content.as_bytes()),
        html_sha256: digest(item.to_string().as_bytes()),
        content,
        paragraphs,
        extractor_version: "reliefweb-api-v2".into(),
        coverage: Some("full_text".into()),
        relevance: Some(
            json!({"scope":"drc","matched_terms":[],"method":"reliefweb_primary_country",
            "note":"Primary country DRC as tagged by ReliefWeb editors",
            "reliefweb_id":item["id"],"sources":sources,"origin":f["origin"],
            "themes":names(&f["theme"]),"disasters":names(&f["disaster"])}),
        ),
    })
}
pub(super) async fn poll_reliefweb(
    db: &SqlitePool,
    client: &reqwest::Client,
    feed: &Feed,
) -> Result<Value> {
    let Some(appname) = reliefweb_appname() else {
        bail!(RELIEFWEB_NOT_CONFIGURED);
    };
    let mut url = Url::parse(&feed.url)?;
    url.query_pairs_mut().append_pair("appname", &appname);
    let since = Utc::now() - chrono::Duration::days(7);
    let mut r = client
        .post(url)
        .json(&reliefweb_query(since))
        .send()
        .await
        .context("ReliefWeb connection failed")?;
    match r.status().as_u16() {
        200 => {}
        403 => bail!("ReliefWeb HTTP 403: appname not approved; {RELIEFWEB_NOT_CONFIGURED}"),
        406 => bail!(
            "ReliefWeb HTTP 406: request blocked by its bot filter; ask hdx@un.org (mention the appname and user agent)"
        ),
        s => bail!("ReliefWeb HTTP {s}"),
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = r.chunk().await? {
        bytes.extend_from_slice(&chunk);
        ensure!(
            bytes.len() <= 20 << 20,
            "ReliefWeb response larger than 20 MB"
        );
    }
    let body: Value = serde_json::from_slice(&bytes).context("ReliefWeb response is not JSON")?;
    let items = body["data"].as_array().cloned().unwrap_or_default();
    let (mut saved, mut no_body, mut chars) = (0, 0, 0usize);
    for item in &items {
        match report_article(feed, item) {
            Some(a) => {
                chars += a.content.chars().count();
                store::save(db, &a, &item.to_string()).await?;
                saved += 1;
            }
            None => no_body += 1,
        }
    }
    Ok(
        json!({"items":items.len(),"total_matching":body["totalCount"],"drc":saved,"regional":0,
        "full_text":saved,"skipped":no_body,"avg_chars":(saved > 0).then(|| chars / saved),"not_modified":false}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn feed(id: &str) -> &'static Feed {
        super::super::feeds().iter().find(|f| f.id == id).unwrap()
    }
    // Trimmed from https://www.radiookapi.net/2026/10/18/actualite/securite/... (Drupal 7).
    const OKAPI: &str = r#"<html lang="fr"><head>
      <meta property="og:title" content="Plus de 24 600 personnes déplacées à Djugu (OCHA)" />
      <meta name="description" content="La reprise des violences armées dans le territoire de Djugu…" />
      <meta property="article:published_time" content="2026-10-18T12:21:45+01:00" />
      <meta property="article:modified_time" content="2026-10-18T12:21:45+01:00" /></head><body>
      <div class="sidebar"><p>Lire aussi SIDEBAR</p></div>
      <article id="node-289283" class="node node-article"><div class="field field-name-body"><div class="field-items"><div class="field-item even" property="content:encoded">
      <p><img src="x.jpg" title="Photo MONUSCO" /><strong>La reprise des violences armées dans le territoire de Djugu, en province de l’Ituri, a provoqué le déplacement de plus de 24 600 personnes.</strong></p>
      <p>Selon l’agence onusienne, la détérioration de la situation sécuritaire dans cette partie de l’Ituri intervient après plusieurs mois d’accalmie &amp; de calme relatif.</p>
      <p><strong>Des risques pour la riposte contre Ebola</strong></p>
      <p>OCHA alerte également sur les conséquences de cette insécurité.</p><p> </p></div></div></div>
      <div class="field field-name-field-article-similaire"><a href="https://www.radiookapi.net/2026/10/16/actualite/securite/x">Lire aussi</a></div></article></body></html>"#;
    // Trimmed from https://7sur7.cd/2026/10/18/ituri-des-chauffeurs-... (Drupal 9 layout builder).
    const SEPT: &str = r#"<html><head>
      <meta property="og:title" content="Ituri : des chauffeurs de poids lourds ont suspendu le trafic sur la RN27" />
      <meta property="article:section" content="Société" />
      <meta property="article:published_time" content="2026-10-18T19:46:02+0100" /></head><body>
      <section class="block"><div class="field field--name-body"><p>7sur7.cd est le plus grand site d'infos en RDC, plus de 223.300 visites par jour, plus de 258.100 fans sur Facebook et plus d'un million de followers.</p></div></section>
      <time datetime="2026-10-18T19:46:02+01:00">Dimanche 18 octobre 2026</time>
      <article typeof="schema:Article" class="article full"><div property="schema:text" class="field field--name-body">
      <h3>À Bunia, chef-lieu de la province de l'Ituri, les chauffeurs de poids lourds montent au créneau face à l'insécurité.</h3>
      <p>Dans un mémorandum adressé aux autorités provinciales et nationales, les chauffeurs ont annoncé la suspension de toutes leurs activités sur la RN27.</p>
      <div style="display: none;"><a href="https://spam.test/">slot dana</a><p>slot dana spam</p></div>
      <p><strong>Joël Losinu, à Komanda</strong></p></div></article></body></html>"#;

    #[test]
    fn okapi_body_is_extracted_without_related_links() {
        let p = extract_page(OKAPI).unwrap();
        assert_eq!(p.title, "Plus de 24 600 personnes déplacées à Djugu (OCHA)");
        assert_eq!(p.paragraphs.len(), 4);
        assert!(p.paragraphs[1].contains("accalmie & de calme"));
        assert!(!p.paragraphs.concat().contains("SIDEBAR"));
        assert!(!p.paragraphs.concat().contains("Lire aussi"));
        assert_eq!(p.published.as_deref(), Some("2026-10-18T11:21:45+00:00"));
        assert!(
            p.author.is_none(),
            "a mid-body bold subheading is not a byline"
        );
        let url = Url::parse("https://www.radiookapi.net/2026/10/18/actualite/securite/x").unwrap();
        let item = super::super::Item {
            author: Some("Radio Okapi".into()),
            summary: "Lede".into(),
            ..Default::default()
        };
        let a = page_article(feed("radiookapi"), &url, p, OKAPI, Some(&item));
        assert_eq!(a.coverage.as_deref(), Some("full_text"));
        assert_eq!(a.author.as_deref(), Some("Radio Okapi"));
        assert_eq!(a.author_method.as_deref(), Some("feed_metadata"));
        assert_eq!(a.category.as_deref(), Some("Sécurité"));
        assert_eq!(a.relevance.unwrap()["scope"], "drc");
    }
    #[test]
    fn sept_body_skips_sidebar_and_hidden_spam() {
        let p = extract_page(SEPT).unwrap();
        let text = p.paragraphs.join(" ");
        assert!(text.starts_with("À Bunia"));
        assert!(
            !text.contains("223.300"),
            "sidebar body field is not the article"
        );
        assert!(
            !text.contains("slot dana"),
            "hidden injected links are dropped"
        );
        assert_eq!(p.section.as_deref(), Some("Société"));
        assert_eq!(p.published.as_deref(), Some("2026-10-18T18:46:02+00:00"));
        assert_eq!(
            p.author,
            Some(("Joël Losinu".into(), "final_bold_paragraph_heuristic"))
        );
        assert!(
            extract_page("<html><title>Just a moment...</title><h1>Checking</h1></html>").is_err()
        );
    }
    #[test]
    fn json_ld_article_body_is_preferred() {
        let body = "Premier paragraphe sur Goma et le Nord-Kivu. ".repeat(5);
        let html = format!(
            r#"<script type="application/ld+json">{{"@graph":[{{"@type":"NewsArticle","headline":"Titre","datePublished":"2026-10-01T08:00:00Z","author":{{"name":"A. Auteur"}},"articleBody":"{body}\nSecond paragraphe."}}]}}</script><article><p>ignored</p></article>"#
        );
        let p = extract_page(&html).unwrap();
        assert_eq!(p.title, "Titre");
        assert_eq!(p.paragraphs.len(), 2);
        assert_eq!(p.author, Some(("A. Auteur".into(), "json_ld")));
        assert_eq!(p.published.as_deref(), Some("2026-10-01T08:00:00+00:00"));
    }
    #[test]
    fn listing_discovers_dated_articles_on_the_site_only() {
        let base = Url::parse("https://7sur7.cd/category/politique").unwrap();
        let html = r#"<a href="/2026/10/18/kinshasa-a">a</a><a href="/index.php/2026/10/18/kinshasa-a?x=1#c">dup</a>
          <a href="https://7sur7.cd/2026/10/17/rdc-b">b</a><a href="/category/securite">section</a>
          <a href="/2026/13/40/bad-date">bad</a><a href="https://evil.test/2026/10/18/x">evil</a>
          <a href="http://7sur7.cd/2026/10/18/plain">http</a><div style="display:none"><a href="/2026/10/18/hidden">h</a></div>"#;
        let found: Vec<String> = discover(html, &base, "7sur7.cd")
            .iter()
            .map(|u| u.to_string())
            .collect();
        assert_eq!(
            found,
            [
                "https://7sur7.cd/2026/10/18/kinshasa-a",
                "https://7sur7.cd/2026/10/17/rdc-b"
            ]
        );
        let okapi = Url::parse("https://www.radiookapi.net/").unwrap();
        assert!(
            article_url(
                "www.radiookapi.net",
                "/2026/10/18/actualite/securite/slug",
                &okapi
            )
            .is_some()
        );
        let dated = |p: &str| url_date(&Url::parse(&format!("https://7sur7.cd{p}")).unwrap());
        assert_eq!(
            dated("/2022/09/16/old-tender"),
            chrono::NaiveDate::from_ymd_opt(2022, 9, 16)
        );
        assert_eq!(dated("/category/une"), None);
    }
    #[test]
    fn crawl_delay_is_the_strictest_directive() {
        assert_eq!(
            crawl_delay("User-agent: *\nCrawl-delay: 10\nDisallow: /admin/"),
            10.
        );
        assert_eq!(
            crawl_delay("User-agent: x\nCrawl-delay: 3 # c\nUser-agent: *\ncrawl-delay:7"),
            7.
        );
        assert_eq!(crawl_delay("User-agent: *\nDisallow:"), 0.);
        let rules = "User-agent: *\nCrawl-delay: 10\nDisallow: /search/\nDisallow: /admin/\n";
        let m = |u: &str| {
            robotstxt::DefaultMatcher::default().one_agent_allowed_by_robots(rules, AGENT, u)
        };
        assert!(m(
            "https://www.radiookapi.net/2026/10/18/actualite/securite/x"
        ));
        assert!(!m("https://7sur7.cd/search/node?keys=rdc"));
    }
    #[tokio::test]
    async fn reliefweb_without_appname_makes_no_request() {
        if reliefweb_appname().is_some() {
            return;
        }
        let db = store::open("sqlite::memory:").await.unwrap();
        // An unroutable endpoint proves no request is attempted.
        let mut f = feed("reliefweb").clone();
        f.url = "https://127.0.0.1:9/never".into();
        let e = poll_reliefweb(&db, &reqwest::Client::new(), &f)
            .await
            .unwrap_err();
        assert_eq!(e.to_string(), RELIEFWEB_NOT_CONFIGURED);
    }
    #[test]
    fn reliefweb_reports_are_parsed_and_attributed() {
        let resp = json!({"totalCount":2,"count":2,"data":[
          {"id":"4100001","fields":{"title":"DR Congo: Ituri displacement flash update","url":"https://reliefweb.int/node/4100001",
            "url_alias":"https://reliefweb.int/report/democratic-republic-congo/ituri-flash-update",
            "body-html":"<h2>Highlights</h2><ul><li>24,600 people displaced in Djugu</li></ul><p>Clashes between armed groups in Fataki, Drodro and Lita &amp; nearby areas.</p>",
            "body":"## Highlights\n\n- 24,600 people displaced","date":{"created":"2026-10-15T10:00:00+00:00","original":"2026-10-15T00:00:00+00:00"},
            "source":[{"name":"UN Office for the Coordination of Humanitarian Affairs","shortname":"OCHA"}],
            "language":[{"code":"en"}],"format":[{"name":"Situation Report"}],"theme":[{"name":"Protection and Human Rights"}],
            "disaster":[{"name":"DR Congo: Complex Emergency"}]}},
          {"id":"4100002","fields":{"title":"Map only","url":"https://reliefweb.int/node/4100002","source":[{"name":"IOM"}]}}]});
        let f = feed("reliefweb");
        let items = resp["data"].as_array().unwrap();
        let a = report_article(f, &items[0]).unwrap();
        assert_eq!(
            a.source,
            "ReliefWeb · UN Office for the Coordination of Humanitarian Affairs"
        );
        assert_eq!(
            a.url,
            "https://reliefweb.int/report/democratic-republic-congo/ituri-flash-update"
        );
        assert_eq!(a.paragraphs[0], "Highlights");
        assert!(a.content.contains("Lita & nearby"));
        assert_eq!(a.category.as_deref(), Some("Situation Report"));
        assert_eq!(a.published_at.as_deref(), Some("2026-10-15T00:00:00+00:00"));
        assert_eq!(a.coverage.as_deref(), Some("full_text"));
        assert_eq!(
            a.relevance.as_ref().unwrap()["themes"][0],
            "Protection and Human Rights"
        );
        assert!(
            report_article(f, &items[1]).is_none(),
            "attachment-only reports are skipped"
        );
        let q = reliefweb_query(
            DateTime::parse_from_rfc3339("2026-10-11T00:00:00Z")
                .unwrap()
                .into(),
        );
        assert_eq!(q["filter"]["conditions"][0]["value"], "cod");
        assert_eq!(
            q["filter"]["conditions"][1]["value"]["from"],
            "2026-10-11T00:00:00+00:00"
        );
    }
}
