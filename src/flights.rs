//! Shared, bounded ADS-B snapshots over the DRC from two community networks, enriched with
//! adsbdb aircraft and route records. Only the latest snapshot is kept; no flight history.
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
/// Region kept around the DRC: its bounding box widened by ~2°, so approaches and
/// cross-border traffic (Kigali, Entebbe, Brazzaville, Ndola) stay visible but marked.
fn in_approaches(lon: f64, lat: f64) -> bool {
    (10.0..=33.5).contains(&lon) && (-15.5..=7.5).contains(&lat)
}
fn km(a: (f64, f64), b: (f64, f64)) -> f64 {
    let r = std::f64::consts::PI / 180.;
    let (dlat, dlon) = ((b.1 - a.1) * r, (b.0 - a.0) * r);
    let h =
        (dlat / 2.).sin().powi(2) + (a.1 * r).cos() * (b.1 * r).cos() * (dlon / 2.).sin().powi(2);
    6371. * 2. * h.sqrt().min(1.).asin()
}
const MILITARY_TERMS: [&str; 12] = [
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
    "fardc",
    "luftwaffe",
];
const HUMANITARIAN_TERMS: [&str; 12] = [
    "united nations",
    "unhas",
    "world food programme",
    "world food program",
    "monusco",
    "red cross",
    "icrc",
    "mission aviation fellowship",
    "aviation sans frontieres",
    "aviation sans frontières",
    "echo flight",
    "medecins sans frontieres",
];
/// ICAO designators of carriers that operate dedicated freighters.
const CARGO_OPERATORS: [&str; 20] = [
    "ACP", "CLX", "CKS", "GTI", "ABW", "BOX", "BCS", "DHK", "DHX", "FDX", "UPS", "MPH", "CAO",
    "ICV", "AZG", "NCA", "SQC", "WGN", "TAY", "ABR",
];
/// Airframes that in this region fly almost exclusively as freighters.
const CARGO_AIRFRAMES: [&str; 9] = [
    "AN12", "AN26", "AN32", "AN72", "AN74", "A124", "IL76", "L382", "AN22",
];
fn has_term(s: &str, terms: &[&str]) -> bool {
    let s = s.to_lowercase();
    terms.iter().any(|t| s.contains(t))
}
/// Freighter variants in adsbdb type strings: "747 48EF", "777 F6N", "767 3S2F", "737 4B6(SF)", "757 236 BCF".
fn freighter_type(t: &str) -> bool {
    let t = t.to_uppercase();
    ["BCF", "BDSF", "P2F", "(SF)", "(F)", "-F", "FREIGHTER"]
        .iter()
        .any(|m| t.contains(m))
        || t.split_whitespace().skip(1).any(|w| {
            let b = w.as_bytes();
            b.len() >= 2
                && (b[0].is_ascii_digit() && w.ends_with('F')
                    || b[0] == b'F' && b[1].is_ascii_digit())
        })
}
fn normalized(a: &Value, snapshot: f64, now: f64) -> Option<Value> {
    let flags = a["dbFlags"].as_u64().unwrap_or(0);
    // PIA and LADD are owner-requested privacy programmes; honour them. Military (bit 1) is shown, labelled.
    if flags & (4 | 8) != 0 {
        return None;
    }
    if a["alt_baro"] == "ground" || a["on_ground"] == true {
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
        || !in_approaches(lon, lat)
    {
        return None;
    }
    let observed = snapshot - seen;
    if observed > now + 30. || now - observed > 180. {
        return None;
    }
    let inside = in_drc(lon, lat);
    let squawk = clean(a, "squawk");
    let emergency = match squawk.as_deref() {
        Some("7500") => Some("7500 · unlawful interference"),
        Some("7600") => Some("7600 · radio failure"),
        Some("7700") => Some("7700 · general emergency"),
        _ => clean(a, "emergency")
            .filter(|e| e != "none")
            .map(|_| "Emergency status broadcast"),
    };
    let kind = match a["category"].as_str() {
        Some("A7") => "helicopter",
        Some("A1") => "light",
        Some("A2") => "small",
        Some("A3") => "large",
        Some("A4") => "large",
        Some("A5") => "heavy",
        Some("B6") => "drone",
        _ => "unknown",
    };
    let military = flags & 1 != 0
        || clean(a, "ownOp").is_some_and(|s| has_term(&s, &MILITARY_TERMS))
        || clean(a, "desc").is_some_and(|s| has_term(&s, &MILITARY_TERMS));
    let mut v = json!({"icao24":hex,"callsign":clean(a,"flight"),"registration":clean(a,"r"),"aircraft_type":clean(a,"t"),"description":clean(a,"desc"),"owner":clean(a,"ownOp"),"longitude":lon,"latitude":lat,"altitude_ft":a["alt_baro"].as_f64(),"ground_speed_knots":a["gs"].as_f64(),"track_degrees":a["track"].as_f64(),"vertical_rate_fpm":a["baro_rate"].as_f64(),"squawk":squawk,"emergency":emergency,"kind":kind,"position_source":clean(a,"type"),"observed_at":observed as i64,"region":if inside {"drc"} else {"approach"},"category":"unknown","classification_basis":"No operator, airframe or route information matched","military":military});
    if military {
        v["category"] = json!("military");
        v["classification_basis"] = json!(if flags & 1 != 0 {
            "Flagged military in the public ADS-B aircraft database"
        } else {
            "Operator name indicates a military or state air arm"
        });
    }
    Some(v)
}
/// Category from (in order) military flag, humanitarian operator, freighter evidence, airline match.
/// Each basis says what matched; none of them proves the payload or mission of this flight.
fn classify(v: &mut Value, airline: &Value, aircraft: &Value) {
    if let Some(name) = airline["name"].as_str() {
        v["airline"] = json!(name);
        v["airline_icao"] = airline["icao"].clone();
        v["airline_country"] = airline["country"].clone();
    }
    if let Some(t) = aircraft["type"].as_str() {
        v["type_name"] = json!(
            format!("{} {}", aircraft["manufacturer"].as_str().unwrap_or(""), t)
                .trim()
                .to_string()
        );
    }
    if let Some(o) = aircraft["registered_owner"].as_str() {
        v["owner"] = json!(o);
        v["owner_country"] = aircraft["registered_owner_country_name"].clone();
    }
    if v["registration"].is_null() {
        v["registration"] = aircraft["registration"].clone();
    }
    let names: Vec<String> = ["owner", "airline", "description"]
        .iter()
        .filter_map(|k| v[*k].as_str().map(str::to_string))
        .collect();
    let set = |v: &mut Value, c: &str, basis: String| {
        v["category"] = json!(c);
        v["classification_basis"] = json!(basis);
    };
    if v["category"] == "military" {
        return;
    }
    if let Some(n) = names.iter().find(|n| has_term(n, &MILITARY_TERMS)) {
        v["military"] = json!(true);
        return set(
            v,
            "military",
            format!("Operator “{n}” is a military or state air arm"),
        );
    }
    let callsign = v["callsign"].as_str().unwrap_or("").to_string();
    if callsign.starts_with("UNO") || callsign.starts_with("UNH") {
        return set(
            v,
            "humanitarian",
            "Callsign uses the United Nations designator".into(),
        );
    }
    if let Some(n) = names.iter().find(|n| has_term(n, &HUMANITARIAN_TERMS)) {
        return set(
            v,
            "humanitarian",
            format!("Operator “{n}” is a UN or humanitarian air service"),
        );
    }
    if let Some(t) = aircraft["type"].as_str().filter(|t| freighter_type(t)) {
        return set(
            v,
            "cargo",
            format!("Airframe registered as a freighter variant ({t})"),
        );
    }
    let icao = airline["icao"]
        .as_str()
        .or(aircraft["registered_owner_operator_flag_code"].as_str())
        .unwrap_or("");
    if CARGO_OPERATORS.contains(&icao) {
        return set(v, "cargo", format!("{icao} operates dedicated freighters"));
    }
    if let Some(n) = names
        .iter()
        .find(|n| has_term(n, &["cargo", "freight", "logistics"]))
    {
        return set(v, "cargo", format!("Operator “{n}” is a cargo carrier"));
    }
    if let Some(t) = v["aircraft_type"]
        .as_str()
        .filter(|t| CARGO_AIRFRAMES.contains(t))
    {
        return set(
            v,
            "cargo",
            format!("{t} airframes fly almost exclusively as freighters in the region"),
        );
    }
    if let Some(n) = airline["name"].as_str() {
        set(
            v,
            "airline",
            format!(
                "Callsign matches {n} in the public airline directory; passenger or cargo mission unverified"
            ),
        );
    }
}
/// Scheduled route for a callsign, kept only with a plausibility verdict against the observed position.
fn route(v: &mut Value, r: &Value) {
    let airport = |a: &Value| json!({"iata":a["iata_code"],"icao":a["icao_code"],"name":a["name"],"city":a["municipality"],"country":a["country_name"],"lat":a["latitude"],"lon":a["longitude"]});
    let (o, d) = (&r["origin"], &r["destination"]);
    if o.is_null() || d.is_null() {
        return;
    }
    let pos = (
        v["longitude"].as_f64().unwrap_or(0.),
        v["latitude"].as_f64().unwrap_or(0.),
    );
    let plausible = match (
        o["longitude"].as_f64(),
        o["latitude"].as_f64(),
        d["longitude"].as_f64(),
        d["latitude"].as_f64(),
    ) {
        (Some(ox), Some(oy), Some(dx), Some(dy)) => {
            let direct = km((ox, oy), (dx, dy));
            km((ox, oy), pos) + km(pos, (dx, dy)) <= direct * 1.3 + 300.
        }
        _ => false,
    };
    let callsign = v["callsign"].as_str().unwrap_or("").to_string();
    v["route"] = json!({"origin":airport(o),"destination":airport(d),"plausible":plausible,"source_url":format!("https://api.adsbdb.com/v0/callsign/{callsign}")});
}
struct Provider {
    name: &'static str,
    url: fn(i32, i32) -> String,
    list: &'static str,
    pace: u64,
}
const PROVIDERS: [Provider; 2] = [
    Provider {
        name: "ADSB.lol",
        url: |lat, lon| format!("{SOURCE}/v2/point/{lat}/{lon}/250"),
        list: "ac",
        // ADSB.lol answered 429 after ~12 requests at 4 s spacing (2026-10-19).
        pace: 8,
    },
    Provider {
        name: "adsb.fi",
        url: |lat, lon| format!("https://opendata.adsb.fi/api/v2/lat/{lat}/lon/{lon}/dist/250"),
        list: "aircraft",
        pace: 2,
    },
];
/// 250-nautical-mile circles on a 5° grid cover the DRC bounding region; exact filtering follows.
/// Eastern circles first: they hold most of the traffic of interest.
const TILES: [(i32, i32); 16] = [
    (-2, 29),
    (-7, 29),
    (3, 29),
    (-12, 29),
    (-2, 24),
    (-7, 24),
    (3, 24),
    (-12, 24),
    (-2, 19),
    (-7, 19),
    (3, 19),
    (-12, 19),
    (-2, 14),
    (-7, 14),
    (3, 14),
    (-12, 14),
];
async fn reserve(db: &SqlitePool, provider: &str) -> Result<()> {
    // One counter per provider per UTC day (key "YYYY-MM-DD|provider").
    let day = format!("{}|{provider}", chrono::Utc::now().format("%Y-%m-%d"));
    let r = sqlx::query("INSERT INTO flight_requests(day,n) SELECT ?,1 WHERE ?>0 ON CONFLICT(day) DO UPDATE SET n=n+1 WHERE n<?")
        .bind(day).bind(cap()).bind(cap()).execute(db).await?;
    ensure!(
        r.rows_affected() == 1,
        "{provider} daily request limit reached"
    );
    Ok(())
}
async fn backoff_until(db: &SqlitePool, provider: &str) -> Result<i64> {
    Ok(
        sqlx::query_scalar::<_, String>("SELECT data FROM flight_metadata WHERE key=?")
            .bind(format!("backoff-{provider}"))
            .fetch_optional(db)
            .await?
            .and_then(|s| s.parse().ok())
            .unwrap_or(0),
    )
}
async fn get(db: &SqlitePool, c: &reqwest::Client, provider: &str, url: &str) -> Result<Value> {
    reserve(db, provider).await?;
    let mut r = c.get(url).send().await?;
    if r.status() == 429 || r.status() == 503 {
        let pause = r
            .headers()
            .get("retry-after")
            .and_then(|s| s.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(600)
            .clamp(120, 86400);
        let until = chrono::Utc::now().timestamp() + pause;
        sqlx::query("INSERT INTO flight_metadata VALUES(?,?,?) ON CONFLICT(key) DO UPDATE SET data=excluded.data,expires=excluded.expires")
            .bind(format!("backoff-{provider}")).bind(until.to_string()).bind(until).execute(db).await?;
    }
    ensure!(r.status().is_success(), "{provider} HTTP {}", r.status());
    let mut bytes = Vec::new();
    while let Some(chunk) = r.chunk().await? {
        ensure!(
            bytes.len() + chunk.len() <= 8_000_000,
            "{provider} response exceeds limit"
        );
        bytes.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&bytes)?)
}
/// Cached adsbdb lookup (airline, aircraft or callsign route). Misses are cached too.
async fn lookup(
    db: &SqlitePool,
    c: &reqwest::Client,
    kind: &str,
    id: &str,
    ttl: i64,
) -> Result<(Value, bool)> {
    let key = format!("{kind}-{id}");
    let now = chrono::Utc::now().timestamp();
    if let Some(data) = sqlx::query_scalar::<_, String>(
        "SELECT data FROM flight_metadata WHERE key=? AND expires>?",
    )
    .bind(&key)
    .bind(now)
    .fetch_optional(db)
    .await?
    {
        return Ok((serde_json::from_str(&data)?, false));
    }
    ensure!(
        backoff_until(db, "adsbdb").await? <= now,
        "adsbdb backing off"
    );
    tokio::time::sleep(Duration::from_millis(1100)).await;
    let url = format!("https://api.adsbdb.com/v0/{kind}/{id}");
    let found = match get(db, c, "adsbdb", &url).await {
        Ok(data) => match kind {
            "airline" => data["response"]
                .as_array()
                .and_then(|a| a.iter().find(|r| r["icao"] == id))
                .cloned()
                .unwrap_or(Value::Null),
            "aircraft" => data["response"]["aircraft"].clone(),
            _ => data["response"]["flightroute"].clone(),
        },
        // adsbdb answers unknown aircraft and callsigns with HTTP 404: a cacheable miss.
        Err(e) if e.to_string().contains("404") => Value::Null,
        Err(e) => return Err(e),
    };
    let ttl = if found.is_null() { ttl.min(86400) } else { ttl };
    sqlx::query("INSERT INTO flight_metadata VALUES(?,?,?) ON CONFLICT(key) DO UPDATE SET data=excluded.data,expires=excluded.expires")
        .bind(key).bind(found.to_string()).bind(now + ttl).execute(db).await?;
    Ok((found, true))
}
/// Runs one provider over all tiles; returns its aircraft and a status record.
async fn scan_provider(db: &SqlitePool, c: &reqwest::Client, p: &Provider) -> (Vec<Value>, Value) {
    let (mut rows, mut errors, mut ok, mut attempted) = (Vec::new(), Vec::<String>::new(), 0, 0);
    let now = chrono::Utc::now().timestamp();
    match backoff_until(db, p.name).await {
        Ok(until) if until > now => {
            return (
                rows,
                json!({"name":p.name,"tiles_successful":0,"tiles_attempted":0,"errors":[format!("Rate limited; resuming in {} min", (until - now + 59) / 60)],"aircraft":0}),
            );
        }
        _ => {}
    }
    for (lat, lon) in TILES {
        attempted += 1;
        match get(db, c, p.name, &(p.url)(lat, lon)).await {
            Ok(data) => {
                let time = data["now"]
                    .as_f64()
                    .map(|t| if t > 1e11 { t / 1000. } else { t });
                let now = chrono::Utc::now().timestamp() as f64;
                match (time, data[p.list].as_array()) {
                    (Some(t), Some(list)) if (now - t).abs() < 180. => {
                        ok += 1;
                        rows.extend(list.iter().filter_map(|r| normalized(r, t, now)));
                    }
                    (Some(_), Some(_)) => {
                        errors.push(format!("{} returned a stale snapshot", p.name))
                    }
                    _ => errors.push(format!("{} returned an unexpected response", p.name)),
                }
            }
            Err(e) => {
                let m = e.to_string();
                let stop = m.contains("429") || m.contains("503") || m.contains("daily request");
                errors.push(m);
                if stop {
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(p.pace)).await;
    }
    errors.dedup();
    let status = json!({"name":p.name,"tiles_successful":ok,"tiles_attempted":attempted,"errors":errors,"aircraft":rows.len()});
    (rows, status)
}
async fn scan(db: &SqlitePool) -> Result<()> {
    let c = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .user_agent("CongoSignal/0.2 (public ADS-B research)")
        .build()?;
    let start = chrono::Utc::now().timestamp();
    // Both networks in parallel: their receiver sets differ, so the merge sees more aircraft.
    let ((a, sa), (b, sb)) = tokio::join!(
        scan_provider(db, &c, &PROVIDERS[0]),
        scan_provider(db, &c, &PROVIDERS[1])
    );
    let mut providers = vec![sa, sb];
    let mut aircraft = BTreeMap::<String, Value>::new();
    let mut sources = BTreeMap::<String, Vec<&str>>::new();
    for (rows, name) in [(a, PROVIDERS[0].name), (b, PROVIDERS[1].name)] {
        for row in rows {
            let id = row["icao24"].as_str().unwrap().to_string();
            sources.entry(id.clone()).or_default().push(name);
            if aircraft
                .get(&id)
                .is_none_or(|v| v["observed_at"].as_i64() < row["observed_at"].as_i64())
            {
                aircraft.insert(id, row);
            }
        }
    }
    // Military aircraft list: one global request, filtered to the region.
    if backoff_until(db, "ADSB.lol").await? <= start {
        match get(db, &c, "ADSB.lol", &format!("{SOURCE}/v2/mil")).await {
            Ok(data) => {
                let (t, now) = (data["now"].as_f64().unwrap_or(0.) / 1000., chrono::Utc::now().timestamp() as f64);
                let mut n = 0;
                for row in data["ac"].as_array().into_iter().flatten().filter_map(|r| normalized(r, t, now)) {
                    let id = row["icao24"].as_str().unwrap().to_string();
                    sources.entry(id.clone()).or_default().push("ADSB.lol military feed");
                    aircraft.entry(id).or_insert(row);
                    n += 1;
                }
                providers.push(json!({"name":"ADSB.lol military feed","tiles_successful":1,"tiles_attempted":1,"errors":[],"aircraft":n}));
            }
            Err(e) => providers.push(json!({"name":"ADSB.lol military feed","tiles_successful":0,"tiles_attempted":1,"errors":[e.to_string()],"aircraft":0})),
        }
    }
    for (id, v) in aircraft.iter_mut() {
        let mut s = sources.remove(id).unwrap_or_default();
        s.dedup();
        v["sources"] = json!(s);
    }
    // Enrichment from adsbdb (cached; bounded per scan so a busy sky cannot exhaust the free API).
    let mut errors = Vec::new();
    let (mut fresh, budget) = (0, 40);
    for v in aircraft.values_mut() {
        if fresh >= budget {
            break;
        }
        let id = v["icao24"].as_str().unwrap().to_string();
        let callsign = v["callsign"].as_str().map(str::to_string);
        let mut used = false;
        let mut take = |r: Result<(Value, bool)>, errors: &mut Vec<String>| match r {
            Ok((v, f)) => {
                used |= f;
                v
            }
            Err(e) => {
                errors.push(e.to_string());
                Value::Null
            }
        };
        let craft = take(
            lookup(db, &c, "aircraft", &id, 86400 * 30).await,
            &mut errors,
        );
        let mut airline = Value::Null;
        if let Some(cs) = callsign
            .as_deref()
            .filter(|cs| cs.len() >= 4 && cs.chars().all(|c| c.is_ascii_alphanumeric()))
        {
            let r = take(lookup(db, &c, "callsign", cs, 43200).await, &mut errors);
            airline = r["airline"].clone();
            if !r.is_null() {
                route(v, &r);
            }
            let prefix: String = cs.chars().take(3).collect();
            if airline.is_null() && prefix.bytes().all(|b| b.is_ascii_uppercase()) {
                airline = take(
                    lookup(db, &c, "airline", &prefix, 86400 * 7).await,
                    &mut errors,
                );
            }
        }
        classify(v, &airline, &craft);
        if used {
            fresh += 1;
        }
    }
    // Aircraft beyond the per-scan budget still get military/airframe classification without lookups.
    for v in aircraft.values_mut().filter(|v| {
        v["classification_basis"] == "No operator, airframe or route information matched"
    }) {
        classify(v, &Value::Null, &Value::Null);
    }
    errors.sort();
    errors.dedup();
    if !errors.is_empty() {
        providers.push(json!({"name":"adsbdb (routes and aircraft)","tiles_successful":0,"tiles_attempted":0,"errors":errors.into_iter().take(3).collect::<Vec<_>>(),"aircraft":0}));
    }
    let tiles_ok: i64 = providers
        .iter()
        .take(2)
        .map(|p| p["tiles_successful"].as_i64().unwrap_or(0))
        .sum();
    let full = providers
        .iter()
        .take(2)
        .any(|p| p["tiles_successful"] == 16);
    let mut out = json!({"items":aircraft.into_values().collect::<Vec<_>>(),"checked_at":chrono::Utc::now().to_rfc3339(),"scan_started_at":start,"tiles_successful":tiles_ok,"tiles_attempted":providers.iter().take(2).map(|p| p["tiles_attempted"].as_i64().unwrap_or(0)).sum::<i64>(),"tiles_total":32,"providers":providers,"errors":[],"status":if full {"ok"} else if tiles_ok>0 {"partial"} else {"unavailable"}});
    if tiles_ok == 0 {
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
    sqlx::query("DELETE FROM flight_requests WHERE substr(day,1,10)<date('now','-35 days')")
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
            if r.rows_affected() == 1
                && let Err(e) = scan(&db).await
            {
                tracing::warn!(error=%e,"Flight scan failed");
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
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    v["requests_today"] = json!(
        sqlx::query_scalar::<_, i64>(
            "SELECT coalesce(sum(n),0) FROM flight_requests WHERE substr(day,1,10)=?"
        )
        .bind(today)
        .fetch_one(&app.db)
        .await?
    );
    v["daily_request_limit"] = json!(cap());
    if v["status"].is_null() {
        v["status"] = json!("pending")
    }
    v["source"] = json!({"name":"ADSB.lol + adsb.fi","url":"https://api.adsb.lol/docs","license":"ODbL 1.0 (ADSB.lol); adsb.fi open data, non-commercial","metadata":"https://www.adsbdb.com/"});
    v["coverage_note"] = json!(
        "Community receiver networks have very few receivers in central Africa, so most aircraft over the DRC are not visible. Empty results do not mean empty airspace. Positions are not extrapolated. Military aircraft that broadcast ADS-B are shown and labelled; owner-requested privacy (PIA/LADD) aircraft are excluded. Routes are the published schedule for the callsign and are checked against the observed position."
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
    fn privacy_ground_stale_and_missing_positions_are_dropped_military_is_labelled() {
        let a = json!({"hex":"abc123","lat":-4,"lon":25,"seen_pos":2,"alt_baro":30000,"dbFlags":0});
        let v = normalized(&a, 1000., 1000.).unwrap();
        assert_eq!(v["region"], "drc");
        assert_eq!(v["category"], "unknown");
        for flag in [4, 8] {
            let mut x = a.clone();
            x["dbFlags"] = json!(flag);
            assert!(
                normalized(&x, 1000., 1000.).is_none(),
                "privacy flag {flag}"
            );
        }
        let mut m = a.clone();
        m["dbFlags"] = json!(1);
        assert_eq!(
            normalized(&m, 1000., 1000.).unwrap()["category"],
            "military"
        );
        let mut near = a.clone();
        near["lon"] = json!(30.1);
        near["lat"] = json!(-1.9); // Kigali: outside the DRC, inside the approaches
        assert_eq!(
            normalized(&near, 1000., 1000.).unwrap()["region"],
            "approach"
        );
        for (key, value) in [
            ("seen_pos", json!(200)),
            ("lat", Value::Null),
            ("alt_baro", json!("ground")),
            ("lon", json!(40)),
        ] {
            let mut x = a.clone();
            x[key] = value;
            assert!(normalized(&x, 1000., 1000.).is_none(), "{key}");
        }
        assert!(normalized(&a, 1000., 2000.).is_none());
        let mut sq = a.clone();
        sq["squawk"] = json!("7700");
        assert!(
            normalized(&sq, 1000., 1000.).unwrap()["emergency"]
                .as_str()
                .unwrap()
                .starts_with("7700")
        );
    }
    #[test]
    fn classification_states_its_basis() {
        let base = || {
            normalized(&json!({"hex":"abc123","lat":-4,"lon":25,"seen_pos":2,"alt_baro":30000,"flight":"ABD2616 "}), 1000., 1000.).unwrap()
        };
        let mut a = base();
        classify(
            &mut a,
            &json!({"name":"Ethiopian Airlines","icao":"ETH"}),
            &Value::Null,
        );
        assert_eq!(a["category"], "airline");
        let mut c = base();
        classify(
            &mut c,
            &json!({"name":"Air Atlanta Icelandic","icao":"ABD"}),
            &json!({"type":"747 48EF","manufacturer":"Boeing","registered_owner":"Astral Aviation"}),
        );
        assert_eq!(c["category"], "cargo");
        assert!(c["classification_basis"].as_str().unwrap().contains("48EF"));
        let mut u = base();
        u["callsign"] = json!("UNO123");
        classify(&mut u, &Value::Null, &Value::Null);
        assert_eq!(u["category"], "humanitarian");
        let mut m = base();
        classify(
            &mut m,
            &Value::Null,
            &json!({"type":"C-130H","registered_owner":"Force Aérienne"}),
        );
        assert_eq!(m["category"], "military");
        let mut an = base();
        an["aircraft_type"] = json!("AN26");
        classify(&mut an, &Value::Null, &Value::Null);
        assert_eq!(an["category"], "cargo");
        for t in [
            "747 48EF",
            "777 F6N",
            "767 3S2F",
            "737 4B6(SF)",
            "757 236 BCF",
        ] {
            assert!(freighter_type(t), "{t}");
        }
        for t in ["737 8AS", "A320 214", "787 9", "777 36N(ER)"] {
            assert!(!freighter_type(t), "{t}");
        }
    }
    #[test]
    fn routes_are_checked_against_position() {
        let mut v = json!({"callsign":"ABD2616","longitude":28.2,"latitude":-25.0});
        let r = json!({"origin":{"iata_code":"LGG","latitude":50.6374,"longitude":5.44322,"name":"Liège Airport"},"destination":{"iata_code":"JNB","latitude":-26.1392,"longitude":28.246,"name":"OR Tambo"}});
        route(&mut v, &r);
        assert_eq!(v["route"]["plausible"], true);
        let mut far = json!({"callsign":"ABD2616","longitude":100.0,"latitude":10.0});
        route(&mut far, &r);
        assert_eq!(far["route"]["plausible"], false);
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
