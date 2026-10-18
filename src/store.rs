use crate::extract::{Article, digest};
use anyhow::Result;
use serde_json::Value;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};
use std::{str::FromStr, time::Duration};

pub async fn open(path: &str) -> Result<SqlitePool> {
    let opts = SqliteConnectOptions::from_str(path)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(10));
    let db = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(opts)
        .await?;
    sqlx::raw_sql(r#"
        CREATE TABLE IF NOT EXISTS articles (
          id TEXT PRIMARY KEY, url TEXT NOT NULL UNIQUE, title TEXT NOT NULL, content TEXT NOT NULL,
          category TEXT, published_at TEXT, first_seen_at TEXT NOT NULL, last_seen_at TEXT NOT NULL, data TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS versions (
          article_id TEXT NOT NULL, fingerprint TEXT NOT NULL, fetched_at TEXT NOT NULL,
          html TEXT NOT NULL, data TEXT NOT NULL, PRIMARY KEY(article_id, fingerprint));
        CREATE TABLE IF NOT EXISTS jobs (
          id TEXT PRIMARY KEY, url TEXT NOT NULL, kind TEXT NOT NULL, status TEXT NOT NULL,
          created_at TEXT NOT NULL, finished_at TEXT, error TEXT, result TEXT);
        CREATE UNIQUE INDEX IF NOT EXISTS jobs_active ON jobs(url,kind) WHERE status IN ('pending','running');
        CREATE INDEX IF NOT EXISTS articles_date ON articles(published_at);
    "#).execute(&db).await?;
    Ok(db)
}
pub async fn enqueue(db: &SqlitePool, url: &str, kind: &str) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT OR IGNORE INTO jobs(id,url,kind,status,created_at) VALUES(?,?,?,'pending',?)",
    )
    .bind(&id)
    .bind(url)
    .bind(kind)
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(db)
    .await?;
    Ok(sqlx::query_scalar(
        "SELECT id FROM jobs WHERE url=? AND kind=? AND status IN ('pending','running')",
    )
    .bind(url)
    .bind(kind)
    .fetch_one(db)
    .await?)
}
pub async fn save(db: &SqlitePool, a: &Article, html: &str) -> Result<()> {
    let data = serde_json::to_string(a)?;
    // Ignore acquisition timestamps and rotating ad HTML when identifying editorial revisions.
    let fingerprint = digest(
        serde_json::to_string(&serde_json::json!([
            a.title,
            a.description,
            a.category,
            a.author,
            a.language,
            a.published_at,
            a.modified_at,
            a.content
        ]))?
        .as_bytes(),
    );
    let mut tx = db.begin().await?;
    sqlx::query("INSERT OR IGNORE INTO versions VALUES(?,?,?,?,?)")
        .bind(&a.id)
        .bind(fingerprint)
        .bind(&a.fetched_at)
        .bind(html)
        .bind(&data)
        .execute(&mut *tx)
        .await?;
    sqlx::query(r#"INSERT INTO articles VALUES(?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET
        title=excluded.title, content=excluded.content, category=excluded.category,
        published_at=excluded.published_at, last_seen_at=excluded.last_seen_at, data=excluded.data"#)
        .bind(&a.id).bind(&a.url).bind(&a.title).bind(&a.content).bind(&a.category)
        .bind(&a.published_at).bind(&a.fetched_at).bind(&a.fetched_at).bind(data).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
pub fn json(s: String) -> Result<Value> {
    Ok(serde_json::from_str(&s)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn dedup_and_revisions() {
        let db = open("sqlite::memory:").await.unwrap();
        assert_eq!(
            enqueue(&db, "url", "article").await.unwrap(),
            enqueue(&db, "url", "article").await.unwrap()
        );
        let mut a = crate::extract::extract(
            include_str!("../tests/fixtures/article.html"),
            &crate::extract::normalize("https://actualite.cd/2026/10/12/test").unwrap(),
        )
        .unwrap();
        save(&db, &a, "original").await.unwrap();
        a.fetched_at = chrono::Utc::now().to_rfc3339();
        save(&db, &a, "different ads").await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM versions")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(count, 1);
        a.title = "Correction".into();
        save(&db, &a, "corrected").await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM versions")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(count, 2);
    }
}
