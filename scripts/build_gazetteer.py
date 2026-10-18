"""Build config/gazetteer.json from GeoNames (CC BY 4.0, https://www.geonames.org/) plus Wikidata coordinates (CC0).

Run: python3 -I scripts/build_gazetteer.py [--db data/osint.db]
Downloads CD.zip and admin1CodesASCII.txt into a temporary directory, keeps populated places,
administrative seats and territories, prefers conflict-zone provinces for duplicate names, and
drops names that collide with common words or public figures. With --db, names that appear far
more often in lower case than capitalized in the collected articles are dropped as common words.
"""
import argparse, io, json, re, sqlite3, unicodedata, urllib.parse, urllib.request, zipfile
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
UA = {'User-Agent': 'DrcOsintWorker/0.1 gazetteer build'}
# GeoNames admin1 code -> current French province name.
PROVINCES = {'06': 'Kinshasa', '08': 'Kongo-Central', '19': 'Kwango', '20': 'Kwilu', '24': 'Mai-Ndombe', '18': 'Kasaï',
             '23': 'Kasaï-Central', '04': 'Kasaï-Oriental', '21': 'Lomami', '27': 'Sankuru', '10': 'Maniema', '12': 'Sud-Kivu',
             '11': 'Nord-Kivu', '17': 'Ituri', '16': 'Haut-Uele', '30': 'Tshopo', '13': 'Bas-Uele', '26': 'Nord-Ubangi',
             '25': 'Mongala', '28': 'Sud-Ubangi', '02': 'Équateur', '31': 'Tshuapa', '29': 'Tanganyika', '15': 'Haut-Lomami',
             '22': 'Lualaba', '14': 'Haut-Katanga'}
# Provinces where every named village is kept, in preference order for duplicate names.
CONFLICT = ['11', '12', '17', '29', '10', '16']
# Lower rank wins for duplicate names. Territories outrank unpopulated villages with the same name.
RANK = {'PPLC': 0, 'PPLA': 1, 'PPLA2': 2, 'PPLA3': 3, 'PPLA4': 4, 'ADM2': 6, 'ADM3': 6}
# Kinshasa communes outrank same-named villages elsewhere.
KINSHASA_SECTION = 4
# Known seats GeoNames ranks below same-named villages: name -> admin1 code to prefer.
PREFER = {'kenge': '19'}
STOP = set('''
mais selon dans pour avec apres lors nous elle cette depuis entre contre vers sans sous chez dont ainsi aussi alors encore enfin
president ministre province ville centre nord sud est ouest grand grande petit petite haut bas mont lac port camp mission route
rive saint sainte notre dame paix union france belgique rwanda uganda ouganda burundi congo afrique europe kigali kampala
jean marie joseph pierre paul jacques moise felix daniel samuel david esther sarah rose grace jeanne luc marc andre michel louis
kabila tshisekedi kagame museveni lumumba mobutu kabarebe nangaa makenga bemba katumbi fayulu muzito sesanga matata kamerhe
suminwa bisimwa kanyuka ngoma ntaganda lubanga katanga nkunda mukulu sheka yakutumba amisi numbi boshab kanyama
mazembe vita mesa mobile petro samba kenya togo mali nairobi lusaka luanda kampala bujumbura doha washington paris bruxelles londres
mai mayi wazalendo fardc monusco mars avril juin juillet aout septembre octobre novembre decembre janvier fevrier
'''.split())

def fold(s):
    return ''.join(c for c in unicodedata.normalize('NFD', s) if unicodedata.category(c) != 'Mn').lower()

def fetch(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=120) as r:
        return r.read()

# Conflict localities absent from GeoNames: English Wikipedia title -> province. Coordinates come from Wikidata (CC0).
SUPPLEMENT = {'Rubaya': 'Nord-Kivu', 'Bunagana': 'Nord-Kivu', 'Kibumba': 'Nord-Kivu', 'Kanyabayonga': 'Nord-Kivu', 'Nyiragongo': 'Nord-Kivu',
              'Kamanyola': 'Sud-Kivu', 'Kavumu': 'Sud-Kivu', 'Mushaki': 'Nord-Kivu', 'Kiwanja': 'Nord-Kivu', 'Rubaya mine': None}

