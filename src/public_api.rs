//! Read-only news for anonymous readers of the news edition (password/OIDC deployments).
//! Only published content is returned: titles, text, places, publisher photos and the English
//! headline/summary. Analyst notes, review state, AI assessments and costs are never included.
use crate::{ApiResult, App};
use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde_json::{Value, json};

/// Anonymous GET paths when PUBLIC_NEWS is not "0".
pub fn allowed(method: &str, path: &str) -> bool {
    std::env::var("PUBLIC_NEWS").as_deref() != Ok("0")
        && method == "GET"
        && (path.starts_with("/v1/public/")
            || [
                "/v1/events",
                "/v1/control",
                "/v1/news/images",
                "/v1/radio",
                "/v1/config",
            ]
            .contains(&path))
}
fn summary(v: &Value) -> Value {
    let d = &v["data"];
    if d.is_object() {
        json!({"data":{"title_en":d["title_en"],"summary_en":d["summary_en"]}})
    } else {
        Value::Null
    }
}
fn public_item(a: &Value) -> Value {
    json!({"id":a["id"],"title":a["title"],"url":a["url"],"source":a["source"],"category":a["category"],"description":a["description"],
        "published_at":a["published_at"],"language":a["language"],"places":a["places"],"coverage":a["coverage"],"relevance":a["relevance"],
        "record_type":a["record_type"],"search_text":a["search_text"],"rank":{"score":a["rank"]["score"],"band":a["rank"]["band"]},
        "preview":summary(&a["preview"]),"brief":summary(&a["brief"]),"analyst":{"saved":false}})
}
pub async fn news(State(app): State<App>) -> ApiResult<Json<Value>> {
    let query = serde_json::from_value(json!({"limit":300})).map_err(anyhow::Error::from)?;
    let feed = crate::intelligence::feed(State(app), Query(query)).await?.0;
    let items: Vec<Value> = feed["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["record_type"] != "evidence")
        .map(public_item)
        .collect();
    Ok(Json(
        json!({"items":items,"total":items.len(),"generated_at":feed["generated_at"]}),
    ))
}
pub async fn article(State(app): State<App>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let d = crate::intelligence::detail(State(app), Path(id)).await?.0;
    let t = &d["translation"]["data"];
    Ok(Json(
        json!({"article":d["article"],"translation":if t.is_object() { json!({"data":{"title":t["title"],"paragraphs":t["paragraphs"]}}) } else { Value::Null },"preview":summary(&d["preview"])}),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_published_reads_are_public() {
        assert!(allowed("GET", "/v1/public/news"));
        assert!(allowed("GET", "/v1/control"));
        assert!(!allowed("POST", "/v1/public/news"));
        for p in [
            "/v1/intelligence",
            "/v1/team",
            "/v1/costs",
            "/v1/watches",
            "/v1/alerts",
            "/v1/social",
            "/v1/settings/ai",
        ] {
            assert!(!allowed("GET", p), "{p}");
        }
        let item = public_item(
            &json!({"id":"a","analyst":{"note":"secret","saved":true},"brief":{"status":"completed","data":{"title_en":"T","impact":"high","rationale_en":"x"}},"rank":{"score":80,"band":"high","reasons":[1]}}),
        );
        assert_eq!(item["analyst"]["saved"], false);
        assert!(item["analyst"].get("note").is_none());
        assert!(item["brief"]["data"].get("impact").is_none());
        assert!(item["rank"].get("reasons").is_none());
    }
}
