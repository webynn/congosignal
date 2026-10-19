//! Outbound notifications: email over SMTP and Telegram bot messages.
//! Credentials come from the environment and never reach the browser:
//! SMTP_HOST, SMTP_PORT (587 STARTTLS; 465 implicit TLS), SMTP_USERNAME, SMTP_PASSWORD, SMTP_FROM;
//! TELEGRAM_BOT_TOKEN. APP_PUBLIC_URL (else APP_ORIGIN) is used for links back to the dashboard.
use anyhow::{Context, Result, ensure};
use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart, header::ContentType};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use serde_json::{Value, json};

fn env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
pub fn public_url() -> String {
    env("APP_PUBLIC_URL")
        .or_else(|| env("APP_ORIGIN"))
        .unwrap_or_else(|| "http://127.0.0.1:8080".into())
        .trim_end_matches('/')
        .to_string()
}
pub fn email_configured() -> bool {
    env("SMTP_HOST").is_some() && env("SMTP_FROM").is_some()
}
pub fn telegram_configured() -> bool {
    env("TELEGRAM_BOT_TOKEN").is_some()
}
/// Channel status for the UI: what is configured, never the secrets themselves.
pub fn status() -> Value {
    json!({"email":email_configured(),"telegram":telegram_configured(),"email_from":env("SMTP_FROM"),"public_url":public_url()})
}
/// Plausible email address (no header injection, one recipient).
pub fn valid_email(s: &str) -> bool {
    let s = s.trim();
    s.len() <= 254
        && s.matches('@').count() == 1
        && !s.chars().any(|c| {
            c.is_whitespace() || c.is_control() || matches!(c, ',' | ';' | '<' | '>' | '"')
        })
        && s.split('@').all(|p| !p.is_empty())
        && s.rsplit('@').next().is_some_and(|d| d.contains('.'))
}
/// Telegram chat ids are integers (negative for groups) or @channelusername.
pub fn valid_chat(s: &str) -> bool {
    let s = s.trim();
    (s.len() <= 32
        && s.trim_start_matches('-')
            .chars()
            .all(|c| c.is_ascii_digit())
        && s.trim_start_matches('-').len() >= 3)
        || (s.starts_with('@')
            && s.len() <= 33
            && s[1..]
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_'))
}
pub struct Mail<'a> {
    pub to: &'a str,
    pub subject: &'a str,
    pub text: &'a str,
    pub html: &'a str,
    pub attachment: Option<(&'a str, &'a str, Vec<u8>)>,
}
pub async fn email(m: Mail<'_>) -> Result<()> {
    ensure!(
        email_configured(),
        "Email is not configured (SMTP_HOST, SMTP_FROM)"
    );
    ensure!(valid_email(m.to), "Invalid email address");
    let from: Mailbox = env("SMTP_FROM")
        .unwrap()
        .parse()
        .context("SMTP_FROM is not a valid address")?;
    let body = MultiPart::alternative()
        .singlepart(SinglePart::plain(m.text.to_string()))
        .singlepart(SinglePart::html(m.html.to_string()));
    let body = match m.attachment {
        Some((name, mime, bytes)) => MultiPart::mixed()
            .multipart(body)
            .singlepart(Attachment::new(name.to_string()).body(bytes, ContentType::parse(mime)?)),
        None => body,
    };
    let message = Message::builder()
        .from(from)
        .to(m.to.trim().parse()?)
        .subject(m.subject.chars().take(200).collect::<String>())
        .multipart(body)?;
    let host = env("SMTP_HOST").unwrap();
    let port: u16 = env("SMTP_PORT").and_then(|p| p.parse().ok()).unwrap_or(587);
    let mut builder = if port == 465 {
        AsyncSmtpTransport::<Tokio1Executor>::relay(&host)?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&host)?
    }
    .port(port)
    .timeout(Some(std::time::Duration::from_secs(20)));
    if let (Some(u), Some(p)) = (env("SMTP_USERNAME"), env("SMTP_PASSWORD")) {
        builder = builder.credentials(Credentials::new(u, p));
    }
    builder
        .build()
        .send(message)
        .await
        .context("SMTP delivery failed")?;
    Ok(())
}
fn telegram_url(method: &str) -> Result<String> {
    let token =
        env("TELEGRAM_BOT_TOKEN").context("Telegram is not configured (TELEGRAM_BOT_TOKEN)")?;
    Ok(format!("https://api.telegram.org/bot{token}/{method}"))
}
fn client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()?)
}
/// Never surface the bot token in error text.
fn telegram_error(r: &Value, status: reqwest::StatusCode) -> anyhow::Error {
    anyhow::anyhow!(
        "Telegram HTTP {}: {}",
        status.as_u16(),
        r["description"].as_str().unwrap_or("request failed")
    )
}
/// Telegram message in its HTML subset; messages over Telegram's 4096-char limit are cut.
pub async fn telegram(chat: &str, html: &str) -> Result<()> {
    ensure!(valid_chat(chat), "Invalid Telegram chat id");
    let text: String = html.chars().take(4000).collect();
    let r = client()?
        .post(telegram_url("sendMessage")?)
        .json(&json!({"chat_id":chat.trim(),"text":text,"parse_mode":"HTML","disable_web_page_preview":true}))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Telegram connection failed: {}", e.without_url()))?;
    let status = r.status();
    let v: Value = r.json().await.unwrap_or_default();
    ensure!(
        status.is_success() && v["ok"] == true,
        telegram_error(&v, status)
    );
    Ok(())
}
pub async fn telegram_document(
    chat: &str,
    name: &str,
    mime: &str,
    bytes: Vec<u8>,
    caption: &str,
) -> Result<()> {
    ensure!(valid_chat(chat), "Invalid Telegram chat id");
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(name.to_string())
        .mime_str(mime)?;
    let form = reqwest::multipart::Form::new()
        .text("chat_id", chat.trim().to_string())
        .text("caption", caption.chars().take(1000).collect::<String>())
        .text("parse_mode", "HTML")
        .part("document", part);
    let r = client()?
        .post(telegram_url("sendDocument")?)
        .multipart(form)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Telegram connection failed: {}", e.without_url()))?;
    let status = r.status();
    let v: Value = r.json().await.unwrap_or_default();
    ensure!(
        status.is_success() && v["ok"] == true,
        telegram_error(&v, status)
    );
    Ok(())
}
/// Chats that recently messaged the bot, so users can pick their chat id instead of looking it up.
pub async fn telegram_chats() -> Result<Value> {
    let r = client()?
        .get(telegram_url("getUpdates")?)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Telegram connection failed: {}", e.without_url()))?;
    let status = r.status();
    let v: Value = r.json().await.unwrap_or_default();
    ensure!(
        status.is_success() && v["ok"] == true,
        telegram_error(&v, status)
    );
    let mut chats: Vec<Value> = Vec::new();
    for u in v["result"].as_array().into_iter().flatten() {
        let c = u["message"]["chat"]
            .as_object()
            .or(u["channel_post"]["chat"].as_object())
            .or(u["my_chat_member"]["chat"].as_object());
        if let Some(c) = c {
            let id = c.get("id").map(|i| i.to_string()).unwrap_or_default();
            if !id.is_empty() && !chats.iter().any(|x| x["id"] == id) {
                let label = c
                    .get("title")
                    .or(c.get("username"))
                    .or(c.get("first_name"))
                    .and_then(Value::as_str)
                    .unwrap_or("chat");
                chats.push(json!({"id":id,"label":label,"type":c.get("type")}));
            }
        }
    }
    let me = client()?.get(telegram_url("getMe")?).send().await.ok();
    let bot = match me {
        Some(r) => r
            .json::<Value>()
            .await
            .ok()
            .and_then(|v| v["result"]["username"].as_str().map(str::to_string)),
        None => None,
    };
    Ok(json!({"chats":chats,"bot":bot}))
}
/// Minimal HTML escaping for email and Telegram bodies.
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn addresses_and_chats_are_validated() {
        assert!(valid_email("analyst@example.org"));
        for bad in [
            "a@b",
            "x y@z.org",
            "a@b.org, c@d.org",
            "a@b.org\nBcc: x@y.z",
            "@b.org",
            "a@",
        ] {
            assert!(!valid_email(bad), "{bad}");
        }
        assert!(valid_chat("123456789"));
        assert!(valid_chat("-1001234567890"));
        assert!(valid_chat("@congo_signal"));
        assert!(!valid_chat("12"));
        assert!(!valid_chat("123; drop"));
        assert_eq!(esc("<b>&\""), "&lt;b&gt;&amp;&quot;");
    }
}
