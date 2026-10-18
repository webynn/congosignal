//! Mineral market context for the minerals mode of the situation room:
//! - monthly benchmark prices from the World Bank Pink Sheet (CC BY 4.0);
//! - who buys DRC minerals, from UN Comtrade: partner-reported imports from the DRC ("mirror"
//!   data, usually more complete than DRC declarations) alongside DRC-reported exports, with
//!   customs unit values for commodities that have no public benchmark (cobalt, coltan, tungsten…);
//! - a cached Macrostrat lookup describing the bedrock unit at a point (CC BY 4.0).
//!
//! Trade partners are first recorded importers, not end users; transit countries appear as buyers.
use crate::{ApiError, ApiResult, App, reference};
use anyhow::{Context, Result};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
};
use chrono::{Datelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::collections::{BTreeMap, HashMap};
use std::sync::LazyLock;

#[derive(Deserialize, Serialize, Clone)]
struct Hs {
    code: String,
    en: String,
    fr: String,
}
#[derive(Deserialize, Serialize, Clone)]
struct Price {
    series: String,
    unit: String,
    /// Price units per kilogram of metal (to value capacities in kg/t); null when not mass-based.
    per_kg: Option<f64>,
}
#[derive(Deserialize, Serialize, Clone)]
struct Commodity {
    id: String,
    en: String,
    fr: String,
    labels: Vec<String>,
    price: Option<Price>,
    hs: Vec<Hs>,
}
#[derive(Deserialize)]
struct Config {
    commodities: Vec<Commodity>,
}
static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../config/commodities.json")).expect("valid commodities")
});
const PINK_SHEET: &str = "https://thedocs.worldbank.org/en/doc/74e8be41ceb20fa0da750cda2f6b9e4e-0050012026/related/CMO-Historical-Data-Monthly.xlsx";
const PINK_PAGE: &str = "https://www.worldbank.org/en/research/commodity-markets";
const COMTRADE: &str = "https://comtradeapi.un.org/public/v1/preview/C/A/HS";
const COMTRADE_PARTNERS: &str =
    "https://comtradeapi.un.org/files/v1/app/reference/partnerAreas.json";
const DRC: &str = "180";
/// (HS code, year, direction, partner, value USD, net kg).
type FlowRow = (String, i64, String, String, Option<f64>, Option<f64>);

pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql(
        r#"
        CREATE TABLE IF NOT EXISTS commodity_prices (series TEXT NOT NULL, period TEXT NOT NULL, value REAL NOT NULL, PRIMARY KEY(series, period));
        CREATE TABLE IF NOT EXISTS trade_flows (hs TEXT NOT NULL, year INTEGER NOT NULL, direction TEXT NOT NULL, partner TEXT NOT NULL,
          value_usd REAL, net_kg REAL, PRIMARY KEY(hs, year, direction, partner));
        CREATE TABLE IF NOT EXISTS geology_units (key TEXT PRIMARY KEY, data TEXT NOT NULL, fetched_at TEXT NOT NULL);
        "#,
    )
    .execute(db)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// World Bank Pink Sheet

