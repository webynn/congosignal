//! NewsData latest API. Credentials stay in a sensitive header, never URLs or raw evidence.
use super::*;
use anyhow::{bail, ensure};
use chrono::NaiveDateTime;

pub(super) fn api_key() -> Option<String> {
    std::env::var("NEWSDATA_API_KEY")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}
fn article(feed: &Feed, item: &Value) -> Option<Article> {
    let title = strip_tags(item["title"].as_str()?);
    if title.is_empty() {
        return None;
    }
    let mut url = Url::parse(item["link"].as_str()?).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    url.set_fragment(None);
    let language = match item["language"].as_str()? {
        "english" | "en" => "en",
        "french" | "fr" => "fr",
        _ => return None,
    };
    let summary = item["description"]
        .as_str()
        .map(strip_tags)
        .unwrap_or_default();
    let mut rel = relevance(&format!("{title}. {summary}"));
    if rel["scope"] == "excluded" {
        return None;
    }
    rel["provider"] = json!("newsdata");
    rel["provider_article_id"] = item["article_id"].clone();
    rel["publisher"] = item["source_name"].clone();
    let content = if summary.is_empty() {
        title.clone()
    } else {
        summary.clone()
    };
    let author = item["creator"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|s| !s.is_empty());
    let published = item["pubDate"].as_str().and_then(|d| {
        date(d).or_else(|| {
            // Never assign UTC to an explicitly non-UTC naive timestamp.
            if item["pubDateTZ"].as_str().is_some_and(|z| z != "UTC") {
                return None;
            }
            NaiveDateTime::parse_from_str(d, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|d| d.and_utc().to_rfc3339())
        })
    });
    Some(Article {
        id: digest(url.as_str().as_bytes()),
        source: format!(
            "{} · {}",
            feed.name,
            item["source_name"]
                .as_str()
                .or(item["source_id"].as_str())
                .unwrap_or("Unknown publisher")
        ),
        url: url.to_string(),
        fetched_url: feed.url.clone(),
        title,
        description: (!summary.is_empty()).then_some(summary),
        category: item["category"][0].as_str().map(String::from),
        author_method: author.as_ref().map(|_| "api_metadata".into()),
        author,
        language: language.into(),
        published_at: published,
        modified_at: None,
        fetched_at: Utc::now().to_rfc3339(),
        content_sha256: digest(content.as_bytes()),
        html_sha256: digest(item.to_string().as_bytes()),
        paragraphs: vec![content.clone()],
        content,
        extractor_version: "newsdata-v1".into(),
        coverage: Some("api_summary".into()),
        relevance: Some(rel),
    })
}

