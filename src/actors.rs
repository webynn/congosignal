//! Key figures and sanctions. Official DRC-regime lists (UN 1533 "DRC", US OFAC "DRCONGO",
//! EU "COD", UK "Democratic Republic of the Congo") are merged with curated public figures,
//! enriched from Wikidata/Commons and linked to mentions in news, X posts and documents.
//! Passport and identity-document numbers are never stored. Nothing is inferred beyond what
//! the lists, Wikidata or the cited text say.
use crate::reference::{self, CorpusDoc};
use crate::text::{self, Doc, Term};
use crate::{ApiError, ApiResult, App};
use anyhow::{Context, Result};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};
use std::collections::{BTreeSet, HashMap};
use std::sync::LazyLock;

#[derive(Deserialize, Serialize, Clone)]
pub struct Group {
    pub id: String,
    pub en: String,
    pub fr: String,
    pub terms: Vec<String>,
}
#[derive(Deserialize, Clone)]
struct Curated {
    id: String,
    kind: String,
    name: String,
    group: Option<String>,
    enwiki: Option<String>,
    #[serde(default)]
    terms: Vec<String>,
    #[serde(default)]
    aliases: Vec<String>,
}
#[derive(Deserialize)]
struct Config {
    groups: Vec<Group>,
    people: Vec<Curated>,
}
static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../config/key-figures.json")).expect("valid key figures")
});
pub fn groups() -> &'static [Group] {
    &CONFIG.groups
}

struct List {
    id: &'static str,
    label: &'static str,
    url: &'static str,
    license: &'static str,
    parse: fn(&[u8]) -> Result<Vec<Record>>,
}
const LISTS: &[List] = &[
    List {
        id: "UN",
        label: "UN Security Council 1533 (DRC)",
        url: "https://scsanctions.un.org/resources/xml/en/consolidated.xml",
        license: "United Nations public information; attribute the UN Security Council",
        parse: parse_un,
    },
    List {
        id: "US",
        label: "US Treasury OFAC SDN (DRCONGO)",
        url: "https://www.treasury.gov/ofac/downloads/sdn.csv",
        license: "US Government work, public domain",
        parse: parse_ofac,
    },
    List {
        id: "EU",
        label: "EU Financial Sanctions (COD)",
        url: "https://webgate.ec.europa.eu/fsd/fsf/public/files/xmlFullSanctionsList_1_1/content?token=dG9rZW4tMjAxNw",
        license: "European Commission reuse policy (Decision 2011/833/EU), attribution required",
        parse: parse_eu,
    },
    List {
        id: "UK",
        label: "UK Sanctions List (DRC regime)",
        url: "https://ofsistorage.blob.core.windows.net/publishlive/2022format/ConList.csv",
        license: "Open Government Licence v3.0",
        parse: parse_uk,
    },
];

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Record {
    list: String,
    reference: String,
    kind: String,
    name: String,
    aliases: Vec<String>,
    program: String,
    listed_on: Option<String>,
    titles: Vec<String>,
    dob: Vec<String>,
    pob: Vec<String>,
    nationality: Vec<String>,
    reason: Option<String>,
    url: String,
}

// ---------------------------------------------------------------------------------------------
// Parsers

