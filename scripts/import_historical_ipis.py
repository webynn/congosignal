"""Build historical province/year statistics; no site positions or control inference.
Reads the pinned IPIS download used by import_geology.py. Standard library only.
"""
import csv
import hashlib
import json
from collections import Counter, defaultdict
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROVINCES = ('Nord-Kivu', 'Sud-Kivu', 'Ituri')
SOURCE = 'https://ipisresearch.be/home/maps-data/open-data/'


def aggregate(rows):
    latest = {}
    visits = Counter()
    for r in rows:
        if r['province'] not in PROVINCES:
            continue
        day = date.fromisoformat(r['visit_date'])
        if not 2009 <= day.year <= 2023:
            continue
        if not r['pcode']:
            raise ValueError('Missing site identifier')
        for field in ('presence', 'interference'):
            if r[field] not in ('', '0', '1'):
                raise ValueError(f'Unknown {field} value')
        key = (r['province'], day.year, r['pcode'])
        visits[key[:2]] += 1
        if key not in latest or (r['visit_date'], int(r['vid'])) > (latest[key]['visit_date'], int(latest[key]['vid'])):
            latest[key] = r
    groups = defaultdict(list)
    for (province, year, _), r in latest.items():
        groups[province, year].append(r)
    result = []
    for (province, year), records in sorted(groups.items()):
        actors = Counter()
        for r in records:
            actors.update({r.get(f'armed_group{i}', '').strip() for i in range(1, 4)} - {''})
        metrics = {field: {'yes': sum(r[field] == '1' for r in records),
                           'no': sum(r[field] == '0' for r in records),
                           'unknown': sum(r[field] == '' for r in records)}
                   for field in ('presence', 'interference')}
        result.append(dict(province=province, year=year, sites=len(records),
                           visits=visits[province, year], **metrics,
                           actor_mentions=[dict(name=name, sites=n) for name, n in actors.most_common()]))
    return result


def main():
    raw = ROOT / 'data/geology/ipis.csv'
    with raw.open() as f:
        rows = list(csv.DictReader(f))
    data = dict(source='IPIS', url=SOURCE, license='ODC-BY 1.0',
                source_sha256=hashlib.sha256(raw.read_bytes()).hexdigest(),
                period=[2009, 2023],
                methodology='Latest visit per site within each calendar year, in three provinces. Missing values remain unknown. Actor mentions are not actor-specific interference findings. Sampling changes between years; counts are not territorial control or population estimates. No carry-forward into years without observations. Derived from a later corrected download, not a reconstruction of data available at the time.',
                rows=aggregate(rows))
    out = ROOT / 'dashboard/geology/historical-ipis.json'
    out.write_text(json.dumps(data, ensure_ascii=False, separators=(',', ':')))
    print(json.dumps(dict(province_years=len(data['rows']), site_years=sum(r['sites'] for r in data['rows']))))


if __name__ == '__main__':
    main()
