//! Server-only permanent key; one bounded, single-use STT credential per budget reservation.
use crate::{ApiError, ApiResult};
use axum::{
    Json,
    http::{HeaderMap, StatusCode},
};
use serde_json::{Value, json};

pub async fn status(
    axum::extract::State(app): axum::extract::State<crate::App>,
) -> ApiResult<Json<Value>> {
    let paused: i64 = sqlx::query_scalar("SELECT paused FROM cost_limits WHERE provider='soniox'")
        .fetch_one(&app.db)
        .await?;
    Ok(Json(
        json!({"configured":crate::ai_settings::key("soniox").is_some(),"paused":paused != 0,"provider":"soniox","max_session_seconds":60,"model":crate::ai_settings::model("soniox")}),
    ))
}
/// Built-in voices and languages, used when Soniox's catalogue cannot be fetched.
const FALLBACK_VOICES: [(&str, &str, &str); 28] = [
    ("Adrian", "male", ""),
    ("Maya", "female", ""),
    ("Daniel", "male", ""),
    ("Noah", "male", ""),
    ("Nina", "female", ""),
    ("Emma", "female", ""),
    ("Jack", "male", ""),
    ("Claire", "female", ""),
    ("Grace", "female", ""),
    ("Owen", "male", ""),
    ("Mina", "female", ""),
    ("Kenji", "male", ""),
    ("Rafael", "male", "spanish"),
    ("Mateo", "male", "spanish"),
    ("Lucia", "female", "spanish"),
    ("Sofia", "female", "spanish"),
    ("Oliver", "male", "british"),
    ("Arthur", "male", "british"),
    ("Isla", "female", "british"),
    ("Victoria", "female", "british"),
    ("Cooper", "male", "australian"),
    ("Mason", "male", "australian"),
    ("Ruby", "female", "australian"),
    ("Elise", "female", "australian"),
    ("Arjun", "male", "indian"),
    ("Rohan", "male", "indian"),
    ("Priya", "female", "indian"),
    ("Meera", "female", "indian"),
];
/// Soniox TTS languages (ISO 639-1); speech-to-text supports the same set.
const FALLBACK_LANGUAGES: [&str; 60] = [
    "af", "sq", "ar", "az", "eu", "be", "bn", "bs", "bg", "ca", "zh", "hr", "cs", "da", "nl", "en",
    "et", "fi", "fr", "gl", "de", "el", "gu", "he", "hi", "hu", "id", "it", "ja", "kn", "kk", "ko",
    "lv", "lt", "mk", "ms", "ml", "mr", "no", "fa", "pl", "pt", "pa", "ro", "ru", "sr", "sk", "sl",
    "es", "sw", "sv", "tl", "ta", "te", "th", "tr", "uk", "ur", "vi", "cy",
];
static OPTIONS: std::sync::Mutex<Option<(std::time::Instant, Value)>> = std::sync::Mutex::new(None);
fn fallback_options(model: &str) -> Value {
    json!({"model":model,"source":"built-in list","voices":FALLBACK_VOICES.iter().map(|(id,g,a)|json!({"id":id,"gender":g,"accent":a})).collect::<Vec<_>>(),"languages":FALLBACK_LANGUAGES.iter().map(|c|json!({"code":c})).collect::<Vec<_>>()})
}
/// Voices and languages available for the configured TTS model, from Soniox's catalogue
/// (models plus shared voices), cached for a day. Listing is free; no credential reaches the browser.
pub async fn options() -> ApiResult<Json<Value>> {
    let model = crate::ai_settings::model("soniox_tts");
    if let Some((at, v)) = OPTIONS.lock().unwrap().as_ref()
        && at.elapsed() < std::time::Duration::from_secs(86400)
        && v["model"] == model.as_str()
    {
        return Ok(Json(v.clone()));
    }
    let Some(key) = crate::ai_settings::key("soniox") else {
        return Ok(Json(fallback_options(&model)));
    };
    let fetched = async {
        let client = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(std::time::Duration::from_secs(15)).build()?;
        let get = |url: String| {
            let (client, key) = (client.clone(), key.clone());
            async move {
                let r = client.get(url).bearer_auth(key).send().await?;
                anyhow::ensure!(r.status().is_success(), "Soniox catalogue returned HTTP {}", r.status());
                anyhow::ensure!(r.content_length().unwrap_or(0) < 4_000_000, "Soniox catalogue too large");
                Ok::<Value, anyhow::Error>(r.json().await?)
            }
        };
        let models = get("https://api.soniox.com/v1/tts-models".into()).await?;
        let entry = models["models"].as_array().into_iter().flatten()
            .find(|m| m["id"] == model.as_str() || m["aliased_model_id"] == model.as_str())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Configured TTS model {model} is not in the Soniox catalogue"))?;
        let mut voices: Vec<Value> = entry["voices"].as_array().cloned().unwrap_or_default();
        // Shared voices carry accent/age/style; merge them in by id (bounded pagination).
        let mut cursor = String::new();
        for _ in 0..3 {
            let page = get(format!("https://api.soniox.com/v1/shared-voices?model={model}&limit=200{}", if cursor.is_empty() { String::new() } else { format!("&cursor={cursor}") })).await;
            let Ok(page) = page else { break };
            for v in page["voices"].as_array().into_iter().flatten() {
                match voices.iter_mut().find(|x| x["id"] == v["id"]) {
                    Some(x) => { for k in ["accent", "age", "style", "use_case", "description", "gender"] { if !v[k].is_null() { x[k] = v[k].clone() } } }
                    None => voices.push(v.clone()),
                }
            }
            match page["next_page_cursor"].as_str() { Some(c) if !c.is_empty() => cursor = c.to_string(), _ => break }
        }
        voices.retain(|v| v["id"].as_str().is_some_and(|id| !id.is_empty() && id.len() <= 64));
        voices.truncate(600);
        let languages = entry["languages"].as_array().cloned().unwrap_or_default();
        anyhow::ensure!(!voices.is_empty() && !languages.is_empty(), "Soniox catalogue had no voices or languages");
        Ok::<Value, anyhow::Error>(json!({"model":model,"source":"Soniox catalogue","voices":voices,"languages":languages}))
    }
    .await;
    let value = match fetched {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "Soniox voice catalogue unavailable; using built-in list");
            let mut v = fallback_options(&model);
            v["notice"] = json!(e.to_string());
            return Ok(Json(v));
        }
    };
    *OPTIONS.lock().unwrap() = Some((std::time::Instant::now(), value.clone()));
    Ok(Json(value))
}
fn token_spec() -> Value {
    json!({"usage_type":"transcribe_websocket","expires_in_seconds":60,"single_use":true,"max_session_duration_seconds":60})
}
pub async fn token(
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResult<(HeaderMap, Json<Value>)> {
    // Even local mode requires a same-origin browser request. No ambient cross-site spending.
    let expected = std::env::var("APP_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    if headers.get("origin").and_then(|v| v.to_str().ok()) != Some(expected.trim_end_matches('/')) {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "Voice requests require the configured app origin".into(),
        ));
    }
    let key = crate::ai_settings::key("soniox").ok_or_else(|| {
        ApiError(
            StatusCode::SERVICE_UNAVAILABLE,
            "Soniox is not configured".into(),
        )
    })?;
    let speaking = body["usage"] == "tts";
    let reservation = crate::costs::reserve(if speaking {"soniox_tts"} else {"soniox"}, "voice_60s_single_use").await.map_err(|_| ApiError(StatusCode::FORBIDDEN, "Voice budget paused, unconfigured, or exhausted. Check Costs & budgets and your member allowance.".into()))?;
    let mut spec = token_spec();
    if speaking {
        spec["usage_type"] = json!("tts_rt");
    }
    let actor = crate::authentication::actor();
    spec["client_reference_id"] = json!(format!("congosignal:{reservation}:{}", actor.user_id));
    let result=async {
        let client=reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(std::time::Duration::from_secs(15)).build()?;
        let mut response=client.post("https://api.soniox.com/v1/auth/temporary-api-key").bearer_auth(key).json(&spec).send().await?;
        anyhow::ensure!(response.status().is_success(),"Soniox credential request returned HTTP {}",response.status());
        let mut bytes=Vec::new();
        while let Some(chunk)=response.chunk().await? {anyhow::ensure!(bytes.len()+chunk.len()<65536,"Unexpected voice response size");bytes.extend_from_slice(&chunk);}
        let data:Value=serde_json::from_slice(&bytes)?;
        anyhow::ensure!(data["api_key"].as_str().is_some_and(|v|!v.is_empty()),"Soniox returned no temporary credential");
        Ok::<Value,anyhow::Error>(json!({"api_key":data["api_key"],"expires_at":data["expires_at"],"max_session_seconds":60,"model":crate::ai_settings::model(if speaking {"soniox_tts"}else{"soniox"}),"voice":"Adrian"}))
    }.await;
    // Never persist credentials, transcripts or microphone audio in cost logs.
    crate::costs::finish(reservation,if result.is_ok(){"credential_issued"}else{"credential_failed"},json!({"max_session_seconds":60,"billing":"allocated estimate; actual provider usage may differ"})).await?;
    let mut out = HeaderMap::new();
    out.insert("cache-control", "no-store".parse().unwrap());
    Ok((out, Json(result?)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credentials_are_single_use_and_bounded() {
        let s = token_spec();
        assert_eq!(s["single_use"], true);
        assert_eq!(s["max_session_duration_seconds"], 60);
        assert_eq!(s["expires_in_seconds"], 60);
        assert_eq!(s["usage_type"], "transcribe_websocket");
    }
}

#[derive(serde::Deserialize)]
pub struct Conversation {
    message: String,
    #[serde(default)]
    history: Vec<Value>,
    #[serde(default)]
    view: String,
    /// ISO 639-1 code the user chose (or Soniox detected) for the conversation; empty = follow the question.
    #[serde(default)]
    language: String,
}
fn limited(v: &Value, n: usize) -> String {
    v.as_str().unwrap_or("").chars().take(n).collect()
}
fn selected(rows: Vec<Value>, question: &str, limit: usize) -> Vec<Value> {
    let words: Vec<_> = question
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 3)
        .map(str::to_lowercase)
        .collect();
    let mut rows: Vec<_> = rows.into_iter().enumerate().collect();
    rows.sort_by_key(|(i, v)| {
        let text = v.to_string().to_lowercase();
        (
            std::cmp::Reverse(words.iter().filter(|w| text.contains(w.as_str())).count()),
            *i,
        )
    });
    rows.into_iter().take(limit).map(|(_, v)| v).collect()
}
fn validate_answer(mut answer: Value, sources: &[Value]) -> anyhow::Result<Value> {
    anyhow::ensure!(
        answer["answer"]
            .as_str()
            .is_some_and(|s| !s.is_empty() && s.len() < 12000),
        "Invalid assistant answer"
    );
    let action = &answer["action"];
    anyhow::ensure!(
        ["none", "navigate", "search", "collect", "analyze"]
            .contains(&action["type"].as_str().unwrap_or("")),
        "Unsupported assistant action"
    );
    if action["type"] == "navigate" {
        anyhow::ensure!(
            [
                "briefing",
                "situation",
                "reports",
                "minerals",
                "flights",
                "radio",
                "actors",
                "agreements",
                "sources",
                "social",
                "costs"
            ]
            .contains(&action["target"].as_str().unwrap_or("")),
            "Unsupported destination"
        );
    }
    if action["type"] == "search" {
        anyhow::ensure!(
            action["query"]
                .as_str()
                .is_some_and(|s| !s.trim().is_empty() && s.chars().count() <= 160),
            "Invalid search"
        );
    }
    let ids = answer["citations"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing citations"))?;
    anyhow::ensure!(
        ids.len() <= 8 && ids.iter().all(|id| sources.iter().any(|s| s["id"] == *id)),
        "Assistant returned unknown source citations"
    );
    let citations = ids
        .iter()
        .filter_map(|id| sources.iter().find(|s| s["id"] == *id))
        .map(|s| json!({"id":s["id"],"title":s["title"],"url":s["url"],"date":s["date"]}))
        .collect::<Vec<_>>();
    answer["citations"] = json!(citations);
    answer["requires_confirmation"] =
        json!(["collect", "analyze"].contains(&answer["action"]["type"].as_str().unwrap_or("")));
    Ok(answer)
}
pub async fn converse(
    axum::extract::State(app): axum::extract::State<crate::App>,
    Json(request): Json<Conversation>,
) -> ApiResult<(HeaderMap, Json<Value>)> {
    use axum::extract::{Query, State};
    if request.message.trim().is_empty()
        || request.message.chars().count() > 2000
        || request.history.len() > 8
        || request.view.len() > 32
        || request.language.len() > 8
        || !request
            .language
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == '-')
    {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Message or conversation exceeds limit".into(),
        ));
    }
    let history = request
        .history
        .iter()
        .map(|h| json!({"user":limited(&h["user"],2000),"assistant":limited(&h["assistant"],3000)}))
        .collect::<Vec<_>>();
    let query = serde_json::from_value(json!({"limit":500})).map_err(anyhow::Error::from)?;
    let news = crate::intelligence::feed(State(app.clone()), Query(query))
        .await?
        .0;
    let mut sources = Vec::new();
    let mut rows = Vec::new();
    for a in news["items"].as_array().into_iter().flatten() {
        rows.push(json!({"id":format!("report:{}",a["id"].as_str().unwrap_or("")),"title":a["title"],"url":a["url"],"date":a["published_at"],"fetched_at":a["fetched_at"],"publisher":a["source"],"excerpt":limited(&a["search_text"],600),"description":a["description"],"verification":a["verification"]}));
    }
    sources.extend(selected(rows, &request.message, 10));
    let social = crate::social::feed(State(app.clone())).await?.0;
    for topic in social["items"].as_array().into_iter().flatten() {
        let mut counts = topic["data"].clone();
        if let Some(map) = counts.as_object_mut() {
            map.remove("buckets");
        }
        sources.push(json!({"id":format!("topic:{}",topic["topic"]["id"].as_str().unwrap_or("")),"title":topic["topic"],"url":"/#social","date":topic["checked_at"],"stale":topic["stale"],"data":counts,"archived_posts":topic["archived_posts"],"error":topic["error"]}));
    }
    let posts = crate::social::posts(State(app.clone()), Query(Default::default()))
        .await?
        .0;
    let rows=posts["items"].as_array().into_iter().flatten().map(|p|json!({"id":format!("post:{}",p["id"].as_str().unwrap_or("")),"title":limited(&p["text"],140),"url":p["url"],"date":p["created_at"],"text":limited(&p["text"],800),"author_handle":p["author_handle"],"collected_at":p["collected_at"],"verification":"unverified public post"})).collect();
    sources.extend(selected(rows, &request.message, 8));
    let minerals: Value = serde_json::from_str(include_str!("../dashboard/geology/sites.geo.json"))
        .map_err(anyhow::Error::from)?;
    let sites = minerals["features"].as_array().unwrap();
    sources.push(json!({"id":"minerals:overview","title":"Mineral site inventory","url":"/#minerals","total_records":sites.len(),"notice":"Historical IPIS and USGS records, not proven reserves, current ownership or operations."}));
    let rows=sites.iter().map(|f|json!({"id":format!("site:{}",f["id"]),"title":f["properties"]["name"],"url":"/#minerals","properties":f["properties"]})).collect();
    sources.extend(selected(rows, &request.message, 8));
    let agreements = crate::agreements::list(State(app.clone())).await?.0;
    let rows=agreements["items"].as_array().into_iter().flatten().map(|a|json!({"id":format!("agreement:{}",a["id"].as_str().unwrap_or("")),"title":a["title"],"url":"/#agreements","date":a["signed_at"],"metadata":a})).collect();
    let agreement_rows = selected(rows, &request.message, 3);
    for mut a in agreement_rows {
        a["metadata"] = json!(
            a["metadata"]
                .to_string()
                .chars()
                .take(2400)
                .collect::<String>()
        );
        sources.push(a);
    }
    let actors = crate::actors::list(State(app.clone())).await?.0;
    let rows=actors["items"].as_array().into_iter().flatten().map(|a|json!({"id":format!("actor:{}",a["id"].as_str().unwrap_or("")),"title":a["name"],"url":format!("/#actor/{}",a["id"].as_str().unwrap_or("")),"description":a["description"],"titles":a["titles"],"sanctions":a["sanctions"],"notice":actors["notice"]})).collect();
    sources.extend(selected(rows, &request.message, 5));
    let input = json!({"question":request.message,"reply_language":request.language,"conversation":history,"current_view":request.view,"as_of":chrono::Utc::now().to_rfc3339(),"coverage":{"news_total":news["total"],"news_retrieval":"ranked selection from up to 500 most recent loaded reports; snippets truncated; not exhaustive","social":social["notice"],"social_archive":social["archive"],"posts":posts["coverage"],"agreement_metadata":"metadata only, may be truncated; full PDF analysis not performed"},"sources":sources});
    let answer=crate::intelligence::okapi_answer(input).await.map_err(|e|ApiError(StatusCode::SERVICE_UNAVAILABLE,if e.to_string().contains("Cost controls"){"Okapi's AI budget is paused, unconfigured or exhausted. Check Costs & budgets.".into()}else{"Okapi's AI provider is unavailable or returned an incomplete answer. Check AI configuration.".into()}))?;
    let answer = validate_answer(answer, &sources).map_err(|_| {
        ApiError(
            StatusCode::BAD_GATEWAY,
            "Okapi returned an unsupported action or citation. No action was taken.".into(),
        )
    })?;
    let mut headers = HeaderMap::new();
    headers.insert("cache-control", "no-store".parse().unwrap());
    Ok((headers, Json(answer)))
}

#[cfg(test)]
mod conversation_tests {
    use super::*;
    #[test]
    fn rejects_unknown_actions_and_citations() {
        let sources = vec![json!({"id":"a","title":"source","url":"/#reports"})];
        let base = json!({"answer":"Hello","citations":["a"],"action":{"type":"navigate","target":"minerals","query":""}});
        assert!(validate_answer(base.clone(), &sources).is_ok());
        let mut invalid = base.clone();
        invalid["action"]["target"] = json!("https://evil.example");
        assert!(validate_answer(invalid, &sources).is_err());
        let mut invalid = base.clone();
        invalid["citations"] = json!(["invented"]);
        assert!(validate_answer(invalid, &sources).is_err());
        let mut write = base;
        write["action"]["type"] = json!("collect");
        assert_eq!(
            validate_answer(write, &sources).unwrap()["requires_confirmation"],
            true
        );
    }
}
