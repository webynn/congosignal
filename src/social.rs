//! X topic monitoring: aggregate counts, a retained archive of public posts that match the
//! configured topic queries, and on-demand account histories filtered to those topics.
//! Accounts are attributed, never labeled: no stance, affiliation or bot scores are inferred.
use crate::{ApiError, ApiResult, App};
use anyhow::{Context, Result};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{DateTime, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

#[derive(Clone, Deserialize, Serialize)]
struct Topic {
    id: String,
    en: String,
    fr: String,
    query: String,
}
static TOPICS: LazyLock<Vec<(Topic, Rule)>> = LazyLock::new(|| {
    let topics: Vec<Topic> = serde_json::from_str(include_str!("../config/x-topics.json"))
        .expect("valid bundled topics");
    topics
        .into_iter()
        .map(|t| {
            let rule = parse_rule(&t.query).expect("valid topic query");
            (t, rule)
        })
        .collect()
});
fn topics() -> impl Iterator<Item = &'static Topic> {
    TOPICS.iter().map(|(t, _)| t)
}
fn configured() -> bool {
    std::env::var("X_BEARER_TOKEN").is_ok_and(|s| !s.trim().is_empty())
}
fn enabled() -> bool {
    std::env::var("X_ENABLED").as_deref() == Ok("1")
}
fn env_i64(key: &str, default: i64, min: i64, max: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
        .clamp(min, max)
}
fn daily_limit() -> i64 {
    env_i64("X_DAILY_REQUEST_LIMIT", 24, 0, 96)
}
fn post_limit() -> i64 {
    env_i64("X_POST_REQUEST_LIMIT", 4, 0, 24)
}
fn posts_per_topic() -> i64 {
    env_i64("X_POSTS_PER_TOPIC", 10, 10, 100)
}
fn account_limit() -> i64 {
    env_i64("X_ACCOUNT_REQUEST_LIMIT", 10, 0, 100)
}
fn history_pages() -> i64 {
    env_i64("X_ACCOUNT_HISTORY_PAGES", 1, 1, 5)
}
fn retention_days() -> i64 {
    env_i64("X_RETENTION_DAYS", 90, 1, 3650)
}
// Test-only override; the bearer token is sent to this origin.
fn api_base() -> String {
    std::env::var("X_API_URL")
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|_| "https://api.x.com".into())
}
const TWEET_FIELDS: &str = "created_at,lang,author_id,referenced_tweets,note_tweet,public_metrics,entities,conversation_id";
const USER_FIELDS: &str =
    "name,username,created_at,description,location,public_metrics,verified,verified_type,protected";