pub(super) async fn poll(db: &SqlitePool, feed: &Feed) -> Result<Value> {
    let key = api_key().context("not configured: set NEWSDATA_API_KEY")?;
    let mut header = reqwest::header::HeaderValue::from_str(&key)
        .map_err(|_| anyhow::anyhow!("Invalid NewsData key format"))?;
    header.set_sensitive(true);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(reference::user_agent())
        .build()?;
    let (mut items, mut saved, mut regional, mut duplicates, mut skipped, mut pages) =
        (0, 0, 0, 0, 0, 0);
    let mut next: Option<String> = None;
    let mut partial_error: Option<String> = None;
    let mut total = Value::Null;
    for _ in 0..feed.max_pages.clamp(1, 20) {
        let mut request = client
            .get("https://newsdata.io/api/1/latest")
            .header("X-ACCESS-KEY", header.clone())
            .query(&[("country", "cd"), ("language", "fr,en")]);
        if let Some(cursor) = &next {
            request = request.query(&[("page", cursor)]);
        }
        let fetched: Result<Value> = async {
            let mut r = request
                .send()
                .await
                .map_err(|_| anyhow::anyhow!("NewsData connection failed"))?;
            ensure!(
                r.status().is_success(),
                "NewsData HTTP {} (check key, quota or plan; retry on next poll)",
                r.status().as_u16()
            );
            let mut bytes = Vec::new();
            while let Some(chunk) = r
                .chunk()
                .await
                .map_err(|_| anyhow::anyhow!("NewsData response interrupted"))?
            {
                bytes.extend_from_slice(&chunk);
                ensure!(bytes.len() <= 5 << 20, "NewsData response exceeds 5 MB");
            }
            let body: Value = serde_json::from_slice(&bytes).context("NewsData invalid JSON")?;
            ensure!(
                body["status"] == "success",
                "NewsData API rejected request (check quota or plan)"
            );
            ensure!(body["results"].is_array(), "NewsData results missing");
            Ok(body)
        }
        .await;
        let body = match fetched {
            Ok(b) => b,
            Err(e) if pages > 0 => {
                partial_error = Some(e.to_string());
                break;
            }
            Err(e) => return Err(e),
        };
        pages += 1;
        total = body["totalResults"].clone();
        for item in body["results"].as_array().unwrap() {
            items += 1;
            let Some(a) = article(feed, item) else {
                skipped += 1;
                continue;
            };
            // Preserve independently collected full text and publisher attribution; URL is the dedup key.
            let exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM articles WHERE id=? OR url=?)")
                    .bind(&a.id)
                    .bind(&a.url)
                    .fetch_one(db)
                    .await?;
            if exists {
                duplicates += 1;
                continue;
            }
            store::save(db, &a, &item.to_string()).await?;
            if a.relevance
                .as_ref()
                .is_some_and(|r| r["scope"] == "regional")
            {
                regional += 1;
            } else {
                saved += 1;
            }
        }
        let cursor = body["nextPage"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(String::from);
        if cursor.is_none() || cursor == next {
            next = None;
            break;
        }
        next = cursor;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    if pages == 0 {
        bail!("NewsData returned no pages");
    }
    Ok(
        json!({"items":items,"drc":saved,"regional":regional,"duplicates":duplicates,"skipped":skipped,
        "pages":pages,"total_matching":total,"more_available":next.is_some(),"partial_error":partial_error,
        "summary_only":saved+regional,"full_text":0,"not_modified":false}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_summary_attribution_and_filters() {
        let feed = feeds().iter().find(|f| f.id == "newsdata").unwrap();
        let mut item = json!({"title":"Goma: nouvelles discussions de paix","description":"Les autorités de la RDC discutent.","link":"https://example.org/news#fragment","language":"french","source_name":"Publisher","creator":["Reporter"],"pubDate":"2026-10-19 10:00:00","pubDateTZ":"UTC","content":"ONLY AVAILABLE IN PAID PLANS"});
        let a = article(feed, &item).unwrap();
        assert_eq!(a.source, "NewsData.io · Publisher");
        assert_eq!(a.coverage.as_deref(), Some("api_summary"));
        assert_eq!(a.language, "fr");
        assert_eq!(a.published_at.as_deref(), Some("2026-10-19T10:00:00+00:00"));
        assert!(!a.content.contains("PAID PLANS"));
        assert_eq!(a.url, "https://example.org/news");
        item["title"] = json!("Hollywood movie premiere");
        item["description"] = Value::Null;
        assert!(article(feed, &item).is_none());
        item["title"] = json!("Goma");
        item["link"] = json!("javascript:alert(1)");
        assert!(article(feed, &item).is_none());
    }
    #[tokio::test]
    #[ignore]
    async fn live_newsdata() {
        dotenvy::dotenv().ok();
        let db = store::open(&std::env::var("LIVE_DB").unwrap_or("sqlite::memory:".into()))
            .await
            .unwrap();
        reference::init(&db).await.unwrap();
        let feed = feeds().iter().find(|f| f.id == "newsdata").unwrap();
        let result = poll(&db, feed).await.unwrap();
        reference::record(
            &db,
            "news-feed:newsdata",
            &Ok(result["drc"].as_u64().unwrap() as usize
                + result["regional"].as_u64().unwrap() as usize),
            result.clone(),
        )
        .await
        .unwrap();
        println!("{result}");
    }
}
