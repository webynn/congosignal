//! Topic monitoring and bounded public-post snapshots; no inferred account labels.
use crate::{ApiResult, App};
use anyhow::{Context, Result};
use axum::{Json, extract::State};
use chrono::{Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};

#[derive(Clone, Deserialize, Serialize)]
struct Topic {
    id: String,
    en: String,
    fr: String,
    query: String,
}
fn topics() -> Vec<Topic> {
    serde_json::from_str(include_str!("../config/x-topics.json")).expect("valid bundled topics")
}
fn configured() -> bool {
    std::env::var("X_BEARER_TOKEN").is_ok_and(|s| !s.trim().is_empty())
}
fn enabled() -> bool {
    std::env::var("X_ENABLED").as_deref() == Ok("1")
}
fn daily_limit() -> i64 {
    std::env::var("X_DAILY_REQUEST_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24)
        .clamp(0, 96)
}

pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS x_topics (id TEXT PRIMARY KEY, checked_at TEXT, next_at INTEGER NOT NULL DEFAULT 0, error TEXT, data TEXT); CREATE TABLE IF NOT EXISTS x_requests (id INTEGER PRIMARY KEY AUTOINCREMENT, day TEXT NOT NULL, topic TEXT NOT NULL, reserved_at TEXT NOT NULL); CREATE INDEX IF NOT EXISTS x_requests_day ON x_requests(day);").execute(db).await?;
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS x_post_topics (id TEXT PRIMARY KEY, next_at INTEGER NOT NULL DEFAULT 0, checked_at TEXT, error TEXT, data TEXT); CREATE TABLE IF NOT EXISTS x_post_requests (id INTEGER PRIMARY KEY AUTOINCREMENT, day TEXT NOT NULL, topic TEXT NOT NULL);").execute(db).await?;
    for t in topics() {
        sqlx::query("INSERT OR IGNORE INTO x_post_topics(id) VALUES(?)")
            .bind(&t.id)
            .execute(db)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO x_topics(id) VALUES(?)")
            .bind(t.id)
            .execute(db)
            .await?;
    }
    Ok(())
}

#[derive(Clone, Deserialize, Serialize, Debug)]
struct Bucket {
    start: String,
    end: String,
    tweet_count: u64,
}
#[derive(Deserialize)]
struct Counts {
    data: Option<Vec<Bucket>>,
    errors: Option<Value>,
    meta: Option<Value>,
}
fn validated_counts(
    body: Counts,
    start: chrono::DateTime<Utc>,
    end: chrono::DateTime<Utc>,
) -> Result<Vec<Bucket>> {
    anyhow::ensure!(
        body.errors
            .as_ref()
            .is_none_or(|v| v.as_array().is_some_and(|a| a.is_empty())),
        "X returned a partial response; coverage is unavailable"
    );
    anyhow::ensure!(
        body.meta
            .as_ref()
            .and_then(|v| v.get("next_token"))
            .is_none(),
        "X returned paginated counts; coverage is incomplete"
    );
    let mut buckets = body.data.context("X returned no count buckets")?;
    buckets.sort_by(|a, b| a.start.cmp(&b.start));
    anyhow::ensure!(buckets.len() == 24, "Expected 24 complete hourly buckets");
    for (i, b) in buckets.iter().enumerate() {
        let s = chrono::DateTime::parse_from_rfc3339(&b.start)?.with_timezone(&Utc);
        let e = chrono::DateTime::parse_from_rfc3339(&b.end)?.with_timezone(&Utc);
        anyhow::ensure!(
            s == start + Duration::hours(i as i64) && e == s + Duration::hours(1) && e <= end,
            "Invalid hourly coverage"
        );
    }
    Ok(buckets)
}
fn summary(buckets: &[Bucket]) -> Value {
    let total: u64 = buckets.iter().map(|b| b.tweet_count).sum();
    let latest = buckets.last().map_or(0, |b| b.tweet_count);
    let previous: Vec<_> = buckets.iter().rev().skip(1).take(6).collect();
    let baseline = if previous.is_empty() {
        0.0
    } else {
        previous.iter().map(|b| b.tweet_count as f64).sum::<f64>() / previous.len() as f64
    };
    let spike = previous.len() == 6 && latest >= 20 && latest as f64 >= 3.0 * baseline.max(1.0);
    json!({"total":total,"latest_hour":latest,"baseline_hourly":baseline,"rising":spike,"rule":"latest complete hour >= 20 matching posts and >= 3x previous six-hour mean (floor 1)","buckets":buckets})
}

