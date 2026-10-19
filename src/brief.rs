//! Daily and weekly briefs from the collection, as Word (.docx), print-ready HTML (save as PDF)
//! or JSON. Every item cites its source in a numbered list; nothing is generated beyond the stored
//! reports, their AI summaries where an analyst ran one, the event log and the flight picture.
use crate::{ApiError, ApiResult, App};
use anyhow::Result;
use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use std::collections::BTreeMap;
use std::io::Write;

pub const DOCX_MIME: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

fn tr(lang: &str, en: &str, fr: &str) -> String {
    if lang == "fr" { fr.into() } else { en.into() }
}
fn event_label(kind: &str, lang: &str) -> String {
    let (en, fr) = match kind {
        "armed_clash" => ("Armed clashes", "Affrontements armés"),
        "attack_on_civilians" => ("Attacks on civilians", "Attaques contre des civils"),
        "explosive_or_drone" => (
            "Drones, shelling, explosions",
            "Drones, bombardements, explosions",
        ),
        "abduction" => ("Abductions", "Enlèvements"),
        "displacement" => ("Displacement", "Déplacements"),
        "territorial_control" => ("Territorial control", "Contrôle territorial"),
        "protest_unrest" => ("Protests and unrest", "Manifestations et troubles"),
        "arrest_detention" => ("Arrests and detentions", "Arrestations et détentions"),
        "sexual_violence" => ("Sexual violence", "Violences sexuelles"),
        "health_outbreak" => ("Disease outbreaks", "Épidémies"),
        "disaster" => ("Disasters", "Catastrophes"),
        _ => (kind, kind),
    };
    tr(lang, en, fr)
}
/// Everything a brief contains, in one structure shared by every format.
pub async fn collect(db: &SqlitePool, period: &str, lang: &str) -> Result<Value> {
    let weekly = period == "weekly";
    let now = Utc::now();
    let from = now - Duration::days(if weekly { 7 } else { 1 });
    let app = App {
        db: db.clone(),
        token: None,
    };
    let query = serde_json::from_value(json!({"limit":500}))?;
    let feed = crate::intelligence::feed(axum::extract::State(app), axum::extract::Query(query))
        .await
        .map_err(|e| anyhow::anyhow!(e.1))?
        .0;
    let in_window = |a: &&Value| {
        a["published_at"]
            .as_str()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .is_some_and(|t| t >= from)
    };
    let mut reports: Vec<&Value> = feed["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(in_window)
        .collect();
    let total = reports.len();
    let outlets = reports
        .iter()
        .filter_map(|a| a["source"].as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let high = reports
        .iter()
        .filter(|a| a["rank"]["band"] == "high")
        .count();
    let mut provinces: BTreeMap<String, usize> = BTreeMap::new();
    for a in &reports {
        let mut seen = std::collections::BTreeSet::new();
        for p in a["places"].as_array().into_iter().flatten() {
            if let Some(pv) = p["province"].as_str()
                && seen.insert(pv)
            {
                *provinces.entry(pv.to_string()).or_default() += 1;
            }
        }
    }
    let mut provinces: Vec<(String, usize)> = provinces.into_iter().collect();
    provinces.sort_by_key(|p| std::cmp::Reverse(p.1));
    provinces.truncate(8);
    reports.sort_by(|a, b| {
        b["rank"]["score"]
            .as_u64()
            .cmp(&a["rank"]["score"].as_u64())
            .then(b["published_at"].as_str().cmp(&a["published_at"].as_str()))
    });
    reports.truncate(if weekly { 20 } else { 12 });
    let mut sources: Vec<Value> = Vec::new();
    let mut cite = |title: &Value, source: &Value, url: &Value, date: &Value| -> usize {
        if let Some(i) = sources.iter().position(|s| s["url"] == *url) {
            return i + 1;
        }
        sources.push(json!({"title":title,"source":source,"url":url,"date":date}));
        sources.len()
    };
    let summary_key = if lang == "fr" {
        "summary_fr"
    } else {
        "summary_en"
    };
    let top: Vec<Value> = reports.iter().map(|a| {
        let ai = a["brief"]["data"][summary_key].as_str().filter(|s| !s.is_empty());
        let text = ai.or(a["description"].as_str()).unwrap_or("").chars().take(600).collect::<String>();
        let n = cite(&a["title"], &a["source"], &a["url"], &a["published_at"]);
        json!({"title":a["title"],"source":a["source"],"date":a["published_at"],"score":a["rank"]["score"],"band":a["rank"]["band"],"places":a["places"].as_array().map(|p| p.iter().filter_map(|x| x["name"].as_str()).take(4).collect::<Vec<_>>()),"text":text,"ai_summary":ai.is_some(),"cite":n})
    }).collect();
    let events = crate::events::between(
        db,
        (now - Duration::days(if weekly { 7 } else { 2 })).date_naive(),
    )
    .await?;
    let mut by_type: BTreeMap<String, (usize, i64)> = BTreeMap::new();
    for e in &events {
        let entry = by_type
            .entry(e["kind"].as_str().unwrap_or("").to_string())
            .or_default();
        entry.0 += 1;
        entry.1 += e["fatalities"].as_i64().unwrap_or(0);
    }
    let mut key_events: Vec<&Value> = events.iter().collect();
    key_events.sort_by(|a, b| {
        (b["fatalities"].as_i64().unwrap_or(0), b["outlets"].as_u64())
            .cmp(&(a["fatalities"].as_i64().unwrap_or(0), a["outlets"].as_u64()))
    });
    let incidents: Vec<Value> = key_events.into_iter().take(10).map(|e| {
        let cites: Vec<usize> = e["sources"].as_array().into_iter().flatten().take(3).map(|s| cite(&s["title"], &s["source"], &s["url"], &s["published_at"])).collect();
        json!({"date":e["date"],"kind":event_label(e["kind"].as_str().unwrap_or(""), lang),"place":e["place"],"province":e["province"],"actors":e["actors"],"fatalities":e["fatalities"],"outlets":e["outlets"],"cites":cites})
    }).collect();
    let flights: String = sqlx::query_scalar("SELECT data FROM flight_snapshot WHERE id=1")
        .fetch_optional(db)
        .await?
        .unwrap_or_default();
    let flights: Value = serde_json::from_str(&flights).unwrap_or_default();
    let mut air: BTreeMap<String, usize> = BTreeMap::new();
    for f in flights["items"].as_array().into_iter().flatten() {
        *air.entry(f["category"].as_str().unwrap_or("unknown").to_string())
            .or_default() += 1;
    }
    let title = if weekly {
        tr(lang, "Weekly brief", "Synthèse hebdomadaire")
    } else {
        tr(lang, "Daily brief", "Synthèse quotidienne")
    };
    let headline = format!(
        "{} · {} {} · {} {} · {} {}",
        title,
        total,
        tr(lang, "reports", "articles"),
        high,
        tr(lang, "high priority", "prioritaires"),
        events.len(),
        tr(lang, "incidents", "incidents")
    );
    Ok(json!({
        "title":title,"headline":headline,"period":period,"lang":lang,"generated_at":now.to_rfc3339(),"from":from.to_rfc3339(),"to":now.to_rfc3339(),
        "figures":{"reports":total,"outlets":outlets,"high_priority":high,"incidents":events.len(),"fatalities_reported":by_type.values().map(|v| v.1).sum::<i64>()},
        "provinces":provinces,"top":top,"incidents":incidents,
        "event_types":by_type.iter().map(|(k,(n,d))| json!({"kind":event_label(k, lang),"count":n,"fatalities":d})).collect::<Vec<_>>(),
        "air":air,"air_checked_at":flights["checked_at"],"sources":sources,
        "caveat":tr(lang,"Compiled automatically from open sources collected by Congo Signal. Reports are single-source and unverified unless stated; incidents are extracted automatically from report text; death tolls are the highest reported by any source. Check the cited sources before relying on any item.","Compilé automatiquement à partir des sources ouvertes collectées par Congo Signal. Les articles sont à source unique et non vérifiés sauf mention ; les incidents sont extraits automatiquement des textes ; les bilans sont les plus élevés rapportés par une source. Vérifiez les sources citées avant toute utilisation.")
    }))
}
pub async fn summary(db: &SqlitePool, period: &str) -> Result<Value> {
    collect(db, period, "en").await
}

// ---------------------------------------------------------------------------------------------
// HTML (print / PDF)

fn h(s: &str) -> String {
    crate::notify::esc(s)
}
fn date_short(v: &Value) -> String {
    v.as_str()
        .map(|s| s.chars().take(10).collect())
        .unwrap_or_default()
}
pub fn html(b: &Value, print: bool) -> String {
    let lang = b["lang"].as_str().unwrap_or("en");
    let mut o = format!("<!doctype html><html lang=\"{lang}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Congo Signal · {}</title><style>
body{{font:14px/1.55 Georgia,'Times New Roman',serif;color:#111;max-width:780px;margin:32px auto;padding:0 20px;background:#fff}}h1{{font:600 26px system-ui,sans-serif;margin:0}}h2{{font:600 15px system-ui,sans-serif;text-transform:uppercase;letter-spacing:.08em;border-bottom:2px solid #111;padding-bottom:4px;margin-top:28px}}
.meta{{font:12px system-ui,sans-serif;color:#555}}.figs{{display:flex;gap:18px;flex-wrap:wrap;font:13px system-ui,sans-serif;margin:16px 0}}.figs b{{display:block;font-size:22px}}
.item{{margin:12px 0}}.item h3{{font:600 14px system-ui,sans-serif;margin:0}}.item small,.src small{{font:11.5px system-ui,sans-serif;color:#555}}sup{{font:600 10px system-ui,sans-serif;color:#555}}
table{{border-collapse:collapse;width:100%;font:12.5px system-ui,sans-serif}}td,th{{border-bottom:1px solid #ddd;padding:5px 6px;text-align:left;vertical-align:top}}ol.src{{font-size:12px;padding-left:22px}}ol.src a{{color:#114;word-break:break-all}}
.caveat{{font:11.5px system-ui,sans-serif;color:#555;border-left:3px solid #aaa;padding:6px 10px;margin-top:24px}}.bar{{position:sticky;top:0;background:#fff;padding:8px 0;border-bottom:1px solid #ddd;font:13px system-ui,sans-serif}}@media print{{.bar{{display:none}}body{{margin:0}}a{{color:#000}}}}
</style></head><body>", h(b["title"].as_str().unwrap_or("")));
    o.push_str(&format!(
        "<div class=\"bar\"><button onclick=\"print()\">{}</button></div>",
        tr(lang, "Print / save as PDF", "Imprimer / enregistrer en PDF")
    ));
    o.push_str(&format!(
        "<h1>Congo Signal · {}</h1><p class=\"meta\">{} {} → {} (UTC) · {} {}</p>",
        h(b["title"].as_str().unwrap_or("")),
        tr(lang, "Period", "Période"),
        h(&b["from"].as_str().unwrap_or("").replace('T', " ")
            [..16.min(b["from"].as_str().unwrap_or("").len())]),
        h(&b["to"].as_str().unwrap_or("").replace('T', " ")
            [..16.min(b["to"].as_str().unwrap_or("").len())]),
        tr(lang, "generated", "généré le"),
        h(&date_short(&b["generated_at"]))
    ));
    let f = &b["figures"];
    o.push_str(&format!("<div class=\"figs\"><span><b>{}</b>{}</span><span><b>{}</b>{}</span><span><b>{}</b>{}</span><span><b>{}</b>{}</span><span><b>{}</b>{}</span></div>",
        f["reports"], tr(lang, "reports", "articles"), f["outlets"], tr(lang, "outlets", "médias"), f["high_priority"], tr(lang, "high priority", "prioritaires"), f["incidents"], tr(lang, "incidents", "incidents"), f["fatalities_reported"], tr(lang, "deaths reported", "morts rapportés")));
    if let Some(p) = b["provinces"].as_array().filter(|p| !p.is_empty()) {
        o.push_str(&format!(
            "<p class=\"meta\">{}: {}</p>",
            tr(lang, "Most-reported provinces", "Provinces les plus citées"),
            p.iter()
                .map(|x| format!("{} ({})", h(x[0].as_str().unwrap_or("")), x[1]))
                .collect::<Vec<_>>()
                .join(" · ")
        ));
    }
    o.push_str(&format!(
        "<h2>{}</h2>",
        tr(lang, "Priority reports", "Articles prioritaires")
    ));
    for t in b["top"].as_array().into_iter().flatten() {
        o.push_str(&format!("<div class=\"item\"><h3>{} <sup>[{}]</sup></h3><small>{} · {} · {} {}{}</small><p>{}</p></div>",
            h(t["title"].as_str().unwrap_or("")), t["cite"], h(t["source"].as_str().unwrap_or("")), h(&date_short(&t["date"])), tr(lang, "priority", "priorité"), t["score"],
            t["places"].as_array().filter(|p| !p.is_empty()).map(|p| format!(" · {}", p.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "))).unwrap_or_default(),
            h(t["text"].as_str().unwrap_or(""))));
    }
    if let Some(inc) = b["incidents"].as_array().filter(|i| !i.is_empty()) {
        o.push_str(&format!("<h2>{}</h2><table><tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>", tr(lang, "Key incidents (extracted)", "Incidents clés (extraits)"), tr(lang, "Date", "Date"), tr(lang, "Type", "Type"), tr(lang, "Place", "Lieu"), tr(lang, "Actors", "Acteurs"), tr(lang, "Deaths", "Morts"), tr(lang, "Sources", "Sources")));
        for e in inc {
            o.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{} · {}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                h(&date_short(&e["date"])),
                h(e["kind"].as_str().unwrap_or("")),
                h(e["place"].as_str().unwrap_or("")),
                h(e["province"].as_str().unwrap_or("")),
                h(&e["actors"]
                    .as_array()
                    .map(|a| a
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", "))
                    .unwrap_or_default()),
                e["fatalities"]
                    .as_i64()
                    .map(|n| n.to_string())
                    .unwrap_or("—".into()),
                e["cites"]
                    .as_array()
                    .map(|c| c.iter().map(|n| format!("[{n}]")).collect::<String>())
                    .unwrap_or_default()
            ));
        }
        o.push_str("</table>");
    }
    if let Some(t) = b["event_types"].as_array().filter(|t| !t.is_empty()) {
        o.push_str(&format!(
            "<p class=\"meta\">{}: {}</p>",
            tr(lang, "Incidents by type", "Incidents par type"),
            t.iter()
                .map(|x| format!("{} {}", h(x["kind"].as_str().unwrap_or("")), x["count"]))
                .collect::<Vec<_>>()
                .join(" · ")
        ));
    }
    if let Some(air) = b["air"].as_object().filter(|a| !a.is_empty()) {
        o.push_str(&format!("<h2>{}</h2><p>{} {}. <small class=\"meta\">{}</small></p>", tr(lang, "Air picture", "Situation aérienne"), air.iter().map(|(k, v)| format!("{v} {k}")).collect::<Vec<_>>().join(", "), tr(lang, "aircraft visible at the last scan", "aéronefs visibles au dernier scan"), tr(lang, "Community ADS-B coverage over the DRC is sparse; most traffic is not visible.", "La couverture ADS-B communautaire est faible au-dessus de la RDC ; la plupart du trafic n’est pas visible.")));
    }
    o.push_str(&format!(
        "<h2>{}</h2><ol class=\"src\">",
        tr(lang, "Sources", "Sources")
    ));
    for s in b["sources"].as_array().into_iter().flatten() {
        let url = s["url"].as_str().unwrap_or("");
        o.push_str(&format!(
            "<li>{} — {}, {}<br><a href=\"{}\">{}</a></li>",
            h(s["title"].as_str().unwrap_or("")),
            h(s["source"].as_str().unwrap_or("")),
            h(&date_short(&s["date"])),
            h(url),
            h(url)
        ));
    }
    o.push_str(&format!(
        "</ol><p class=\"caveat\">{}</p>",
        h(b["caveat"].as_str().unwrap_or(""))
    ));
    if print {
        o.push_str("<script>addEventListener('load',()=>setTimeout(print,300))</script>");
    }
    o.push_str("</body></html>");
    o
}

// ---------------------------------------------------------------------------------------------
// Word (.docx): a minimal WordprocessingML package with styles and hyperlinks.

fn x(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}
struct Doc {
    body: String,
    links: Vec<String>,
}
impl Doc {
    fn run(text: &str, bold: bool, size: Option<u32>, color: Option<&str>) -> String {
        let mut rpr = String::new();
        if bold {
            rpr.push_str("<w:b/>");
        }
        if let Some(c) = color {
            rpr.push_str(&format!("<w:color w:val=\"{c}\"/>"));
        }
        if let Some(s) = size {
            rpr.push_str(&format!("<w:sz w:val=\"{s}\"/>"));
        }
        format!(
            "<w:r>{}<w:t xml:space=\"preserve\">{}</w:t></w:r>",
            if rpr.is_empty() {
                String::new()
            } else {
                format!("<w:rPr>{rpr}</w:rPr>")
            },
            x(text)
        )
    }
    fn para(&mut self, style: Option<&str>, runs: &[String]) {
        let ppr = style
            .map(|s| format!("<w:pPr><w:pStyle w:val=\"{s}\"/></w:pPr>"))
            .unwrap_or_default();
        self.body
            .push_str(&format!("<w:p>{ppr}{}</w:p>", runs.concat()));
    }
    fn link(&mut self, url: &str, text: &str) -> String {
        self.links.push(url.to_string());
        format!(
            "<w:hyperlink r:id=\"rIdL{}\"><w:r><w:rPr><w:rStyle w:val=\"Hyperlink\"/><w:color w:val=\"1F4E79\"/><w:u w:val=\"single\"/><w:sz w:val=\"18\"/></w:rPr><w:t xml:space=\"preserve\">{}</w:t></w:r></w:hyperlink>",
            self.links.len(),
            x(text)
        )
    }
}
pub fn docx_bytes(b: &Value) -> Result<Vec<u8>> {
    let lang = b["lang"].as_str().unwrap_or("en");
    let mut d = Doc {
        body: String::new(),
        links: Vec::new(),
    };
    let r = |t: &str| Doc::run(t, false, None, None);
    let small = |t: &str| Doc::run(t, false, Some(18), Some("555555"));
    d.para(
        Some("Title"),
        &[r(&format!(
            "Congo Signal · {}",
            b["title"].as_str().unwrap_or("")
        ))],
    );
    d.para(
        None,
        &[small(&format!(
            "{} {} → {} UTC",
            tr(lang, "Period", "Période"),
            b["from"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(16)
                .collect::<String>()
                .replace('T', " "),
            b["to"]
                .as_str()
                .unwrap_or("")
                .chars()
                .take(16)
                .collect::<String>()
                .replace('T', " ")
        ))],
    );
    let f = &b["figures"];
    d.para(
        None,
        &[Doc::run(
            &format!(
                "{} {} · {} {} · {} {} · {} {} · {} {}",
                f["reports"],
                tr(lang, "reports", "articles"),
                f["outlets"],
                tr(lang, "outlets", "médias"),
                f["high_priority"],
                tr(lang, "high priority", "prioritaires"),
                f["incidents"],
                tr(lang, "incidents", "incidents"),
                f["fatalities_reported"],
                tr(lang, "deaths reported", "morts rapportés")
            ),
            true,
            None,
            None,
        )],
    );
    if let Some(p) = b["provinces"].as_array().filter(|p| !p.is_empty()) {
        d.para(
            None,
            &[small(&format!(
                "{}: {}",
                tr(lang, "Most-reported provinces", "Provinces les plus citées"),
                p.iter()
                    .map(|x| format!("{} ({})", x[0].as_str().unwrap_or(""), x[1]))
                    .collect::<Vec<_>>()
                    .join(" · ")
            ))],
        );
    }
    d.para(
        Some("Heading1"),
        &[r(&tr(lang, "Priority reports", "Articles prioritaires"))],
    );
    for t in b["top"].as_array().into_iter().flatten() {
        d.para(
            Some("Heading2"),
            &[
                r(t["title"].as_str().unwrap_or("")),
                Doc::run(
                    &format!(" [{}]", t["cite"]),
                    false,
                    Some(16),
                    Some("555555"),
                ),
            ],
        );
        let places = t["places"]
            .as_array()
            .filter(|p| !p.is_empty())
            .map(|p| {
                format!(
                    " · {}",
                    p.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .unwrap_or_default();
        d.para(
            None,
            &[small(&format!(
                "{} · {} · {} {}{places}",
                t["source"].as_str().unwrap_or(""),
                date_short(&t["date"]),
                tr(lang, "priority", "priorité"),
                t["score"]
            ))],
        );
        if let Some(text) = t["text"].as_str().filter(|s| !s.is_empty()) {
            d.para(None, &[r(text)]);
        }
    }
    if let Some(inc) = b["incidents"].as_array().filter(|i| !i.is_empty()) {
        d.para(
            Some("Heading1"),
            &[r(&tr(
                lang,
                "Key incidents (extracted)",
                "Incidents clés (extraits)",
            ))],
        );
        for e in inc {
            let actors = e["actors"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            let deaths = e["fatalities"]
                .as_i64()
                .map(|n| format!(" · {n} {}", tr(lang, "deaths reported", "morts rapportés")))
                .unwrap_or_default();
            let cites = e["cites"]
                .as_array()
                .map(|c| c.iter().map(|n| format!("[{n}]")).collect::<String>())
                .unwrap_or_default();
            d.para(
                None,
                &[
                    Doc::run(
                        &format!(
                            "{} · {} · ",
                            date_short(&e["date"]),
                            e["kind"].as_str().unwrap_or("")
                        ),
                        true,
                        None,
                        None,
                    ),
                    r(&format!(
                        "{}, {}{}{deaths} ",
                        e["place"].as_str().unwrap_or(""),
                        e["province"].as_str().unwrap_or(""),
                        if actors.is_empty() {
                            String::new()
                        } else {
                            format!(" · {actors}")
                        }
                    )),
                    small(&cites),
                ],
            );
        }
    }
    if let Some(air) = b["air"].as_object().filter(|a| !a.is_empty()) {
        d.para(
            Some("Heading1"),
            &[r(&tr(lang, "Air picture", "Situation aérienne"))],
        );
        d.para(None, &[r(&format!("{} {}.", air.iter().map(|(k, v)| format!("{v} {k}")).collect::<Vec<_>>().join(", "), tr(lang, "aircraft visible at the last scan", "aéronefs visibles au dernier scan"))), small(&format!(" {}", tr(lang, "Community ADS-B coverage over the DRC is sparse; most traffic is not visible.", "La couverture ADS-B communautaire est faible au-dessus de la RDC ; la plupart du trafic n’est pas visible.")))]);
    }
    d.para(Some("Heading1"), &[r(&tr(lang, "Sources", "Sources"))]);
    for (i, s) in b["sources"].as_array().into_iter().flatten().enumerate() {
        let url = s["url"].as_str().unwrap_or("");
        let link = d.link(url, url);
        d.para(
            None,
            &[
                Doc::run(&format!("[{}] ", i + 1), true, Some(18), None),
                small(&format!(
                    "{} — {}, {} ",
                    s["title"].as_str().unwrap_or(""),
                    s["source"].as_str().unwrap_or(""),
                    date_short(&s["date"])
                )),
                link,
            ],
        );
    }
    d.para(None, &[small(b["caveat"].as_str().unwrap_or(""))]);
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><w:body>{}<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/><w:pgMar w:top=\"1134\" w:right=\"1134\" w:bottom=\"1134\" w:left=\"1134\" w:header=\"567\" w:footer=\"567\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>",
        d.body
    );
    let rels = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rIdStyles\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>{}</Relationships>",
        d.links.iter().enumerate().map(|(i, u)| format!("<Relationship Id=\"rIdL{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink\" Target=\"{}\" TargetMode=\"External\"/>", i + 1, x(u))).collect::<String>());
    let styles = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><w:styles xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii=\"Calibri\" w:hAnsi=\"Calibri\" w:cs=\"Calibri\"/><w:sz w:val=\"21\"/><w:lang w:val=\"en-GB\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after=\"100\" w:line=\"276\" w:lineRule=\"auto\"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:name w:val=\"Normal\"/></w:style><w:style w:type=\"paragraph\" w:styleId=\"Title\"><w:name w:val=\"Title\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:spacing w:after=\"60\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"40\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Heading1\"><w:name w:val=\"heading 1\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:keepNext/><w:spacing w:before=\"320\" w:after=\"120\"/><w:pBdr><w:bottom w:val=\"single\" w:sz=\"8\" w:space=\"2\" w:color=\"111111\"/></w:pBdr><w:outlineLvl w:val=\"0\"/></w:pPr><w:rPr><w:b/><w:caps/><w:sz w:val=\"24\"/></w:rPr></w:style><w:style w:type=\"paragraph\" w:styleId=\"Heading2\"><w:name w:val=\"heading 2\"/><w:basedOn w:val=\"Normal\"/><w:pPr><w:keepNext/><w:spacing w:before=\"200\" w:after=\"40\"/><w:outlineLvl w:val=\"1\"/></w:pPr><w:rPr><w:b/><w:sz w:val=\"22\"/></w:rPr></w:style><w:style w:type=\"character\" w:styleId=\"Hyperlink\"><w:name w:val=\"Hyperlink\"/><w:rPr><w:color w:val=\"1F4E79\"/><w:u w:val=\"single\"/></w:rPr></w:style></w:styles>";
    let content_types = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/><Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/><Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/></Types>";
    let root_rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"word/document.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/></Relationships>";
    let core = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:dcterms=\"http://purl.org/dc/terms/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><dc:title>Congo Signal · {}</dc:title><dc:creator>Congo Signal</dc:creator><dcterms:created xsi:type=\"dcterms:W3CDTF\">{}</dcterms:created></cp:coreProperties>",
        x(b["title"].as_str().unwrap_or("")),
        x(b["generated_at"].as_str().unwrap_or(""))
            .chars()
            .take(19)
            .collect::<String>()
            + "Z"
    );
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in [
        ("[Content_Types].xml", content_types.to_string()),
        ("_rels/.rels", root_rels.to_string()),
        ("docProps/core.xml", core),
        ("word/document.xml", document),
        ("word/styles.xml", styles.to_string()),
        ("word/_rels/document.xml.rels", rels),
    ] {
        zip.start_file(name, opts)?;
        zip.write_all(data.as_bytes())?;
    }
    Ok(zip.finish()?.into_inner())
}
/// File name and bytes of the current brief as Word.
pub async fn docx(db: &SqlitePool, period: &str) -> Result<(String, Vec<u8>)> {
    let b = collect(db, period, "en").await?;
    Ok((
        format!(
            "congo-signal-{period}-brief-{}.docx",
            Utc::now().format("%Y-%m-%d")
        ),
        docx_bytes(&b)?,
    ))
}
#[derive(Deserialize)]
pub struct BriefQuery {
    period: Option<String>,
    format: Option<String>,
    lang: Option<String>,
    print: Option<bool>,
}
pub async fn get(State(app): State<App>, Query(q): Query<BriefQuery>) -> ApiResult<Response> {
    let period = if q.period.as_deref() == Some("weekly") {
        "weekly"
    } else {
        "daily"
    };
    let lang = if q.lang.as_deref() == Some("fr") {
        "fr"
    } else {
        "en"
    };
    let b = collect(&app.db, period, lang).await?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    match q.format.as_deref().unwrap_or("json") {
        "json" => Ok((headers, Json(b)).into_response()),
        "html" => {
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            );
            Ok((headers, html(&b, q.print.unwrap_or(false))).into_response())
        }
        "docx" => {
            let bytes = docx_bytes(&b)?;
            headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(DOCX_MIME));
            let name = format!(
                "congo-signal-{period}-brief-{}-{lang}.docx",
                Utc::now().format("%Y-%m-%d")
            );
            headers.insert(
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&format!("attachment; filename=\"{name}\""))
                    .map_err(anyhow::Error::from)?,
            );
            Ok((headers, bytes).into_response())
        }
        _ => Err(ApiError(
            StatusCode::BAD_REQUEST,
            "format must be json, html or docx".into(),
        )),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn docx_is_a_valid_package_with_escaped_text_and_links() {
        let b = json!({"title":"Daily brief","lang":"en","from":"2026-10-18T00:00:00Z","to":"2026-10-19T00:00:00Z","generated_at":"2026-10-19T00:00:00Z","figures":{"reports":2,"outlets":1,"high_priority":1,"incidents":0,"fatalities_reported":0},"provinces":[["Ituri",2]],
            "top":[{"title":"Attaque à Djugu <& co>","source":"Actualite.cd","date":"2026-10-18","score":85,"places":["Djugu"],"text":"Résumé","cite":1}],"incidents":[],"air":{},"sources":[{"title":"Attaque","source":"Actualite.cd","url":"https://actualite.cd/a?b=1&c=2","date":"2026-10-18"}],"caveat":"Unverified."});
        let bytes = docx_bytes(&b).unwrap();
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut doc = String::new();
        std::io::Read::read_to_string(&mut z.by_name("word/document.xml").unwrap(), &mut doc)
            .unwrap();
        assert!(doc.contains("Attaque à Djugu &lt;&amp; co&gt;"));
        let mut rels = String::new();
        std::io::Read::read_to_string(
            &mut z.by_name("word/_rels/document.xml.rels").unwrap(),
            &mut rels,
        )
        .unwrap();
        assert!(rels.contains("https://actualite.cd/a?b=1&amp;c=2"));
        assert!(z.by_name("[Content_Types].xml").is_ok());
        let page = html(&b, false);
        assert!(page.contains("&lt;&amp; co&gt;") && page.contains("[1]"));
    }
}
