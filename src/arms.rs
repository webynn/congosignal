//! Arms & equipment: weapon types tracked in collected reporting (config/equipment.json), with
//! Wikidata facts (origin, manufacturer, operators), a representative Sketchfab 3D model,
//! cited mentions, actors co-mentioned in the same sentence, and SIPRI arms-transfer records
//! imported from analyst-downloaded CSV exports. A co-mention is not attribution.
use crate::reference::{self, CorpusDoc};
use crate::text::{self, Term};
use crate::{ApiError, ApiResult, App};
use anyhow::{Context, Result};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

#[derive(Deserialize, Serialize, Clone)]
struct Category {
    id: String,
    en: String,
    fr: String,
}
#[derive(Deserialize, Serialize, Clone)]
struct Item {
    id: String,
    name: String,
    category: String,
    enwiki: Option<String>,
    model_query: Option<String>,
    terms: Vec<String>,
}
#[derive(Deserialize)]
struct Config {
    categories: Vec<Category>,
    items: Vec<Item>,
}
static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../config/equipment.json")).expect("valid equipment config")
});
fn sipri_dir() -> std::path::PathBuf {
    std::env::var("SIPRI_DIR")
        .unwrap_or_else(|_| "data/sipri".into())
        .into()
}

pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql(
        r#"
        CREATE TABLE IF NOT EXISTS equipment_reference (id TEXT PRIMARY KEY, checked_at TEXT NOT NULL, wikidata TEXT, model TEXT, error TEXT);
        CREATE TABLE IF NOT EXISTS equipment_mentions (equipment_id TEXT NOT NULL, kind TEXT NOT NULL, ref TEXT NOT NULL, page INTEGER NOT NULL DEFAULT -1,
          title TEXT, url TEXT, at TEXT, snippet TEXT, hits INTEGER NOT NULL, groups TEXT NOT NULL, PRIMARY KEY(equipment_id, kind, ref, page));
        CREATE TABLE IF NOT EXISTS sipri_transfers (id INTEGER PRIMARY KEY AUTOINCREMENT, file TEXT NOT NULL, data TEXT NOT NULL);
        "#,
    )
    .execute(db)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Reference data