fn clean(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn push_unique(v: &mut Vec<String>, s: String) {
    let s = clean(&s);
    if !s.is_empty() && s != "-0-" && !v.iter().any(|x| x.eq_ignore_ascii_case(&s)) {
        v.push(s);
    }
}
fn xml_doc(bytes: &[u8]) -> Result<String> {
    Ok(String::from_utf8_lossy(bytes).into_owned())
}
fn child_text(n: roxmltree::Node, tag: &str) -> Option<String> {
    n.children()
        .find(|c| c.tag_name().name() == tag)
        .and_then(|c| c.text())
        .map(clean)
        .filter(|s| !s.is_empty())
}
fn values(n: roxmltree::Node, tag: &str) -> Vec<String> {
    let mut out = Vec::new();
    for c in n.children().filter(|c| c.tag_name().name() == tag) {
        for v in c.descendants().filter(|d| d.tag_name().name() == "VALUE") {
            if let Some(t) = v.text() {
                push_unique(&mut out, t.into());
            }
        }
    }
    out
}
fn parse_un(bytes: &[u8]) -> Result<Vec<Record>> {
    let xml = xml_doc(bytes)?;
    let doc = roxmltree::Document::parse(&xml)?;
    let mut out = Vec::new();
    for n in doc
        .descendants()
        .filter(|n| matches!(n.tag_name().name(), "INDIVIDUAL" | "ENTITY"))
    {
        if child_text(n, "UN_LIST_TYPE").as_deref() != Some("DRC") {
            continue;
        }
        let person = n.tag_name().name() == "INDIVIDUAL";
        let name = ["FIRST_NAME", "SECOND_NAME", "THIRD_NAME", "FOURTH_NAME"]
            .iter()
            .filter_map(|t| child_text(n, t))
            .collect::<Vec<_>>()
            .join(" ");
        let mut r = Record {
            list: "UN".into(),
            reference: child_text(n, "REFERENCE_NUMBER").unwrap_or_default(),
            kind: if person { "person" } else { "entity" }.into(),
            name: clean(&name),
            program: "DRC (resolution 1533)".into(),
            listed_on: child_text(n, "LISTED_ON"),
            reason: child_text(n, "COMMENTS1"),
            url: "https://main.un.org/securitycouncil/en/sanctions/1533/materials".into(),
            ..Default::default()
        };
        for a in n
            .descendants()
            .filter(|d| d.tag_name().name() == "ALIAS_NAME")
        {
            if let Some(t) = a.text() {
                push_unique(&mut r.aliases, t.into());
            }
        }
        r.titles = [values(n, "TITLE"), values(n, "DESIGNATION")].concat();
        r.nationality = values(n, "NATIONALITY");
        for d in n
            .children()
            .filter(|c| c.tag_name().name() == "INDIVIDUAL_DATE_OF_BIRTH")
        {
            if let Some(v) = child_text(d, "DATE").or_else(|| child_text(d, "YEAR")) {
                push_unique(&mut r.dob, v);
            }
        }
        for p in n
            .children()
            .filter(|c| c.tag_name().name() == "INDIVIDUAL_PLACE_OF_BIRTH")
        {
            let place = ["CITY", "STATE_PROVINCE", "COUNTRY"]
                .iter()
                .filter_map(|t| child_text(p, t))
                .collect::<Vec<_>>()
                .join(", ");
            push_unique(&mut r.pob, place);
        }
        out.push(r);
    }
    Ok(out)
}
/// Remarks segments that carry identity-document or contact details are dropped.
fn sensitive(seg: &str) -> bool {
    let s = seg.to_lowercase();
    [
        "passport",
        "national id",
        "identification",
        "cedula",
        "ssn",
        "tax id",
        "email",
        "phone",
        "registration",
        "id no",
        "residency",
        "driver",
    ]
    .iter()
    .any(|k| s.contains(k))
}
fn parse_ofac(bytes: &[u8]) -> Result<Vec<Record>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(bytes);
    let mut out = Vec::new();
    for row in rdr.records() {
        let row = row?;
        let f = |i: usize| {
            row.get(i)
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && *s != "-0-")
                .map(String::from)
        };
        let program = f(3).unwrap_or_default();
        if !program.contains("DRCONGO") {
            continue;
        }
        let raw_name = f(1).unwrap_or_default();
        let name = match raw_name.split_once(", ") {
            Some((last, first)) => format!("{first} {last}"),
            None => raw_name,
        };
        let id = f(0).unwrap_or_default();
        let mut r = Record {
            list: "US".into(),
            reference: format!("OFAC {id}"),
            kind: if f(2).as_deref() == Some("individual") {
                "person"
            } else {
                "entity"
            }
            .into(),
            name: clean(&name),
            program: "DRCONGO".into(),
            url: format!("https://sanctionssearch.ofac.treas.gov/Details.aspx?id={id}"),
            ..Default::default()
        };
        if let Some(t) = f(4) {
            r.titles.push(t);
        }
        let mut reason = Vec::new();
        for seg in f(11).unwrap_or_default().split(';').map(str::trim) {
            let seg = seg.trim_end_matches('.');
            let lower = seg.to_lowercase();
            if let Some(v) = seg.strip_prefix("alt. DOB ").or(seg.strip_prefix("DOB ")) {
                push_unique(&mut r.dob, v.into());
            } else if let Some(v) = seg.strip_prefix("alt. POB ").or(seg.strip_prefix("POB ")) {
                push_unique(&mut r.pob, v.into());
            } else if let Some(v) = seg
                .strip_prefix("nationality ")
                .or(seg.strip_prefix("citizen "))
                .or(seg.strip_prefix("alt. nationality "))
                .or(seg.strip_prefix("alt. citizen "))
            {
                push_unique(&mut r.nationality, v.into());
            } else if lower.contains("a.k.a.") {
                for part in seg.split('\'').skip(1).step_by(2) {
                    push_unique(&mut r.aliases, part.into());
                }
            } else if !seg.is_empty() && !sensitive(seg) && !lower.starts_with("gender") {
                reason.push(seg.to_string());
            }
        }
        r.reason = (!reason.is_empty()).then(|| reason.join("; "));
        out.push(r);
    }
    Ok(out)
}
fn parse_eu(bytes: &[u8]) -> Result<Vec<Record>> {
    let xml = xml_doc(bytes)?;
    let doc = roxmltree::Document::parse(&xml)?;
    let mut out = Vec::new();
    for n in doc
        .descendants()
        .filter(|n| n.tag_name().name() == "sanctionEntity")
    {
        let regs: Vec<_> = n
            .children()
            .filter(|c| c.tag_name().name() == "regulation")
            .collect();
        if !regs.iter().any(|r| r.attribute("programme") == Some("COD")) {
            continue;
        }
        let kind = n
            .children()
            .find(|c| c.tag_name().name() == "subjectType")
            .and_then(|c| c.attribute("code"))
            .unwrap_or("enterprise");
        let mut r = Record {
            list: "EU".into(),
            reference: n.attribute("euReferenceNumber").unwrap_or_default().into(),
            kind: if kind == "person" { "person" } else { "entity" }.into(),
            program: "COD".into(),
            listed_on: n.attribute("designationDate").map(String::from),
            url: regs
                .iter()
                .find_map(|r| {
                    r.children()
                        .find(|c| c.tag_name().name() == "publicationUrl")
                        .and_then(|c| c.text())
                })
                .unwrap_or("https://www.sanctionsmap.eu/")
                .into(),
            ..Default::default()
        };
        for a in n.children().filter(|c| c.tag_name().name() == "nameAlias") {
            let whole = clean(a.attribute("wholeName").unwrap_or_default());
            if r.name.is_empty() {
                r.name = whole;
            } else {
                push_unique(&mut r.aliases, whole);
            }
            if let Some(f) = a.attribute("function") {
                push_unique(&mut r.titles, f.into());
            }
        }
        for b in n.children().filter(|c| c.tag_name().name() == "birthdate") {
            if let Some(v) = b.attribute("birthdate").or(b.attribute("year")) {
                push_unique(&mut r.dob, v.into());
            }
            let place = [b.attribute("city"), b.attribute("countryDescription")]
                .into_iter()
                .flatten()
                .filter(|s| !s.is_empty() && *s != "UNKNOWN")
                .collect::<Vec<_>>()
                .join(", ");
            push_unique(&mut r.pob, place);
        }
        for c in n
            .children()
            .filter(|c| c.tag_name().name() == "citizenship")
        {
            if let Some(v) = c
                .attribute("countryDescription")
                .filter(|v| *v != "UNKNOWN")
            {
                push_unique(&mut r.nationality, v.into());
            }
        }
        let remarks: Vec<String> = n
            .children()
            .filter(|c| c.tag_name().name() == "remark")
            .filter_map(|c| c.text())
            .map(clean)
            .filter(|s| !sensitive(s))
            .collect();
        r.reason = (!remarks.is_empty()).then(|| remarks.join(" "));
        if !r.name.is_empty() {
            out.push(r);
        }
    }
    Ok(out)
}
/// Values of "(Key):value" segments in the UK "Other Information" field.
fn uk_field(info: &str, key: &str) -> Option<String> {
    let start = info.find(&format!("({key}):"))? + key.len() + 3;
    let rest = &info[start..];
    // The next segment starts with "(Label):" where the label has no parentheses.
    let mut end = rest.len();
    for (i, _) in rest.match_indices('(') {
        if let Some(close) = rest[i..].find("):")
            && close < 45
            && !rest[i + 1..i + close].contains('(')
        {
            end = i;
            break;
        }
    }
    Some(clean(&rest[..end])).filter(|s| !s.is_empty())
}
fn uk_date(s: &str) -> Option<String> {
    let p: Vec<&str> = s.trim().split('/').collect();
    match p.as_slice() {
        [d, m, y] if y.len() == 4 => Some(match (*d, *m) {
            ("00", "00") | (_, "00") => y.to_string(),
            ("00", m) => format!("{y}-{m}"),
            (d, m) => format!("{y}-{m}-{d}"),
        }),
        _ => None,
    }
}
fn parse_uk(bytes: &[u8]) -> Result<Vec<Record>> {
    let text = String::from_utf8_lossy(bytes);
    let body = text.split_once('\n').map_or("", |(_, rest)| rest);
    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(body.as_bytes());
    let headers = rdr.headers()?.clone();
    let col = |name: &str| headers.iter().position(|h| h.trim() == name);
    let (regime, group_id) = (
        col("Regime").context("UK list: missing Regime column")?,
        col("Group ID").context("UK list: missing Group ID column")?,
    );
    let mut groups: Vec<(String, Vec<csv::StringRecord>)> = Vec::new();
    for row in rdr.records() {
        let row = row?;
        if row.get(regime) != Some("Democratic Republic of the Congo") {
            continue;
        }
        let id = row.get(group_id).unwrap_or_default().to_string();
        match groups.iter_mut().find(|(g, _)| *g == id) {
            Some((_, rows)) => rows.push(row),
            None => groups.push((id, vec![row])),
        }
    }
    let get = |row: &csv::StringRecord, name: &str| {
        col(name)
            .and_then(|i| row.get(i))
            .map(clean)
            .filter(|s| !s.is_empty())
    };
    let full_name = |row: &csv::StringRecord| {
        clean(
            &["Name 1", "Name 2", "Name 3", "Name 4", "Name 5", "Name 6"]
                .iter()
                .filter_map(|n| get(row, n))
                .collect::<Vec<_>>()
                .join(" "),
        )
    };
    let mut out = Vec::new();
    for (id, rows) in groups {
        let primary = rows
            .iter()
            .find(|r| get(r, "Alias Type").is_some_and(|t| t.to_lowercase().contains("primary")))
            .unwrap_or(&rows[0]);
        let info = get(primary, "Other Information").unwrap_or_default();
        let mut r = Record {
            list: "UK".into(),
            reference: uk_field(&info, "UK Sanctions List Ref")
                .map(|r| r.trim_end_matches('.').to_string())
                .unwrap_or_else(|| format!("UK group {id}")),
            kind: if get(primary, "Group Type").as_deref() == Some("Individual") {
                "person"
            } else {
                "entity"
            }
            .into(),
            name: full_name(primary),
            program: "Democratic Republic of the Congo".into(),
            listed_on: get(primary, "Listed On").and_then(|d| uk_date(&d)),
            reason: uk_field(&info, "UK Statement of Reasons"),
            url: "https://www.gov.uk/government/publications/the-uk-sanctions-list".into(),
            ..Default::default()
        };
        for row in &rows {
            push_unique(&mut r.aliases, full_name(row));
            if let Some(v) = get(row, "Position") {
                push_unique(&mut r.titles, v);
            }
            if let Some(v) = get(row, "DOB").and_then(|d| uk_date(&d)) {
                push_unique(&mut r.dob, v);
            }
            let pob = [get(row, "Town of Birth"), get(row, "Country of Birth")]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(", ");
            push_unique(&mut r.pob, pob);
            if let Some(v) = get(row, "Nationality") {
                push_unique(&mut r.nationality, v);
            }
        }
        let name = r.name.clone();
        r.aliases.retain(|a| !a.eq_ignore_ascii_case(&name));
        out.push(r);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------
// Merging lists and curated figures into actors

struct Actor {
    id: String,
    kind: String,
    name: String,
    aliases: Vec<String>,
    curated: Option<Curated>,
    records: Vec<Record>,
    groups: Vec<String>,
    designation_groups: Vec<String>,
}
fn title_case(s: &str) -> String {
    s.split(' ')
        .map(|w| {
            w.split('-')
                .map(|p| {
                    let mut c = p.chars();
                    c.next()
                        .map(|f| {
                            f.to_uppercase()
                                .chain(c.flat_map(char::to_lowercase))
                                .collect()
                        })
                        .unwrap_or_default()
                })
                .collect::<Vec<String>>()
                .join("-")
        })
        .collect::<Vec<_>>()
        .join(" ")
}
fn slug(s: &str) -> String {
    text::tokens(s).join("-")
}
fn years(r: &Record) -> BTreeSet<String> {
    r.dob
        .iter()
        .flat_map(|d| {
            d.split(|c: char| !c.is_ascii_digit())
                .filter(|p| p.len() == 4)
                .map(String::from)
                .collect::<Vec<_>>()
        })
        .collect()
}
/// Name variants as token sets; entity names split on ';' and parenthesized acronyms.
fn variants(names: &[String], entity: bool) -> Vec<BTreeSet<String>> {
    let mut out = Vec::new();
    for n in names {
        for part in n.split(';') {
            let set: BTreeSet<String> = text::tokens(part).into_iter().collect();
            if set.len() >= 2 || (entity && set.iter().any(|t| t.len() >= 3)) {
                out.push(set);
            }
            if entity {
                for acr in part.split('(').skip(1).filter_map(|s| s.split(')').next()) {
                    let set: BTreeSet<String> = text::tokens(acr).into_iter().collect();
                    if set.len() == 1
                        && set
                            .iter()
                            .all(|t| t.len() >= 3 && !matches!(t.as_str(), "ngo" | "ltd" | "sarl"))
                    {
                        out.push(set);
                    }
                }
            }
        }
    }
    out
}
fn find(p: &mut [usize], i: usize) -> usize {
    if p[i] != i {
        p[i] = find(p, p[i]);
    }
    p[i]
}
fn build_actors(records: Vec<Record>) -> Vec<Actor> {
    let curated = &CONFIG.people;
    let n = records.len() + curated.len();
    let mut parent: Vec<usize> = (0..n).collect();
    let union = |p: &mut Vec<usize>, a: usize, b: usize| {
        let (ra, rb) = (find(p, a), find(p, b));
        if ra != rb {
            p[ra] = rb;
        }
    };
    let names_of = |i: usize| -> (Vec<String>, bool) {
        if i < records.len() {
            let r = &records[i];
            (
                [vec![r.name.clone()], r.aliases.clone()].concat(),
                r.kind == "entity",
            )
        } else {
            let c = &curated[i - records.len()];
            (
                [vec![c.name.clone()], c.aliases.clone()].concat(),
                c.kind != "person",
            )
        }
    };
    let all: Vec<(Vec<BTreeSet<String>>, bool)> = (0..n)
        .map(|i| {
            let (names, entity) = names_of(i);
            (variants(&names, entity), entity)
        })
        .collect();
    let mut exact: HashMap<(BTreeSet<String>, bool), usize> = HashMap::new();
    for (i, (vs, entity)) in all.iter().enumerate() {
        for v in vs {
            match exact.get(&(v.clone(), *entity)) {
                Some(&j) => union(&mut parent, i, j),
                None => {
                    exact.insert((v.clone(), *entity), i);
                }
            }
        }
    }
    // Subset matches: curated names inside list names, and cross-list persons with compatible birth years.
    for i in 0..n {
        for j in 0..n {
            if i == j || all[i].1 != all[j].1 {
                continue;
            }
            let curated_i = i >= records.len();
            let compatible = curated_i
                || (j < records.len() && records[i].list != records[j].list && {
                    let (a, b) = (years(&records[i]), years(&records[j]));
                    a.is_empty() || b.is_empty() || !a.is_disjoint(&b)
                });
            if compatible
                && all[i].0.iter().any(|a| {
                    a.len() >= 2 && all[j].0.iter().any(|b| a.len() < b.len() && a.is_subset(b))
                })
            {
                union(&mut parent, i, j);
            }
        }
    }
    let mut components: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        components.entry(find(&mut parent, i)).or_default().push(i);
    }
    let mut actors = Vec::new();
    let mut used_ids = BTreeSet::new();
    let mut comps: Vec<Vec<usize>> = components.into_values().collect();
    comps.sort();
    for members in comps {
        let cur = members
            .iter()
            .find(|&&i| i >= records.len())
            .map(|&i| curated[i - records.len()].clone());
        let mut recs: Vec<Record> = members
            .iter()
            .filter(|&&i| i < records.len())
            .map(|&i| records[i].clone())
            .collect();
        recs.sort_by_key(|r| ["UN", "US", "EU", "UK"].iter().position(|l| *l == r.list));
        // Primary names: curated, then UN, UK, US (reordered on parse), then EU, whose first alias
        // is not always the primary name. List names in capitals are title-cased.
        let display = cur.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| {
            let r = ["UN", "UK", "US", "EU"]
                .iter()
                .find_map(|l| recs.iter().find(|r| r.list == *l))
                .unwrap_or(&recs[0]);
            let name = clean(&r.name.replace(',', " "));
            if name.chars().any(char::is_lowercase) && r.list != "US" {
                name
            } else {
                title_case(&name.to_lowercase())
            }
        });
        let mut aliases = Vec::new();
        for r in &recs {
            push_unique(&mut aliases, title_case(&r.name.to_lowercase()));
            for a in &r.aliases {
                push_unique(&mut aliases, a.clone());
            }
        }
        aliases.retain(|a| text::fold(a) != text::fold(&display));
        aliases.truncate(20);
        let mut id = cur
            .as_ref()
            .map(|c| c.id.clone())
            .unwrap_or_else(|| slug(&display));
        if !used_ids.insert(id.clone()) {
            id = format!("{id}-{}", slug(&recs[0].reference));
            used_ids.insert(id.clone());
        }
        let kind = match &cur {
            Some(c) => c.kind.clone(),
            None if recs[0].kind == "person" => "person".into(),
            None => "organization".into(),
        };
        let designation = Doc::new(
            &recs
                .iter()
                .map(|r| {
                    format!(
                        "{} {}",
                        r.titles.join(" "),
                        r.reason.clone().unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join(" . "),
        );
        let mut designation_groups: Vec<String> = groups()
            .iter()
            .filter(|g| g.terms.iter().any(|t| designation.has(&text::tokens(t))))
            .map(|g| g.id.clone())
            .collect();
        if [
            "gold",
            "mineral",
            "minerals",
            "refinery",
            "coltan",
            "minerais",
            "cassiterite",
            "tantalum",
        ]
        .iter()
        .any(|w| designation.has(&[w.to_string()]))
            && recs
                .iter()
                .any(|r| r.kind == "entity" || r.reason.is_some())
            && !designation_groups.contains(&"minerals".to_string())
        {
            designation_groups.push("minerals".into());
        }
        actors.push(Actor {
            id,
            kind,
            groups: cur
                .as_ref()
                .and_then(|c| c.group.clone())
                .into_iter()
                .collect(),
            name: display,
            aliases,
            curated: cur,
            records: recs,
            designation_groups,
        });
    }
    actors.sort_by(|a, b| a.name.cmp(&b.name));
    actors
}
/// Watchable actors: id, display name, sanctioned flag and the match terms used for mentions.
pub async fn watch_terms(db: &SqlitePool) -> Result<Vec<(String, String, bool, Vec<Term>)>> {
    Ok(build_actors(load_records(db).await?)
        .into_iter()
        .map(|a| {
            let terms = a.terms();
            (a.id, a.name, !a.records.is_empty(), terms)
        })
        .collect())
}
impl Actor {
    fn terms(&self) -> Vec<Term> {
        let mut out: Vec<Term> = Vec::new();
        let mut add = |raw: &str, acronym: bool| {
            let tokens = text::tokens(raw);
            if !tokens.is_empty() && !out.iter().any(|t| t.tokens == tokens) {
                out.push(Term { tokens, acronym });
            }
        };
        if let Some(c) = &self.curated {
            for t in &c.terms {
                add(t, t.len() <= 5 && t.chars().all(|c| !c.is_lowercase()));
            }
        }
        let person = self.kind == "person";
        for name in std::iter::once(&self.name).chain(&self.aliases) {
            for part in name.split(';') {
                let toks = text::tokens(part);
                if person {
                    if toks.len() >= 2 {
                        add(part, false);
                    }
                    if toks.len() >= 3 {
                        add(&format!("{} {}", toks[0], toks[toks.len() - 1]), false);
                    }
                } else if toks.len() >= 2 {
                    add(part, false);
                }
                if !person {
                    for acr in part.split('(').skip(1).filter_map(|s| s.split(')').next()) {
                        let t = acr.trim();
                        if t.len() >= 3
                            && t.chars().all(|c| !c.is_lowercase())
                            && !matches!(t, "NGO" | "LTD" | "SARL")
                        {
                            add(t, true);
                        }
                    }
                    if toks.len() == 1
                        && part.trim().len() >= 3
                        && part.trim().chars().all(|c| !c.is_lowercase())
                    {
                        add(part.trim(), true);
                    }
                }
            }
        }
        out
    }
    fn enwiki_candidates(&self) -> Vec<(String, &'static str)> {
        if let Some(c) = &self.curated {
            return c
                .enwiki
                .iter()
                .map(|t| (t.clone(), "curated_title"))
                .collect();
        }
        if self.kind != "person" {
            return Vec::new();
        }
        let mut out: Vec<String> = Vec::new();
        for r in &self.records {
            push_unique(&mut out, title_case(&r.name.to_lowercase()));
        }
        push_unique(&mut out, self.name.clone());
        out.into_iter().take(3).map(|t| (t, "name_title")).collect()
    }
}

// ---------------------------------------------------------------------------------------------
// Storage and sync

pub async fn init(db: &SqlitePool) -> Result<()> {
    sqlx::raw_sql(
        r#"
        CREATE TABLE IF NOT EXISTS sanction_records (list TEXT NOT NULL, reference TEXT NOT NULL, data TEXT NOT NULL, PRIMARY KEY(list, reference));
        CREATE TABLE IF NOT EXISTS actor_wikidata (actor_id TEXT PRIMARY KEY, checked_at TEXT NOT NULL, data TEXT, error TEXT);
        CREATE TABLE IF NOT EXISTS actor_mentions (actor_id TEXT NOT NULL, kind TEXT NOT NULL, ref TEXT NOT NULL, page INTEGER NOT NULL DEFAULT -1,
          title TEXT, url TEXT, at TEXT, snippet TEXT, hits INTEGER NOT NULL, PRIMARY KEY(actor_id, kind, ref, page));
        CREATE INDEX IF NOT EXISTS actor_mentions_ref ON actor_mentions(kind, ref);
        "#,
    )
    .execute(db)
    .await?;
    Ok(())
}
async fn load_records(db: &SqlitePool) -> Result<Vec<Record>> {
    let rows: Vec<String> =
        sqlx::query_scalar("SELECT data FROM sanction_records ORDER BY list, reference")
            .fetch_all(db)
            .await?;
    Ok(rows
        .iter()
        .filter_map(|r| serde_json::from_str(r).ok())
        .collect())
}
async fn refresh_list(db: &SqlitePool, client: &reqwest::Client, list: &List) -> Result<usize> {
    let bytes = reference::get_bytes(client, list.url, 96 << 20).await?;
    let mut records = (list.parse)(&bytes)?;
    // An empty parse means the format changed; keep the previous records.
    anyhow::ensure!(
        !records.is_empty(),
        "no DRC-regime records found; format may have changed"
    );
    records.sort_by(|a, b| a.reference.cmp(&b.reference));
    records.dedup_by(|a, b| a.reference == b.reference);
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM sanction_records WHERE list=?")
        .bind(list.id)
        .execute(&mut *tx)
        .await?;
    for r in &records {
        sqlx::query("INSERT INTO sanction_records(list,reference,data) VALUES(?,?,?)")
            .bind(list.id)
            .bind(&r.reference)
            .bind(serde_json::to_string(r)?)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(records.len())
}
async fn enrich(db: &SqlitePool, client: &reqwest::Client) -> Result<()> {
    let actors = build_actors(load_records(db).await?);
    for a in actors {
        let candidates = a.enwiki_candidates();
        if candidates.is_empty() {
            continue;
        }
        let row = sqlx::query("SELECT checked_at, error FROM actor_wikidata WHERE actor_id=?")
            .bind(&a.id)
            .fetch_optional(db)
            .await?;
        let fresh = row.as_ref().is_some_and(|r| {
            let age = chrono::DateTime::parse_from_rfc3339(&r.get::<String, _>("checked_at"))
                .map(|t| Utc::now().signed_duration_since(t).num_days())
                .unwrap_or(99);
            age < if r.get::<Option<String>, _>("error").is_some() {
                1
            } else {
                7
            }
        });
        if fresh {
            continue;
        }
        let mut found: Result<Option<Value>> = Ok(None);
        for (title, method) in &candidates {
            match reference::entity_by_title(client, title).await {
                Ok(Some(e)) => {
                    let accept = match (*method, a.kind.as_str()) {
                        ("curated_title", "person") => reference::is_human(&e),
                        ("curated_title", _) => true,
                        _ => reference::is_human(&e) && reference::regional(&e),
                    };
                    if accept {
                        found = reference::summarize(client, &e, method).await.map(Some);
                        break;
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    found = Err(e);
                    break;
                }
            }
        }
        let (data, error) = match found {
            Ok(d) => (d.map(|d| d.to_string()), None),
            Err(e) if reference::throttled(&e) => {
                tracing::warn!("Wikidata throttled actor enrichment; resuming next cycle");
                return Ok(());
            }
            Err(e) => (None, Some(e.to_string())),
        };
        sqlx::query("INSERT INTO actor_wikidata(actor_id,checked_at,data,error) VALUES(?,?,?,?) ON CONFLICT(actor_id) DO UPDATE SET checked_at=excluded.checked_at,data=coalesce(excluded.data,CASE WHEN excluded.error IS NULL THEN NULL ELSE data END),error=excluded.error")
            .bind(&a.id)
            .bind(Utc::now().to_rfc3339())
            .bind(data)
            .bind(error)
            .execute(db)
            .await?;
    }
    Ok(())
}
async fn scan_mentions(db: &SqlitePool, corpus: &[CorpusDoc]) -> Result<usize> {
    let actors = build_actors(load_records(db).await?);
    let mut rows = Vec::new();
    for a in &actors {
        let terms = a.terms();
        for c in corpus {
            let mut first: Option<(usize, usize)> = None;
            let mut hits = 0;
            for t in &terms {
                let found = c.doc.find_term(t);
                hits += found.len();
                if let Some(&i) = found.first()
                    && first.is_none_or(|(j, _)| i < j)
                {
                    first = Some((i, t.tokens.len()));
                }
            }
            if let Some((i, len)) = first {
                rows.push((a.id.clone(), c, c.doc.snippet(i, len, 160), hits));
            }
        }
    }
    let mut tx = db.begin().await?;
    sqlx::query("DELETE FROM actor_mentions")
        .execute(&mut *tx)
        .await?;
    for (id, c, snippet, hits) in &rows {
        sqlx::query("INSERT OR REPLACE INTO actor_mentions(actor_id,kind,ref,page,title,url,at,snippet,hits) VALUES(?,?,?,?,?,?,?,?,?)")
            .bind(id)
            .bind(c.kind)
            .bind(&c.id)
            .bind(c.page.unwrap_or(-1))
            .bind(&c.title)
            .bind(&c.url)
            .bind(&c.at)
            .bind(snippet)
            .bind(*hits as i64)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(rows.len())
}
pub async fn worker(db: SqlitePool) -> Result<()> {
    let client = reference::client()?;
    let mut last_scan: Option<std::time::Instant> = None;
    loop {
        let mut changed = false;
        if reference::sync_enabled() {
            for list in LISTS {
                let id = format!("sanctions:{}", list.id);
                if reference::due(&db, &id, 86400).await? {
                    let result = refresh_list(&db, &client, list).await;
                    if let Err(e) = &result {
                        tracing::warn!(list = list.id, error = %e, "sanctions refresh failed");
                    }
                    reference::record(&db, &id, &result, json!({"url":list.url})).await?;
                    changed |= result.is_ok();
                }
            }
            if let Err(e) = enrich(&db, &client).await {
                tracing::warn!(error = %e, "actor enrichment failed");
            }
        }
        if changed || last_scan.is_none_or(|t| t.elapsed().as_secs() >= 1800) {
            let corpus = reference::corpus(&db).await?;
            let result = scan_mentions(&db, &corpus).await;
            reference::record(&db, "mentions:actors", &result, json!({})).await?;
            last_scan = Some(std::time::Instant::now());
        }
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
            _ = reference::REFRESH.notified() => { last_scan = None; }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// API

async fn wikidata_map(db: &SqlitePool) -> Result<HashMap<String, Value>> {
    Ok(
        sqlx::query("SELECT actor_id, data FROM actor_wikidata WHERE data IS NOT NULL")
            .fetch_all(db)
            .await?
            .iter()
            .filter_map(|r| {
                Some((
                    r.get::<String, _>("actor_id"),
                    serde_json::from_str(&r.get::<String, _>("data")).ok()?,
                ))
            })
            .collect(),
    )
}
/// Profiles without a Wikidata/Commons photo, for web image search (key figures and the most
/// mentioned first).
pub async fn image_subjects(db: &SqlitePool) -> Result<Vec<crate::images::Subject>> {
    let actors = build_actors(load_records(db).await?);
    let wd = wikidata_map(db).await?;
    let mentions: HashMap<String, i64> =
        sqlx::query("SELECT actor_id, count(*) AS n FROM actor_mentions GROUP BY actor_id")
            .fetch_all(db)
            .await?
            .iter()
            .map(|r| (r.get("actor_id"), r.get("n")))
            .collect();
    Ok(actors
        .into_iter()
        .filter(|a| wd.get(&a.id).is_none_or(|w| w["image"].is_null()))
        .map(|a| {
            let groups: Vec<String> = a
                .groups
                .iter()
                .chain(&a.designation_groups)
                .filter_map(|g| groups().iter().find(|x| &x.id == g).map(|x| x.en.clone()))
                .collect();
            let first = |f: fn(&Record) -> &Vec<String>| {
                a.records
                    .iter()
                    .flat_map(f)
                    .next()
                    .cloned()
                    .unwrap_or_default()
            };
            let lists: BTreeSet<&str> = a.records.iter().map(|r| r.list.as_str()).collect();
            let context = format!(
                "{}. {}. Roles: {}. Affiliation: {}. Nationality: {}. Born: {}. Sanctioned by: {}.",
                if a.kind == "person" {
                    "Person"
                } else {
                    "Organization"
                },
                wd.get(&a.id)
                    .and_then(|w| w.pointer("/description/en"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                a.records
                    .iter()
                    .flat_map(|r| &r.titles)
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; "),
                groups.join(", "),
                first(|r| &r.nationality),
                first(|r| &r.dob),
                lists.into_iter().collect::<Vec<_>>().join(", "),
            );
            // Names the image or page may use: full name, name without "(ACRONYM)", aliases, terms.
            let bare = a
                .name
                .split('(')
                .next()
                .unwrap_or(&a.name)
                .trim()
                .to_string();
            let mut names: Vec<String> = vec![a.name.clone(), bare];
            names.extend(
                a.aliases
                    .iter()
                    .filter(|x| text::tokens(x).len() >= 2)
                    .cloned(),
            );
            names.extend(a.terms().into_iter().map(|t| t.tokens.join(" ")));
            names.dedup();
            crate::images::Subject {
                kind: "actor",
                priority: if a.curated.is_some() { 100_000 } else { 0 }
                    + mentions.get(&a.id).copied().unwrap_or(0) * 10
                    + i64::from(!a.records.is_empty()),
                person: a.kind == "person",
                id: a.id,
                name: a.name,
                context,
                names,
            }
        })
        .collect())
}
/// Image precedence: Wikidata/Commons, then a verified web-search image.
fn summary_json(
    a: &Actor,
    wd: Option<&Value>,
    web: Option<&Value>,
    counts: Option<&Value>,
) -> Value {
    let titles: Vec<&String> = a.records.iter().flat_map(|r| &r.titles).take(4).collect();
    json!({
        "id": a.id, "kind": a.kind, "name": a.name, "aliases": a.aliases, "curated": a.curated.is_some(),
        "groups": a.groups, "designation_groups": a.designation_groups,
        "sanctioned": !a.records.is_empty(),
        "sanctions": a.records.iter().map(|r| json!({"list":r.list,"reference":r.reference,"program":r.program,"listed_on":r.listed_on})).collect::<Vec<_>>(),
        "titles": titles,
        "description": wd.map(|w| w["description"].clone()),
        "image": wd.map(|w| w["image"].clone()).filter(|i| !i.is_null()).or_else(|| web.cloned()),
        "x_username": wd.and_then(|w| w["x_username"].as_str()),
        "mentions": counts.cloned().unwrap_or(json!({})),
    })
}
pub async fn list(State(app): State<App>) -> ApiResult<Json<Value>> {
    let actors = build_actors(load_records(&app.db).await?);
    let wd = wikidata_map(&app.db).await?;
    let web = crate::images::found(&app.db, "actor").await?;
    let week = (Utc::now() - chrono::Duration::days(7)).to_rfc3339();
    let mut counts: HashMap<String, serde_json::Map<String, Value>> = HashMap::new();
    for r in sqlx::query("SELECT actor_id, kind, count(*) AS n, sum(CASE WHEN at>=? THEN 1 ELSE 0 END) AS recent, max(at) AS latest FROM actor_mentions GROUP BY actor_id, kind")
        .bind(&week)
        .fetch_all(&app.db)
        .await?
    {
        let m = counts.entry(r.get("actor_id")).or_default();
        let kind: String = r.get("kind");
        let n: i64 = r.get("n");
        let recent: i64 = r.get("recent");
        m.insert(kind, json!(n));
        let total = m.get("total").and_then(Value::as_i64).unwrap_or(0) + n;
        m.insert("total".into(), json!(total));
        let rec = m.get("recent").and_then(Value::as_i64).unwrap_or(0) + recent;
        m.insert("recent".into(), json!(rec));
        let latest: Option<String> = r.get("latest");
        if latest.as_deref() > m.get("latest").and_then(Value::as_str) {
            m.insert("latest".into(), json!(latest));
        }
    }
    let mut index: HashMap<String, Vec<String>> = HashMap::new();
    for r in sqlx::query(
        "SELECT actor_id, kind, ref FROM actor_mentions WHERE kind IN ('report','post')",
    )
    .fetch_all(&app.db)
    .await?
    {
        index
            .entry(format!(
                "{}:{}",
                r.get::<String, _>("kind"),
                r.get::<String, _>("ref")
            ))
            .or_default()
            .push(r.get("actor_id"));
    }
    let items: Vec<Value> = actors
        .iter()
        .map(|a| {
            summary_json(
                a,
                wd.get(&a.id),
                web.get(&a.id),
                counts.get(&a.id).map(|m| Value::Object(m.clone())).as_ref(),
            )
        })
        .collect();
    let sync = reference::sync_rows(&app.db, "sanctions:").await?;
    let lists: Vec<Value> = LISTS
        .iter()
        .map(|l| {
            let s = sync
                .iter()
                .find(|s| s["id"] == format!("sanctions:{}", l.id));
            json!({"id":l.id,"label":l.label,"url":l.url,"license":l.license,"sync":s})
        })
        .collect();
    Ok(Json(json!({
        "items": items, "groups": groups(), "lists": lists, "index": index,
        "sync_enabled": reference::sync_enabled(),
        "notice": "Sanctions are reproduced from official lists with attribution. Profiles are compiled from Wikidata and cited text; a mention is not evidence of involvement. Name matches across lists and to Wikidata are automated and should be verified."
    })))
}
pub async fn detail(State(app): State<App>, Path(id): Path<String>) -> ApiResult<Json<Value>> {
    let actors = build_actors(load_records(&app.db).await?);
    let a = actors
        .iter()
        .find(|a| a.id == id)
        .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "actor not found".into()))?;
    let wd: Option<Value> =
        sqlx::query_scalar::<_, Option<String>>("SELECT data FROM actor_wikidata WHERE actor_id=?")
            .bind(&a.id)
            .fetch_optional(&app.db)
            .await?
            .flatten()
            .and_then(|d| serde_json::from_str(&d).ok());
    let mentions: Vec<Value> = sqlx::query("SELECT * FROM actor_mentions WHERE actor_id=? ORDER BY at DESC LIMIT 400")
        .bind(&a.id)
        .fetch_all(&app.db)
        .await?
        .iter()
        .map(|r| json!({"kind":r.get::<String,_>("kind"),"ref":r.get::<String,_>("ref"),"page":Some(r.get::<i64,_>("page")).filter(|p|*p>=0),"title":r.get::<Option<String>,_>("title"),"url":r.get::<Option<String>,_>("url"),"at":r.get::<Option<String>,_>("at"),"snippet":r.get::<Option<String>,_>("snippet"),"hits":r.get::<i64,_>("hits")}))
        .collect();
    let web = crate::images::found(&app.db, "actor").await?;
    let mut out = summary_json(a, wd.as_ref(), web.get(&a.id), None);
    out["sanctions"] = json!(a.records);
    out["wikidata"] = wd.unwrap_or(Value::Null);
    out["mentions"] = json!(mentions);
    out["terms"] = json!(
        a.terms()
            .iter()
            .map(|t| t.tokens.join(" "))
            .collect::<Vec<_>>()
    );
    out["lists"] = json!(
        LISTS
            .iter()
            .map(|l| json!({"id":l.id,"label":l.label,"license":l.license}))
            .collect::<Vec<_>>()
    );
    Ok(Json(out))
}
pub async fn refresh(State(app): State<App>) -> ApiResult<Json<Value>> {
    sqlx::query("UPDATE reference_sync SET checked_at=NULL WHERE id LIKE 'sanctions:%' OR id LIKE 'equipment:%'")
        .execute(&app.db)
        .await?;
    sqlx::query("DELETE FROM actor_wikidata WHERE error IS NOT NULL")
        .execute(&app.db)
        .await?;
    reference::REFRESH.notify_waiters();
    Ok(Json(
        json!({"queued":true,"sync_enabled":reference::sync_enabled()}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    const UN: &str = r#"<CONSOLIDATED_LIST><INDIVIDUALS><INDIVIDUAL><FIRST_NAME>SULTANI</FIRST_NAME><SECOND_NAME>MAKENGA</SECOND_NAME><UN_LIST_TYPE>DRC</UN_LIST_TYPE><REFERENCE_NUMBER>CDi.008</REFERENCE_NUMBER><LISTED_ON>2012-12-31</LISTED_ON><COMMENTS1>Military leader of the M23.</COMMENTS1><NATIONALITY><VALUE>Democratic Republic of the Congo</VALUE></NATIONALITY><INDIVIDUAL_ALIAS><ALIAS_NAME>Emmanuel Sultani Makenga</ALIAS_NAME></INDIVIDUAL_ALIAS><INDIVIDUAL_DATE_OF_BIRTH><DATE>1973-12-25</DATE></INDIVIDUAL_DATE_OF_BIRTH></INDIVIDUAL><INDIVIDUAL><FIRST_NAME>OTHER</FIRST_NAME><SECOND_NAME>PERSON</SECOND_NAME><UN_LIST_TYPE>Al-Qaida</UN_LIST_TYPE><REFERENCE_NUMBER>QDi.1</REFERENCE_NUMBER></INDIVIDUAL></INDIVIDUALS><ENTITIES><ENTITY><FIRST_NAME>M23</FIRST_NAME><UN_LIST_TYPE>DRC</UN_LIST_TYPE><REFERENCE_NUMBER>CDe.011</REFERENCE_NUMBER></ENTITY></ENTITIES></CONSOLIDATED_LIST>"#;
    const OFAC: &str = "15527,\"MAKENGA, Sultani\",\"individual\",\"DRCONGO\",\"Colonel\",-0- ,-0- ,-0- ,-0- ,-0- ,-0- ,\"DOB 25 Dec 1973; POB Rutshuru, Democratic Republic of the Congo; Passport X123 (Rwanda); a.k.a. 'MAKENGA, Emmanuel Sultani'; Colonel.\"\n15585,\"M23\",-0- ,\"DRCONGO\",-0- ,-0- ,-0- ,-0- ,-0- ,-0- ,-0- ,\"Website www.m23mars.org.\"\n1,\"CUBA CO\",-0- ,\"CUBA\",-0-,-0-,-0-,-0-,-0-,-0-,-0-,-0-\n";
    #[test]
    fn lists_parse_and_merge_without_identity_documents() {
        let un = parse_un(UN.as_bytes()).unwrap();
        assert_eq!(un.len(), 2);
        assert_eq!(un[0].dob, ["1973-12-25"]);
        let us = parse_ofac(OFAC.as_bytes()).unwrap();
        assert_eq!(us.len(), 2);
        assert_eq!(us[0].name, "Sultani MAKENGA");
        assert!(!serde_json::to_string(&us).unwrap().contains("X123"));
        let actors = build_actors([un, us].concat());
        let makenga = actors.iter().find(|a| a.id == "sultani-makenga").unwrap();
        assert_eq!(
            makenga.records.len(),
            2,
            "UN and OFAC records merge into the curated profile"
        );
        assert!(makenga.designation_groups.contains(&"m23-afc".to_string()));
        let m23 = actors.iter().find(|a| a.id == "m23").unwrap();
        assert_eq!(m23.records.len(), 2);
        let d = Doc::new("Le chef du M23, Sultani Makenga, et un membre m23 du comité.");
        let hits: usize = m23.terms().iter().map(|t| d.find_term(t).len()).sum();
        assert_eq!(hits, 1, "acronyms only match in capitals");
        assert!(makenga.terms().iter().any(|t| d.has_term(t)));
    }
    #[test]
    fn uk_fields_and_dates_parse() {
        let info = "(UK Sanctions List Ref):DRC0001. (UK Statement of Reasons):Leader of an armed group (M23). (Gender):Male";
        assert_eq!(
            uk_field(info, "UK Sanctions List Ref").as_deref(),
            Some("DRC0001.")
        );
        assert_eq!(
            uk_field(info, "UK Statement of Reasons").as_deref(),
            Some("Leader of an armed group (M23).")
        );
        assert_eq!(uk_date("00/00/1970").as_deref(), Some("1970"));
        assert_eq!(uk_date("25/12/1973").as_deref(), Some("1973-12-25"));
    }
}
