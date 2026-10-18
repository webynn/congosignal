"""Offline API integration test with deterministic mock Ollama; never touches production data."""
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import sqlite3
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ROOT = Path(__file__).resolve().parent.parent
PROVIDER = os.environ.get("TEST_PROVIDER", "ollama")
calls = []
incomplete_response = False
provider_failure = False
broken_translation = False
class MockOllama(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def do_GET(self):
        self.send_response(200); self.end_headers()
        self.wfile.write(b'{"models":[{"name":"test-model"}]}')
    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        calls.append(body)
        if PROVIDER == 'claude':
            assert self.path == '/v1/messages'
            assert self.headers['x-api-key'] == 'test-only-key'
            assert self.headers['anthropic-version'] == '2023-06-01'
            assert body['tool_choice']['name'] == 'research_result'
            assert body['tools'][0]['input_schema']['type'] == 'object'
        if provider_failure:
            self.send_response(401); self.end_headers(); self.wfile.write(b'{"error":"test-only-key must never appear in client errors"}'); return
        content = json.loads(body['messages'][0 if PROVIDER == 'claude' else 1]['content'])
        if 'paragraphs' in content:
            data = {'paragraphs': [] if broken_translation else ['English translation '+p for p in content['paragraphs']]}
        else:
            data = dict(title_en='DRC public health report', summary_en='The source reports a public health concern.', summary_fr='La source signale un problème de santé publique.', impact='high', rationale_en='Reported public health impact.', rationale_fr='Impact sanitaire rapporté.', actors=['MSF'])
        self.send_response(200); self.send_header('Content-Type','application/json'); self.end_headers()
        response = {'stop_reason':'max_tokens' if incomplete_response else 'tool_use','content':[{'type':'tool_use','name':'research_result','input':data}]} if PROVIDER == 'claude' else {'done':True,'done_reason':'length' if incomplete_response else 'stop','message':{'content':json.dumps(data)}}
        self.wfile.write(json.dumps(response).encode())
mock = ThreadingHTTPServer(('127.0.0.1',0),MockOllama)
threading.Thread(target=mock.serve_forever,daemon=True).start()
with socket.socket() as s:
    s.bind(('127.0.0.1',0)); port=s.getsockname()[1]
base=f'http://127.0.0.1:{port}'
def api(path,data=None,auth=True):
    req=urllib.request.Request(base+path,data=json.dumps(data).encode() if data is not None else None,headers={'Content-Type':'application/json',**({'Authorization':'Bearer integration-test-secret-long'} if auth else {})})
    try:
        with urllib.request.urlopen(req,timeout=3) as r:return r.status,json.load(r)
    except urllib.error.HTTPError as e:return e.code,json.load(e)
def wait_task(task):
    for _ in range(50):
        detail=api('/v1/intelligence/test-article')[1]
        result=detail[task]
        if result['status'] in ['completed','failed']:return result
        time.sleep(.15)
    raise AssertionError('AI task did not finish')
with tempfile.TemporaryDirectory(prefix='congo-signal-test-') as directory:
    db=Path(directory)/'test.db'
    env=dict(os.environ,EVIDENCE_ARCHIVE_DIR=str(Path(directory)/'evidence'),EVIDENCE_POLL_SECONDS='0',LOAD_DOTENV='0',AI_PROVIDER=PROVIDER,ANTHROPIC_API_KEY='test-only-key',CLAUDE_MODEL='test-model',CLAUDE_API_URL=f'http://127.0.0.1:{mock.server_port}/v1/messages',MAPBOX_PUBLIC_TOKEN='sk.must-never-expose',DATABASE_URL=f'sqlite://{db}',BIND_ADDR=f'127.0.0.1:{port}',AI_ENABLED='1',OLLAMA_URL=f'http://127.0.0.1:{mock.server_port}',OLLAMA_MODEL='test-model',API_TOKEN='integration-test-secret-long')
    env.pop('POLL_SECONDS',None)
    env['X_ENABLED']='0'
    env.pop('X_BEARER_TOKEN',None)
    def start():
        p=subprocess.Popen([str(ROOT/'target/debug/drc-osint-worker')],env=env,cwd=ROOT,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        for _ in range(50):
            try:
                if api('/health',auth=False)[0]==200:return p
            except OSError:time.sleep(.1)
        raise AssertionError('Startup failed')
    def stop(p):p.send_signal(signal.SIGINT);p.wait(timeout=10)
    p=start()
    try:
        article=dict(id='test-article',source='Actualite.cd',url='https://actualite.cd/2026/10/12/test',fetched_url='https://actualite.cd/2026/10/12/test',title='Santé à Goma',description='Une source de recherche.',category='Santé',author=None,author_method=None,language='fr',published_at='2026-10-12T11:00:00+00:00',modified_at=None,fetched_at='2026-10-12T12:00:00+00:00',content='Une épidémie à Goma.\n\nSelon MSF.',paragraphs=['Une épidémie à Goma.','Selon MSF.'],content_sha256=hashlib.sha256(b'test').hexdigest(),html_sha256=hashlib.sha256(b'html').hexdigest(),extractor_version='fixture')
        with sqlite3.connect(db) as c:c.execute('INSERT INTO articles VALUES(?,?,?,?,?,?,?,?,?)',(article['id'],article['url'],article['title'],article['content'],article['category'],article['published_at'],article['fetched_at'],article['fetched_at'],json.dumps(article)))
        # Curated evidence endpoints share authentication and preserve the last good snapshot.
        assert api('/v1/social/posts',auth=False)[0]==401
        assert api('/v1/social/posts')[1]['items']==[]
        assert api('/v1/social',auth=False)[0]==401
        social=api('/v1/social')[1]
        assert social['configured'] is False and social['enabled'] is False
        assert len(social['items'])==4 and all(i['data'] is None for i in social['items'])
        assert social['requests_today']==0
        assert api('/v1/sources',auth=False)[0]==401
        assert api('/v1/evidence',auth=False)[0]==401
        assert len(api('/v1/sources')[1]['items'])==14
        assert api('/v1/sources/missing/refresh',{})[0]==404
        assert api('/v1/sources/hrw-kanombe-thread/refresh',{})[0]==400
        assert api('/v1/evidence/missing')[0]==404
        assert api('/v1/evidence-feed',auth=False)[0]==401
        assert api('/v1/evidence/missing/analyze',{})[0]==404
        source={'title':'Test report','publisher':'Test publisher','url':'https://example.org/report','reference_date':'2024-05-31'}
        with sqlite3.connect(db) as c:
            for identity, text in [('old','Old content'),('new','Historical Kibumba finding')]:
                evidence={'id':identity,'source':source,'pages':[{'page':15,'text':text},{'page':20,'text':'Other page'}]}
                c.execute('INSERT INTO evidence_snapshots VALUES(?,?,?,?,?,?,?)',(identity,'s-2024-432',identity,'2026-10-18T00:00:00Z',source['url'],'application/pdf',json.dumps(evidence)))
            c.execute("UPDATE evidence_sources SET status='failed',error='Temporary upstream error' WHERE id='s-2024-432'")
        result=api('/v1/evidence?source_id=s-2024-432&q=kibumba&page=15')[1]
        assert result['total']==1 and result['items'][0]['document_id']=='new'
        assert result['items'][0]['claim_status']=='source_attributed_not_independently_verified'
        assert api('/v1/evidence?q=Old')[1]['total']==0
        assert api('/v1/evidence/old')[1]['pages'][0]['text']=='Old content'
        assert len(api('/v1/evidence?limit=1&offset=1')[1]['items'])==1
        assert api('/v1/intelligence',auth=False)[0]==401
        assert api('/v1/intelligence/missing')[0]==404
        assert api('/v1/intelligence/analyze',{'article_id':'test-article','task':'bad'})[0]==400
        assert api('/v1/intelligence/status')[1]['available'] is True
        item=api('/v1/intelligence')[1]['items'][0]
        assert item['verification']=='single_source_unverified' and item['places'][0]['name']=='Goma'
        assert api('/v1/intelligence/analyze',{'article_id':'test-article','task':'brief'})[0]==202
        assert wait_task('brief')['status']=='completed'
        assert api('/v1/intelligence/analyze',{'article_id':'test-article','task':'brief'})[1]['status']=='completed'
        assert len(calls)==1,'completed task should be cached'
        assert 'UNTRUSTED SOURCE DATA' in (calls[0]['system'] if PROVIDER=='claude' else calls[0]['messages'][0]['content'])
        assert api('/v1/intelligence/test-article')[1]['brief']['provider']==PROVIDER
        assert api('/v1/config',auth=False)[0]==401
        assert api('/v1/config')[1]=={'mapbox_public_token':None}
        assert 'test-only-key' not in json.dumps(api('/v1/intelligence/status')[1])
        api('/v1/intelligence/analyze',{'article_id':'test-article','task':'translation'})
        translation=wait_task('translation')
        assert translation['status']=='completed' and len(translation['data']['paragraphs'])==2
        n={'saved':True,'review':'follow_up','note':'<script>test</script> is stored as plain text'}
        assert api('/v1/intelligence/test-article/notes',n)[1]==n
        assert api('/v1/intelligence/test-article/notes',{**n,'review':'verified'})[0]==400
        stop(p);p=start()
        assert api('/v1/intelligence/test-article')[1]['analyst']==n
        article['title']='Updated article'
        with sqlite3.connect(db) as c:c.execute('UPDATE articles SET data=?,title=? WHERE id=?',(json.dumps(article),article['title'],article['id']))
        assert api('/v1/intelligence/test-article')[1]['brief']['status']=='not_requested'
        broken_translation=True
        api('/v1/intelligence/analyze',{'article_id':'test-article','task':'translation'})
        result=wait_task('translation')
        assert result['status']=='failed' and result['data'] is None
        assert 'alignment' in result['error']
        broken_translation=False;incomplete_response=True
        api('/v1/intelligence/analyze',{'article_id':'test-article','task':'translation'})
        result=wait_task('translation');assert result['status']=='failed' and 'incomplete' in result['error']
        incomplete_response=False;provider_failure=True
        api('/v1/intelligence/analyze',{'article_id':'test-article','task':'translation'})
        result=wait_task('translation');assert result['status']=='failed' and '401' in result['error'] and 'test-only-key' not in json.dumps(result)
        provider_failure=False
        document={'id':'brief-fixture','source':{'title':'DRC test report','publisher':'Fixture publisher','url':'https://example.org/report','language':'en','reference_date':'2024-05-31','kind':'ngo_report'},'fetched_url':'https://example.org/report','fetched_at':'2026-10-18T00:00:00Z','sha256':'fixture-hash','pages':[{'page':1,'text':'Public health in Goma in May 2024.'}]}
        with sqlite3.connect(db) as c:
            c.execute('INSERT INTO evidence_snapshots VALUES(?,?,?,?,?,?,?)',('brief-fixture','fixture-source','fixture-hash',document['fetched_at'],document['fetched_url'],'text/html',json.dumps(document)))
        assert api('/v1/evidence/brief-fixture/analyze',{})[0]==202
        for _ in range(60):
            feed=api('/v1/evidence-feed')[1]
            report=next(x for x in feed['items'] if x['id']=='brief-fixture')
            if report['brief']['status'] in ['completed','failed']:break
            time.sleep(.1)
        assert report['brief']['status']=='completed',report
        assert report['brief']['data']['source_sha256']=='fixture-hash'
        assert report['brief']['data']['summary_fr'] and report['brief']['data']['summary_en']
        combined=api('/v1/intelligence')[1]
        combined_report=next(x for x in combined['items'] if x['id']=='brief-fixture')
        assert combined_report['record_type']=='evidence'
        assert combined_report['source']=='Fixture publisher'
        assert combined_report['published_at']=='2024-05-31T00:00:00Z'
        assert combined_report['places'][0]['name']=='Goma'
        assert combined['source_count']==2
        assert combined['evidence_count']==1
        regional=json.loads(json.dumps(document));regional['id']='regional-fixture';regional['source']['publisher']='Regional publisher';regional['source']['title']='Rwanda regional report';regional['pages']=[{'page':1,'text':'Rwanda training in CAR.'}]
        with sqlite3.connect(db) as c:c.execute('INSERT INTO evidence_snapshots VALUES(?,?,?,?,?,?,?)',('regional-fixture','regional-source','regional-hash',regional['fetched_at'],regional['fetched_url'],'text/html',json.dumps(regional)))
        assert not any(x['id']=='regional-fixture' for x in api('/v1/intelligence')[1]['items'])
        regional_item=next(x for x in api('/v1/intelligence?scope=regional')[1]['items'] if x['id']=='regional-fixture')
        assert regional_item['relevance']['scope']=='regional'


        before=len(calls);api('/v1/evidence/brief-fixture/analyze',{});time.sleep(1.2)
        assert len(calls)==before,'Evidence brief should be cached by snapshot'
        pdf_id='a'*64
        pdf_bytes=b'%PDF-1.4\n% Synthetic endpoint fixture\n%%EOF'
        archive=Path(directory)/'evidence';archive.mkdir(exist_ok=True)
        (archive/(pdf_id+'.pdf')).write_bytes(pdf_bytes)
        with sqlite3.connect(db) as c:
            c.execute('INSERT INTO evidence_snapshots VALUES(?,?,?,?,?,?,?)',(pdf_id,'pdf-fixture',hashlib.sha256(pdf_bytes).hexdigest(),'2026-10-18','https://example.org/test.pdf','application/pdf',json.dumps({'source':{'format':'pdf'}})))
        assert api('/v1/evidence/'+pdf_id+'/pdf',auth=False)[0]==401
        request=urllib.request.Request(base+'/v1/evidence/'+pdf_id+'/pdf',headers={'Authorization':'Bearer integration-test-secret-long'})
        with urllib.request.urlopen(request) as response:
            assert response.headers['Content-Type']=='application/pdf'
            assert response.headers['Content-Disposition'].startswith('inline')
            assert response.read()==pdf_bytes
        (archive/(pdf_id+'.pdf')).write_bytes(b'%PDF-1.4 tampered')
        assert api('/v1/evidence/'+pdf_id+'/pdf')[0]==409
        (archive/(pdf_id+'.pdf')).unlink()
        assert api('/v1/evidence/'+pdf_id+'/pdf')[0]==404
        assert api('/v1/evidence/brief-fixture/pdf')[0]==404
        print(PROVIDER, 'PASS: authenticated API, ranked feed, AI schema, cache deduplication, bilingual brief, paragraph translation, notes persistence, source invalidation, partial-output rejection.')
    finally:
        if p.poll() is None:stop(p)
mock.shutdown()