async fn sketchfab(client: &reqwest::Client, query: &str) -> Result<Option<Value>> {
    let url = format!(
        "https://api.sketchfab.com/v3/search?type=models&count=8&q={}",
        reference::urlencode(query)
    );
    let body = reference::get_json(client, &url).await?;
    Ok(body["results"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|m| m["isAgeRestricted"] != true && m["uid"].is_string())
        .map(|m| {
            let thumb = m
                .pointer("/thumbnails/images")
                .and_then(Value::as_array)
                .and_then(|imgs| {
                    imgs.iter()
                        .filter(|i| i["width"].as_i64().is_some_and(|w| w <= 720))
                        .max_by_key(|i| i["width"].as_i64())
                })
                .and_then(|i| i["url"].as_str());
            json!({"uid":m["uid"],"name":m["name"],"author":m.pointer("/user/displayName"),"author_url":m.pointer("/user/profileUrl"),
                   "viewer_url":m["viewerUrl"],"embed_url":m["embedUrl"],"license":m.pointer("/license/label"),"thumb":thumb,"query":query})
        }))
}
async fn enrich(db: &SqlitePool, client: &reqwest::Client) -> Result<()> {
    for item in &CONFIG.items {
        let fresh: Option<String> = sqlx::query_scalar("SELECT checked_at FROM equipment_reference WHERE id=? AND (error IS NULL AND checked_at>=? OR checked_at>=?)")
            .bind(&item.id)
            .bind((Utc::now() - chrono::Duration::days(7)).to_rfc3339())
            .bind((Utc::now() - chrono::Duration::days(1)).to_rfc3339())
            .fetch_optional(db)
            .await?;
        if fresh.is_some() {
            continue;
        }
        let mut errors: Vec<String> = Vec::new();
        let wikidata = match &item.enwiki {
            Some(title) => match reference::entity_by_title(client, title).await {
                Ok(Some(e)) => reference::summarize(client, &e, "curated_title")
                    .await
                    .map_err(|e| errors.push(e.to_string()))
                    .ok(),
                Ok(None) => None,
                Err(e) => {
                    errors.push(e.to_string());
                    None
                }
            },
            None => None,
        };
        let model = match &item.model_query {
            Some(q) => sketchfab(client, q)
                .await
                .map_err(|e| errors.push(format!("Sketchfab: {e}")))
                .ok()
                .flatten(),
            None => None,
        };
        if errors.iter().any(|e| e.contains("HTTP 429")) {
            tracing::warn!("reference source throttled equipment enrichment; resuming next cycle");
            return Ok(());
        }
        sqlx::query("INSERT INTO equipment_reference(id,checked_at,wikidata,model,error) VALUES(?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET checked_at=excluded.checked_at,wikidata=coalesce(excluded.wikidata,wikidata),model=coalesce(excluded.model,model),error=excluded.error")
            .bind(&item.id)
            .bind(Utc::now().to_rfc3339())
            .bind(wikidata.map(|w| w.to_string()))
            .bind(model.map(|m| m.to_string()))
            .bind((!errors.is_empty()).then(|| errors.join("; ")))
            .execute(db)
            .await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// SIPRI trade-register CSV import

const SIPRI_COLUMNS: &[(&str, &[&str])] = &[
    ("recipient", &["recipient"]),
    ("supplier", &["supplier"]),
    (
        "order_year",
        &["year of order", "order date", "year(s) of order"],
    ),
    ("ordered", &["number ordered", "no. ordered"]),
    ("designation", &["weapon designation", "designation"]),
    ("description", &["weapon description", "description"]),
    ("delivered", &["number delivered", "no. delivered"]),
    (
        "delivery_years",
        &["year(s) of deliver", "years of deliver", "delivery year"],
    ),
    ("status", &["status"]),
    ("comments", &["comments"]),
    ("tiv_unit", &["tiv per unit"]),
    ("tiv_total", &["tiv for total order"]),
    ("tiv_delivered", &["tiv of delivered"]),
];
/// Parse a SIPRI trade-register CSV export; the header row is located by its column names.
fn parse_sipri(raw: &str) -> Result<Vec<Value>> {
    let lines: Vec<&str> = raw.lines().collect();
    let header = lines
        .iter()
        .position(|l| {
            let l = l.to_lowercase();
            l.contains("recipient") && l.contains("supplier")
        })
        .context("no header row with Recipient and Supplier columns")?;
    let body = lines[header..].join("\n");
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(body.as_bytes());
    let headers: Vec<String> = rdr
        .headers()?
        .iter()
        .map(|h| h.trim().to_lowercase())
        .collect();
    let mut map: Vec<(&str, usize)> = Vec::new();
    for (key, names) in SIPRI_COLUMNS {
        if let Some(i) = headers
            .iter()
            .position(|h| names.iter().any(|n| h.contains(n)))
        {
            map.push((key, i));
        }
    }
    anyhow::ensure!(
        map.iter().any(|(k, _)| *k == "supplier") && map.iter().any(|(k, _)| *k == "recipient"),
        "Supplier/Recipient columns not found"
    );
    let mut out = Vec::new();
    for row in rdr.records() {
        let row = row?;
        let mut v = serde_json::Map::new();
        for (key, i) in &map {
            let cell = row.get(*i).unwrap_or("").trim();
            if !cell.is_empty() {
                v.insert(key.to_string(), json!(cell));
            }
        }
        if v.contains_key("supplier") && v.contains_key("recipient") {
            out.push(Value::Object(v));
        }
    }
    Ok(out)
}
fn sipri_signature() -> Vec<(String, u64, i64)> {
    let mut files: Vec<(String, u64, i64)> = std::fs::read_dir(sipri_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("csv"))
        })
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            let modified = m
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs() as i64;
            Some((
                e.file_name().to_string_lossy().into_owned(),
                m.len(),
                modified,
            ))
        })
        .collect();
    files.sort();
    files
}
async fn import_sipri(db: &SqlitePool) -> Result<()> {
    let signature = json!(sipri_signature());
    let previous: Option<String> =
        sqlx::query_scalar("SELECT detail FROM reference_sync WHERE id='equipment:sipri'")
            .fetch_optional(db)
            .await?
            .flatten();
    if previous
        .and_then(|p| serde_json::from_str::<Value>(&p).ok())
        .as_ref()
        == Some(&json!({"files":signature}))
    {
        return Ok(());
    }
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    for (name, _, _) in sipri_signature() {
        let bytes = std::fs::read(sipri_dir().join(&name))?;
        match parse_sipri(&String::from_utf8_lossy(&bytes)) {
            Ok(r) => rows.extend(r.into_iter().map(|v| (name.clone(), v))),
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM sipri_transfers")
        .execute(&mut *tx)
        .await?;
    for (file, v) in &rows {
        sqlx::query("INSERT INTO sipri_transfers(file,data) VALUES(?,?)")
            .bind(file)
            .bind(v.to_string())
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    let result = if errors.is_empty() {
        Ok(rows.len())
    } else {
        Err(anyhow::anyhow!(errors.join("; ")))
    };
    reference::record(db, "equipment:sipri", &result, json!({"files":signature})).await
}

// ---------------------------------------------------------------------------------------------
// Mentions

struct GroupTerms {
    id: String,
    terms: Vec<Term>,
}
fn group_terms() -> Vec<GroupTerms> {
    crate::actors::groups()
        .iter()
        .filter(|g| !g.terms.is_empty())
        .map(|g| GroupTerms {
            id: g.id.clone(),
            terms: g.terms.iter().map(|t| text::term(t)).collect(),
        })
        .collect()
}
async fn scan_mentions(db: &SqlitePool, corpus: &[CorpusDoc]) -> Result<usize> {
    let groups = group_terms();
    let mut rows = Vec::new();
    for item in &CONFIG.items {
        let terms: Vec<Term> = item.terms.iter().map(|t| text::term(t)).collect();
        for c in corpus {
            let mut hits: Vec<(usize, usize)> = terms
                .iter()
                .flat_map(|t| c.doc.find_term(t).into_iter().map(|i| (i, t.tokens.len())))
                .collect();
            if hits.is_empty() {
                continue;
            }
            hits.sort();
            hits.dedup_by_key(|h| h.0);
            let mut co: BTreeMap<String, usize> = BTreeMap::new();
            for &(i, _) in hits.iter().take(25) {
                let sentence = c.doc.span_doc(c.doc.sentence(i));
                for g in &groups {
                    if g.terms.iter().any(|t| sentence.has_term(t)) {
                        *co.entry(g.id.clone()).or_default() += 1;
                    }
                }
            }
            let (i, len) = hits[0];
            rows.push((
                item.id.as_str(),
                c,
                c.doc.snippet(i, len, 180),
                hits.len(),
                co,
            ));
        }
    }
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM equipment_mentions")
        .execute(&mut *tx)
        .await?;
    for (id, c, snippet, hits, co) in &rows {
        sqlx::query("INSERT OR REPLACE INTO equipment_mentions(equipment_id,kind,ref,page,title,url,at,snippet,hits,groups) VALUES(?,?,?,?,?,?,?,?,?,?)")
            .bind(id)
            .bind(c.kind)
            .bind(&c.id)
            .bind(c.page.unwrap_or(-1))
            .bind(&c.title)
            .bind(&c.url)
            .bind(&c.at)
            .bind(snippet)
            .bind(*hits as i64)
            .bind(serde_json::to_string(co)?)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(rows.len())
}
pub async fn worker(db: SqlitePool) -> Result<()> {
    let client = reference::client()?;
    let mut last_scan: Option<std::time::Instant> = None;
    loop {
        if reference::sync_enabled()
            && let Err(e) = enrich(&db, &client).await
        {
            tracing::warn!(error = %e, "equipment enrichment failed");
        }
        import_sipri(&db).await?;
        if last_scan.is_none_or(|t| t.elapsed().as_secs() >= 1800) {
            let corpus = reference::corpus(&db).await?;
            let result = scan_mentions(&db, &corpus).await;
            reference::record(&db, "mentions:equipment", &result, json!({})).await?;
            last_scan = Some(std::time::Instant::now());
        }
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
            _ = reference::REFRESH.notified() => { last_scan = None; }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// API

/// Mention counts by source kind, and co-mention counts by group.
type MentionStats = (serde_json::Map<String, Value>, BTreeMap<String, i64>);

async fn references(
    db: &SqlitePool,
) -> Result<HashMap<String, (Option<Value>, Option<Value>, Option<String>)>> {
    Ok(sqlx::query("SELECT * FROM equipment_reference")
        .fetch_all(db)
        .await?
        .iter()
        .map(|r| {
            let parse = |c: &str| {
                r.get::<Option<String>, _>(c)
                    .and_then(|s| serde_json::from_str::<Value>(&s).ok())
            };
            (
                r.get::<String, _>("id"),
                (
                    parse("wikidata"),
                    parse("model"),
                    r.get::<Option<String>, _>("error"),
                ),
            )
        })
        .collect())
}
async fn transfers(db: &SqlitePool) -> Result<Vec<Value>> {
    let rows: Vec<String> = sqlx::query_scalar("SELECT data FROM sipri_transfers ORDER BY id")
        .fetch_all(db)
        .await?;
    Ok(rows
        .iter()
        .filter_map(|r| serde_json::from_str(r).ok())
        .collect())
}
fn tiv(v: &Value) -> f64 {
    ["tiv_delivered", "tiv_total"]
        .iter()
        .find_map(|k| {
            v[*k]
                .as_str()
                .and_then(|s| s.replace(',', "").parse::<f64>().ok())
        })
        .unwrap_or(0.0)
}
fn matches_item(item: &Item, row: &Value) -> bool {
    let d = text::Doc::new(&format!(
        "{} {}",
        row["designation"].as_str().unwrap_or(""),
        row["description"].as_str().unwrap_or("")
    ));
    item.terms.iter().any(|t| d.has_term(&text::term(t)))
}
pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let refs = references(&app.db).await?;
    let mut stats: HashMap<String, MentionStats> = HashMap::new();
    for r in sqlx::query("SELECT equipment_id, kind, groups, at FROM equipment_mentions")
        .fetch_all(&app.db)
        .await?
    {
        let (counts, groups) = stats.entry(r.get("equipment_id")).or_default();
        let kind: String = r.get("kind");
        let n = counts.get(&kind).and_then(Value::as_i64).unwrap_or(0) + 1;
        counts.insert(kind, json!(n));
        let total = counts.get("total").and_then(Value::as_i64).unwrap_or(0) + 1;
        counts.insert("total".into(), json!(total));
        let at: Option<String> = r.get("at");
        if at.as_deref() > counts.get("latest").and_then(Value::as_str) {
            counts.insert("latest".into(), json!(at));
        }
        let co: BTreeMap<String, i64> =
            serde_json::from_str(&r.get::<String, _>("groups")).unwrap_or_default();
        for (g, n) in co {
            *groups.entry(g).or_default() += n;
        }
    }
    let rows = transfers(&app.db).await?;
    let items: Vec<Value> = CONFIG
        .items
        .iter()
        .map(|item| {
            let (wd, model, error) = refs.get(&item.id).cloned().unwrap_or_default();
            let (counts, groups) = stats.get(&item.id).cloned().unwrap_or_default();
            json!({
                "id": item.id, "name": item.name, "category": item.category, "terms": item.terms,
                "description": wd.as_ref().map(|w| w["description"].clone()),
                "image": wd.as_ref().map(|w| w["image"].clone()).filter(|i| !i.is_null()),
                "origin": wd.as_ref().map(|w| w["origin"].clone()),
                "operators": wd.as_ref().map(|w| w["operators"].clone()),
                "model": model, "reference_error": error,
                "mentions": counts, "groups": groups,
                "transfers": rows.iter().filter(|r| matches_item(item, r)).count(),
            })
        })
        .collect();
    let mut by_recipient: BTreeMap<String, BTreeMap<String, (i64, f64)>> = BTreeMap::new();
    for r in &rows {
        let e = by_recipient
            .entry(r["recipient"].as_str().unwrap_or("?").into())
            .or_default()
            .entry(r["supplier"].as_str().unwrap_or("?").into())
            .or_default();
        e.0 += 1;
        e.1 += tiv(r);
    }
    let flows: Vec<Value> = by_recipient
        .iter()
        .map(|(recipient, suppliers)| {
            let mut s: Vec<Value> = suppliers
                .iter()
                .map(|(name, (n, t))| json!({"supplier":name,"orders":n,"tiv":t}))
                .collect();
            s.sort_by(|a, b| {
                b["tiv"]
                    .as_f64()
                    .partial_cmp(&a["tiv"].as_f64())
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(b["orders"].as_i64().cmp(&a["orders"].as_i64()))
            });
            json!({"recipient":recipient,"suppliers":s})
        })
        .collect();
    let sync = reference::sync_rows(&app.db, "equipment:").await?;
    Ok(Json(json!({
        "categories": CONFIG.categories, "items": items, "groups": crate::actors::groups(),
        "transfers": {"rows": rows.len(), "flows": flows, "records": rows.iter().take(1000).collect::<Vec<_>>(),
                      "directory": sipri_dir().to_string_lossy(), "sync": sync},
        "sync_enabled": reference::sync_enabled(),
        "notice": "Mentions are text matches in collected sources. An actor named in the same sentence is a co-mention, not proof of possession, use or supply. 3D models are representative of the equipment type, not the specific items documented."
    })))
}
pub async fn detail(State(app): State<App>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let item = CONFIG
        .items
        .iter()
        .find(|i| i.id == id)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "equipment not found".into()))?;
    let (wd, model, error) = references(&app.db)
        .await?
        .remove(&item.id)
        .unwrap_or_default();
    let mentions: Vec<Value> = sqlx::query("SELECT * FROM equipment_mentions WHERE equipment_id=? ORDER BY at DESC LIMIT 400")
        .bind(&item.id)
        .fetch_all(&app.db)
        .await?
        .iter()
        .map(|r| json!({"kind":r.get::<String,_>("kind"),"ref":r.get::<String,_>("ref"),"page":Some(r.get::<i64,_>("page")).filter(|p|*p>=0),"title":r.get::<Option<String>,_>("title"),"url":r.get::<Option<String>,_>("url"),"at":r.get::<Option<String>,_>("at"),"snippet":r.get::<Option<String>,_>("snippet"),"hits":r.get::<i64,_>("hits"),"groups":serde_json::from_str::<Value>(&r.get::<String,_>("groups")).unwrap_or_default()}))
        .collect();
    let related: Vec<Value> = transfers(&app.db)
        .await?
        .into_iter()
        .filter(|r| matches_item(item, r))
        .collect();
    Ok(Json(json!({
        "item": item, "category": CONFIG.categories.iter().find(|c| c.id == item.category),
        "wikidata": wd, "model": model, "reference_error": error,
        "mentions": mentions, "transfers": related,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sipri_export_parses_by_header_names() {
        let raw = "SIPRI Trade Register\nTransfers of major weapons\n\nRecipient,Supplier,Year of order,,Number ordered,,Weapon designation,Weapon description,Number delivered,,Year(s) of delivery,status,Comments,SIPRI TIV per unit,SIPRI TIV for total order,SIPRI TIV of delivered weapons\n\"DR Congo\",China,2023,,9,,CH-4,armed UAV,9,,2023; 2024,,,1.5,13.5,13.5\n";
        let rows = parse_sipri(raw).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["supplier"], "China");
        assert_eq!(rows[0]["designation"], "CH-4");
        assert_eq!(tiv(&rows[0]), 13.5);
        let ch4 = CONFIG.items.iter().find(|i| i.id == "ch-4").unwrap();
        assert!(matches_item(ch4, &rows[0]));
        assert!(parse_sipri("a,b\n1,2").is_err());
    }
    #[test]
    fn equipment_terms_tokenize() {
        for item in &CONFIG.items {
            assert!(
                item.terms.iter().all(|t| !text::tokens(t).is_empty()),
                "{}",
                item.id
            );
        }
        let d = text::Doc::new(
            "Des drones kamikazes et un obus de mortier de 120 mm ont visé des positions FARDC.",
        );
        let loiter = CONFIG
            .items
            .iter()
            .find(|i| i.id == "loitering-munitions")
            .unwrap();
        assert!(loiter.terms.iter().any(|t| d.has_term(&text::term(t))));
    }
}