fn cell_text(c: &calamine::Data) -> String {
    c.to_string().trim().to_string()
}
fn cell_number(c: &calamine::Data) -> Option<f64> {
    match c {
        calamine::Data::Float(f) => Some(*f),
        calamine::Data::Int(i) => Some(*i as f64),
        calamine::Data::String(s) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|v: &f64| v.is_finite())
}
/// (series, "YYYY-MM", value) for the configured series from the "Monthly Prices" sheet rows.
fn pink_rows(
    rows: &[Vec<calamine::Data>],
    wanted: &[String],
    since_year: i32,
) -> Result<Vec<(String, String, f64)>> {
    let header = rows
        .iter()
        .position(|r| r.iter().any(|c| cell_text(c) == "Copper"))
        .context("Pink Sheet header row not found")?;
    let cols: Vec<(String, usize)> = wanted
        .iter()
        .filter_map(|w| {
            rows[header]
                .iter()
                .position(|c| cell_text(c) == *w)
                .map(|i| (w.clone(), i))
        })
        .collect();
    anyhow::ensure!(
        cols.len() == wanted.len(),
        "Pink Sheet series missing: {:?}",
        wanted
    );
    let mut out = Vec::new();
    for r in &rows[header + 1..] {
        let label = r.first().map(cell_text).unwrap_or_default();
        let Some((y, m)) = label.split_once('M') else {
            continue;
        };
        let (Ok(year), Ok(month)) = (y.parse::<i32>(), m.parse::<u32>()) else {
            continue;
        };
        if year < since_year || !(1..=12).contains(&month) {
            continue;
        }
        for (series, i) in &cols {
            if let Some(v) = r.get(*i).and_then(cell_number) {
                out.push((series.clone(), format!("{year}-{month:02}"), v));
            }
        }
    }
    anyhow::ensure!(!out.is_empty(), "no Pink Sheet observations parsed");
    Ok(out)
}
async fn sync_prices(db: &SqlitePool, client: &reqwest::Client) -> Result<usize> {
    use calamine::Reader;
    let bytes = reference::get_bytes(client, PINK_SHEET, 20 << 20).await?;
    let mut wb: calamine::Xlsx<_> = calamine::open_workbook_from_rs(std::io::Cursor::new(bytes))?;
    let range = wb.worksheet_range("Monthly Prices")?;
    let rows: Vec<Vec<calamine::Data>> = range.rows().map(|r| r.to_vec()).collect();
    let wanted: Vec<String> = CONFIG
        .commodities
        .iter()
        .filter_map(|c| c.price.as_ref().map(|p| p.series.clone()))
        .collect();
    let obs = pink_rows(&rows, &wanted, Utc::now().year() - 6)?;
    let mut tx = db.begin().await?;
    for (series, period, value) in &obs {
        sqlx::query("INSERT INTO commodity_prices(series,period,value) VALUES(?,?,?) ON CONFLICT(series,period) DO UPDATE SET value=excluded.value")
            .bind(series)
            .bind(period)
            .bind(value)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(obs.len())
}

// ---------------------------------------------------------------------------------------------
// UN Comtrade (public preview: one period per request, ≤500 rows, rate-limited)

async fn comtrade(client: &reqwest::Client, query: &str) -> Result<Vec<Value>> {
    for attempt in 0..3 {
        tokio::time::sleep(std::time::Duration::from_secs(4 + attempt * 6)).await;
        let r = client
            .get(format!("{COMTRADE}?{query}"))
            .send()
            .await
            .context("Comtrade connection failed")?;
        if r.status().as_u16() == 429 {
            continue;
        }
        anyhow::ensure!(
            r.status().is_success(),
            "Comtrade HTTP {}",
            r.status().as_u16()
        );
        let body: Value = r.json().await?;
        if let Some(e) = body["error"].as_str().filter(|e| !e.is_empty()) {
            anyhow::bail!("Comtrade: {e}");
        }
        return Ok(body["data"].as_array().cloned().unwrap_or_default());
    }
    anyhow::bail!("Comtrade rate limit persisted")
}
async fn partner_names(client: &reqwest::Client) -> Result<HashMap<i64, String>> {
    let body = reference::get_json(client, COMTRADE_PARTNERS).await?;
    Ok(body["results"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| Some((r["id"].as_i64()?, r["text"].as_str()?.to_string())))
        .collect())
}
/// Aggregates (World, EU, regional "nes" areas) are excluded so buyers are countries.
/// 490 "Other Asia, nes" is kept: Comtrade uses it for Taiwan.
fn is_aggregate(code: i64) -> bool {
    matches!(
        code,
        0 | 97 | 899 | 838 | 839 | 849 | 527 | 577 | 636 | 637 | 697 | 879
    )
}
async fn sync_trade(db: &SqlitePool, client: &reqwest::Client) -> Result<usize> {
    let names = partner_names(client).await?;
    let codes: Vec<String> = CONFIG
        .commodities
        .iter()
        .flat_map(|c| c.hs.iter().map(|h| h.code.clone()))
        .collect();
    let this_year = Utc::now().year();
    let mut stored = 0;
    let mut tx_rows: Vec<FlowRow> = Vec::new();
    for year in (this_year - 3)..this_year {
        for batch in codes.chunks(8) {
            let cmd = batch.join(",");
            // Who buys: every reporter's imports whose partner is the DRC.
            for r in comtrade(
                client,
                &format!("partnerCode={DRC}&flowCode=M&period={year}&cmdCode={cmd}"),
            )
            .await?
            {
                let code = r["reporterCode"].as_i64().unwrap_or(0);
                if is_aggregate(code) {
                    continue;
                }
                let name = names
                    .get(&code)
                    .cloned()
                    .unwrap_or_else(|| code.to_string());
                tx_rows.push((
                    r["cmdCode"].as_str().unwrap_or("").into(),
                    year as i64,
                    "partner_import".into(),
                    name,
                    r["primaryValue"].as_f64(),
                    r["netWgt"].as_f64(),
                ));
            }
            // What the DRC itself declares exporting, by destination.
            for r in comtrade(
                client,
                &format!("reporterCode={DRC}&flowCode=X&period={year}&cmdCode={cmd}"),
            )
            .await?
            {
                let code = r["partnerCode"].as_i64().unwrap_or(0);
                if is_aggregate(code) {
                    continue;
                }
                let name = names
                    .get(&code)
                    .cloned()
                    .unwrap_or_else(|| code.to_string());
                tx_rows.push((
                    r["cmdCode"].as_str().unwrap_or("").into(),
                    year as i64,
                    "drc_export".into(),
                    name,
                    r["primaryValue"].as_f64(),
                    r["netWgt"].as_f64(),
                ));
            }
        }
    }
    let mut tx = db.begin().await?;
    for (hs, year, direction, partner, value, kg) in &tx_rows {
        sqlx::query("INSERT INTO trade_flows(hs,year,direction,partner,value_usd,net_kg) VALUES(?,?,?,?,?,?) ON CONFLICT(hs,year,direction,partner) DO UPDATE SET value_usd=excluded.value_usd,net_kg=excluded.net_kg")
            .bind(hs)
            .bind(year)
            .bind(direction)
            .bind(partner)
            .bind(value)
            .bind(kg)
            .execute(&mut *tx)
            .await?;
        stored += 1;
    }
    tx.commit().await?;
    Ok(stored)
}

pub async fn worker(db: SqlitePool) -> Result<()> {
    let client = reference::client()?;
    loop {
        if reference::sync_enabled() {
            if reference::due(&db, "commodities:prices", 86400).await? {
                let r = sync_prices(&db, &client).await;
                if let Err(e) = &r {
                    tracing::warn!(error = %e, "Pink Sheet sync failed");
                }
                reference::record(&db, "commodities:prices", &r, json!({"url":PINK_SHEET})).await?;
            }
            if reference::due(&db, "commodities:trade", 7 * 86400).await? {
                let r = sync_trade(&db, &client).await;
                if let Err(e) = &r {
                    tracing::warn!(error = %e, "Comtrade sync failed");
                }
                reference::record(&db, "commodities:trade", &r, json!({"url":COMTRADE})).await?;
            }
        }
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(600)) => {}
            _ = reference::REFRESH.notified() => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// API

struct Flow {
    partner: String,
    value: f64,
    kg: f64,
}
/// Latest year with data per HS code and direction, partners sorted by value.
fn summarize_trade(rows: &[FlowRow], hs: &str, direction: &str) -> Option<(i64, Vec<Flow>)> {
    let year = rows
        .iter()
        .filter(|r| r.0 == hs && r.2 == direction && r.4.unwrap_or(0.0) > 0.0)
        .map(|r| r.1)
        .max()?;
    let mut flows: Vec<Flow> = rows
        .iter()
        .filter(|r| r.0 == hs && r.2 == direction && r.1 == year && r.4.unwrap_or(0.0) > 0.0)
        .map(|r| Flow {
            partner: r.3.clone(),
            value: r.4.unwrap_or(0.0),
            kg: r.5.unwrap_or(0.0),
        })
        .collect();
    flows.sort_by(|a, b| b.value.total_cmp(&a.value));
    Some((year, flows))
}
fn flows_json(year: i64, flows: &[Flow]) -> Value {
    let total: f64 = flows.iter().map(|f| f.value).sum();
    let kg: f64 = flows.iter().map(|f| f.kg).sum();
    json!({
        "year": year, "total_usd": total, "total_kg": kg,
        "unit_value_usd_per_kg": (kg > 0.0).then(|| total / kg),
        "partners": flows.iter().take(12).map(|f| json!({"partner":f.partner,"value_usd":f.value,"net_kg":f.kg,"share":if total > 0.0 { f.value / total } else { 0.0 },"unit_value_usd_per_kg":(f.kg > 0.0).then(|| f.value / f.kg)})).collect::<Vec<_>>(),
    })
}
pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let mut prices: BTreeMap<String, Vec<(String, f64)>> = BTreeMap::new();
    for r in sqlx::query("SELECT series, period, value FROM commodity_prices ORDER BY period")
        .fetch_all(&app.db)
        .await?
    {
        prices
            .entry(r.get("series"))
            .or_default()
            .push((r.get("period"), r.get("value")));
    }
    let trade: Vec<FlowRow> = sqlx::query("SELECT * FROM trade_flows")
        .fetch_all(&app.db)
        .await?
        .iter()
        .map(|r| {
            (
                r.get("hs"),
                r.get("year"),
                r.get("direction"),
                r.get("partner"),
                r.get("value_usd"),
                r.get("net_kg"),
            )
        })
        .collect();
    let items: Vec<Value> = CONFIG
        .commodities
        .iter()
        .map(|c| {
            let price = c.price.as_ref().map(|p| {
                let series = prices.get(&p.series).cloned().unwrap_or_default();
                let latest = series.last().cloned();
                let year_ago = latest.as_ref().and_then(|(period, _)| {
                    let (y, m) = period.split_once('-')?;
                    let target = format!("{}-{m}", y.parse::<i32>().ok()? - 1);
                    series.iter().find(|(p, _)| *p == target).map(|(_, v)| *v)
                });
                json!({"series":p.series,"unit":p.unit,"per_kg":p.per_kg,
                       "latest":latest.as_ref().map(|(p, v)| json!({"period":p,"value":v})),
                       "change_12m":latest.as_ref().zip(year_ago).map(|((_, v), a)| (v - a) / a),
                       "history":series.iter().rev().take(36).rev().map(|(p, v)| json!({"period":p,"value":v})).collect::<Vec<_>>()})
            });
            let hs: Vec<Value> = c
                .hs
                .iter()
                .map(|h| {
                    json!({"code":h.code,"en":h.en,"fr":h.fr,
                           "buyers":summarize_trade(&trade, &h.code, "partner_import").map(|(y, f)| flows_json(y, &f)),
                           "drc_declared":summarize_trade(&trade, &h.code, "drc_export").map(|(y, f)| flows_json(y, &f))})
                })
                .collect();
            json!({"id":c.id,"en":c.en,"fr":c.fr,"labels":c.labels,"price":price,"hs":hs})
        })
        .collect();
    let sync = reference::sync_rows(&app.db, "commodities:").await?;
    Ok(Json(json!({
        "items": items, "sync": sync, "sync_enabled": reference::sync_enabled(),
        "sources": [
            {"name":"World Bank Commodity Price Data (Pink Sheet)","url":PINK_PAGE,"license":"CC BY 4.0"},
            {"name":"UN Comtrade (public preview API)","url":"https://comtradeplus.un.org/","license":"UN Comtrade terms of use; attribution required"},
        ],
        "notice": "Benchmark prices are global reference prices for refined metal, not prices paid at DRC mine sites. Buyers are the first recorded importing countries in customs data (including transit hubs), not end users or companies. Partner-reported imports and DRC-reported exports often disagree; both are shown."
    })))
}

#[derive(Deserialize)]
pub struct Point {
    lat: f64,
    lng: f64,
}
/// Bedrock unit at a point from Macrostrat, cached per ~100 m cell, with its source map.
pub async fn geology_unit(
    State(app): State<App>,
    Query(p): Query<Point>,
) -> ApiResult<Json<Value>> {
    if !(-14.0..=6.0).contains(&p.lat) || !(11.0..=32.0).contains(&p.lng) {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "point outside the DRC region".into(),
        ));
    }
    let key = format!("{:.3},{:.3}", p.lat, p.lng);
    if let Some(cached) =
        sqlx::query_scalar::<_, String>("SELECT data FROM geology_units WHERE key=?")
            .bind(&key)
            .fetch_optional(&app.db)
            .await?
    {
        return Ok(Json(
            serde_json::from_str(&cached).map_err(anyhow::Error::from)?,
        ));
    }
    let client = reference::client()?;
    let url = format!(
        "https://macrostrat.org/api/v2/geologic_units/map?lat={:.4}&lng={:.4}",
        p.lat, p.lng
    );
    let body = reference::get_json(&client, &url)
        .await
        .map_err(|e| ApiError(StatusCode::BAD_GATEWAY, format!("Macrostrat: {e}")))?;
    let units = body
        .pointer("/success/data")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    // Prefer the most detailed map available at this point.
    let rank = |s: &str| {
        ["large", "medium", "small", "tiny"]
            .iter()
            .position(|x| *x == s)
            .unwrap_or(9)
    };
    let unit = units
        .iter()
        .min_by_key(|u| rank(u["scale"].as_str().unwrap_or("")))
        .cloned();
    let source = match unit.as_ref().and_then(|u| u["source_id"].as_i64()) {
        Some(id) => reference::get_json(
            &client,
            &format!("https://macrostrat.org/api/v2/defs/sources?source_id={id}"),
        )
        .await
        .ok()
        .and_then(|b| b.pointer("/success/data/0").cloned()),
        None => None,
    };
    let data = json!({
        "found": unit.is_some(),
        "name": unit.as_ref().and_then(|u| u["name"].as_str()).filter(|s| !s.is_empty()),
        "strat_name": unit.as_ref().and_then(|u| u["strat_name"].as_str()).filter(|s| !s.is_empty()),
        "lith": unit.as_ref().and_then(|u| u["lith"].as_str()),
        "description": unit.as_ref().and_then(|u| u["descrip"].as_str()).filter(|s| !s.is_empty()),
        "age": unit.as_ref().map(|u| json!({"top":u["t_int_name"],"bottom":u["b_int_name"],"top_ma":u["t_age"],"bottom_ma":u["b_age"],"best":u["best_int_name"]})),
        "color": unit.as_ref().and_then(|u| u["color"].as_str()),
        "scale": unit.as_ref().and_then(|u| u["scale"].as_str()),
        "source": source.map(|s| json!({"name":s["name"],"title":s["ref_title"],"authors":s["authors"],"year":s["ref_year"],"publisher":s["ref_source"],"doi":s["isbn_doi"],"url":s["url"]})),
        "attribution": "Macrostrat (CC BY 4.0) and the cited source map",
    });
    sqlx::query("INSERT OR REPLACE INTO geology_units(key,data,fetched_at) VALUES(?,?,?)")
        .bind(&key)
        .bind(data.to_string())
        .bind(Utc::now().to_rfc3339())
        .execute(&app.db)
        .await?;
    Ok(Json(data))
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::Data;
    #[test]
    fn pink_sheet_rows_parse_by_header() {
        let s = |v: &str| Data::String(v.into());
        let rows = vec![
            vec![s("World Bank Commodity Price Data")],
            vec![s(""), s("Aluminum"), s("Copper"), s("Gold")],
            vec![s(""), s("($/mt)"), s("($/mt)"), s("($/troy oz)")],
            vec![
                s("2019M12"),
                Data::Float(1770.0),
                Data::Float(6000.0),
                Data::Float(1480.0),
            ],
            vec![
                s("2026M08"),
                Data::Float(2900.0),
                Data::Float(14100.0),
                s("…"),
            ],
            vec![
                s("2026M09"),
                Data::Float(3283.0),
                Data::Float(14474.0),
                Data::Float(4319.0),
            ],
        ];
        let obs = pink_rows(&rows, &["Copper".into(), "Gold".into()], 2020).unwrap();
        assert_eq!(
            obs,
            vec![
                ("Copper".into(), "2026-08".into(), 14100.0),
                ("Copper".into(), "2026-09".into(), 14474.0),
                ("Gold".into(), "2026-09".into(), 4319.0)
            ]
        );
        assert!(pink_rows(&rows, &["Cobalt".into()], 2020).is_err());
    }
    #[test]
    fn trade_uses_latest_year_and_unit_values() {
        let row = |y: i64, p: &str, v: f64, kg: f64| {
            (
                "7403".to_string(),
                y,
                "partner_import".to_string(),
                p.to_string(),
                Some(v),
                Some(kg),
            )
        };
        let rows = vec![
            row(2023, "China", 9.0e9, 1.0e9),
            row(2024, "China", 14.0e9, 1.48e9),
            row(2024, "Rep. of Korea", 0.94e9, 0.1e9),
        ];
        let (year, flows) = summarize_trade(&rows, "7403", "partner_import").unwrap();
        assert_eq!(year, 2024);
        assert_eq!(flows[0].partner, "China");
        let j = flows_json(year, &flows);
        assert!((j["partners"][0]["share"].as_f64().unwrap() - 14.0 / 14.94).abs() < 1e-9);
        assert!(summarize_trade(&rows, "2605", "partner_import").is_none());
        assert!(is_aggregate(0) && is_aggregate(97) && !is_aggregate(156));
    }
}
