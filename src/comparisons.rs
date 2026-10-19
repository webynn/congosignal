//! Cross-publisher story candidates and evidence-backed editorial comparisons.
use crate::{
    ApiError, ApiResult, App,
    extract::{Article, digest},
    intelligence, text,
};
use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use std::collections::{BTreeMap, BTreeSet};
use tokio::sync::Mutex;
static ANALYZING: Mutex<()> = Mutex::const_new(());
pub async fn init(db: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query("CREATE TABLE IF NOT EXISTS article_comparisons(id TEXT PRIMARY KEY, created_at TEXT NOT NULL, provider TEXT NOT NULL, model TEXT NOT NULL, data TEXT NOT NULL)").execute(db).await?;
    Ok(())
}
fn publisher(a: &Article) -> String {
    a.source
        .strip_prefix("NewsData.io · ")
        .unwrap_or(&a.source)
        .trim()
        .to_lowercase()
}
fn words(s: &str) -> BTreeSet<String> {
    const STOP: &str = " les des une dans pour avec sur par aux est sont que qui cette ces ses leur leurs plus apres entre selon aussi tout tous comme lors depuis vers sous contre deux trois the and for with from this that have has was were been into over after before about says said news congo rdc drc nouvelle nouvelles actualite";
    text::tokens(s)
        .into_iter()
        .filter(|w| w.len() > 3 && !STOP.split_whitespace().any(|s| s == w))
        .collect()
}
fn score(a: &Article, b: &Article) -> Option<(f64, Vec<String>)> {
    if publisher(a) == publisher(b) {
        return None;
    }
    let (da, db) = (
        DateTime::parse_from_rfc3339(a.published_at.as_deref()?).ok()?,
        DateTime::parse_from_rfc3339(b.published_at.as_deref()?).ok()?,
    );
    if (da - db).num_hours().abs() > 72 {
        return None;
    }
    let (wa, wb) = (words(&a.title), words(&b.title));
    let common: Vec<String> = wa.intersection(&wb).cloned().collect();
    let similarity = common.len() as f64 / wa.len().min(wb.len()).max(1) as f64;
    (common.len() >= 3 && similarity >= 0.4).then_some((similarity, common))
}
fn card(a: &Article) -> Value {
    json!({"id":a.id,"title":a.title,"source":a.source,"publisher":publisher(a),"author":a.author,"author_method":a.author_method,"published_at":a.published_at,"url":a.url,"coverage":a.coverage,"description":a.description,"category":a.category})
}
async fn articles(db: &SqlitePool) -> anyhow::Result<Vec<Article>> {
    let rows: Vec<String> =
        sqlx::query_scalar("SELECT data FROM articles ORDER BY published_at DESC LIMIT 3000")
            .fetch_all(db)
            .await?;
    rows.iter().map(|r| Ok(serde_json::from_str(r)?)).collect()
}
pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let all = articles(&app.db).await?;
    let mut cards = Vec::new();
    for a in &all {
        let mut c = card(a);
        c["fingerprint"] = json!(intelligence::fingerprint(a));
        let brief = intelligence::enrichment(&app.db, a, "brief").await?;
        let preview = intelligence::enrichment(&app.db, a, "preview").await?;
        for cached in [&brief, &preview] {
            if c["title_en"].is_null() && cached["status"] == "completed" {
                c["title_en"] = cached["data"]["title_en"].clone();
            }
        }
        c["language"] = json!(a.language);
        cards.push(c);
    }
    let mut pairs = Vec::new();
    for (i, a) in all.iter().take(600).enumerate() {
        for b in all.iter().take(600).skip(i + 1) {
            if let Some((s, terms)) = score(a, b) {
                pairs.push(json!({"articles":[a.id,b.id],"score":s,"shared_terms":terms}));
            }
        }
    }
    pairs.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .unwrap()
            .total_cmp(&a["score"].as_f64().unwrap())
    });
    pairs.truncate(100);
    let saved: Vec<String> = sqlx::query_scalar(
        "SELECT data FROM article_comparisons ORDER BY created_at DESC LIMIT 100",
    )
    .fetch_all(&app.db)
    .await?;
    let history: Vec<Value> = saved
        .iter()
        .filter_map(|s| serde_json::from_str(s).ok())
        .collect();
    let mut authors: BTreeMap<String, Value> = BTreeMap::new();
    let mut missing = 0;
    for a in &all {
        let Some(name) = a.author.as_deref().filter(|n| !n.trim().is_empty()) else {
            missing += 1;
            continue;
        };
        // Exact byline+publisher; never assume matching names are the same person across outlets.
        let key = digest(format!("{}:{}", publisher(a), text::fold(name.trim())).as_bytes());
        let entry=authors.entry(key.clone()).or_insert_with(||json!({"id":key,"name":name,"publisher":a.source,"articles":[],"findings":[],"categories":{}}));
        entry["articles"].as_array_mut().unwrap().push(json!(a.id));
        let cat = a.category.as_deref().unwrap_or("Unspecified");
        let n = entry["categories"][cat].as_u64().unwrap_or(0);
        entry["categories"][cat] = json!(n + 1);
        for report in &history {
            // Discard observations for superseded source versions.
            let current = report["sources"].as_array().is_some_and(|ss| {
                ss.iter()
                    .any(|s| s["id"] == a.id && s["fingerprint"] == intelligence::fingerprint(a))
            });
            if !current {
                continue;
            }
            for f in report["analysis"]["findings"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|f| f["article_id"] == a.id)
            {
                entry["findings"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"comparison_id":report["id"],"finding":f}));
            }
        }
    }
    let mut authors: Vec<Value> = authors.into_values().collect();
    authors.sort_by_key(|a| std::cmp::Reverse(a["articles"].as_array().unwrap().len()));
    Ok(Json(
        json!({"articles":cards,"candidates":pairs,"journalists":authors,"history":history,"missing_byline":missing,"scope":"Latest 3000 stored articles; candidates scan the latest 600; suggested matches are lexical candidates within 72 hours, not verified same events. Byline entries may represent teams or publishers."}),
    ))
}
#[derive(Deserialize)]
pub struct Request {
    ids: Vec<String>,
}
fn snapshot_key(all: &[Article]) -> String {
    digest(
        format!(
            "comparison-v1:{}",
            all.iter()
                .map(|a| format!(
                    "{}:{}:{}",
                    a.id,
                    intelligence::fingerprint(a),
                    a.description.as_deref().unwrap_or("")
                ))
                .collect::<Vec<_>>()
                .join("|")
        )
        .as_bytes(),
    )
}
pub async fn analyze(State(app): State<App>, Json(req): Json<Request>) -> ApiResult<Json<Value>> {
    let mut ids = req.ids;
    ids.sort();
    ids.dedup();
    if !(2..=4).contains(&ids.len()) {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Select 2–4 articles".into(),
        ));
    }
    let mut all = Vec::new();
    for id in ids {
        let raw: Option<String> = sqlx::query_scalar("SELECT data FROM articles WHERE id=?")
            .bind(&id)
            .fetch_optional(&app.db)
            .await?;
        let raw = raw.ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "Article not found".into()))?;
        all.push(serde_json::from_str::<Article>(&raw).map_err(anyhow::Error::from)?);
    }
    let key = snapshot_key(&all);
    let _lock = ANALYZING.try_lock().map_err(|_| {
        ApiError(
            StatusCode::CONFLICT,
            "A comparison is already running. Try again shortly.".into(),
        )
    })?;
    if let Some(raw) =
        sqlx::query_scalar::<_, String>("SELECT data FROM article_comparisons WHERE id=?")
            .bind(&key)
            .fetch_optional(&app.db)
            .await?
    {
        return Ok(Json(
            serde_json::from_str(&raw).map_err(anyhow::Error::from)?,
        ));
    }
    if !crate::ai_settings::enabled() {
        return Err(ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "AI is disabled".into(),
        ));
    }
    let provider = crate::ai_settings::provider(false);
    let model = crate::ai_settings::model(&provider);
    let input:Vec<Value>=all.iter().map(|a|json!({"article_id":a.id,"title":a.title,"source":a.source,"author":a.author,"published_at":a.published_at,"coverage":a.coverage,"text":a.content.chars().take(7000).collect::<String>()})).collect();
    let schema = json!({"type":"object","properties":{
        "same_event":{"type":"string","enum":["likely","uncertain","different"]},
        "summary_en":{"type":"string"},"summary_fr":{"type":"string"},
        "agreements_en":{"type":"string"},"agreements_fr":{"type":"string"},
        "differences_en":{"type":"string"},"differences_fr":{"type":"string"},
        "limitations_en":{"type":"string"},"limitations_fr":{"type":"string"},
        "findings":{"type":"array","maxItems":16,"items":{"type":"object","properties":{
            "article_id":{"type":"string"},"kind":{"type":"string","enum":["framing","loaded_language","attribution","uncertainty","angle"]},
            "quote":{"type":"string"},"observation_en":{"type":"string"},"observation_fr":{"type":"string"}},
            "required":["article_id","kind","quote","observation_en","observation_fr"],"additionalProperties":false}}},
        "required":["same_event","summary_en","summary_fr","agreements_en","agreements_fr","differences_en","differences_fr","limitations_en","limitations_fr","findings"],"additionalProperties":false});
    let mut analysis =
        intelligence::compare_chat(&provider, &model, json!({"articles":input}), schema)
            .await
            .map_err(|e| ApiError(StatusCode::BAD_GATEWAY, format!("Comparison failed: {e}")))?;
    // JSON-mode providers sometimes return a more specific category than the requested enum.
    // Keep the evidence and observation, but label unrecognised categories generically.
    if let Some(findings) = analysis["findings"].as_array_mut() {
        for f in findings {
            if f.is_object()
                && ![
                    "framing",
                    "loaded_language",
                    "attribution",
                    "uncertainty",
                    "angle",
                ]
                .contains(&f["kind"].as_str().unwrap_or(""))
            {
                f["kind"] = json!("editorial_observation");
            }
        }
    }
    // Never display an invented or paraphrased passage as a verbatim quotation.
    // Keep independently valid findings rather than discarding them with one bad quote.
    let mut rejected = 0usize;
    if let Some(findings) = analysis["findings"].as_array_mut() {
        findings.retain(|f| {
            let valid = verified_quote(f, &input);
            if !valid {
                rejected += 1;
            }
            valid
        });
    }
    analysis["unverified_findings_omitted"] = json!(rejected);
    validate(&analysis, &input).map_err(|e| ApiError(StatusCode::BAD_GATEWAY, e.to_string()))?;
    let sources: Vec<Value> = all
        .iter()
        .map(|a| {
            let mut c = card(a);
            c["fingerprint"] = json!(intelligence::fingerprint(a));
            c["input_truncated"] = json!(a.content.chars().count() > 7000);
            c
        })
        .collect();
    let out = json!({"id":key,"created_at":Utc::now().to_rfc3339(),"provider":provider,"model":model,"sources":sources,"analysis":analysis});
    sqlx::query("INSERT OR IGNORE INTO article_comparisons VALUES(?,?,?,?,?)")
        .bind(&key)
        .bind(out["created_at"].as_str())
        .bind(provider)
        .bind(model)
        .bind(out.to_string())
        .execute(&app.db)
        .await?;
    Ok(Json(out))
}
fn verified_quote(f: &Value, input: &[Value]) -> bool {
    let quote = f["quote"].as_str().unwrap_or("");
    quote.chars().count() >= 8
        && quote.chars().count() <= 600
        && input.iter().any(|a| {
            a["article_id"] == f["article_id"]
                && (a["text"].as_str().unwrap_or("").contains(quote)
                    || a["title"].as_str().unwrap_or("").contains(quote))
        })
}
fn validate(out: &Value, input: &[Value]) -> anyhow::Result<()> {
    anyhow::ensure!(
        ["likely", "uncertain", "different"].contains(&out["same_event"].as_str().unwrap_or("")),
        "Invalid event assessment"
    );
    for k in [
        "summary_en",
        "summary_fr",
        "agreements_en",
        "agreements_fr",
        "differences_en",
        "differences_fr",
        "limitations_en",
        "limitations_fr",
    ] {
        anyhow::ensure!(
            out[k]
                .as_str()
                .is_some_and(|s| !s.is_empty() && s.len() < 12000),
            "Incomplete comparison"
        );
    }
    let findings = out["findings"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing evidence"))?;
    anyhow::ensure!(findings.len() <= 16, "Too many findings");
    for f in findings {
        let a = input
            .iter()
            .find(|a| a["article_id"] == f["article_id"])
            .ok_or_else(|| anyhow::anyhow!("Unknown evidence article"))?;
        let quote = f["quote"].as_str().unwrap_or("");
        anyhow::ensure!(
            quote.chars().count() >= 8
                && quote.chars().count() <= 600
                && (a["text"].as_str().unwrap_or("").contains(quote)
                    || a["title"].as_str().unwrap_or("").contains(quote)),
            "AI quotation could not be verified against source text"
        );
        anyhow::ensure!(
            [
                "framing",
                "loaded_language",
                "attribution",
                "uncertainty",
                "angle",
                "editorial_observation"
            ]
            .contains(&f["kind"].as_str().unwrap_or("")),
            "Invalid finding type"
        );
        for k in ["observation_en", "observation_fr"] {
            anyhow::ensure!(
                f[k].as_str()
                    .is_some_and(|s| !s.is_empty() && s.len() < 4000),
                "Incomplete finding"
            );
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn a() -> Article {
        crate::extract::extract(
            include_str!("../tests/fixtures/article.html"),
            &url::Url::parse("https://actualite.cd/2026/01/01/test").unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn matching_requires_time_and_different_publishers() {
        let mut x = a();
        x.title = "Goma : cessez-le-feu annoncé après les combats".into();
        x.published_at = Some("2026-10-19T10:00:00Z".into());
        let mut y = x.clone();
        y.source = "Radio Okapi".into();
        assert!(score(&x, &y).is_some());
        y.source = x.source.clone();
        assert!(score(&x, &y).is_none());
        y.source = "Radio Okapi".into();
        y.published_at = Some("2026-09-01T10:00:00Z".into());
        assert!(score(&x, &y).is_none());
    }
    #[test]
    fn cache_changes_with_source_and_ignores_collection_time() {
        let x = a();
        let mut y = x.clone();
        y.fetched_at = "later".into();
        assert_eq!(
            snapshot_key(std::slice::from_ref(&x)),
            snapshot_key(&[y.clone()])
        );
        y.title = "Changed".into();
        assert_ne!(snapshot_key(&[x]), snapshot_key(&[y]));
    }
    #[test]
    fn invented_quotes_are_rejected() {
        let mut out = json!({"same_event":"likely","findings":[{"article_id":"a","quote":"invented quotation","kind":"framing","observation_en":"test","observation_fr":"test"}]});
        for k in [
            "summary_en",
            "summary_fr",
            "agreements_en",
            "agreements_fr",
            "differences_en",
            "differences_fr",
            "limitations_en",
            "limitations_fr",
        ] {
            out[k] = json!("test");
        }
        let input =
            vec![json!({"article_id":"a","text":"Actual source quotation","title":"title"})];
        assert!(validate(&out, &input).is_err());
        out["findings"][0]["quote"] = json!("Actual source quotation");
        assert!(validate(&out, &input).is_ok());
    }
}
