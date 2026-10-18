"""Collect official announcements as discovery evidence, never as executed agreements."""
import urllib.request,re,html,json,hashlib
from pathlib import Path
root=Path(__file__).resolve().parents[1]
entries=[
 ('us-drc-2025','DRC–United States Strategic Partnership Agreement','2025-12-04',['minerals','infrastructure'],'https://economie.gouv.cd/actualites/accord-partenariat-strategique-rdc-usa','DRC Ministry of Economy'),
 ('mota-engil-2026','DRC–Mota-Engil concession and collaboration instruments','2026-08-26',['infrastructure'],'https://www.apcsc.gouv.cd/article.php?id=28','DRC APCSC'),
]
out=[]
for aid,title,date,cats,url,pub in entries:
 doc=dict(id=aid+'-announcement',role='official_announcement',url=url,format='html',status='unavailable',text='',pages=[])
 try:
  data=urllib.request.urlopen(url,timeout=30).read();s=data.decode('utf8',errors='replace')
  text='\n\n'.join(html.unescape(re.sub('<[^>]+>',' ',t)) for t in re.findall(r'<(?:p|h1)[^>]*>(.*?)</(?:p|h1)>',s,re.S|re.I))
  doc.update(status='available',text=text,sha256=hashlib.sha256(data).hexdigest())
 except Exception as e:doc['error']=str(e)[:200]
 out.append(dict(id=aid,title=title,date=date,date_basis='Official signing announcement; executed text not yet obtained',signed_term='Félix Tshisekedi',categories=cats,instrument='reported_agreement',signature_status='reported_signed',parties='DRC government and named counterpart; full executed signatory list requires verification',scope='economic',publisher=pub,url=url,license='Official publication; underlying rights apply',documents=[doc]))
 print(aid,doc['status'])
(root/'data/agreements/supplements.json').write_text(json.dumps(out,ensure_ascii=False))
