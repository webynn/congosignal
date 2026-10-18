# Congo Signal · DRC OSINT worker

Rust/Axum API and background collector for public Actualite.cd articles. SQLite keeps the job queue, article records, and editorial revisions with source HTML. This is an ingestion foundation for DRC research; collected reporting is source material, not independently verified intelligence.

## Multi-user authentication

See [authentication setup](docs/authentication.txt) and [environment template](config/authentication.env.example). ZITADEL OIDC must be configured before public deployment. The default loopback development mode is a local owner, not a multi-user login. Non-loopback binding requires `AUTH_MODE=oidc`. Provider budgets and per-member allowances are separate.

## Run

With Rust 1.99+ installed:

```sh
mkdir -p data
cargo run --locked
```

On this machine an isolated toolchain is available:

```sh
cd '/Users/webynn/Source Control/drc-osint-worker'
export CARGO_HOME="$PWD/.toolchain/cargo"
export RUSTUP_HOME="$PWD/.toolchain/rustup"
export PATH="$CARGO_HOME/bin:$PATH"
cargo run --locked
```

The API binds to `127.0.0.1:8080`. Configuration uses environment variables (`.env` is loaded unless `LOAD_DOTENV=0`):

| Variable | Default | Purpose |
| --- | --- | --- |
| `BIND_ADDR` | `127.0.0.1:8080` | Listening address |
| `DATABASE_URL` | `sqlite://data/osint.db` | SQLite file; create its parent directory first |
| `API_TOKEN` | unset | Bearer token for all `/v1` endpoints; at least 24 characters required when binding beyond loopback |
| `POLL_SECONDS` | unset | Optional homepage collection interval, at least 60 seconds; 900 is a reasonable starting value |
| `CRAWLER_USER_AGENT` | `DrcOsintWorker/0.1` | Must start with `DrcOsintWorker`; append an operator contact for deployment |
| `RUST_LOG` | `info` | Logging filter |

Run **one process per database**. Multiple workers need leased job claiming and shared rate limiting before horizontal scaling. The API shuts down on Ctrl-C; interrupted running jobs return to pending on restart.

## Collect

```sh
curl -X POST http://127.0.0.1:8080/v1/jobs \
  -H 'Content-Type: application/json' \
  -d '{"url":"https://actualite.cd/2026/10/12/musique-gally-garvey-devoile-adn-un-album-de-22-titres-porte-notamment-par-des"}'
```

Returns HTTP 202 with `job_id` and `status_url`. Poll that URL until `completed` or `failed`. Completed article jobs return the article ID; discovery jobs return child job IDs, which finish separately.

Submit `https://actualite.cd/`, `https://actualite.cd/actualite`, or a category URL such as `https://actualite.cd/category/actualite/securite` to discover up to 50 article links on that page. Discovery does not recursively traverse archives or pagination. Query strings are stripped. Repeat a completed article submission to check for updates; active duplicate jobs are coalesced.

When `API_TOKEN` is set, add `-H "Authorization: Bearer $API_TOKEN"` to API requests.

## Query

| Method | Endpoint | Result |
| --- | --- | --- |
| GET | `/health` | Process health, unauthenticated |
| POST | `/v1/jobs` | Queue an article or discovery page |
| GET | `/v1/jobs/{id}` | Persistent job status, result or error |
| GET | `/v1/articles?q=Kinshasa&category=Culture&limit=20&offset=0` | Paginated article records |
| GET | `/v1/articles?since=2026-10-01T00%3A00%3A00Z` | Filter by publication time |
| GET | `/v1/articles/{id}` | Article, first seen and last seen times |
| GET | `/v1/articles/{id}/versions` | Latest 100 editorial revisions |
| GET | `/v1/stats` | Counts by category and job status |

Search is a literal substring over title and body. SQLite's default `lower()` is case insensitive for ASCII only; accents are preserved and accent/case folding for French is future work. Category matching is exact. `since` accepts RFC3339, and pagination limits are clamped to 1–100. These ingestion endpoints return source material. The Congo Signal endpoints below add bilingual AI briefs, explicit place mentions and priority ranking; they do not establish event attribution or cross-source corroboration.

## Provenance and collection behavior

Records contain canonical/fetched URLs, source, title, description, category, author, author extraction method, language, publication/modification/fetch timestamps, paragraphs, plain text, SHA-256 hashes, and extractor version. Timestamps are normalized to UTC when the source provides them. Missing metadata remains null. The final-bold-paragraph author fallback is explicitly labeled `final_bold_paragraph_heuristic` and may require analyst review.

