#!/usr/bin/env python3
"""Build dashboard/geo/territories.geo.json from geoBoundaries COD ADM2 (OCHA RGC, CC BY 3.0 IGO).

Usage: python3 -I scripts/import_territories.py ADM2.geojson dashboard/geology/provinces.geo.json OUT.geojson

Coordinates are rounded to 3 decimals (~100 m) and consecutive duplicates dropped, which keeps the
war-room map light. Each territory gets its province from the province polygon containing a point
of the territory.
"""
import json
import sys

SOURCE = "https://www.geoboundaries.org/api/current/gbOpen/COD/ADM2/"
DOWNLOAD = "https://github.com/wmgeolab/geoBoundaries/raw/9469f09/releaseData/gbOpen/COD/ADM2/geoBoundaries-COD-ADM2_simplified.geojson"


def rings(geom):
    if geom["type"] == "Polygon":
        return [geom["coordinates"]]
    return geom["coordinates"]


def inside(pt, ring):
    x, y = pt
    c = False
    j = len(ring) - 1
    for i in range(len(ring)):
        xi, yi = ring[i][:2]
        xj, yj = ring[j][:2]
        if (yi > y) != (yj > y) and x < (xj - xi) * (y - yi) / (yj - yi) + xi:
            c = not c
        j = i
    return c


def slim_ring(ring):
    out = []
    for x, y in (p[:2] for p in ring):
        q = [round(x, 3), round(y, 3)]
        if not out or out[-1] != q:
            out.append(q)
    if out[0] != out[-1]:
        out.append(out[0])
    return out if len(out) >= 4 else None


def interior_point(geom):
    """Mean of the outer ring of the largest polygon; good enough for province assignment."""
    poly = max(rings(geom), key=lambda p: len(p[0]))
    ring = poly[0]
    return [sum(p[0] for p in ring) / len(ring), sum(p[1] for p in ring) / len(ring)]


def main(adm2_path, provinces_path, out_path):
    adm2 = json.load(open(adm2_path))
    provinces = json.load(open(provinces_path))
    out = []
    for f in adm2["features"]:
        geom = f["geometry"]
        polys = []
        for poly in rings(geom):
            r = [slim_ring(x) for x in poly]
            if r[0]:
                polys.append([x for x in r if x])
        pt = interior_point(geom)
        province = next((p["properties"]["shapeName"] for p in provinces["features"] if any(inside(pt, poly[0]) for poly in rings(p["geometry"]))), None)
        name = f["properties"]["shapeName"]
        out.append({
            "type": "Feature",
            "id": len(out) + 1,
            "properties": {"name": name, "province": province, "ref": f["properties"]["shapeID"]},
            "geometry": {"type": "MultiPolygon", "coordinates": polys},
        })
    doc = {
        "type": "FeatureCollection",
        "source": {"name": "geoBoundaries COD ADM2 (Référentiel Géographique Commun, OCHA DR Congo)", "catalogue": SOURCE, "download": DOWNLOAD, "license": "CC BY 3.0 IGO", "processing": "coordinates rounded to 3 decimals; province assigned by containment"},
        "features": out,
    }
    json.dump(doc, open(out_path, "w"), ensure_ascii=False, separators=(",", ":"))
    missing = [f["properties"]["name"] for f in out if not f["properties"]["province"]]
    print(f"{len(out)} territories written; province not found for: {missing}")


if __name__ == "__main__":
    main(*sys.argv[1:4])
