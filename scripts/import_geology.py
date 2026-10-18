"""Rebuild bundled geology data from downloaded IPIS CSV and USGS GDB.
Usage: data/geology/venv/bin/python scripts/import_geology.py
Requires fiona for the USGS OpenFileGDB reader. Raw inputs stay in ignored data/.
"""
import csv, json, hashlib, math
from pathlib import Path
from datetime import datetime, timezone
import fiona
ROOT = Path(__file__).resolve().parents[1]
RAW = ROOT / 'data/geology'
OUT = ROOT / 'dashboard/geology'
IPIS = 'https://ipisresearch.be/wp-content/uploads/2026/05/cod_mines_curated_all_opendata_p_ipis.csv'
USGS = 'https://doi.org/10.5066/P97EQWXP'
features = []
def feature(key, coordinates, properties):
    lon, lat = map(float, coordinates[:2])
    if not (math.isfinite(lon) and math.isfinite(lat) and 12 <= lon <= 32 and -14 <= lat <= 6):
        raise ValueError(f'Unexpected DRC coordinates for {key}: {coordinates}')
    return dict(type='Feature', id=key, geometry=dict(type='Point', coordinates=[round(lon,6),round(lat,6)]), properties=dict(id=key, **properties))
rows = list(csv.DictReader((RAW / 'ipis.csv').open()))
latest, counts = {}, {}
for r in rows:
    key = r['pcode']
    counts[key] = counts.get(key, 0) + 1
    if key not in latest or (r['visit_date'], int(r['vid'])) > (latest[key]['visit_date'], int(latest[key]['vid'])):
        latest[key] = r
for key, r in sorted(latest.items()):
    minerals = list(dict.fromkeys(r[f'mineral{i}'] for i in range(1,4) if r[f'mineral{i}']))
    features.append(feature('ipis:' + key, [r['longitude'],r['latitude']], dict(
        dataset='IPIS', kind='artisanal', name=r['name'], minerals=minerals, province=r['province'],
        date=r['visit_date'], observations=counts[key], source=r['source'], location_origin=r['location_origin'],
        reference='https://ipisresearch.be/home/maps-data/open-data/', license='ODC-BY 1.0')))
def clean(v):
    return '' if v is None or str(v).strip() in ('<null>', 'NULL') else str(v).strip()
for layer, kind in [('AFR_Mineral_Facilities','facility'),('AFR_Mineral_Deposits','deposit'),('AFR_Mineral_Exploration','exploration')]:
    with fiona.open(RAW / 'Africa_GIS.gdb', layer=layer) as records:
        assert records.crs.to_epsg() == 4326
        for record in records:
            p=record['properties']
            if p['Country'] != 'Congo (Kinshasa)': continue
            # Attribute meanings differ by feature class; preserve raw fields for audit.
            minerals = [clean(p['DsgAttr02'])] if kind == 'facility' else list(dict.fromkeys(clean(p.get(f'DsgAttr{i:02}')) for i in range(1,9) if clean(p.get(f'DsgAttr{i:02}'))))
            date=clean(p.get('DsgAttr06') if kind == 'facility' else p.get('DsgAttr11') if kind == 'exploration' else '')
            features.append(feature('usgs:'+kind+':'+p['FeatureUID']+':'+str(record.id),record['geometry']['coordinates'],dict(
                dataset='USGS',kind=kind,name=p['FeatureNam'],minerals=minerals,province=clean(p.get('ADM1')),
                date=date,source='USGS Africa GIS (2021)',reference=USGS,license='CC0 1.0',
                citation=clean(p.get('InfSource1')), original_attributes=dict(p))))
assert len({f['id'] for f in features}) == len(features)
metadata=dict(downloaded_at=datetime.now(timezone.utc).isoformat(), ipis_visits=len(rows), ipis_sites=len(latest),
    usgs_records=sum(f['properties']['dataset']=='USGS' for f in features),
    sources=[dict(name='IPIS',url=IPIS,license='ODC-BY 1.0',sha256=hashlib.sha256((RAW/'ipis.csv').read_bytes()).hexdigest()),
             dict(name='USGS Africa GIS',url=USGS,license='CC0 1.0',sha256=hashlib.sha256((RAW/'africa.zip').read_bytes()).hexdigest())],
    limitations='Historical observations and compiled records; not current operating status, reserves, ownership or territorial control. IPIS latest visit per pcode; USGS records can overlap IPIS and each other. Dates are source observation/report years; blank means unknown. No inferred rare-earth deposits.')
OUT.mkdir(exist_ok=True)
(OUT/'sites.geo.json').write_text(json.dumps(dict(type='FeatureCollection',metadata=metadata,features=features),ensure_ascii=False,separators=(',',':')))
print(json.dumps({k:v for k,v in metadata.items() if k not in ('sources','limitations')}))
