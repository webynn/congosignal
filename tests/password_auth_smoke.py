"""Isolated password flow, permissions and recovery regression; no real keys or users."""
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import secrets
import signal
import socket
import sqlite3
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

ROOT=Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory() as tmp:
    with socket.socket() as sock:
        sock.bind(('127.0.0.1',0)); port=sock.getsockname()[1]
    base=f'http://127.0.0.1:{port}'
    env=dict(os.environ,LOAD_DOTENV='0',AUTH_MODE='password',AUTH_OWNER_EMAIL='owner@example.org',APP_ORIGIN=base,DATABASE_URL=f'sqlite://{tmp}/test.db',BIND_ADDR=f'127.0.0.1:{port}',AI_ENABLED='0',AI_AUTO_PREVIEWS='false',X_ENABLED='0',FLIGHTS_ENABLED='0',REFERENCE_SYNC='0',FEED_POLL_SECONDS='0',EVIDENCE_POLL_SECONDS='0',AGREEMENTS_POLL_SECONDS='0',AI_SETTINGS_PATH=f'{tmp}/ai.json',EVIDENCE_ARCHIVE_DIR=f'{tmp}/evidence')
    env.pop('POLL_SECONDS',None)
    log=open(f'{tmp}/server.log','w+')
    p=subprocess.Popen([str(ROOT/'target/debug/drc-osint-worker')],env=env,cwd=ROOT,stdout=log,stderr=log)
    def request(path,body=None,cookie='',csrf='',origin=base):
        headers={'Origin':origin,'Cookie':cookie,'X-CSRF-Token':csrf}
        if body is not None: headers['Content-Type']='application/json'
        req=urllib.request.Request(base+path,data=json.dumps(body).encode() if body is not None else None,headers=headers)
        try: r=urllib.request.urlopen(req,timeout=30)
        except urllib.error.HTTPError as e:r=e
        data=r.read(); content=json.loads(data) if r.headers.get('content-type','').startswith('application/json') else data.decode()
        return r.code,content,r.headers
    def login(email,password='correct horse battery staple'):
        status,data,h=request('/auth/password/login',{'email':email,'password':password});assert status==200,(status,data)
        cookie=h['Set-Cookie'].split(';')[0];assert 'HttpOnly' in h['Set-Cookie'] and 'SameSite=Lax' in h['Set-Cookie']
        status,me,_=request('/v1/auth/me',cookie=cookie);assert status==200
        return cookie,me['csrf'],me
    try:
        for _ in range(100):
            try:
                if request('/ready')[0]==200:break
            except (OSError,urllib.error.URLError):pass
            time.sleep(.1)
        else:raise AssertionError('server failed to start')
        assert request('/auth/config')[1]['mode']=='password'
        assert request('/v1/auth/me')[0]==401
        page=request('/auth/login');assert page[0]==200 and 'password' in page[1] and page[2]['Referrer-Policy']=='no-referrer'
        assert request('/auth/password/login',{'email':'x','password':'x'},origin='https://evil.example')[0]==403
        db=sqlite3.connect(f'{tmp}/test.db')
        uid='existing-oidc-user';db.execute('INSERT INTO auth_users VALUES(?,?,?)',(uid,'issuer|subject','owner@example.org'));db.execute('UPDATE memberships SET user_id=? WHERE email=?',(uid,'owner@example.org'))
        token=secrets.token_hex(32);db.execute('INSERT INTO password_links VALUES(?,?,?,?)',(hashlib.sha256(token.encode()).hexdigest(),'owner@example.org',int(time.time())+1800,'test'));db.commit()
        expired=secrets.token_hex(32)
        db.execute('INSERT INTO password_links VALUES(?,?,?,?)',(hashlib.sha256(expired.encode()).hexdigest(),'owner@example.org',int(time.time())-1,'test'));db.commit()
        assert request('/auth/password/setup',{'token':expired,'password':'correct horse battery staple'})[0]==400
        assert request('/auth/password/setup',{'token':token,'password':'short'})[0]==400
        assert request('/auth/password/setup',{'token':token,'password':'correct horse battery staple'})[0]==200
        assert request('/auth/password/setup',{'token':token,'password':'correct horse battery staple'})[0]==400
        assert db.execute('SELECT password_hash FROM password_credentials WHERE user_id=?',(uid,)).fetchone()[0].startswith('$argon2id$')
        assert request('/auth/password/login',{'email':'owner@example.org','password':'bad'})[0]==401
        owner,csrf,me=login('OWNER@example.org');assert me['user']['user_id']==uid
        def post(path,body,c=owner,t=csrf):return request(path,body,c,t)
        assert request('/v1/team/access-link',{'email':'owner@example.org'},cookie=owner)[0]==403
        def invite(email,role):
            assert post('/v1/team',dict(email=email,role=role,active=True,daily_requests=0,daily_usd=0,monthly_usd=0))[0]==200
            code,d,_=post('/v1/team/access-link',{'email':email});assert code==200
            return d['url'].split('#setup=')[1]
        t=invite('viewer@example.org','viewer')
        assert request('/auth/password/setup',{'token':t,'password':'correct horse battery staple'})[0]==200
        viewer,vc,_=login('viewer@example.org')
        assert post('/v1/team/access-link',{'email':'owner@example.org'},viewer,vc)[0]==403
        t=invite('admin@example.org','administrator')
        assert request('/auth/password/setup',{'token':t,'password':'correct horse battery staple'})[0]==200
        admin,ac,_=login('admin@example.org')
        assert post('/v1/team/access-link',{'email':'owner@example.org'},admin,ac)[0]==403
        code,d,_=post('/v1/team/access-link',{'email':'viewer@example.org'});assert code==200
        old=d['url'].split('#setup=')[1]
        code,d,_=post('/v1/team/access-link',{'email':'viewer@example.org'});new=d['url'].split('#setup=')[1]
        assert request('/auth/password/setup',{'token':old,'password':'new strong password 123'})[0]==400
        with concurrent.futures.ThreadPoolExecutor(2) as pool:
            results=list(pool.map(lambda _:request('/auth/password/setup',{'token':new,'password':'new strong password 123'})[0],range(2)))
        assert sorted(results)==[200,400],results
        assert request('/v1/auth/me',cookie=viewer)[0]==401
        assert request('/auth/password/login',{'email':'viewer@example.org','password':'correct horse battery staple'})[0]==401
        viewer,vc,_=login('viewer@example.org','new strong password 123')
        assert post('/v1/auth/logout',{},viewer,vc)[0]==200
        assert request('/v1/auth/me',cookie=viewer)[0]==401
        t=invite('disabled@example.org','viewer')
        assert post('/v1/team',dict(email='disabled@example.org',role='viewer',active=False,daily_requests=0,daily_usd=0,monthly_usd=0))[0]==200
        assert request('/auth/password/setup',{'token':t,'password':'correct horse battery staple'})[0]==400
        for _ in range(10):assert request('/auth/password/login',{'email':'unknown@example.org','password':'bad'})[0]==401
        assert request('/auth/password/login',{'email':'unknown@example.org','password':'bad'})[0]==429
        assert request('/auth/password/reset-request',{'email':'unknown@example.org'})[0]==200
        print('PASS: setup, Argon2 hashes, identity preservation, login/logout, CSRF, permissions, one-use/concurrent resets, revoked sessions/invitations, rate limits')
    finally:
        p.send_signal(signal.SIGINT)
        try:p.wait(timeout=10)
        except subprocess.TimeoutExpired:p.kill();p.wait()
        log.close()
