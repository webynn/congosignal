//! Private, immutable briefing snapshots from saved source material. No inference calls.
use crate::{ApiError, ApiResult, App, authentication, extract::Article, intelligence};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use std::collections::HashSet;
pub async fn init(db: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query("CREATE TABLE IF NOT EXISTS research_briefs(id TEXT PRIMARY KEY,workspace_id TEXT NOT NULL,user_id TEXT NOT NULL,created_at TEXT NOT NULL,data TEXT NOT NULL)").execute(db).await?;
    Ok(())
}
#[derive(Deserialize)]
pub struct Create {
    title: String,
    lang: String,
    #[serde(default)]
    article_ids: Vec<String>,
}
pub async fn create(State(app): State<App>, Json(q): Json<Create>) -> ApiResult<Json<Value>> {
    if !(1..=160).contains(&q.title.trim().chars().count())
        || !["en", "fr"].contains(&q.lang.as_str())
    {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Use a title of 1–160 characters and language en/fr".into(),
        ));
    }
    let actor = authentication::actor();
    let ids = if q.article_ids.is_empty() {
        sqlx::query_scalar::<_,String>("SELECT article_id FROM private_notes WHERE workspace_id=? AND user_id=? AND saved=1 ORDER BY updated_at DESC LIMIT 51").bind(&actor.workspace_id).bind(&actor.user_id).fetch_all(&app.db).await?
    } else {
        q.article_ids
    };
    if ids.is_empty() || ids.len() > 50 || ids.iter().collect::<HashSet<_>>().len() != ids.len() {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "Select 1–50 distinct saved articles".into(),
        ));
    }
    let mut articles = Vec::new();
    for id in ids {
        let raw: Option<String> = sqlx::query_scalar("SELECT data FROM articles WHERE id=?")
            .bind(&id)
            .fetch_optional(&app.db)
            .await?;
        let a: Article = serde_json::from_str(&raw.ok_or_else(|| {
            ApiError(StatusCode::NOT_FOUND, "Article no longer available".into())
        })?)
        .map_err(anyhow::Error::from)?;
        let brief = intelligence::enrichment(&app.db, &a, "brief").await?;
        let preview = intelligence::enrichment(&app.db, &a, "preview").await?;
        let cached = if brief["status"] == "completed" {
            &brief
        } else {
            &preview
        };
        let completed = cached["status"] == "completed";
        let title = if q.lang == "en" && completed {
            cached["data"]["title_en"].as_str().unwrap_or(&a.title)
        } else {
            &a.title
        };
        let key = if q.lang == "en" {
            "summary_en"
        } else {
            "summary_fr"
        };
        let summary = if completed {
            cached["data"][key].as_str()
        } else {
            None
        };
        let fallback = a
            .description
            .as_deref()
            .unwrap_or(&a.content)
            .chars()
            .take(1500)
            .collect::<String>();
        articles.push(json!({"id":a.id,"title":title,"original_title":a.title,"text":summary.unwrap_or(&fallback),"text_kind":if summary.is_some(){"cached_ai_summary"}else{"source_excerpt"},"source_language":a.language,"source":a.source,"url":a.url,"published_at":a.published_at,"fetched_at":a.fetched_at,"fingerprint":intelligence::fingerprint(&a),"coverage":a.coverage,"ai_provenance":if summary.is_some(){cached.clone()}else{Value::Null}}));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let out = json!({"id":id,"title":q.title.trim(),"lang":q.lang,"created_at":now,"articles":articles,"notice":"Private immutable snapshot. Cached AI summaries require source review; untranslated items retain the original language. No new AI calls. Private research notes are excluded."});
    sqlx::query("INSERT INTO research_briefs VALUES(?,?,?,?,?)")
        .bind(&id)
        .bind(&actor.workspace_id)
        .bind(&actor.user_id)
        .bind(&now)
        .bind(out.to_string())
        .execute(&app.db)
        .await?;
    Ok(Json(out))
}
async fn owned(
    db: &SqlitePool,
    id: &str,
    actor: &authentication::Actor,
) -> anyhow::Result<Option<Value>> {
    let raw: Option<String> = sqlx::query_scalar(
        "SELECT data FROM research_briefs WHERE id=? AND workspace_id=? AND user_id=?",
    )
    .bind(id)
    .bind(&actor.workspace_id)
    .bind(&actor.user_id)
    .fetch_optional(db)
    .await?;
    raw.map(|s| Ok(serde_json::from_str(&s)?)).transpose()
}
pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let a = authentication::actor();
    let rows:Vec<String>=sqlx::query_scalar("SELECT data FROM research_briefs WHERE workspace_id=? AND user_id=? ORDER BY created_at DESC LIMIT 100").bind(a.workspace_id).bind(a.user_id).fetch_all(&app.db).await?;
    let items:Vec<Value>=rows.iter().filter_map(|s|serde_json::from_str::<Value>(s).ok()).map(|b|json!({"id":b["id"],"title":b["title"],"lang":b["lang"],"created_at":b["created_at"],"articles":b["articles"].as_array().map(Vec::len)})).collect();
    Ok(Json(json!({"items":items})))
}
fn html(b: &Value) -> String {
    let e = crate::notify::esc;
    let fr = b["lang"] == "fr";
    let mut out = format!(
        "<!doctype html><html lang=\"{}\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{}</title><style>body{{max-width:850px;margin:40px auto;padding:20px;font:16px/1.6 system-ui}}article{{margin:30px 0;border-top:1px solid #ccc}}a{{overflow-wrap:anywhere}}small{{color:#555}}@media print{{article{{break-inside:avoid}}}}</style><h1>{}</h1><p>{}</p><p>{}</p>",
        if fr { "fr" } else { "en" },
        e(b["title"].as_str().unwrap_or("")),
        e(b["title"].as_str().unwrap_or("")),
        e(b["created_at"].as_str().unwrap_or("")),
        if fr {
            "Instantané de recherche. Vérifiez les sources et les résumés IA. Les extraits non traduits restent dans leur langue d’origine."
        } else {
            "Research snapshot. Check sources and AI summaries. Untranslated excerpts retain their original language."
        }
    );
    for (i, a) in b["articles"].as_array().into_iter().flatten().enumerate() {
        let url = a["url"].as_str().unwrap_or("");
        let safe = url::Url::parse(url).is_ok_and(|u| ["https", "http"].contains(&u.scheme()));
        out.push_str(&format!(
            "<article><h2>[{}] {}</h2><small>{} · {} · {}</small><p>{}</p><p>{}</p></article>",
            i + 1,
            e(a["title"].as_str().unwrap_or("")),
            e(a["source"].as_str().unwrap_or("")),
            e(a["published_at"].as_str().unwrap_or("—")),
            if a["text_kind"] == "cached_ai_summary" {
                if fr {
                    "Résumé IA en cache"
                } else {
                    "Cached AI summary"
                }
            } else if fr {
                "Extrait de source"
            } else {
                "Source excerpt"
            },
            e(a["text"].as_str().unwrap_or("")),
            if safe {
                format!(
                    "<a href=\"{}\" rel=\"noopener noreferrer\">{}</a>",
                    e(url),
                    e(url)
                )
            } else {
                e(url)
            }
        ));
    }
    out.push_str("</html>");
    out
}
#[derive(Deserialize)]
pub struct Export {
    format: Option<String>,
}
pub async fn get(
    State(app): State<App>,
    Path(id): Path<String>,
    Query(q): Query<Export>,
) -> ApiResult<Response> {
    let b = owned(&app.db, &id, &authentication::actor())
        .await?
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "Brief not found".into()))?;
    if q.format.as_deref() == Some("html") {
        Ok((
            [
                ("content-type", "text/html; charset=utf-8"),
                ("cache-control", "no-store"),
            ],
            html(&b),
        )
            .into_response())
    } else {
        Ok(Json(b).into_response())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn private_briefs_require_matching_user_and_workspace() {
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        sqlx::query("INSERT INTO research_briefs VALUES('one','space','alice','now','{}')")
            .execute(&db)
            .await
            .unwrap();
        let mut a = authentication::Actor {
            user_id: "alice".into(),
            workspace_id: "space".into(),
            role: "owner".into(),
            email: "".into(),
        };
        assert!(owned(&db, "one", &a).await.unwrap().is_some());
        a.user_id = "bob".into();
        assert!(owned(&db, "one", &a).await.unwrap().is_none());
        a.user_id = "alice".into();
        a.workspace_id = "other".into();
        assert!(owned(&db, "one", &a).await.unwrap().is_none());
    }
    #[test]
    fn html_escapes_source_content_and_rejects_script_links() {
        let b = json!({"lang":"en","title":"<script>x</script>","articles":[{"title":"unsafe","url":"javascript:alert(1)","text":"<img src=x onerror=alert(1)>"}]});
        let rendered = html(&b);
        assert!(!rendered.contains("<script>"));
        assert!(!rendered.contains("href=\"javascript:"));
        assert!(!rendered.contains("<img"));
    }
}
