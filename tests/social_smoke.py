"""Offline X account-history test against a mock X API; never touches production data or the real X API."""
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.parse
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = Path(__file__).resolve().parent.parent
TOKEN = 'x-test-token'
calls = []
USER = {'id': '4242', 'name': 'Observatoire Kivu', 'username': 'obs_kivu', 'created_at': '2019-03-01T00:00:00.000Z',
        'description': 'Veille citoyenne', 'location': 'Goma', 'public_metrics': {'followers_count': 1200, 'following_count': 300, 'tweet_count': 9000, 'listed_count': 4},
        'verified': False, 'protected': False}
now = time.strftime('%Y-%m-%dT%H:%M:%S.000Z', time.gmtime(time.time() - 3600))
TIMELINE = {
    'data': [
        {'id': '9003', 'author_id': '4242', 'text': 'Affrontements signalés à Masisi ce matin #RDC', 'created_at': now, 'lang': 'fr', 'public_metrics': {'like_count': 5, 'retweet_count': 2, 'reply_count': 1, 'quote_count': 0}, 'entities': {'hashtags': [{'tag': 'RDC'}]}},
        {'id': '9002', 'author_id': '4242', 'text': 'Match de football ce soir', 'created_at': now, 'lang': 'fr'},
        {'id': '9001', 'author_id': '4242', 'text': 'Appel au cessez-le-feu à Kinshasa @MONUSCO', 'created_at': now, 'lang': 'fr', 'referenced_tweets': [{'type': 'quoted', 'id': '1'}], 'entities': {'mentions': [{'username': 'MONUSCO'}]}},
    ],
    'includes': {'users': [USER]},
    'meta': {'newest_id': '9003', 'oldest_id': '9001', 'result_count': 3},
}

class MockX(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def do_GET(self):
        url = urllib.parse.urlparse(self.path)
        query = urllib.parse.parse_qs(url.query)
        calls.append((url.path, query))
        assert self.headers['Authorization'] == f'Bearer {TOKEN}'
        if url.path == '/2/users/by/username/obs_kivu':
            body = {'data': USER}
        elif url.path == '/2/users/by/username/nobody_here':
            body = {'errors': [{'detail': 'not found'}]}
        elif url.path == '/2/users/4242/tweets':
            assert query['max_results'] == ['100']
            body = {'meta': {'result_count': 0}} if 'since_id' in query else TIMELINE
        else:
            # Scheduled counts/sample collection is not under test here.
            self.send_response(503); self.end_headers(); return
        self.send_response(200); self.send_header('Content-Type', 'application/json'); self.end_headers()
        self.wfile.write(json.dumps(body).encode())

mock = ThreadingHTTPServer(('127.0.0.1', 0), MockX)
threading.Thread(target=mock.serve_forever, daemon=True).start()
with socket.socket() as s:
    s.bind(('127.0.0.1', 0)); port = s.getsockname()[1]
base = f'http://127.0.0.1:{port}'
def api(path, data=None, auth=True):
    req = urllib.request.Request(base + path, data=json.dumps(data).encode() if data is not None else None, headers={'Content-Type': 'application/json', **({'Authorization': 'Bearer integration-test-secret-long'} if auth else {})})
    try:
        with urllib.request.urlopen(req, timeout=10) as r: return r.status, json.load(r)
    except urllib.error.HTTPError as e: return e.code, json.load(e)

with tempfile.TemporaryDirectory(prefix='congo-signal-social-') as directory:
    env = dict(os.environ, LOAD_DOTENV='0', DATABASE_URL=f'sqlite://{Path(directory)/"test.db"}', BIND_ADDR=f'127.0.0.1:{port}', API_TOKEN='integration-test-secret-long',
               EVIDENCE_ARCHIVE_DIR=str(Path(directory)/'evidence'), EVIDENCE_POLL_SECONDS='0', AI_ENABLED='0',
               X_ENABLED='1', X_BEARER_TOKEN=TOKEN, X_API_URL=f'http://127.0.0.1:{mock.server_port}', X_ACCOUNT_REQUEST_LIMIT='3', X_DAILY_REQUEST_LIMIT='0', X_POST_REQUEST_LIMIT='0')
    env.pop('POLL_SECONDS', None)
    p = subprocess.Popen([str(ROOT/'target/debug/drc-osint-worker')], env=env, cwd=ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try:
                if api('/health', auth=False)[0] == 200: break
            except OSError: time.sleep(.1)
        assert api('/v1/social/accounts/obs_kivu/history', {}, auth=False)[0] == 401
        assert api('/v1/social/accounts/bad%2Fhandle/history', {})[0] == 400
        assert api('/v1/social/accounts/obs_kivu')[0] == 404
        status, result = api('/v1/social/accounts/obs_kivu/history', {})
        assert status == 200, result
        assert result['scanned'] == 3 and result['matched'] == 2 and result['new'] == 2 and result['requests'] == 2, result
        status, dossier = api('/v1/social/accounts/OBS_KIVU')
        assert status == 200
        assert dossier['account']['profile']['public_metrics']['followers_count'] == 1200
        assert [x['id'] for x in dossier['posts']] == ['9003', '9001']
        topics = {t['topic'] for x in dossier['posts'] for t in x['topics']}
        assert topics == {'eastern-drc', 'diplomacy'}, topics
        quote = next(x for x in dossier['posts'] if x['id'] == '9001')
        assert quote['kind'] == 'quote' and quote['mentions'] == ['MONUSCO']
        assert 'stance' not in quote and 'label' not in dossier['account']
        assert api('/v1/social/accounts/obs_kivu/history', {})[0] == 429  # cooldown
        posts = api('/v1/social/posts?topic=diplomacy')[1]
        assert [x['id'] for x in posts['items']] == ['9001']
        accounts = api('/v1/social/accounts')[1]
        assert accounts['items'][0]['handle'] == 'obs_kivu' and accounts['items'][0]['posts'] == 2
        social = api('/v1/social')[1]
        assert social['archive'] == {'posts': 2, 'accounts': 1, 'retention_days': 90}
        assert social['account_requests_today'] == 2 and social['account_request_limit'] == 3
        # One request left: the username lookup succeeds but the account is missing.
        status, missing = api('/v1/social/accounts/nobody_here/history', {})
        assert status == 404, (status, missing)
        assert api('/v1/social/accounts/someone_else/history', {})[0] == 429  # daily budget exhausted
        timeline_calls = [c for c in calls if c[0] == '/2/users/4242/tweets']
        assert len(timeline_calls) == 1
        print('social smoke test passed')
    finally:
        p.send_signal(signal.SIGINT); p.wait(timeout=10)
