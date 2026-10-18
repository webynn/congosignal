"""Import a DRC research subset of PA-X and curated CongoMines/official documents.
Run with bundled Python (pypdf). Inputs/archived PDFs stay in ignored data/agreements.
Metadata, extracted text, provenance and gaps are bundled for reproducibility.
"""
import csv, json, re, html, hashlib, urllib.request, concurrent.futures
from pathlib import Path
from datetime import datetime, timezone
from pypdf import PdfReader
ROOT=Path(__file__).resolve().parents[1];RAW=ROOT/'data/agreements';RAW.mkdir(exist_ok=True)
csv.field_size_limit(20000000)
items=[]
def term(date):
 if not date:return 'unverified'
 if date >= '2019-01-24':return 'Félix Tshisekedi'
 if date >= '2001-01-26':return 'Joseph Kabila'
 if date >= '1997-05-17':return 'Laurent-Désiré Kabila'
 return 'Mobutu Sese Seko'
for r in csv.DictReader((RAW/'corpus.csv').open(encoding='utf-8-sig')):
 if not ('Democratic Republic of Congo' in r['Con'] or r['Loc1ISO']=='COD' or r['Loc2ISO']=='COD'): continue
 text=r['Agreement text'];aid='pax-'+r['AgtId']
 items.append(dict(id=aid,title=r['Agt'],date=r['Dat'],date_basis='PA-X agreement date; signature status requires review',signed_term=term(r['Dat']),categories=['peace'],instrument=r['Stage'],signature_status='needs_review',parties=r['Part'],source_status=r['Status'],scope=r['Agtp'],publisher='PA-X / University of Edinburgh',url='https://www.peaceagreements.org/agreements/'+r['AgtId']+'/',license='CC BY-NC-SA 4.0; © University of Edinburgh; Bell & Badanjak (2019)',documents=[dict(id=aid+'-text',role='agreement',url='https://www.peaceagreements.org/agreements/'+r['AgtId']+'/',text=text,sha256=hashlib.sha256(text.encode()).hexdigest(),format='corpus_text',pages=[],status='available')]))
links=re.findall(r'<div class="pgdc-card-title"><a href="([^"]+)"[^>]*>(.*?)</a>',(RAW/'contracts.html').read_text(),re.S)
# Explicitly exclude reports, general legislation and decrees from agreement counts.
excluded={2,3,5,8,9,10,40,41,42}
seen=set();jobs=[]
for i,(url,title) in enumerate(links):
 if i in excluded or url in seen: continue
 seen.add(url);aid='cm-'+hashlib.sha256(url.encode()).hexdigest()[:12]
 title=html.unescape(title).removesuffix('.pdf').replace('_',' ')
 date=None;basis='Signature date not verified; filename year is not treated as a signing date'
 if i==4:date='2024-03-14';basis='IMF Country Report 24/226 reports signature on 14 March 2024'
 item=dict(id=aid,title=title,date=date,date_basis=basis,signed_term=term(date),categories=['minerals','infrastructure'] if 'sicomines' in title.lower() else ['minerals'],instrument='contract_or_amendment',signature_status='reported_signed' if i==4 else 'needs_review',parties='Requires document review: state / SOE / private / community must be distinguished',scope='economic',publisher='CongoMines / Carter Center document repository',url=url,license='Publicly disclosed document; underlying document rights apply',documents=[])
 items.append(item);jobs.append((item,url,'agreement'))
 if i==4:jobs.append((item,links[2][0],'implementation_report'))
