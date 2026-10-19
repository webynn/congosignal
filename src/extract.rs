use anyhow::{Result, bail, ensure};
use chrono::{DateTime, Utc};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

pub const ORIGIN: &str = "https://actualite.cd";
pub fn selector(s: &str) -> Selector {
    Selector::parse(s).expect("static CSS selector")
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn normalize(input: &str) -> Result<Url> {
    let mut url = Url::parse(input)?;
    ensure!(
        url.scheme() == "https"
            && url.host_str() == Some("actualite.cd")
            && url.port().is_none()
            && url.username().is_empty()
            && url.password().is_none(),
        "only https://actualite.cd URLs are allowed"
    );
    // Normalize Drupal aliases so the same article has one identity.
    if let Some(path) = url.path().strip_prefix("/index.php") {
        let path = if path.is_empty() { "/" } else { path }.to_owned();
        url.set_path(&path);
    }
    url.set_fragment(None);
    url.set_query(None);
    Ok(url)
}
pub fn is_article(url: &Url) -> bool {
    let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
    parts.len() == 4
        && parts[0].len() == 4
        && !parts[3].is_empty()
        && chrono::NaiveDate::parse_from_str(&parts[..3].join("-"), "%Y-%m-%d").is_ok()
}
pub fn seed(input: &str) -> Result<Url> {
    let original = Url::parse(input)?;
    let mut url = normalize(input)?;
    if !is_article(&url)
        && let Some((_, value)) = original.query_pairs().find(|(k, _)| k == "page")
    {
        let page: u32 = value
            .parse()
            .map_err(|_| anyhow::anyhow!("page must be an integer"))?;
        ensure!(page <= 10000, "archive page exceeds limit");
        url.set_query(Some(&format!("page={page}")));
    }
    ensure!(
        is_article(&url)
            || url.path() == "/"
            || url.path() == "/actualite"
            || url.path().starts_with("/category/"),
        "expected an article, homepage, /actualite or /category/... URL"
    );
    Ok(url)
}
pub fn discover(html: &str, base: &Url, limit: usize) -> Vec<String> {
    let doc = Html::parse_document(html);
    let mut links = Vec::new();
    for node in doc.select(&selector("a[href]")) {
        if let Some(url) = node
            .value()
            .attr("href")
            .and_then(|h| base.join(h).ok())
            .and_then(|u| normalize(u.as_str()).ok())
            .filter(is_article)
        {
            let s = url.to_string();
            if !links.contains(&s) {
                links.push(s);
            }
            if links.len() >= limit {
                break;
            }
        }
    }
    links
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub id: String,
    pub source: String,
    pub url: String,
    pub fetched_url: String,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub author: Option<String>,
    pub author_method: Option<String>,
    pub language: String,
    pub published_at: Option<String>,
    pub modified_at: Option<String>,
    pub fetched_at: String,
    pub content: String,
    pub paragraphs: Vec<String>,
    pub content_sha256: String,
    pub html_sha256: String,
    pub extractor_version: String,
    /// "feed_summary" when only a publisher feed's title and lede are available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coverage: Option<String>,
    /// DRC/regional screening for multi-country publishers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relevance: Option<serde_json::Value>,
}
pub fn extract(html: &str, fetched: &Url) -> Result<Article> {
    let doc = Html::parse_document(html);
    let meta = |key: &str| -> Option<String> {
        doc.select(&selector("meta"))
            .find(|n| {
                n.value().attr("property") == Some(key) || n.value().attr("name") == Some(key)
            })
            .and_then(|n| n.value().attr("content"))
            .map(str::to_owned)
    };
    let canonical = doc
        .select(&selector("link[rel='canonical']"))
        .next()
        .and_then(|n| n.value().attr("href"))
        .map(|h| fetched.join(h))
        .transpose()?
        .unwrap_or_else(|| fetched.clone());
    let canonical = normalize(canonical.as_str())?;
    ensure!(is_article(&canonical), "not a dated article");
    let title = meta("og:title")
        .or_else(|| {
            doc.select(&selector("h1"))
                .next()
                .map(|n| n.text().collect::<String>())
        })
        .unwrap_or_default()
        .trim()
        .to_owned();
    ensure!(!title.is_empty(), "article title missing");
    let body = doc
        .select(&selector("[id$='-corps'], .field--name-body"))
        .next();
    let Some(body) = body else {
        bail!("article body missing; site layout may have changed")
    };
    let paragraphs: Vec<String> = body
        .select(&selector("p, h2, h3, li"))
        .map(|p| {
            p.text()
                .collect::<String>()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })
        .filter(|p| !p.is_empty())
        .collect();
    let content = paragraphs.join("\n\n");
    ensure!(content.len() >= 80, "article body too short");
    let explicit_author = meta("author").filter(|a| !a.trim().is_empty());
    // Drupal stores many bylines as a final bold paragraph. Label the inference.
    let inferred = body.select(&selector("p")).last().and_then(|p| {
        let t = p.text().collect::<String>().trim().to_owned();
        let bold = p
            .select(&selector("strong"))
            .next()?
            .text()
            .collect::<String>();
        (t == bold.trim()
            && (2..=5).contains(&t.split_whitespace().count())
            && t.chars().count() < 70
            && !t.contains([':', '.', '!', '?']))
        .then_some(t)
    });
    let (author, author_method) = if let Some(a) = explicit_author {
        (Some(a), Some("metadata".into()))
    } else if let Some(a) = inferred {
        (Some(a), Some("final_bold_paragraph_heuristic".into()))
    } else {
        (None, None)
    };
    let date = |s: Option<String>| {
        s.and_then(|s| {
            DateTime::parse_from_rfc3339(&s)
                .or_else(|_| DateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S%z"))
                .ok()
        })
        .map(|d| d.with_timezone(&Utc).to_rfc3339())
    };
    Ok(Article {
        id: digest(canonical.as_str().as_bytes()),
        source: "Actualite.cd".into(),
        url: canonical.to_string(),
        fetched_url: fetched.to_string(),
        title,
        description: meta("description"),
        category: meta("article:section"),
        author,
        author_method,
        language: doc
            .select(&selector("html"))
            .next()
            .and_then(|n| n.value().attr("lang"))
            .unwrap_or("und")
            .into(),
        published_at: date(meta("article:published_time")),
        modified_at: date(meta("og:updated_time")),
        fetched_at: Utc::now().to_rfc3339(),
        content_sha256: digest(content.as_bytes()),
        html_sha256: digest(html.as_bytes()),
        content,
        paragraphs,
        extractor_version: "actualite-v1".into(),
        coverage: None,
        relevance: None,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn origin_guard_and_aliases() {
        for u in [
            "http://actualite.cd/",
            "https://actualite.cd.evil.test/",
            "https://user@actualite.cd/",
            "https://127.0.0.1/",
            "https://actualite.cd:444/",
        ] {
            assert!(normalize(u).is_err());
        }
        assert_eq!(
            normalize("https://actualite.cd/index.php/2026/10/12/test?utm=a#b")
                .unwrap()
                .as_str(),
            "https://actualite.cd/2026/10/12/test"
        );
        assert!(seed("https://actualite.cd/search/node").is_err());
    }
    #[test]
    fn extraction_keeps_provenance_and_excludes_navigation() {
        let html = include_str!("../tests/fixtures/article.html");
        let a = extract(
            html,
            &normalize("https://actualite.cd/2026/10/12/test").unwrap(),
        )
        .unwrap();
        assert_eq!(a.category.as_deref(), Some("Culture"));
        assert_eq!(a.author.as_deref(), Some("Japhet Toko"));
        assert_eq!(a.published_at.as_deref(), Some("2026-10-12T11:34:11+00:00"));
        assert!(a.content.contains("héritage & culture"));
        assert!(!a.content.contains("SIDEBAR"));
        assert!(extract("<h1>blocked</h1>", &normalize(&a.url).unwrap()).is_err());
    }
    #[test]
    fn discovery_deduplicates() {
        let html = r#"<a href='/2026/10/12/test'>a</a><a href='/index.php/2026/10/12/test?x=1'>b</a><a href='https://evil.test/2026/10/12/test'>c</a>"#;
        assert_eq!(discover(html, &normalize(ORIGIN).unwrap(), 10).len(), 1);
    }
}

#[cfg(test)]
mod archive_tests {
    use super::*;
    #[test]
    fn archive_page_preserved_only_for_listings() {
        assert_eq!(
            seed("https://actualite.cd/category/actualite/politique?page=5&tracking=x")
                .unwrap()
                .as_str(),
            "https://actualite.cd/category/actualite/politique?page=5"
        );
        assert!(seed("https://actualite.cd/category/actualite/politique?page=-1").is_err());
        assert!(seed("https://actualite.cd/category/actualite/politique?page=10001").is_err());
        assert!(
            seed("https://actualite.cd/2026/10/18/test?page=5")
                .unwrap()
                .query()
                .is_none()
        );
    }
}
