//! Watchlists (saved searches) and alerts.
//!
//! A watch belongs to one member. Kinds:
//!   search          any of the listed words or phrases (acronyms such as M23 match in capitals only)
//!   place           any of the listed DRC places (gazetteer match, else the name as a phrase)
//!   actor           any of the listed key figures / organisations (their names and aliases)
//!   sanctioned      any sanctioned person or entity
//!   military_flight a military-labelled aircraft in the flight picture (option region: drc | any)
//!   aircraft        callsign, registration, ICAO hex or type designator
//!   brief           a daily or weekly brief (options period, hour_utc), delivered as a Word file
//! New articles and X posts are matched as they are collected (no backlog flood when a watch is
//! created; the watch's results view shows past matches). Every hit lands in the member's alert
//! inbox; email and Telegram are optional per watch and batched per run.
use crate::text::{self, Doc, Term};
use crate::{ApiError, ApiResult, App, notify};
use anyhow::Result;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{Datelike, Timelike, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

const KINDS: [&str; 7] = [
    "search",
    "place",
    "actor",
    "sanctioned",
    "military_flight",
    "aircraft",
    "brief",
];

pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS watches(id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL, user_id TEXT NOT NULL, name TEXT NOT NULL, kind TEXT NOT NULL, terms TEXT NOT NULL DEFAULT '[]', options TEXT NOT NULL DEFAULT '{}', notify TEXT NOT NULL DEFAULT '{}', active INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
         CREATE INDEX IF NOT EXISTS watches_owner ON watches(workspace_id,user_id);
         CREATE TABLE IF NOT EXISTS alerts(id INTEGER PRIMARY KEY AUTOINCREMENT, watch_id TEXT NOT NULL, workspace_id TEXT NOT NULL, user_id TEXT NOT NULL, kind TEXT NOT NULL, ref TEXT NOT NULL, title TEXT NOT NULL, detail TEXT NOT NULL, created_at TEXT NOT NULL, read_at TEXT, delivery TEXT NOT NULL DEFAULT '{}', UNIQUE(watch_id,kind,ref));
         CREATE INDEX IF NOT EXISTS alerts_owner ON alerts(workspace_id,user_id,created_at);
         CREATE TABLE IF NOT EXISTS watch_state(key TEXT PRIMARY KEY, value TEXT NOT NULL);",
    )
    .execute(db)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Matching

#[derive(Clone)]
pub struct Watch {
    pub id: String,
    pub workspace_id: String,
    pub user_id: String,
    pub name: String,
    pub kind: String,
    pub terms: Vec<String>,
    pub options: Value,
    pub notify: Value,
}
fn row_watch(r: &sqlx::sqlite::SqliteRow) -> Watch {
    Watch {
        id: r.get("id"),
        workspace_id: r.get("workspace_id"),
        user_id: r.get("user_id"),
        name: r.get("name"),
        kind: r.get("kind"),
        terms: serde_json::from_str(&r.get::<String, _>("terms")).unwrap_or_default(),
        options: serde_json::from_str(&r.get::<String, _>("options")).unwrap_or_default(),
        notify: serde_json::from_str(&r.get::<String, _>("notify")).unwrap_or_default(),
    }
}
/// Text rule for a watch: labelled terms, plus gazetteer place names for place watches.
pub struct Rule {
    terms: Vec<(String, Term)>,
    places: Vec<String>,
}
pub type ActorTerms = Vec<(String, String, bool, Vec<Term>)>;
pub fn rule(w: &Watch, actors: &ActorTerms) -> Option<Rule> {
    let mut terms = Vec::new();
    let mut places = Vec::new();
    match w.kind.as_str() {
        "search" => {
            for t in &w.terms {
                let term = text::term(t);
                if !term.tokens.is_empty() {
                    terms.push((t.trim().to_string(), term));
                }
            }
        }
        "place" => {
            for t in &w.terms {
                places.push(text::fold(t.trim()));
                terms.push((
                    t.trim().to_string(),
                    Term {
                        tokens: text::tokens(t),
                        acronym: false,
                    },
                ));
            }
        }
        "actor" => {
            for (id, name, _, ts) in actors {
                if w.terms.iter().any(|t| t == id) {
                    terms.extend(ts.iter().map(|t| (name.clone(), t.clone())));
                }
            }
        }
        "sanctioned" => {
            for (_, name, sanctioned, ts) in actors {
                if *sanctioned {
                    terms.extend(ts.iter().map(|t| (name.clone(), t.clone())));
                }
            }
        }
        _ => return None,
    }
    (!terms.is_empty() || !places.is_empty()).then_some(Rule { terms, places })
}
/// Labels that matched and a snippet around the first match, or None.
pub fn matches(rule: &Rule, doc: &Doc, place_names: &[String]) -> Option<(Vec<String>, String)> {
    let mut labels: Vec<String> = Vec::new();
    let mut first: Option<(usize, usize)> = None;
    for (label, t) in &rule.terms {
        if let Some(&i) = doc.find_term(t).first() {
            if !labels.contains(label) {
                labels.push(label.clone());
            }
            if first.is_none_or(|(j, _)| i < j) {
                first = Some((i, t.tokens.len()));
            }
        }
    }
    for p in place_names {
        let f = text::fold(p);
        if rule.places.contains(&f) && !labels.iter().any(|l| text::fold(l) == f) {
            labels.push(p.clone());
        }
    }
    if labels.is_empty() {
        return None;
    }
    let snippet = first
        .map(|(i, n)| doc.snippet(i, n, 110))
        .unwrap_or_default();
    Some((labels, snippet))
}
fn flight_match(w: &Watch, f: &Value) -> Option<String> {
    match w.kind.as_str() {
        "military_flight" => {
            let any = w.options["region"] == "any";
            (f["category"] == "military" && (any || f["region"] == "drc"))
                .then(|| "military".into())
        }
        "aircraft" => {
            let fields = ["callsign", "registration", "icao24", "aircraft_type"]
                .map(|k| f[k].as_str().unwrap_or("").trim().to_uppercase());
            w.terms.iter().map(|t| t.trim().to_uppercase()).find(|t| {
                !t.is_empty()
                    && fields.iter().any(|v| {
                        !v.is_empty() && (v == t || (t.len() >= 3 && v.starts_with(t.as_str())))
                    })
            })
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------
// Worker

async fn state(db: &SqlitePool, key: &str) -> Result<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM watch_state WHERE key=?")
            .bind(key)
            .fetch_optional(db)
            .await?,
    )
}
async fn set_state(db: &SqlitePool, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO watch_state VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(db)
    .await?;
    Ok(())
}
async fn insert(
    db: &SqlitePool,
    w: &Watch,
    kind: &str,
    r#ref: &str,
    title: &str,
    detail: Value,
) -> Result<Option<i64>> {
    let res = sqlx::query("INSERT OR IGNORE INTO alerts(watch_id,workspace_id,user_id,kind,ref,title,detail,created_at) VALUES(?,?,?,?,?,?,?,?)")
        .bind(&w.id).bind(&w.workspace_id).bind(&w.user_id).bind(kind).bind(r#ref)
        .bind(title.chars().take(300).collect::<String>()).bind(detail.to_string()).bind(Utc::now().to_rfc3339())
        .execute(db).await?;
    Ok((res.rows_affected() == 1).then(|| res.last_insert_rowid()))
}
async fn active(db: &SqlitePool) -> Result<Vec<Watch>> {
    Ok(sqlx::query("SELECT * FROM watches WHERE active=1")
        .fetch_all(db)
        .await?
        .iter()
        .map(row_watch)
        .collect())
}
/// Matches new articles and X posts collected since the cursor. The first run only sets the cursor.
async fn match_new(
    db: &SqlitePool,
    watches: &[Watch],
    fresh: &mut HashMap<String, Vec<i64>>,
) -> Result<()> {
    let text_watches: Vec<&Watch> = watches
        .iter()
        .filter(|w| ["search", "place", "actor", "sanctioned"].contains(&w.kind.as_str()))
        .collect();
    let actors = if text_watches
        .iter()
        .any(|w| w.kind == "actor" || w.kind == "sanctioned")
    {
        crate::actors::watch_terms(db).await?
    } else {
        Vec::new()
    };
    let rules: Vec<(&Watch, Rule)> = text_watches
        .iter()
        .filter_map(|w| rule(w, &actors).map(|r| (*w, r)))
        .collect();
    for (table, key, select) in [
        (
            "articles",
            "cursor:articles",
            "SELECT id, title, content AS body, url, first_seen_at AS seen, json_extract(data,'$.source') AS source, published_at AS at FROM articles WHERE first_seen_at>? ORDER BY first_seen_at LIMIT 300",
        ),
        (
            "x_posts",
            "cursor:x_posts",
            "SELECT id, substr(text,1,140) AS title, text AS body, 'https://x.com/i/web/status/'||id AS url, first_seen_at AS seen, 'X' AS source, created_at AS at FROM x_posts WHERE first_seen_at>? ORDER BY first_seen_at LIMIT 300",
        ),
    ] {
        let Some(cursor) = state(db, key).await? else {
            set_state(db, key, &Utc::now().to_rfc3339()).await?;
            continue;
        };
        let rows = sqlx::query(select).bind(&cursor).fetch_all(db).await?;
        let mut last = cursor.clone();
        for r in &rows {
            let (id, title, body): (String, String, String) =
                (r.get("id"), r.get("title"), r.get("body"));
            last = r.get("seen");
            if rules.is_empty() {
                continue;
            }
            let full = format!("{title}. {body}");
            let doc = Doc::new(&full);
            let places = crate::intelligence::place_names(&full);
            for (w, rule) in &rules {
                if let Some((labels, snippet)) = matches(rule, &doc, &places) {
                    let kind = if table == "articles" {
                        "article"
                    } else {
                        "post"
                    };
                    let detail = json!({"matched":labels,"snippet":snippet,"url":r.get::<String,_>("url"),"source":r.get::<Option<String>,_>("source"),"at":r.get::<Option<String>,_>("at"),"places":places});
                    if let Some(aid) = insert(db, w, kind, &id, &title, detail).await? {
                        fresh.entry(w.id.clone()).or_default().push(aid);
                    }
                }
            }
        }
        set_state(db, key, &last).await?;
    }
    Ok(())
}
async fn match_flights(
    db: &SqlitePool,
    watches: &[Watch],
    fresh: &mut HashMap<String, Vec<i64>>,
) -> Result<()> {
    let flight_watches: Vec<&Watch> = watches
        .iter()
        .filter(|w| w.kind == "military_flight" || w.kind == "aircraft")
        .collect();
    if flight_watches.is_empty() {
        return Ok(());
    }
    let data: String = sqlx::query_scalar("SELECT data FROM flight_snapshot WHERE id=1")
        .fetch_one(db)
        .await?;
    let snap: Value = serde_json::from_str(&data).unwrap_or_default();
    let now = Utc::now();
    // One alert per aircraft per six-hour window, so a loitering aircraft does not repeat every scan.
    let window = format!("{}-{}", now.format("%Y%m%d"), now.hour() / 6);
    for f in snap["items"].as_array().into_iter().flatten() {
        if now.timestamp() - f["observed_at"].as_i64().unwrap_or(0) > 600 {
            continue;
        }
        for w in &flight_watches {
            if let Some(m) = flight_match(w, f) {
                let name = f["callsign"]
                    .as_str()
                    .or(f["registration"].as_str())
                    .unwrap_or(f["icao24"].as_str().unwrap_or("?"))
                    .trim()
                    .to_string();
                let route = f["route"]["origin"]["iata"]
                    .as_str()
                    .zip(f["route"]["destination"]["iata"].as_str())
                    .map(|(a, b)| format!(" {a}→{b}"))
                    .unwrap_or_default();
                let title = format!(
                    "{name} · {}{route}",
                    f["type_name"]
                        .as_str()
                        .or(f["aircraft_type"].as_str())
                        .unwrap_or("aircraft")
                );
                let detail = json!({"matched":[m],"category":f["category"],"basis":f["classification_basis"],"region":f["region"],"altitude_ft":f["altitude_ft"],"lat":f["latitude"],"lon":f["longitude"],"route":f["route"],"icao24":f["icao24"],"url":format!("{}/#flights",notify::public_url())});
                if let Some(aid) = insert(
                    db,
                    w,
                    "flight",
                    &format!("{}:{window}", f["icao24"].as_str().unwrap_or("")),
                    &title,
                    detail,
                )
                .await?
                {
                    fresh.entry(w.id.clone()).or_default().push(aid);
                }
            }
        }
    }
    Ok(())
}
/// Daily briefs at hour_utc; weekly briefs on Mondays at hour_utc.
async fn due_briefs(
    db: &SqlitePool,
    watches: &[Watch],
    fresh: &mut HashMap<String, Vec<i64>>,
) -> Result<()> {
    let now = Utc::now();
    for w in watches.iter().filter(|w| w.kind == "brief") {
        let weekly = w.options["period"] == "weekly";
        let hour = w.options["hour_utc"].as_u64().unwrap_or(6).min(23) as u32;
        if now.hour() < hour || (weekly && now.weekday() != chrono::Weekday::Mon) {
            continue;
        }
        let key = if weekly {
            format!("{}-W{:02}", now.iso_week().year(), now.iso_week().week())
        } else {
            now.format("%Y-%m-%d").to_string()
        };
        let exists: Option<i64> =
            sqlx::query_scalar("SELECT id FROM alerts WHERE watch_id=? AND kind='brief' AND ref=?")
                .bind(&w.id)
                .bind(&key)
                .fetch_optional(db)
                .await?;
        if exists.is_some() {
            continue;
        }
        let period = if weekly { "weekly" } else { "daily" };
        let summary = crate::brief::summary(db, period).await?;
        let detail = json!({"period":period,"summary":summary["headline"],"url":format!("{}/v1/brief?period={period}&format=docx",notify::public_url()),"html":format!("{}/v1/brief?period={period}&format=html",notify::public_url())});
        let title = format!("{} brief · {key}", if weekly { "Weekly" } else { "Daily" });
        if let Some(aid) = insert(db, w, "brief", &key, &title, detail).await? {
            fresh.entry(w.id.clone()).or_default().push(aid);
        }
    }
    Ok(())
}
/// Email and Telegram for this run's new alerts, one batched message per watch.
async fn deliver(
    db: &SqlitePool,
    watches: &[Watch],
    fresh: &HashMap<String, Vec<i64>>,
) -> Result<()> {
    let base = notify::public_url();
    for (wid, ids) in fresh {
        let Some(w) = watches.iter().find(|w| &w.id == wid) else {
            continue;
        };
        let email = w.notify["email"].as_str().filter(|s| !s.trim().is_empty());
        let chat = w.notify["telegram"]
            .as_str()
            .filter(|s| !s.trim().is_empty());
        if email.is_none() && chat.is_none() {
            continue;
        }
        let mut rows = Vec::new();
        for id in ids.iter().take(15) {
            if let Some(r) = sqlx::query("SELECT kind, ref, title, detail FROM alerts WHERE id=?")
                .bind(id)
                .fetch_optional(db)
                .await?
            {
                rows.push((
                    r.get::<String, _>("kind"),
                    r.get::<String, _>("ref"),
                    r.get::<String, _>("title"),
                    serde_json::from_str::<Value>(&r.get::<String, _>("detail"))
                        .unwrap_or_default(),
                ));
            }
        }
        let more = ids.len().saturating_sub(rows.len());
        let link = |kind: &str, r: &str, d: &Value| match kind {
            "article" => format!("{base}/#report/{r}"),
            _ => d["url"].as_str().unwrap_or(&base).to_string(),
        };
        let subject = format!("Congo Signal · {}: {} new", w.name, ids.len());
        let mut text_body = format!("{subject}\n\n");
        let mut html = format!(
            "<h2 style=\"font-family:sans-serif\">{}</h2><ul style=\"font-family:sans-serif\">",
            notify::esc(&subject)
        );
        let mut tg = format!("<b>{}</b>\n", notify::esc(&subject));
        for (kind, r, title, d) in &rows {
            let l = link(kind, r, d);
            let matched = d["matched"]
                .as_array()
                .map(|m| {
                    m.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let source = d["source"].as_str().unwrap_or("");
            text_body.push_str(&format!("- {title} ({source}) [{matched}]\n  {l}\n"));
            html.push_str(&format!(
                "<li><a href=\"{}\">{}</a><br><small>{} · {}</small><br><small>{}</small></li>",
                notify::esc(&l),
                notify::esc(title),
                notify::esc(source),
                notify::esc(&matched),
                notify::esc(d["snippet"].as_str().unwrap_or(""))
            ));
            tg.push_str(&format!(
                "\n• <a href=\"{}\">{}</a>\n<i>{}{}</i>\n",
                notify::esc(&l),
                notify::esc(title),
                notify::esc(source),
                if matched.is_empty() {
                    String::new()
                } else {
                    format!(" · {}", notify::esc(&matched))
                }
            ));
        }
        if more > 0 {
            text_body.push_str(&format!(
                "\n…and {more} more in the alert inbox: {base}/#alerts\n"
            ));
            html.push_str(&format!(
                "<li>…and {more} more in the <a href=\"{base}/#alerts\">alert inbox</a></li>"
            ));
            tg.push_str(&format!("\n…and {more} more: {base}/#alerts"));
        }
        html.push_str("</ul><p style=\"font-family:sans-serif;color:#666;font-size:12px\">Single-source reports are unverified unless stated. Manage this watch in Congo Signal → Watchlists.</p>");
        // Briefs attach the Word file.
        let brief = rows
            .iter()
            .find(|(k, ..)| k == "brief")
            .map(|(_, _, _, d)| d["period"].as_str().unwrap_or("daily").to_string());
        let attachment = match &brief {
            Some(p) => crate::brief::docx(db, p).await.ok(),
            None => None,
        };
        let mut delivery = json!({});
        if let Some(to) = email {
            let r = notify::email(notify::Mail {
                to,
                subject: &subject,
                text: &text_body,
                html: &html,
                attachment: attachment
                    .as_ref()
                    .map(|(n, b)| (n.as_str(), crate::brief::DOCX_MIME, b.clone())),
            })
            .await;
            delivery["email"] = json!(
                r.as_ref()
                    .map(|_| "sent".to_string())
                    .unwrap_or_else(|e| format!("failed: {e}"))
            );
        }
        if let Some(chat) = chat {
            let r = match &attachment {
                Some((name, bytes)) => {
                    notify::telegram_document(
                        chat,
                        name,
                        crate::brief::DOCX_MIME,
                        bytes.clone(),
                        &tg,
                    )
                    .await
                }
                None => notify::telegram(chat, &tg).await,
            };
            delivery["telegram"] = json!(
                r.as_ref()
                    .map(|_| "sent".to_string())
                    .unwrap_or_else(|e| format!("failed: {e}"))
            );
        }
        delivery["at"] = json!(Utc::now().to_rfc3339());
        for id in ids {
            sqlx::query("UPDATE alerts SET delivery=? WHERE id=?")
                .bind(delivery.to_string())
                .bind(id)
                .execute(db)
                .await?;
        }
    }
    Ok(())
}
pub async fn run_once(db: &SqlitePool) -> Result<usize> {
    let watches = active(db).await?;
    let mut fresh = HashMap::new();
    match_new(db, &watches, &mut fresh).await?;
    match_flights(db, &watches, &mut fresh).await?;
    due_briefs(db, &watches, &mut fresh).await?;
    deliver(db, &watches, &fresh).await?;
    Ok(fresh.values().map(Vec::len).sum())
}
pub async fn worker(db: SqlitePool) -> Result<()> {
    loop {
        if let Err(e) = run_once(&db).await {
            tracing::warn!(error = %e, "watchlist run failed");
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

// ---------------------------------------------------------------------------------------------
// API

fn bad(msg: &str) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}
#[derive(Deserialize)]
pub struct WatchInput {
    id: Option<String>,
    name: String,
    kind: String,
    #[serde(default)]
    terms: Vec<String>,
    #[serde(default)]
    options: Value,
    #[serde(default)]
    notify: Value,
    #[serde(default = "yes")]
    active: bool,
}
fn yes() -> bool {
    true
}
pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let me = crate::authentication::actor();
    let rows = sqlx::query("SELECT w.*, (SELECT count(*) FROM alerts a WHERE a.watch_id=w.id) AS hits, (SELECT count(*) FROM alerts a WHERE a.watch_id=w.id AND a.read_at IS NULL) AS unread, (SELECT max(created_at) FROM alerts a WHERE a.watch_id=w.id) AS last_hit FROM watches w WHERE workspace_id=? AND user_id=? ORDER BY created_at DESC")
        .bind(&me.workspace_id).bind(&me.user_id).fetch_all(&app.db).await?;
    let items: Vec<Value> = rows.iter().map(|r| {
        let w = row_watch(r);
        json!({"id":w.id,"name":w.name,"kind":w.kind,"terms":w.terms,"options":w.options,"notify":w.notify,"active":r.get::<i64,_>("active")==1,"created_at":r.get::<String,_>("created_at"),"hits":r.get::<i64,_>("hits"),"unread":r.get::<i64,_>("unread"),"last_hit":r.get::<Option<String>,_>("last_hit")})
    }).collect();
    Ok(Json(
        json!({"items":items,"channels":notify::status(),"email_default":me.email}),
    ))
}
pub async fn save(State(app): State<App>, Json(input): Json<WatchInput>) -> ApiResult<Json<Value>> {
    let me = crate::authentication::actor();
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(bad("Name must be 1–80 characters"));
    }
    if !KINDS.contains(&input.kind.as_str()) {
        return Err(bad("Unknown watch kind"));
    }
    let terms: Vec<String> = input
        .terms
        .iter()
        .map(|t| t.trim().chars().take(80).collect::<String>())
        .filter(|t| !t.is_empty())
        .take(40)
        .collect();
    if ["search", "place", "actor", "aircraft"].contains(&input.kind.as_str()) && terms.is_empty() {
        return Err(bad("Add at least one term"));
    }
    let email = input.notify["email"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let telegram = input.notify["telegram"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if email.is_some_and(|e| !notify::valid_email(e)) {
        return Err(bad("Invalid email address"));
    }
    if telegram.is_some_and(|c| !notify::valid_chat(c)) {
        return Err(bad("Invalid Telegram chat id"));
    }
    let options = json!({
        "region": if input.options["region"] == "any" { "any" } else { "drc" },
        "period": if input.options["period"] == "weekly" { "weekly" } else { "daily" },
        "hour_utc": input.options["hour_utc"].as_u64().unwrap_or(6).min(23),
    });
    let notify_v = json!({"inbox":true,"email":email,"telegram":telegram});
    let now = Utc::now().to_rfc3339();
    let id = match input.id.filter(|s| !s.is_empty()) {
        Some(id) => {
            let r = sqlx::query("UPDATE watches SET name=?,kind=?,terms=?,options=?,notify=?,active=?,updated_at=? WHERE id=? AND workspace_id=? AND user_id=?")
                .bind(name).bind(&input.kind).bind(json!(terms).to_string()).bind(options.to_string()).bind(notify_v.to_string()).bind(input.active).bind(&now)
                .bind(&id).bind(&me.workspace_id).bind(&me.user_id).execute(&app.db).await?;
            if r.rows_affected() == 0 {
                return Err(ApiError(StatusCode::NOT_FOUND, "Watch not found".into()));
            }
            id
        }
        None => {
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM watches WHERE workspace_id=? AND user_id=?",
            )
            .bind(&me.workspace_id)
            .bind(&me.user_id)
            .fetch_one(&app.db)
            .await?;
            if count >= 100 {
                return Err(bad("Watch limit reached (100)"));
            }
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO watches(id,workspace_id,user_id,name,kind,terms,options,notify,active,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?)")
                .bind(&id).bind(&me.workspace_id).bind(&me.user_id).bind(name).bind(&input.kind).bind(json!(terms).to_string()).bind(options.to_string()).bind(notify_v.to_string()).bind(input.active).bind(&now).bind(&now)
                .execute(&app.db).await?;
            id
        }
    };
    Ok(Json(json!({"id":id})))
}
pub async fn remove(State(app): State<App>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let me = crate::authentication::actor();
    let r = sqlx::query("DELETE FROM watches WHERE id=? AND workspace_id=? AND user_id=?")
        .bind(&id)
        .bind(&me.workspace_id)
        .bind(&me.user_id)
        .execute(&app.db)
        .await?;
    sqlx::query("DELETE FROM alerts WHERE watch_id=? AND workspace_id=? AND user_id=?")
        .bind(&id)
        .bind(&me.workspace_id)
        .bind(&me.user_id)
        .execute(&app.db)
        .await?;
    Ok(Json(json!({"deleted":r.rows_affected()})))
}
async fn owned(db: &SqlitePool, id: &str) -> ApiResult<Watch> {
    let me = crate::authentication::actor();
    sqlx::query("SELECT * FROM watches WHERE id=? AND workspace_id=? AND user_id=?")
        .bind(id)
        .bind(&me.workspace_id)
        .bind(&me.user_id)
        .fetch_optional(db)
        .await?
        .map(|r| row_watch(&r))
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "Watch not found".into()))
}
/// Saved-search results: past matches among the latest 1000 articles and X posts, or the current flight picture.
pub async fn results(State(app): State<App>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let w = owned(&app.db, &id).await?;
    let mut items = Vec::new();
    if w.kind == "military_flight" || w.kind == "aircraft" {
        let data: String = sqlx::query_scalar("SELECT data FROM flight_snapshot WHERE id=1")
            .fetch_one(&app.db)
            .await?;
        let snap: Value = serde_json::from_str(&data).unwrap_or_default();
        for f in snap["items"].as_array().into_iter().flatten() {
            if let Some(m) = flight_match(&w, f) {
                items.push(json!({"kind":"flight","ref":f["icao24"],"title":f["callsign"].as_str().or(f["registration"].as_str()).unwrap_or("?"),"matched":[m],"at":f["observed_at"],"detail":f}));
            }
        }
        return Ok(Json(
            json!({"items":items,"scope":"current flight picture"}),
        ));
    }
    let actors = if w.kind == "actor" || w.kind == "sanctioned" {
        crate::actors::watch_terms(&app.db).await?
    } else {
        Vec::new()
    };
    let Some(rule) = rule(&w, &actors) else {
        return Ok(Json(json!({"items":[],"scope":"no matchable terms"})));
    };
    for (kind, select) in [
        (
            "article",
            "SELECT id, title, content AS body, url, json_extract(data,'$.source') AS source, published_at AS at FROM articles ORDER BY published_at DESC LIMIT 1000",
        ),
        (
            "post",
            "SELECT id, substr(text,1,140) AS title, text AS body, 'https://x.com/i/web/status/'||id AS url, 'X' AS source, created_at AS at FROM x_posts ORDER BY created_at DESC LIMIT 1000",
        ),
    ] {
        for r in sqlx::query(select).fetch_all(&app.db).await? {
            let (title, body): (String, String) = (r.get("title"), r.get("body"));
            let full = format!("{title}. {body}");
            let doc = Doc::new(&full);
            let places = crate::intelligence::place_names(&full);
            if let Some((labels, snippet)) = matches(&rule, &doc, &places) {
                items.push(json!({"kind":kind,"ref":r.get::<String,_>("id"),"title":title,"url":r.get::<String,_>("url"),"source":r.get::<Option<String>,_>("source"),"at":r.get::<Option<String>,_>("at"),"matched":labels,"snippet":snippet}));
            }
        }
    }
    items.sort_by(|a, b| b["at"].as_str().cmp(&a["at"].as_str()));
    items.truncate(200);
    Ok(Json(
        json!({"items":items,"scope":"latest 1000 articles and 1000 X posts"}),
    ))
}
#[derive(Deserialize)]
pub struct AlertQuery {
    unread: Option<bool>,
    limit: Option<i64>,
}
pub async fn alerts(State(app): State<App>, Query(q): Query<AlertQuery>) -> ApiResult<Json<Value>> {
    let me = crate::authentication::actor();
    let rows = sqlx::query("SELECT a.*, w.name AS watch_name, w.kind AS watch_kind FROM alerts a LEFT JOIN watches w ON w.id=a.watch_id WHERE a.workspace_id=? AND a.user_id=? AND (?=0 OR a.read_at IS NULL) ORDER BY a.id DESC LIMIT ?")
        .bind(&me.workspace_id).bind(&me.user_id).bind(q.unread.unwrap_or(false)).bind(q.limit.unwrap_or(100).clamp(1, 500)).fetch_all(&app.db).await?;
    let unread: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM alerts WHERE workspace_id=? AND user_id=? AND read_at IS NULL",
    )
    .bind(&me.workspace_id)
    .bind(&me.user_id)
    .fetch_one(&app.db)
    .await?;
    let items: Vec<Value> = rows.iter().map(|r| json!({"id":r.get::<i64,_>("id"),"watch_id":r.get::<String,_>("watch_id"),"watch_name":r.get::<Option<String>,_>("watch_name"),"watch_kind":r.get::<Option<String>,_>("watch_kind"),"kind":r.get::<String,_>("kind"),"ref":r.get::<String,_>("ref"),"title":r.get::<String,_>("title"),"detail":serde_json::from_str::<Value>(&r.get::<String,_>("detail")).unwrap_or_default(),"created_at":r.get::<String,_>("created_at"),"read":r.get::<Option<String>,_>("read_at").is_some(),"delivery":serde_json::from_str::<Value>(&r.get::<String,_>("delivery")).unwrap_or_default()})).collect();
    Ok(Json(json!({"items":items,"unread":unread})))
}
pub async fn mark_read(State(app): State<App>, Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let me = crate::authentication::actor();
    let now = Utc::now().to_rfc3339();
    let n = if body["all"] == true {
        sqlx::query(
            "UPDATE alerts SET read_at=? WHERE workspace_id=? AND user_id=? AND read_at IS NULL",
        )
        .bind(&now)
        .bind(&me.workspace_id)
        .bind(&me.user_id)
        .execute(&app.db)
        .await?
        .rows_affected()
    } else {
        let mut n = 0;
        for id in body["ids"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_i64)
            .take(500)
        {
            n += sqlx::query("UPDATE alerts SET read_at=? WHERE id=? AND workspace_id=? AND user_id=? AND read_at IS NULL").bind(&now).bind(id).bind(&me.workspace_id).bind(&me.user_id).execute(&app.db).await?.rows_affected();
        }
        n
    };
    Ok(Json(json!({"marked":n})))
}
/// Sends a test message on one channel so members can check delivery before relying on it.
pub async fn test(Json(body): Json<Value>) -> ApiResult<Json<Value>> {
    let target = body["target"].as_str().unwrap_or("").trim();
    let r = match body["channel"].as_str() {
        Some("email") => notify::email(notify::Mail { to: target, subject: "Congo Signal · test alert", text: "This is a test alert from Congo Signal. Email delivery works.", html: "<p style=\"font-family:sans-serif\">This is a test alert from <b>Congo Signal</b>. Email delivery works.</p>", attachment: None }).await,
        Some("telegram") => notify::telegram(target, "<b>Congo Signal</b> · test alert\nTelegram delivery works.").await,
        _ => return Err(bad("Unknown channel")),
    };
    r.map_err(|e| ApiError(StatusCode::BAD_GATEWAY, e.to_string()))?;
    Ok(Json(json!({"sent":true})))
}
pub async fn telegram_chats() -> ApiResult<Json<Value>> {
    Ok(Json(notify::telegram_chats().await.map_err(|e| {
        ApiError(StatusCode::BAD_GATEWAY, e.to_string())
    })?))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn w(kind: &str, terms: &[&str]) -> Watch {
        Watch {
            id: "w".into(),
            workspace_id: "ws".into(),
            user_id: "u".into(),
            name: "n".into(),
            kind: kind.into(),
            terms: terms.iter().map(|s| s.to_string()).collect(),
            options: json!({}),
            notify: json!({}),
        }
    }
    #[test]
    fn search_terms_respect_acronyms_and_accents() {
        let r = rule(&w("search", &["M23", "Goma", "cessez-le-feu"]), &Vec::new()).unwrap();
        let hit = |s: &str| matches(&r, &Doc::new(s), &[]).map(|(l, _)| l);
        assert_eq!(hit("Les rebelles du M23 avancent").unwrap(), vec!["M23"]);
        assert!(
            hit("une m23 minuscule").is_none(),
            "acronyms only in capitals"
        );
        assert_eq!(hit("À GOMA, la situation").unwrap(), vec!["Goma"]);
        assert!(hit("Le cessez le feu est violé").is_some());
        assert!(hit("Gomaland").is_none(), "whole words only");
    }
    #[test]
    fn place_and_actor_watches() {
        let p = rule(&w("place", &["Bukavu"]), &Vec::new()).unwrap();
        assert!(matches(&p, &Doc::new("Situation tendue"), &["Bukavu".into()]).is_some());
        let actors: ActorTerms = vec![
            (
                "nangaa".into(),
                "Corneille Nangaa".into(),
                true,
                vec![text::term("Corneille Nangaa")],
            ),
            (
                "other".into(),
                "Other Person".into(),
                false,
                vec![text::term("Other Person")],
            ),
        ];
        let a = rule(&w("actor", &["nangaa"]), &actors).unwrap();
        assert!(matches(&a, &Doc::new("Corneille Nangaa a déclaré"), &[]).is_some());
        let s = rule(&w("sanctioned", &[]), &actors).unwrap();
        assert!(matches(&s, &Doc::new("Other Person était là"), &[]).is_none());
        assert_eq!(
            matches(&s, &Doc::new("selon Corneille Nangaa"), &[])
                .unwrap()
                .0,
            vec!["Corneille Nangaa"]
        );
        assert!(rule(&w("actor", &["missing"]), &actors).is_none());
    }
    #[test]
    fn flight_watches() {
        let mil = json!({"category":"military","region":"approach","callsign":"RRR123","icao24":"43c123"});
        assert!(
            flight_match(&w("military_flight", &[]), &mil).is_none(),
            "DRC only by default"
        );
        let mut any = w("military_flight", &[]);
        any.options = json!({"region":"any"});
        assert!(flight_match(&any, &mil).is_some());
        let f = json!({"callsign":"ETH3922","registration":"ET-APL","icao24":"a7bb53","aircraft_type":"B738"});
        assert!(
            flight_match(&w("aircraft", &["eth"]), &f).is_some(),
            "callsign prefix"
        );
        assert!(flight_match(&w("aircraft", &["et-apl"]), &f).is_some());
        assert!(flight_match(&w("aircraft", &["KQA"]), &f).is_none());
    }
    #[tokio::test]
    async fn first_run_sets_cursor_then_matches_new_articles_once() {
        let path = std::env::temp_dir().join(format!("watch-test-{}.db", uuid::Uuid::new_v4()));
        let db = crate::store::open(&format!("sqlite://{}", path.display()))
            .await
            .unwrap();
        init(&db).await.unwrap();
        sqlx::raw_sql("CREATE TABLE IF NOT EXISTS x_posts(id TEXT PRIMARY KEY, author_id TEXT NOT NULL, created_at TEXT NOT NULL, lang TEXT, text TEXT NOT NULL, data TEXT NOT NULL, first_seen_at TEXT NOT NULL, last_seen_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS flight_snapshot(id INTEGER PRIMARY KEY, data TEXT NOT NULL, next_at INTEGER NOT NULL DEFAULT 0); INSERT INTO flight_snapshot VALUES(1,'{}',0);").execute(&db).await.unwrap();
        sqlx::query("INSERT INTO watches(id,workspace_id,user_id,name,kind,terms,options,notify,active,created_at,updated_at) VALUES('w','ws','u','Goma','search','[\"Goma\"]','{}','{}',1,'t','t')").execute(&db).await.unwrap();
        let add = |id: &str, title: &str, seen: &str| {
            let (id, title, seen) = (id.to_string(), title.to_string(), seen.to_string());
            let db = db.clone();
            async move {
                sqlx::query("INSERT INTO articles(id,url,title,content,first_seen_at,last_seen_at,data) VALUES(?,?,?,?,?,?,'{}')").bind(&id).bind(format!("https://x/{id}")).bind(&title).bind("corps").bind(&seen).bind(&seen).execute(&db).await.unwrap();
            }
        };
        add("old", "Goma hier", "2000-01-01T00:00:00Z").await;
        assert_eq!(
            run_once(&db).await.unwrap(),
            0,
            "first run sets the cursor only"
        );
        add("new", "Combats près de Goma", "2999-01-01T00:00:00Z").await;
        add("other", "Kinshasa", "2999-01-01T00:00:01Z").await;
        assert_eq!(run_once(&db).await.unwrap(), 1);
        assert_eq!(run_once(&db).await.unwrap(), 0, "no duplicates");
    }
}
