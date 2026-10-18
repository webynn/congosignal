"""Explicit chronological evidence links; upload dates are not report dates."""
from pathlib import Path
import json,copy,shutil
root=Path(__file__).resolve().parents[1];p=root/'config/agreements.json';j=json.loads(p.read_text());byid={i['id']:i for i in j['items']}
amendment=byid['cm-0b26c21a6618'];original=byid['cm-c24ee67a2dba']
report=next(d for d in amendment['documents'] if 'implementation_report' in d['id'])
report.update(role='historical_context',report_date='2021-09',date_precision='month',date_basis='Report title pages: Kinshasa, Septembre 2021; not the 2026 upload path')
linked=copy.deepcopy(report);linked.update(id=original['id']+'-implementation_report',role='implementation_report')
original['documents']=[d for d in original['documents'] if d['id']!=linked['id']]+[linked]
original.update(date='2008-04-22',signed_term='Joseph Kabila',date_basis='Original convention date explicitly identified on Amendment 5 title page (2024); not inferred from filename',date_source_url=amendment['url'])
shutil.copyfile(root/'data/agreements'/ (report['id']+'.pdf'),root/'data/agreements'/(linked['id']+'.pdf'))
for d in byid['transport-2022']['documents']:
 if d['role']=='implementation_report':d.update(report_date='2023-04-07',date_precision='day',date_basis='World Bank ISR archived 7 April 2023')
p.write_text(json.dumps(j,ensure_ascii=False,separators=(',',':')))