// Atomic reservation: attempted requests count even after a timeout or process crash.
async fn reserve(db: &SqlitePool, id: &str, limit: i64) -> Result<bool> {
    let now = Utc::now();
    let day = now.format("%Y-%m-%d").to_string();
    let mut tx = db.begin().await?;
    let claimed = sqlx::query("UPDATE x_topics SET next_at=? WHERE id=? AND next_at<=? AND (SELECT count(*) FROM x_requests WHERE day=?)<?")
        .bind(now.timestamp()+14400).bind(id).bind(now.timestamp()).bind(&day).bind(limit).execute(&mut *tx).await?.rows_affected();
    if claimed == 0 {
        tx.rollback().await?;
        return Ok(false);
    }
    sqlx::query("INSERT INTO x_requests(day,topic,reserved_at) VALUES(?,?,?)")
        .bind(day)
        .bind(id)
        .bind(now.to_rfc3339())
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(true)
}
async fn collect(
    db: &SqlitePool,
    client: &reqwest::Client,
    topic: &Topic,
    token: &str,
) -> Result<()> {
    // End on a complete UTC hour, behind X's indexing delay.
    let end = (Utc::now() - Duration::seconds(30))
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    let start = end - Duration::hours(24);
    let response = client
        .get("https://api.x.com/2/tweets/counts/recent")
        .bearer_auth(token)
        .query(&[
            ("query", topic.query.clone()),
            ("granularity", "hour".into()),
            ("start_time", start.to_rfc3339()),
            ("end_time", end.to_rfc3339()),
        ])
        .send()
        .await
        .context("X connection failed")?;
    let status = response.status();
    if status.as_u16() == 429 {
        let reset = response
            .headers()
            .get("x-rate-limit-reset")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(Utc::now().timestamp() + 3600);
        sqlx::query("UPDATE x_topics SET next_at=max(next_at, ?)")
            .bind(reset + 60)
            .execute(db)
            .await?;
    }
    // Never persist arbitrary upstream error bodies (may contain sensitive request details).
    anyhow::ensure!(
        status.is_success(),
        "X HTTP {} — check access, credits or rate limits",
        status.as_u16()
    );
    let body: Counts = response.json().await.context("Invalid counts response")?;
    let buckets = validated_counts(body, start, end)?;
    let mut data = summary(&buckets);
    data["window_start"] = json!(start.to_rfc3339());
    data["window_end"] = json!(end.to_rfc3339());
    sqlx::query("UPDATE x_topics SET checked_at=?,error=NULL,data=? WHERE id=?")
        .bind(Utc::now().to_rfc3339())
        .bind(data.to_string())
        .bind(&topic.id)
        .execute(db)
        .await?;
    Ok(())
}
pub async fn worker(db: SqlitePool) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    loop {
        sqlx::query("UPDATE x_post_topics SET data=NULL WHERE checked_at<?")
            .bind((Utc::now() - Duration::hours(24)).to_rfc3339())
            .execute(&db)
            .await?;
        if enabled() && configured() {
            let token = std::env::var("X_BEARER_TOKEN")?;
            // Least recently attempted first; prevents starvation at a small daily cap.
            let ids: Vec<String> =
                sqlx::query_scalar("SELECT id FROM x_topics ORDER BY next_at,id")
                    .fetch_all(&db)
                    .await?;
            for id in ids {
                if let Some(topic) = topics().into_iter().find(|t| t.id == id) {
                    if reserve(&db, &id, daily_limit()).await? {
                        if let Err(e) = collect(&db, &client, &topic, &token).await {
                            sqlx::query("UPDATE x_topics SET error=? WHERE id=?")
                                .bind(e.to_string())
                                .bind(&id)
                                .execute(&db)
                                .await?;
                        }
                    }
                }
            }
        }
        if enabled() && configured() {
            collect_posts_cycle(&db, &client).await?;
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
pub async fn feed(State(app): State<App>) -> ApiResult<Json<Value>> {
    let day = Utc::now().format("%Y-%m-%d").to_string();
    let used: i64 = sqlx::query_scalar("SELECT count(*) FROM x_requests WHERE day=?")
        .bind(day)
        .fetch_one(&app.db)
        .await?;
    let mut items = Vec::new();
    for topic in topics() {
        let row = sqlx::query("SELECT * FROM x_topics WHERE id=?")
            .bind(&topic.id)
            .fetch_one(&app.db)
            .await?;
        let data: Option<String> = row.get("data");
        let checked: Option<String> = row.get("checked_at");
        let stale = checked
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .is_none_or(|d| Utc::now().signed_duration_since(d).num_minutes() > 270);
        let mut search = url::Url::parse("https://x.com/search").unwrap();
        search
            .query_pairs_mut()
            .append_pair("q", &topic.query)
            .append_pair("f", "live");
        items.push(json!({"topic":topic,"data":data.and_then(|d|serde_json::from_str::<Value>(&d).ok()),"checked_at":checked,"stale":stale,"error":row.get::<Option<String>,_>("error"),"search_url":search.as_str()}));
    }
    Ok(Json(
        json!({"mode":"aggregate_counts","poll_seconds":14400,"enabled":enabled(),"configured":configured(),"requests_today":used,"daily_request_limit":daily_limit(),"budget_paused":used>=daily_limit(),"items":items,"generated_at":Utc::now().to_rfc3339(),"notice":"Counts include reposts and overlapping topics. They do not establish independent sources, endorsement or verified events. No account profiles are collected."}),
    ))
}

fn post_limit() -> i64 {
    std::env::var("X_POST_REQUEST_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4)
        .clamp(0, 24)
}
fn normalize_posts(body: &Value) -> Result<Vec<Value>> {
    anyhow::ensure!(
        body.get("errors").is_none(),
        "X returned incomplete post data"
    );
    let empty = Vec::new();
    let authors = body
        .pointer("/includes/users")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let posts = body.get("data").and_then(Value::as_array).unwrap_or(&empty);
    anyhow::ensure!(posts.len() <= 10, "Unexpected post batch size");
    let mut out = Vec::new();
    for p in posts {
        let id = p["id"].as_str().context("Missing post ID")?;
        anyhow::ensure!(
            !id.is_empty() && id.bytes().all(|c| c.is_ascii_digit()),
            "Invalid post ID"
        );
        let a = authors
            .iter()
            .find(|a| a["id"] == p["author_id"])
            .context("Missing public author attribution")?;
        let handle = a["username"].as_str().context("Missing author handle")?;
        anyhow::ensure!(
            !handle.is_empty()
                && handle
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_'),
            "Invalid author handle"
        );
        let text = p
            .pointer("/note_tweet/text")
            .or_else(|| p.get("text"))
            .and_then(Value::as_str)
            .context("Missing post text")?;
        let created = p["created_at"]
            .as_str()
            .context("Missing publication time")?;
        chrono::DateTime::parse_from_rfc3339(created)?;
        out.push(json!({"id":id,"text":text,"created_at":created,"lang":p["lang"].as_str().unwrap_or("und"),"author_name":a["name"].as_str().unwrap_or(handle),"author_handle":handle,"url":format!("https://x.com/{handle}/status/{id}"),"author_url":format!("https://x.com/{handle}"),"references":p["referenced_tweets"]}));
    }
    Ok(out)
}
async fn reserve_posts(db: &SqlitePool, id: &str, limit: i64) -> Result<bool> {
    let mut tx = db.begin().await?;
    let now = Utc::now();
    let day = now.format("%Y-%m-%d").to_string();
    let n=sqlx::query("UPDATE x_post_topics SET next_at=? WHERE id=? AND next_at<=? AND (SELECT count(*) FROM x_post_requests WHERE day=?)<?")
        .bind(now.timestamp()+86400).bind(id).bind(now.timestamp()).bind(&day).bind(limit).execute(&mut *tx).await?.rows_affected();
    if n == 0 {
        tx.rollback().await?;
        return Ok(false);
    }
    sqlx::query("INSERT INTO x_post_requests(day,topic) VALUES(?,?)")
        .bind(day)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(true)
}
async fn fetch_posts(
    db: &SqlitePool,
    client: &reqwest::Client,
    topic: &Topic,
    token: &str,
) -> Result<()> {
    let start = (Utc::now() - Duration::hours(24)).to_rfc3339();
    let r = client
        .get("https://api.x.com/2/tweets/search/recent")
        .bearer_auth(token)
        .query(&[
            ("query", topic.query.as_str()),
            ("max_results", "10"),
            ("sort_order", "recency"),
            ("start_time", start.as_str()),
            (
                "tweet.fields",
                "created_at,lang,author_id,referenced_tweets,note_tweet",
            ),
            ("expansions", "author_id"),
            ("user.fields", "name,username"),
        ])
        .send()
        .await
        .context("X post connection failed")?;
    let status = r.status();
    if status.as_u16() == 429 {
        let reset = r
            .headers()
            .get("x-rate-limit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(Utc::now().timestamp() + 3600);
        sqlx::query("UPDATE x_post_topics SET next_at=max(next_at,?)")
            .bind(reset + 60)
            .execute(db)
            .await?;
    }
    anyhow::ensure!(
        status.is_success(),
        "X posts HTTP {} — check access, credits or rate limits",
        status.as_u16()
    );
    let body: Value = r.json().await.context("Invalid X post response")?;
    let posts = normalize_posts(&body)?;
    sqlx::query("UPDATE x_post_topics SET checked_at=?,error=NULL,data=? WHERE id=?")
        .bind(Utc::now().to_rfc3339())
        .bind(serde_json::to_string(&posts)?)
        .bind(&topic.id)
        .execute(db)
        .await?;
    Ok(())
}
async fn collect_posts_cycle(db: &SqlitePool, client: &reqwest::Client) -> Result<()> {
    // Remove expired content, including authors, rather than serving stale post snapshots.
    sqlx::query("UPDATE x_post_topics SET data=NULL WHERE checked_at<?")
        .bind((Utc::now() - Duration::hours(24)).to_rfc3339())
        .execute(db)
        .await?;
    let token = std::env::var("X_BEARER_TOKEN")?;
    for t in topics() {
        if reserve_posts(db, &t.id, post_limit()).await? {
            if let Err(e) = fetch_posts(db, client, &t, &token).await {
                sqlx::query("UPDATE x_post_topics SET data=NULL,error=? WHERE id=?")
                    .bind(e.to_string())
                    .bind(&t.id)
                    .execute(db)
                    .await?;
            }
        }
    }
    Ok(())
}
pub async fn posts(State(app): State<App>) -> ApiResult<Json<Value>> {
    let day = Utc::now().format("%Y-%m-%d").to_string();
    let used: i64 = sqlx::query_scalar("SELECT count(*) FROM x_post_requests WHERE day=?")
        .bind(day)
        .fetch_one(&app.db)
        .await?;
    let mut items = Vec::new();
    let mut sources = Vec::new();
    let cutoff = (Utc::now() - Duration::hours(24)).to_rfc3339();
    for t in topics() {
        let r = sqlx::query("SELECT checked_at,error,data FROM x_post_topics WHERE id=?")
            .bind(&t.id)
            .fetch_one(&app.db)
            .await?;
        let checked: Option<String> = r.get("checked_at");
        sources.push(
            json!({"topic":t.id,"checked_at":checked,"error":r.get::<Option<String>,_>("error")}),
        );
        if checked.as_ref().is_some_and(|s| s >= &cutoff) {
            if let Some(raw) = r.get::<Option<String>, _>("data") {
                let batch: Vec<Value> = serde_json::from_str(&raw).map_err(anyhow::Error::from)?;
                for mut p in batch {
                    p["topic"] = json!(t.id);
                    p["collected_at"] = json!(checked);
                    items.push(p);
                }
            }
        }
    }
    items.sort_by(|a, b| b["created_at"].as_str().cmp(&a["created_at"].as_str()));
    Ok(Json(
        json!({"items":items,"sources":sources,"requests_today":used,"daily_request_limit":post_limit(),"sample_size_per_topic":10,"poll_seconds":86400,"coverage":"latest matching posts sampled; not exhaustive"}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn posts_preserve_attribution_without_labels() {
        let body = json!({"data":[{"id":"123","author_id":"99","text":"Public statement <script>","created_at":"2026-10-18T12:00:00Z","lang":"fr"}],"includes":{"users":[{"id":"99","name":"Example","username":"example"}]}});
        let posts = normalize_posts(&body).unwrap();
        assert_eq!(posts[0]["author_handle"], "example");
        assert_eq!(posts[0]["text"], "Public statement <script>");
        assert!(posts[0].get("stance").is_none());
        let mut invalid = body.clone();
        invalid["includes"]["users"][0]["username"] = json!("bad/handle");
        assert!(normalize_posts(&invalid).is_err());
        let mut missing = body.clone();
        missing["includes"] = json!({});
        assert!(normalize_posts(&missing).is_err());
    }
    #[tokio::test]
    async fn post_budget_and_cooldown_are_persistent() {
        let db = crate::store::open("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        assert!(reserve_posts(&db, "armed-groups", 1).await.unwrap());
        assert!(!reserve_posts(&db, "armed-groups", 1).await.unwrap());
        assert!(!reserve_posts(&db, "diplomacy", 1).await.unwrap());
    }
    #[tokio::test]
    async fn expired_posts_are_not_served() {
        let db = crate::store::open("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        sqlx::query("UPDATE x_post_topics SET checked_at=?,data=? WHERE id='armed-groups'")
            .bind((Utc::now() - Duration::hours(25)).to_rfc3339())
            .bind("[{\"id\":\"123\",\"text\":\"expired\"}]")
            .execute(&db)
            .await
            .unwrap();
        let Json(result) = posts(State(App { db, token: None })).await.ok().unwrap();
        assert_eq!(result["items"], json!([]));
    }
    #[test]
    fn rules_fit_recent_counts() {
        for t in topics() {
            assert!(t.query.chars().count() <= 512);
            assert!(!t.query.contains("lang:"));
        }
    }
    #[test]
    fn surge_requires_volume_and_baseline() {
        let mut b: Vec<_> = (0..24)
            .map(|_| Bucket {
                start: String::new(),
                end: String::new(),
                tweet_count: 2,
            })
            .collect();
        b[23].tweet_count = 19;
        assert_eq!(summary(&b)["rising"], false);
        b[23].tweet_count = 20;
        assert_eq!(summary(&b)["rising"], true);
        assert_eq!(summary(&b[..1])["rising"], false);
    }
    #[tokio::test]
    async fn reservation_is_durable_and_capped() {
        let db = crate::store::open("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        assert!(reserve(&db, "armed-groups", 1).await.unwrap());
        assert!(!reserve(&db, "armed-groups", 1).await.unwrap());
        assert!(!reserve(&db, "diplomacy", 1).await.unwrap());
    }
    #[test]
    fn complete_counts_validate_and_gaps_fail() {
        let end = chrono::DateTime::parse_from_rfc3339("2026-10-18T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let start = end - Duration::hours(24);
        let buckets: Vec<_> = (0..24)
            .map(|i| Bucket {
                start: (start + Duration::hours(i)).to_rfc3339(),
                end: (start + Duration::hours(i + 1)).to_rfc3339(),
                tweet_count: 0,
            })
            .collect();
        assert!(
            validated_counts(
                Counts {
                    data: Some(buckets.clone()),
                    errors: None,
                    meta: None
                },
                start,
                end
            )
            .is_ok()
        );
        let mut invalid = buckets.clone();
        invalid[5] = invalid[4].clone();
        assert!(
            validated_counts(
                Counts {
                    data: Some(invalid),
                    errors: None,
                    meta: None
                },
                start,
                end
            )
            .is_err()
        );
        assert!(
            validated_counts(
                Counts {
                    data: Some(buckets),
                    errors: None,
                    meta: Some(json!({"next_token":"more"}))
                },
                start,
                end
            )
            .is_err()
        );
    }
    #[test]
    fn missing_buckets_are_not_silent_zeros() {
        let end = Utc::now();
        let start = end - Duration::hours(24);
        assert!(
            validated_counts(
                Counts {
                    data: Some(vec![]),
                    errors: None,
                    meta: None
                },
                start,
                end
            )
            .is_err()
        );
        assert!(
            validated_counts(
                Counts {
                    data: None,
                    errors: Some(json!([{"detail":"partial"}])),
                    meta: None
                },
                start,
                end
            )
            .is_err()
        );
    }
}
