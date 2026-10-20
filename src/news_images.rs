//! Lead images for news stories: the publisher's own share image (og:image / twitter:image in the
//! stored article page, or `image_url` in API records). Only the URL and alt text are kept; the
//! browser loads the image from the publisher with a credit and a link to the story. Images reused
//! across many articles are the site's default picture, not the story's, and are not returned.
use crate::{ApiResult, App};
use anyhow::Result;
use axum::{Json, extract::State};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};

pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS article_images(article_id TEXT PRIMARY KEY, url TEXT, alt TEXT, checked_at TEXT NOT NULL);")
        .execute(db)
        .await?;
    Ok(())
}
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    for q in ['"', '\''] {
        let key = format!("{name}={q}");
        if let Some(i) = tag.find(&key) {
            let rest = &tag[i + key.len()..];
            return rest.find(q).map(|j| &rest[..j]);
        }
    }
    None
}
/// Content of the first <meta> whose property/name is one of `keys`.
fn meta(html: &str, keys: &[&str]) -> Option<String> {
    let mut rest = html;
    while let Some(i) = rest.find("<meta") {
        let tail = &rest[i..];
        let end = tail.find('>').unwrap_or(tail.len());
        let tag = &tail[..end];
        let key = attr(tag, "property")
            .or_else(|| attr(tag, "name"))
            .unwrap_or("");
        if keys.iter().any(|k| key.eq_ignore_ascii_case(k))
            && let Some(c) = attr(tag, "content").filter(|c| !c.trim().is_empty())
        {
            return Some(c.trim().replace("&amp;", "&"));
        }
        rest = &tail[end.min(tail.len())..];
        if rest.len() == tail.len() {
            break;
        }
    }
    None
}
/// Share image and alt text from a stored page (HTML) or API record (JSON).
pub fn extract(raw: &str) -> Option<(String, Option<String>)> {
    let url = meta(
        raw,
        &[
            "og:image",
            "og:image:url",
            "og:image:secure_url",
            "twitter:image",
        ],
    )
    .or_else(|| {
        serde_json::from_str::<Value>(raw)
            .ok()
            .and_then(|v| v["image_url"].as_str().map(str::to_string))
    })?;
    let ok = url::Url::parse(&url)
        .ok()
        .filter(|u| u.scheme() == "https" || u.scheme() == "http")?;
    let path = ok.path().to_lowercase();
    if path.ends_with(".svg")
        || path.contains("logo")
        || path.contains("placeholder")
        || path.contains("default-image")
    {
        return None;
    }
    let alt =
        meta(raw, &["og:image:alt", "twitter:image:alt"]).filter(|a| a.chars().count() <= 300);
    Some((ok.to_string(), alt))
}
/// Records the share image for articles not yet checked (latest stored version of each).
pub async fn scan(db: &SqlitePool, limit: i64) -> Result<usize> {
    let rows = sqlx::query("SELECT a.id, (SELECT v.html FROM versions v WHERE v.article_id=a.id ORDER BY v.fetched_at DESC LIMIT 1) AS html FROM articles a LEFT JOIN article_images i ON i.article_id=a.id WHERE i.article_id IS NULL ORDER BY a.first_seen_at DESC LIMIT ?")
        .bind(limit)
        .fetch_all(db)
        .await?;
    let now = chrono::Utc::now().to_rfc3339();
    for r in &rows {
        let html: Option<String> = r.get("html");
        let found = html.as_deref().and_then(extract);
        sqlx::query("INSERT OR REPLACE INTO article_images VALUES(?,?,?,?)")
            .bind(r.get::<String, _>("id"))
            .bind(found.as_ref().map(|f| f.0.clone()))
            .bind(found.and_then(|f| f.1))
            .bind(&now)
            .execute(db)
            .await?;
    }
    Ok(rows.len())
}
pub async fn worker(db: SqlitePool) -> Result<()> {
    loop {
        if let Err(e) = scan(&db, 500).await {
            tracing::warn!(error = %e, "news image scan failed");
        }
        tokio::time::sleep(std::time::Duration::from_secs(120)).await;
    }
}
/// article id → {url, alt}; images shared by more than three articles are treated as defaults.
pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let rows = sqlx::query("SELECT article_id, url, alt FROM article_images WHERE url IS NOT NULL AND url IN (SELECT url FROM article_images WHERE url IS NOT NULL GROUP BY url HAVING count(*) <= 3)")
        .fetch_all(&app.db)
        .await?;
    let mut out = serde_json::Map::new();
    for r in rows {
        out.insert(
            r.get("article_id"),
            json!({"url":r.get::<String,_>("url"),"alt":r.get::<Option<String>,_>("alt")}),
        );
    }
    Ok(Json(
        json!({"items":out,"notice":"Images are the publishers' own share images, loaded from their sites and credited; Congo Signal does not store or republish them."}),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn share_images_are_read_from_pages_and_api_records() {
        let html = r#"<head><meta property="og:title" content="x"><meta property="og:image" content="https://actualite.cd/files/a%20b.jpg?itok=1&amp;s=2" /><meta property="og:image:alt" content="Goma, 2026"></head>"#;
        assert_eq!(
            extract(html),
            Some((
                "https://actualite.cd/files/a%20b.jpg?itok=1&s=2".into(),
                Some("Goma, 2026".into())
            ))
        );
        assert_eq!(
            extract(r#"{"image_url":"https://depeche.cd/u/p.jpg","link":"x"}"#)
                .unwrap()
                .0,
            "https://depeche.cd/u/p.jpg"
        );
        assert!(
            extract(r#"<meta name="twitter:image" content="https://x.cd/logo.png">"#).is_none(),
            "logos skipped"
        );
        assert!(extract(r#"<meta property="og:image" content="javascript:alert(1)">"#).is_none());
        assert!(extract("<item><title>no image</title></item>").is_none());
    }
}
