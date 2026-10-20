//! Territorial control: who holds which towns, from a curated and cited baseline
//! (config/territorial-control.json) plus analyst updates that must cite a source. Nothing is
//! inferred from mentions: extracted "seized/took control" events are returned separately as
//! unconfirmed reports. Territories are shaded client-side from the places inside them.
use crate::{ApiError, ApiResult, App};
use anyhow::Result;
use axum::{Json, extract::State, http::StatusCode};
use chrono::Utc;
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::sync::LazyLock;

static BASELINE: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../config/territorial-control.json"))
        .expect("valid territorial-control.json")
});
pub const CONTROLLERS: [&str; 7] = [
    "AFC/M23 (FRN)",
    "Government (FARDC)",
    "FARDC with Wazalendo",
    "Wazalendo",
    "Twirwaneho",
    "Contested",
    "Other",
];
pub const STATUSES: [&str; 5] = ["held", "contested", "recaptured", "withdrawn", "unclear"];

pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS control_dismissed(id TEXT PRIMARY KEY, user_id TEXT NOT NULL, at TEXT NOT NULL);")
        .execute(db)
        .await?;
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS control_updates(id INTEGER PRIMARY KEY AUTOINCREMENT, place TEXT NOT NULL, province TEXT, controller TEXT NOT NULL, status TEXT NOT NULL, as_of TEXT NOT NULL, note TEXT NOT NULL DEFAULT '', source_url TEXT NOT NULL, source_title TEXT NOT NULL DEFAULT '', user_id TEXT NOT NULL, created_at TEXT NOT NULL);")
        .execute(db)
        .await?;
    Ok(())
}
/// Gazetteer position for a place, preferring a match in the stated province.
fn locate(name: &str, province: Option<&str>) -> Option<(f64, f64, String)> {
    let found = crate::intelligence::place_tuples(&format!("à {name}"));
    let norm = |s: &str| crate::text::fold(s).replace(['-', ' '], "");
    let want = province.map(norm);
    found
        .iter()
        .find(|(n, p, ..)| norm(n) == norm(name) && want.as_ref().is_none_or(|w| &norm(p) == w))
        .or_else(|| found.iter().find(|(n, ..)| norm(n) == norm(name)))
        .map(|(_, p, lat, lon)| (*lat, *lon, p.clone()))
}
/// Baseline places with analyst updates applied (latest `as_of` wins), each with its history.
pub async fn places(db: &SqlitePool) -> Result<Vec<Value>> {
    let mut out: Vec<Value> = BASELINE["places"].as_array().cloned().unwrap_or_default();
    for p in out.iter_mut() {
        let entry = json!({"as_of":p["as_of"],"controller":p["controller"],"status":p["status"],"note":p["note"],"sources":p["sources"],"origin":"baseline"});
        p["history"] = json!([entry]);
        p["origin"] = json!("baseline");
    }
    let updates = sqlx::query("SELECT * FROM control_updates ORDER BY as_of, id")
        .fetch_all(db)
        .await?;
    for u in &updates {
        let name: String = u.get("place");
        let entry = json!({"as_of":u.get::<String,_>("as_of"),"controller":u.get::<String,_>("controller"),"status":u.get::<String,_>("status"),"note":u.get::<String,_>("note"),
            "sources":[{"title":u.get::<String,_>("source_title"),"url":u.get::<String,_>("source_url"),"date":u.get::<String,_>("as_of")}],"origin":"analyst","id":u.get::<i64,_>("id"),"created_at":u.get::<String,_>("created_at")});
        let idx = out.iter().position(|p| {
            crate::text::fold(p["name"].as_str().unwrap_or("")) == crate::text::fold(&name)
        });
        let p = match idx {
            Some(i) => &mut out[i],
            None => {
                out.push(json!({"name":name,"province":u.get::<Option<String>,_>("province"),"history":[],"origin":"analyst"}));
                out.last_mut().unwrap()
            }
        };
        p["history"].as_array_mut().unwrap().push(entry.clone());
        if p["as_of"]
            .as_str()
            .is_none_or(|a| a <= entry["as_of"].as_str().unwrap_or(""))
        {
            for k in ["controller", "status", "as_of", "note", "sources"] {
                p[k] = entry[k].clone();
            }
            p["origin"] = json!("analyst");
        }
    }
    for p in out.iter_mut() {
        // Baseline coordinates (GeoNames/OpenStreetMap, cited in the file) take precedence.
        if p["lat"].is_f64() {
            continue;
        }
        if let Some((lat, lon, prov)) =
            locate(p["name"].as_str().unwrap_or(""), p["province"].as_str())
        {
            p["lat"] = json!(lat);
            p["lon"] = json!(lon);
            if p["province"].is_null() {
                p["province"] = json!(prov);
            }
        }
    }
    Ok(out)
}
/// Armed-group activity zones (attacks, not control), placed on the gazetteer.
fn activity() -> Vec<Value> {
    BASELINE["activity"]
        .as_array()
        .into_iter()
        .flatten()
        .cloned()
        .map(|mut a| {
            if a["lat"].is_f64() {
                return a;
            }
            if let Some((lat, lon, _)) =
                locate(a["name"].as_str().unwrap_or(""), a["province"].as_str())
            {
                a["lat"] = json!(lat);
                a["lon"] = json!(lon);
            }
            a
        })
        .collect()
}
const REBEL: [&str; 6] = ["m23", "afc", "frn", "rebelles", "rebels", "rebelle"];
const GOV: [&str; 7] = [
    "fardc",
    "armee",
    "army",
    "wazalendo",
    "loyalistes",
    "militaires",
    "forces",
];
const SEIZE: [&str; 13] = [
    "empare",
    "empares",
    "emparee",
    "emparent",
    "pris le controle",
    "prennent le controle",
    "prise de",
    "seized",
    "captured",
    "took control",
    "entrent dans",
    "occupent",
    "tombe aux mains",
];
const RECAPTURE: [&str; 7] = [
    "reprise de",
    "reprennent",
    "ont repris",
    "recaptured",
    "retook",
    "reconquis",
    "reconquete",
];
const WITHDRAW: [&str; 6] = [
    "retrait",
    "se retire",
    "se retirent",
    "withdrew",
    "withdrawal",
    "evacue",
];
fn has(text: &str, cues: &[&str]) -> bool {
    let t = format!(" {} ", crate::text::tokens(text).join(" "));
    cues.iter()
        .any(|c| t.contains(&format!(" {} ", crate::text::tokens(c).join(" "))))
}
/// Proposed change from an extracted incident sentence: (controller, status, reason).
pub fn suggest(kind: &str, excerpt: &str) -> Option<(&'static str, &'static str, &'static str)> {
    let rebel = has(excerpt, &REBEL);
    let gov = has(excerpt, &GOV);
    if has(excerpt, &RECAPTURE) && gov {
        return Some((
            "Government (FARDC)",
            "recaptured",
            "Report of a recapture by government forces",
        ));
    }
    if has(excerpt, &WITHDRAW) && rebel {
        return Some((
            "Government (FARDC)",
            "withdrawn",
            "Report of a rebel withdrawal",
        ));
    }
    if has(excerpt, &SEIZE) && rebel {
        return Some((
            "AFC/M23 (FRN)",
            "held",
            "Report of a seizure by AFC/M23 (FRN)",
        ));
    }
    if kind == "armed_clash" || kind == "territorial_control" {
        return Some(("Contested", "contested", "Report of fighting at this place"));
    }
    None
}
/// Review queue: recent incident reports at places on the control list (or seizures anywhere)
/// that are newer than the place's last cited status. Analysts confirm or dismiss each one.
async fn suggestions(db: &SqlitePool, places: &[Value]) -> Result<Vec<Value>> {
    let since = (Utc::now() - chrono::Duration::days(21)).date_naive();
    let dismissed: Vec<String> = sqlx::query_scalar("SELECT id FROM control_dismissed")
        .fetch_all(db)
        .await?;
    let short = |n: &str| {
        crate::text::fold(
            n.split('(')
                .next()
                .unwrap_or(n)
                .split('/')
                .next()
                .unwrap_or(n)
                .trim(),
        )
    };
    let mut out = Vec::new();
    for e in crate::events::between(db, since).await? {
        let kind = e["kind"].as_str().unwrap_or("");
        if kind != "armed_clash" && kind != "territorial_control" {
            continue;
        }
        let place = e["place"].as_str().unwrap_or("");
        let near = |p: &&Value| {
            short(p["name"].as_str().unwrap_or("")) == crate::text::fold(place)
                || match (
                    p["lat"].as_f64(),
                    p["lon"].as_f64(),
                    e["lat"].as_f64(),
                    e["lon"].as_f64(),
                ) {
                    (Some(a), Some(b), Some(c), Some(d)) => {
                        (a - c).abs() < 0.04 && (b - d).abs() < 0.04
                    }
                    _ => false,
                }
        };
        let current = places.iter().find(near);
        if current.is_none() && kind != "territorial_control" {
            continue;
        }
        let date = e["date"].as_str().unwrap_or("");
        for s in e["sources"].as_array().into_iter().flatten() {
            let excerpt = s["excerpt"].as_str().unwrap_or("");
            // "territoire de Masisi" is the territory, not the town of the same name.
            let ftoks = format!(" {} ", crate::text::tokens(excerpt).join(" "));
            let ptoks = crate::text::tokens(place).join(" ");
            if ftoks.contains(&format!(" territoire de {ptoks} "))
                || ftoks.contains(&format!(" territory of {ptoks} "))
            {
                continue;
            }
            let Some((controller, status, reason)) = suggest(kind, excerpt) else {
                continue;
            };
            // A town-centre entry needs the report to put the event at the town ("à Walungu"),
            // not just name the territory in a headline ("Walungu : … à Kaziba").
            if current.is_some_and(|c| c["name"].as_str().unwrap_or("").contains("(centre)"))
                && !crate::events::located(&crate::text::tokens(excerpt), place)
            {
                continue;
            }
            let name = current
                .and_then(|c| c["name"].as_str())
                .unwrap_or(place)
                .to_string();
            let id = format!(
                "{}:{}",
                e["id"].as_str().unwrap_or(""),
                s["article_id"].as_str().unwrap_or("")
            );
            if dismissed.contains(&id) {
                continue;
            }
            // Already reflected: the place's cited status is as new and says the same thing.
            if let Some(c) = current {
                let as_of = c["as_of"].as_str().unwrap_or("");
                let same = (c["controller"] == controller && c["status"] == status)
                    || (status == "contested" && c["status"] == "contested");
                if as_of.get(..10).unwrap_or(as_of) >= date || same {
                    continue;
                }
            }
            out.push(json!({"id":id,"place":name,"province":current.and_then(|c| c["province"].as_str()).or(e["province"].as_str()),"date":date,"kind":kind,
                "suggested":{"controller":controller,"status":status},"reason":reason,
                "current":current.map(|c| json!({"controller":c["controller"],"status":c["status"],"as_of":c["as_of"]})),
                "excerpt":excerpt,"source":{"title":s["title"],"publisher":s["source"],"url":s["url"],"published_at":s["published_at"],"article_id":s["article_id"]}}));
        }
    }
    // One suggestion per place and proposed status: the newest report, with how many agree.
    out.sort_by(|a, b| b["date"].as_str().cmp(&a["date"].as_str()));
    let mut merged: Vec<Value> = Vec::new();
    for mut x in out {
        if let Some(m) = merged.iter_mut().find(|m| {
            m["place"] == x["place"] && m["suggested"]["status"] == x["suggested"]["status"]
        }) {
            m["reports"] = json!(m["reports"].as_i64().unwrap_or(1) + 1);
            continue;
        }
        x["reports"] = json!(1);
        merged.push(x);
    }
    merged.truncate(60);
    Ok(merged)
}
pub async fn dismiss(State(app): State<App>, Json(b): Json<Value>) -> ApiResult<Json<Value>> {
    let id = b["id"]
        .as_str()
        .unwrap_or("")
        .chars()
        .take(200)
        .collect::<String>();
    if id.is_empty() {
        return Err(ApiError(StatusCode::BAD_REQUEST, "id is required".into()));
    }
    sqlx::query("INSERT OR IGNORE INTO control_dismissed VALUES(?,?,?)")
        .bind(&id)
        .bind(crate::authentication::actor().user_id)
        .bind(Utc::now().to_rfc3339())
        .execute(&app.db)
        .await?;
    Ok(Json(json!({"dismissed":id})))
}
pub async fn get(State(app): State<App>) -> ApiResult<Json<Value>> {
    let places = places(&app.db).await?;
    let public = crate::authentication::actor().role == "public";
    // Extracted claims of seizures from the last 45 days: leads for analysts, never shown as control.
    let since = (Utc::now() - chrono::Duration::days(45)).date_naive();
    let claims: Vec<Value> = crate::events::between(&app.db, since)
        .await?
        .into_iter()
        .filter(|e| e["kind"] == "territorial_control")
        .collect();
    let latest = places
        .iter()
        .filter_map(|p| p["as_of"].as_str())
        .max()
        .map(str::to_string);
    Ok(Json(json!({
        "places": places,
        "activity": activity(),
        "baseline": BASELINE["meta"],
        "claims": if public { json!([]) } else { json!(claims) },
        // The review queue and unconfirmed claims are for signed-in analysts only.
        "suggestions": if public { json!([]) } else { json!(suggestions(&app.db, &places).await?) },
        "controllers": CONTROLLERS,
        "statuses": STATUSES,
        "latest_as_of": latest,
        "notice": "Control is shown only where a cited source reports it, as of the date given. Front lines move faster than reporting; unshaded areas have no reported change and remain under government administration. Extracted 'seized/took control' reports are leads, not confirmed control. Armed-group activity zones are not control.",
        "notice_fr": "Le contrôle n’est affiché que lorsqu’une source citée le rapporte, à la date indiquée. Les lignes de front évoluent plus vite que les rapports ; les zones non colorées n’ont aucun changement signalé et restent sous administration gouvernementale. Les « prises de contrôle » extraites des articles sont des pistes, pas un contrôle confirmé. Les zones d’activité de groupes armés ne sont pas des zones de contrôle."
    })))
}
/// Analyst update: a new status for a place, which must cite an http(s) source.
pub async fn update(State(app): State<App>, Json(b): Json<Value>) -> ApiResult<Json<Value>> {
    let bad = |m: &str| ApiError(StatusCode::BAD_REQUEST, m.into());
    let s = |k: &str| b[k].as_str().map(str::trim).unwrap_or("").to_string();
    let (place, controller, status, as_of, url) = (
        s("place"),
        s("controller"),
        s("status"),
        s("as_of"),
        s("source_url"),
    );
    if place.is_empty() || place.chars().count() > 80 {
        return Err(bad("Place is required (max 80 characters)"));
    }
    if !CONTROLLERS.contains(&controller.as_str()) || !STATUSES.contains(&status.as_str()) {
        return Err(bad("Unknown controller or status"));
    }
    if chrono::NaiveDate::parse_from_str(&as_of, "%Y-%m-%d").is_err() {
        return Err(bad("as_of must be a date (YYYY-MM-DD)"));
    }
    match url::Url::parse(&url) {
        Ok(u) if ["http", "https"].contains(&u.scheme()) && u.host_str().is_some() => {}
        _ => {
            return Err(bad(
                "A source link (http or https) is required for every control update",
            ));
        }
    }
    let actor = crate::authentication::actor();
    let id = sqlx::query("INSERT INTO control_updates(place,province,controller,status,as_of,note,source_url,source_title,user_id,created_at) VALUES(?,?,?,?,?,?,?,?,?,?)")
        .bind(&place).bind(Some(s("province")).filter(|p| !p.is_empty())).bind(&controller).bind(&status).bind(&as_of)
        .bind(s("note").chars().take(400).collect::<String>()).bind(&url).bind(s("source_title").chars().take(200).collect::<String>())
        .bind(&actor.user_id).bind(Utc::now().to_rfc3339())
        .execute(&app.db).await?.last_insert_rowid();
    Ok(Json(json!({"id":id})))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn baseline_is_cited_and_valid() {
        let places = BASELINE["places"].as_array().unwrap();
        assert!(!places.is_empty());
        for p in places {
            let name = p["name"].as_str().unwrap();
            assert!(
                CONTROLLERS.contains(&p["controller"].as_str().unwrap()),
                "{name} controller"
            );
            assert!(
                STATUSES.contains(&p["status"].as_str().unwrap()),
                "{name} status"
            );
            assert!(
                p["as_of"].as_str().is_some_and(|d| d.len() >= 7),
                "{name} as_of"
            );
            let sources = p["sources"].as_array().unwrap();
            assert!(!sources.is_empty(), "{name} has a source");
            assert!(
                sources
                    .iter()
                    .all(|s| s["url"].as_str().is_some_and(|u| u.starts_with("http"))),
                "{name} source urls"
            );
        }
    }
    #[test]
    fn suggestions_read_the_direction_of_change() {
        assert_eq!(
            suggest(
                "territorial_control",
                "Le FRN s'est emparé de Kalembe mardi."
            )
            .unwrap()
            .0,
            "AFC/M23 (FRN)"
        );
        assert_eq!(
            suggest(
                "armed_clash",
                "Les FARDC ont repris Mubi après des combats."
            )
            .unwrap()
            .1,
            "recaptured"
        );
        assert_eq!(
            suggest(
                "territorial_control",
                "Retrait des rebelles du M23 de Sange."
            )
            .unwrap()
            .1,
            "withdrawn"
        );
        assert_eq!(
            suggest("armed_clash", "Des affrontements à Kibati.")
                .unwrap()
                .1,
            "contested"
        );
        assert!(suggest("displacement", "Des déplacés à Goma.").is_none());
    }
    #[test]
    fn major_towns_geocode() {
        for (n, p) in [
            ("Goma", "Nord-Kivu"),
            ("Bukavu", "Sud-Kivu"),
            ("Uvira", "Sud-Kivu"),
            ("Bunia", "Ituri"),
        ] {
            assert!(locate(n, Some(p)).is_some(), "{n}");
        }
    }
}