def wikidata_places(existing):
    titles = [t for t, prov in SUPPLEMENT.items() if prov and fold(t) not in existing]
    if not titles:
        return []
    url = 'https://www.wikidata.org/w/api.php?action=wbgetentities&sites=enwiki&props=claims|sitelinks&format=json&titles=' + urllib.parse.quote('|'.join(titles))
    out = []
    for e in json.loads(fetch(url)).get('entities', {}).values():
        title = e.get('sitelinks', {}).get('enwiki', {}).get('title')
        coord = (e.get('claims', {}).get('P625') or [{}])[0].get('mainsnak', {}).get('datavalue', {}).get('value')
        if title in SUPPLEMENT and coord:
            out.append({'name': title, 'province': SUPPLEMENT[title], 'lat': round(coord['latitude'], 4), 'lon': round(coord['longitude'], 4), 'kind': 'WIKIDATA', 'population': 0, 'major': True, 'wikidata': e['id']})
    return out

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--db')
    args = ap.parse_args()
    rows = zipfile.ZipFile(io.BytesIO(fetch('https://download.geonames.org/export/dump/CD.zip'))).read('CD.txt').decode().splitlines()
    candidates, admin_names = {}, set()
    province_names = {fold(p).replace('-', ' ') for p in PROVINCES.values()} - {'kinshasa'}
    for line in rows:
        f = line.split('\t')
        name, lat, lon, fclass, code, admin1, pop = f[1], float(f[4]), float(f[5]), f[6], f[7], f[10], int(f[14] or 0)
        if admin1 not in PROVINCES or len(name) < 4 or len(name) > 40 or not re.match(r"^[\w' -]+$", name):
            continue
        kinshasa_section = admin1 == '06' and (fclass == 'P' or code == 'ADM3')
        keep = code in RANK or kinshasa_section or (fclass == 'P' and (pop > 0 or admin1 in CONFLICT))
        if not keep or fold(name) in STOP or fold(name).replace('-', ' ') in province_names:
            continue
        if code in ('ADM2', 'ADM3'):
            admin_names.add((fold(name), admin1))
        # Lower score wins: administrative rank, then population, then conflict-province preference.
        rank = KINSHASA_SECTION if kinshasa_section else RANK.get(code, 5 if pop > 0 else 7)
        if PREFER.get(fold(name)) == admin1:
            rank = -1
        score = (rank, 0 if pop > 0 else 1, CONFLICT.index(admin1) if admin1 in CONFLICT else 9, -pop)
        key = fold(name)
        if key not in candidates or score < candidates[key][0]:
            candidates[key] = (score, {'name': name, 'province': PROVINCES[admin1], 'lat': round(lat, 4), 'lon': round(lon, 4), 'kind': code, 'population': pop})
    if args.db:
        text = ' '.join(r[0] or '' for r in sqlite3.connect(args.db).execute('SELECT title || " " || content FROM articles'))
        words = Counter(re.findall(r"[\w'-]+", text))
        dropped = []
        for key, (_, p) in list(candidates.items()):
            if ' ' in p['name']:
                continue
            lower = sum(n for w, n in words.items() if w.islower() and fold(w) == key)
            upper = sum(n for w, n in words.items() if w[:1].isupper() and fold(w) == key)
            if lower >= 3 and lower > upper:
                dropped.append(p['name']); del candidates[key]
        print('dropped as common words:', len(dropped), sorted(dropped)[:80])
    code_of = {v: k for k, v in PROVINCES.items()}
    for key, (score, p) in candidates.items():
        # Major places match anywhere; minor villages need a locative cue in the text.
        p['major'] = score[0] <= 6 or (key, code_of[p['province']]) in admin_names
    extra = wikidata_places(candidates)
    print('wikidata supplements:', [p['name'] for p in extra])
    places = sorted([p for _, p in candidates.values()] + extra, key=lambda p: (p['province'], p['name']))
    out = ROOT / 'config/gazetteer.json'
    out.write_text(json.dumps({'source': 'GeoNames (https://www.geonames.org/), CC BY 4.0; supplements from Wikidata, CC0', 'places': places}, ensure_ascii=False, separators=(',', ':')))
    print(f'{len(places)} places -> {out}')

main()
