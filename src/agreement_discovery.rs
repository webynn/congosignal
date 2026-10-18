//! Bounded polling of configured public indexes. Discovery is not signature verification.
use crate::{ApiResult, App};
use anyhow::Result;
use axum::{Json, extract::State};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use url::Url;
const SOURCES: &[(&str, &str, &str)] = &[
    (
        "congomines",
        "CongoMines",
        "https://congomines.org/fr/all-documents/?pgdc_search=Convention&pgdc_per_page=100",
    ),
    (
        "economy",
        "DRC Ministry of Economy",
        "https://economie.gouv.cd/actualites",
    ),
    ("apcsc", "APCSC", "https://www.apcsc.gouv.cd/"),
];
pub fn interval() -> u64 {
    std::env::var("AGREEMENTS_POLL_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(21600)
        .max(3600)
}
pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql("CREATE TABLE IF NOT EXISTS agreement_discovery_sources(id TEXT PRIMARY KEY,checked_at TEXT,last_success_at TEXT,error TEXT); CREATE TABLE IF NOT EXISTS agreement_discoveries(id TEXT PRIMARY KEY,url TEXT UNIQUE NOT NULL,data TEXT NOT NULL); CREATE TABLE IF NOT EXISTS agreement_discovery_queue(url TEXT PRIMARY KEY,title TEXT NOT NULL,source_id TEXT NOT NULL,status TEXT NOT NULL DEFAULT 'pending',error TEXT);").execute(db).await?;
    for (id, _, _) in SOURCES {
        sqlx::query("INSERT OR IGNORE INTO agreement_discovery_sources(id) VALUES(?)")
            .bind(id)
            .execute(db)
            .await?;
    }
    Ok(())
}
pub async fn records(db: &SqlitePool) -> Result<Vec<Value>> {
    let rows =
        sqlx::query_scalar::<_, String>("SELECT data FROM agreement_discoveries ORDER BY id")
            .fetch_all(db)
            .await?;
    rows.iter().map(|s| Ok(serde_json::from_str(s)?)).collect()
}
pub async fn status(db: &SqlitePool) -> Result<Value> {
    let mut sources = Vec::new();
    for r in sqlx::query("SELECT * FROM agreement_discovery_sources ORDER BY id")
        .fetch_all(db)
        .await?
    {
        sources.push(json!({"id":r.get::<String,_>("id"),"checked_at":r.get::<Option<String>,_>("checked_at"),"last_success_at":r.get::<Option<String>,_>("last_success_at"),"error":r.get::<Option<String>,_>("error")}));
    }
    let pending: i64 =
        sqlx::query_scalar("SELECT count(*) FROM agreement_discovery_queue WHERE status!='saved'")
            .fetch_one(db)
            .await?;
    Ok(
        json!({"interval_seconds":interval(),"sources":sources,"pending":pending,"mode":"source_indexes_no_paid_ai","coverage":"Configured indexes only; candidate records require verification"}),
    )
}
pub async fn refresh(State(app): State<App>) -> ApiResult<Json<Value>> {
    sqlx::query("UPDATE agreement_discovery_sources SET checked_at=NULL")
        .execute(&app.db)
        .await?;
    Ok(Json(json!({"status":"queued"})))
}
fn links(html: &str, base: &Url) -> Vec<(String, String)> {
    let doc = scraper::Html::parse_document(html);
    let sel = scraper::Selector::parse("a[href]").unwrap();
    let mut found = std::collections::BTreeMap::new();
    for a in doc.select(&sel) {
        let title = a
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let Ok(mut u) = base.join(a.value().attr("href").unwrap()) else {
            continue;
        };
        u.set_fragment(None);
        if u.scheme() != "https"
            || (u.host_str() != base.host_str()
                && !(base.host_str() == Some("congomines.org")
                    && u.host_str() == Some("wpstorage7ceb827ed5.blob.core.windows.net")))
        {
            continue;
        }
        let t = format!("{} {}", title, u.path()).to_lowercase();
        if [
            "accord",
            "agreement",
            "convention",
            "avenant",
            "memorandum",
            "protocole",
            "concession",
        ]
        .iter()
        .any(|k| t.contains(k))
            && title.chars().count() > 18
            && u.path() != base.path()
        {
            found.insert(u.to_string(), title);
        }
    }
    found.into_iter().take(100).collect()
}
async fn collect(
    db: &SqlitePool,
    f: &mut crate::evidence::Fetcher,
    url: &str,
    title: &str,
    publisher: &str,
) -> Result<()> {
    let (base, bytes, ty) = f.get(url).await?;
    let id = format!(
        "discovered-{}",
        &crate::extract::digest(url.as_bytes())[..20]
    );
    let hash = crate::extract::digest(&bytes);
    let mut documents = Vec::new();
    tokio::fs::create_dir_all("data/agreements").await?;
    let is_pdf = bytes.starts_with(b"%PDF-");
    let (text, pdfs) = if is_pdf {
        (String::new(), Vec::new())
    } else {
        anyhow::ensure!(ty.contains("html"), "unsupported discovery document");
        let html = scraper::Html::parse_document(&String::from_utf8_lossy(&bytes));
        let text = html
            .select(&scraper::Selector::parse("h1,p").unwrap())
            .map(|n| n.text().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n\n");
        let pdfs = html
            .select(&scraper::Selector::parse("a[href]").unwrap())
            .filter_map(|n| base.join(n.value().attr("href")?).ok())
            .filter(|u| u.path().to_lowercase().ends_with(".pdf"))
            .map(|u| u.to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .take(3)
            .collect::<Vec<_>>();
        (text, pdfs)
    };
    let docid = format!("{id}-source");
    tokio::fs::write(
        format!(
            "data/agreements/{docid}.{}",
            if is_pdf { "pdf" } else { "html" }
        ),
        &bytes,
    )
    .await?;
    documents.push(json!({"id":docid,"role":"discovery_source","format":if is_pdf{"pdf"}else{"html"},"status":if is_pdf{"ocr_required"}else if text.is_empty(){"unavailable"}else{"available"},"url":url,"sha256":hash,"text":text,"pages":[]}));
    for (n, link) in pdfs.iter().enumerate() {
        let did = format!("{id}-pdf-{n}");
        let mut d = json!({"id":did,"role":"unverified_document","format":"pdf","url":link,"status":"unavailable","text":"","pages":[]});
        match f.get(link).await {
            Ok((_, b, _)) if b.starts_with(b"%PDF-") => {
                tokio::fs::write(format!("data/agreements/{did}.pdf"), &b).await?;
                d["status"] = json!("ocr_required");
                d["sha256"] = json!(crate::extract::digest(&b));
            }
            Ok(_) => d["error"] = json!("Source did not return a PDF"),
            Err(e) => d["error"] = json!(e.to_string()),
        }
        documents.push(d);
    }
    let t = title.to_lowercase();
    let mut categories = Vec::new();
    if ["paix", "peace", "cessez", "ceasefire"]
        .iter()
        .any(|k| t.contains(k))
    {
        categories.push("peace")
    }
    if ["min", "sicomines", "cuivre"].iter().any(|k| t.contains(k)) || publisher == "CongoMines" {
        categories.push("minerals")
    }
    if [
        "infrastructure",
        "route",
        "lobito",
        "concession",
        "construction",
        "nkamba",
    ]
    .iter()
    .any(|k| t.contains(k))
    {
        categories.push("infrastructure")
    }
    let v = json!({"id":id,"title":title,"date":null,"date_basis":"Automatically discovered source; signing date and parties require review","signed_term":"Unverified","categories":categories,"instrument":"discovery_candidate","signature_status":"needs_review","parties":"Unverified","scope":"discovery","publisher":publisher,"url":url,"license":"Underlying publisher rights apply","discovered_at":chrono::Utc::now().to_rfc3339(),"documents":documents});
    sqlx::query("INSERT OR IGNORE INTO agreement_discoveries VALUES(?,?,?)")
        .bind(id)
        .bind(url)
        .bind(v.to_string())
        .execute(db)
        .await?;
    Ok(())
}
pub async fn worker(db: SqlitePool) -> Result<()> {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(30));
    loop {
        tick.tick().await;
        let result = cycle(&db).await;
        if let Err(e) = result {
            tracing::warn!(error=%e,"Agreement discovery cycle failed");
        }
    }
}
async fn cycle(db: &SqlitePool) -> Result<()> {
    let mut f = crate::evidence::Fetcher::new()?;
    for (sid, publisher, url) in SOURCES {
        let checked: Option<String> =
            sqlx::query_scalar("SELECT checked_at FROM agreement_discovery_sources WHERE id=?")
                .bind(sid)
                .fetch_one(db)
                .await?;
        if checked
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .is_some_and(|d| {
                (chrono::Utc::now() - d.with_timezone(&chrono::Utc)).num_seconds()
                    < interval() as i64
            })
        {
            continue;
        }
        let now = chrono::Utc::now().to_rfc3339();
        let result=async {
   let (base,bytes,_)=f.get(url).await?;
   let known=crate::agreements::catalogue();
   for (link,title) in links(&String::from_utf8_lossy(&bytes),&base){
    if known["items"].as_array().unwrap().iter().any(|v|v["url"]==link){continue}
    sqlx::query("INSERT OR IGNORE INTO agreement_discovery_queue(url,title,source_id) VALUES(?,?,?)").bind(link).bind(title).bind(sid).execute(db).await?;
   }
   let queue=sqlx::query("SELECT url,title FROM agreement_discovery_queue WHERE source_id=? AND status!='saved' ORDER BY CASE status WHEN 'pending' THEN 0 ELSE 1 END,rowid LIMIT 10").bind(sid).fetch_all(db).await?;
   for row in queue {
    let link:String=row.get("url"); let title:String=row.get("title");
    let error=collect(db,&mut f,&link,&title,publisher).await.err().map(|e|e.to_string());
    sqlx::query("UPDATE agreement_discovery_queue SET status=?,error=? WHERE url=?").bind(if error.is_some(){"failed"}else{"saved"}).bind(error).bind(link).execute(db).await?;
   }
   Ok::<(),anyhow::Error>(())
  }.await;
        let err = result.err().map(|e| e.to_string());
        sqlx::query("UPDATE agreement_discovery_sources SET checked_at=?,last_success_at=CASE WHEN ? IS NULL THEN ? ELSE last_success_at END,error=? WHERE id=?").bind(&now).bind(&err).bind(&now).bind(&err).bind(sid).execute(db).await?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_filters_links() {
        let base = Url::parse("https://economie.gouv.cd/actualites").unwrap();
        let html = r#"<a href='/actualites/accord-paix#top'>Signature d'un accord de paix</a><a href='/actualites/accord-paix'>Signature d'un accord de paix</a><a href='http://127.0.0.1/accord'>Un accord malveillant externe</a><a href='/weather'>Conditions météorologiques</a>"#;
        let l = links(html, &base);
        assert_eq!(l.len(), 1);
        assert!(!l[0].0.contains('#'));
    }
    #[tokio::test]
    async fn schema_and_queue_survive_init() {
        let db = SqlitePool::connect("sqlite::memory:").await.unwrap();
        init(&db).await.unwrap();
        init(&db).await.unwrap();
        assert_eq!(
            status(&db).await.unwrap()["sources"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert!(records(&db).await.unwrap().is_empty());
    }
}