Editorial changes create revisions. Identical text and metadata do not create revisions just because ads or fetch time changed. Each stored revision retains its acquisition HTML and matching `html_sha256` in SQLite's `versions` table. The current record's HTML hash describes the latest fetch; unchanged fetch HTML is not retained. Query retained evidence directly, for example: `SELECT html,data FROM versions WHERE article_id = ?`.

Requests are serialized with a minimum two-second delay, a 30-second timeout, a 5 MB response cap, and origin validation on every redirect. The worker fetches robots.txt before collection and refreshes it hourly, honors allow/disallow rules and conservatively applies crawl delays. Missing robots.txt (404) permits collection; other policy fetch failures fail the job. HTTP 429/5xx responses get up to three attempts with backoff and Retry-After handling; long requested cooldowns fail the job for later resubmission. Network errors are recorded as failures without automatic retry. The nominal pending queue cap is 500 (concurrent submissions may briefly exceed it).

The source adapter targets the site's observed Drupal body block, excluding navigation, related stories, and footer material. A missing or suspiciously short body fails visibly instead of storing the whole page. HTTP clients only fetch the `https://actualite.cd` origin; there is no arbitrary-URL fetch endpoint. Public deployment should put TLS and request limits in front of the API. Raw HTML is stored as evidence and never executed.

## Docker

```sh
docker build -t drc-osint-worker .
docker run --rm -p 8080:8080 \
  -e API_TOKEN='replace-with-a-long-random-token' \
  -e POLL_SECONDS=900 \
  -v drc-osint-data:/data drc-osint-worker
```

## Validation

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Tests use synthetic HTML modeled on the observed site structure, exercising French text, provenance, navigation exclusion, URL restrictions, discovery deduplication, robots rules, and persistent article revisions. A live smoke test should additionally submit an article through the API and inspect the completed record. Docker requires a Docker runtime and is a separate deployment check.

## Congo Signal dashboard and AI

Open **http://127.0.0.1:8080/** for the research workspace. The static dashboard is embedded in the Rust binary, so rebuild after frontend edits. Start both the collector and AI worker with:

```sh
./scripts/start.sh
```

The development workspace uses Claude Haiku 4.5 and Mapbox, configured in a private `.env` file (owner-only permissions, excluded from Git and Docker builds). The binary reads `.env` at startup without overriding existing environment variables; set `LOAD_DOTENV=0` to disable loading for tests. Never put the Claude key in frontend files. Configuration:

| Variable | Default | Purpose |
| --- | --- | --- |
| `AI_ENABLED` | disabled in binary; `1` in start script | Enable the separate, serial AI worker |
| `AI_PROVIDER` | `ollama` | Set `claude` to use the Anthropic Messages API |
| `ANTHROPIC_API_KEY` | none | Server-only Claude credential |
| `CLAUDE_MODEL` | `claude-haiku-4-5-20251001` | Claude model for new tasks |
| `MAPBOX_PUBLIC_TOKEN` | none | Existing public `pk.` token; secret `sk.` tokens are never returned to browsers |
| `OLLAMA_URL` | `http://127.0.0.1:11434` | Operator-configured Ollama endpoint, never exposed as a browser setting |
| `OLLAMA_MODEL` | `gemma4:12b` | Model to use for new tasks |

The interface supports English and French, topic/place filters, accent-insensitive search across loaded reports and briefs, priority/newest sorting, geographic mentions, saved reports, analyst review notes, and filtered JSON export. The dashboard loads the latest 200 articles, expandable to 500; the API supports offset pagination for larger collections. API-token deployments ask for a bearer token in Connection settings and retain it only in the current tab's session storage. UI assets are public but all research data and mutation endpoints use the existing `/v1` authentication policy.

Priority is an explainable attention score (0–100), **not a confidence or truth score**. Category contributes 5–35, impact-related terms up to 24, recency up to 20, a recognized DRC location mention 6, and optional AI impact assessment up to 15. High starts at 65, medium at 40. Map markers use a 20-place gazetteer and approximate town centers; they are not verified incident locations. All reporting currently comes from Actualite.cd and is explicitly marked single-source/unverified.

Click **Analyze top 5 reports** to queue bilingual briefs for the highest-ranked unprocessed reports in the current filter. Reports show English headlines once their brief is ready; French remains the original. Each report also has an on-demand **English translation** tab for its full text. Claude sends the requested public article text to Anthropic for inference; Ollama remains an optional local provider. Generation is on demand and consumes the configured provider’s usage. The source article is treated as untrusted input; Claude uses a schema-only result carrier with no executable tools or action permissions. Model outputs remain fallible and require source review.