# Official financing announcement is explicitly not mislabeled as a full contract.
extras=[('lobito-2023','Lobito Corridor cooperation memorandum','2023-10-26',['infrastructure','minerals'],'https://ec.europa.eu/commission/presscorner/api/files/document/print/en/ip_23_5303/IP_23_5303_EN.pdf','European Commission','official_announcement'),('transport-2022','DRC Transport and Connectivity Support Project (P161877)','2022-06-28',['infrastructure'],'https://documents1.worldbank.org/curated/en/141991655916124024/pdf/Congo-Democratic-Republic-of-Transport-and-Connectivity-Support-Project.pdf','World Bank','project_document'),('eu-minerals-2023','EU–DRC strategic raw-materials partnership memorandum','2023-10-26',['minerals'],'https://ec.europa.eu/commission/presscorner/api/files/document/print/en/ip_23_5303/IP_23_5303_EN.pdf','European Commission','official_announcement')]
for aid,title,date,cats,url,pub,role in extras:
 item=dict(id=aid,title=title,date=date,date_basis='Official signing announcement; full executed text not verified',signed_term=term(date),categories=cats,instrument='financing' if aid.startswith('transport') else 'memorandum',signature_status='reported_signed',parties='DRC government and partners; exact executed signatory list requires review',scope='economic',publisher=pub,url=url,license='Publisher reuse terms apply; attribution retained',documents=[]);items.append(item);jobs.append((item,url,role))
 if aid=='transport-2022':jobs.append((item,'https://documents1.worldbank.org/curated/en/099040723132520390/pdf/P16187703228e408a0bf2704ca2871bbfe1.pdf','implementation_report'))
def fetch(job):
 item,url,role=job;did=item['id']+'-'+role;p=RAW/(did+'.pdf');doc=dict(id=did,role=role,url=url,format='pdf',pages=[],text='',status='unavailable')
 try:
  if not p.exists():
   with urllib.request.urlopen(url,timeout=35) as r: content=r.read(25000001)
   if len(content)>25000000 or not content.startswith(b'%PDF'):raise ValueError('Not a supported PDF under 25 MB')
   p.write_bytes(content)
  reader=PdfReader(p);pages=[dict(page=i+1,text=page.extract_text() or '') for i,page in enumerate(reader.pages)]
  doc.update(pages=pages,text='\n\n'.join(x['text'] for x in pages),sha256=hashlib.sha256(p.read_bytes()).hexdigest(),status='available' if sum(len(x['text']) for x in pages)>100 else 'ocr_required')
 except Exception as e:doc['error']=str(e)[:200]
 ocr=RAW/(did+'.ocr.json')
 if ocr.exists():
  pages=json.loads(ocr.read_text());doc.update(pages=pages,text='\n\n'.join(x['text'] for x in pages),status='available',extraction='macOS Vision OCR; verify against original PDF')
 item['documents'].append(doc)
 print(did,doc['status'],len(doc['text']),flush=True)
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:list(pool.map(fetch,jobs))
supplements=RAW/'supplements.json'
if supplements.exists():items.extend(json.loads(supplements.read_text()))
for item in items:
 if item['id']=='pax-2868':item['categories']=['peace','minerals','infrastructure']
 if item['id']=='cm-0b26c21a6618':item['date_source_url']='https://www.elibrary.imf.org/view/journals/002/2024/226/article-A001-en.xml'
 if item['id']=='transport-2022':item['date_source_url']='https://www.worldbank.org/en/news/press-release/2022/06/28/world-bank-approves-750-million-to-support-critical-governance-reforms-transport-infrastructure-and-digital-connectivity'
meta=dict(imported_at=datetime.now(timezone.utc).isoformat(),coverage='Initial research register, NOT exhaustive. PA-X DRC subset through 2025; selected disclosed mining contracts and infrastructure instruments. Records include amendments, statements and local agreements. Central-government signature verification pending except where explicitly stated.',pax_source='https://www.peaceagreements.org/cms/documents/4494/pax_corpus_v10_agreements.csv',pax_sha256=hashlib.sha256((RAW/'corpus.csv').read_bytes()).hexdigest(),pax_license='CC BY-NC-SA 4.0, © University of Edinburgh; Bell & Badanjak (2019). Non-commercial research only. https://www.peaceagreements.org/media/documents/Terms_of_Use.pdf',methodology='Signing-term grouping is descriptive, not causal credit. Unknown dates stay unverified. AI drafts require review. Promises and observed outcomes are separate. No evidence is not zero effectiveness.')
(ROOT/'config/agreements.json').write_text(json.dumps(dict(metadata=meta,items=items),ensure_ascii=False,separators=(',',':')))
print('TOTAL',len(items),'DOCS',sum(len(i['documents']) for i in items))
