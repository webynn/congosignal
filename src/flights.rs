//! Shared, bounded civilian ADS-B snapshots. No operational military tracking or flight history.
use crate::{ApiResult, App};
use anyhow::{Result, ensure};
use axum::{Json, extract::State};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::{collections::BTreeMap, sync::LazyLock, time::Duration};
const SOURCE: &str = "https://api.adsb.lol";
static BOUNDARY: LazyLock<Value> =
    LazyLock::new(|| serde_json::from_str(include_str!("../dashboard/drc.geo.json")).unwrap());
fn enabled() -> bool {
    std::env::var("FLIGHTS_ENABLED").as_deref() != Ok("0")
}
fn poll() -> u64 {
    std::env::var("FLIGHTS_POLL_SECONDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300)
        .max(300)
}
fn cap() -> i64 {
    std::env::var("FLIGHTS_DAILY_REQUEST_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000)
        .clamp(0, 10000)
}
pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS flight_snapshot(id INTEGER PRIMARY KEY CHECK(id=1),data TEXT NOT NULL,next_at INTEGER NOT NULL DEFAULT 0);INSERT OR IGNORE INTO flight_snapshot(id,data) VALUES(1,'{}');CREATE TABLE IF NOT EXISTS flight_requests(day TEXT PRIMARY KEY,n INTEGER NOT NULL);CREATE TABLE IF NOT EXISTS flight_metadata(key TEXT PRIMARY KEY,data TEXT NOT NULL,expires INTEGER NOT NULL);").execute(db).await?;
    Ok(())
}
fn ring_contains(ring: &Value, x: f64, y: f64) -> bool {
    let Some(points) = ring.as_array() else {
        return false;
    };
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (xi, yi, xj, yj) = (
            points[i][0].as_f64().unwrap(),
            points[i][1].as_f64().unwrap(),
            points[j][0].as_f64().unwrap(),
            points[j][1].as_f64().unwrap(),
        );
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside
        }
        j = i;
    }
    inside
}
fn polygon_contains(p: &Value, x: f64, y: f64) -> bool {
    p.as_array().is_some_and(|rings| {
        !rings.is_empty()
            && ring_contains(&rings[0], x, y)
            && !rings.iter().skip(1).any(|r| ring_contains(r, x, y))
    })
}
fn in_drc(lon: f64, lat: f64) -> bool {
    BOUNDARY["features"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| match f["geometry"]["type"].as_str() {
            Some("Polygon") => polygon_contains(&f["geometry"]["coordinates"], lon, lat),
            Some("MultiPolygon") => f["geometry"]["coordinates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| polygon_contains(p, lon, lat)),
            _ => false,
        })
}
fn clean(v: &Value, key: &str) -> Option<String> {
    v[key]
        .as_str()
        .map(|s| s.trim().chars().take(120).collect::<String>())
        .filter(|s| !s.is_empty())
}
fn restricted_name(s: &str) -> bool {
    let s = s.to_lowercase();
    [
        "air force",
        "airforce",
        "military",
        "army",
        "navy",
        "defence",
        "defense",
        "armed forces",
        "force aerienne",
        "force aérienne",
    ]
    .iter()
    .any(|v| s.contains(v))
}
fn normalized(a: &Value, snapshot: f64, now: f64) -> Option<Value> {
    let flags = a["dbFlags"].as_u64().unwrap_or(0);
    if flags & (1 | 4 | 8) != 0 {
        return None;
    } // military, PIA, LADD
    if a["alt_baro"] == "ground" || a["on_ground"] == true {
        return None;
    }
    if clean(a, "ownOp").is_some_and(|s| restricted_name(&s))
        || clean(a, "desc").is_some_and(|s| restricted_name(&s))
    {
        return None;
    }
    let hex = clean(a, "hex")?.to_lowercase();
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let (lon, lat, seen) = (
        a["lon"].as_f64()?,
        a["lat"].as_f64()?,
        a["seen_pos"].as_f64()?,
    );
    if !lon.is_finite()
        || !lat.is_finite()
        || !seen.is_finite()
        || !(0.0..=90.0).contains(&seen)
        || !in_drc(lon, lat)
    {
        return None;
    }
    let observed = snapshot - seen;
    if observed > now + 30. || now - observed > 180. {
        return None;
    }
    Some(
        json!({"icao24":hex,"callsign":clean(a,"flight"),"registration":clean(a,"r"),"aircraft_type":clean(a,"t"),"description":clean(a,"desc"),"longitude":lon,"latitude":lat,"altitude_ft":a["alt_baro"].as_f64(),"ground_speed_knots":a["gs"].as_f64(),"track_degrees":a["track"].as_f64(),"position_source":clean(a,"type"),"observed_at":observed as i64,"category":"unknown","classification_basis":"No verified flight-purpose information","mission":"unknown","position_source_url":"https://api.adsb.lol/docs"}),
    )
}
fn classify(v: &mut Value, airline: &Value) {
    let Some(name) = airline["name"].as_str() else {
        return;
    };
    let n = name.to_lowercase();
    // A source-reported airline match is not proof of the payload or mission of this flight.
    v["airline"] = json!(name);
    v["airline_icao"] = airline["icao"].clone();
    v["metadata_source_url"] = json!(format!(
        "https://api.adsbdb.com/v0/airline/{}",
        airline["icao"].as_str().unwrap_or("")
    ));
    if n.contains("cargo")
        || n.contains("fedex")
        || n.contains("federal express")
        || n == "ups airlines"
    {
        v["category"] = json!("cargo_operator");
        v["classification_basis"] = json!(
            "Public airline directory identifies a cargo operator; this flight's payload and mission are unverified"
        );
    } else {
        v["category"] = json!("airline");
        v["classification_basis"] = json!(
            "Callsign prefix matches the public airline directory; passenger/cargo mission unverified"
        );
    }
}
async fn reserve(db: &SqlitePool) -> Result<()> {
    let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let r=sqlx::query("INSERT INTO flight_requests(day,n) SELECT ?,1 WHERE ?>0 ON CONFLICT(day) DO UPDATE SET n=n+1 WHERE n<?").bind(day).bind(cap()).bind(cap()).execute(db).await?;
    ensure!(
        r.rows_affected() == 1,
        "Public-flight daily request limit reached"
    );
    Ok(())
}
async fn get(db: &SqlitePool, c: &reqwest::Client, url: &str) -> Result<Value> {
    reserve(db).await?;
    let mut r = c.get(url).send().await?;
    if r.status() == 429 || r.status() == 503 {
        let pause = r
            .headers()
            .get("retry-after")
            .and_then(|s| s.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(900)
            .clamp(300, 86400);
        sqlx::query("UPDATE flight_snapshot SET next_at=max(next_at,?) WHERE id=1")
            .bind(chrono::Utc::now().timestamp() + pause)
            .execute(db)
            .await?;
    }
    ensure!(
        r.status().is_success(),
        "Public-flight source HTTP {}",
        r.status()
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = r.chunk().await? {
        ensure!(
            bytes.len() + chunk.len() <= 4_000_000,
            "Flight response exceeds limit"
        );
        bytes.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&bytes)?)
}
async fn airline(db: &SqlitePool, c: &reqwest::Client, prefix: &str) -> Result<Value> {
    let key = format!("airline-{prefix}");
    let now = chrono::Utc::now().timestamp();
    if let Some(data) = sqlx::query_scalar::<_, String>(
        "SELECT data FROM flight_metadata WHERE key=? AND expires>?",
    )
    .bind(&key)
    .bind(now)
    .fetch_optional(db)
    .await?
    {
        return Ok(serde_json::from_str(&data)?);
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    let data = get(
        db,
        c,
        &format!("https://api.adsbdb.com/v0/airline/{prefix}"),
    )
    .await?;
    let found = data["response"]
        .as_array()
        .and_then(|a| a.iter().find(|r| r["icao"] == prefix))
        .cloned()
        .unwrap_or(Value::Null);
    sqlx::query("INSERT INTO flight_metadata VALUES(?,?,?) ON CONFLICT(key) DO UPDATE SET data=excluded.data,expires=excluded.expires").bind(key).bind(found.to_string()).bind(now+86400*7).execute(db).await?;
    Ok(found)
}
async fn scan(db: &SqlitePool) -> Result<()> {
    let c = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .user_agent("CongoSignal/0.1 (public civilian ADS-B research)")
        .build()?;
    let start = chrono::Utc::now().timestamp();
    let mut aircraft = BTreeMap::<String, Value>::new();
    let mut errors = Vec::new();
    let mut completed = 0;
    let mut attempted = 0;
    // 250 nautical-mile circles cover the DRC bounding region. Exact footprint filtering follows.
    'tiles: for lat in [-12, -7, -2, 3] {
        for lon in [14, 19, 24, 29] {
            attempted += 1;
            let url = format!("{SOURCE}/v2/point/{lat}/{lon}/250");
            match get(db, &c, &url).await {
                Ok(data) => {
                    let parsed = (|| -> Result<Vec<Value>> {
                        let time = data["now"]
                            .as_f64()
                            .ok_or_else(|| anyhow::anyhow!("Missing feed timestamp"))?
                            / 1000.;
                        let now = chrono::Utc::now().timestamp() as f64;
                        ensure!((now - time).abs() < 180., "Stale upstream snapshot");
                        let rows = data["ac"]
                            .as_array()
                            .ok_or_else(|| anyhow::anyhow!("Invalid aircraft array"))?;
                        Ok(rows
                            .iter()
                            .filter_map(|r| normalized(r, time, now))
                            .collect())
                    })();
                    match parsed {
                        Ok(rows) => {
                            completed += 1;
                            for row in rows {
                                let id = row["icao24"].as_str().unwrap().to_string();
                                if aircraft.get(&id).is_none_or(|v| {
                                    v["observed_at"].as_i64() < row["observed_at"].as_i64()
                                }) {
                                    aircraft.insert(id, row);
                                }
                            }
                        }
                        Err(e) => errors.push(e.to_string()),
                    }
                }
                Err(e) => {
                    errors.push(e.to_string());
                    if errors.last().is_some_and(|s| {
                        s.contains("429") || s.contains("503") || s.contains("daily request")
                    }) {
                        break 'tiles;
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
    let mut prefixes = BTreeMap::new();
    for v in aircraft.values() {
        if let Some(call) = v["callsign"].as_str() {
            let p = call.chars().take(3).collect::<String>();
            if p.len() == 3 && p.bytes().all(|b| b.is_ascii_uppercase()) && prefixes.len() < 12 {
                prefixes.insert(p, Value::Null);
            }
        }
    }
    for (prefix, v) in prefixes.iter_mut() {
        match airline(db, &c, prefix).await {
            Ok(a) => *v = a,
            Err(e) => {
                let message = e.to_string();
                let stop = message.contains("429")
                    || message.contains("503")
                    || message.contains("daily request");
                errors.push(message);
                if stop {
                    break;
                }
            }
        }
    }
    aircraft.retain(|_, v| {
        let prefix = v["callsign"]
            .as_str()
            .unwrap_or("")
            .chars()
            .take(3)
            .collect::<String>();
        if let Some(a) = prefixes.get(&prefix) {
            if a["name"].as_str().is_some_and(restricted_name) {
                return false;
            }
            classify(v, a);
        }
        true
    });
    let mut out = json!({"items":aircraft.into_values().collect::<Vec<_>>(),"checked_at":chrono::Utc::now().to_rfc3339(),"scan_started_at":start,"tiles_successful":completed,"tiles_attempted":attempted,"tiles_total":16,"errors":errors,"status":if completed==16{"ok"}else if completed>0{"partial"}else{"unavailable"}});
    if completed == 0 {
        let old: String = sqlx::query_scalar("SELECT data FROM flight_snapshot WHERE id=1")
            .fetch_one(db)
            .await?;
        let old: Value = serde_json::from_str(&old)?;
        out["items"] = old["items"]
            .as_array()
            .map(|a| json!(a))
            .unwrap_or(json!([]));
    }
    sqlx::query("UPDATE flight_snapshot SET data=? WHERE id=1")
        .bind(out.to_string())
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM flight_metadata WHERE expires<?")
        .bind(start)
        .execute(db)
        .await?;
    sqlx::query("DELETE FROM flight_requests WHERE day<date('now','-35 days')")
        .execute(db)
        .await?;
    Ok(())
}
pub async fn worker(db: SqlitePool) -> Result<()> {
    loop {
        if enabled() {
            let now = chrono::Utc::now().timestamp();
            let r = sqlx::query("UPDATE flight_snapshot SET next_at=? WHERE id=1 AND next_at<=?")
                .bind(now + poll() as i64)
                .bind(now)
                .execute(&db)
                .await?;
            if r.rows_affected() == 1 {
                if let Err(e) = scan(&db).await {
                    tracing::warn!(error=%e,"Flight scan failed");
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}
pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let r = sqlx::query("SELECT data,next_at FROM flight_snapshot WHERE id=1")
        .fetch_one(&app.db)
        .await?;
    let mut v: Value = serde_json::from_str(r.get("data")).map_err(anyhow::Error::from)?;
    let now = chrono::Utc::now().timestamp();
    let items = v["items"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|mut a| {
            let age = now - a["observed_at"].as_i64()?;
            if !(0..=600).contains(&age) {
                return None;
            }
            a["age_seconds"] = json!(age);
            a["stale"] = json!(age > 120);
            Some(a)
        })
        .collect::<Vec<_>>();
    v["items"] = json!(items);
    v["enabled"] = json!(enabled());
    v["poll_seconds"] = json!(poll());
    v["next_check_at"] = json!(r.get::<i64, _>("next_at"));
    v["requests_today"] = json!(
        sqlx::query_scalar::<_, i64>("SELECT n FROM flight_requests WHERE day=?")
            .bind(chrono::Utc::now().format("%Y-%m-%d").to_string())
            .fetch_optional(&app.db)
            .await?
            .unwrap_or(0)
    );
    v["daily_request_limit"] = json!(cap());
    if v["status"].is_null() {
        v["status"] = json!("pending")
    }
    v["source"] = json!({"name":"ADSB.lol","url":"https://api.adsb.lol/docs","license":"ODbL 1.0","metadata":"https://www.adsbdb.com/"});
    v["coverage_note"] = json!(
        "Public receiver coverage is incomplete. Empty results do not mean empty airspace. Geographic DRC footprint is approximate, not an official FIR boundary. Positions are not extrapolated. Military/PIA/LADD flagged aircraft excluded; unknown classifications remain unverified."
    );
    Ok(Json(v))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn borders_and_holes() {
        assert!(in_drc(25., -4.));
        assert!(!in_drc(28., -15.));
        assert!(!in_drc(32., 0.));
        let p = json!([
            [[0, 0], [4, 0], [4, 4], [0, 4], [0, 0]],
            [[1, 1], [3, 1], [3, 3], [1, 3], [1, 1]]
        ]);
        assert!(polygon_contains(&p, 0.5, 0.5));
        assert!(!polygon_contains(&p, 2., 2.));
    }
    #[test]
    fn reject_restricted_ground_stale_and_missing_positions() {
        let a = json!({"hex":"abc123","lat":-4,"lon":25,"seen_pos":2,"alt_baro":30000,"dbFlags":0});
        assert!(normalized(&a, 1000., 1000.).is_some());
        for flag in [1, 4, 8] {
            let mut x = a.clone();
            x["dbFlags"] = json!(flag);
            assert!(normalized(&x, 1000., 1000.).is_none());
        }
        for (key, value) in [
            ("seen_pos", json!(200)),
            ("lat", Value::Null),
            ("alt_baro", json!("ground")),
            ("ownOp", json!("Air Force")),
        ] {
            let mut x = a.clone();
            x[key] = value;
            assert!(normalized(&x, 1000., 1000.).is_none());
        }
        assert!(normalized(&a, 1000., 2000.).is_none());
    }
    #[test]
    fn airline_does_not_prove_mission() {
        let mut a = json!({"mission":"unknown"});
        classify(&mut a, &json!({"name":"Ethiopian Airlines","icao":"ETH"}));
        assert_eq!(a["category"], "airline");
        assert_eq!(a["mission"], "unknown");
        classify(&mut a, &json!({"name":"Cargolux","icao":"CLX"}));
        assert_eq!(a["category"], "cargo_operator");
    }
    #[tokio::test]
    async fn schema_is_restart_safe() {
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        init(&db).await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM flight_snapshot")
                .fetch_one(&db)
                .await
                .unwrap(),
            1
        );
    }
}