Provider and model are retained per task, including queued tasks. Switching providers preserves completed cached results and their original provenance. Enrichment tasks are durable and serialized separately from scraping. Results are cached by article ID, source fingerprint and task. Changed source content/title/category/publication date invalidates the displayed analysis. Failed tasks can be retried explicitly; interrupted tasks return to pending after restart. Briefs use up to 14,000 source characters and record/display truncation. Full translations support up to 60,000 characters, reject individual paragraphs above 9,000 characters, and process small paragraph batches. Incomplete or misaligned translations fail instead of silently saving partial text. Analyst notes are independent of AI output.

| Method | Endpoint | Purpose |
| --- | --- | --- |
| GET | `/v1/config` | Browser-safe Mapbox public token only; same API authentication, no-store response |
| GET | `/v1/intelligence?limit=200&offset=0` | Ranked recent articles, places, briefs and review state |
| GET | `/v1/intelligence/status` | AI provider, model and task counts; Claude availability means key configured, not a live billing/connectivity check |
| GET | `/v1/intelligence/{id}` | Original article, brief, translation and notes |
| POST | `/v1/intelligence/analyze` | `{ "article_id": "...", "task": "brief" }` or task `translation` |
| POST | `/v1/intelligence/{id}/notes` | `{ "saved": true, "review": "follow_up", "note": "..." }` |

Review values: `unreviewed`, `reviewed`, `follow_up`. Reviewed does not certify claims. Task statuses: `not_requested`, `pending`, `running`, `completed`, `failed`.

Offline integration validation (uses a temporary SQLite database and a deterministic mock model; no production data or real inference):

```sh
cargo test --locked
cargo build --locked
node --check dashboard/app.js
python3 tests/intelligence_smoke.py
TEST_PROVIDER=claude python3 tests/intelligence_smoke.py
python3 tests/social_smoke.py
```

### Hosting the AI-enabled version

A 4 GB VPS can run the collector and dashboard with `AI_PROVIDER=claude`, because inference runs through Anthropic. Configure the server environment with the Claude key and Mapbox public token, use HTTPS and the existing API bearer-token protection, and persist the SQLite data directory. API and Mapbox usage are billed to the accounts owning the configured keys. Do not embed `.env` in an image; inject it at runtime (for example Docker `--env-file`). No hosting deployment is included in this workspace.

For local Ollama, run the service separately with the configured model installed. The 12B model does not fit a 4 GB VPS; use a separately provisioned inference service or appropriately sized hardware. In Docker, `127.0.0.1:11434` refers to the container. Keep one API/worker process per SQLite database.

The interactive map loads Mapbox GL JS 3.33.0 and the Mapbox Light style with the configured public token. Allow the deployed origin in any existing Mapbox URL restrictions. It retains one map instance through feed refreshes. Without a public token, supported WebGL or a successful initial style load, the bundled static outline remains available. Coordinates represent approximate place mentions; zoom does not make them precise incident locations. Mapbox attribution is displayed on the map.

### References and attribution

