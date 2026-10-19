//! Event log: dated incidents (what, where, when, who, how many) extracted from collected reports.
//!
//! Rule-based and free: each sentence of a report is scanned for French/English incident cues;
//! a sentence with a cue and a DRC place yields an event of that type at that place. The date is
//! the publication date, moved back for "hier/yesterday" or a named weekday, or set from an explicit
//! "le 12 octobre". Armed groups and death tolls are read from the same sentence. Reports of the
//! same type, place and day merge into one incident whose source count shows corroboration.
//! Extraction is automatic and unverified; every event keeps its source excerpts.
use crate::text::{self, Doc, Term};
use crate::{ApiResult, App};
use anyhow::Result;
use axum::{
    Json,
    extract::{Query, State},
};
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::sync::LazyLock;

/// Bump to re-extract every article after changing the rules.
const VERSION: i64 = 3;

/// (type, cues). A cue of one word matches as a prefix ("tué" → tués, tuée); phrases match exactly.
const TYPES: [(&str, &[&str]); 11] = [
    (
        "armed_clash",
        &[
            "affrontement",
            "affrontements",
            "combats",
            "accrochage",
            "accrochages",
            "echanges de tirs",
            "clashes",
            "fighting",
            "heurts armes",
            "offensive",
        ],
    ),
    (
        "attack_on_civilians",
        &[
            "massacre",
            "massacres",
            "tues",
            "tuees",
            "tue",
            "assassine",
            "assassines",
            "decapite",
            "egorge",
            "killed",
            "attaque contre des civils",
            "incursion",
            "raid",
            "attaque",
            "attaques",
            "attacked",
        ],
    ),
    (
        "explosive_or_drone",
        &[
            "drone",
            "drones",
            "bombardement",
            "bombardements",
            "obus",
            "explosion",
            "engin explosif",
            "shelling",
            "airstrike",
            "frappe aerienne",
            "bombe",
        ],
    ),
    (
        "abduction",
        &[
            "enlevement",
            "enlevements",
            "enleves",
            "enlevees",
            "kidnapping",
            "kidnappes",
            "otages",
            "abducted",
            "abduction",
        ],
    ),
    (
        "displacement",
        &[
            "deplaces",
            "deplacement massif",
            "fuient",
            "fuyant",
            "displaced",
            "refugies",
            "exode",
        ],
    ),
    (
        "territorial_control",
        &[
            "s emparent",
            "s est empare",
            "prise de",
            "pris le controle",
            "reprise de",
            "seized",
            "captured",
            "took control",
            "retrait des",
        ],
    ),
    (
        "protest_unrest",
        &[
            "manifestation",
            "manifestations",
            "manifestants",
            "emeute",
            "emeutes",
            "ville morte",
            "greve",
            "protest",
            "protesters",
            "riot",
        ],
    ),
    (
        "arrest_detention",
        &[
            "arrestation",
            "arrestations",
            "interpelle",
            "interpelles",
            "arrested",
            "detained",
        ],
    ),
    (
        "sexual_violence",
        &[
            "viol",
            "viols",
            "violences sexuelles",
            "rape",
            "sexual violence",
        ],
    ),
    (
        "health_outbreak",
        &[
            "ebola", "cholera", "mpox", "variole", "epidemie", "rougeole", "outbreak", "anthrax",
            "peste",
        ],
    ),
    (
        "disaster",
        &[
            "inondation",
            "inondations",
            "glissement de terrain",
            "eboulement",
            "naufrage",
            "incendie",
            "flood",
            "floods",
            "landslide",
            "shipwreck",
            "foudre",
            "lightning",
            "fire",
        ],
    ),
];
/// Armed actors read from the event sentence (acronyms match in capitals only).
const ACTORS: [&str; 23] = [
    "M23",
    "AFC",
    "FRN",
    "AFC/M23",
    "ADF",
    "CODECO",
    "FARDC",
    "Wazalendo",
    "Twirwaneho",
    "Zaïre",
    "MONUSCO",
    "RDF",
    "FDLR",
    "Mai-Mai",
    "Maï-Maï",
    "Nyatura",
    "CRP",
    "PNC",
    "Raia Mutomboki",
    "FDNB",
    "UPDF",
    "Kuluna",
    "SADC",
];
/// Each type with its cues as folded token sequences.
type Cues = Vec<(&'static str, Vec<Vec<String>>)>;
static CUES: LazyLock<Cues> = LazyLock::new(|| {
    TYPES
        .iter()
        .map(|(t, cues)| (*t, cues.iter().map(|c| text::tokens(c)).collect()))
        .collect()
});
static ACTOR_TERMS: LazyLock<Vec<(&'static str, Term)>> =
    LazyLock::new(|| ACTORS.iter().map(|a| (*a, text::term(a))).collect());
const CASUALTY: [&str; 12] = [
    "morts",
    "mort",
    "tues",
    "tuees",
    "victimes",
    "corps",
    "cadavres",
    "killed",
    "dead",
    "deaths",
    "bodies",
    "personnes tuees",
];
const NUMBER_WORDS: [(&str, i64); 20] = [
    ("deux", 2),
    ("trois", 3),
    ("quatre", 4),
    ("cinq", 5),
    ("six", 6),
    ("sept", 7),
    ("huit", 8),
    ("neuf", 9),
    ("dix", 10),
    ("onze", 11),
    ("douze", 12),
    ("quinze", 15),
    ("vingt", 20),
    ("trente", 30),
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("ten", 10),
    ("dozen", 12),
];
const MONTHS: [(&str, u32); 24] = [
    ("janvier", 1),
    ("fevrier", 2),
    ("mars", 3),
    ("avril", 4),
    ("mai", 5),
    ("juin", 6),
    ("juillet", 7),
    ("aout", 8),
    ("septembre", 9),
    ("octobre", 10),
    ("novembre", 11),
    ("decembre", 12),
    ("january", 1),
    ("february", 2),
    ("march", 3),
    ("april", 4),
    ("may", 5),
    ("june", 6),
    ("july", 7),
    ("august", 8),
    ("september", 9),
    ("october", 10),
    ("november", 11),
    ("december", 12),
];
const WEEKDAYS: [(&str, u32); 14] = [
    ("lundi", 0),
    ("mardi", 1),
    ("mercredi", 2),
    ("jeudi", 3),
    ("vendredi", 4),
    ("samedi", 5),
    ("dimanche", 6),
    ("monday", 0),
    ("tuesday", 1),
    ("wednesday", 2),
    ("thursday", 3),
    ("friday", 4),
    ("saturday", 5),
    ("sunday", 6),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub kind: &'static str,
    pub place: String,
    pub province: String,
    pub lat: f64,
    pub lon: f64,
    pub date: NaiveDate,
    pub actors: Vec<String>,
    pub fatalities: Option<i64>,
    pub excerpt: String,
}
fn has_cue(tokens: &[String], cue: &[String]) -> bool {
    if cue.len() == 1 {
        let c = &cue[0];
        // Single words also match plural/feminine endings; short cues must match exactly.
        return tokens.iter().any(|t| {
            t == c || (c.len() >= 5 && t.starts_with(c.as_str()) && t.len() <= c.len() + 2)
        });
    }
    tokens.windows(cue.len()).any(|w| w == cue)
}
fn number(tok: &str) -> Option<i64> {
    tok.parse::<i64>()
        .ok()
        .filter(|n| (1..=5000).contains(n))
        .or_else(|| {
            NUMBER_WORDS
                .iter()
                .find(|(w, _)| *w == tok)
                .map(|(_, n)| *n)
        })
}
/// Largest "N morts / N killed / N civils ont été tués" figure in the sentence (cue within five words).
fn fatalities(tokens: &[String]) -> Option<i64> {
    let mut best = None;
    for (i, t) in tokens.iter().enumerate() {
        if let Some(n) = number(t) {
            let after = &tokens[i + 1..tokens.len().min(i + 6)];
            let near = after.iter().any(|w| {
                CASUALTY
                    .iter()
                    .any(|c| w == c || (c.len() >= 5 && w.starts_with(c)))
            });
            if near {
                best = Some(best.map_or(n, |b: i64| b.max(n)));
            }
        }
    }
    best
}
/// Event date from the sentence relative to the publication date.
fn event_date(tokens: &[String], published: NaiveDate) -> NaiveDate {
    if tokens.iter().any(|t| t == "hier" || t == "yesterday") {
        return published - Duration::days(1);
    }
    for (i, t) in tokens.iter().enumerate() {
        if let Some((_, m)) = MONTHS.iter().find(|(name, _)| name == t) {
            let day = i
                .checked_sub(1)
                .and_then(|j| tokens[j].parse::<u32>().ok())
                .or_else(|| tokens.get(i + 1).and_then(|x| x.parse::<u32>().ok()));
            if let Some(d) = day.and_then(|d| NaiveDate::from_ymd_opt(published.year(), *m, d)) {
                let d = if d > published {
                    NaiveDate::from_ymd_opt(published.year() - 1, *m, d.day()).unwrap_or(d)
                } else {
                    d
                };
                if published - d <= Duration::days(60) {
                    return d;
                }
            }
        }
    }
    if let Some((_, wd)) = tokens
        .iter()
        .find_map(|t| WEEKDAYS.iter().find(|(w, _)| w == t))
    {
        let back = (published.weekday().num_days_from_monday() as i64 - *wd as i64).rem_euclid(7);
        return published - Duration::days(back);
    }
    published
}
fn sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in text.chars() {
        cur.push(c);
        if matches!(c, '.' | '!' | '?' | '\n') && cur.trim().len() > 3 {
            out.push(cur.trim().to_string());
            cur.clear();
        }
    }
    if cur.trim().len() > 3 {
        out.push(cur.trim().to_string());
    }
    out
}
/// Cues that a sentence recalls background rather than reporting an incident.
const CONTEXT: [&str; 5] = [
    "contexte",
    "consequences",
    "rappel",
    "historique",
    "conference",
];
/// Natural causes take precedence: "tués par la foudre" is a disaster, not an attack.
const NATURAL: [&str; 9] = [
    "foudre",
    "lightning",
    "inondation",
    "inondations",
    "glissement",
    "eboulement",
    "naufrage",
    "flood",
    "landslide",
];
const ANCHORS: [&str; 30] = [
    "aeroport de",
    "airport",
    "camp de",
    "site de",
    "base de",
    "axe",
    "route",
    "port de",
    "a",
    "au",
    "dans",
    "pres de",
    "near",
    "in",
    "at",
    "vers",
    "sur",
    "localite de",
    "village de",
    "territoire de",
    "ville de",
    "cite de",
    "secteur de",
    "chefferie de",
    "groupement de",
    "quartier",
    "commune de",
    "region de",
    "aux environs de",
    "outskirts of",
];
/// The place is where something happened ("à Goma", "territoire de Masisi", "Walikale :"), not a
/// metonym such as "Kinshasa et Kigali" for the two governments.
fn anchored(sentence: &str, toks: &[String], place: &str) -> bool {
    let p = text::tokens(place);
    if p.is_empty() {
        return false;
    }
    // Headline form "Walikale : …".
    let folded = text::fold(sentence);
    if folded
        .trim_start()
        .strip_prefix(&text::fold(place))
        .is_some_and(|rest| rest.trim_start().starts_with(':'))
    {
        return true;
    }
    ANCHORS.iter().any(|a| {
        let mut needle = text::tokens(a);
        needle.extend(p.iter().cloned());
        toks.windows(needle.len()).any(|w| w == needle.as_slice())
    })
}
/// Incidents in one report. At most one event per (type, place) per report.
pub fn extract(title: &str, content: &str, published: NaiveDate) -> Vec<Found> {
    let mut out: Vec<Found> = Vec::new();
    let body: Vec<String> = sentences(content).into_iter().take(60).collect();
    for s in std::iter::once(title.to_string()).chain(body) {
        if s.chars().count() > 900 {
            continue;
        }
        let toks = text::tokens(&s);
        let kinds: Vec<&str> = CUES
            .iter()
            .filter(|(_, cues)| cues.iter().any(|c| has_cue(&toks, c)))
            .map(|(k, _)| *k)
            .collect();
        if kinds.is_empty() {
            continue;
        }
        if toks.iter().any(|t| CONTEXT.contains(&t.as_str())) {
            continue;
        }
        let places = crate::intelligence::place_tuples(&s);
        let Some((place, province, lat, lon)) = places
            .into_iter()
            .find(|(name, ..)| anchored(&s, &toks, name))
        else {
            continue;
        };
        let doc = Doc::new(&s);
        let actors: Vec<String> = ACTOR_TERMS
            .iter()
            .filter(|(_, t)| doc.has_term(t))
            .map(|(a, _)| a.to_string())
            .collect();
        let mut actors = actors;
        if actors.iter().any(|a| a == "AFC/M23") {
            actors.retain(|a| a != "AFC" && a != "M23");
        }
        let date = event_date(&toks, published);
        let deaths = fatalities(&toks);
        // An attack that kills, or a clash, is one incident: prefer the most specific type per sentence.
        let primary = [
            "attack_on_civilians",
            "armed_clash",
            "explosive_or_drone",
            "abduction",
            "sexual_violence",
            "territorial_control",
            "displacement",
            "health_outbreak",
            "disaster",
            "protest_unrest",
            "arrest_detention",
        ]
        .into_iter()
        .find(|k| kinds.contains(k))
        .unwrap();
        let primary =
            if kinds.contains(&"disaster") && toks.iter().any(|t| NATURAL.contains(&t.as_str())) {
                "disaster"
            } else {
                primary
            };
        // Seizing ground needs an armed actor in the sentence ("prise de" is also "prise de position").
        if primary == "territorial_control" && actors.is_empty() {
            continue;
        }
        // "attaque" alone also describes political attacks; require a violent context for it.
        if primary == "attack_on_civilians"
            && deaths.is_none()
            && actors.is_empty()
            && !toks.iter().any(|t| {
                [
                    "civils",
                    "civilians",
                    "massacre",
                    "tues",
                    "killed",
                    "village",
                    "localite",
                ]
                .contains(&t.as_str())
            })
        {
            continue;
        }
        if out.iter().any(|e| e.kind == primary && e.place == place) {
            if let Some(e) = out
                .iter_mut()
                .find(|e| e.kind == primary && e.place == place)
            {
                e.fatalities = e.fatalities.max(deaths);
                for a in &actors {
                    if !e.actors.contains(a) {
                        e.actors.push(a.clone());
                    }
                }
            }
            continue;
        }
        out.push(Found {
            kind: primary,
            place,
            province,
            lat,
            lon,
            date,
            actors,
            fatalities: deaths,
            excerpt: s.chars().take(360).collect(),
        });
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Storage

pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS events(id TEXT PRIMARY KEY, kind TEXT NOT NULL, date TEXT NOT NULL, place TEXT NOT NULL, province TEXT NOT NULL, lat REAL NOT NULL, lon REAL NOT NULL, actors TEXT NOT NULL, fatalities INTEGER, sources TEXT NOT NULL, updated_at TEXT NOT NULL);
         CREATE INDEX IF NOT EXISTS events_date ON events(date);
         CREATE TABLE IF NOT EXISTS event_articles(article_id TEXT PRIMARY KEY, version INTEGER NOT NULL, processed_at TEXT NOT NULL);",
    )
    .execute(db)
    .await?;
    Ok(())
}
fn event_id(kind: &str, place: &str, date: NaiveDate) -> String {
    crate::extract::digest(format!("{kind}|{}|{date}", text::fold(place)).as_bytes())[..24]
        .to_string()
}
/// Actors and death toll of an incident, recomputed from its current sources: the union of the
/// actors and the highest toll any source reported (each source keeps its own figure).
fn merged(sources: &[Value]) -> (Vec<String>, Option<i64>) {
    let mut actors: Vec<String> = Vec::new();
    for a in sources
        .iter()
        .flat_map(|s| s["actors"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
    {
        if !actors.iter().any(|x| x == a) {
            actors.push(a.to_string());
        }
    }
    if actors.iter().any(|a| a == "AFC/M23") {
        actors.retain(|a| a != "AFC" && a != "M23");
    }
    let deaths = sources
        .iter()
        .filter_map(|s| s["fatalities"].as_i64())
        .max();
    (actors, deaths)
}
/// Processes up to `limit` articles not yet extracted with the current rules.
pub async fn process(db: &SqlitePool, limit: i64) -> Result<usize> {
    let rows = sqlx::query("SELECT a.id, a.title, a.content, a.url, a.published_at, json_extract(a.data,'$.source') AS source FROM articles a LEFT JOIN event_articles e ON e.article_id=a.id WHERE (e.version IS NULL OR e.version<?) AND a.published_at IS NOT NULL ORDER BY a.published_at DESC LIMIT ?")
        .bind(VERSION).bind(limit).fetch_all(db).await?;
    let mut n = 0;
    for r in &rows {
        let id: String = r.get("id");
        let published: String = r.get("published_at");
        let Some(date) = published
            .get(..10)
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        else {
            continue;
        };
        // Re-extraction replaces this article's previous contributions.
        for e in sqlx::query("SELECT id, sources FROM events WHERE sources LIKE ?")
            .bind(format!("%\"{id}\"%"))
            .fetch_all(db)
            .await?
        {
            let mut sources: Vec<Value> =
                serde_json::from_str(&e.get::<String, _>("sources")).unwrap_or_default();
            sources.retain(|s| s["article_id"] != id.as_str());
            if sources.is_empty() {
                sqlx::query("DELETE FROM events WHERE id=?")
                    .bind(e.get::<String, _>("id"))
                    .execute(db)
                    .await?;
            } else {
                let (actors, deaths) = merged(&sources);
                sqlx::query("UPDATE events SET sources=?, actors=?, fatalities=? WHERE id=?")
                    .bind(json!(sources).to_string())
                    .bind(json!(actors).to_string())
                    .bind(deaths)
                    .bind(e.get::<String, _>("id"))
                    .execute(db)
                    .await?;
            }
        }
        for f in extract(
            &r.get::<String, _>("title"),
            &r.get::<String, _>("content"),
            date,
        ) {
            let eid = event_id(f.kind, &f.place, f.date);
            let source = json!({"article_id":id,"title":r.get::<String,_>("title"),"source":r.get::<Option<String>,_>("source"),"url":r.get::<String,_>("url"),"published_at":published,"excerpt":f.excerpt,"fatalities":f.fatalities,"actors":f.actors});
            let existing: Option<String> =
                sqlx::query_scalar("SELECT sources FROM events WHERE id=?")
                    .bind(&eid)
                    .fetch_optional(db)
                    .await?;
            let mut sources: Vec<Value> = existing
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            sources.retain(|s| s["article_id"] != id.as_str());
            sources.push(source);
            let (actors, deaths) = merged(&sources);
            sqlx::query("INSERT INTO events(id,kind,date,place,province,lat,lon,actors,fatalities,sources,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET actors=excluded.actors, fatalities=excluded.fatalities, sources=excluded.sources, updated_at=excluded.updated_at")
                .bind(&eid).bind(f.kind).bind(f.date.to_string()).bind(&f.place).bind(&f.province).bind(f.lat).bind(f.lon)
                .bind(json!(actors).to_string()).bind(deaths).bind(json!(sources).to_string()).bind(Utc::now().to_rfc3339())
                .execute(db).await?;
        }
        sqlx::query("INSERT INTO event_articles VALUES(?,?,?) ON CONFLICT(article_id) DO UPDATE SET version=excluded.version, processed_at=excluded.processed_at")
            .bind(&id).bind(VERSION).bind(Utc::now().to_rfc3339()).execute(db).await?;
        n += 1;
    }
    Ok(n)
}
pub async fn worker(db: SqlitePool) -> Result<()> {
    loop {
        match process(&db, 200).await {
            Ok(n) if n > 0 => tracing::info!(articles = n, "events extracted"),
            Err(e) => tracing::warn!(error = %e, "event extraction failed"),
            _ => {}
        }
        tokio::time::sleep(std::time::Duration::from_secs(120)).await;
    }
}
pub fn row_json(r: &sqlx::sqlite::SqliteRow) -> Value {
    let sources: Vec<Value> =
        serde_json::from_str(&r.get::<String, _>("sources")).unwrap_or_default();
    let outlets = sources
        .iter()
        .filter_map(|s| s["source"].as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    json!({"id":r.get::<String,_>("id"),"kind":r.get::<String,_>("kind"),"date":r.get::<String,_>("date"),"place":r.get::<String,_>("place"),"province":r.get::<String,_>("province"),"lat":r.get::<f64,_>("lat"),"lon":r.get::<f64,_>("lon"),"actors":serde_json::from_str::<Value>(&r.get::<String,_>("actors")).unwrap_or_default(),"fatalities":r.get::<Option<i64>,_>("fatalities"),"sources":sources,"reports":sources.len(),"outlets":outlets})
}
pub async fn between(db: &SqlitePool, from: NaiveDate) -> Result<Vec<Value>> {
    Ok(
        sqlx::query("SELECT * FROM events WHERE date>=? ORDER BY date DESC, id")
            .bind(from.to_string())
            .fetch_all(db)
            .await?
            .iter()
            .map(row_json)
            .collect(),
    )
}
#[derive(Deserialize)]
pub struct EventQuery {
    days: Option<i64>,
}
pub async fn list(State(app): State<App>, Query(q): Query<EventQuery>) -> ApiResult<Json<Value>> {
    let days = q.days.unwrap_or(30).clamp(1, 400);
    let from = Utc::now().date_naive() - Duration::days(days);
    let items = between(&app.db, from).await?;
    let processed: i64 = sqlx::query_scalar("SELECT count(*) FROM event_articles WHERE version=?")
        .bind(VERSION)
        .fetch_one(&app.db)
        .await?;
    Ok(Json(
        json!({"items":items,"days":days,"articles_processed":processed,"types":TYPES.iter().map(|(t,_)|*t).collect::<Vec<_>>(),
        "notice":"Incidents are extracted automatically from report text (cue words, gazetteer places, dates relative to publication). They are unverified: check the source excerpts. Reports of the same type, place and day are merged; the death toll is the highest any source reported."}),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }
    #[test]
    fn extracts_type_place_actor_toll_and_date() {
        let e = extract(
            "Djugu : 11 morts dans une nouvelle attaque attribuée à la CODECO à Bunia",
            "",
            d("2026-10-14"),
        );
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].kind, "attack_on_civilians");
        assert_eq!(e[0].place, "Djugu");
        assert_eq!(e[0].fatalities, Some(11));
        assert_eq!(e[0].actors, vec!["CODECO"]);
        let c = extract(
            "Combats",
            "Hier, des affrontements ont opposé le M23 aux Wazalendo à Masisi.",
            d("2026-10-14"),
        );
        assert_eq!(c[0].kind, "armed_clash");
        assert_eq!(c[0].date, d("2026-10-13"));
        assert_eq!(c[0].actors, vec!["M23", "Wazalendo"]);
        let w = extract(
            "",
            "Dimanche, un drone a frappé l'aéroport de Kisangani.",
            d("2026-10-15"),
        );
        assert_eq!(
            (w[0].kind, w[0].date),
            ("explosive_or_drone", d("2026-10-11")),
            "15 Oct 2026 is a Thursday"
        );
        let x = extract(
            "",
            "Le 9 octobre, au moins vingt civils ont été tués à Beni.",
            d("2026-10-14"),
        );
        assert_eq!((x[0].date, x[0].fatalities), (d("2026-10-09"), Some(20)));
    }
    #[test]
    fn needs_a_place_and_violent_context() {
        assert!(
            extract("Des affrontements ont eu lieu", "", d("2026-10-14")).is_empty(),
            "no place"
        );
        assert!(
            extract(
                "L'opposition attaque le gouvernement à Kinshasa",
                "",
                d("2026-10-14")
            )
            .is_empty(),
            "political 'attaque'"
        );
        assert!(
            extract("Choléra : 30 cas à Kalemie", "", d("2026-10-14"))
                .iter()
                .any(|e| e.kind == "health_outbreak")
        );
        assert!(extract("La m23 minuscule", "", d("2026-10-14")).is_empty());
    }
    #[test]
    fn background_metonyms_and_natural_causes() {
        assert!(
            extract(
                "",
                "Dans un contexte marqué par les combats à Goma, MSF a rouvert un centre.",
                d("2026-10-14")
            )
            .is_empty(),
            "background"
        );
        assert!(
            extract(
                "",
                "Kinshasa et Kigali s'accusent mutuellement après des affrontements.",
                d("2026-10-14")
            )
            .is_empty(),
            "capital as metonym"
        );
        let l = extract(
            "Walikale : deux enfants tués par la foudre à Mubi",
            "",
            d("2026-10-14"),
        );
        assert_eq!(l[0].kind, "disaster");
        assert!(
            extract(
                "",
                "Les transporteurs privés occupent une place prépondérante à Kinshasa.",
                d("2026-10-14")
            )
            .is_empty()
        );
        let a = extract(
            "",
            "Les combats entre l'AFC/M23 et les FARDC se poursuivent à Masisi.",
            d("2026-10-14"),
        );
        assert_eq!(a[0].actors, vec!["AFC/M23", "FARDC"]);
    }
    #[test]
    fn same_incident_merges_into_one_event_id() {
        assert_eq!(
            event_id("armed_clash", "Masisi", d("2026-10-13")),
            event_id("armed_clash", "MASISI", d("2026-10-13"))
        );
        assert_ne!(
            event_id("armed_clash", "Masisi", d("2026-10-13")),
            event_id("armed_clash", "Masisi", d("2026-10-14"))
        );
    }
}
