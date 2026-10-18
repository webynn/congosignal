"""Opt-in live API smoke test; fetches the supplied article twice, respecting worker pacing.
Run after cargo build: python3 tests/smoke.py
Uses a fresh SQLite file under data/ and stops its temporary API when finished.
"""
import hashlib
import json
import os
from pathlib import Path
import secrets
import signal
import socket
import sqlite3
import subprocess
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
URL = "https://actualite.cd/2026/10/12/musique-gally-garvey-devoile-adn-un-album-de-22-titres-porte-notamment-par-des"
TOKEN = secrets.token_urlsafe(32)
with socket.socket() as s:
    s.bind(("127.0.0.1", 0))
    PORT = s.getsockname()[1]
BASE = f"http://127.0.0.1:{PORT}"
DB = ROOT / "data" / f"smoke-{int(time.time())}.db"
DB.parent.mkdir(exist_ok=True)

def request(path, data=None, auth=True):
    headers = {"Content-Type": "application/json"}
    if auth:
        headers["Authorization"] = f"Bearer {TOKEN}"
    req = urllib.request.Request(BASE + path, data=None if data is None else json.dumps(data).encode(), headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=5) as r:
            return r.status, json.load(r)
    except urllib.error.HTTPError as e:
        return e.code, json.load(e)

def start():
    env = {**os.environ, "LOAD_DOTENV": "0", "REFERENCE_SYNC": "0", "FEED_POLL_SECONDS": "0", "BIND_ADDR": f"127.0.0.1:{PORT}", "API_TOKEN": TOKEN, "DATABASE_URL": f"sqlite://{DB}"}
    env.pop("POLL_SECONDS", None)
    p = subprocess.Popen([str(ROOT / "target/debug/drc-osint-worker")], cwd=ROOT, env=env)
    for _ in range(100):
        try:
            if request("/health", auth=False)[0] == 200:
                return p
        except OSError:
            pass
        if p.poll() is not None:
            raise RuntimeError("worker exited at startup")
        time.sleep(0.1)
    p.terminate()
    raise RuntimeError("worker startup timeout")

def stop(p):
    p.send_signal(signal.SIGINT)
    p.wait(timeout=10)

def collect():
    code, queued = request("/v1/jobs", {"url": URL})
    assert code == 202, queued
    for _ in range(90):
        code, job = request(queued["status_url"])
        assert code == 200, job
        if job["status"] == "failed":
            raise AssertionError(job)
        if job["status"] == "completed":
            return job["result"]["article_id"]
        time.sleep(1)
    raise AssertionError("collection timeout")

p = start()
try:
    assert request("/v1/articles", auth=False)[0] == 401
    assert request("/v1/jobs", {"url": "https://example.com/"})[0] == 400
    assert request("/v1/articles?since=invalid")[0] == 400
    assert request("/v1/jobs/missing")[0] == 404
    article_id = collect()
    code, article = request(f"/v1/articles/{article_id}")
    assert code == 200
    assert article["category"] == "Culture", article
    assert article["author"] == "Japhet Toko", article
    assert len(article["paragraphs"]) > 10
    assert article["published_at"] == "2026-10-12T11:34:11+00:00"
    assert hashlib.sha256(article["content"].encode()).hexdigest() == article["content_sha256"]
    assert collect() == article_id
    versions = request(f"/v1/articles/{article_id}/versions")[1]["items"]
    assert len(versions) == 1, "unchanged article should not create another version"
    assert request("/v1/articles?q=Kinshasa&category=Culture")[1]["items"][0]["id"] == article_id
    assert request("/v1/articles?category=Missing")[1]["items"] == []
    with sqlite3.connect(DB) as db:
        html, data = db.execute("SELECT html,data FROM versions WHERE article_id=?", (article_id,)).fetchone()
        assert hashlib.sha256(html.encode()).hexdigest() == json.loads(data)["html_sha256"]
    stop(p)
    p = start()
    assert request(f"/v1/articles/{article_id}")[0] == 200
    summary = {"result": "passed", "article_id": article_id, "category": article["category"], "author": article["author"], "paragraphs": len(article["paragraphs"]), "database": str(DB), "checks": ["authentication", "URL validation", "date validation", "job processing", "live extraction", "search", "deduplication", "evidence hash", "restart persistence"]}
    (ROOT / "data/live-smoke-result.json").write_text(json.dumps(summary, indent=2))
    print(json.dumps(summary, indent=2))
finally:
    if p.poll() is None:
        stop(p)