Original implementation inspired by [OpenCTI's source reliability and confidence distinctions](https://docs.opencti.io/latest/usage/reliability-confidence/) and [Ushahidi's report visualization views](https://docs.ushahidi.com/platform-user-manual/5.-modes-for-visualizing-and-managing-data-on-your-deployment). No application code from these platforms was copied. The bundled simplified country outline comes from [world.geo.json](https://github.com/johan/world.geo.json), based on Natural Earth data. Map points are a small hand-curated gazetteer; source articles remain the evidence for place mentions.

Provider references: [Anthropic Messages API](https://platform.claude.com/docs/en/api/messages/create) and [Mapbox GL JS](https://docs.mapbox.com/mapbox-gl-js/example/simple-map/).

## Situation room (October 18, 2026)

The situation room (`/#situation`) is a map-first workspace inspired by God's Eye View. It provides selectable topic layers, a high-priority filter, rolling 24-hour/7-day/30-day publication windows, day selection in a publication histogram, map/satellite basemaps, flat/tilted camera views, location shortcuts, and a linked report rail. Search, date and topic selection apply to map counts and reports; selecting a place narrows the rail. Timeline bins use Kinshasa calendar dates (UTC+1), not inferred incident dates. Rolling windows use elapsed hours from the current time. Undated items appear only in the unrestricted window. Marker color represents the most common topic at that place; marker counts represent articles, not incidents. Articles without a recognized place remain in the report list and are counted separately.

The existing overview, library, saved reports, bilingual Claude analysis, translation, evidence and notes remain accessible. The loaded collection starts at 200 recent reports and can expand to 500; filters and the timeline describe that loaded set, not an exhaustive DRC event database. Satellite imagery is a basemap, not a live feed. No aircraft, vessel, camera or satellite-tracking feeds have been added.

Reference repository: https://github.com/bilawalsidhu/gods-eye-view, inspected at commit `93ea088f4556a5a65b3292a2d15142d5a6c562bc`, cloned separately to `../gods-eye-view-reference`. Its source code is MIT licensed; third-party datasets and models retain separate licenses. This adaptation implements the interface patterns in Congo Signal's own frontend and does not incorporate its source modules or bundled third-party datasets/assets. The reference clone retains the original LICENSE and notices.


## Primary-source evidence collection

The screenshots are mapped to 11 curated records, supplemented by three discovery listings, in `config/evidence-sources.json`: five UN Group of Experts reports (2022–2026), the UN report index and peacekeeping dashboard, HRW's Rwanda chapter and linked Kanombe social thread, the Rwanda MOD CAR statement, and IISS analysis. Records retain screenshot numbers, cited pages, dates, source category and attribution notes. The social thread is reference-only. Retrieval failures are explicit and do not erase earlier successful snapshots.

- `GET /v1/sources`: registered sources, provenance notes and retrieval status.
- `POST /v1/sources/{id}/refresh`: enqueue one registered source; poll the source list for completion.
- `GET /v1/evidence?source_id=s-2024-432&q=Kibumba&page=12&limit=20&offset=0`: search latest successfully collected documents by page (PDF physical pages are 1-based; HTML has a null page).
- `GET /v1/evidence/{document_id}`: full extracted text, outgoing links, provenance and content hash for a saved version.

These routes use the same bearer-token protection as the other API routes. The worker validates document identity, respects robots rules and crawl delays, checks each redirect against a fixed HTTPS origin list, and limits response sizes. It runs a separate durable serial queue; refreshes run on the configured schedule and can also be requested explicitly. Raw documents are archived under `EVIDENCE_ARCHIVE_DIR` (default `data/evidence`). Back up that directory with SQLite. Keep one worker process per database.

PDF extraction needs Poppler (`pdftotext` on PATH or `PDFTOTEXT_BIN`). The Dockerfile installs Poppler and persists archives under `/data/evidence`. Failed retrieval, an HTML challenge or a PDF identity mismatch never becomes a successful imported document. UN Digital Library fetch addresses are validated against the document symbol on retrieval.

Source claims remain attributed historical findings. Documents are separate from the news feed, map events and article AI tasks. Bilingual AI briefs are available on demand for document snapshots; full document translation and automatic claim verification are not provided. The peacekeeping screenshot's July 2026 Rwanda military-only total requires a matching dated export; scraping the catalogue does not import that numerical series. HRW's World Report chapter is related context linking the screenshot's thread, not a substitute for the original post. Government statements are labeled as such.


## Scheduled discovery and Sources & reports

`EVIDENCE_POLL_SECONDS` defaults to 21600 (six hours); set 0 to disable, or 3600–604800 for another interval. The running local instance uses six hours. Due checks run once a minute and survive restarts through persisted check times. Discovery checks HRW's DRC country page, Rwanda MOD news, IISS Strategic Comments, the UN expert index and the peacekeeping catalogue. Unavailable publishers remain failed and retry when due; robots rules are rechecked each schedule scan. Collection runs while the worker process is running, not while the computer is asleep or the service stopped.

Discovery is bounded to 30 unique publication links per landing-page fetch and 500 discovered records overall. It scans the current listing, not a complete historical archive. New URLs are normalized, deduplicated and saved before collection. Articles are screened using explicit DRC terms with word boundaries; ambiguous “Congo” alone does not qualify. Regional context is opt-in; excluded text remains archived for auditing. Relevance is a heuristic, not factual verification. No social-platform feed or numerical peacekeeping series is collected.

Open **Sources & reports** from the dashboard. Search the collected reports, include regional context, read original extracted text, inspect source status, or generate a bilingual brief. AI briefs are on demand, cached per document snapshot, retain model/hash/time, and show when input is truncated to 14,000 characters. Collected reports now appear in the combined feed; map markers are explicit place mentions, not verified events. Blocked sources are not silently substituted.

New authenticated endpoints:
- `GET /v1/evidence-feed`: DRC-relevant latest snapshots; `?scope=regional` also includes regional context; `?q=...` searches text.
- `POST /v1/evidence/{document_id}/analyze`: queue an English/French brief. Poll the feed for status.

Validation includes discovery URL deduplication, origin restrictions, language-aware relevance terms, due-time handling, authentication, snapshot preservation and bilingual brief cache behavior. Docker packaging was updated for configuration and Poppler but has not been built on this machine.


### In-site report reader

The map sidebar now groups all registered source records by publisher and shows collection counts and failures. Counts describe source records, including discovery listings, not verified incidents. Map markers now include explicit place mentions from collected reports as well as news.

Archived PDFs open inside the report reader; extracted text remains available below. `GET /v1/evidence/{document_id}/pdf` requires the same API authentication, validates the snapshot ID, PDF type and SHA-256 against the archived file, and serves only PDF bytes with inline disposition and no-store caching. The frontend fetches with the bearer header and uses a temporary blob URL, revoked on close. Missing/uncollected PDFs show an in-site explanation; no publisher restrictions are bypassed. HTML reports open as extracted text, not as embedded third-party webpages.

PDF endpoint checks cover authentication, inline delivery, missing files and tamper detection with an isolated test fixture. The current blocked UN PDFs cannot yet be previewed against a real downloaded snapshot.


## Combined feed

The situation room and library now combine the recent Actualite.cd news window with the latest successfully collected DRC-relevant evidence snapshots. Publisher filtering applies to cards and the situation-room map/timeline. Regional context is optional through the checkbox (`GET /v1/intelligence?scope=regional`), so Rwanda/CAR background is not silently classified as DRC reporting. Publication dates are preserved and normalized when known; unknown dates remain null. Day-only source dates are tagged with day precision. Blocked/uncollected documents and discovery catalogues are excluded.

The existing `limit`/`offset` select news; all matching latest evidence snapshots are appended, with `news_limit` and `evidence_count` returned explicitly. Therefore combined item counts can exceed the news limit. Rankings apply to the loaded set; evidence without a source topic receives the existing uncategorized score. Evidence cards open the in-site document reader; their original text and existing bilingual brief remain available. New evidence briefs can be queued through the top-five action or Sources & reports. Evidence bookmarking is not exposed in the library.


## Historical collection

Archive listing requests support a bounded integer `page` query (0–10000), for example `POST /v1/jobs` with `{"url":"https://actualite.cd/category/actualite/politique?page=5"}`. Article URLs still strip tracking/query parameters. Historical listing jobs skip URLs already in the article store, use the same robots rules/throttling and persist their child job IDs in the job result. An archive pass is not a complete historical crawl. The October 18 backfill requested pages 1, 5 and 10 of politics, security and society; `data/backfill-run.json` records its job IDs and starting count.

Both `POLL_SECONDS` (news) and `EVIDENCE_POLL_SECONDS` (primary sources) are configured to 21600 in the local environment. Avoid overriding `POLL_SECONDS` with 900 when restarting. Immediate manual jobs do not wait for the next scheduled poll.


## Dashboard (October 18, 2026 redesign)

The dashboard is one app shell with a persistent navigation rail, live collector status, an EN/FR switch and a dark/light theme toggle (defaults to the operating-system preference; the choice is remembered per browser). Views are addressable by URL hash:

| View | Hash | Purpose |
| --- | --- | --- |
| Briefing | `#briefing` (default) | KPIs, priority queue, X conversation pulse, hotspots, most active accounts, collection health |
| Situation room | `#situation` | Map, layers, publication timeline and report rail |
| Reports / Saved | `#reports`, `#saved` | Filterable ranked library with analyst bookmarks |
| Social intelligence | `#social` | X topic volume, post archive, accounts, hashtags, mentions and linked domains |
| Account dossier | `#account/<handle>` | Shareable dossier for one X account |
| Primary sources | `#sources` | Collected documents, in-site PDF reader, source status |
| Methodology | `#method` | Scoring, AI, mapping and social-signal method notes |

Press **⌘K / Ctrl+K** (or `/`) for the command palette: jump to views, reports, places or `@accounts`, or search reports and X posts. Charts carry hover tooltips; all filtered lists export as JSON. The frontend remains dependency-free vanilla JavaScript embedded in the binary; rebuild after edits.

## X social signals

`GET /v1/social` returns aggregate topic snapshots, a rolling hourly count history (up to 7 days shown), query definitions, per-budget daily usage and archive size. The worker requests `https://api.x.com/2/tweets/counts/recent` for the bundled queries in `config/x-topics.json`. Each snapshot covers the previous 24 complete UTC hours with hourly bins and includes reposts. Topics overlap: do not sum their counts as distinct posts. Every snapshot's hourly bins are also stored (`x_counts`), so the volume history grows over time. Keywords are not exhaustive; regional-language vocabulary needs review.

Configure only on the server in the Git-ignored `.env` (permissions 0600):

```dotenv
X_BEARER_TOKEN=your-app-only-token
X_ENABLED=1
X_DAILY_REQUEST_LIMIT=24      # counts requests per UTC day (0–96)
X_POST_REQUEST_LIMIT=4        # topic post samples per UTC day (0–24)
X_POSTS_PER_TOPIC=10          # posts per topic sample (10–100)
X_ACCOUNT_REQUEST_LIMIT=10    # account-history requests per UTC day (0–100)
X_ACCOUNT_HISTORY_PAGES=1     # timeline pages (100 posts each) per dossier fetch (1–5)
X_RETENTION_DAYS=90           # archive retention for posts and counts (1–3650)
```

Collection is disabled unless explicitly enabled and a token is present. Every request is reserved against its daily cap in SQLite *before* it is sent, so failures and restarts cannot reset a budget. X rate-limit resets on 429 defer scheduled collection. No redirects are followed with the bearer credential. Refreshing the dashboard only reads SQLite and never triggers a paid request. X bills counts, post reads and user reads; check X billing for current prices.

A volume flag requires ≥20 posts in the latest complete hour and ≥3× the preceding six-hour mean (minimum baseline 1). It is an exploratory indicator, not event confirmation.

### Post archive and topic rules

Topic samples (`/2/tweets/search/recent`, once per topic per day) and analyst-requested account histories are stored in a retained archive (`x_posts`, `x_accounts`, `x_post_matches`). A post is archived **only if it matches at least one topic rule**. A local evaluator applies the same query syntax used in `config/x-topics.json` (terms, quoted phrases, `OR`, implicit AND, parentheses, `-` negation) and records which terms matched. It approximates X's server-side matching; posts returned by an X topic query are always tagged with that topic. Posts keep text, language, type (post/reply/quote/repost), public engagement metrics, hashtags, mentions and linked domains. Accounts keep their public profile (name, bio, location, creation date, public metrics, verification) and previously seen handles. Posts older than `X_RETENTION_DAYS` are purged, along with accounts that have no remaining posts. The earlier 24-hour snapshot format is migrated on startup by re-arming the daily sample.

### Account dossiers

Open any author, @mention or handle (Social intelligence → **Account dossier**, the command palette, or `/#account/<handle>`). A dossier shows the account's archived topic posts with matched terms highlighted, topic mix, post types, languages, posting hours (Kinshasa time), daily activity, hashtags, mentions and linked domains. Dossiers export as JSON.

**Fetch history** calls `/2/users/{id}/tweets` (preceded by `/2/users/by/username/{handle}` for accounts not yet in the archive), reads up to 100 recent posts per page and archives only those matching the topic rules. Later fetches request only newer posts (`since_id`). Each account can be fetched at most once per 10 minutes. This is on demand only; nothing fetches account histories automatically.

Accounts are attributed, never labeled: no stance, affiliation, automation, coordination or credibility scores are inferred, and activity volume is not influence. Posts are not sent to AI services.

| Method | Endpoint | Purpose |
| --- | --- | --- |
| GET | `/v1/social` | Topic counts, hourly history, budgets, archive size |
| GET | `/v1/social/posts?topic=&lang=&q=&since=&limit=&offset=` | Archived topic posts, newest first (limit ≤ 500) |
| GET | `/v1/social/accounts?q=&topic=&limit=` | Accounts ranked by archived topic posts, with per-topic counts |
| GET | `/v1/social/accounts/{handle}` | Dossier: profile, history status, up to 1,000 archived posts |
| POST | `/v1/social/accounts/{handle}/history` | Budgeted fetch of the account's recent posts filtered to topic rules |

`X_API_URL` overrides the X origin for offline tests only; the bearer token is sent to it. `python3 tests/social_smoke.py` exercises the account-history flow against a mock X server.

Official API references: https://docs.x.com/x-api/posts/counts/introduction, https://docs.x.com/x-api/users/user-posts-timeline-by-user-id and https://docs.x.com/x-api/getting-started/pricing . Use remains subject to your approved X use case and developer terms.

## Key figures & sanctions (`/#actors`)

Profiles merge the DRC sanctions regimes of four official lists, refreshed daily, with curated public figures in `config/key-figures.json`:

| List | Regime | Source |
| --- | --- | --- |
| UN Security Council 1533 | `UN_LIST_TYPE=DRC` | `scsanctions.un.org` consolidated XML |
| US Treasury OFAC SDN | program `DRCONGO` | `treasury.gov/ofac/downloads/sdn.csv` |
| EU financial sanctions | programme `COD` | EU Financial Sanctions Files XML |
| UK Sanctions List | regime "Democratic Republic of the Congo" | OFSI consolidated CSV |

Records merge across lists by name tokens and compatible birth years; curated names merge when contained in a listed name. Passport, ID, tax, registration and contact details are dropped at parse time. An empty or changed-format download keeps the previous records. Profiles are enriched weekly from Wikidata (resolved by English Wikipedia title; non-curated people must be human and tied to the region) with roles, positions, citizenship, X username and a Commons photo with its license and author. Mentions are scanned every 30 minutes across news, archived X posts and each primary-document page; acronyms (M23, ADF) only match in capitals, and person terms are full names plus curated distinctive surnames. Report cards, X posts and the report drawer show chips for named actors; sanctioned actors are flagged.

## Arms & equipment (`/#arms`)

`config/equipment.json` lists weapon types and their EN/FR terms. For each type the worker stores Wikidata facts (description, origin, manufacturer, service entry, operators, image) and the first non-age-restricted Sketchfab search result as a representative 3D model; the model loads only when the analyst clicks it. Mentions record the source, date, PDF page and snippet, plus actor groups named in the same sentence (a co-mention, not attribution). The co-mention heatmap and per-weapon drawers expose those leads.

SIPRI's database host was unreachable from this machine, so transfers are imported from analyst exports: open armstransfers.sipri.org → Trade Register, choose recipients (DR Congo, Rwanda, Uganda, Burundi), export CSV and save it into `SIPRI_DIR` (default `data/sipri/`). Files are re-imported within a minute of changing; columns are located by header name.

## Gazetteer

`config/gazetteer.json` (about 6,000 places) is generated by `python3 -I scripts/build_gazetteer.py --db data/osint.db` from GeoNames, preferring administrative seats, territories and Kinshasa communes for duplicate names and the conflict provinces for unpopulated villages, with Wikidata coordinates for missing localities (Rubaya, Kanyabayonga). Province, country and common-word names are excluded; with `--db`, names used mostly in lower case in collected articles are dropped. Matching is capitalized, longest-name-first; single-word names next to another capitalized word are treated as personal names, and unpopulated villages need a locative cue.

| Variable | Default | Purpose |
| --- | --- | --- |
| `REFERENCE_SYNC` | enabled | `0` disables network refresh of sanctions, Wikidata and Sketchfab (tests/offline) |
| `REFERENCE_CONTACT` | unset | Contact (email or URL) appended to the User-Agent, as Wikimedia's policy requests |
| `SIPRI_DIR` | `data/sipri` | Folder of SIPRI trade-register CSV exports |
| `FEED_POLL_SECONDS` | `1800` | Publisher feed polling interval; `0` disables |

New endpoints (same bearer-token policy): `GET /v1/actors`, `GET /v1/actors/{id}`, `POST /v1/actors/refresh` (re-queues lists and reference data), `GET /v1/arms`, `GET /v1/arms/{id}`.

### Data licenses

| Source | License / terms | How it is used |
| --- | --- | --- |
| UN Security Council sanctions list | UN public information; attribute the Security Council | Reproduced with links to the committee |
| US OFAC SDN list | US Government work, public domain | Reproduced with OFAC record links |
| EU Financial Sanctions Files | Commission reuse policy (Decision 2011/833/EU), attribution | Reproduced with the regulation link |
| UK Sanctions List | Open Government Licence v3.0, attribution | Reproduced with attribution |
| Wikidata | CC0 | Descriptions, roles, facts |
| Wikimedia Commons images | Per file (CC BY, CC BY-SA, public domain…) | Hotlinked thumbnails with author and license shown |
| GeoNames | CC BY 4.0, attribution required | Gazetteer (credited in the methodology page) |
| Sketchfab models | Sketchfab terms; per-model licenses (some non-commercial) | Embedded viewer only, author and license shown; never downloaded |
| SIPRI Arms Transfers Database | SIPRI terms of use; cite SIPRI | Imported only from your own exports |
| ACLED | Requires registration; redistribution limits | Not integrated |

Public deployment that redistributes these data should keep the attributions shown in the interface. Sketchfab models marked non-commercial must not be used in commercial products.

## Publisher feeds: Jeune Afrique

`config/news-feeds.json` lists publisher syndication feeds collected as news, alongside the Actualite.cd crawler. Jeune Afrique's site answers automated clients (including `robots.txt` and its own `/feed/`) with a Cloudflare bot challenge, and many articles are paywalled; the collector does not attempt to bypass either. It reads the publisher's official FeedBurner feed (`feeds.feedburner.com/jeuneafrique`) instead, which carries the latest ~30 items with title, lede, author, section and publication time.

- Polled every `FEED_POLL_SECONDS` (default 1800; 300–86400; `0` disables) with conditional GET (ETag / If-Modified-Since). `POST /v1/feeds/refresh` (also triggered by **Collect latest**) polls immediately; `GET /v1/feeds` reports status and counts.
- Items are kept only when they explicitly concern the DRC (terms such as RDC, Kinshasa, Kivu, M23, FARDC, Tshisekedi, or a recognized DRC place). Great Lakes items (Rwanda, Uganda, Burundi) are stored as regional context and appear only with **Include regional context**; Congo-Brazzaville and other countries are skipped. Links outside the publisher's domain are rejected.
- Articles are marked `coverage: "feed_summary"`; the dashboard labels them, AI briefs note that they rest on the headline and lede, and the report links to the full article. The raw feed item is stored as revision evidence, so edited headlines create versions.
- Submitting a Jeune Afrique URL to `POST /v1/jobs` returns `200 already_collected` with the article ID, or `422 feed_only` explaining that pages are not fetched. The feed carries no archive, so articles published before polling started are not available.

Add another publisher by appending a feed (id, name, url, site, language, coverage, access_note); no code change is needed.

## Web-search images (OpenAI)

Profiles without a Wikidata/Commons photo, and equipment with neither an image nor a 3D model, get an image found by OpenAI web search (Responses API, `web_search` tool). Images that already exist are kept. Nothing is generated: the model must return an existing image URL and the page it appears on, and the server then checks independently that

1. the URL is https and serves a raster image (JPEG, PNG, WebP or GIF; SVG refused) between 4 KB and 15 MB, and
2. the source page names the subject (a full name or, for people, at least two name tokens including the surname).

Images passing both checks are `verified`; if the source page cannot be fetched they are `unverified` and labelled as such; otherwise they are `rejected_auto` and never shown. Only links are stored (`web_images` table). The dashboard credits the source domain and verification state and offers **Reject**, which hides the image permanently (`POST /v1/images/{actor|equipment}/{id}/reject`). `GET /v1/images` reports budget and counts. Key figures and the most-mentioned profiles are searched first; "not found" results are retried after 30 days, errors after a day.

| Variable | Default | Purpose |
| --- | --- | --- |
| `OPENAI_API_KEY` (or `OPENAI_KEY`) | unset | Enables image search (server-side only) |
| `OPENAI_IMAGE_MODEL` | `gpt-5-mini` | Model used with the web search tool |
| `IMAGE_SEARCH_DAILY_LIMIT` | `40` | Searches per UTC day (0–500), reserved before each call |
| `IMAGE_SEARCH_ENABLED` | enabled | `0` disables; also off when `REFERENCE_SYNC=0` |

Rights: web images are third-party works whose reuse rights are not cleared. They are hotlinked for internal research display with their source; do not republish them. Subject names and short context (role, affiliation, nationality, birth year from the sanctions lists) are sent to OpenAI.

## Live radio (`/#radio`)

**Situation room → Radio** maps DRC radio stations and plays their live streams in the browser. A station can be opened directly with `#radio/<station-id>`. Playback continues across views, and an **ON AIR** chip in the top bar controls it. Selecting a station shows its stream details, reports that mention places within 60 km, and other stations nearby.

- **Sources.** Curated official streams come from `config/radio-stations.json`; currently Radio Okapi and Radio Maendeleo, taken from the players on their own websites. Other stations come from the community [Radio Browser](https://www.radio-browser.info/) directory (country code `CD`), synced daily. A curated entry takes precedence over a directory duplicate.
- **Health.** Every 30 minutes each stream is probed: the first 8 KB is read, `.pls`/`.m3u` playlists are resolved, and an audio content type is required. The result (codec, bitrate, latency, error) is shown, and the list defaults to live streams.
- **Location.** A station is placed by its directory coordinates when those fall inside the DRC; otherwise by the gazetteer, using its listed city, name or tags. Stations that can't be placed are listed as *National & online*. A location is the city where the station is listed, not its transmitter coverage.
- **Proxying.** Audio is not proxied; the browser connects to the broadcaster directly. A few streams are plain `http:`, and browsers block those on an HTTPS deployment.
- **API.** `GET /v1/radio` lists stations. `POST /v1/radio/{id}/listen` counts a play locally and, for directory stations, registers a click with Radio Browser. `REFERENCE_SYNC=0` disables the directory sync and the health checks.

To add a station, append an entry to `config/radio-stations.json` with these fields: `id`, `name`, `city`, `homepage`, `stream`, `languages`, `tags`, `coverage` and `source_note`. Use the stream URL from the station's own player.
