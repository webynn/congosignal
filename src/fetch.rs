use crate::extract::{ORIGIN, normalize};
use anyhow::{Result, bail, ensure};
use reqwest::{Client, StatusCode};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use url::Url;

const AGENT: &str = "DrcOsintWorker";
pub struct Fetcher {
    client: Client,
    last: Instant,
    delay: Duration,
    robots: Option<(Instant, String)>,
}
impl Fetcher {
    pub fn new() -> Result<Self> {
        let ua =
            std::env::var("CRAWLER_USER_AGENT").unwrap_or_else(|_| "DrcOsintWorker/0.1".into());
        ensure!(
            ua.starts_with(AGENT),
            "CRAWLER_USER_AGENT must start with DrcOsintWorker"
        );
        Ok(Self {
            client: Client::builder()
                .user_agent(ua)
                .timeout(Duration::from_secs(30))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            last: Instant::now(),
            delay: Duration::from_secs(2),
            robots: None,
        })
    }
    async fn raw(&mut self, mut url: Url, robots: bool) -> Result<(Url, String)> {
        for _ in 0..6 {
            for attempt in 0..3 {
                sleep(self.delay.saturating_sub(self.last.elapsed())).await;
                self.last = Instant::now();
                let response = self.client.get(url.clone()).send().await?;
                let status = response.status();
                if status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
                    if attempt == 2 {
                        bail!("upstream {status}: retries exhausted");
                    }
                    let wait = response
                        .headers()
                        .get("retry-after")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| {
                            s.parse::<u64>().ok().or_else(|| {
                                chrono::DateTime::parse_from_rfc2822(s).ok().map(|d| {
                                    (d.with_timezone(&chrono::Utc) - chrono::Utc::now())
                                        .num_seconds()
                                        .max(0) as u64
                                })
                            })
                        })
                        .unwrap_or(2_u64.pow(attempt + 1));
                    ensure!(
                        wait <= 120,
                        "upstream requested a long cooldown; retry this job later"
                    );
                    sleep(Duration::from_secs(wait)).await;
                    continue;
                }
                if status.is_redirection() {
                    let target = response
                        .headers()
                        .get("location")
                        .ok_or_else(|| anyhow::anyhow!("redirect without location"))?
                        .to_str()?;
                    let joined = url.join(target)?;
                    // Validate origin but preserve the actual redirect path/query.
                    normalize(joined.as_str())?;
                    if !robots {
                        self.allowed(&joined)?;
                    }
                    url = joined;
                    break;
                }
                if robots && status == StatusCode::NOT_FOUND {
                    return Ok((url, String::new()));
                }
                response.error_for_status_ref()?;
                if !robots {
                    ensure!(
                        response
                            .headers()
                            .get("content-type")
                            .and_then(|v| v.to_str().ok())
                            .is_some_and(|v| v.contains("text/html")),
                        "expected HTML response"
                    );
                }
                let max = if robots { 512_000 } else { 5_000_000 };
                ensure!(
                    response.content_length().unwrap_or(0) <= max,
                    "response too large"
                );
                let mut response = response;
                let mut bytes = Vec::new();
                while let Some(chunk) = response.chunk().await? {
                    ensure!(
                        bytes.len() + chunk.len() <= max as usize,
                        "response too large"
                    );
                    bytes.extend_from_slice(&chunk);
                }
                return Ok((url, String::from_utf8(bytes)?));
            }
        }
        bail!("too many redirects")
    }
    fn allowed(&self, url: &Url) -> Result<()> {
        let (_, rules) = self
            .robots
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("robots policy unavailable"))?;
        ensure!(
            robotstxt::DefaultMatcher::default().one_agent_allowed_by_robots(
                rules,
                AGENT,
                url.as_str()
            ),
            "robots.txt disallows this URL"
        );
        Ok(())
    }
    pub async fn get(&mut self, url: &Url) -> Result<(Url, String)> {
        normalize(url.as_str())?;
        if self
            .robots
            .as_ref()
            .is_none_or(|(at, _)| at.elapsed() > Duration::from_secs(3600))
        {
            let (_, rules) = self
                .raw(Url::parse(&format!("{ORIGIN}/robots.txt"))?, true)
                .await?;
            // Conservatively honor any Crawl-delay directive, even from other groups.
            let mut delay = 2.0_f64;
            for line in rules.lines() {
                if let Some((k, v)) = line.split('#').next().unwrap_or("").split_once(':')
                    && k.trim().eq_ignore_ascii_case("crawl-delay")
                    && let Ok(n) = v.trim().parse::<f64>()
                    && n.is_finite()
                {
                    delay = delay.max(n);
                }
            }
            ensure!(delay <= 300., "crawl delay exceeds supported limit");
            self.delay = Duration::from_secs_f64(delay);
            self.robots = Some((Instant::now(), rules));
        }
        self.allowed(url)?;
        self.raw(url.clone(), false).await
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn robots_rules() {
        let mut m = robotstxt::DefaultMatcher::default();
        let r = "User-agent: *\nDisallow: /search/\nDisallow: /*/private\n";
        assert!(!m.one_agent_allowed_by_robots(
            r,
            "DrcOsintWorker",
            "https://actualite.cd/search/node"
        ));
        assert!(m.one_agent_allowed_by_robots(
            r,
            "DrcOsintWorker",
            "https://actualite.cd/2026/10/12/test"
        ));
    }
}
