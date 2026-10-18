//! Live DRC radio: stations from Radio Browser (community directory, public API) merged with a
//! curated list of official streams (config/radio-stations.json), each health-checked so only
//! streams that currently serve audio are offered as live. Stations are placed on the map from
//! directory coordinates or the gazetteer; the dashboard plays streams directly in the browser.
use crate::{ApiError, ApiResult, App, reference};
use anyhow::{Context, Result};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::sync::LazyLock;

#[derive(Deserialize, Clone)]
struct Curated {
    id: String,
    name: String,
    city: Option<String>,
    homepage: Option<String>,
    stream: String,
    #[serde(default)]
    languages: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    coverage: Option<String>,
    source_note: Option<String>,
}
#[derive(Deserialize)]
struct Config {
    stations: Vec<Curated>,
}
static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../config/radio-stations.json"))
        .expect("valid radio stations")
});
const DIRECTORY: &str = "https://de1.api.radio-browser.info";
/// DRC bounding box; directory coordinates outside it are ignored.
fn in_drc(lat: f64, lon: f64) -> bool {
    (-13.6..=5.5).contains(&lat) && (12.0..=31.5).contains(&lon)
}

pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS radio_stations (id TEXT PRIMARY KEY, data TEXT NOT NULL, health TEXT, checked_at TEXT, listens INTEGER NOT NULL DEFAULT 0);",
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Station location: directory coordinates inside the DRC, else the first gazetteer place in its
/// city/state, name or tags; otherwise none (national or online-only).
fn locate(city: &str, name: &str, tags: &str, lat: Option<f64>, lon: Option<f64>) -> Value {
    if let (Some(lat), Some(lon)) = (lat, lon)
        && in_drc(lat, lon)
    {
        let near = crate::intelligence::first_place(city).map(|p| p.0);
        return json!({"lat":lat,"lon":lon,"place":near.or_else(|| Some(city.trim().to_string()).filter(|c| !c.is_empty())),"method":"directory_coordinates"});
    }
    for (text, method) in [
        (city, "city_gazetteer"),
        (name, "name_gazetteer"),
        (tags, "tags_gazetteer"),
    ] {
        // The gazetteer expects capitalized place names.
        let titled: String = text
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|w| !w.is_empty())
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().chain(c).collect::<String>())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ");
        if let Some((place, province, lat, lon)) =
            crate::intelligence::first_place(&format!("à {titled}"))
        {
            return json!({"lat":lat,"lon":lon,"place":place,"province":province,"method":method});
        }
    }
    Value::Null
}
fn split_list(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.trim().to_lowercase())
        .filter(|x| !x.is_empty())
        .collect()
}
fn stream_key(url: &str) -> String {
    url.trim()
        .trim_end_matches(['/', ';'])
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .to_lowercase()
}
async fn sync_directory(db: &SqlitePool, client: &reqwest::Client) -> Result<usize> {
    let rows = reference::get_json(
        client,
        &format!("{DIRECTORY}/json/stations/bycountrycodeexact/CD?limit=1000"),
    )
    .await?;
    let rows = rows.as_array().context("unexpected directory response")?;
    let curated_keys: Vec<(String, String)> = CONFIG
        .stations
        .iter()
        .map(|c| (stream_key(&c.stream), c.name.to_lowercase()))
        .collect();
    let mut stations: Vec<(String, Value)> = Vec::new();
    for c in &CONFIG.stations {
        let city = c.city.clone().unwrap_or_default();
        stations.push((c.id.clone(), json!({
            "id": c.id, "name": c.name, "source": "curated", "stream": c.stream, "homepage": c.homepage,
            "city": c.city, "languages": c.languages, "tags": c.tags, "coverage": c.coverage, "source_note": c.source_note,
            "location": locate(&city, &c.name, "", None, None), "favicon": null, "codec": null, "bitrate": null,
            "directory_ok": null, "directory_clicks": null,
        })));
    }
    for r in rows {
        let name = r["name"].as_str().unwrap_or("").trim().to_string();
        let stream = r["url_resolved"]
            .as_str()
            .filter(|u| !u.is_empty())
            .or(r["url"].as_str())
            .unwrap_or("")
            .to_string();
        let uuid = r["stationuuid"].as_str().unwrap_or("").to_string();
        if name.is_empty() || stream.is_empty() || uuid.is_empty() {
            continue;
        }
        // Curated official streams take precedence over directory entries for the same station.
        if curated_keys
            .iter()
            .any(|(k, n)| *k == stream_key(&stream) || *n == name.to_lowercase())
        {
            continue;
        }
        let state = r["state"].as_str().unwrap_or("").trim().to_string();
        let tags = r["tags"].as_str().unwrap_or("");
        let id = format!("rb-{uuid}");
        stations.push((id.clone(), json!({
            "id": id, "uuid": uuid, "name": name, "source": "radio-browser", "stream": stream,
            "homepage": r["homepage"].as_str().filter(|h| !h.is_empty()), "city": Some(state.clone()).filter(|s| !s.is_empty()),
            "languages": split_list(r["language"].as_str().unwrap_or("")), "tags": split_list(tags),
            "location": locate(&state, &name, tags, r["geo_lat"].as_f64(), r["geo_long"].as_f64()),
            "favicon": r["favicon"].as_str().filter(|f| f.starts_with("https://")),
            "codec": r["codec"].as_str(), "bitrate": r["bitrate"].as_i64().filter(|b| *b > 0),
            "directory_ok": r["lastcheckok"].as_i64() == Some(1), "directory_clicks": r["clickcount"].as_i64(),
        })));
    }
    let mut tx = db.begin().await?;
    let ids: Vec<String> = stations.iter().map(|(id, _)| id.clone()).collect();
    for (id, data) in &stations {
        sqlx::query("INSERT INTO radio_stations(id,data) VALUES(?,?) ON CONFLICT(id) DO UPDATE SET data=excluded.data")
            .bind(id)
            .bind(data.to_string())
            .execute(&mut *tx)
            .await?;
    }
    // Stations that left the directory and the curated list are removed.
    for id in sqlx::query_scalar::<_, String>("SELECT id FROM radio_stations")
        .fetch_all(&mut *tx)
        .await?
    {
        if !ids.contains(&id) {
            sqlx::query("DELETE FROM radio_stations WHERE id=?")
                .bind(&id)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Ok(stations.len())
}

/// Live check: the stream answers with audio (playlists are resolved to their first stream).
async fn probe(client: &reqwest::Client, url: &str, depth: u8) -> Value {
    let started = std::time::Instant::now();
    let result = async {
        let mut r = client
            .get(url)
            .header("Range", "bytes=0-8191")
            .header("Icy-MetaData", "0")
            .send()
            .await
            .context("unreachable")?;
        anyhow::ensure!(r.status().is_success(), "HTTP {}", r.status().as_u16());
        let header = |n: &str| r.headers().get(n).and_then(|v| v.to_str().ok()).map(String::from);
        let kind = header("content-type").unwrap_or_default().to_lowercase();
        let meta = json!({"content_type":kind,"icy_name":header("icy-name"),"icy_genre":header("icy-genre"),"icy_bitrate":header("icy-br"),"cors":header("access-control-allow-origin").is_some()});
        let playlist = kind.contains("mpegurl") || kind.contains("scpls") || url.ends_with(".pls") || url.ends_with(".m3u");
        let mut body = Vec::new();
        while let Ok(Some(chunk)) = tokio::time::timeout(std::time::Duration::from_secs(6), r.chunk()).await.unwrap_or(Ok(None)) {
            body.extend_from_slice(&chunk);
            if body.len() >= 4096 {
                break;
            }
        }
        Ok::<_, anyhow::Error>((kind, meta, playlist, body))
    }
    .await;
    match result {
        Ok((_, _, true, body)) if depth == 0 => {
            let text = String::from_utf8_lossy(&body);
            match text
                .split(['\n', '='])
                .map(str::trim)
                .find(|l| l.starts_with("http"))
            {
                Some(inner) => {
                    let mut v = Box::pin(probe(client, inner, 1)).await;
                    v["resolved_from_playlist"] = json!(inner);
                    v
                }
                None => json!({"ok":false,"error":"playlist without stream URL"}),
            }
        }
        Ok((kind, meta, _, body)) => {
            let audio = kind.starts_with("audio/")
                || kind.contains("ogg")
                || kind.contains("octet-stream")
                || kind.contains("aacp");
            let mut v = meta;
            v["ok"] = json!(audio && !body.is_empty());
            v["latency_ms"] = json!(started.elapsed().as_millis() as u64);
            if !audio {
                v["error"] = json!(format!("not audio ({kind})"));
            }
            v
        }
        Err(e) => {
            json!({"ok":false,"error":e.to_string(),"latency_ms":started.elapsed().as_millis() as u64})
        }
    }
}
async fn check_health(db: &SqlitePool, client: &reqwest::Client) -> Result<usize> {
    let rows = sqlx::query("SELECT id, data FROM radio_stations")
        .fetch_all(db)
        .await?;
    let mut live = 0;
    for r in rows {
        let data: Value = serde_json::from_str(&r.get::<String, _>("data"))?;
        let health = probe(client, data["stream"].as_str().unwrap_or(""), 0).await;
        live += usize::from(health["ok"] == true);
        sqlx::query("UPDATE radio_stations SET health=?, checked_at=? WHERE id=?")
            .bind(health.to_string())
            .bind(Utc::now().to_rfc3339())
            .bind(r.get::<String, _>("id"))
            .execute(db)
            .await?;
    }
    Ok(live)
}
pub async fn worker(db: SqlitePool) -> Result<()> {
    let client = reference::client()?;
    // Streams never end: cap each probe and read only the first bytes.
    let probe_client = reqwest::Client::builder()
        .user_agent(reference::user_agent())
        .connect_timeout(std::time::Duration::from_secs(8))
        .timeout(std::time::Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::limited(4))
        .build()?;
    loop {
        if reference::sync_enabled() {
            if reference::due(&db, "radio:directory", 86400).await? {
                let r = sync_directory(&db, &client).await;
                if let Err(e) = &r {
                    tracing::warn!(error = %e, "radio directory sync failed");
                }
                reference::record(&db, "radio:directory", &r, json!({"url":DIRECTORY})).await?;
            }
            if reference::due(&db, "radio:health", 1800).await? {
                let r = check_health(&db, &probe_client).await;
                reference::record(&db, "radio:health", &r, json!({})).await?;
            }
        }
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(120)) => {}
            _ = reference::REFRESH.notified() => {}
        }
    }
}

pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let mut items: Vec<Value> = sqlx::query("SELECT * FROM radio_stations")
        .fetch_all(&app.db)
        .await?
        .iter()
        .filter_map(|r| {
            let mut v: Value = serde_json::from_str(&r.get::<String, _>("data")).ok()?;
            v["health"] = r
                .get::<Option<String>, _>("health")
                .and_then(|h| serde_json::from_str(&h).ok())
                .unwrap_or(Value::Null);
            v["checked_at"] = json!(r.get::<Option<String>, _>("checked_at"));
            v["listens"] = json!(r.get::<i64, _>("listens"));
            Some(v)
        })
        .collect();
    // Live first, curated official streams first, then directory popularity.
    items.sort_by_key(|v| {
        (
            v["health"]["ok"] != true,
            v["source"] != "curated",
            -v["directory_clicks"].as_i64().unwrap_or(0),
            v["name"].as_str().unwrap_or("").to_lowercase(),
        )
    });
    let sync = reference::sync_rows(&app.db, "radio:").await?;
    Ok(Json(json!({
        "items": items, "sync": sync, "sync_enabled": reference::sync_enabled(),
        "sources": [{"name":"Radio Browser","url":"https://www.radio-browser.info/","license":"Open community directory (public API)"},
                    {"name":"Station websites","url":null,"license":"Official streams from each station's own player"}],
        "notice": "Streams are played directly from the broadcasters' servers. Availability changes often; each stream is re-checked every 30 minutes. Station locations are approximate (directory coordinates or the city in the station's listing)."
    })))
}
/// Counts a listen locally and, for directory stations, sends Radio Browser's requested click.
pub async fn listen(State(app): State<App>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let data: Option<String> = sqlx::query_scalar("SELECT data FROM radio_stations WHERE id=?")
        .bind(&id)
        .fetch_optional(&app.db)
        .await?;
    let data: Value = serde_json::from_str(
        &data.ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "unknown station".into()))?,
    )
    .map_err(anyhow::Error::from)?;
    sqlx::query("UPDATE radio_stations SET listens=listens+1 WHERE id=?")
        .bind(&id)
        .execute(&app.db)
        .await?;
    if let Some(uuid) = data["uuid"].as_str().map(String::from)
        && reference::sync_enabled()
    {
        tokio::spawn(async move {
            if let Ok(client) = reference::client() {
                let _ = client
                    .get(format!("{DIRECTORY}/json/url/{uuid}"))
                    .send()
                    .await;
            }
        });
    }
    Ok(Json(json!({"ok":true})))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stations_are_placed_from_coordinates_or_gazetteer() {
        let goma = locate("", "Radio Kivu", "", Some(-1.67), Some(29.22));
        assert_eq!(goma["method"], "directory_coordinates");
        assert!(
            locate("", "Radio X", "", Some(48.8), Some(2.3)).is_null(),
            "coordinates outside the DRC are ignored"
        );
        let bukavu = locate("bukavu", "Radio Svein", "radio comunitaria", None, None);
        assert_eq!(bukavu["place"], "Bukavu");
        assert_eq!(bukavu["province"], "Sud-Kivu");
        let kin = locate("Kinshasa ", "Top Congo FM", "", None, None);
        assert_eq!(kin["place"], "Kinshasa");
        assert!(locate("", "Gospel FM", "music,religion", None, None).is_null());
    }
    #[test]
    fn stream_keys_ignore_scheme_and_trailing_marks() {
        assert_eq!(
            stream_key("http://Host:8000/;"),
            stream_key("https://host:8000")
        );
        assert!(
            CONFIG
                .stations
                .iter()
                .all(|s| s.stream.starts_with("https://"))
        );
    }
}
