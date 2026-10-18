"""OCR unreadable local PDFs, keeping source hashes and marking extraction uncertainty."""
import json,subprocess,concurrent.futures
from pathlib import Path
root=Path(__file__).resolve().parents[1];p=root/'config/agreements.json';data=json.loads(p.read_text());raw=root/'data/agreements'
jobs=[d for i in data['items'] for d in i['documents'] if d['status']=='ocr_required']
def run(d):
 out=raw/(d['id']+'.ocr.json')
 try:
  if not out.exists():subprocess.run([str(raw/'ocr'),str(raw/(d['id']+'.pdf')),str(out)],check=True,timeout=600,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
  pages=json.loads(out.read_text());d.update(pages=pages,text='\n\n'.join(x['text'] for x in pages),status='available',extraction='macOS Vision OCR; verify against original PDF');print(d['id'],'OCR',len(pages),'pages',flush=True)
 except Exception as e:print(d['id'],'OCR pending',type(e).__name__,flush=True)
with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:list(pool.map(run,jobs))
p.write_text(json.dumps(data,ensure_ascii=False,separators=(',',':')))