const HISTORY_COOLDOWN_MINUTES: i64 = 10;
/// X's own timestamp shape, so stored values compare lexically.
fn xts(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}
fn today() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS x_topics (id TEXT PRIMARY KEY, checked_at TEXT, next_at INTEGER NOT NULL DEFAULT 0, error TEXT, data TEXT); CREATE TABLE IF NOT EXISTS x_requests (id INTEGER PRIMARY KEY AUTOINCREMENT, day TEXT NOT NULL, topic TEXT NOT NULL, reserved_at TEXT NOT NULL); CREATE INDEX IF NOT EXISTS x_requests_day ON x_requests(day);").execute(db).await?;
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS x_post_topics (id TEXT PRIMARY KEY, next_at INTEGER NOT NULL DEFAULT 0, checked_at TEXT, error TEXT, data TEXT); CREATE TABLE IF NOT EXISTS x_post_requests (id INTEGER PRIMARY KEY AUTOINCREMENT, day TEXT NOT NULL, topic TEXT NOT NULL);").execute(db).await?;
    sqlx::raw_sql(
        r#"
        CREATE TABLE IF NOT EXISTS x_accounts (
          id TEXT PRIMARY KEY, handle TEXT NOT NULL, name TEXT NOT NULL, profile TEXT,
          previous_handles TEXT NOT NULL DEFAULT '[]', first_seen_at TEXT NOT NULL, last_seen_at TEXT NOT NULL,
          history_checked_at TEXT, history_newest_id TEXT, history_last TEXT, history_error TEXT);
        CREATE INDEX IF NOT EXISTS x_accounts_handle ON x_accounts(lower(handle));
        CREATE TABLE IF NOT EXISTS x_posts (
          id TEXT PRIMARY KEY, author_id TEXT NOT NULL, created_at TEXT NOT NULL, lang TEXT,
          text TEXT NOT NULL, data TEXT NOT NULL, first_seen_at TEXT NOT NULL, last_seen_at TEXT NOT NULL);
        CREATE INDEX IF NOT EXISTS x_posts_author ON x_posts(author_id, created_at);
        CREATE INDEX IF NOT EXISTS x_posts_created ON x_posts(created_at);
        CREATE TABLE IF NOT EXISTS x_post_matches (
          post_id TEXT NOT NULL, topic TEXT NOT NULL, terms TEXT NOT NULL, via TEXT NOT NULL,
          PRIMARY KEY(post_id, topic));
        CREATE INDEX IF NOT EXISTS x_post_matches_topic ON x_post_matches(topic);
        CREATE TABLE IF NOT EXISTS x_counts (topic TEXT NOT NULL, start TEXT NOT NULL, tweet_count INTEGER NOT NULL, PRIMARY KEY(topic, start));
        CREATE TABLE IF NOT EXISTS x_account_requests (id INTEGER PRIMARY KEY AUTOINCREMENT, day TEXT NOT NULL, handle TEXT NOT NULL, kind TEXT NOT NULL);
        CREATE INDEX IF NOT EXISTS x_account_requests_day ON x_account_requests(day);
        "#,
    )
    .execute(db)
    .await?;
    for t in topics() {
        sqlx::query("INSERT OR IGNORE INTO x_post_topics(id) VALUES(?)")
            .bind(&t.id)
            .execute(db)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO x_topics(id) VALUES(?)")
            .bind(&t.id)
            .execute(db)
            .await?;
    }
    // Seed the hourly history from count snapshots stored before x_counts existed.
    sqlx::query("INSERT OR IGNORE INTO x_counts(topic,start,tweet_count) SELECT t.id, strftime('%Y-%m-%dT%H:%M:%fZ', json_extract(b.value,'$.start')), json_extract(b.value,'$.tweet_count') FROM x_topics t, json_each(t.data,'$.buckets') b WHERE t.data IS NOT NULL")
        .execute(db)
        .await?;
    // Earlier releases kept a 24-hour post snapshot (a JSON array) that lacked account IDs; it is
    // superseded by the archive. Re-arm sampling once so the archive fills within the daily cap.
    let legacy = sqlx::query("UPDATE x_post_topics SET data=NULL WHERE data LIKE '[%'")
        .execute(db)
        .await?
        .rows_affected();
    if legacy > 0 {
        sqlx::query("UPDATE x_post_topics SET next_at=0")
            .execute(db)
            .await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Topic rules: a local evaluator for the subset of X query syntax used in config/x-topics.json
// (terms, "quoted phrases", OR, implicit AND, parentheses, -negation). It tags posts with the
// topics and terms they match; it approximates X's server-side matching, not a replacement.

#[derive(Debug, Clone)]
enum Rule {
    Term(String, Vec<String>),
    Any,
    Not(Box<Rule>),
    And(Vec<Rule>),
    Or(Vec<Rule>),
}
#[derive(Debug, PartialEq)]
enum Lex {
    Open,
    Close,
    Or,
    Neg,
    Word(String),
    Phrase(String),
}
fn tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(String::from)
        .collect()
}
fn lex(q: &str) -> Result<Vec<Lex>> {
    let mut out = Vec::new();
    let mut chars = q.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            c if c.is_whitespace() => {
                chars.next();
            }
            '(' => {
                chars.next();
                out.push(Lex::Open)
            }
            ')' => {
                chars.next();
                out.push(Lex::Close)
            }
            '"' => {
                chars.next();
                let phrase: String = chars.by_ref().take_while(|&c| c != '"').collect();
                out.push(Lex::Phrase(phrase))
            }
            '-' => {
                chars.next();
                out.push(Lex::Neg)
            }
            _ => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_whitespace() || c == '(' || c == ')' || c == '"' {
                        break;
                    }
                    word.push(c);
                    chars.next();
                }
                out.push(if word == "OR" {
                    Lex::Or
                } else {
                    Lex::Word(word)
                })
            }
        }
    }
    Ok(out)
}
fn parse_rule(q: &str) -> Result<Rule> {
    let lexed = lex(q)?;
    let mut pos = 0;
    let rule = parse_or(&lexed, &mut pos)?;
    anyhow::ensure!(pos == lexed.len(), "Unbalanced topic query");
    Ok(rule)
}
// X applies AND before OR.
fn parse_or(l: &[Lex], pos: &mut usize) -> Result<Rule> {
    let mut alts = vec![parse_and(l, pos)?];
    while l.get(*pos) == Some(&Lex::Or) {
        *pos += 1;
        alts.push(parse_and(l, pos)?);
    }
    Ok(if alts.len() == 1 {
        alts.remove(0)
    } else {
        Rule::Or(alts)
    })
}
fn parse_and(l: &[Lex], pos: &mut usize) -> Result<Rule> {
    let mut all = Vec::new();
    while let Some(tok) = l.get(*pos) {
        if matches!(tok, Lex::Or | Lex::Close) {
            break;
        }
        all.push(parse_unary(l, pos)?);
    }
    anyhow::ensure!(!all.is_empty(), "Empty topic query group");
    Ok(if all.len() == 1 {
        all.remove(0)
    } else {
        Rule::And(all)
    })
}
fn parse_unary(l: &[Lex], pos: &mut usize) -> Result<Rule> {
    let tok = l.get(*pos).context("Unexpected end of topic query")?;
    *pos += 1;
    Ok(match tok {
        Lex::Neg => Rule::Not(Box::new(parse_unary(l, pos)?)),
        Lex::Open => {
            let inner = parse_or(l, pos)?;
            anyhow::ensure!(
                l.get(*pos) == Some(&Lex::Close),
                "Missing ')' in topic query"
            );
            *pos += 1;
            inner
        }
        // Operators such as has:links are not evaluated locally.
        Lex::Word(w) if w.contains(':') => Rule::Any,
        Lex::Word(w) | Lex::Phrase(w) => Rule::Term(w.clone(), tokens(w)),
        Lex::Close | Lex::Or => anyhow::bail!("Unexpected operator in topic query"),
    })
}
impl Rule {
    fn eval(&self, toks: &[String], hits: &mut Vec<String>) -> bool {
        match self {
            Rule::Any => true,
            Rule::Term(label, needle) => {
                let hit = !needle.is_empty() && toks.windows(needle.len()).any(|w| w == needle);
                if hit && !hits.contains(label) {
                    hits.push(label.clone());
                }
                hit
            }
            Rule::Not(inner) => !inner.eval(toks, &mut Vec::new()),
            Rule::And(all) => {
                let mut local = Vec::new();
                let ok = all.iter().all(|r| r.eval(toks, &mut local));
                if ok {
                    for h in local {
                        if !hits.contains(&h) {
                            hits.push(h);
                        }
                    }
                }
                ok
            }
            Rule::Or(alts) => {
                // Evaluate every branch (no short-circuit) so all matching terms are reported.
                let mut any = false;
                for r in alts {
                    any |= r.eval(toks, hits);
                }
                any
            }
        }
    }
}
/// Topic IDs and matched terms for a post's text.
fn match_topics(text: &str) -> Vec<(String, Vec<String>)> {
    let toks = tokens(text);
    TOPICS
        .iter()
        .filter_map(|(t, rule)| {
            let mut hits = Vec::new();
            rule.eval(&toks, &mut hits).then(|| (t.id.clone(), hits))
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// X HTTP

#[derive(Debug)]
struct RateLimited(i64);
impl std::fmt::Display for RateLimited {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let at = DateTime::from_timestamp(self.0, 0)
            .map_or("later".into(), |t| t.format("%H:%M UTC").to_string());
        write!(f, "X rate limit reached; retry after {at}")
    }
}
impl std::error::Error for RateLimited {}

fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?)
}
async fn x_get(
    client: &reqwest::Client,
    token: &str,
    path: &str,
    query: &[(&str, String)],
) -> Result<Value> {
    let response = client
        .get(format!("{}{}", api_base(), path))
        .bearer_auth(token)
        .query(query)
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
        return Err(RateLimited(reset).into());
    }
    // Never persist arbitrary upstream error bodies (may contain sensitive request details).
    anyhow::ensure!(
        status.is_success(),
        "X HTTP {} — check access level, credits or rate limits",
        status.as_u16()
    );
    response.json().await.context("Invalid X response")
}
async fn defer_on_rate_limit(db: &SqlitePool, table: &str, e: &anyhow::Error) -> Result<()> {
    if let Some(RateLimited(reset)) = e.downcast_ref::<RateLimited>() {
        sqlx::query(&format!("UPDATE {table} SET next_at=max(next_at, ?)"))
            .bind(reset + 60)
            .execute(db)
            .await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Aggregate counts

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
fn validated_counts(body: Counts, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<Bucket>> {
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
        let s = DateTime::parse_from_rfc3339(&b.start)?.with_timezone(&Utc);
        let e = DateTime::parse_from_rfc3339(&b.end)?.with_timezone(&Utc);
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
    let day = today();
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
    let body = x_get(
        client,
        token,
        "/2/tweets/counts/recent",
        &[
            ("query", topic.query.clone()),
            ("granularity", "hour".into()),
            ("start_time", start.to_rfc3339()),
            ("end_time", end.to_rfc3339()),
        ],
    )
    .await?;
    let body: Counts = serde_json::from_value(body).context("Invalid counts response")?;
    let buckets = validated_counts(body, start, end)?;
    let mut data = summary(&buckets);
    data["window_start"] = json!(start.to_rfc3339());
    data["window_end"] = json!(end.to_rfc3339());
    let mut tx = db.begin().await?;
    for b in &buckets {
        // Later snapshots supersede earlier counts for the same hour.
        let hour = DateTime::parse_from_rfc3339(&b.start)?.with_timezone(&Utc);
        sqlx::query("INSERT INTO x_counts(topic,start,tweet_count) VALUES(?,?,?) ON CONFLICT(topic,start) DO UPDATE SET tweet_count=excluded.tweet_count")
            .bind(&topic.id)
            .bind(xts(hour))
            .bind(b.tweet_count as i64)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("UPDATE x_topics SET checked_at=?,error=NULL,data=? WHERE id=?")
        .bind(Utc::now().to_rfc3339())
        .bind(data.to_string())
        .bind(&topic.id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Posts and accounts

fn valid_handle(h: &str) -> bool {
    !h.is_empty() && h.len() <= 15 && h.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
}
fn valid_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 20 && s.bytes().all(|c| c.is_ascii_digit())
}
/// Public profile fields only; no derived labels.
fn normalize_user(u: &Value) -> Result<Value> {
    let id = u["id"].as_str().context("Missing account ID")?;
    anyhow::ensure!(valid_id(id), "Invalid account ID");
    let handle = u["username"].as_str().context("Missing author handle")?;
    anyhow::ensure!(valid_handle(handle), "Invalid author handle");
    Ok(json!({
        "id": id, "handle": handle, "name": u["name"].as_str().unwrap_or(handle),
        "profile": {
            "created_at": u["created_at"], "description": u["description"], "location": u["location"],
            "public_metrics": u["public_metrics"], "verified": u["verified"],
            "verified_type": u["verified_type"], "protected": u["protected"],
        }
    }))
}
fn entity_list(p: &Value, kind: &str, field: &str) -> Vec<String> {
    let mut out: Vec<String> = p
        .pointer(&format!("/entities/{kind}"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|e| e[field].as_str())
        .map(String::from)
        .collect();
    out.dedup();
    out
}
fn normalize_posts(body: &Value, max: usize) -> Result<(Vec<Value>, HashMap<String, Value>)> {
    anyhow::ensure!(
        body.get("errors").is_none(),
        "X returned incomplete post data"
    );
    let empty = Vec::new();
    let mut users = HashMap::new();
    for u in body
        .pointer("/includes/users")
        .and_then(Value::as_array)
        .unwrap_or(&empty)
    {
        let account = normalize_user(u)?;
        users.insert(
            account["id"].as_str().unwrap_or_default().to_string(),
            account,
        );
    }
    let posts = body.get("data").and_then(Value::as_array).unwrap_or(&empty);
    anyhow::ensure!(posts.len() <= max, "Unexpected post batch size");
    let mut out = Vec::new();
    for p in posts {
        let id = p["id"].as_str().context("Missing post ID")?;
        anyhow::ensure!(valid_id(id), "Invalid post ID");
        let author_id = p["author_id"].as_str().context("Missing author ID")?;
        let author = users
            .get(author_id)
            .context("Missing public author attribution")?;
        let text = p
            .pointer("/note_tweet/text")
            .or_else(|| p.get("text"))
            .and_then(Value::as_str)
            .context("Missing post text")?;
        let created = p["created_at"]
            .as_str()
            .context("Missing publication time")?;
        let created = DateTime::parse_from_rfc3339(created)?.with_timezone(&Utc);
        let kind = match p
            .pointer("/referenced_tweets/0/type")
            .and_then(Value::as_str)
        {
            Some("retweeted") => "repost",
            Some("replied_to") => "reply",
            Some("quoted") => "quote",
            _ => "post",
        };
        let domains: Vec<String> = entity_list(p, "urls", "expanded_url")
            .iter()
            .filter_map(|u| url::Url::parse(u).ok()?.host_str().map(String::from))
            .filter(|h| h != "x.com" && h != "twitter.com")
            .collect();
        let handle = author["handle"].as_str().unwrap_or_default();
        out.push(json!({
            "id": id, "author_id": author_id, "text": text, "created_at": xts(created),
            "lang": p["lang"].as_str().unwrap_or("und"), "kind": kind,
            "url": format!("https://x.com/{handle}/status/{id}"),
            "references": p["referenced_tweets"], "metrics": p["public_metrics"],
            "conversation_id": p["conversation_id"],
            "hashtags": entity_list(p, "hashtags", "tag"),
            "mentions": entity_list(p, "mentions", "username"),
            "domains": domains,
        }));
    }
    Ok((out, users))
}
async fn upsert_account(db: &SqlitePool, a: &Value) -> Result<()> {
    let id = a["id"].as_str().context("Missing account ID")?;
    let handle = a["handle"].as_str().context("Missing handle")?;
    let now = Utc::now().to_rfc3339();
    let existing = sqlx::query("SELECT handle,previous_handles FROM x_accounts WHERE id=?")
        .bind(id)
        .fetch_optional(db)
        .await?;
    let mut previous: Vec<String> = existing
        .as_ref()
        .and_then(|r| serde_json::from_str(&r.get::<String, _>("previous_handles")).ok())
        .unwrap_or_default();
    // Handle changes are retained as attribution history.
    if let Some(r) = &existing {
        let old: String = r.get("handle");
        if old != handle && !previous.contains(&old) {
            previous.push(old);
        }
    }
    sqlx::query("INSERT INTO x_accounts(id,handle,name,profile,previous_handles,first_seen_at,last_seen_at) VALUES(?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET handle=excluded.handle,name=excluded.name,profile=excluded.profile,previous_handles=excluded.previous_handles,last_seen_at=excluded.last_seen_at")
        .bind(id)
        .bind(handle)
        .bind(a["name"].as_str().unwrap_or(handle))
        .bind(a["profile"].to_string())
        .bind(serde_json::to_string(&previous)?)
        .bind(&now)
        .bind(&now)
        .execute(db)
        .await?;
    Ok(())
}
#[derive(Default, Debug)]
struct Stored {
    matched: usize,
    added: usize,
}
/// Archive posts that match at least one topic. `sampled` is the topic X itself matched.
async fn store_posts(
    db: &SqlitePool,
    posts: &[Value],
    users: &HashMap<String, Value>,
    via: &str,
    sampled: Option<&str>,
) -> Result<Stored> {
    let now = Utc::now().to_rfc3339();
    let mut stored = Stored::default();
    let mut authors = Vec::new();
    for p in posts {
        let text = p["text"].as_str().unwrap_or_default();
        let mut found = match_topics(text);
        if let Some(t) = sampled
            && !found.iter().any(|(id, _)| id == t)
        {
            found.push((t.to_string(), Vec::new()));
        }
        if found.is_empty() {
            continue;
        }
        let author = p["author_id"].as_str().unwrap_or_default();
        if !authors.contains(&author) {
            authors.push(author);
            if let Some(a) = users.get(author) {
                upsert_account(db, a).await?;
            }
        }
        let mut tx = db.begin().await?;
        let inserted = sqlx::query("INSERT OR IGNORE INTO x_posts(id,author_id,created_at,lang,text,data,first_seen_at,last_seen_at) VALUES(?,?,?,?,?,?,?,?)")
            .bind(p["id"].as_str())
            .bind(author)
            .bind(p["created_at"].as_str())
            .bind(p["lang"].as_str())
            .bind(text)
            .bind(p.to_string())
            .bind(&now)
            .bind(&now)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        if inserted == 0 {
            // Refresh engagement metrics on re-observation.
            sqlx::query("UPDATE x_posts SET data=?,last_seen_at=? WHERE id=?")
                .bind(p.to_string())
                .bind(&now)
                .bind(p["id"].as_str())
                .execute(&mut *tx)
                .await?;
        }
        for (topic, terms) in &found {
            sqlx::query("INSERT INTO x_post_matches(post_id,topic,terms,via) VALUES(?,?,?,?) ON CONFLICT(post_id,topic) DO UPDATE SET terms=excluded.terms")
                .bind(p["id"].as_str())
                .bind(topic)
                .bind(serde_json::to_string(terms)?)
                .bind(via)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        stored.matched += 1;
        stored.added += inserted as usize;
    }
    Ok(stored)
}
async fn purge(db: &SqlitePool) -> Result<()> {
    let cutoff = Utc::now() - Duration::days(retention_days());
    sqlx::query("DELETE FROM x_posts WHERE created_at<?")
        .bind(xts(cutoff))
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM x_post_matches WHERE post_id NOT IN (SELECT id FROM x_posts)")
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM x_accounts WHERE id NOT IN (SELECT author_id FROM x_posts) AND coalesce(history_checked_at,last_seen_at)<?")
        .bind(cutoff.to_rfc3339())
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM x_counts WHERE start<?")
        .bind(xts(cutoff))
        .execute(db)
        .await?;
    Ok(())
}

async fn reserve_posts(db: &SqlitePool, id: &str, limit: i64) -> Result<bool> {
    let mut tx = db.begin().await?;
    let now = Utc::now();
    let day = today();
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
    let body = x_get(
        client,
        token,
        "/2/tweets/search/recent",
        &[
            ("query", topic.query.clone()),
            ("max_results", posts_per_topic().to_string()),
            ("sort_order", "recency".into()),
            (
                "start_time",
                (Utc::now() - Duration::hours(24)).to_rfc3339(),
            ),
            ("tweet.fields", TWEET_FIELDS.into()),
            ("expansions", "author_id".into()),
            ("user.fields", USER_FIELDS.into()),
        ],
    )
    .await?;
    let (posts, users) = normalize_posts(&body, posts_per_topic() as usize)?;
    let stored = store_posts(db, &posts, &users, "topic_sample", Some(&topic.id)).await?;
    sqlx::query("UPDATE x_post_topics SET checked_at=?,error=NULL,data=? WHERE id=?")
        .bind(Utc::now().to_rfc3339())
        .bind(json!({"returned":posts.len(),"new":stored.added}).to_string())
        .bind(&topic.id)
        .execute(db)
        .await?;
    Ok(())
}
async fn collect_posts_cycle(db: &SqlitePool, client: &reqwest::Client, token: &str) -> Result<()> {
    for t in topics() {
        if reserve_posts(db, &t.id, post_limit()).await?
            && let Err(e) = fetch_posts(db, client, t, token).await
        {
            defer_on_rate_limit(db, "x_post_topics", &e).await?;
            sqlx::query("UPDATE x_post_topics SET error=? WHERE id=?")
                .bind(e.to_string())
                .bind(&t.id)
                .execute(db)
                .await?;
        }
    }
    Ok(())
}
pub async fn worker(db: SqlitePool) -> Result<()> {
    let client = client()?;
    loop {
        purge(&db).await?;
        if enabled() && configured() {
            let token = std::env::var("X_BEARER_TOKEN")?;
            // Least recently attempted first; prevents starvation at a small daily cap.
            let ids: Vec<String> =
                sqlx::query_scalar("SELECT id FROM x_topics ORDER BY next_at,id")
                    .fetch_all(&db)
                    .await?;
            for id in ids {
                if let Some(topic) = topics().find(|t| t.id == id)
                    && reserve(&db, &id, daily_limit()).await?
                    && let Err(e) = collect(&db, &client, topic, &token).await
                {
                    defer_on_rate_limit(&db, "x_topics", &e).await?;
                    sqlx::query("UPDATE x_topics SET error=? WHERE id=?")
                        .bind(e.to_string())
                        .bind(&id)
                        .execute(&db)
                        .await?;
                }
            }
            collect_posts_cycle(&db, &client, &token).await?;
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

// ---------------------------------------------------------------------------------------------
// Account history (on demand, budgeted)

async fn reserve_account(db: &SqlitePool, handle: &str, kind: &str) -> Result<bool> {
    let day = today();
    // Single statement: the cap check and the reservation are atomic.
    let n = sqlx::query("INSERT INTO x_account_requests(day,handle,kind) SELECT ?,?,? WHERE (SELECT count(*) FROM x_account_requests WHERE day=?)<?")
        .bind(&day)
        .bind(handle)
        .bind(kind)
        .bind(&day)
        .bind(account_limit())
        .execute(db)
        .await?
        .rows_affected();
    Ok(n == 1)
}
fn upstream(e: anyhow::Error) -> ApiError {
    let code = if e.is::<RateLimited>() {
        StatusCode::TOO_MANY_REQUESTS
    } else {
        StatusCode::BAD_GATEWAY
    };
    ApiError(code, e.to_string())
}
fn budget_exhausted() -> ApiError {
    ApiError(
        StatusCode::TOO_MANY_REQUESTS,
        "Daily X account-history budget reached (X_ACCOUNT_REQUEST_LIMIT)".into(),
    )
}
pub async fn history(State(app): State<App>, Path(handle): Path<String>) -> ApiResult<Json<Value>> {
    if !valid_handle(&handle) {
        return Err(ApiError(StatusCode::BAD_REQUEST, "invalid X handle".into()));
    }
    if !(enabled() && configured()) {
        return Err(ApiError(
            StatusCode::CONFLICT,
            "X collection is disabled or not configured on the server".into(),
        ));
    }
    let token = std::env::var("X_BEARER_TOKEN").map_err(anyhow::Error::from)?;
    let client = client()?;
    let db = &app.db;
    let mut requests = 0;
    let known: Option<String> =
        sqlx::query_scalar("SELECT id FROM x_accounts WHERE lower(handle)=lower(?)")
            .bind(&handle)
            .fetch_optional(db)
            .await?;
    let id = match known {
        Some(id) => id,
        None => {
            if !reserve_account(db, &handle, "user").await? {
                return Err(budget_exhausted());
            }
            requests += 1;
            let body = x_get(
                &client,
                &token,
                &format!("/2/users/by/username/{handle}"),
                &[("user.fields", USER_FIELDS.into())],
            )
            .await
            .map_err(upstream)?;
            let user = body
                .get("data")
                .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "X account not found".into()))?;
            let account = normalize_user(user)?;
            upsert_account(db, &account).await?;
            account["id"].as_str().unwrap_or_default().to_string()
        }
    };
    let now = Utc::now();
    let previous: Option<String> =
        sqlx::query_scalar("SELECT history_checked_at FROM x_accounts WHERE id=?")
            .bind(&id)
            .fetch_one(db)
            .await?;
    let claimed = sqlx::query("UPDATE x_accounts SET history_checked_at=? WHERE id=? AND (history_checked_at IS NULL OR history_checked_at<?)")
        .bind(now.to_rfc3339())
        .bind(&id)
        .bind((now - Duration::minutes(HISTORY_COOLDOWN_MINUTES)).to_rfc3339())
        .execute(db)
        .await?
        .rows_affected();
    if claimed == 0 {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            format!("This account was checked in the last {HISTORY_COOLDOWN_MINUTES} minutes"),
        ));
    }
    let since: Option<String> =
        sqlx::query_scalar("SELECT history_newest_id FROM x_accounts WHERE id=?")
            .bind(&id)
            .fetch_one(db)
            .await?;
    let (mut scanned, mut stored, mut newest, mut oldest_scanned) =
        (0, Stored::default(), None, None);
    let mut page_token: Option<String> = None;
    for page in 0..history_pages() {
        if !reserve_account(db, &handle, "timeline").await? {
            if page == 0 {
                sqlx::query("UPDATE x_accounts SET history_checked_at=? WHERE id=?")
                    .bind(previous)
                    .bind(&id)
                    .execute(db)
                    .await?;
                return Err(budget_exhausted());
            }
            break;
        }
        requests += 1;
        let mut query = vec![
            ("max_results", "100".to_string()),
            ("tweet.fields", TWEET_FIELDS.into()),
            ("expansions", "author_id".into()),
            ("user.fields", USER_FIELDS.into()),
        ];
        if let Some(s) = &since {
            query.push(("since_id", s.clone()));
        }
        if let Some(p) = &page_token {
            query.push(("pagination_token", p.clone()));
        }
        let body = match x_get(&client, &token, &format!("/2/users/{id}/tweets"), &query).await {
            Ok(b) => b,
            Err(e) => {
                sqlx::query("UPDATE x_accounts SET history_error=? WHERE id=?")
                    .bind(e.to_string())
                    .bind(&id)
                    .execute(db)
                    .await?;
                return Err(upstream(e));
            }
        };
        if page == 0 {
            newest = body
                .pointer("/meta/newest_id")
                .and_then(Value::as_str)
                .filter(|s| valid_id(s))
                .map(String::from);
        }
        let (posts, users) = normalize_posts(&body, 100)?;
        scanned += posts.len();
        if let Some(last) = posts.last() {
            oldest_scanned = last["created_at"].as_str().map(String::from);
        }
        let s = store_posts(db, &posts, &users, "account_history", None).await?;
        stored.matched += s.matched;
        stored.added += s.added;
        page_token = body
            .pointer("/meta/next_token")
            .and_then(Value::as_str)
            .map(String::from);
        if page_token.is_none() {
            break;
        }
    }
    let result = json!({"handle":handle,"scanned":scanned,"matched":stored.matched,"new":stored.added,"requests":requests,"oldest_scanned":oldest_scanned,"incremental":since.is_some(),"checked_at":now.to_rfc3339()});
    sqlx::query("UPDATE x_accounts SET history_newest_id=coalesce(?,history_newest_id),history_last=?,history_error=NULL WHERE id=?")
        .bind(newest)
        .bind(result.to_string())
        .bind(&id)
        .execute(db)
        .await?;
    Ok(Json(result))
}

// ---------------------------------------------------------------------------------------------
// Read API

fn retention_cutoff() -> String {
    xts(Utc::now() - Duration::days(retention_days()))
}
const POST_SELECT: &str = "SELECT p.data, p.first_seen_at, a.handle, a.name, count(*) OVER () AS total,
  (SELECT json_group_array(json_object('topic',m.topic,'terms',json(m.terms),'via',m.via)) FROM x_post_matches m WHERE m.post_id=p.id) AS topics
  FROM x_posts p JOIN x_accounts a ON a.id=p.author_id
  WHERE p.created_at>=?
   AND (? IS NULL OR EXISTS(SELECT 1 FROM x_post_matches m WHERE m.post_id=p.id AND m.topic=?))
   AND (? IS NULL OR p.lang=?)
   AND (? IS NULL OR instr(lower(p.text || ' ' || a.handle || ' ' || a.name), ?)>0)
   AND (? IS NULL OR p.author_id=?)
   AND (? IS NULL OR p.created_at>=?)
  ORDER BY p.created_at DESC, p.id DESC LIMIT ? OFFSET ?";
#[derive(Deserialize, Default)]
pub struct PostQuery {
    topic: Option<String>,
    lang: Option<String>,
    q: Option<String>,
    since: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}
async fn query_posts(
    db: &SqlitePool,
    f: &PostQuery,
    author: Option<&str>,
    limit: i64,
) -> Result<(Vec<Value>, i64)> {
    let q =
        f.q.as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());
    let since = f
        .since
        .as_ref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| xts(d.with_timezone(&Utc)));
    let rows = sqlx::query(POST_SELECT)
        .bind(retention_cutoff())
        .bind(&f.topic)
        .bind(&f.topic)
        .bind(&f.lang)
        .bind(&f.lang)
        .bind(&q)
        .bind(&q)
        .bind(author)
        .bind(author)
        .bind(&since)
        .bind(&since)
        .bind(limit)
        .bind(f.offset.unwrap_or(0).max(0))
        .fetch_all(db)
        .await?;
    let total = rows.first().map_or(0, |r| r.get::<i64, _>("total"));
    let mut out = Vec::new();
    for r in rows {
        let mut p: Value = serde_json::from_str(&r.get::<String, _>("data"))?;
        let topics: Value = serde_json::from_str(&r.get::<String, _>("topics"))?;
        let handle: String = r.get("handle");
        p["topic"] = topics.pointer("/0/topic").cloned().unwrap_or(Value::Null);
        p["topics"] = topics;
        p["author_handle"] = json!(handle);
        p["author_name"] = json!(r.get::<String, _>("name"));
        p["author_url"] = json!(format!("https://x.com/{handle}"));
        p["collected_at"] = json!(r.get::<String, _>("first_seen_at"));
        out.push(p);
    }
    Ok((out, total))
}
async fn post_budget(db: &SqlitePool) -> Result<(i64, i64)> {
    let day = today();
    let posts: i64 = sqlx::query_scalar("SELECT count(*) FROM x_post_requests WHERE day=?")
        .bind(&day)
        .fetch_one(db)
        .await?;
    let accounts: i64 = sqlx::query_scalar("SELECT count(*) FROM x_account_requests WHERE day=?")
        .bind(&day)
        .fetch_one(db)
        .await?;
    Ok((posts, accounts))
}
pub async fn posts(State(app): State<App>, Query(f): Query<PostQuery>) -> ApiResult<Json<Value>> {
    let limit = f.limit.unwrap_or(200).clamp(1, 500);
    let (items, total) = query_posts(&app.db, &f, None, limit).await?;
    let mut sources = Vec::new();
    for r in sqlx::query("SELECT id,checked_at,error,data FROM x_post_topics ORDER BY id")
        .fetch_all(&app.db)
        .await?
    {
        sources.push(json!({"topic":r.get::<String,_>("id"),"checked_at":r.get::<Option<String>,_>("checked_at"),"error":r.get::<Option<String>,_>("error"),"last_run":r.get::<Option<String>,_>("data").and_then(|d|serde_json::from_str::<Value>(&d).ok())}));
    }
    let (used, account_used) = post_budget(&app.db).await?;
    Ok(Json(
        json!({"items":items,"total":total,"limit":limit,"sources":sources,"requests_today":used,"daily_request_limit":post_limit(),"account_requests_today":account_used,"account_request_limit":account_limit(),"sample_size_per_topic":posts_per_topic(),"poll_seconds":86400,"retention_days":retention_days(),"coverage":"archive of sampled topic posts and analyst-requested account histories that match topic rules; not exhaustive"}),
    ))
}
#[derive(Deserialize)]
pub struct AccountQuery {
    q: Option<String>,
    topic: Option<String>,
    limit: Option<i64>,
}
fn account_json(r: &sqlx::sqlite::SqliteRow) -> Value {
    let parse = |col: &str| {
        r.get::<Option<String>, _>(col)
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            .unwrap_or(Value::Null)
    };
    let handle: String = r.get("handle");
    json!({"id":r.get::<String,_>("id"),"handle":handle,"name":r.get::<String,_>("name"),"url":format!("https://x.com/{handle}"),
        "profile":parse("profile"),"previous_handles":parse("previous_handles"),"first_seen_at":r.get::<String,_>("first_seen_at"),
        "last_seen_at":r.get::<String,_>("last_seen_at"),"history_checked_at":r.get::<Option<String>,_>("history_checked_at"),
        "history_last":parse("history_last"),"history_error":r.get::<Option<String>,_>("history_error")})
}
pub async fn accounts(
    State(app): State<App>,
    Query(f): Query<AccountQuery>,
) -> ApiResult<Json<Value>> {
    let limit = f.limit.unwrap_or(100).clamp(1, 500);
    let q =
        f.q.as_ref()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty());
    let cutoff = retention_cutoff();
    let rows = sqlx::query(
        "SELECT a.*, count(DISTINCT p.id) AS posts, max(p.created_at) AS last_post, min(p.created_at) AS first_post
         FROM x_accounts a JOIN x_posts p ON p.author_id=a.id
         WHERE p.created_at>=? AND (? IS NULL OR instr(lower(a.handle || ' ' || a.name), ?)>0)
          AND (? IS NULL OR EXISTS(SELECT 1 FROM x_post_matches m WHERE m.post_id=p.id AND m.topic=?))
         GROUP BY a.id ORDER BY posts DESC, last_post DESC LIMIT ?",
    )
    .bind(&cutoff)
    .bind(&q)
    .bind(&q)
    .bind(&f.topic)
    .bind(&f.topic)
    .bind(limit)
    .fetch_all(&app.db)
    .await?;
    let mut by_topic: HashMap<String, BTreeMap<String, i64>> = HashMap::new();
    for r in sqlx::query("SELECT p.author_id, m.topic, count(*) AS n FROM x_posts p JOIN x_post_matches m ON m.post_id=p.id WHERE p.created_at>=? GROUP BY p.author_id, m.topic")
        .bind(&cutoff)
        .fetch_all(&app.db)
        .await?
    {
        by_topic
            .entry(r.get("author_id"))
            .or_default()
            .insert(r.get("topic"), r.get("n"));
    }
    let items: Vec<Value> = rows
        .iter()
        .map(|r| {
            let mut a = account_json(r);
            a["posts"] = json!(r.get::<i64, _>("posts"));
            a["first_post"] = json!(r.get::<String, _>("first_post"));
            a["last_post"] = json!(r.get::<String, _>("last_post"));
            a["topic_counts"] = json!(by_topic.get(&r.get::<String, _>("id")));
            a
        })
        .collect();
    let total: i64 =
        sqlx::query_scalar("SELECT count(DISTINCT author_id) FROM x_posts WHERE created_at>=?")
            .bind(&cutoff)
            .fetch_one(&app.db)
            .await?;
    Ok(Json(
        json!({"items":items,"total":total,"retention_days":retention_days()}),
    ))
}
pub async fn account(State(app): State<App>, Path(handle): Path<String>) -> ApiResult<Json<Value>> {
    if !valid_handle(&handle) {
        return Err(ApiError(StatusCode::BAD_REQUEST, "invalid X handle".into()));
    }
    let row = sqlx::query("SELECT * FROM x_accounts WHERE lower(handle)=lower(?)")
        .bind(&handle)
        .fetch_optional(&app.db)
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "account not in the archive".into()))?;
    let account = account_json(&row);
    let id: String = row.get("id");
    let (posts, total) = query_posts(&app.db, &PostQuery::default(), Some(&id), 1000).await?;
    let (_, used) = post_budget(&app.db).await?;
    Ok(Json(json!({
        "account":account,"posts":posts,"total":total,"retention_days":retention_days(),
        "history":{"available":enabled()&&configured(),"requests_today":used,"request_limit":account_limit(),"pages":history_pages(),"cooldown_minutes":HISTORY_COOLDOWN_MINUTES},
        "notice":"Posts shown are those matching the configured topic rules. Matching is keyword-based and does not establish stance, affiliation or coordination."
    })))
}
pub async fn feed(State(app): State<App>) -> ApiResult<Json<Value>> {
    let used: i64 = sqlx::query_scalar("SELECT count(*) FROM x_requests WHERE day=?")
        .bind(today())
        .fetch_one(&app.db)
        .await?;
    let week = xts(Utc::now() - Duration::days(7));
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
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .is_none_or(|d| Utc::now().signed_duration_since(d).num_minutes() > 270);
        let history: Vec<Value> = sqlx::query(
            "SELECT start,tweet_count FROM x_counts WHERE topic=? AND start>=? ORDER BY start",
        )
        .bind(&topic.id)
        .bind(&week)
        .fetch_all(&app.db)
        .await?
        .iter()
        .map(|r| json!({"start":r.get::<String,_>("start"),"count":r.get::<i64,_>("tweet_count")}))
        .collect();
        let archived: i64 = sqlx::query_scalar("SELECT count(*) FROM x_post_matches WHERE topic=?")
            .bind(&topic.id)
            .fetch_one(&app.db)
            .await?;
        let mut search = url::Url::parse("https://x.com/search").unwrap();
        search
            .query_pairs_mut()
            .append_pair("q", &topic.query)
            .append_pair("f", "live");
        items.push(json!({"topic":topic,"data":data.and_then(|d|serde_json::from_str::<Value>(&d).ok()),"checked_at":checked,"stale":stale,"error":row.get::<Option<String>,_>("error"),"search_url":search.as_str(),"history":history,"archived_posts":archived}));
    }
    let archive_posts: i64 = sqlx::query_scalar("SELECT count(*) FROM x_posts")
        .fetch_one(&app.db)
        .await?;
    let archive_accounts: i64 = sqlx::query_scalar("SELECT count(DISTINCT author_id) FROM x_posts")
        .fetch_one(&app.db)
        .await?;
    let (post_used, account_used) = post_budget(&app.db).await?;
    Ok(Json(
        json!({"mode":"aggregate_counts","poll_seconds":14400,"enabled":enabled(),"configured":configured(),"requests_today":used,"daily_request_limit":daily_limit(),"budget_paused":used>=daily_limit(),
        "post_requests_today":post_used,"post_request_limit":post_limit(),"account_requests_today":account_used,"account_request_limit":account_limit(),
        "archive":{"posts":archive_posts,"accounts":archive_accounts,"retention_days":retention_days()},
        "items":items,"generated_at":Utc::now().to_rfc3339(),"notice":"Counts include reposts and overlapping topics. They do not establish independent sources, endorsement or verified events. Accounts are attributed by public handle only; no stance, affiliation or automation labels are inferred."}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn user(id: &str, handle: &str) -> Value {
        json!({"id":id,"name":"Example","username":handle,"public_metrics":{"followers_count":12}})
    }
    fn post(id: &str, author: &str, text: &str, created: &str) -> Value {
        json!({"id":id,"author_id":author,"text":text,"created_at":created,"lang":"fr"})
    }
    #[test]
    fn rules_match_terms_phrases_and_groups() {
        let found = match_topics("Combats signalés près de #Goma ce matin");
        let eastern = found.iter().find(|(t, _)| t == "eastern-drc").unwrap();
        assert!(eastern.1.contains(&"Goma".to_string()));
        assert!(eastern.1.contains(&"combats".to_string()));
        assert!(
            match_topics("Le M 23 avance vers Bukavu")
                .iter()
                .any(|(t, _)| t == "armed-groups")
        );
        assert!(
            match_topics("Appel au cessez-le-feu à Kinshasa")
                .iter()
                .any(|(t, _)| t == "diplomacy")
        );
        assert!(
            match_topics("Goma est une belle ville")
                .iter()
                .all(|(t, _)| t != "eastern-drc")
        );
        let negated = parse_rule("(goma OR bukavu) -sport").unwrap();
        assert!(negated.eval(&tokens("goma news"), &mut Vec::new()));
        assert!(!negated.eval(&tokens("goma sport"), &mut Vec::new()));
        assert!(parse_rule("(goma").is_err());
    }
    #[test]
    fn posts_preserve_attribution_without_labels() {
        let body = json!({"data":[{"id":"123","author_id":"99","text":"Public statement <script>","created_at":"2026-10-18T12:00:00Z","lang":"fr","referenced_tweets":[{"type":"quoted","id":"1"}],"entities":{"hashtags":[{"tag":"RDC"}],"urls":[{"expanded_url":"https://example.org/a"}]}}],"includes":{"users":[user("99","example")]}});
        let (posts, users) = normalize_posts(&body, 10).unwrap();
        assert_eq!(users["99"]["handle"], "example");
        assert_eq!(posts[0]["text"], "Public statement <script>");
        assert_eq!(posts[0]["kind"], "quote");
        assert_eq!(posts[0]["hashtags"], json!(["RDC"]));
        assert_eq!(posts[0]["domains"], json!(["example.org"]));
        assert!(posts[0].get("stance").is_none());
        let mut invalid = body.clone();
        invalid["includes"]["users"][0]["username"] = json!("bad/handle");
        assert!(normalize_posts(&invalid, 10).is_err());
        let mut missing = body.clone();
        missing["includes"] = json!({});
        assert!(normalize_posts(&missing, 10).is_err());
    }
    #[tokio::test]
    async fn archive_keeps_topic_matches_and_drops_expired_posts() {
        let db = crate::store::open("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        let recent = xts(Utc::now() - Duration::hours(1));
        let old = xts(Utc::now() - Duration::days(retention_days() + 1));
        let body = json!({"data":[
            post("1","99","Affrontements à Goma entre FARDC et M23",&recent),
            post("2","99","Bon anniversaire à tous",&recent),
            post("3","99","Attaque à Beni",&old)],
            "includes":{"users":[user("99","example")]}});
        let (posts, users) = normalize_posts(&body, 100).unwrap();
        let stored = store_posts(&db, &posts, &users, "account_history", None)
            .await
            .unwrap();
        assert_eq!(stored.matched, 2);
        purge(&db).await.unwrap();
        let app = App { db, token: None };
        let Json(result) = super::posts(State(app.clone()), Query(PostQuery::default()))
            .await
            .ok()
            .unwrap();
        assert_eq!(result["total"], 1);
        assert_eq!(result["items"][0]["id"], "1");
        assert_eq!(result["items"][0]["author_handle"], "example");
        let Json(dossier) = account(State(app.clone()), Path("EXAMPLE".into()))
            .await
            .ok()
            .unwrap();
        assert_eq!(dossier["posts"].as_array().unwrap().len(), 1);
        let Json(list) = accounts(
            State(app),
            Query(AccountQuery {
                q: None,
                topic: Some("armed-groups".into()),
                limit: None,
            }),
        )
        .await
        .ok()
        .unwrap();
        assert_eq!(list["items"][0]["topic_counts"]["armed-groups"], 1);
    }
    #[tokio::test]
    async fn handle_changes_are_retained() {
        let db = crate::store::open("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        upsert_account(&db, &normalize_user(&user("7", "old_name")).unwrap())
            .await
            .unwrap();
        upsert_account(&db, &normalize_user(&user("7", "new_name")).unwrap())
            .await
            .unwrap();
        let prev: String =
            sqlx::query_scalar("SELECT previous_handles FROM x_accounts WHERE id='7'")
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(prev, "[\"old_name\"]");
    }
    #[tokio::test]
    async fn budgets_are_persistent_and_capped() {
        let db = crate::store::open("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        assert!(reserve_posts(&db, "armed-groups", 1).await.unwrap());
        assert!(!reserve_posts(&db, "armed-groups", 1).await.unwrap());
        assert!(!reserve_posts(&db, "diplomacy", 1).await.unwrap());
        assert!(reserve(&db, "armed-groups", 1).await.unwrap());
        assert!(!reserve(&db, "armed-groups", 1).await.unwrap());
        assert!(!reserve(&db, "diplomacy", 1).await.unwrap());
        for _ in 0..account_limit() {
            assert!(reserve_account(&db, "example", "timeline").await.unwrap());
        }
        assert!(!reserve_account(&db, "example", "timeline").await.unwrap());
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
    #[test]
    fn complete_counts_validate_and_gaps_fail() {
        let end = DateTime::parse_from_rfc3339("2026-10-18T12:00:00Z")
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
        let counts = |data, meta| Counts {
            data: Some(data),
            errors: None,
            meta,
        };
        assert!(validated_counts(counts(buckets.clone(), None), start, end).is_ok());
        let mut invalid = buckets.clone();
        invalid[5] = invalid[4].clone();
        assert!(validated_counts(counts(invalid, None), start, end).is_err());
        assert!(
            validated_counts(
                counts(buckets, Some(json!({"next_token":"more"}))),
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
