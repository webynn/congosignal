//! Owner-managed server credentials. Never serialize stored keys into API responses.
use crate::{ApiError, ApiResult};
use anyhow::{Result, ensure};
use axum::{
    Json,
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{OnceLock, RwLock},
};
const ENGINES: &[&str] = &["claude", "openai", "deepseek", "kimi", "ollama"];
const SERVICES: &[&str] = &[
    "claude",
    "openai",
    "deepseek",
    "kimi",
    "soniox",
    "soniox_tts",
    "ollama",
];
#[derive(Clone, Default, Deserialize, Serialize)]
struct Entry {
    model: Option<String>,
    key: Option<String>,
}
#[derive(Clone, Default, Deserialize, Serialize)]
struct Settings {
    analysis: Option<String>,
    okapi: Option<String>,
    enabled: Option<bool>,
    #[serde(default)]
    providers: BTreeMap<String, Entry>,
}
static SETTINGS: OnceLock<RwLock<Settings>> = OnceLock::new();
fn path() -> PathBuf {
    std::env::var("AI_SETTINGS_PATH")
        .unwrap_or_else(|_| "data/ai-settings.json".into())
        .into()
}
pub fn init() -> Result<()> {
    let path = path();
    let s = if path.exists() {
        let s: Settings = serde_json::from_slice(&std::fs::read(&path)?)?;
        protect(&path)?;
        s
    } else {
        Settings::default()
    };
    let _ = SETTINGS.set(RwLock::new(s));
    Ok(())
}
fn snapshot() -> Settings {
    SETTINGS
        .get()
        .and_then(|v| v.read().ok().map(|s| s.clone()))
        .unwrap_or_default()
}
fn protect(path: &std::path::Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}
fn persist(path: &std::path::Path, s: &Settings) -> Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let result = (|| -> Result<()> {
        let mut f = opts.open(&tmp)?;
        f.write_all(&serde_json::to_vec(s)?)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(tmp);
    }
    result
}
fn env_key(p: &str) -> Option<String> {
    let names: &[&str] = match p {
        "claude" => &["ANTHROPIC_API_KEY"],
        "openai" => &["OPENAI_API_KEY", "OPENAI_KEY"],
        "deepseek" => &["DEEPSEEK_API_KEY"],
        "kimi" => &["MOONSHOT_API_KEY", "KIMI_API_KEY"],
        "soniox" | "soniox_tts" => &["SONIOX_API_KEY"],
        _ => &[],
    };
    names
        .iter()
        .filter_map(|n| std::env::var(n).ok())
        .find(|s| !s.trim().is_empty())
}
pub fn key(p: &str) -> Option<String> {
    let p = if p == "soniox_tts" { "soniox" } else { p };
    snapshot()
        .providers
        .get(p)
        .and_then(|e| e.key.clone())
        .or_else(|| env_key(p))
        .filter(|s| !s.trim().is_empty())
}
pub fn enabled() -> bool {
    snapshot()
        .enabled
        .unwrap_or_else(|| std::env::var("AI_ENABLED").is_ok_and(|v| v == "1"))
}
pub fn provider(okapi: bool) -> String {
    let s = snapshot();
    if okapi {
        s.okapi
            .or_else(|| std::env::var("OKAPI_PROVIDER").ok())
            .or(s.analysis)
    } else {
        s.analysis
    }
    .unwrap_or_else(|| std::env::var("AI_PROVIDER").unwrap_or_else(|_| "claude".into()))
}
pub fn model(p: &str) -> String {
    snapshot()
        .providers
        .get(p)
        .and_then(|e| e.model.clone())
        .unwrap_or_else(|| {
            let (env, default) = match p {
                "claude" => ("CLAUDE_MODEL", "claude-haiku-4-5-20251001"),
                "openai" => ("OPENAI_MODEL", "gpt-4.1-mini"),
                "deepseek" => ("DEEPSEEK_MODEL", "deepseek-flash"),
                "kimi" => ("KIMI_MODEL", "kimi-k2.6"),
                "soniox" => ("SONIOX_STT_MODEL", "stt-rt-v5"),
                "soniox_tts" => ("SONIOX_TTS_MODEL", "tts-rt-v2"),
                _ => ("OLLAMA_MODEL", "gemma4:12b"),
            };
            std::env::var(env).unwrap_or_else(|_| default.into())
        })
}
pub fn endpoint(p: &str) -> Option<&'static str> {
    match p {
        "claude" => Some("https://api.anthropic.com/v1/messages"),
        "openai" => Some("https://api.openai.com/v1/chat/completions"),
        "deepseek" => Some("https://api.deepseek.com/chat/completions"),
        "kimi" => Some("https://api.moonshot.ai/v1/chat/completions"),
        _ => None,
    }
}
fn public(s: &Settings) -> Value {
    json!({"analysis_provider":provider(false),"okapi_provider":provider(true),"enabled":enabled(),"scope":"server-wide owner settings","providers":SERVICES.iter().map(|p|json!({"id":p,"model":model(p),"configured":*p=="ollama"||key(p).is_some(),"key_source":if s.providers.get(if *p=="soniox_tts"{"soniox"}else{p}).is_some_and(|e|e.key.is_some()){"saved"}else{"environment"},"text_engine":ENGINES.contains(p),"endpoint":endpoint(p)})).collect::<Vec<_>>()})
}
pub async fn get() -> (HeaderMap, Json<Value>) {
    let mut h = HeaderMap::new();
    h.insert("cache-control", "no-store".parse().unwrap());
    (h, Json(public(&snapshot())))
}
fn origin(h: &HeaderMap) -> ApiResult<()> {
    let expected = std::env::var("APP_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
    if h.get("origin").and_then(|v| v.to_str().ok()) != Some(expected.trim_end_matches('/')) {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "Settings require the configured app origin".into(),
        ));
    }
    Ok(())
}
#[derive(Deserialize)]
pub struct Update {
    provider: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    #[serde(default)]
    remove_key: bool,
    analysis_provider: Option<String>,
    okapi_provider: Option<String>,
    enabled: Option<bool>,
}
fn apply(s: &mut Settings, u: Update) -> Result<()> {
    for p in [&u.analysis_provider, &u.okapi_provider]
        .into_iter()
        .flatten()
    {
        ensure!(ENGINES.contains(&p.as_str()), "Unsupported AI engine");
    }
    if let Some(p) = u.provider {
        ensure!(SERVICES.contains(&p.as_str()), "Unsupported provider");
        if let Some(m) = &u.model {
            ensure!(
                compatible_model(&p, m),
                "Model is not supported by this provider integration. Refresh the model list."
            );
            ensure!(
                !m.is_empty()
                    && m.len() <= 120
                    && m.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.:/".contains(&b)),
                "Invalid model ID"
            );
        }
        if let Some(k) = &u.api_key {
            ensure!(
                k.len() <= 4096 && k.bytes().all(|b| b.is_ascii_graphic()),
                "Invalid API key"
            );
            ensure!(!u.remove_key, "Choose replacement or removal, not both");
        }
        if let Some(m) = u.model {
            s.providers.entry(p.clone()).or_default().model = Some(m);
        }
        if u.remove_key || u.api_key.as_ref().is_some_and(|k| !k.is_empty()) {
            ensure!(
                p != "ollama" && p != "soniox_tts",
                "Configure the shared Soniox key on Soniox STT"
            );
            s.providers.entry(p).or_default().key = Some(if u.remove_key {
                String::new()
            } else {
                u.api_key.unwrap()
            });
        }
    } else {
        ensure!(
            u.model.is_none() && u.api_key.is_none() && !u.remove_key,
            "Provider required"
        );
    }
    if let Some(p) = u.analysis_provider {
        s.analysis = Some(p);
    }
    if let Some(p) = u.okapi_provider {
        s.okapi = Some(p);
    }
    if let Some(e) = u.enabled {
        s.enabled = Some(e);
    }
    Ok(())
}
pub async fn save(h: HeaderMap, Json(u): Json<Update>) -> ApiResult<(HeaderMap, Json<Value>)> {
    origin(&h)?;
    {
        let lock = SETTINGS
            .get()
            .ok_or_else(|| anyhow::anyhow!("Settings not initialized"))?;
        let mut current = lock
            .write()
            .map_err(|_| anyhow::anyhow!("Settings unavailable"))?;
        let mut next = current.clone();
        apply(&mut next, u).map_err(|e| ApiError(StatusCode::BAD_REQUEST, e.to_string()))?;
        persist(&path(), &next).map_err(|_| {
            ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not save settings. No settings were changed.".into(),
            )
        })?;
        *current = next;
    }
    Ok(get().await)
}
#[derive(Deserialize)]
pub struct Check {
    provider: String,
}
// Reviewed against official provider docs, 2026-10-18. Only IDs returned by
// the provider are offered; this policy determines compatibility, not existence.
pub fn compatible_model(provider: &str, id: &str) -> bool {
    fn version(id: &str, base: &str) -> bool {
        id == base
            || id
                .strip_prefix(base)
                .and_then(|x| x.strip_prefix('-'))
                .is_some_and(|x| {
                    x.len() == 10
                        && x.bytes().enumerate().all(|(i, c)| {
                            if i == 4 || i == 7 {
                                c == b'-'
                            } else {
                                c.is_ascii_digit()
                            }
                        })
                })
    }
    match provider {
        "openai" => [
            "gpt-4.1",
            "gpt-4.1-mini",
            "gpt-4.1-nano",
            "gpt-4o",
            "gpt-4o-mini",
            "gpt-5",
            "gpt-5-mini",
            "gpt-5-nano",
            "gpt-5.1",
            "gpt-5.2",
            "gpt-5.4",
            "gpt-5.4-mini",
            "gpt-5.4-nano",
            "gpt-5.5",
            "gpt-6.1-sol",
        ]
        .iter()
        .any(|base| version(id, base)),
        // Forced tool output requires thinking to be disabled. Always-thinking
        // models need a separate adapter and are not offered here.
        "claude" => [
            "claude-haiku-4-5-20251001",
            "claude-sonnet-4-5-20250929",
            "claude-opus-4-5-20251101",
            "claude-sonnet-4-6",
            "claude-opus-4-6",
            "claude-sonnet-5-5",
            "claude-haiku-5-5",
        ]
        .contains(&id),
        "deepseek" => ["deepseek-flash", "deepseek-v4-pro"].contains(&id),
        "kimi" => [
            "kimi-k2.6",
            "kimi-k2.7-code",
            "kimi-k2.7-code-highspeed",
            "kimi-k3",
        ]
        .contains(&id),
        "soniox" => id == "stt-rt-v5",
        "soniox_tts" => id == "tts-rt-v2",
        "ollama" => true,
        _ => false,
    }
}
fn supported_entry(provider: &str, row: &Value) -> bool {
    let Some(id) = row["id"].as_str().or_else(|| row["name"].as_str()) else {
        return false;
    };
    if !compatible_model(provider, id)
        || row["lifecycle"] == "retired"
        || row["lifecycle"] == "deprecated"
    {
        return false;
    }
    if provider == "claude"
        && row.pointer("/capabilities/thinking/types/disabled/supported")
            == Some(&Value::Bool(false))
    {
        return false;
    }
    true
}
pub async fn check(h: HeaderMap, Json(q): Json<Check>) -> ApiResult<Json<Value>> {
    origin(&h)?;
    let url = match q.provider.as_str() {
        "claude" => "https://api.anthropic.com/v1/models?limit=1000",
        "openai" => "https://api.openai.com/v1/models",
        "deepseek" => "https://api.deepseek.com/models",
        "kimi" => "https://api.moonshot.ai/v1/models",

        _ => {
            return Err(ApiError(
                StatusCode::BAD_REQUEST,
                "Model discovery is available for hosted providers only".into(),
            ));
        }
    };
    let key = key(&q.provider)
        .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "Save an API key first".into()))?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(anyhow::Error::from)?;
    let request = client.get(url);
    let request = if q.provider == "claude" {
        request
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
    } else {
        request.bearer_auth(key)
    };
    let mut r = request
        .send()
        .await
        .map_err(|_| ApiError(StatusCode::BAD_GATEWAY, "Provider connection failed".into()))?;
    if !r.status().is_success() {
        return Err(ApiError(
            StatusCode::BAD_GATEWAY,
            format!(
                "Provider returned HTTP {}. Check the key and API account access.",
                r.status().as_u16()
            ),
        ));
    }
    let mut bytes = Vec::new();
    while let Some(c) = r.chunk().await.map_err(anyhow::Error::from)? {
        if bytes.len() + c.len() > 1_000_000 {
            return Err(ApiError(
                StatusCode::BAD_GATEWAY,
                "Model list too large".into(),
            ));
        }
        bytes.extend_from_slice(&c);
    }
    let v: Value = serde_json::from_slice(&bytes).map_err(|_| {
        ApiError(
            StatusCode::BAD_GATEWAY,
            "Provider returned an invalid model list".into(),
        )
    })?;
    let models = v["data"]
        .as_array()
        .or_else(|| v["models"].as_array())
        .into_iter()
        .flatten()
        .filter(|r| supported_entry(&q.provider, r))
        .filter_map(|r| r["id"].as_str().or_else(|| r["name"].as_str()))
        .filter(|s| s.len() <= 120)
        .take(1000)
        .collect::<Vec<_>>();
    Ok(Json(
        json!({"provider":q.provider,"authenticated":true,"models":models,"checked_at":chrono::Utc::now().to_rfc3339(),"source":url,"notice":"Credentials accepted by model-list endpoint. This does not verify structured generation or available credit. Use a text/chat model with JSON output."}),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_catalog_excludes_wrong_providers_and_incompatible_variants() {
        for (p, m) in [
            ("openai", "gpt-4.1-mini"),
            ("openai", "gpt-5.5-2026-04-23"),
            ("openai", "gpt-6.1-sol"),
            ("deepseek", "deepseek-flash"),
            ("kimi", "kimi-k3"),
        ] {
            assert!(compatible_model(p, m));
        }
        for (p, m) in [
            ("openai", "gpt-5-pro"),
            ("openai", "gpt-5.4-codex"),
            ("openai", "gpt-4o-audio-preview"),
            ("openai", "gpt-99"),
            ("openai", "gpt-4.1-invalid"),
            ("kimi", "deepseek-flash"),
            ("claude", "claude-fable-5-1"),
        ] {
            assert!(!compatible_model(p, m));
        }
        assert!(!supported_entry(
            "claude",
            &json!({"id":"claude-sonnet-5-5","capabilities":{"thinking":{"types":{"disabled":{"supported":false}}}}})
        ));
        assert!(!supported_entry(
            "claude",
            &json!({"id":"claude-sonnet-5-5","lifecycle":"retired"})
        ));
    }
    #[test]
    fn updates_preserve_keys_and_reject_bad_inputs() {
        let mut s = Settings::default();
        apply(
            &mut s,
            serde_json::from_value(
                json!({"provider":"kimi","api_key":"test-secret","model":"kimi-k2.6"}),
            )
            .unwrap(),
        )
        .unwrap();
        apply(
            &mut s,
            serde_json::from_value(json!({"provider":"kimi","model":"kimi-k3"})).unwrap(),
        )
        .unwrap();
        assert_eq!(s.providers["kimi"].key.as_deref(), Some("test-secret"));
        assert!(
            apply(
                &mut s,
                serde_json::from_value(json!({"analysis_provider":"https://evil.example"}))
                    .unwrap()
            )
            .is_err()
        );
        assert!(
            apply(
                &mut s,
                serde_json::from_value(json!({"provider":"kimi","api_key":"a\nb"})).unwrap()
            )
            .is_err()
        );
        apply(
            &mut s,
            serde_json::from_value(json!({"provider":"kimi","remove_key":true})).unwrap(),
        )
        .unwrap();
        assert_eq!(s.providers["kimi"].key.as_deref(), Some(""));
    }
    #[test]
    fn public_settings_do_not_contain_credentials() {
        let mut s = Settings::default();
        s.providers.insert(
            "kimi".into(),
            Entry {
                key: Some("never-echo-this-secret".into()),
                model: None,
            },
        );
        assert!(!public(&s).to_string().contains("never-echo-this-secret"));
    }
    #[test]
    fn settings_persist_with_private_permissions() {
        let path = std::env::temp_dir().join(format!("settings-{}.json", uuid::Uuid::new_v4()));
        let s = Settings {
            analysis: Some("deepseek".into()),
            ..Settings::default()
        };
        persist(&path, &s).unwrap();
        let loaded: Settings = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(loaded.analysis, s.analysis);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        std::fs::remove_file(path).unwrap();
    }
}
