// Congo Signal dashboard — vanilla ES module, no build step.
// Every untrusted string (articles, posts, profiles, errors) passes through esc() before it reaches innerHTML.

const $ = (s, root = document) => root.querySelector(s);
const $$ = (s, root = document) => [...root.querySelectorAll(s)];
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const store = {
  get: k => { try { return localStorage.getItem(k) } catch { return null } },
  set: (k, v) => { try { localStorage.setItem(k, v) } catch {} },
};

const paths = {
  grid: 'M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z',
  map: 'm3 6 6-3 6 3 6-3v15l-6 3-6-3-6 3Z M9 3v15 M15 6v15',
  bookmark: 'M6 3h12v18l-6-4-6 4Z',
  layers: 'm12 3 10 5-10 5L2 8Z M2 12l10 5 10-5 M2 16l10 5 10-5',
  search: 'M21 21l-5-5 M18 10a8 8 0 1 1-16 0 8 8 0 0 1 16 0',
  arrow: 'M5 12h14 M13 6l6 6-6 6',
  download: 'M12 3v12 m-5-5 5 5 5-5 M4 16v5h16v-5',
  refresh: 'M20 10a8 8 0 1 0-2 8 M20 3v7h-7',
  globe: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0 M3 12h18 M12 3c-6 6-6 12 0 18 6-6 6-12 0-18',
  spark: 'm12 2 3 7 7 3-7 3-3 7-3-7-7-3 7-3Z',
  activity: 'M2 12h5l3-8 4 16 3-8h5',
  clock: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0 M12 7v5l3 2',
  pin: 'M19 10c0 5-7 11-7 11S5 15 5 10a7 7 0 0 1 14 0 M14 10a2 2 0 1 1-4 0 2 2 0 0 1 4 0',
  shield: 'm12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6Z M8 12l3 3 5-6',
  book: 'M3 4h6c2 0 3 1 3 2 0-1 1-2 3-2h6v16h-6c-2 0-3 1-3 1s-1-1-3-1H3Z M12 6v15',
  settings: 'M4 21v-7 M4 10V3 M12 21v-9 M12 8V3 M20 21v-5 M20 12V3 M1 14h6 M9 8h6 M17 16h6',
  close: 'm6 6 12 12 M6 18 18 6',
  external: 'M14 3h7v7 M21 3 10 14 M10 3H3v18h18v-7',
  check: 'm4 12 5 5L20 6',
  radio: 'M4.9 19.1a10 10 0 0 1 0-14.2 M19.1 4.9a10 10 0 0 1 0 14.2 M7.8 16.2a6 6 0 0 1 0-8.4 M16.2 7.8a6 6 0 0 1 0 8.4 M14 12a2 2 0 1 1-4 0 2 2 0 0 1 4 0',
  user: 'M20 21a8 8 0 0 0-16 0 M16 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0',
  users: 'M17 21a6 6 0 0 0-12 0 M15 7a4 4 0 1 1-8 0 4 4 0 0 1 8 0 M22 21a6 6 0 0 0-4-5.6 M16 3.2a4 4 0 0 1 0 7.6',
  hash: 'M4 9h16 M4 15h16 M10 3 8 21 M16 3l-2 18',
  at: 'M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0 M16 8v5a3 3 0 0 0 6 0v-1a10 10 0 1 0-4 8',
  link: 'M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.7 1.7 M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7',
  copy: 'M9 9h12v12H9z M5 15H3V3h12v2',
  sun: 'M12 17a5 5 0 1 0 0-10 5 5 0 0 0 0 10 M12 1v2 M12 21v2 M4.2 4.2l1.4 1.4 M18.4 18.4l1.4 1.4 M1 12h2 M21 12h2 M4.2 19.8l1.4-1.4 M18.4 5.6l1.4-1.4',
  moon: 'M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8Z',
  reply: 'M9 14 4 9l5-5 M4 9h11a5 5 0 0 1 5 5v6',
  repost: 'm17 2 4 4-4 4 M3 11V9a3 3 0 0 1 3-3h15 M7 22l-4-4 4-4 M21 13v2a3 3 0 0 1-3 3H3',
  heart: 'M20.8 4.6a5.5 5.5 0 0 0-7.8 0L12 5.7l-1-1.1a5.5 5.5 0 0 0-7.8 7.8L12 21l8.8-8.6a5.5 5.5 0 0 0 0-7.8Z',
  quote: 'M3 21c3 0 7-1 7-8V5H3v8h4 M14 21c3 0 7-1 7-8V5h-7v8h4',
  eye: 'M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12Z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0',
  trend: 'm22 7-8.5 8.5-5-5L2 17 M16 7h6v6',
  history: 'M3 12a9 9 0 1 0 3-6.7L3 8 M3 3v5h5 M12 7v5l4 2',
  alert: 'M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z M12 9v4 M12 17h.01',
  calendar: 'M3 5h18v16H3z M3 10h18 M8 3v4 M16 3v4',
  flag: 'M4 22V4 M4 4h13l-2 4 2 4H4',
  target: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0 M17 12a5 5 0 1 1-10 0 5 5 0 0 1 10 0 M12 1v4 M12 19v4 M1 12h4 M19 12h4',
};
const icon = n => `<svg class="icon" viewBox="0 0 24 24" aria-hidden="true"><path d="${paths[n] || paths.grid}"/></svg>`;

// ---------------------------------------------------------------------------------------------
// State

const VIEWS = ['briefing', 'situation', 'reports', 'social', 'sources', 'actors', 'arms', 'saved', 'method', 'agreements', 'costs', 'team'];
const LEGACY_VIEWS = { map: 'situation', overview: 'briefing' };
const viewFromHash = () => { const h = location.hash.slice(1); if (h.startsWith('account/')) return 'social'; if (h.startsWith('actor/')) return 'actors'; if (h.startsWith('weapon/')) return 'arms'; const v = LEGACY_VIEWS[h] || h; return VIEWS.includes(v) ? v : 'briefing' };
const hashAccount = () => location.hash.startsWith('#account/') ? decodeURIComponent(location.hash.slice(9)) : '';
const hashPart = prefix => location.hash.startsWith(`#${prefix}/`) ? decodeURIComponent(location.hash.slice(prefix.length + 2)) : '';
function openFromHash() { if (hashAccount()) openDossier(hashAccount()); else if (hashPart('actor')) openActor(hashPart('actor')); else if (hashPart('weapon')) openWeapon(hashPart('weapon')) }
const state = {
  lang: store.get('congo-lang') || 'en', theme: document.documentElement.dataset.theme || 'dark', view: viewFromHash(),
  loaded: false, error: '', items: [], total: 0, limit: 200, status: {}, config: null, ev: null, sources: null, social: null, posts: null, accounts: null, updated: null, geometry: null,
  q: '', category: 'all', place: '', sort: 'priority', publisher: '', includeRegional: false, visible: 20,
  layers: ['security', 'politics', 'health', 'economy', 'other'], windowDays: 0, selectedDay: '', highOnly: false, basemap: 'base', tilt: false,
  sf: { topic: '', lang: '', kind: '', q: '', sort: 'latest', accountQ: '', visible: 40 },
  af: { kind: 'all', group: '', q: '', sort: 'mentions', visible: 48 }, wf: { category: '', q: '', sort: 'mentions' },
  actors: null, arms: null, actor: null, weapon: null, hud: {}, enter: false,
  agreements:null, agDetail:null, agSearch:'', agCategory:'', agTerm:'', agDrafts:false,
  evQuery: '', evRegional: false,
  modal: null, detail: null, detailId: null, detailTab: 'brief', dossier: null, palette: null,
};
const L = (en, fr) => state.lang === 'fr' ? fr : en;
const locale = () => state.lang === 'fr' ? 'fr-FR' : 'en-GB';
const nf = n => Number(n || 0).toLocaleString(locale());
const compact = n => new Intl.NumberFormat(locale(), { notation: 'compact', maximumFractionDigits: 1 }).format(Number(n || 0));
const norm = s => String(s ?? '').normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();

function date(s, short = false) {
  if (!s) return '—';
  return new Date(s).toLocaleString(locale(), { day: '2-digit', month: 'short', ...(short ? {} : { year: 'numeric' }), hour: '2-digit', minute: '2-digit', timeZone: 'Africa/Kinshasa' });
}
function dayLabel(d) { return new Date(d + 'T12:00:00Z').toLocaleDateString(locale(), { day: 'numeric', month: 'short', timeZone: 'UTC' }) }
function age(s) {
  if (!s) return '—';
  const m = Math.max(0, Math.floor((Date.now() - Date.parse(s)) / 60000));
  if (m < 1) return L('now', 'maint.');
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  return h < 24 ? `${h} h` : `${Math.floor(h / 24)} ${L('d', 'j')}`;
}
// Kinshasa is UTC+1 year-round.
const kinDay = s => { const n = Date.parse(s); return Number.isFinite(n) ? new Date(n + 3600000).toISOString().slice(0, 10) : '' };
const kinHour = s => { const n = Date.parse(s); return Number.isFinite(n) ? new Date(n + 3600000).getUTCHours() : null };

// ---------------------------------------------------------------------------------------------
// Domain helpers

const CATS = { all: null, security: ['Sécurité'], politics: ['Politique', 'Justice'], health: ['Santé'], economy: ['Économie'], other: ['Société', 'Culture', 'Femme', 'Afrique', 'Sport', 'Opinion', 'Caricature', "Offre d'emploi", "Appel d'offre"] };
const LAYERS = ['security', 'politics', 'health', 'economy', 'other'];
const layerName = k => ({ all: L('All', 'Tous'), security: L('Security', 'Sécurité'), politics: L('Politics', 'Politique'), health: L('Health', 'Santé'), economy: L('Economy', 'Économie'), other: L('Other', 'Autres') })[k];
const layerColor = k => `var(--series-${LAYERS.indexOf(k) + 1})`;
const reportLayer = a => LAYERS.find(k => CATS[k].includes(a.category)) || 'other';
const categoryName = c => state.lang === 'en' ? ({ 'Sécurité': 'Security', 'Santé': 'Health', 'Politique': 'Politics', 'Société': 'Society', 'Économie': 'Economy', 'Femme': 'Women', 'Culture': 'Culture', 'Afrique': 'Africa', 'Sport': 'Sport', 'Opinion': 'Opinion', 'Caricature': 'Cartoon', 'Justice': 'Justice', "Offre d'emploi": 'Job notice', "Appel d'offre": 'Tender notice' }[c] || c) : c;
const brief = a => a.brief?.status === 'completed' ? a.brief.data : null;
const title = a => state.lang === 'en' && brief(a)?.title_en ? brief(a).title_en : a.title;
const summary = a => brief(a)?.[`summary_${state.lang}`] || a.description || '';
const bandLabel = b => ({ high: L('High', 'Élevée'), medium: L('Medium', 'Moyenne'), low: L('Low', 'Faible') })[b] || b;

const topicsList = () => state.social?.items?.map(i => i.topic) || [];
const topicIndex = id => topicsList().findIndex(t => t.id === id);
const topicColor = id => `var(--series-${topicIndex(id) + 1 || 5})`;
const topicName = id => topicsList().find(t => t.id === id)?.[state.lang] || id;
const kindName = k => ({ post: L('Post', 'Publication'), reply: L('Reply', 'Réponse'), quote: L('Quote', 'Citation'), repost: L('Repost', 'Republication') })[k] || k;
const engagement = p => ['like_count', 'retweet_count', 'reply_count', 'quote_count'].reduce((n, k) => n + (p.metrics?.[k] || 0), 0);
const hue = s => [...String(s)].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7);
const initials = (name, handle) => (String(name || handle).normalize('NFKC').replace(/[^\p{L}\p{N} ]/gu, '').trim().split(/\s+/).slice(0, 2).map(w => w[0]).join('') || handle?.[0] || '?');
const avatar = (name, handle, size = '') => `<span class="avatar ${size}" style="--h:${hue(handle)}" aria-hidden="true">${esc(initials(name, handle))}</span>`;
const validHandle = h => /^[A-Za-z0-9_]{1,15}$/.test(h);

function countBy(list, key) {
  const m = new Map();
  for (const x of list) for (const k of [].concat(key(x) ?? [])) if (k !== '' && k != null) m.set(k, (m.get(k) || 0) + 1);
  return [...m].sort((a, b) => b[1] - a[1]);
}
function placesList(items = state.items) {
  const m = new Map();
  for (const a of items) for (const p of a.places) m.set(p.name, { ...p, count: (m.get(p.name)?.count || 0) + 1 });
  return [...m.values()].sort((a, b) => b.count - a.count);
}
function matchesQuery(a, q) {
  return !q || norm([a.title, a.description, a.search_text, JSON.stringify(brief(a)), ...a.places.map(p => p.name + ' ' + p.province)].join(' ')).includes(norm(q));
}
const bySort = (a, b) => state.sort === 'latest' ? String(b.published_at).localeCompare(String(a.published_at)) : b.rank.score - a.rank.score;
function libraryItems() {
  return state.items.filter(a => (!state.publisher || a.source === state.publisher)
    && (state.view !== 'saved' || a.analyst.saved)
    && (!state.place || a.places.some(p => p.name === state.place))
    && (state.category === 'all' || CATS[state.category].includes(a.category) || (state.category === 'other' && !a.category))
    && matchesQuery(a, state.q)).sort(bySort);
}
function inSituation(a, ignoreDay = false) {
  const ts = Date.parse(a.published_at);
  return (!state.publisher || a.source === state.publisher) && state.layers.includes(reportLayer(a)) && (!state.highOnly || a.rank.band === 'high')
    && (!state.windowDays || (Number.isFinite(ts) && ts >= Date.now() - state.windowDays * 86400000 && ts <= Date.now()))
    && (ignoreDay || !state.selectedDay || kinDay(a.published_at) === state.selectedDay);
}
function situationItems(ignorePlace = false, ignoreDay = false) {
  return state.items.filter(a => inSituation(a, ignoreDay) && (ignorePlace || !state.place || a.places.some(p => p.name === state.place)) && matchesQuery(a, state.q)).sort(bySort);
}
function filteredPosts() {
  const f = state.sf, q = norm(f.q.trim());
  const rows = (state.posts?.items || []).filter(p => (!f.topic || p.topics?.some(t => t.topic === f.topic)) && (!f.lang || p.lang === f.lang) && (!f.kind || p.kind === f.kind)
    && (!q || norm(`${p.text} @${p.author_handle} ${p.author_name}`).includes(q)));
  return f.sort === 'engagement' ? rows.sort((a, b) => engagement(b) - engagement(a)) : rows;
}
const activeTasks = () => state.status.tasks?.filter(x => ['pending', 'running'].includes(x.status)).reduce((n, x) => n + x.count, 0) || 0;

// ---------------------------------------------------------------------------------------------
// API

async function api(path, data) {
  const token = sessionStorage.getItem('congo-token');
  const r = await fetch(path, { method: data ? 'POST' : 'GET', headers: { ...(data ? { 'Content-Type': 'application/json', ...(state.auth?.csrf ? {'X-CSRF-Token':state.auth.csrf} : {}) } : {}), ...(token ? { Authorization: `Bearer ${token}` } : {}) }, ...(data ? { body: JSON.stringify(data) } : {}) });
  if (!r.ok) {
    let e; try { e = await r.json() } catch {}
    const err = Error(r.status === 401 ? L('This API requires a token. Open connection settings.', 'Cette API nécessite un jeton. Ouvrez les paramètres de connexion.') : (e?.error || `${L('Unable to connect to the API.', 'Impossible de joindre l’API.')} (${r.status})`));
    err.status = r.status;
    throw err;
  }
  return r.json();
}
async function load() {
  try {
    state.authConfig=await fetch('/auth/config').then(r=>r.json());
    state.auth=await api('/v1/auth/me');state.loginRequired=false;
  } catch(e) {if(state.authConfig?.mode==='oidc'){state.loginRequired=true;state.auth=null;state.loaded=true;closeOverlay();render();return}}
  try {
    const [data, status, config, ev, sources, social, posts, accounts, actors, arms, feeds, agreements, costs, team] = await Promise.all([
      api(`/v1/intelligence?limit=${state.limit}${state.includeRegional ? '&scope=regional' : ''}`),
      api('/v1/intelligence/status'),
      state.config ? state.config : api('/v1/config'),
      api('/v1/evidence-feed?scope=regional').catch(() => null),
      api('/v1/sources').catch(() => null),
      api('/v1/social').catch(() => null),
      api('/v1/social/posts?limit=500').catch(() => null),
      api('/v1/social/accounts?limit=200').catch(() => null),
      api('/v1/actors').catch(() => null),
      api('/v1/arms').catch(() => null),
      api('/v1/feeds').catch(() => null),
      api('/v1/agreements').catch(() => null),
      api('/v1/costs').catch(() => null),
      api('/v1/team').catch(() => null),
    ]);
    if (state.agDetail?.assessment?.status === 'running') state.agDetail = await api('/v1/agreements/'+state.agDetail.id);
    Object.assign(state, { items: data.items, total: data.total, updated: data.generated_at, status, config, ev, sources, social, posts, accounts, actors, arms, feeds, agreements, costs, team, error: '' });
  } catch (e) { state.error = e.message }
  state.loaded = true;
  render();
}

// ---------------------------------------------------------------------------------------------
// Charts (dataviz: thin marks, 2px gaps, rounded data-ends on the baseline, hover tooltips)

function bars(values, color = 'var(--series-1)', cls = '') {
  const max = Math.max(1, ...values.map(v => v.value));
  return `<div class="bars ${cls}" style="--c:${color}" role="img" aria-label="${esc(values.map(v => v.tip).join('; '))}">${values.map(v => `<i class="${v.value ? '' : 'zero'}" style="height:${v.value ? Math.max(5, v.value / max * 100) : 3}%" data-tip="${esc(v.tip)}"></i>`).join('')}</div>`;
}
function spark(values, color = 'var(--series-1)', w = 84, h = 30) {
  if (values.length < 2) return '';
  const max = Math.max(1, ...values), step = w / (values.length - 1);
  const pts = values.map((v, i) => [i * step, h - 2 - (v / max) * (h - 4)]);
  const line = pts.map((p, i) => `${i ? 'L' : 'M'}${p[0].toFixed(1)},${p[1].toFixed(1)}`).join('');
  return `<svg class="chart spark" viewBox="0 0 ${w} ${h}" preserveAspectRatio="none" aria-hidden="true"><path class="area" fill="${color}" d="${line}L${w},${h}L0,${h}Z"/><path class="line" stroke="${color}" d="${line}"/></svg>`;
}
function areaChart(points, color, h = 56) {
  if (points.length < 2) return `<div class="muted" style="font-size:12px">${L('History builds as snapshots accumulate.', 'L’historique se constitue au fil des instantanés.')}</div>`;
  const w = 300, max = Math.max(1, ...points.map(p => p.value)), step = w / (points.length - 1);
  const xy = points.map((p, i) => [i * step, h - 2 - (p.value / max) * (h - 6)]);
  const line = xy.map((p, i) => `${i ? 'L' : 'M'}${p[0].toFixed(1)},${p[1].toFixed(1)}`).join('');
  const hits = points.map((p, i) => `<rect class="hit" x="${Math.max(0, i * step - step / 2)}" y="0" width="${step}" height="${h}" data-tip="${esc(p.tip)}"/>`).join('');
  return `<svg class="chart" viewBox="0 0 ${w} ${h}" preserveAspectRatio="none" style="height:${h}px" role="img" aria-label="${esc(L('Hourly matching posts, last 7 days', 'Publications horaires, 7 derniers jours'))}"><line class="grid" x1="0" x2="${w}" y1="${h - 0.5}" y2="${h - 0.5}"/><path class="area" fill="${color}" d="${line}L${w},${h}L0,${h}Z"/><path class="line" stroke="${color}" d="${line}" vector-effect="non-scaling-stroke"/>${hits}</svg>`;
}
const meter = (value, max, color = 'var(--accent)') => `<span class="meter" style="--c:${color}"><i style="width:${Math.min(100, max ? value / max * 100 : 0)}%"></i></span>`;
function topicMix(counts) {
  const entries = Object.entries(counts || {}).sort((a, b) => topicIndex(a[0]) - topicIndex(b[0]));
  const total = entries.reduce((n, [, v]) => n + v, 0) || 1;
  return `<div class="mix" role="img" aria-label="${esc(entries.map(([k, v]) => `${topicName(k)}: ${v}`).join(', '))}">${entries.map(([k, v]) => `<i style="flex:${v / total};background:${topicColor(k)}" data-tip="${esc(`${topicName(k)}: ${v}`)}"></i>`).join('')}</div>`;
}

// ---------------------------------------------------------------------------------------------
// Shell

const NAV = () => [
  ['briefing', 'grid', L('Briefing', 'Synthèse'), state.items.filter(a => a.rank.band === 'high').length, true],
  ['situation', 'map', L('Situation room', 'Vue de situation')],
  ['reports', 'layers', L('Reports', 'Articles'), state.total],
  ['social', 'radio', L('Social intelligence', 'Veille sociale'), state.social?.archive?.posts],
  ['sources', 'book', L('Primary sources', 'Sources primaires'), state.ev?.items?.length],
  ['actors', 'flag', L('Key figures & sanctions', 'Acteurs et sanctions'), state.actors?.items?.filter(a => a.sanctioned).length],
  ['arms', 'target', L('Arms & equipment', 'Armes et équipements'), state.arms?.items?.filter(i => i.mentions?.total).length],
  ['saved', 'bookmark', L('Saved', 'Sauvegardés'), state.items.filter(a => a.analyst.saved).length],
  ['agreements', 'history', L('Agreements', 'Accords'), state.agreements?.items?.length],
  ['team', 'shield', L('Team & access', 'Équipe et accès')],
  ['costs', 'layers', L('Costs & budgets', 'Coûts et budgets')],
  ['method', 'shield', L('Methodology', 'Méthodologie')],
];
const viewLabel = v => NAV().find(n => n[0] === v)?.[2] || v;
function feedStatuses() {
  const lastFetch = state.items.filter(a => a.record_type !== 'evidence' && !a.coverage).map(a => a.fetched_at).filter(Boolean).sort().at(-1);
  const s = state.social, src = state.sources?.items?.filter(x => !x.id.startsWith('discovered-')) || [];
  const failed = src.filter(x => x.status === 'failed').length;
  return [
    [state.error ? 'bad' : 'ok live', L('News collector', 'Collecteur d’articles'), lastFetch ? age(lastFetch) : '—'],
    ...(state.feeds?.items || []).map(f => [!f.sync ? 'warn' : f.sync.status === 'ok' ? 'ok' : 'bad', f.name, f.sync?.checked_at ? age(f.sync.checked_at) : state.feeds.poll_seconds ? L('pending', 'en attente') : L('off', 'arrêt')]),
    [!s ? 'bad' : s.enabled && s.configured ? (s.items.some(i => i.error) ? 'warn' : 'ok') : 'warn', 'X / Twitter', !s ? '—' : s.enabled && s.configured ? `${s.requests_today}/${s.daily_request_limit}` : L('paused', 'pause')],
    [state.status.available ? 'ok' : 'warn', L('AI analyst', 'Analyste IA'), state.status.provider || '—'],
    [failed ? 'warn' : 'ok', L('Documents', 'Documents'), src.length ? `${src.length - failed}/${src.length}` : '—'],
  ];
}
const VIEW_CODES = { briefing: 'BRF', situation: 'SIT', reports: 'RPT', social: 'SOC', sources: 'SRC', actors: 'ACT', arms: 'ARM', saved: 'SAV', method: 'MTH' };
/// Top-ranked recent reports for the scrolling priority wire.
function wireItems() {
  const cutoff = Date.now() - 96 * 3600000;
  return [...state.items].filter(a => Date.parse(a.published_at) >= cutoff).sort((a, b) => b.rank.score - a.rank.score).slice(0, 14);
}
function wire() {
  const items = wireItems();
  if (!items.length) return '';
  const row = items.map(a => `<button class="wire-item" data-act="report" data-arg="${esc(a.id)}"><b class="wire-score ${a.rank.band}">${a.rank.score}</b><span>${esc(title(a))}</span><time>${age(a.published_at)}</time></button>`).join('<i class="wire-sep" aria-hidden="true">◆</i>');
  return `<div class="wire" aria-label="${L('Priority wire', 'Fil prioritaire')}"><div class="wire-label"><i class="dot bad live"></i>${L('PRIORITY WIRE', 'FIL PRIORITAIRE')}</div><div class="wire-track"><div class="wire-run">${row}<i class="wire-sep" aria-hidden="true">◆</i>${row}</div></div></div>`;
}
function shell(content) {
  const nav = NAV().filter(([v])=>v!=='costs'||state.auth?.user?.role==='owner');
  return `<div class="app"><aside class="rail" aria-label="${L('Main navigation', 'Navigation principale')}">
    <a class="brand" href="#briefing" data-act="nav" data-arg="briefing"><span class="brand-mark">${icon('globe')}</span><div><strong>CONGO SIGNAL</strong><small>${L('DRC situational awareness', 'Veille de situation · RDC')}</small></div></a>
    <nav>${nav.map(([id, ic, label, count, hot], i) => `${i === 4 ? `<div class="nav-label">${L('Research', 'Recherche')}</div>` : i === 0 ? `<div class="nav-label">${L('Monitor', 'Veille')}</div>` : ''}<button class="nav-item ${state.view === id ? 'active' : ''}" data-act="nav" data-arg="${id}" ${state.view === id ? 'aria-current="page"' : ''} title="${esc(label)} · ${i + 1}">${icon(ic)}<span>${esc(label)}</span>${count ? `<b class="count ${hot ? 'hot' : ''}">${compact(count)}</b>` : `<kbd class="nav-key">${i + 1}</kbd>`}</button>`).join('')}</nav>
    <div class="rail-bottom"><p>${esc(state.auth?.user?.email||'')} · ${esc(state.auth?.user?.role||'')}</p>${state.auth?.mode==='oidc'?`<button class="btn" data-act="logout">${L('Sign out','Déconnexion')}</button>`:''}<div class="nav-label" style="padding:4px 4px 2px">${L('Collection', 'Collecte')}</div>${feedStatuses().map(([c, label, v]) => `<div class="feed-status"><i class="dot ${c}"></i>${esc(label)}<small>${esc(v)}</small></div>`).join('')}</div>
  </aside><div class="workspace"><div class="chrome">
    <div class="classbar"><span>${L('UNCLASSIFIED // OPEN SOURCE', 'NON CLASSIFIÉ // SOURCES OUVERTES')}</span><span class="classbar-mid">CONGO SIGNAL · ${L('DRC SITUATIONAL AWARENESS', 'VEILLE DE SITUATION RDC')}</span><span>${L('ATTRIBUTED · UNVERIFIED UNLESS STATED', 'ATTRIBUÉ · NON VÉRIFIÉ SAUF MENTION')}</span></div>
    <header class="topbar">
      <div class="crumb"><span class="crumb-code">${VIEW_CODES[state.view] || 'OPS'}</span><strong>${esc(viewLabel(state.view))}</strong></div>
      <button class="omni" data-act="palette" aria-label="${L('Search everything', 'Tout rechercher')}">${icon('search')}<span>${L('Search reports, places, actors, weapons…', 'Rechercher articles, lieux, acteurs, armes…')}</span><kbd>⌘K</kbd></button>
      <div class="clocks" aria-label="${L('Current time', 'Heure actuelle')}"><span><small>UTC</small><b id="clock-utc">${clockText(0)}</b></span><span><small>KIN</small><b id="clock-kin">${clockText(1)}</b></span></div>
      <span class="live-pill ${state.error ? 'down' : ''}"><i class="dot ${state.error ? 'bad' : 'ok live'}"></i>${state.error ? L('OFFLINE', 'HORS LIGNE') : 'LIVE'}</span>
      <div class="seg" role="group" aria-label="Language"><button data-act="lang" data-arg="en" class="${state.lang === 'en' ? 'active' : ''}">EN</button><button data-act="lang" data-arg="fr" class="${state.lang === 'fr' ? 'active' : ''}">FR</button></div>
      <button class="iconbtn" data-act="theme" aria-label="${L('Toggle theme', 'Changer de thème')}" title="${L('Toggle theme', 'Changer de thème')}">${icon(state.theme === 'light' ? 'moon' : 'sun')}</button>
      <button class="iconbtn" data-act="settings" aria-label="${L('Connection settings', 'Paramètres de connexion')}">${icon('settings')}</button>
    </header>${state.view === 'situation' ? '' : wire()}</div>
    <main class="view view-${state.view} ${state.enter ? 'enter' : ''}">${state.error && state.view !== 'situation' ? `<div class="errorbar">${icon('alert')}${esc(state.error)} <button class="linkbtn" data-act="settings">${L('Connection settings', 'Paramètres de connexion')}</button></div>` : ''}${content}</main></div></div>`;
}
/// HH:MM:SS at UTC + offset hours (Kinshasa is UTC+1 year-round).
function clockText(offset) { return new Date(Date.now() + offset * 3600000).toISOString().slice(11, 19) }
setInterval(() => { const u = $('#clock-utc'), k = $('#clock-kin'); if (u) u.textContent = clockText(0); if (k) k.textContent = clockText(1) }, 1000);
const pageHead = (eyebrow, h1, p, actions = '') => `<section class="page-head"><div><div class="eyebrow">${eyebrow}</div><h1>${h1}</h1>${p ? `<p>${p}</p>` : ''}</div><div class="actions">${actions}</div></section>`;
const btnExport = () => `<button class="btn" data-act="export">${icon('download')}${L('Export JSON', 'Exporter JSON')}</button>`;
const btnCollect = () => `<button class="btn" data-act="collect">${icon('refresh')}${L('Collect latest', 'Actualiser la collecte')}</button>`;
const btnBatch = () => `<button class="btn primary" data-act="batch">${icon('spark')}${L('Analyze top 5', 'Analyser les 5 premiers')}</button>`;

function render() {
  if (!state.loaded) return;
  if(state.loginRequired){mapHost?.remove();$('#app').innerHTML=`<main class="panel" style="max-width:560px;margin:12vh auto;padding:32px"><h1>Congo Signal</h1><p>${L('Sign in with your invited email to access your workspace.','Connectez-vous avec votre adresse invitée pour accéder à votre espace.')}</p><a class="btn primary" href="/auth/login">${L('Sign in','Se connecter')}</a></main>`;return}
  const active = document.activeElement;
  const keep = active?.id && !active.closest('#overlay') ? { id: active.id, start: active.selectionStart, end: active.selectionEnd } : null;
  const scrolls = $$('#app [data-scroll]').map(el => [el.dataset.scroll, el.scrollTop]);
  document.documentElement.lang = state.lang;
  mapHost?.remove();
  const views = { briefing: briefingView, situation: situationView, reports: reportsView, saved: reportsView, social: socialView, sources: sourcesView, actors: actorsView, arms: armsView, method: methodView, agreements: agreementsView, costs: costsView, team: teamView };
  $('#app').innerHTML = shell(views[state.view]());
  for (const [k, top] of scrolls) { const el = $(`#app [data-scroll="${k}"]`); if (el) el.scrollTop = top }
  if (keep) { const el = document.getElementById(keep.id); if (el) { el.focus({ preventScroll: true }); try { el.setSelectionRange(keep.start, keep.end) } catch {} } }
  mountMap();
  state.enter = false;
}
function go(view) {
  if (!VIEWS.includes(view)) return;
  state.view = view; state.visible = 20; state.enter = true;
  if (location.hash !== '#' + view) history.pushState(null, '', '#' + view);
  render(); window.scrollTo(0, 0);
}

// ---------------------------------------------------------------------------------------------
// Shared fragments

function reportRow(a, compactRow = false) {
  const places = a.places.map(p => p.name);
  const tags = [
    brief(a) ? `<span class="badge accent">${icon('spark')}${L('AI brief', 'Synthèse IA')}</span>` : ['pending', 'running'].includes(a.brief?.status) ? `<span class="badge">${L('AI working', 'IA en cours')}</span>` : '',
    a.analyst?.review === 'follow_up' ? `<span class="badge medium">${L('Follow-up', 'À approfondir')}</span>` : a.analyst?.review === 'reviewed' ? `<span class="badge good">${L('Reviewed', 'Examiné')}</span>` : '',
    a.record_type === 'evidence' ? `<span class="badge">${L('Primary document', 'Document primaire')}</span>` : '',
    a.coverage === 'feed_summary' ? `<span class="badge" data-tip="${esc(L('Headline and lede from the publisher’s official feed; full article on the publisher’s site.', 'Titre et chapeau issus du flux officiel ; article complet sur le site de l’éditeur.'))}">${L('Feed summary', 'Résumé du flux')}</span>` : '',
    a.relevance?.scope === 'regional' ? `<span class="badge">${L('Regional context', 'Contexte régional')}</span>` : '',
    actorChips(namedActors('report', a.id).sort((x, y) => y.sanctioned - x.sanctioned), 3),
  ].join('');
  return `<div class="report" role="button" tabindex="0" data-act="report" data-arg="${esc(a.id)}">
    <div class="score ${a.rank.band}" style="--v:${a.rank.score}" data-tip="${esc(`${L('Priority', 'Priorité')} ${a.rank.score}/100 · ${bandLabel(a.rank.band)}`)}"><span>${a.rank.score}</span></div>
    <div><div class="report-kicker"><span class="tag"><i class="swatch" style="background:${layerColor(reportLayer(a))}"></i>${esc(categoryName(a.category) || (a.record_type === 'evidence' ? L('Document', 'Document') : L('Other', 'Autres')))}</span>${compactRow ? '' : `<span class="sep">·</span><span>${esc(a.source)}</span>`}${places.length ? `<span class="sep">·</span><span>${icon('pin')} ${esc(places.slice(0, 3).join(', '))}</span>` : ''}<span class="sep">·</span><time title="${esc(date(a.published_at))}">${age(a.published_at)}</time></div>
    <h3>${esc(title(a))}</h3>${compactRow ? '' : `<p>${esc(summary(a))}</p>${tags ? `<div class="report-tags">${tags}</div>` : ''}`}</div>
    ${compactRow || a.record_type === 'evidence' ? '' : `<button class="save ${a.analyst.saved ? 'saved' : ''}" data-act="save" data-arg="${esc(a.id)}" aria-label="${a.analyst.saved ? L('Unsave report', 'Retirer des sauvegardes') : L('Save report', 'Sauvegarder')}" title="${a.analyst.saved ? L('Unsave report', 'Retirer des sauvegardes') : L('Save report', 'Sauvegarder')}">${icon('bookmark')}</button>`}
  </div>`;
}
function placeRanking(ps, max = 8) {
  const top = ps[0]?.count || 1;
  return ps.slice(0, max).map(p => `<button class="rank-row" data-act="place" data-arg="${esc(p.name)}"><span>${esc(p.name)} <small class="muted">${esc(p.province || '')}</small></span>${meter(p.count, top, 'var(--series-1)')}<b>${p.count}</b></button>`).join('') || `<div class="muted" style="font-size:12px">${L('No places recognized in this view.', 'Aucun lieu reconnu dans cette vue.')}</div>`;
}
function accountRow(a) {
  return `<button class="account-row" data-act="account" data-arg="${esc(a.handle)}">${avatar(a.name, a.handle, 'sm')}<div style="min-width:0"><strong>${esc(a.name)}</strong><small>@${esc(a.handle)} · ${L('last', 'dernier')} ${age(a.last_post)}</small>${topicMix(a.topic_counts)}</div><div class="n">${nf(a.posts)}<small>${L('posts', 'publ.')}</small></div></button>`;
}
function richText(text, terms) {
  const re = /(https?:\/\/[^\s]+)|(@[A-Za-z0-9_]{1,15})|(#[\p{L}\p{N}_]+)/gu;
  let out = '', last = 0;
  for (const m of text.matchAll(re)) {
    out += highlight(text.slice(last, m.index), terms);
    const s = m[0];
    if (m[1]) out += `<a href="${esc(s)}" target="_blank" rel="noopener noreferrer nofollow">${esc(s.replace(/^https?:\/\//, '').slice(0, 42))}</a>`;
    else if (m[2]) out += `<button class="ent" data-act="account" data-arg="${esc(s.slice(1))}">${esc(s)}</button>`;
    else out += `<button class="ent" data-act="hashtag" data-arg="${esc(s.slice(1))}">${highlight(s, terms)}</button>`;
    last = m.index + s.length;
  }
  return out + highlight(text.slice(last), terms);
}
function highlight(s, terms) {
  const parts = terms.map(t => t.split(/[^\p{L}\p{N}]+/u).filter(Boolean).map(w => w.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('[^\\p{L}\\p{N}]+')).filter(Boolean);
  if (!parts.length) return esc(s);
  const re = new RegExp(`(?<![\\p{L}\\p{N}])(?:${parts.join('|')})(?![\\p{L}\\p{N}])`, 'giu');
  let out = '', last = 0;
  for (const m of s.matchAll(re)) { out += esc(s.slice(last, m.index)) + `<mark>${esc(m[0])}</mark>`; last = m.index + m[0].length }
  return out + esc(s.slice(last));
}
function postCard(p, withAuthor = true) {
  const terms = [...new Set((p.topics || []).flatMap(t => t.terms || []))];
  const m = p.metrics || {};
  const metric = (ic, v, label) => `<span class="metric" data-tip="${esc(label)}">${icon(ic)}${compact(v)}</span>`;
  return `<article class="post">${withAuthor ? avatar(p.author_name, p.author_handle) : ''}<div style="min-width:0;${withAuthor ? '' : 'grid-column:1/-1'}">
    <div class="post-head">${withAuthor ? `<button class="who" data-act="account" data-arg="${esc(p.author_handle)}">${esc(p.author_name)}</button><span class="handle">@${esc(p.author_handle)}</span><span class="muted">·</span>` : ''}<time title="${esc(date(p.created_at))} (Kinshasa)">${date(p.created_at, true)}</time><span class="kind">${p.kind && p.kind !== 'post' ? `<span class="badge">${esc(kindName(p.kind))}</span> ` : ''}<span class="badge">${esc(p.lang)}</span></span></div>
    <div class="post-text ${(p.text || '').length > 480 ? 'clamp' : ''}">${richText(p.text || '', terms)}</div>${(p.text || '').length > 480 ? `<button class="linkbtn" data-act="expand">${L('Show full post', 'Afficher tout')}</button>` : ''}
    <div class="post-foot">${metric('reply', m.reply_count, L('Replies', 'Réponses'))}${metric('repost', m.retweet_count, L('Reposts', 'Republications'))}${metric('quote', m.quote_count, L('Quotes', 'Citations'))}${metric('heart', m.like_count, L('Likes', 'J’aime'))}${m.impression_count ? metric('eye', m.impression_count, L('Impressions', 'Impressions')) : ''}<span class="grow"></span><button class="linkbtn" data-act="copy" data-arg="${esc(p.url)}">${L('Copy link', 'Copier le lien')}</button><a href="${esc(p.url)}" target="_blank" rel="noopener noreferrer">${L('Open on X', 'Ouvrir sur X')} ↗</a></div>
    ${namedActors('post', p.id).length ? `<div class="chips" style="margin-top:8px">${actorChips(namedActors('post', p.id))}</div>` : ''}
    <div class="post-topics">${(p.topics || []).map(t => `<span class="tag" data-tip="${esc(t.via === 'account_history' ? L('Matched locally during an account-history scan', 'Correspondance locale lors d’un historique de compte') : L('Returned by the X topic query', 'Renvoyé par la requête X du thème'))}"><i class="swatch" style="background:${topicColor(t.topic)}"></i>${esc(topicName(t.topic))}${t.terms?.length ? ` <em>· ${esc(t.terms.join(', '))}</em>` : ''}</span>`).join('')}</div>
  </div></article>`;
}

// ---------------------------------------------------------------------------------------------
// Briefing

function briefingView() {
  const news = state.items.filter(a => a.record_type !== 'evidence');
  const high = state.items.filter(a => a.rank.band === 'high');
  const days = [...Array(14)].map((_, i) => new Date(Date.now() + 3600000 - (13 - i) * 86400000).toISOString().slice(0, 10));
  const perDay = days.map(d => news.filter(a => kinDay(a.published_at) === d).length);
  const s = state.social, items = s?.items || [];
  const rising = items.filter(i => i.data?.rising).length;
  const srcs = state.sources?.items?.filter(x => !x.id.startsWith('discovered-')) || [];
  const ready = srcs.filter(x => x.status === 'ready').length;
  const queue = [...state.items].sort((a, b) => b.rank.score - a.rank.score).slice(0, 8);
  const today = new Date().toLocaleDateString(locale(), { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric', timeZone: 'Africa/Kinshasa' });
  const kpi = (label, ic, value, foot, extra = '', cls = '') => `<div class="panel kpi ${cls}"><div class="kpi-label">${icon(ic)}${label}</div><div class="kpi-value">${value}</div><div class="kpi-foot">${foot}</div>${extra}</div>`;
  return pageHead(`DRC · ${L('OPEN-SOURCE MONITORING', 'VEILLE EN SOURCES OUVERTES')} · ${esc(today)}`, L('Daily briefing', 'Synthèse du jour'), L('What changed across news, primary documents and public conversation — ranked for analyst attention, linked to every source.', 'Ce qui a changé dans les articles, les documents primaires et la conversation publique — classé pour l’analyste, relié à chaque source.'), `${btnExport()}${btnCollect()}${btnBatch()}`)
    + `<div class="kpis">
      ${kpi(L('Reports loaded', 'Articles chargés'), 'layers', nf(state.items.length), `${nf(state.total)} ${L('news articles collected', 'articles collectés')}`, `<span data-tip="${esc(L('Publications per day, last 14 days', 'Publications par jour, 14 derniers jours'))}">${spark(perDay)}</span>`)}
      ${kpi(L('High priority', 'Priorité élevée'), 'alert', nf(high.length), L('score ≥ 65 · attention, not truth', 'score ≥ 65 · attention, pas vérité'), '', high.length ? 'alert' : '')}
      ${kpi(L('X topics rising', 'Thèmes X en hausse'), 'trend', `${rising}<small class="muted" style="font-size:14px"> / ${items.length}</small>`, L('volume ≥ 3× six-hour mean', 'volume ≥ 3× moyenne sur 6 h'))}
      ${kpi(L('Accounts archived', 'Comptes archivés'), 'users', nf(s?.archive?.accounts), `${nf(s?.archive?.posts)} ${L('topic posts retained', 'publications conservées')}`)}
      ${kpi(L('AI briefs ready', 'Synthèses IA prêtes'), 'spark', nf(state.items.filter(brief).length), activeTasks() ? `${activeTasks()} ${L('in progress', 'en cours')}` : `${esc(state.status.model || '—')}`)}
      ${kpi(L('Primary sources', 'Sources primaires'), 'book', `${ready}<small class="muted" style="font-size:14px"> / ${srcs.length}</small>`, `${nf(state.ev?.items?.length)} ${L('documents collected', 'documents collectés')}`)}
    </div>
    <div class="brief-grid">
      <section class="panel"><div class="panel-head"><h2>${icon('alert')}${L('Priority queue', 'File prioritaire')}</h2><button class="linkbtn" data-act="nav" data-arg="reports">${L('All reports', 'Tous les articles')} →</button></div><div class="report-list">${queue.map(a => reportRow(a)).join('') || emptyState(L('No reports yet', 'Aucun article'), L('Collection may still be running.', 'La collecte peut être en cours.'))}</div></section>
      <section class="panel"><div class="panel-head"><h2>${icon('radio')}${L('Conversation pulse · X', 'Pouls des conversations · X')}</h2><button class="linkbtn" data-act="nav" data-arg="social">${L('Open', 'Ouvrir')} →</button></div>
        ${items.map(i => { const d = i.data; return `<div class="pulse-row" role="button" tabindex="0" data-act="social-topic" data-arg="${esc(i.topic.id)}"><strong><i class="swatch" style="background:${topicColor(i.topic.id)}"></i>${esc(i.topic[state.lang])}${d?.rising ? ` <span class="badge high">${icon('trend')}${L('Rising', 'Hausse')}</span>` : ''}</strong>${d ? bars(d.buckets.map(b => ({ value: b.tweet_count, tip: `${date(b.start, true)} · ${nf(b.tweet_count)}` })), topicColor(i.topic.id)) : '<span></span>'}<span><span class="big">${d ? compact(d.total) : '—'}</span> <small>${L('posts / 24 h', 'publ. / 24 h')} · ${L('last hour', 'dernière heure')} ${d ? nf(d.latest_hour) : '—'}</small></span></div>` }).join('') || `<div class="panel-body muted">${L('X monitoring is not configured.', 'La veille X n’est pas configurée.')}</div>`}
        <div class="panel-body notice" style="margin:0 16px 14px">${L('Topics overlap and include reposts. Volume is a signal, not an event.', 'Les thèmes se chevauchent et incluent les republications. Le volume est un signal, pas un événement.')}</div></section>
      <div class="stack">
        <section class="panel"><div class="panel-head"><h2>${icon('pin')}${L('Hotspots', 'Points chauds')}</h2><small>${L('place mentions', 'lieux cités')}</small></div><div class="panel-body">${placeRanking(placesList(), 7)}</div></section>
        ${(() => { const top = (state.actors?.items || []).filter(a => a.mentions?.recent).sort((a, b) => b.mentions.recent - a.mentions.recent).slice(0, 6); return top.length ? `<section class="panel"><div class="panel-head"><h2>${icon('flag')}${L('Key figures in the news', 'Personnalités dans l’actualité')}</h2><small>${L('mentions · 7 days', 'mentions · 7 jours')}</small></div>${top.map(a => `<button class="account-row" data-act="actor" data-arg="${esc(a.id)}">${portrait(a, 'sm')}<div style="min-width:0"><strong>${esc(a.name)}</strong><small>${a.sanctioned ? `${L('Sanctioned', 'Sanctionné')} · ${[...new Set(a.sanctions.map(s => s.list))].join(', ')}` : esc(actorDesc(a))}</small></div><div class="n">${a.mentions.recent}<small>${L('mentions', 'mentions')}</small></div></button>`).join('')}</section>` : '' })()}
        <section class="panel"><div class="panel-head"><h2>${icon('users')}${L('Most active accounts', 'Comptes les plus actifs')}</h2><small>${L('on monitored topics', 'sur les thèmes suivis')}</small></div>${(state.accounts?.items || []).slice(0, 5).map(accountRow).join('') || `<div class="panel-body muted" style="font-size:12px">${L('Accounts appear once posts are archived.', 'Les comptes apparaissent après archivage des publications.')}</div>`}</section>
        <section class="panel"><div class="panel-head"><h2>${icon('shield')}${L('Collection health', 'État de la collecte')}</h2></div><div class="panel-body">${feedStatuses().map(([c, label, v]) => `<div class="health-row"><i class="dot ${c}"></i><div><strong>${esc(label)}</strong><small>${esc(v)}</small></div></div>`).join('')}</div></section>
      </div>
    </div>`;
}
const emptyState = (h, p, action = '') => `<div class="empty">${icon('search')}<h3>${h}</h3><p>${p}</p>${action}</div>`;

// ---------------------------------------------------------------------------------------------
// Reports library + saved

function reportsView() {
  const saved = state.view === 'saved';
  const items = libraryItems(), ps = placesList(items);
  const publishers = [...new Set(state.items.map(a => a.source))].sort();
  return pageHead(saved ? L('ANALYST WORKSPACE', 'ESPACE ANALYSTE') : L('NEWS + PRIMARY DOCUMENTS', 'ARTICLES + DOCUMENTS PRIMAIRES'), saved ? L('Saved reports', 'Articles sauvegardés') : L('Report library', 'Bibliothèque'), L('Ranked by an explainable attention score. Every report links to its original source and provenance.', 'Classés selon un score d’attention explicable. Chaque article renvoie à sa source et à sa provenance.'), `${btnExport()}${btnCollect()}${btnBatch()}`)
    + `<div class="split"><section class="panel">
      <div class="toolbar"><label class="search">${icon('search')}<input id="report-q" class="input" type="search" data-model="q" value="${esc(state.q)}" placeholder="${L('Search title, text, places, AI briefs…', 'Rechercher titre, texte, lieux, synthèses…')}" aria-label="${L('Search reports', 'Rechercher')}"></label>
        <select class="select" data-model="publisher" aria-label="${L('Publisher', 'Source')}"><option value="">${L('All publishers', 'Toutes les sources')}</option>${publishers.map(p => `<option value="${esc(p)}" ${state.publisher === p ? 'selected' : ''}>${esc(p)} (${state.items.filter(a => a.source === p).length})</option>`).join('')}</select>
        <select class="select" data-model="sort" aria-label="${L('Sort', 'Tri')}"><option value="priority" ${state.sort === 'priority' ? 'selected' : ''}>${L('Priority', 'Priorité')}</option><option value="latest" ${state.sort === 'latest' ? 'selected' : ''}>${L('Newest', 'Plus récents')}</option></select>
        <div class="chips" style="width:100%">${Object.keys(CATS).map(k => `<button class="chip ${state.category === k ? 'active' : ''}" data-act="category" data-arg="${k}">${k !== 'all' ? `<i class="swatch" style="background:${layerColor(k)}"></i>` : ''}${layerName(k)}</button>`).join('')}${state.place ? `<button class="chip active" data-act="clear-place">${icon('pin')}${esc(state.place)} ×</button>` : ''}<span style="flex:1"></span><label class="check"><input type="checkbox" data-model="includeRegional" ${state.includeRegional ? 'checked' : ''}>${L('Include regional context', 'Inclure le contexte régional')}</label></div>
      </div>
      <div class="list-meta"><span class="num">${nf(items.length)} ${L('reports', 'articles')}${activeTasks() ? ` · ${activeTasks()} ${L('AI tasks in progress', 'tâches IA en cours')}` : ''}</span><span>${L('Single source · unverified', 'Source unique · non corroboré')}</span></div>
      <div class="report-list">${items.slice(0, state.visible).map(a => reportRow(a)).join('') || emptyState(saved ? L('Nothing saved yet', 'Rien de sauvegardé') : L('No reports match this view', 'Aucun article ne correspond'), saved ? L('Use the bookmark on any report to keep it here.', 'Utilisez le signet d’un article pour le conserver ici.') : L('Try another topic, place, or search term.', 'Essayez un autre thème, lieu ou terme.'), `<button class="btn" data-act="reset">${L('Clear filters', 'Effacer les filtres')}</button>`)}</div>
      ${items.length > state.visible || (state.items.length < state.total && state.limit < 500) ? `<div class="loadmore"><button class="btn" data-act="more">${L('Load more reports', 'Charger plus d’articles')}</button></div>` : ''}
    </section><div class="stack">
      <section class="panel"><div class="panel-head"><h2>${icon('pin')}${L('Places in view', 'Lieux de la sélection')}</h2><small>${ps.length}</small></div><div class="panel-body">${placeRanking(ps, 10)}</div></section>
      <section class="panel"><div class="panel-head"><h2>${icon('spark')}${L('AI research assistant', 'Assistant de recherche IA')}</h2><span class="badge accent">${esc((state.status.provider || '—').toUpperCase())}</span></div><div class="panel-body"><p style="margin:0 0 12px;color:var(--text-2);font-size:13px">${L('Generate bilingual briefs and an impact assessment for the five highest-ranked reports in this view that lack one.', 'Générez des synthèses bilingues et une analyse d’impact pour les cinq articles prioritaires de cette vue sans synthèse.')}</p>${btnBatch()}<div class="notice">${L('AI output is interpretation; verify against the original source.', 'Les sorties IA sont des interprétations ; vérifiez la source originale.')}</div></div></section>
    </div></div>`;
}

// ---------------------------------------------------------------------------------------------
// Social intelligence

function socialView() {
  const s = state.social, items = s?.items || [], f = state.sf;
  const posts = filteredPosts();
  const all = state.posts?.items || [];
  const langs = countBy(all, p => p.lang);
  const status = !s ? [L('Connection unavailable', 'Connexion indisponible'), 'bad'] : !s.configured ? [L('Token needed', 'Jeton requis'), 'warn'] : !s.enabled ? [L('Configured · paused', 'Configuré · en pause'), 'warn'] : s.budget_paused ? [L('Daily request cap reached', 'Plafond quotidien atteint'), 'warn'] : [L('Collecting', 'Collecte active'), 'ok live'];
  const budget = (label, used, limit) => `<span class="budget" data-tip="${esc(L('Requests reserved today (UTC). Caps are enforced server-side.', 'Requêtes réservées aujourd’hui (UTC). Plafonds appliqués côté serveur.'))}">${label} <b class="num">${used ?? 0}/${limit ?? 0}</b>${meter(used, limit, (used >= limit ? 'var(--critical)' : 'var(--accent)'))}</span>`;
  const aq = norm(f.accountQ.trim());
  const accounts = (state.accounts?.items || []).filter(a => (!aq || norm(a.handle + ' ' + a.name).includes(aq)) && (!f.topic || a.topic_counts?.[f.topic]));
  const hashtags = countBy(posts, p => p.hashtags).slice(0, 14);
  const mentions = countBy(posts, p => p.mentions).slice(0, 10);
  const domains = countBy(posts, p => p.domains).slice(0, 8);
  return pageHead(`X · ${L('PUBLIC CONVERSATION', 'CONVERSATION PUBLIQUE')}`, L('Social intelligence', 'Veille sociale'), L('Volume, voices and account histories on the topics we monitor. Posts are public, attributed and keyword-matched — never labeled.', 'Volume, voix et historiques de comptes sur les thèmes suivis. Publications publiques, attribuées et repérées par mots-clés — jamais étiquetées.'),
    `<form class="lookup" data-form="lookup"><input id="lookup-handle" class="input" placeholder="@handle" aria-label="${L('Look up an X account', 'Rechercher un compte X')}" autocomplete="off" spellcheck="false"><button class="btn primary" type="submit">${icon('user')}${L('Account dossier', 'Dossier de compte')}</button></form>`)
    + `<section class="panel status-strip"><strong><i class="dot ${status[1]}"></i>${status[0]}</strong>
        ${budget(L('Counts', 'Comptages'), s?.requests_today, s?.daily_request_limit)}${budget(L('Topic samples', 'Échantillons'), s?.post_requests_today, s?.post_request_limit)}${budget(L('On-demand', 'À la demande'), s?.account_requests_today, s?.account_request_limit)}
        <span>${L('Archive', 'Archive')}: <b class="num">${nf(s?.archive?.posts)}</b> ${L('posts', 'publ.')} · <b class="num">${nf(s?.archive?.accounts)}</b> ${L('accounts', 'comptes')} · ${s?.archive?.retention_days ?? '—'} ${L('day retention', 'jours de conservation')}</span>
        <button class="btn sm ghost" data-act="reload" style="margin-left:auto">${icon('refresh')}${L('Refresh view', 'Actualiser')}</button></section>
      <div class="topic-grid">${items.map(topicCard).join('')}</div>
      <div class="split"><section class="panel">
        <div class="panel-head"><h2>${icon('radio')}${L('Post archive', 'Archive des publications')}</h2><small class="num">${nf(posts.length)} ${L('shown', 'affichées')}${state.posts?.total > all.length ? ` · ${L('latest', 'derniers')} ${nf(all.length)} / ${nf(state.posts.total)}` : ''}</small></div>
        <div class="toolbar" style="padding-top:4px">
          <div class="chips" style="width:100%"><button class="chip ${!f.topic ? 'active' : ''}" data-act="social-topic" data-arg="">${L('All topics', 'Tous les thèmes')} <b>${nf(all.length)}</b></button>${topicsList().map(t => `<button class="chip ${f.topic === t.id ? 'active' : ''}" data-act="social-topic" data-arg="${esc(t.id)}"><i class="swatch" style="background:${topicColor(t.id)}"></i>${esc(t[state.lang])} <b>${nf(all.filter(p => p.topics?.some(x => x.topic === t.id)).length)}</b></button>`).join('')}</div>
          <label class="search">${icon('search')}<input id="post-q" class="input" type="search" data-model="sf.q" value="${esc(f.q)}" placeholder="${L('Search text, #hashtag, @handle…', 'Rechercher texte, #hashtag, @compte…')}" aria-label="${L('Search posts', 'Rechercher des publications')}"></label>
          <select class="select" data-model="sf.lang" aria-label="${L('Language', 'Langue')}"><option value="">${L('All languages', 'Toutes les langues')}</option>${langs.map(([l, n]) => `<option value="${esc(l)}" ${f.lang === l ? 'selected' : ''}>${esc(l)} (${n})</option>`).join('')}</select>
          <select class="select" data-model="sf.kind" aria-label="${L('Post type', 'Type')}"><option value="">${L('All types', 'Tous les types')}</option>${['post', 'reply', 'quote', 'repost'].map(k => `<option value="${k}" ${f.kind === k ? 'selected' : ''}>${kindName(k)}</option>`).join('')}</select>
          <select class="select" data-model="sf.sort" aria-label="${L('Sort', 'Tri')}"><option value="latest" ${f.sort === 'latest' ? 'selected' : ''}>${L('Newest', 'Plus récentes')}</option><option value="engagement" ${f.sort === 'engagement' ? 'selected' : ''}>${L('Most engagement', 'Plus d’engagement')}</option></select>
          <button class="btn sm ghost" data-act="export-posts">${icon('download')}JSON</button>
        </div>
        ${(state.posts?.sources || []).filter(x => x.error).map(x => `<div class="errorbar" style="margin:12px 16px 0">${esc(topicName(x.topic))}: ${esc(x.error)}</div>`).join('')}
        <div>${posts.slice(0, f.visible).map(p => postCard(p)).join('') || emptyState(L('No posts match', 'Aucune publication'), !state.posts ? L('Post archive unavailable. Try refreshing the view.', 'Archive indisponible. Essayez d’actualiser.') : L('Topic samples run once a day within the request cap; fetch a topic now or open an account dossier.', 'Les échantillons sont collectés une fois par jour ; récupérez un thème maintenant ou ouvrez un dossier de compte.'), state.social?.enabled && state.social?.configured ? `<button class="btn primary" data-act="sample" data-arg="${esc(f.topic || topicsList()[0]?.id || '')}">${icon('refresh')}${L('Fetch latest posts', 'Récupérer les publications')}${f.topic ? ` · ${esc(topicName(f.topic))}` : ` · ${esc(topicName(topicsList()[0]?.id))}`}</button>` : '')}</div>
        ${posts.length > f.visible ? `<div class="loadmore"><button class="btn" data-act="post-more">${L('Show more posts', 'Afficher plus')}</button></div>` : ''}
      </section><div class="stack">
        <section class="panel"><div class="panel-head"><h2>${icon('users')}${L('Accounts', 'Comptes')}</h2><small class="num">${nf(accounts.length)}${f.topic ? ` · ${esc(topicName(f.topic))}` : ''}</small></div>
          <div class="panel-body" style="padding-bottom:10px"><label class="search">${icon('search')}<input id="account-q" class="input" type="search" data-model="sf.accountQ" value="${esc(f.accountQ)}" placeholder="${L('Filter accounts', 'Filtrer les comptes')}" aria-label="${L('Filter accounts', 'Filtrer les comptes')}"></label></div>
          <div class="scroll" data-scroll="accounts">${accounts.slice(0, 60).map(accountRow).join('') || `<div class="panel-body muted" style="font-size:12px">${L('No archived accounts match.', 'Aucun compte archivé ne correspond.')}</div>`}</div>
          <div class="panel-body notice" style="margin:8px 16px 14px">${L('Ranked by archived topic posts. Activity volume is not influence, coordination or affiliation.', 'Classés par publications archivées. Le volume n’indique ni influence, ni coordination, ni affiliation.')}</div></section>
        ${hashtags.length ? `<section class="panel"><div class="panel-head"><h2>${icon('hash')}${L('Hashtags in view', 'Hashtags de la sélection')}</h2></div><div class="panel-body cloud">${hashtags.map(([h, n]) => `<button data-act="hashtag" data-arg="${esc(h)}">#${esc(h)}<b>${n}</b></button>`).join('')}</div></section>` : ''}
        ${mentions.length ? `<section class="panel"><div class="panel-head"><h2>${icon('at')}${L('Most mentioned', 'Les plus mentionnés')}</h2></div><div class="panel-body cloud">${mentions.map(([h, n]) => `<button data-act="account" data-arg="${esc(h)}">@${esc(h)}<b>${n}</b></button>`).join('')}</div></section>` : ''}
        ${domains.length ? `<section class="panel"><div class="panel-head"><h2>${icon('link')}${L('Linked domains', 'Domaines cités')}</h2></div><div class="panel-body">${domains.map(([d, n]) => `<div class="rank-row" style="cursor:default"><span>${esc(d)}</span>${meter(n, domains[0][1], 'var(--series-1)')}<b>${n}</b></div>`).join('')}</div></section>` : ''}
      </div></div>
      <p class="notice">${esc(s?.notice || '')} ${L('Refreshing this screen never triggers a paid X request; account histories are fetched only when you ask.', 'Actualiser cet écran ne déclenche aucune requête X payante ; les historiques ne sont récupérés qu’à votre demande.')}</p>`;
}
function topicCard(item) {
  const d = item.data, id = item.topic.id, c = topicColor(id);
  const history = (item.history || []).map(h => ({ value: h.count, tip: `${date(h.start, true)} · ${nf(h.count)}` }));
  const state_ = item.error ? `<span style="color:var(--critical)">${L('Collection error', 'Erreur de collecte')}</span>` : !d ? L('Awaiting first collection', 'Première collecte en attente') : item.stale ? L('Snapshot · not live', 'Instantané · non direct') : `${L('Checked', 'Vérifié')} ${age(item.checked_at)}`;
  return `<div class="panel topic-card ${state.sf.topic === id ? 'active' : ''}" style="--c:${c}" role="button" tabindex="0" data-act="social-topic" data-arg="${esc(id)}" aria-pressed="${state.sf.topic === id}">
    ${d?.rising ? `<span class="badge high rising" data-tip="${esc(L('Latest hour ≥ 20 posts and ≥ 3× the previous six-hour mean. Not a confirmed event.', 'Dernière heure ≥ 20 publications et ≥ 3× la moyenne des 6 h précédentes. Événement non confirmé.'))}">${icon('trend')}${L('Rising', 'Hausse')}</span>` : ''}
    <h3><i class="swatch"></i>${esc(item.topic[state.lang])}</h3>
    <div class="topic-stats"><span class="big">${d ? compact(d.total) : '—'}</span><small>${L('posts · 24 h', 'publ. · 24 h')}</small></div>
    ${d ? bars(d.buckets.map(b => ({ value: b.tweet_count, tip: `${date(b.start, true)} → ${nf(b.tweet_count)} ${L('posts', 'publ.')}` })), c) : '<div class="bars"></div>'}
    <div class="topic-meta"><span>${L('Last hour', 'Dernière heure')} <b class="num" style="color:var(--text)">${d ? nf(d.latest_hour) : '—'}</b> · ${L('6 h mean', 'moy. 6 h')} ${d ? Number(d.baseline_hourly).toFixed(0) : '—'}</span></div>
    <div><div class="eyebrow" style="margin-bottom:4px">${L('Hourly history', 'Historique horaire')} · ${history.length > 24 ? `${Math.ceil(history.length / 24)} ${L('days', 'jours')}` : `${history.length} h`}</div>${areaChart(history, c, 40)}</div>
    <div class="topic-meta"><span>${state_}</span><span>${nf(item.archived_posts)} ${L('archived', 'archivées')}</span></div>
    ${state.social?.enabled && state.social?.configured ? `<button class="btn sm" data-act="sample" data-arg="${esc(id)}" data-tip="${esc(L(`Pull the ${state.posts?.sample_size_per_topic || 10} latest matching posts from X now. Uses 1 on-demand request.`, `Récupérer maintenant les ${state.posts?.sample_size_per_topic || 10} dernières publications. Utilise 1 requête à la demande.`))}">${icon('refresh')}${L('Fetch latest posts', 'Récupérer les publications')}</button>` : ''}
    <details data-stop><summary>${L('Query & method', 'Requête et méthode')}</summary><p class="query">${esc(item.topic.query)}</p>${item.error ? `<p class="ev-error">${esc(item.error)}</p>` : ''}<a href="${esc(item.search_url)}" target="_blank" rel="noopener noreferrer">${L('Explore live on X', 'Explorer en direct sur X')} ↗</a></details>
  </div>`;
}

// ---------------------------------------------------------------------------------------------
// Account dossier

async function openDossier(handle, { keepFilters = false } = {}) {
  handle = String(handle || '').replace(/^@/, '').trim();
  if (!validHandle(handle)) { toast(L('Enter a valid X handle (letters, numbers, underscore).', 'Saisissez un identifiant X valide.')); return }
  const prev = state.dossier;
  state.dossier = { handle, data: null, loading: true, error: '', notFound: false, topic: keepFilters ? prev?.topic : '', kind: keepFilters ? prev?.kind : '', visible: 30, busy: false };
  state.modal = 'dossier';
  if (hashAccount() !== handle) history.replaceState(null, '', '#account/' + encodeURIComponent(handle));
  renderDossier();
  try { state.dossier.data = await api(`/v1/social/accounts/${encodeURIComponent(handle)}`) }
  catch (e) { if (e.status === 404) state.dossier.notFound = true; else state.dossier.error = e.message }
  if (state.dossier?.handle !== handle) return;
  state.dossier.loading = false;
  renderDossier();
}
async function fetchHistory() {
  const d = state.dossier; if (!d || d.busy) return;
  d.busy = true; renderDossier();
  try {
    const r = await api(`/v1/social/accounts/${encodeURIComponent(d.handle)}/history`, {});
    toast(L(`Scanned ${r.scanned} recent posts · ${r.matched} on monitored topics · ${r.new} new`, `${r.scanned} publications analysées · ${r.matched} sur les thèmes suivis · ${r.new} nouvelles`));
    await openDossier(d.handle, { keepFilters: true });
    load();
  } catch (e) { toast(e.message); if (state.dossier) { state.dossier.busy = false; renderDossier() } }
}
function renderDossier() {
  const d = state.dossier; if (!d) return;
  const data = d.data, a = data?.account, posts = data?.posts || [];
  const s = state.social, canFetch = data?.history?.available ?? (s?.enabled && s?.configured);
  const limit = data?.history?.request_limit ?? s?.account_request_limit, used = data?.history?.requests_today ?? s?.account_requests_today;
  let body;
  if (d.loading) body = `<div class="loading-line"></div><p class="muted">${L('Loading dossier…', 'Chargement du dossier…')}</p>`;
  else if (d.error) body = `<div class="errorbar">${esc(d.error)}</div>`;
  else {
    const pm = a?.profile?.public_metrics || {};
    const joined = a?.profile?.created_at ? new Date(a.profile.created_at) : null;
    const years = joined ? ((Date.now() - joined) / 31557600000) : null;
    const topicCounts = countBy(posts, p => (p.topics || []).map(t => t.topic));
    const kinds = countBy(posts, p => p.kind);
    const langs = countBy(posts, p => p.lang);
    const hashtags = countBy(posts, p => p.hashtags).slice(0, 12);
    const mentions = countBy(posts, p => p.mentions).slice(0, 10);
    const domains = countBy(posts, p => p.domains).slice(0, 6);
    const dayList = [...new Set(posts.map(p => kinDay(p.created_at)))].sort();
    let days = [];
    if (dayList.length) { const end = Date.parse(dayList.at(-1)), start = Math.max(Date.parse(dayList[0]), end - 59 * 86400000); for (let t = start; t <= end; t += 86400000) days.push(new Date(t).toISOString().slice(0, 10)) }
    const perDay = new Map(countBy(posts, p => kinDay(p.created_at)));
    const perHour = new Map(countBy(posts, p => kinHour(p.created_at)));
    const last = a?.history_last;
    const shown = posts.filter(p => (!d.topic || p.topics?.some(t => t.topic === d.topic)) && (!d.kind || p.kind === d.kind));
    const header = `<div class="dossier-head">${avatar(a?.name || d.handle, d.handle, 'lg')}<div style="min-width:0"><h1>${esc(a?.name || '@' + d.handle)}</h1><div class="handle">@${esc(a?.handle || d.handle)}${a?.profile?.verified ? ` <span class="badge accent">${L('Verified', 'Vérifié')}${a.profile.verified_type && a.profile.verified_type !== 'none' ? ' · ' + esc(a.profile.verified_type) : ''}</span>` : ''}${a?.profile?.protected ? ` <span class="badge">${L('Protected', 'Protégé')}</span>` : ''}</div>
      ${a?.previous_handles?.length ? `<div class="muted" style="font-size:12px;margin-top:4px">${icon('history')} ${L('Previously seen as', 'Auparavant')} ${a.previous_handles.map(h => '@' + esc(h)).join(', ')}</div>` : ''}
      ${a?.profile?.description ? `<p class="dossier-bio">${richText(a.profile.description, [])}</p>` : ''}
      <div class="dossier-facts">${a?.profile?.location ? `<span>${icon('pin')}${esc(a.profile.location)}</span>` : ''}${joined ? `<span>${icon('calendar')}${L('Joined', 'Inscrit')} ${joined.toLocaleDateString(locale(), { month: 'short', year: 'numeric' })} · ${years.toFixed(1)} ${L('yrs', 'ans')}</span>` : ''}${pm.followers_count != null ? `<span><b>${compact(pm.followers_count)}</b> ${L('followers', 'abonnés')}</span><span><b>${compact(pm.following_count)}</b> ${L('following', 'abonnements')}</span><span><b>${compact(pm.tweet_count)}</b> ${L('posts total', 'publications')}</span>` : ''}</div></div></div>`;
    const historyBox = `<div class="panel history-box"><div><strong>${icon('history')} ${L('Topic history from X', 'Historique thématique depuis X')}</strong><p>${last ? L(`Last scan ${age(last.checked_at)} ago: ${last.scanned} recent posts read, ${last.matched} matched monitored topics.`, `Dernière analyse il y a ${age(last.checked_at)} : ${last.scanned} publications lues, ${last.matched} sur les thèmes suivis.`) : L('Reads the account’s most recent posts (up to 100 per page) and keeps only those matching the monitored topic rules.', 'Lit les publications récentes du compte (jusqu’à 100 par page) et ne conserve que celles qui correspondent aux thèmes suivis.')} ${L(`Uses ${a ? 1 : 2}+ of today’s account-lookup budget (${used ?? 0}/${limit ?? 0} used).`, `Utilise ${a ? 1 : 2}+ requêtes du budget quotidien (${used ?? 0}/${limit ?? 0}).`)}${a?.history_error ? ` <span class="ev-error">${esc(a.history_error)}</span>` : ''}</p></div>
      ${canFetch ? `<button class="btn primary" data-act="history" ${d.busy ? 'disabled' : ''}>${icon('refresh')}${d.busy ? L('Fetching…', 'Récupération…') : last ? L('Fetch newer posts', 'Récupérer les nouvelles') : L('Fetch history', 'Récupérer l’historique')}</button>` : `<span class="badge">${L('X collection disabled on server', 'Collecte X désactivée')}</span>`}</div>`;
    if (d.notFound || !a) body = header + historyBox + emptyState(L('Not in the archive yet', 'Pas encore dans l’archive'), L('This account has no archived posts on monitored topics. Fetch its recent history to check.', 'Ce compte n’a aucune publication archivée sur les thèmes suivis. Récupérez son historique récent pour vérifier.'));
    else body = header + historyBox
      + `<div class="stat-grid"><div class="panel stat"><small>${L('Topic posts archived', 'Publications archivées')}</small><b>${nf(posts.length)}</b></div><div class="panel stat"><small>${L('First / last seen', 'Première / dernière')}</small><b style="font-size:14px">${posts.length ? `${date(posts.at(-1).created_at, true)}<br>${date(posts[0].created_at, true)}` : '—'}</b></div><div class="panel stat"><small>${L('Engagement on these posts', 'Engagement sur ces publications')}</small><b>${compact(posts.reduce((n, p) => n + engagement(p), 0))}</b></div><div class="panel stat"><small>${L('Original vs. reposts', 'Originales / republications')}</small><b>${posts.filter(p => p.kind !== 'repost').length} / ${posts.filter(p => p.kind === 'repost').length}</b></div></div>
      <div class="two"><section class="panel"><div class="panel-head"><h2>${L('Topics', 'Thèmes')}</h2></div><div class="panel-body">${topicCounts.map(([t, n]) => `<div class="topic-bar"><span class="tag"><i class="swatch" style="background:${topicColor(t)}"></i>${esc(topicName(t))}</span>${meter(n, topicCounts[0][1], topicColor(t))}<b>${n}</b></div>`).join('')}<div class="divider"></div><div class="legend">${kinds.map(([k, n]) => `<span>${esc(kindName(k))} <b class="num">${n}</b></span>`).join('')}${langs.map(([l, n]) => `<span class="badge">${esc(l)} · ${n}</span>`).join('')}</div></div></section>
        <section class="panel"><div class="panel-head"><h2>${L('Posting hours', 'Heures de publication')}</h2><small>Kinshasa · UTC+1</small></div><div class="panel-body">${bars([...Array(24)].map((_, h) => ({ value: perHour.get(h) || 0, tip: `${String(h).padStart(2, '0')}:00 · ${perHour.get(h) || 0}` })))}<div class="axis"><span>00</span><span>06</span><span>12</span><span>18</span><span>23</span></div><p class="notice" style="margin-bottom:0">${L('Archived topic posts only; a partial sample of the account’s activity.', 'Publications archivées uniquement ; échantillon partiel de l’activité.')}</p></div></section></div>
      ${days.length > 1 ? `<section class="panel" style="margin-bottom:12px"><div class="panel-head"><h2>${L('Activity on monitored topics', 'Activité sur les thèmes suivis')}</h2><small>${dayLabel(days[0])} → ${dayLabel(days.at(-1))}</small></div><div class="panel-body">${bars(days.map(x => ({ value: perDay.get(x) || 0, tip: `${dayLabel(x)} · ${perDay.get(x) || 0}` })), 'var(--series-1)')}<div class="axis"><span>${dayLabel(days[0])}</span><span>${dayLabel(days.at(-1))}</span></div></div></section>` : ''}
      ${hashtags.length || mentions.length || domains.length ? `<div class="two">${hashtags.length ? `<section class="panel"><div class="panel-head"><h2>${icon('hash')}${L('Hashtags', 'Hashtags')}</h2></div><div class="panel-body cloud">${hashtags.map(([h, n]) => `<button data-act="hashtag" data-arg="${esc(h)}">#${esc(h)}<b>${n}</b></button>`).join('')}</div></section>` : ''}${mentions.length || domains.length ? `<section class="panel"><div class="panel-head"><h2>${icon('at')}${L('Mentions & links', 'Mentions et liens')}</h2></div><div class="panel-body cloud">${mentions.map(([h, n]) => `<button data-act="account" data-arg="${esc(h)}">@${esc(h)}<b>${n}</b></button>`).join('')}${domains.map(([h, n]) => `<button disabled style="cursor:default;opacity:1">${icon('link')} ${esc(h)}<b>${n}</b></button>`).join('')}</div></section>` : ''}</div>` : ''}
      <section class="panel"><div class="toolbar"><div class="chips"><button class="chip ${!d.topic ? 'active' : ''}" data-act="dossier-topic" data-arg="">${L('All', 'Tous')} <b>${posts.length}</b></button>${topicCounts.map(([t, n]) => `<button class="chip ${d.topic === t ? 'active' : ''}" data-act="dossier-topic" data-arg="${esc(t)}"><i class="swatch" style="background:${topicColor(t)}"></i>${esc(topicName(t))} <b>${n}</b></button>`).join('')}</div><span style="flex:1"></span><div class="seg">${['', 'post', 'reply', 'quote', 'repost'].map(k => `<button class="${(d.kind || '') === k ? 'active' : ''}" data-act="dossier-kind" data-arg="${k}">${k ? kindName(k) : L('All types', 'Tous')}</button>`).join('')}</div></div>
        ${shown.slice(0, d.visible).map(p => postCard(p, false)).join('') || emptyState(L('No posts in this filter', 'Aucune publication pour ce filtre'), '')}
        ${shown.length > d.visible ? `<div class="loadmore"><button class="btn" data-act="dossier-more">${L('Show more', 'Afficher plus')} (${shown.length - d.visible})</button></div>` : ''}</section>
      <p class="notice">${esc(data.notice || '')}</p>`;
  }
  const scroller = $('#overlay .drawer'), top = scroller?.scrollTop || 0;
  $('#overlay').innerHTML = `<div class="shade" data-close><section class="drawer" role="dialog" aria-modal="true" aria-label="${esc(L('Account dossier', 'Dossier de compte'))} @${esc(d.handle)}"><div class="drawer-top"><span class="eyebrow">${icon('user')}${L('ACCOUNT DOSSIER', 'DOSSIER DE COMPTE')} · X</span><div class="actions">${a ? `<button class="btn sm" data-act="export-dossier">${icon('download')}JSON</button>` : ''}<a class="btn sm" href="https://x.com/${esc(d.handle)}" target="_blank" rel="noopener noreferrer">${icon('external')}${L('Open on X', 'Ouvrir sur X')}</a><button class="iconbtn" data-act="close" aria-label="${L('Close', 'Fermer')}">${icon('close')}</button></div></div><div class="drawer-body">${body}</div></section></div>`;
  const drawer = $('#overlay .drawer'); if (drawer) drawer.scrollTop = top;
  if (!scroller) $('#overlay [data-act="close"]')?.focus();
}

// ---------------------------------------------------------------------------------------------
// Situation room (map-first)

// Versioned, attributed reference data; independent of the report time window.
state.geology = null; state.mineralLayers = ['IPIS', 'USGS']; state.mineralQuery = '';
state.mineralSearch = ''; state.mineralProvince = ''; state.mineralKind = ''; state.mineralSelected = ''; state.mineralVisible = 15; state.nearbyRadius = '25'; state.mapFocus = false;
function mineralFeatures() {
  const q = norm(state.mineralSearch.trim());
  return (state.geology?.features || []).filter(({properties:p}) => state.mineralLayers.includes(p.dataset) &&
    (!state.mineralQuery || p.minerals.includes(state.mineralQuery)) &&
    (!state.mineralProvince || p.province === state.mineralProvince) &&
    (!state.mineralKind || p.kind === state.mineralKind) &&
    (!q || norm([p.name,p.province,...p.minerals].join(' ')).includes(q)));
}
const mineralKindLabel = k => ({artisanal:L('Artisanal site','Site artisanal'),facility:L('Facility','Installation'),deposit:L('Deposit / occurrence','Gisement / indice'),exploration:L('Exploration record','Référence d’exploration')})[k] || k;
function distanceKm(a, b) {
  const rad = Math.PI / 180, dlat = (b[1]-a[1])*rad, dlon = (b[0]-a[0])*rad;
  const h = Math.sin(dlat/2)**2 + Math.cos(a[1]*rad)*Math.cos(b[1]*rad)*Math.sin(dlon/2)**2;
  return 6371 * 2 * Math.asin(Math.sqrt(Math.min(1,Math.max(0,h))));
}
function nearbyMineralReports(f) {
  return situationItems(true).map(a => {
    const places = a.places.filter(p => Number.isFinite(p.lon) && Number.isFinite(p.lat)).map(p => ({...p,km:distanceKm(f.geometry.coordinates,[p.lon,p.lat])})).sort((a,b)=>a.km-b.km);
    return {article:a,place:places[0]};
  }).filter(r => r.place && r.place.km <= Number(state.nearbyRadius)).sort((a,b)=>a.place.km-b.place.km);
}
function selectedMineral() { return mineralFeatures().find(f=>f.id===state.mineralSelected) }
function selectMineral(id) {
  const f = mineralFeatures().find(f=>f.id===id); if (!f) return;
  state.mineralSelected=id; state.mapFocus=false; render();
  mapInstance?.flyTo({center:f.geometry.coordinates,zoom:13,duration:700});
}
function fitMinerals() {
  const fs=mineralFeatures(); if (!fs.length) return;
  if (!mapInstance) { toast(L('Interactive map unavailable; use the site list.', 'Carte interactive indisponible ; utilisez la liste.')); return }
  const xs=fs.map(f=>f.geometry.coordinates[0]),ys=fs.map(f=>f.geometry.coordinates[1]);
  mapInstance.fitBounds([[Math.min(...xs),Math.min(...ys)],[Math.max(...xs),Math.max(...ys)]],{padding:70,maxZoom:13,duration:650});
}
function mineralInspector() {
  const f=selectedMineral(); if (!f) return '';
  const p=f.properties, nearby=nearbyMineralReports(f);
  return `<section class="mineral-inspector" aria-label="${L('Selected mineral record','Référence minière sélectionnée')}"><div class="actions"><span class="badge">${esc(p.dataset)} · ${esc(mineralKindLabel(p.kind))}</span><button class="linkbtn" data-act="mineral-close">${L('Close','Fermer')} ×</button></div>
    <h2>${esc(p.name)}</h2><p>${esc(p.province)} · ${esc(p.minerals.join(' · '))}</p>
    <dl><dt>${L('Reference date','Date de référence')}</dt><dd>${esc(p.date || L('Unknown (2021 compilation)','Inconnue (compilation 2021)'))}</dd><dt>${L('Coordinates','Coordonnées')}</dt><dd>${f.geometry.coordinates[1].toFixed(5)}, ${f.geometry.coordinates[0].toFixed(5)}</dd></dl>
    <p class="muted">${esc(p.source)} · ${esc(p.license)}${p.observations ? ` · ${p.observations} ${L('visits recorded','visites enregistrées')}` : ''}</p>
    <div class="actions"><button class="btn sm" data-act="mineral-select" data-arg="${esc(f.id)}">${L('Centre map','Centrer')}</button><button class="btn sm" data-act="mineral-export">JSON ↓</button><a class="btn sm" href="${esc(p.reference)}" target="_blank" rel="noopener">Source ↗</a></div>
    <label class="nearby-label">${L('Nearby news mentions','Lieux cités à proximité')}<select class="select" data-model="nearbyRadius">${[10,25,50,100].map(n=>`<option value="${n}" ${Number(state.nearbyRadius)===n?'selected':''}>${n} km</option>`).join('')}</select></label>
    <p class="muted">${L('Matches loaded reports and news filters. Distance to an approximate place mention does not establish a connection to this site.','Selon les articles chargés et les filtres. La proximité d’un lieu cité approximatif ne prouve pas un lien avec ce site.')}</p>
    <div class="nearby-reports">${nearby.slice(0,8).map(({article:a,place:p})=>`<button class="mineral-result" data-act="report" data-arg="${esc(a.id)}"><strong>${esc(title(a))}</strong><small>${esc(p.name)} · ${p.km.toFixed(1)} km · ${esc(a.source || '')}</small></button>`).join('') || `<p>${L('No matching loaded reports in this radius.','Aucun article chargé correspondant dans ce rayon.')}</p>`}</div>
    <small>${nf(nearby.length)} ${L('matching reports · historical mineral record; current status unverified.','articles correspondants · référence minière historique ; état actuel non vérifié.')}</small></section>`;
}
function geologyPanel() {
  const all = state.geology?.features || [];
  const minerals = [...new Set(all.flatMap(f => f.properties.minerals))].filter(Boolean).sort();
  const provinces=[...new Set(all.map(f=>f.properties.province))].filter(Boolean).sort();
  const results=mineralFeatures().sort((a,b)=>a.properties.name.localeCompare(b.properties.name));
  return `<div class="pane-section"><h2>${L('Minerals & geology', 'Minerais et géologie')}</h2>
    ${!state.geology ? `<p>${L('Reference dataset unavailable. Reload to retry.', 'Données de référence indisponibles. Rechargez pour réessayer.')}</p>` : ['IPIS','USGS'].map(d => `<label class="layer-row"><input type="checkbox" data-act="mineral-layer" data-arg="${d}" ${state.mineralLayers.includes(d) ? 'checked' : ''}><i class="swatch" style="background:${d === 'IPIS' ? '#d9a441' : '#9b8afb'}"></i><span>${d}</span><b>${nf(all.filter(f => f.properties.dataset === d).length)}</b></label>`).join('')}
    <select class="select" data-model="mineralQuery" aria-label="${L('Mineral', 'Minerai')}"><option value="">${L('All minerals', 'Tous les minerais')}</option>${minerals.map(m => `<option value="${esc(m)}" ${state.mineralQuery === m ? 'selected' : ''}>${esc(m)}</option>`).join('')}</select>
    <input id="mineral-search" class="input" type="search" data-model="mineralSearch" value="${esc(state.mineralSearch)}" placeholder="${L('Find a site or mineral…','Chercher un site ou minerai…')}" aria-label="${L('Find a mining site','Rechercher un site minier')}">
    <select class="select" data-model="mineralProvince" aria-label="${L('Province','Province')}"><option value="">${L('All provinces','Toutes les provinces')}</option>${provinces.map(p=>`<option ${state.mineralProvince===p?'selected':''} value="${esc(p)}">${esc(p)}</option>`).join('')}</select>
    <select class="select" data-model="mineralKind" aria-label="${L('Record type','Type de référence')}"><option value="">${L('All record types','Tous les types')}</option>${['artisanal','facility','deposit','exploration'].map(k=>`<option ${state.mineralKind===k?'selected':''} value="${k}">${mineralKindLabel(k)}</option>`).join('')}</select>
    <div class="actions"><button class="btn sm" data-act="mineral-fit" ${results.length?'':'disabled'}>${L('Zoom to results','Voir les résultats')}</button><button class="linkbtn" data-act="mineral-reset">${L('Clear filters','Effacer les filtres')}</button></div>
    <details class="mineral-directory" ${state.mineralSearch ? 'open' : ''}><summary>${L('Browse sites','Parcourir les sites')} · ${nf(results.length)}</summary><div class="mineral-results">${results.slice(0,state.mineralVisible).map(f=>`<button class="mineral-result ${state.mineralSelected===f.id?'selected':''}" data-act="mineral-select" data-arg="${esc(f.id)}"><strong>${esc(f.properties.name)}</strong><small>${esc(f.properties.province)} · ${esc(f.properties.dataset)}<br>${esc(f.properties.minerals.join(' · '))}</small></button>`).join('') || `<p>${L('No matches. Clear a filter to broaden your search.','Aucun résultat. Effacez un filtre pour élargir la recherche.')}</p>`}</div>${results.length>state.mineralVisible?`<button class="linkbtn" data-act="mineral-more">${L('Show more','Afficher plus')}</button>`:''}</details>
    <p class="muted">${nf(mineralFeatures().length)} ${L('reference records. Click a point for its date and source. Independent of the news timeline.', 'références. Cliquez sur un point pour sa date et sa source. Indépendantes de la chronologie des actualités.')}</p>
    <p class="muted">${L('Historical observations, not proven reserves or current control.', 'Observations historiques, pas des réserves prouvées ni un contrôle actuel.')}</p>
    <small>© <a href="https://ipisresearch.be/home/maps-data/open-data/" target="_blank" rel="noopener">IPIS</a> · ODC-BY 1.0<br>USGS · CC0 · 2021</small>
    <div class="actions"><button class="btn sm" data-act="geology-archive">${L('1931 geological map', 'Carte géologique de 1931')}</button><button class="btn sm" data-act="geology-export">GeoJSON ↓</button></div></div>`;
}
function geologyArchive() {
  state.modal = 'geology'; document.body.style.overflow = 'hidden';
  $('#overlay').innerHTML = `<div class="shade" data-close><section class="drawer" style="width:min(100vw,1100px)" role="dialog" aria-modal="true" aria-label="${L('Historical geological map', 'Carte géologique historique')}"><div class="drawer-top"><strong>Congo belge · Carte géologique · Mai 1931</strong><button class="iconbtn" data-act="close" aria-label="${L('Close', 'Fermer')}">${icon('close')}</button></div><div class="drawer-body"><p>${L('Belgian Ministry of Colonies, geographical service. Scale 1:5,000,000. BnF / Gallica, GE C-15047. Geological formations, not a modern mineral-reserve inventory. Scan is not georeferenced.', 'Ministère belge des Colonies, Service géographique. Échelle 1:5 000 000. BnF / Gallica, GE C-15047. Formations géologiques, pas un inventaire actuel des réserves. Scan non géoréférencé.')}</p><p>${L('Commons marks this work public domain in France. Original attribution preserved.', 'Commons indique le domaine public en France. Attribution originale conservée.')} <a href="https://commons.wikimedia.org/wiki/File:Congo_belge._Carte_g%C3%A9ologique_-_btv1b531215222_(2_of_2).jpg" target="_blank" rel="noopener">${L('Source and rights', 'Source et droits')} ↗</a></p><a class="btn sm" href="/geology/congo-1931.jpg" download="congo-geological-map-1931.jpg">${L('Download original scan', 'Télécharger le scan original')} · 6.83 MB</a><details><summary>${L('Inspect at full resolution (scroll)', 'Examiner en pleine résolution (défilement)')}</summary><div style="overflow:auto;max-height:65vh"><img src="/geology/congo-1931.jpg" style="max-width:none;width:7330px" alt="Congo belge, carte géologique, mai 1931"></div></details><img src="/geology/congo-1931.jpg" style="width:100%;height:auto" alt="Congo belge, carte géologique, mai 1931"></div></section></div>`;
  $('#overlay [data-act="close"]')?.focus();
}
function mountMinerals() {
  if (!mapInstance?.isStyleLoaded() || !state.geology) return;
  const fs=mineralFeatures(), signature=JSON.stringify([state.mineralLayers,state.mineralQuery,state.mineralSearch,state.mineralProvince,state.mineralKind]);
  const data={type:'FeatureCollection',features:fs.map(f=>({...f,properties:{id:f.id,name:f.properties.name,dataset:f.properties.dataset}}))};
  if (mapInstance.getSource('mineral-sites')) {
    if (mapInstance.__mineralSignature!==signature) mapInstance.getSource('mineral-sites').setData(data);
  } else {
    mapInstance.addSource('mineral-sites',{type:'geojson',data,cluster:true,clusterMaxZoom:11,clusterRadius:42,attribution:'IPIS (ODC-BY 1.0) | USGS (CC0, 2021)'});
    mapInstance.addLayer({id:'mineral-clusters',type:'circle',source:'mineral-sites',filter:['has','point_count'],paint:{'circle-radius':['step',['get','point_count'],16,50,22,250,29],'circle-color':'#d9a441','circle-stroke-color':'#172c2c','circle-stroke-width':2}});
    mapInstance.addLayer({id:'mineral-counts',type:'symbol',source:'mineral-sites',filter:['has','point_count'],layout:{'text-field':['get','point_count_abbreviated'],'text-size':12},paint:{'text-color':'#172c2c'}});
    mapInstance.addLayer({id:'mineral-sites',type:'circle',source:'mineral-sites',filter:['!', ['has','point_count']],paint:{'circle-radius':['interpolate',['linear'],['zoom'],3,3,12,7], 'circle-color':['match',['get','dataset'],'IPIS','#d9a441','#9b8afb'],'circle-stroke-width':1,'circle-stroke-color':'#172c2c','circle-opacity':0.9}});
  }
  mapInstance.__mineralSignature=signature;
  const selected=selectedMineral(), selection={type:'FeatureCollection',features:selected?[selected]:[]};
  if (mapInstance.getSource('mineral-selection')) {
    if (mapInstance.__selectionId !== (selected?.id || '')) mapInstance.getSource('mineral-selection').setData(selection);
  }
  else {
    mapInstance.addSource('mineral-selection',{type:'geojson',data:selection});
    mapInstance.addLayer({id:'mineral-selection',type:'circle',source:'mineral-selection',paint:{'circle-radius':12,'circle-color':'transparent','circle-stroke-color':'#00c9bf','circle-stroke-width':3}});
  }
  mapInstance.__selectionId=selected?.id || '';
}
function mineralPopup(e) { const id=e.features?.[0]?.properties.id; if (id) selectMineral(id) }
function mineralCluster(e) {
  const f=e.features?.[0], map=mapInstance; if (!f || !map) return;
  map.getSource('mineral-sites').getClusterExpansionZoom(f.properties.cluster_id,(err,zoom)=>{
    if (!err && map===mapInstance) map.easeTo({center:f.geometry.coordinates,zoom,duration:550});
  });
}

function situationView() {
  const items = situationItems(), ps = placesList(situationItems(true));
  const timeline = (() => {
    const counts = new Map(countBy(situationItems(false, true), a => kinDay(a.published_at)));
    const days = [...counts.keys()].sort(); const max = Math.max(1, ...counts.values());
    return `<div class="timeline-head"><strong>${L('Publication timeline', 'Chronologie des publications')}</strong><span>${L('Kinshasa dates (UTC+1)', 'Dates de Kinshasa (UTC+1)')}</span>${state.selectedDay ? `<button class="linkbtn" data-act="all-days">${L('All days', 'Tous les jours')}</button>` : ''}</div><div class="day-bars">${days.map(d => `<button class="day-bar ${state.selectedDay === d ? 'active' : ''}" data-act="day" data-arg="${d}" aria-pressed="${state.selectedDay === d}" data-tip="${esc(`${dayLabel(d)} · ${counts.get(d)} ${L('reports', 'articles')}`)}"><i style="height:${Math.max(6, counts.get(d) / max * 100)}%"></i></button>`).join('') || `<small class="muted">${L('No dated reports in this window.', 'Aucun article daté dans cette période.')}</small>`}</div>`;
  })();
  return `<div class="situation ${state.mapFocus ? 'map-focus' : ''}">
    <aside class="filters-pane" data-scroll="filters">
      <div class="pane-section"><div class="eyebrow">DRC · ${L('SITUATION ROOM', 'VUE DE SITUATION')}</div><div style="font-size:13px;color:var(--text-2)"><b class="num" style="color:var(--text)">${nf(items.length)}</b> ${L('reports in view', 'articles affichés')} · ${nf(state.items.length)} ${L('loaded', 'chargés')}</div><label class="search">${icon('search')}<input id="sit-q" class="input" data-model="q" value="${esc(state.q)}" placeholder="${L('Filter reports…', 'Filtrer les articles…')}" aria-label="${L('Filter reports', 'Filtrer les articles')}"></label></div>
      <div class="pane-section"><h2>${L('Layers', 'Couches')}<button class="linkbtn" data-act="all-layers">${L('All', 'Toutes')}</button></h2>${LAYERS.map(k => `<label class="layer-row"><input type="checkbox" data-act="layer" data-arg="${k}" ${state.layers.includes(k) ? 'checked' : ''}><i class="swatch" style="background:${layerColor(k)}"></i><span>${layerName(k)}</span><b>${state.items.filter(a => reportLayer(a) === k).length}</b></label>`).join('')}<label class="check"><input type="checkbox" data-model="highOnly" ${state.highOnly ? 'checked' : ''}>${L('High priority only', 'Priorité élevée uniquement')}</label></div>
      ${geologyPanel()}
      <div class="pane-section"><h2>${L('Window', 'Période')}</h2><div class="seg" style="width:max-content">${[0, 1, 7, 30].map(n => `<button class="${state.windowDays === n ? 'active' : ''}" data-act="window" data-arg="${n}">${n ? n === 1 ? '24 h' : `${n} ${L('d', 'j')}` : L('All', 'Tout')}</button>`).join('')}</div></div>
      <div class="pane-section"><h2>${L('Hotspots', 'Points chauds')}</h2>${placeRanking(ps, 8)}</div>
      <div class="pane-section"><h2>${L('Collection', 'Collecte')}</h2>${feedStatuses().map(([c, label, v]) => `<div class="feed-status" style="padding:0"><i class="dot ${c}"></i>${esc(label)}<small>${esc(v)}</small></div>`).join('')}<div class="actions">${btnCollect()}</div></div>
    </aside>
    <section class="stage"><div class="map-area">${mapSVG(ps)}
      <div class="hud" aria-hidden="true"><i class="hud-c tl"></i><i class="hud-c tr"></i><i class="hud-c bl"></i><i class="hud-c br"></i><i class="hud-reticle"></i></div>
      <div class="map-float hud-pos glass hud-readout" id="hud-coords" aria-hidden="true">${hudText(state.hud)}</div>
      <div class="map-float tl glass"><strong>${state.place ? esc(state.place) : L('Democratic Republic of the Congo', 'République démocratique du Congo')}</strong>${nf(ps.reduce((n, p) => n + p.count, 0))} ${L('place mentions', 'lieux cités')} · ${nf(items.filter(a => !a.places.length).length)} ${L('unmapped', 'non cartographiés')}</div>
      <div class="map-float tr"><button class="btn sm glass" data-act="map-focus" aria-pressed="${state.mapFocus}">${state.mapFocus ? L('Show panels','Afficher les panneaux') : L('Expand map','Agrandir')}</button><div class="seg glass" style="padding:2px"><button class="${state.basemap === 'base' ? 'active' : ''}" data-act="base" data-arg="base">${L('Map', 'Carte')}</button><button class="${state.basemap === 'satellite' ? 'active' : ''}" data-act="base" data-arg="satellite">${L('Satellite', 'Satellite')}</button></div><button class="btn sm glass" data-act="tilt">${state.tilt ? L('Flat', 'À plat') : L('3D tilt', 'Inclinée')}</button><button class="btn sm glass" data-act="reset-map">${icon('globe')}${L('Reset', 'Recentrer')}</button></div>
      <div class="map-float bl glass">${mapFailed ? L('Interactive map unavailable; showing outline.', 'Carte interactive indisponible ; contour affiché.') : L('Markers are approximate place mentions, not verified incident locations.', 'Les repères sont des lieux cités approximatifs, pas des incidents vérifiés.')}${state.basemap === 'satellite' ? ` ${L('Imagery is not live.', 'Imagerie non en direct.')}` : ''}</div>
    </div><div class="timeline">${timeline}</div></section>
    <aside class="rail-pane">${mineralInspector()}<div class="rail-head"><div class="eyebrow">${state.place ? esc(state.place) : L('Matching reports', 'Articles de la sélection')}</div><div style="display:flex;gap:8px"><select class="select" data-model="sort" style="flex:1" aria-label="${L('Sort', 'Tri')}"><option value="priority" ${state.sort === 'priority' ? 'selected' : ''}>${L('Priority', 'Priorité')}</option><option value="latest" ${state.sort === 'latest' ? 'selected' : ''}>${L('Newest', 'Plus récents')}</option></select><button class="iconbtn" data-act="export" aria-label="${L('Export JSON', 'Exporter JSON')}" title="${L('Export JSON', 'Exporter JSON')}">${icon('download')}</button></div>${state.place || state.selectedDay ? `<div class="chips"><button class="chip active" data-act="clear-situation">${esc([state.place, state.selectedDay && dayLabel(state.selectedDay)].filter(Boolean).join(' · '))} ×</button></div>` : ''}${state.error ? `<div class="errorbar" style="margin:0">${esc(state.error)}</div>` : ''}</div>
      <div class="rail-list compact report-list" data-scroll="rail">${items.slice(0, state.visible).map(a => reportRow(a, true)).join('') || emptyState(L('No reports', 'Aucun article'), L('Select a layer or widen the window.', 'Sélectionnez une couche ou élargissez la période.'))}${items.length > state.visible ? `<div class="loadmore"><button class="btn sm" data-act="more">${L('Load more', 'Charger plus')}</button></div>` : ''}</div>
      <div class="rail-foot"><span>${L('Single source · unverified', 'Source unique · non corroboré')}</span><span class="num">${nf(items.length)}</span></div></aside>
  </div>`;
}
function mapSVG(ps) {
  if (state.config?.mapbox_public_token && window.mapboxgl && !mapFailed) return '<div id="map-slot"></div>';
  const xy = (lon, lat) => [25 + (lon - 12) * 18, 25 + (5.5 - lat) * 13.8];
  const path = state.geometry ? state.geometry.features[0].geometry.coordinates.map(r => r.map((p, i) => `${i ? 'L' : 'M'}${xy(...p).join(',')}`).join(' ') + 'Z').join(' ') : '';
  return `<div class="static-map"><svg viewBox="0 0 405 290" role="img" aria-label="${L('Map of place mentions', 'Carte des lieux cités')}"><path class="map-country" d="${path}"/>${mineralFeatures().map(f => { const [x,y] = xy(...f.geometry.coordinates); return `<circle data-act="mineral-select" data-arg="${esc(f.id)}" role="button" aria-label="${esc(f.properties.name)}" cx="${x}" cy="${y}" r="${state.mineralSelected === f.id ? 3 : 1.2}" fill="${f.properties.dataset === 'IPIS' ? '#d9a441' : '#9b8afb'}"><title>${esc(f.properties.name)} · ${esc(f.properties.minerals.join(' · '))} · ${esc(f.properties.date || '2021 compilation')} · ${esc(f.properties.dataset)}</title></circle>` }).join('')}${ps.map(p => { const [x, y] = xy(p.lon, p.lat); return `<g class="map-place ${state.place === p.name ? 'active' : ''}" data-act="place" data-arg="${esc(p.name)}" tabindex="0" role="button" aria-label="${esc(p.name)}, ${p.count}"><title>${esc(p.name)} · ${p.count}</title><circle cx="${x}" cy="${y}" r="${Math.min(4 + Math.sqrt(p.count), 10)}"/><text x="${x + 9}" y="${y - 7}">${esc(p.name)}</text></g>` }).join('')}</svg></div>`;
}
// Preserve one map instance through re-renders.
let mapHost = null, mapInstance = null, mapMarkers = [], mapFailed = false, markerSignature = '';
/// Cursor position and camera readout for the map HUD.
function hudText(h = {}) {
  const deg = (v, pos, neg, w) => v == null ? '—'.padEnd(w + 7) : `${Math.abs(v).toFixed(4).padStart(w + 5, '0')}°${v >= 0 ? pos : neg}`;
  return `LAT ${deg(h.lat, 'N', 'S', 2)}  LON ${deg(h.lon, 'E', 'W', 3)}  Z ${(h.zoom ?? 4).toFixed(1)}  P ${Math.round(h.pitch ?? 0)}°`;
}
const mapStyle = () => 'mapbox://styles/mapbox/' + (state.basemap === 'satellite' ? 'satellite-streets-v12' : state.theme === 'light' ? 'light-v11' : 'dark-v11');
function mountMap() {
  const slot = $('#map-slot'); if (!slot) return;
  if (!mapHost) { mapHost = document.createElement('div'); mapHost.className = 'interactive-map' }
  slot.append(mapHost);
  if (!mapInstance) {
    try {
      mapInstance = new mapboxgl.Map({ container: mapHost, accessToken: state.config.mapbox_public_token, style: mapStyle(), bounds: [[12, -13.6], [31.4, 5.5]], fitBoundsOptions: { padding: 40 }, minZoom: 2, maxZoom: 16, attributionControl: true });
      mapInstance.__style = mapStyle();
      mapInstance.addControl(new mapboxgl.NavigationControl({ showCompass: true }), 'top-right');
      mapInstance.addControl(new mapboxgl.ScaleControl({unit:'metric'}), 'bottom-right');
      const readout = () => { const el = $('#hud-coords'); if (el) el.textContent = hudText(state.hud) };
      mapInstance.on('mousemove', e => { state.hud = { ...state.hud, lat: e.lngLat.lat, lon: e.lngLat.lng }; readout() });
      mapInstance.on('move', () => { state.hud = { ...state.hud, zoom: mapInstance.getZoom(), pitch: mapInstance.getPitch() }; readout() });
      mapInstance.on('error', () => { if (!mapInstance?.isStyleLoaded()) { mapFailed = true; mapInstance?.remove(); mapInstance = null; mapHost = null; render() } });
      mapInstance.on('idle', mountMinerals);
      mapInstance.on('click', 'mineral-sites', mineralPopup);
      mapInstance.on('click', 'mineral-clusters', mineralCluster);
      for (const layer of ['mineral-sites','mineral-clusters']) {
        mapInstance.on('mouseenter',layer,()=>{ mapInstance.getCanvas().style.cursor='pointer' });
        mapInstance.on('mouseleave',layer,()=>{ mapInstance.getCanvas().style.cursor='' });
      }
      mapInstance.on('style.load', () => {
        mountMinerals();
        if (state.geometry && !mapInstance.getSource('drc-outline')) {
          const accent = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() || '#c8dd91';
          mapInstance.addSource('drc-outline', { type: 'geojson', data: state.geometry });
          mapInstance.addLayer({ id: 'drc-fill', type: 'fill', source: 'drc-outline', paint: { 'fill-color': accent, 'fill-opacity': 0.04 } });
          mapInstance.addLayer({ id: 'drc-border', type: 'line', source: 'drc-outline', paint: { 'line-color': accent, 'line-width': 1.6, 'line-opacity': 0.7 } });
        }
      });
    } catch { mapFailed = true; mapInstance?.remove(); mapInstance = null; mapHost = null; render(); return }
  }
  if (mapInstance.__style !== mapStyle()) { mapInstance.__style = mapStyle(); mapInstance.setStyle(mapStyle()) }
  requestAnimationFrame(() => mapInstance?.resize());
  mountMinerals();
  const linked = situationItems(true);
  const places = placesList(linked);
  const signature = JSON.stringify([places, state.lang, state.place, state.layers]);
  if (signature === markerSignature) return;
  markerSignature = signature;
  mapMarkers.forEach(m => m.remove()); mapMarkers = [];
  for (const p of places) {
    const b = document.createElement('button');
    const hot = linked.some(a => a.rank.band === 'high' && a.places.some(x => x.name === p.name));
    b.className = 'geo-marker' + (state.place === p.name ? ' selected' : '') + (hot ? ' hot' : '');
    b.textContent = p.count;
    const dominant = countBy(linked.filter(a => a.places.some(x => x.name === p.name)), reportLayer)[0]?.[0];
    b.style.background = layerColor(dominant || 'other');
    b.setAttribute('aria-label', `${p.name}, ${p.count} ${L('reports', 'articles')}`);
    b.dataset.tip = `${p.name} · ${p.province} · ${p.count} ${L('reports', 'articles')}`;
    b.onclick = () => selectPlace(p.name);
    mapMarkers.push(new mapboxgl.Marker({ element: b }).setLngLat([p.lon, p.lat]).addTo(mapInstance));
  }
}
function selectPlace(name) {
  state.place = state.place === name && state.view === 'situation' ? '' : name;
  state.visible = 20;
  if (!['situation', 'reports', 'saved'].includes(state.view)) { go('situation') } else render();
  const p = placesList().find(p => p.name === state.place);
  if (p) mapInstance?.flyTo({ center: [p.lon, p.lat], zoom: 6, duration: 600 });
}

// ---------------------------------------------------------------------------------------------
// Primary sources

// Agreement explorer: coverage is not a performance score.
const agreementStatus=k=>({needs_review:L('Signature unverified','Signature non vérifiée'),reported_signed:L('Signing reported','Signature annoncée'),not_requested:L('Not analyzed','Non analysé'),running:L('Analyzing','Analyse en cours'),completed:L('AI draft ready','Brouillon IA prêt'),stale:L('Source changed','Source modifiée'),failed:L('Analysis failed','Échec de l’analyse'),available:L('Text available','Texte disponible'),unavailable:L('Unavailable','Indisponible'),ocr_required:L('OCR needed','Reconnaissance de texte requise')})[k]||k;
const agreementRole=k=>({agreement:L('Agreement text','Texte de l’accord'),historical_context:L('Historical context','Contexte historique'),implementation_report:L('Implementation evidence','Preuves de mise en œuvre'),official_announcement:L('Official announcement—not executed text','Annonce officielle — pas le texte signé'),project_document:L('Project document—not executed text','Document de projet — pas le texte signé')})[k]||k;
const AGREEMENT_DIMS=['security','revenue','services','sovereignty','accountability'];
const dimName=k=>({security:L('Security','Sécurité'),revenue:L('Revenue','Recettes'),services:L('Services','Services'),sovereignty:L('Sovereignty','Souveraineté'),accountability:L('Accountability','Redevabilité')})[k];
function agreementRows() {return (state.agreements?.items||[]).filter(i=>(!state.agCategory||i.categories.includes(state.agCategory))&&(!state.agTerm||i.signed_term===state.agTerm)&&(!state.agSearch||norm(i.title+' '+i.parties).includes(norm(state.agSearch)))).sort((a,b)=>String(b.date||'').localeCompare(String(a.date||'')))}
const agreementScore=n=>n==null?'—':(n>0?'+':'')+Number(n).toFixed(1);
function teamView(){
 const roles=['owner','administrator','analyst','reviewer','viewer'];const owner=state.auth?.user?.role==='owner';
 const form=(m={email:'',role:'viewer',active:true,daily_requests:0,daily_usd:0,monthly_usd:0})=>`<form class="panel" style="padding:20px;margin:12px 0" data-form="team"><div class="toolbar"><label>${L('Email','Adresse email')}<input class="input" name="email" value="${esc(m.email)}" ${m.email?'readonly':''} required></label><label>${L('Role','Rôle')}<select class="select" name="role">${(m.email==='worker'?['worker']:roles.filter(r=>owner||r!=='owner')).map(r=>`<option ${m.role===r?'selected':''}>${r}</option>`).join('')}</select></label>${[['daily_requests',L('Daily requests','Requêtes / jour')],['daily_usd',L('Daily USD allowance','Provision USD / jour')],['monthly_usd',L('Monthly USD allowance','Provision USD / mois')]].map(([k,l])=>`<label>${l}<input class="input" name="${k}" type="number" min="0" step="${k==='daily_requests'?1:0.01}" value="${m[k]}" ${owner?'':'readonly'}></label>`).join('')}<label><input name="active" type="checkbox" ${m.active?'checked':''}>${L('Active','Actif')}</label><button class="btn primary" type="submit">${m.email?L('Update member','Modifier le membre'):L('Invite member','Inviter un membre')}</button></div></form>`;
 return pageHead(L('WORKSPACE ACCESS','ACCÈS À L’ESPACE'),L('Team & access','Équipe et accès'),L('Invite-only membership. Notes and bookmarks are private to each user.','Accès sur invitation. Notes et favoris privés pour chaque utilisateur.'))+
 `<p class="panel">${state.authConfig?.mode==='oidc'?L('ZITADEL sign-in enabled. Share the site address with invited members; no invitation email is sent automatically. Role changes revoke existing sessions.','Connexion ZITADEL activée. Partagez l’adresse du site avec les membres invités ; aucun email automatique. Un changement révoque les sessions.'):L('Local mode: ZITADEL is not connected yet. Configure OIDC before inviting real users or publishing the site.','Mode local : ZITADEL n’est pas encore connecté. Configurez OIDC avant d’inviter des utilisateurs ou de publier le site.')}</p>`+
 (state.team?`<h2>${L('Invite a member','Inviter un membre')}</h2>${form()}<h2>${L('Members and background worker','Membres et worker')}</h2>${state.team.members.map(form).join('')}<h2>${L('Audit log','Journal d’audit')}</h2>${state.team.audit.map(e=>`<p>${esc(e.at)} · ${esc(e.action)} · ${esc(e.target)} · ${esc(e.user_id)}</p>`).join('')}`:`<p>${L('Only owners and administrators can manage members.','Seuls les propriétaires et administrateurs peuvent gérer les membres.')}</p>`);
}
function costsView(){
 const c=state.costs;
 return pageHead(L('API OPERATIONS','OPÉRATIONS API'),L('Costs & budgets','Coûts et budgets'),L('Persistent controls for Claude, OpenAI research and X. Periods reset in UTC.','Contrôles persistants pour Claude, la recherche OpenAI et X. Périodes en UTC.'))+
 `<p class="panel">${L('These are your planning allowances, not provider invoices. Set an allowance per request and daily/monthly limits before enabling. Actual charges may differ; configure billing limits with each provider too. Historical spend, Mapbox browser usage, hosting and other applications are excluded. Existing X limits also apply. Pausing affects new requests, not requests already sent.','Ces montants sont vos provisions, pas les factures des fournisseurs. Définissez une provision par requête et des limites journalières/mensuelles avant activation. Les frais réels peuvent différer ; configurez aussi les limites des fournisseurs. Historique, Mapbox du navigateur, hébergement et autres applications exclus. Les limites X existantes restent actives. La pause concerne les nouvelles requêtes.')}</p>`+
 (c?.items||[]).map(p=>`<section class="panel" style="padding:20px;margin-bottom:16px" id="cost-${p.provider}"><h2>${esc(p.provider)} · ${p.paused?L('Paused','En pause'):L('Enabled','Activé')}</h2><p>${p.requests_today}/${p.daily_requests} ${L('requests today','requêtes aujourd’hui')} · $${p.allocated_today_usd.toFixed(4)} / $${p.daily_usd.toFixed(2)} ${L('allocated today','provisionnés aujourd’hui')} · $${p.allocated_month_usd.toFixed(4)} / $${p.monthly_usd.toFixed(2)} ${L('this month','ce mois')}</p><div class="toolbar">${[['request_usd',L('USD allowance / request','Provision USD / requête')],['daily_usd',L('Daily USD limit','Limite USD / jour')],['monthly_usd',L('Monthly USD limit','Limite USD / mois')],['daily_requests',L('Daily request cap','Requêtes max / jour')]].map(([key,label])=>`<label>${label}<input class="input" name="${key}" type="number" min="0" step="${key==='daily_requests'?'1':'0.000001'}" value="${p[key]}"></label>`).join('')}<label><input name="paused" type="checkbox" ${p.paused?'checked':''}> ${L('Pause paid requests','Suspendre les requêtes payantes')}</label><button class="btn" data-act="cost-save" data-arg="${p.provider}">${L('Save limits','Enregistrer')}</button></div></section>`).join('')+
 `<section class="panel" style="padding:20px"><h2>${L('Recent request ledger','Journal récent des requêtes')}</h2><p>${L('Failed or uncertain requests keep their allocation. No costs are reconstructed from old activity.','Les requêtes en échec ou incertaines conservent leur provision. Aucun coût historique n’est reconstitué.')}</p><button class="btn" data-act="cost-export">JSON ↓</button>${(c?.events||[]).map(e=>`<p>${esc(e.at)} · ${esc(e.provider)} · ${esc(e.user_id||'legacy')} · ${esc(e.operation)} · $${e.allocated_usd.toFixed(6)} · ${esc(e.status)}</p>`).join('')||L('No metered requests yet.','Aucune requête comptabilisée.')}</section>`;
}
function agreementsView(){
 const rows=agreementRows(),terms=[...new Set((state.agreements?.items||[]).map(i=>i.signed_term))].sort();
 const detail=state.agDetail, a=detail?.assessment?.data;
 const counts=terms.map(term=>({term,items:rows.filter(i=>i.signed_term===term)}));
 return pageHead(L('AGREEMENTS & PUBLIC OUTCOMES','ACCORDS ET RÉSULTATS PUBLICS'),L('Agreements observatory','Observatoire des accords'),L('Peace, minerals and infrastructure: commitments, evidence and implementation. Initial register—not an exhaustive treaty inventory.','Paix, minerais et infrastructures : engagements, preuves et mise en œuvre. Registre initial, non exhaustif.'))+
 `<details class="panel"><summary>${L('Automatic agreement discovery — every 6 hours','Découverte automatique des accords — toutes les 6 heures')}</summary><p>${L('Configured source indexes only. New findings require signature and document verification.','Index des sources configurées uniquement. Les découvertes nécessitent une vérification des signatures et documents.')}</p>${(state.agreements?.discovery?.sources||[]).map(s=>`<p><strong>${esc(s.id)}</strong> · ${esc(s.checked_at||L('Awaiting first check','Premier contrôle en attente'))} ${s.error?' · '+esc(s.error):''}</p>`).join('')}<p>${state.agreements?.discovery?.pending||0} ${L('pending or failed documents','documents en attente ou en échec')}</p></details>
 <div class="toolbar"><input id="agreement-search" class="input" type="search" data-model="agSearch" value="${esc(state.agSearch)}" placeholder="${L('Search agreements or parties…','Chercher un accord ou une partie…')}" aria-label="${L('Search agreements','Rechercher des accords')}"><select class="select" data-model="agCategory" aria-label="${L('Sector','Secteur')}"><option value="">${L('All sectors','Tous les secteurs')}</option>${[['peace','Peace','Paix'],['minerals','Minerals','Minerais'],['infrastructure','Infrastructure','Infrastructures']].map(([k,en,fr])=>`<option value="${k}" ${state.agCategory===k?'selected':''}>${L(en,fr)}</option>`).join('')}</select><select class="select" data-model="agTerm" aria-label="${L('Signing administration','Mandat à la date de l’accord')}"><option value="">${L('All signing administrations','Tous les mandats')}</option>${terms.map(t=>`<option value="${esc(t)}" ${state.agTerm===t?'selected':''}>${esc(t)}</option>`).join('')}</select><button class="btn" data-act="agreement-export">JSON ↓</button></div>
 <div class="agreement-charts panel"><label class="check"><input type="checkbox" data-model="agDrafts" ${state.agDrafts?'checked':''}>${L('Include unreviewed AI drafts (provisional)','Inclure les brouillons IA non relus (provisoire)')}</label><h2>${L('Observed outcomes by signing administration','Résultats observés par mandat de signature')}</h2><p class="muted">${L('Reviewed drafts appear by default; the checkbox includes provisional AI drafts. −2 adverse → +2 beneficial; — means insufficient evidence. Means are qualitative, show sample sizes, and are not causal rankings of presidents. Later implementation can fall under another government.','Les analyses relues apparaissent par défaut ; la case ajoute les brouillons IA provisoires. −2 défavorable → +2 bénéfique ; — signifie preuves insuffisantes. Moyennes qualitatives avec effectifs, pas un classement causal des présidents. La mise en œuvre peut relever d’un autre gouvernement.')}</p><div class="table-scroll"><table class="agreement-table"><thead><tr><th>${L('Signing administration','Mandat')}</th><th>${L('Records','Références')}</th>${AGREEMENT_DIMS.map(d=>`<th>${dimName(d)}</th>`).join('')}</tr></thead><tbody>${counts.filter(c=>c.items.length).map(c=>`<tr><th>${esc(c.term)}</th><td>${c.items.length}</td>${AGREEMENT_DIMS.map(dim=>{const vals=c.items.filter(i=>i.assessment?.reviewed || (state.agDrafts && i.assessment?.status==='completed')).flatMap(i=>(i.assessment.data?.outcomes||[]).filter(s=>s.dimension===dim&&s.score!=null).map(s=>s.score));const mean=vals.length?vals.reduce((a,b)=>a+b,0)/vals.length:null;return `<td><span class="score-cell ${mean==null?'unknown':mean>0?'positive':mean<0?'negative':'neutral'}">${agreementScore(mean)}</span><small> n=${vals.length}</small></td>`}).join('')}</tr>`).join('')}</tbody></table></div><p>${nf(rows.length)} ${L('records in this selection. Counts include statements, amendments and unverified signatures.','références dans cette sélection. Comprend déclarations, avenants et signatures non vérifiées.')}</p></div>
 <div class="agreement-layout"><section class="panel agreement-list" aria-label="${L('Agreement list','Liste des accords')}">${rows.map(i=>`<button class="mineral-result ${detail?.id===i.id?'selected':''}" data-act="agreement-open" data-arg="${esc(i.id)}"><strong>${esc(i.title)}</strong><small>${esc(i.date||L('Date unverified','Date non vérifiée'))} · ${esc(i.publisher)}<br>${esc(agreementStatus(i.signature_status))} · ${esc(agreementStatus(i.assessment?.status||'not_requested'))} · ${i.documents.filter(d=>d.status==='available').length}/${i.documents.length} ${L('readable','lisibles')}</small></button>`).join('')||emptyState(L('No matching records','Aucune référence'),L('Broaden your filters.','Élargissez les filtres.'))}</section><section class="panel agreement-detail">${detail ? agreementDetail(detail,a) : emptyState(L('Select an agreement','Sélectionnez un accord'),L('Read its source, examine obligations, then request an AI assessment.','Lisez la source, examinez les obligations, puis demandez une analyse IA.'))}</section></div>
 <details class="panel" style="padding:16px"><summary>${L('Coverage, rights and methodology','Couverture, droits et méthode')}</summary><p>${esc(state.agreements?.metadata.coverage)}</p><p>${esc(state.agreements?.metadata.methodology)}</p><p>${esc(state.agreements?.metadata.pax_license)}</p><p>${L('State signatures, SOE contracts and local accords require separate verification. Date-based term assignment does not establish who negotiated, ratified or implemented an agreement. Scanned documents may require OCR.','Les signatures étatiques, contrats d’entreprises publiques et accords locaux nécessitent des vérifications distinctes. Le mandat associé à une date ne détermine pas qui a négocié, ratifié ou exécuté un accord. Les scans peuvent nécessiter une reconnaissance de texte.')}</p></details>`;
}
function agreementDetail(d,a){
 const bilingual=k=>a?.[k+'_'+state.lang]||a?.[k+'_en'];
 return `<h2>${esc(d.title)}</h2><p>${esc(d.date||L('Date unverified','Date non vérifiée'))} · ${esc(d.signed_term)} · ${esc(agreementStatus(d.signature_status))}</p><details><summary>${L('Parties and date verification','Parties et vérification de la date')}</summary><p>${esc(d.parties)}</p><p>${esc(d.date_basis)}</p>${d.date_source_url?`<a href="${esc(d.date_source_url)}" target="_blank" rel="noopener">${L('Date source','Source de la date')} ↗</a>`:''}</details>
 <div class="actions"><button class="btn primary" data-act="agreement-analyze" data-arg="${esc(d.id)}" ${['running','completed'].includes(d.assessment?.status)?'disabled':''}>${d.assessment?.status==='running'?L('Analyzing…','Analyse…'):a?L('Analysis saved','Analyse enregistrée'):L('Analyze with AI','Analyser avec IA')}</button><button class="btn" data-act="agreement-open" data-arg="${esc(d.id)}">${L('Refresh','Actualiser')}</button>${a&&!d.assessment.reviewed?`<button class="btn" data-act="agreement-review" data-arg="${esc(d.id)}">${L('I reviewed the evidence · include in charts','Preuves relues · inclure aux graphiques')}</button>`:''}</div>
 ${d.assessment?.error?`<p class="errorbar">${esc(d.assessment.error)}</p>`:''}
 ${a?`<h3>${d.assessment.reviewed?L('Human-reviewed assessment','Analyse relue'):L('AI draft · not included in charts','Brouillon IA · exclu des graphiques')}</h3><p>${esc(bilingual('summary'))}</p><ul>${(bilingual('commitments')||[]).map(c=>`<li>${esc(c)}</li>`).join('')}</ul>${['design','outcomes'].map(section=>`<h3>${section==='design'?L('Institutional potential—not delivered results','Potentiel institutionnel — pas des résultats livrés'):L('Observed effectiveness','Efficacité observée')}</h3><div class="agreement-scores">${(a[section]||[]).map(s=>`<details><summary><span>${dimName(s.dimension)}</span><b class="score-cell ${s.score==null?'unknown':s.score>0?'positive':s.score<0?'negative':'neutral'}">${agreementScore(s.score)}</b></summary><p>${esc(s['rationale_'+state.lang]||s.rationale_en)}</p>${s.quote && s.quote_verified?`<blockquote>${esc(s.quote)}</blockquote><small>${esc(s.document_id)}</small>`:''}${s.validation_note?`<p>${esc(s.validation_note)}</p>`:''}</details>`).join('')}</div>`).join('')}<p class="muted">${esc(bilingual('limitations'))}</p><small>${esc(a.model)} · ${esc(a.generated_at)} · ${esc(a.method_version)}</small>`:''}
 <h3>${L('Source documents','Documents sources')}</h3>${d.documents.map(doc=>`<details class="agreement-document"><summary>${esc(agreementRole(doc.role))} · ${esc(agreementStatus(doc.status))} ${doc.report_date?' · '+esc(doc.report_date):''} ${doc.extraction?'· OCR':''}</summary><p><a href="${esc(doc.url)}" target="_blank" rel="noopener">${L('Original source','Source originale')} ↗</a> ${doc.format==='pdf'&&doc.status!=='unavailable'?`<button class="btn sm" data-act="agreement-pdf" data-arg="${esc(d.id+'|'+doc.id)}">${L('Read PDF here','Lire le PDF ici')}</button>`:''}</p><small>SHA-256: ${esc(doc.sha256||'—')}</small><pre class="agreement-text">${esc(doc.text||L('No extracted text. Scan requires OCR or source is unavailable.','Pas de texte extrait. Scan à reconnaître ou source indisponible.'))}</pre></details>`).join('')}`;
}
async function openAgreement(id){state.agDetail=await api('/v1/agreements/'+encodeURIComponent(id));render()}
async function agreementPdf(arg){
 const [id,doc]=arg.split('|'), token=sessionStorage.getItem('congo-token');
 const r=await fetch(`/v1/agreements/${encodeURIComponent(id)}/documents/${encodeURIComponent(doc)}/pdf`,{headers:token?{Authorization:'Bearer '+token}:{}});
 if(!r.ok)throw Error(L('PDF unavailable locally; use original source.','PDF indisponible localement ; utilisez la source originale.'));
 closeOverlay();evidenceBlob=URL.createObjectURL(await r.blob());state.modal='agreement-pdf';
 $('#overlay').innerHTML=`<div class="shade" data-close><section class="drawer" style="width:95vw" role="dialog" aria-modal="true" aria-label="PDF"><div class="drawer-top"><strong>${L('Agreement source','Source de l’accord')}</strong><button class="btn" data-act="close">${L('Close','Fermer')}</button></div><iframe src="${evidenceBlob}" title="${L('Source PDF','PDF source')}" style="border:0;width:100%;height:85vh"></iframe></section></div>`;
 $('#overlay button')?.focus();
}


function sourcesView() {
  const q = norm(state.evQuery);
  const docs = (state.ev?.items || []).filter(d => (state.evRegional || d.relevance.scope === 'drc') && (!q || norm(d.source.title + ' ' + d.excerpt + ' ' + JSON.stringify(d.brief.data)).includes(q)));
  const groups = new Map();
  for (const s of (state.sources?.items || []).filter(s => !s.id.startsWith('discovered-'))) groups.set(s.publisher, [...(groups.get(s.publisher) || []), s]);
  const statusName = s => ({ ready: L('Collected', 'Collecté'), failed: L('Unavailable', 'Indisponible'), pending: L('Queued', 'En attente'), running: L('Collecting', 'Collecte en cours'), not_requested: L('Not checked', 'Non vérifié'), reference_only: L('Reference only', 'Référence uniquement') })[s] || s;
  const dotFor = s => s === 'ready' ? 'ok' : s === 'failed' ? 'bad' : 'warn';
  const aiButton = d => `<button class="btn sm ${d.brief.status === 'completed' ? '' : 'primary'}" data-act="evidence-ai" data-arg="${esc(d.id)}" ${['pending', 'running', 'completed'].includes(d.brief.status) ? 'disabled' : ''}>${icon('spark')}${d.brief.status === 'completed' ? L('EN + FR ready', 'EN + FR disponibles') : ['pending', 'running'].includes(d.brief.status) ? L('AI working', 'IA en cours') : d.brief.status === 'failed' ? L('Retry', 'Réessayer') : L('Bilingual brief', 'Synthèse bilingue')}</button>`;
  return pageHead(L('PRIMARY SOURCE MONITORING', 'VEILLE DES SOURCES PRIMAIRES'), L('Primary sources', 'Sources primaires'), L('UN expert reports, NGO investigations and official statements — archived with hashes, readable in place.', 'Rapports d’experts de l’ONU, enquêtes d’ONG et déclarations officielles — archivés avec empreintes, lisibles sur place.'), `<span class="badge">${state.ev?.schedule_seconds ? L(`Checked every ${state.ev.schedule_seconds / 3600} h`, `Vérifié toutes les ${state.ev.schedule_seconds / 3600} h`) : L('Scheduled checks off', 'Vérifications désactivées')}</span>`)
    + `<div class="split"><section class="panel"><div class="toolbar"><label class="search">${icon('search')}<input id="ev-q" class="input" type="search" data-model="evQuery" value="${esc(state.evQuery)}" placeholder="${L('Search documents…', 'Rechercher des documents…')}" aria-label="${L('Search documents', 'Rechercher des documents')}"></label><label class="check"><input type="checkbox" data-model="evRegional" ${state.evRegional ? 'checked' : ''}>${L('Include regional context', 'Inclure le contexte régional')}</label></div>
      <div class="list-meta"><span>${nf(docs.length)} ${L('documents', 'documents')}</span><span>${L('Relevance screening is not verification', 'Le filtrage de pertinence n’est pas une vérification')}</span></div>
      ${docs.map(d => `<article class="doc"><div class="report-kicker"><span class="badge ${d.relevance.scope === 'drc' ? 'accent' : ''}">${d.relevance.scope === 'drc' ? 'DRC / RDC' : L('Regional', 'Régional')}</span><span>${esc(d.source.publisher)}</span><span class="sep">·</span><span>${L('Source date', 'Date source')} ${esc(d.source.reference_date || '—')}</span><span class="sep">·</span><span>${L('Collected', 'Collecté')} ${date(d.fetched_at, true)}</span></div><h3>${esc(state.lang === 'en' && d.brief.data?.title_en ? d.brief.data.title_en : d.source.title)}</h3><p>${esc(d.brief.data?.['summary_' + state.lang] || d.excerpt)}</p>${d.relevance.matched_terms?.length ? `<div class="muted" style="font-size:11.5px">${L('Matched terms', 'Termes détectés')}: ${esc(d.relevance.matched_terms.join(', '))}</div>` : ''}<div class="actions"><button class="btn sm" data-act="evidence-open" data-arg="${esc(d.id)}">${icon('book')}${d.source.format === 'pdf' ? L('Read PDF here', 'Lire le PDF ici') : L('Read here', 'Lire ici')}</button><a class="btn sm" href="${esc(d.source.url)}" target="_blank" rel="noopener noreferrer">${icon('external')}${L('Original', 'Original')}</a>${aiButton(d)}</div>${d.brief.error ? `<p class="ev-error">${esc(d.brief.error)}</p>` : ''}</article>`).join('') || emptyState(L('No documents match', 'Aucun document'), L('Collection may still be running.', 'La collecte peut être en cours.'))}
    </section><div class="stack"><section class="panel"><div class="panel-head"><h2>${icon('layers')}${L('News sources', 'Sources d’actualité')}</h2></div>
      <div class="source-row"><a href="https://actualite.cd/" target="_blank" rel="noopener noreferrer">Actualite.cd</a><span class="feed-status" style="padding:0"><i class="dot ${state.error ? 'bad' : 'ok'}"></i>${L('Full articles · site crawler', 'Articles complets · collecteur')}<small>${nf(state.items.filter(a => a.source === 'Actualite.cd').length)} ${L('loaded', 'chargés')}</small></span></div>
      ${(state.feeds?.items || []).map(f => `<div class="source-row"><a href="https://${esc(f.site)}/" target="_blank" rel="noopener noreferrer">${esc(f.name)}</a><span class="feed-status" style="padding:0"><i class="dot ${!f.sync ? 'warn' : f.sync.status === 'ok' ? 'ok' : 'bad'}"></i>${L('Official feed · headline + lede', 'Flux officiel · titre + chapeau')}<small>${nf(f.collected)} ${L('collected', 'collectés')}${f.sync?.checked_at ? ` · ${age(f.sync.checked_at)}` : ''}</small></span>${f.sync?.detail ? `<small class="muted">${L('Last poll', 'Dernier relevé')}: ${f.sync.detail.not_modified ? L('no change', 'inchangé') : `${f.sync.detail.items} ${L('items', 'éléments')} · ${f.sync.detail.drc} RDC · ${f.sync.detail.regional} ${L('regional', 'régional')}`}</small>` : ''}${f.sync?.error ? `<span class="ev-error">${esc(f.sync.error)}</span>` : ''}<small class="muted">${esc(f.access_note)}</small></div>`).join('')}
    </section><section class="panel"><div class="panel-head"><h2>${icon('shield')}${L('Collection status', 'État de la collecte')}</h2></div>
      ${[...groups].map(([pub, rows]) => `<div class="publisher">${esc(pub)}</div>${rows.map(s => `<div class="source-row"><a href="${esc(s.url)}" target="_blank" rel="noopener noreferrer">${esc(s.title)}</a><span class="feed-status" style="padding:0"><i class="dot ${dotFor(s.status)}"></i>${esc(statusName(s.status))}<small>${s.checked_at ? date(s.checked_at, true) : '—'}</small></span>${s.error ? `<span class="ev-error">${esc(s.error)}</span>` : ''}<div class="actions">${s.format === 'pdf' ? `<button class="btn sm" data-act="source-doc" data-arg="${esc(s.id)}">${L('Read PDF', 'Lire le PDF')}</button>` : ''}${s.enabled ? `<button class="btn sm ghost" data-act="source-refresh" data-arg="${esc(s.id)}">${icon('refresh')}${L('Check now', 'Vérifier')}</button>` : ''}</div></div>`).join('')}`).join('')}
    </section></div></div>`;
}

// ---------------------------------------------------------------------------------------------
// Methodology

const METHOD = {
  en: [
    ['Evidence before conclusions.', 'Congo Signal is a bilingual open-source research workspace for the DRC. It borrows source evaluation and report management ideas from OpenCTI and geographic exploration from Ushahidi. It is an original implementation, not a fork of either platform.'],
    ['Explainable priority, not a truth score', 'Priority combines the source category (5–35 points), impact-related words (up to 24), recency (up to 20), recognized DRC place mentions (6) and an optional AI impact assessment (up to 15). High is 65+, medium 40–64. Rankings apply to the loaded set; a mention or allegation is not an independently established event.'],
    ['Two languages, one source trail', 'The French article is preserved. The configured AI provider generates an English headline, English and French briefs and an impact explanation. Full translations are generated on demand with aligned paragraphs. Model, generation time, source fingerprint and status are retained. AI can make mistakes; a translation is not an additional source.'],
    ['Place mentions, not verified events', 'A gazetteer of about 6,000 DRC towns, villages, territories and Kinshasa communes (GeoNames, with Wikidata coordinates for a few missing conflict localities) matches capitalized place names. Small villages only match after a location cue such as “à”, “near” or “de”, and a name directly after another capitalized word is treated as a person’s name. Coordinates are approximate; a place may be mentioned as background, and no incident location is inferred.'],
    ['Sanctions and key figures', 'Profiles merge the DRC regimes of the UN Security Council 1533 list, the US OFAC SDN list (DRCONGO), the EU (COD) and the UK, refreshed daily, with curated public figures. Records are merged across lists by name and birth year; Wikidata supplies roles and Commons photos with their licenses. Identity-document numbers are never stored. Name matching across lists, to Wikidata and in mentions is automated and can confuse namesakes; every claim links to its official source.'],
    ['Arms and equipment', 'Weapon types are matched by keyword in news, X posts and document pages. Actors named in the same sentence are counted as co-mentions: a lead for review, never proof of possession, use or supply. Descriptions, origins and operators come from Wikidata; 3D models are representative Sketchfab models of the type. State-to-state transfers come from SIPRI exports you import.'],
    ['Social signals: attributed, never labeled', 'X topic counts cover the previous 24 complete hours and include reposts; topics overlap. Topic samples and analyst-requested account histories are archived only when a post matches a topic rule. Account dossiers show public profile facts, posting hours and topic mix of archived posts. No stance, affiliation, automation or coordination labels are inferred, and activity volume is not influence. Every X request is reserved against a daily cap before it is sent.'],
    ['Multiple news sources', 'Actualite.cd articles are collected in full. Jeune Afrique blocks automated page access and is often paywalled, so only its official feed is used: headline, lede, author and time, labeled “Feed summary” and linked to the original. Its DRC items are kept; Rwanda, Uganda and Burundi items are regional context shown on request. Several outlets reporting the same claim is not independent corroboration.'],
    ['Analyst-controlled review', 'Bookmarks, review status and notes are stored locally in SQLite. Marking a report reviewed does not certify its claims. No automatic credibility rating is assigned.'],
  ],
  fr: [
    ['Les sources avant les conclusions.', 'Congo Signal est un espace de recherche bilingue en sources ouvertes consacré à la RDC. Il s’inspire de l’évaluation des sources d’OpenCTI et de l’exploration géographique d’Ushahidi. Il s’agit d’une réalisation originale.'],
    ['Une priorité explicable, pas un score de vérité', 'La priorité combine la catégorie (5–35 points), les termes d’impact (jusqu’à 24), la récence (jusqu’à 20), les lieux de RDC reconnus (6) et une analyse d’impact IA facultative (jusqu’à 15). Élevé : 65+, moyen : 40–64. Une mention ou allégation ne constitue pas un événement établi.'],
    ['Deux langues, une même traçabilité', 'L’article français est conservé. Le service IA génère un titre anglais, des synthèses en français et en anglais et une explication d’impact. Les traductions intégrales sont produites à la demande. Le modèle, la date, l’empreinte source et le statut sont conservés. L’IA peut se tromper ; une traduction n’est pas une source supplémentaire.'],
    ['Des lieux cités, pas des incidents vérifiés', 'Un répertoire d’environ 6 000 villes, villages, territoires et communes de Kinshasa (GeoNames, complété par Wikidata) repère les noms de lieux écrits avec une majuscule. Les petits villages ne sont retenus qu’après un indice de lieu (« à », « près de », « de »), et un nom suivant directement un autre mot à majuscule est traité comme un nom de personne. Les coordonnées sont approximatives ; aucune position d’incident n’est déduite.'],
    ['Sanctions et personnalités', 'Les profils fusionnent les régimes RDC de la liste 1533 du Conseil de sécurité, de l’OFAC américain (DRCONGO), de l’UE (COD) et du Royaume-Uni, actualisés chaque jour, avec des personnalités publiques sélectionnées. Wikidata fournit fonctions et photos Commons avec leurs licences. Les numéros de documents d’identité ne sont jamais conservés. Les rapprochements de noms sont automatiques et peuvent confondre des homonymes ; chaque information renvoie à sa source officielle.'],
    ['Armes et équipements', 'Les types d’armes sont repérés par mots-clés dans les articles, publications X et pages de documents. Les acteurs cités dans la même phrase sont des co-mentions : une piste, jamais une preuve de possession, d’usage ou de fourniture. Descriptions, origines et utilisateurs proviennent de Wikidata ; les modèles 3D Sketchfab sont représentatifs du type. Les transferts entre États proviennent des exports SIPRI importés.'],
    ['Signaux sociaux : attribués, jamais étiquetés', 'Les comptages X couvrent les 24 dernières heures complètes et incluent les republications ; les thèmes se chevauchent. Les échantillons et historiques de comptes demandés par l’analyste ne sont archivés que si une publication correspond à un thème. Les dossiers affichent le profil public, les heures de publication et la répartition thématique. Aucune position, affiliation, automatisation ou coordination n’est déduite. Chaque requête X est réservée sur un plafond quotidien avant envoi.'],
    ['Plusieurs sources d’actualité', 'Les articles d’Actualite.cd sont collectés en intégralité. Jeune Afrique bloque l’accès automatisé et est souvent payant : seul son flux officiel est utilisé (titre, chapeau, auteur, date), signalé « Résumé du flux » et relié à l’original. Ses articles sur la RDC sont conservés ; ceux sur le Rwanda, l’Ouganda et le Burundi sont un contexte régional affiché sur demande. Plusieurs médias relayant une même affirmation ne constituent pas une corroboration indépendante.'],
    ['Un examen contrôlé par l’analyste', 'Les sauvegardes, statuts et notes sont conservés localement dans SQLite. Marquer un article comme examiné ne certifie pas ses affirmations. Aucune note automatique de crédibilité n’est attribuée.'],
  ],
};
function methodView() {
  const [[h, intro], ...rest] = METHOD[state.lang];
  return `<section class="panel method"><div class="eyebrow">CONGO SIGNAL · ${L('METHODOLOGY', 'MÉTHODOLOGIE')}</div><h2 style="margin-top:8px">${h}</h2><p>${intro}</p>${rest.map(([t, p]) => `<h3>${t}</h3><p>${p}</p>`).join('')}
    <h3>${L('References & attribution', 'Références et attribution')}</h3><p><a target="_blank" rel="noopener noreferrer" href="https://github.com/bilawalsidhu/gods-eye-view">God’s Eye View ↗</a><br><a target="_blank" rel="noopener noreferrer" href="https://docs.opencti.io/latest/usage/reliability-confidence/">OpenCTI — Reliability & confidence ↗</a><br><a target="_blank" rel="noopener noreferrer" href="https://docs.ushahidi.com/platform-user-manual/5.-modes-for-visualizing-and-managing-data-on-your-deployment">Ushahidi — Geographic report views ↗</a><br><a target="_blank" rel="noopener noreferrer" href="https://github.com/johan/world.geo.json">World GeoJSON / Natural Earth ↗</a><br><a target="_blank" rel="noopener noreferrer" href="https://docs.x.com/x-api/introduction">X API ↗</a><br><a target="_blank" rel="noopener noreferrer" href="https://www.geonames.org/">GeoNames (CC BY 4.0) ↗</a><br><a target="_blank" rel="noopener noreferrer" href="https://www.wikidata.org/">Wikidata (CC0) · Wikimedia Commons (per-image licenses) ↗</a><br><a target="_blank" rel="noopener noreferrer" href="https://main.un.org/securitycouncil/en/sanctions/1533">UN Security Council 1533 Committee ↗</a> · <a target="_blank" rel="noopener noreferrer" href="https://ofac.treasury.gov/">OFAC ↗</a> · <a target="_blank" rel="noopener noreferrer" href="https://www.sanctionsmap.eu/">EU ↗</a> · <a target="_blank" rel="noopener noreferrer" href="https://www.gov.uk/government/publications/the-uk-sanctions-list">UK (OGL v3) ↗</a><br><a target="_blank" rel="noopener noreferrer" href="https://sketchfab.com/">Sketchfab 3D models (per-model licenses) ↗</a> · <a target="_blank" rel="noopener noreferrer" href="https://www.sipri.org/databases/armstransfers">SIPRI Arms Transfers Database ↗</a></p></section>`;
}

// ---------------------------------------------------------------------------------------------
// Report detail drawer

async function openDetail(id) {
  if (state.items.find(a => a.id === id)?.record_type === 'evidence') { await openEvidence(id); return }
  state.detailId = id; state.detailTab = 'brief'; state.detail = null; state.modal = 'detail';
  $('#overlay').innerHTML = `<div class="shade" data-close><section class="drawer"><div class="drawer-top"><span class="muted">${L('Loading…', 'Chargement…')}</span><button class="iconbtn" data-act="close" aria-label="${L('Close', 'Fermer')}">${icon('close')}</button></div><div class="drawer-body"><div class="loading-line"></div></div></section></div>`;
  try { const data = await api(`/v1/intelligence/${id}`); if (state.detailId !== id) return; state.detail = data; renderDetail() }
  catch (e) { closeOverlay(); toast(e.message) }
}
function taskBox(task, info) {
  if (info.status === 'completed') return '';
  const active = ['pending', 'running'].includes(info.status);
  return `<div class="analysis">${active ? '<div class="loading-line"></div>' : ''}<p>${active ? L('AI is working…', 'Analyse IA en cours…') : info.status === 'failed' ? `${L('AI task failed', 'Échec de l’analyse IA')}: ${esc(info.error)}` : task === 'brief' ? L('This report has not been analyzed yet.', 'Cet article n’a pas encore été analysé.') : L('Generate a full English translation. Paragraphs stay aligned with the French source.', 'Générez une traduction anglaise intégrale, alignée sur la source française.')}</p>${!active ? `<button class="btn primary" data-act="ai" data-arg="${task}">${icon('spark')}${info.status === 'failed' ? L('Retry', 'Réessayer') : task === 'brief' ? L('Generate bilingual brief', 'Générer une synthèse bilingue') : L('Translate full article', 'Traduire l’article')}</button>` : ''}</div>`;
}
function renderDetail() {
  if (!state.detail) return;
  const d = state.detail, a = d.article, b = d.brief.status === 'completed' ? d.brief.data : null, item = state.items.find(x => x.id === a.id), tab = state.detailTab;
  const notice = L('AI-generated interpretation; verify against the original source. Priority indicates relevance for review, not factual certainty.', 'Interprétation générée par IA ; à vérifier avec la source. La priorité indique l’intérêt pour l’analyse, pas la certitude des faits.');
  let content = '';
  if (tab === 'brief') content = (a.coverage === 'feed_summary' ? `<div class="notice">${L('Brief based on the feed headline and lede only.', 'Synthèse fondée uniquement sur le titre et le chapeau du flux.')}</div>` : '') + taskBox('brief', d.brief) + (b ? `<div class="analysis"><h3>${icon('spark')}${L('AI brief', 'Synthèse IA')} · ${esc(d.brief.model)}</h3><p>${esc(b['summary_' + state.lang])}</p>${b['rationale_' + state.lang] ? `<p class="muted" style="font-size:13px;margin:0"><b>${L('Impact', 'Impact')} · ${esc(b.impact)}</b> — ${esc(b['rationale_' + state.lang])}</p>` : ''}</div><div class="notice">${notice}${b.input_truncated ? ' ' + L('Analysis limited to the first 14,000 characters.', 'Analyse limitée aux 14 000 premiers caractères.') : ''}</div>${b.actors?.length ? `<h3 style="font-size:12.5px;color:var(--text-2)">${L('Public actors mentioned', 'Acteurs publics mentionnés')}</h3><div class="actor-list">${b.actors.map(x => `<span>${esc(x)}</span>`).join('')}</div>` : ''}` : '')
    + `<h3 style="font-size:13px;margin-top:24px">${L('Why this report ranks here', 'Pourquoi cette priorité ?')} · <span class="mono">${item?.rank.score ?? '—'}/100</span></h3>${(item?.rank.reasons || []).map(r => `<div class="reason"><span>${esc(r[state.lang])}</span><b>+${r.points}</b></div>`).join('')}<div class="notice">${L('Single source · unverified', 'Source unique · non corroboré')}</div>`;
  const feedNote = a.coverage === 'feed_summary' ? `<div class="notice">${L(`${esc(a.source)} publishes only the headline and lede in its feed. Read the full article on the publisher’s site (it may require a subscription).`, `${esc(a.source)} ne publie que le titre et le chapeau dans son flux. Lisez l’article complet sur le site de l’éditeur (abonnement parfois requis).`)} <a href="${esc(a.url)}" target="_blank" rel="noopener noreferrer">${L('Open article', 'Ouvrir l’article')} ↗</a></div>` : '';
  if (tab === 'original') content = feedNote + `<div class="article-text" lang="${esc(a.language || 'fr')}">${a.paragraphs.map(p => `<p>${esc(p)}</p>`).join('')}</div>`;
  if (tab === 'english') content = taskBox('translation', d.translation) + (d.translation.status === 'completed' ? `<div class="notice">${notice} · ${esc(d.translation.model)}</div><div class="article-text" lang="en">${d.translation.data.paragraphs.map(p => `<p>${esc(p)}</p>`).join('')}</div>` : '');
  if (tab === 'evidence') content = `<div class="notice">${L('Single source · unverified', 'Source unique · non corroboré')}</div><dl class="props"><dt>${L('Source', 'Source')}</dt><dd><a href="${esc(a.url)}" target="_blank" rel="noopener noreferrer">${esc(a.url)}</a></dd><dt>${L('Byline', 'Signature')}</dt><dd>${esc(a.author || L('Not extracted', 'Non extraite'))} ${a.author_method ? `(${esc(a.author_method)})` : ''}</dd><dt>${L('Published', 'Publié')}</dt><dd>${esc(a.published_at || '—')}</dd><dt>${L('Collected', 'Collecté')}</dt><dd>${esc(a.fetched_at)}</dd><dt>${L('Content SHA-256', 'Empreinte SHA-256')}</dt><dd><code>${esc(a.content_sha256)}</code></dd><dt>HTML SHA-256</dt><dd><code>${esc(a.html_sha256)}</code></dd><dt>${L('Extractor', 'Extracteur')}</dt><dd>${esc(a.extractor_version)}</dd><dt>${L('AI brief', 'Synthèse IA')}</dt><dd>${esc(d.brief.status)} · ${esc(d.brief.model || '—')} · ${esc(d.brief.completed_at || '—')}</dd></dl>`;
  if (tab === 'notes') content = `<form data-form="notes" class="fields"><label>${L('Review status', 'Statut de lecture')}<select id="review">${[['unreviewed', L('Not reviewed', 'Non examiné')], ['reviewed', L('Reviewed', 'Examiné')], ['follow_up', L('Needs follow-up', 'À approfondir')]].map(([v, l]) => `<option value="${v}" ${d.analyst.review === v ? 'selected' : ''}>${l}</option>`).join('')}</select></label><label>${L('Your research notes', 'Vos notes de recherche')}<textarea id="note" maxlength="10000" placeholder="${L('Record observations, missing context, or sources to check…', 'Consignez vos observations, le contexte manquant ou les sources à consulter…')}">${esc(d.analyst.note)}</textarea></label><div><button class="btn primary" type="submit">${icon('check')}${L('Save notes', 'Enregistrer les notes')}</button></div></form>`;
  const scroller = $('#overlay .drawer'), top = scroller?.scrollTop || 0;
  const tabs = [['brief', L('AI brief', 'Synthèse IA')], ['original', L('Original · FR', 'Original · FR')], ['english', L('English translation', 'Traduction anglaise')], ['evidence', L('Provenance', 'Traçabilité')], ['notes', L('Analyst notes', 'Notes d’analyse')]];
  $('#overlay').innerHTML = `<div class="shade" data-close><section class="drawer" role="dialog" aria-modal="true" aria-labelledby="detail-title"><div class="drawer-top"><span class="eyebrow">${L('REPORT', 'ARTICLE')} · ${esc(a.source)}</span><div class="actions"><a class="btn sm" href="${esc(a.url)}" target="_blank" rel="noopener noreferrer">${icon('external')}${L('Original', 'Original')}</a><button class="btn sm" data-act="detail-save">${icon('bookmark')}${d.analyst.saved ? L('Saved', 'Sauvegardé') : L('Save', 'Sauvegarder')}</button><button class="iconbtn" data-act="close" aria-label="${L('Close', 'Fermer')}">${icon('close')}</button></div></div>
    <div class="drawer-body"><div class="report-kicker">${item ? `<span class="badge ${item.rank.band}">${bandLabel(item.rank.band)} · ${item.rank.score}</span>` : ''}<span class="tag"><i class="swatch" style="background:${layerColor(item ? reportLayer(item) : 'other')}"></i>${esc(categoryName(a.category))}</span>${(item?.places || []).map(p => `<span>${icon('pin')} ${esc(p.name)}</span>`).join('')}</div><h1 id="detail-title">${esc(state.lang === 'en' && b?.title_en ? b.title_en : a.title)}</h1><div class="drawer-meta">${date(a.published_at)} (Kinshasa) · ${L('Single source · unverified', 'Source unique · non corroboré')}</div>${namedActors('report', a.id).length ? `<div class="chips" style="margin-bottom:6px">${actorChips(namedActors('report', a.id), 12)}</div>` : ''}
    <div class="tabs" role="tablist">${tabs.map(([k, l]) => `<button data-act="tab" data-arg="${k}" class="${tab === k ? 'active' : ''}" role="tab" aria-selected="${tab === k}">${l}</button>`).join('')}</div><section role="tabpanel">${content}</section></div></section></div>`;
  const drawer = $('#overlay .drawer'); if (drawer) drawer.scrollTop = top;
  if (!scroller) $('#overlay [data-act="close"]')?.focus();
}

// ---------------------------------------------------------------------------------------------
// Evidence reader

let evidenceBlob = null, evidenceGeneration = 0;
function showEvidenceDialog(heading, body) {
  closeOverlay(); state.modal = 'evidence';
  $('#overlay').innerHTML = `<div class="shade" data-close><section class="drawer" role="dialog" aria-modal="true" aria-label="${esc(heading)}"><div class="drawer-top"><strong style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap">${esc(heading)}</strong><button class="iconbtn" data-act="close" aria-label="${L('Close', 'Fermer')}">${icon('close')}</button></div><div class="drawer-body">${body}</div></section></div>`;
  $('#overlay [data-act="close"]')?.focus();
}
async function openEvidence(id) {
  try {
    const d = await api(`/v1/evidence/${id}`), item = state.items.find(a => a.id === id);
    showEvidenceDialog(d.source.title, `<div class="notice">${esc(d.source.publisher)} · ${esc(d.source.reference_date || '—')}<br>${esc(d.source.notes)}</div>${item?.brief?.data ? `<section class="analysis"><h3>${icon('spark')}${L('AI brief', 'Synthèse IA')}</h3><p>${esc(item.brief.data['summary_' + state.lang])}</p></section>` : ''}${d.source.format === 'pdf' ? `<div id="pdf-status" class="muted" role="status">${L('Loading archived PDF…', 'Chargement du PDF archivé…')}</div><div id="pdf-view"></div>` : ''}<details ${d.source.format === 'pdf' ? '' : 'open'}><summary>${L('Extracted text and provenance', 'Texte extrait et provenance')}</summary><p class="mono muted" style="font-size:11px;word-break:break-all">SHA-256: ${esc(d.sha256)}</p><div class="article-text">${d.pages.map(p => `<section><h3 style="font-size:13px">${p.page ? 'Page ' + p.page : L('Original text', 'Texte original')}</h3><p style="white-space:pre-wrap">${esc(p.text)}</p></section>`).join('')}</div></details>`);
    if (d.source.format === 'pdf') {
      const generation = evidenceGeneration, token = sessionStorage.getItem('congo-token');
      const r = await fetch(`/v1/evidence/${encodeURIComponent(id)}/pdf`, { headers: token ? { Authorization: `Bearer ${token}` } : {} });
      if (!r.ok) throw Error(L('The archived PDF could not be loaded. Extracted text is still available.', 'Le PDF archivé ne peut pas être chargé. Le texte extrait reste disponible.'));
      const blob = await r.blob();
      if (generation !== evidenceGeneration) return;
      evidenceBlob = URL.createObjectURL(blob);
      $('#pdf-view').innerHTML = `<iframe title="${esc(d.source.title)}" src="${evidenceBlob}#page=${d.source.pages?.[0] || 1}" class="pdf-frame"></iframe>`;
      $('#pdf-status').textContent = L('Archived original PDF.', 'PDF original archivé.');
    }
  } catch (e) { if ($('#pdf-status')) $('#pdf-status').textContent = e.message; else toast(e.message) }
}

// ---------------------------------------------------------------------------------------------
// Key figures & sanctions

const LIST_LABEL = { UN: 'UN', US: 'US', EU: 'EU', UK: 'UK' };
const groupName = id => { const g = (state.actors?.groups || state.arms?.groups || []).find(g => g.id === id); return g ? g[state.lang] : id };
const actorById = id => state.actors?.items?.find(a => a.id === id);
const actorGroups = a => [...new Set([...(a.groups || []), ...(a.designation_groups || [])])];
const actorDesc = a => a.description?.[state.lang] || a.description?.en || a.titles?.[0] || '';
const sanctionBadges = a => [...new Set((a.sanctions || []).map(s => s.list))].sort().map(l => `<span class="badge high" data-tip="${esc(L(`Listed by ${l}`, `Inscrit par ${l}`))}">${LIST_LABEL[l] || esc(l)}</span>`).join('');
const portrait = (a, size = '') => a.image?.thumb ? `<img class="portrait ${size}" src="${esc(a.image.thumb)}" alt="" loading="lazy" referrerpolicy="no-referrer">` : avatar(a.name, a.id, size === 'xl' ? 'lg' : size);
const webCredit = (img, kind, id) => {
  let domain = ''; try { domain = new URL(img.page || img.thumb).hostname.replace(/^www\./, '') } catch {}
  return `<div class="credit">${L('Found by web search', 'Trouvée par recherche web')} · ${img.page ? `<a href="${esc(img.page)}" target="_blank" rel="noopener noreferrer">${esc(domain)}</a>` : esc(domain)} · <span data-tip="${esc(img.status === 'verified' ? L('The source page was fetched and names this subject.', 'La page source a été vérifiée et cite ce sujet.') : L('The source page could not be fetched; the match is unconfirmed.', 'La page source n’a pas pu être vérifiée ; correspondance non confirmée.'))}">${img.status === 'verified' ? L('source page names subject ✓', 'page source vérifiée ✓') : L('unverified match', 'correspondance non vérifiée')}</span> · ${img.license ? `${img.artist ? esc(String(img.artist).slice(0, 60)) + ' · ' : ''}${img.license_url ? `<a href="${esc(img.license_url)}" target="_blank" rel="noopener noreferrer">${esc(img.license)}</a>` : esc(img.license)}` : L('rights not cleared', 'droits non vérifiés')} · <button class="linkbtn" data-act="reject-image" data-arg="${esc(kind)}:${esc(id)}">${L('Wrong image? Reject', 'Image erronée ? Rejeter')}</button></div>`;
};
const credit = (img, kind, id) => img?.source === 'web_search' ? webCredit(img, kind, id) : img ? `<div class="credit">${L('Image', 'Image')}: ${img.artist ? esc(img.artist.slice(0, 80)) + ' · ' : ''}${img.license_url ? `<a href="${esc(img.license_url)}" target="_blank" rel="noopener noreferrer">${esc(img.license || L('license', 'licence'))}</a>` : esc(img.license || '')} · <a href="${esc(img.page)}" target="_blank" rel="noopener noreferrer">Wikimedia Commons</a></div>` : '';
/// Actors named in a report or archived post, from the mention index.
const namedActors = (kind, id) => (state.actors?.index?.[`${kind}:${id}`] || []).map(actorById).filter(Boolean);
const actorChips = (list, max = 4) => list.slice(0, max).map(a => `<button class="chip actor-chip ${a.sanctioned ? 'sanctioned' : ''}" data-act="actor" data-arg="${esc(a.id)}" data-tip="${esc(a.sanctioned ? L('Sanctioned · ', 'Sanctionné · ') + [...new Set(a.sanctions.map(s => s.list))].join(', ') : actorDesc(a))}">${a.sanctioned ? icon('flag') : icon('user')}${esc(a.name)}</button>`).join('') + (list.length > max ? `<span class="muted" style="font-size:11.5px">+${list.length - max}</span>` : '');

function actorsView() {
  const d = state.actors, f = state.af, q = norm(f.q.trim());
  const all = d?.items || [];
  const items = all.filter(a => (f.kind === 'all' || (f.kind === 'sanctioned' && a.sanctioned) || (f.kind === 'curated' && a.curated) || (f.kind === 'person' && a.kind === 'person') || (f.kind === 'organization' && a.kind !== 'person'))
    && (!f.group || actorGroups(a).includes(f.group)) && (!q || norm([a.name, ...a.aliases, actorDesc(a), ...(a.titles || [])].join(' ')).includes(q)))
    .sort((a, b) => f.sort === 'name' ? a.name.localeCompare(b.name) : f.sort === 'listed' ? String(b.sanctions.map(s => s.listed_on).sort().at(-1)).localeCompare(String(a.sanctions.map(s => s.listed_on).sort().at(-1))) : ((b.mentions.recent || 0) - (a.mentions.recent || 0)) || ((b.mentions.total || 0) - (a.mentions.total || 0)) || (b.curated - a.curated));
  const groupCounts = countBy(all, actorGroups);
  const kinds = [['all', L('All', 'Tous'), all.length], ['sanctioned', L('Sanctioned', 'Sanctionnés'), all.filter(a => a.sanctioned).length], ['curated', L('Key figures', 'Personnalités clés'), all.filter(a => a.curated).length], ['person', L('People', 'Personnes'), all.filter(a => a.kind === 'person').length], ['organization', L('Organizations', 'Organisations'), all.filter(a => a.kind !== 'person').length]];
  return pageHead(L('SANCTIONS · KEY FIGURES', 'SANCTIONS · PERSONNALITÉS'), L('Key figures & sanctions', 'Acteurs et sanctions'), L('Who is who in the conflict: official UN, US, EU and UK sanctions designations merged with public profiles, and every place they appear in our sources.', 'Qui est qui dans le conflit : désignations officielles de l’ONU, des États-Unis, de l’UE et du Royaume-Uni, profils publics et chaque mention dans nos sources.'), `<button class="btn" data-act="refresh-reference">${icon('refresh')}${L('Refresh lists', 'Actualiser les listes')}</button>`)
    + `<section class="panel status-strip">${(d?.lists || []).map(l => `<span class="feed-status" style="padding:0" data-tip="${esc(`${l.label}\n${l.license}`)}"><i class="dot ${!l.sync ? 'warn' : l.sync.status === 'ok' ? 'ok' : 'bad'}"></i><b>${esc(l.id)}</b>&nbsp;${l.sync?.records ?? '—'} ${L('records', 'entrées')} · ${l.sync?.checked_at ? age(l.sync.checked_at) : L('pending', 'en attente')}</span>`).join('')}${d && !d.sync_enabled ? `<span class="badge">${L('Network sync disabled', 'Synchronisation désactivée')}</span>` : ''}<span style="margin-left:auto" class="muted">${nf(all.filter(a => a.sanctioned).length)} ${L('sanctioned profiles', 'profils sanctionnés')} · ${nf(all.filter(a => a.image).length)} ${L('with photos', 'avec photo')}</span></section>
    <section class="panel" style="margin-bottom:16px"><div class="toolbar"><div class="chips" style="width:100%">${kinds.map(([k, l, n]) => `<button class="chip ${f.kind === k ? 'active' : ''}" data-act="actor-kind" data-arg="${k}">${l} <b>${n}</b></button>`).join('')}</div>
      <label class="search">${icon('search')}<input id="actor-q" class="input" type="search" data-model="af.q" value="${esc(f.q)}" placeholder="${L('Search names, aliases, roles…', 'Rechercher noms, alias, fonctions…')}" aria-label="${L('Search actors', 'Rechercher')}"></label>
      <select class="select" data-model="af.group" aria-label="${L('Group', 'Groupe')}"><option value="">${L('All groups', 'Tous les groupes')}</option>${groupCounts.map(([g, n]) => `<option value="${esc(g)}" ${f.group === g ? 'selected' : ''}>${esc(groupName(g))} (${n})</option>`).join('')}</select>
      <select class="select" data-model="af.sort" aria-label="${L('Sort', 'Tri')}"><option value="mentions" ${f.sort === 'mentions' ? 'selected' : ''}>${L('Most mentioned', 'Plus mentionnés')}</option><option value="listed" ${f.sort === 'listed' ? 'selected' : ''}>${L('Recently listed', 'Inscrits récemment')}</option><option value="name" ${f.sort === 'name' ? 'selected' : ''}>${L('Name', 'Nom')}</option></select></div>
      <div class="list-meta"><span>${nf(items.length)} ${L('profiles', 'profils')}</span><span>${L('Mentions are text matches, not evidence of involvement', 'Une mention n’est pas une preuve d’implication')}</span></div>
      <div class="actor-grid">${items.slice(0, f.visible).map(actorCard).join('') || emptyState(L('No profiles match', 'Aucun profil'), d ? L('Try another filter.', 'Essayez un autre filtre.') : L('Sanctions lists are still syncing.', 'Les listes sont en cours de synchronisation.'))}</div>
      ${items.length > f.visible ? `<div class="loadmore"><button class="btn" data-act="actor-more">${L('Show more', 'Afficher plus')} (${items.length - f.visible})</button></div>` : ''}</section>
    <p class="notice">${esc(d?.notice || '')}</p>`;
}
function actorCard(a) {
  const m = a.mentions || {};
  return `<div class="panel actor-card" role="button" tabindex="0" data-act="actor" data-arg="${esc(a.id)}">${portrait(a)}
    <div class="actor-body"><div class="actor-badges">${sanctionBadges(a)}${a.curated ? `<span class="badge accent">${L('Key figure', 'Clé')}</span>` : ''}${a.kind !== 'person' ? `<span class="badge">${L('Org.', 'Org.')}</span>` : ''}</div>
    <h3>${esc(a.name)}</h3><p>${esc(actorDesc(a))}</p>
    <div class="actor-foot">${actorGroups(a).slice(0, 2).map(g => `<span class="tag">${esc(groupName(g))}</span>`).join('')}<span class="grow"></span>${m.total ? `<span class="num" data-tip="${esc(L(`${m.report || 0} reports · ${m.post || 0} X posts · ${m.document || 0} document pages`, `${m.report || 0} articles · ${m.post || 0} publications X · ${m.document || 0} pages de documents`))}">${icon('activity')} ${nf(m.total)}${m.recent ? ` <b class="hot">+${m.recent}</b>` : ''}</span>` : ''}</div></div></div>`;
}
async function openActor(id) {
  state.modal = 'actor'; state.actor = { id, data: null, kind: '' };
  if (location.hash !== '#actor/' + id) history.replaceState(null, '', '#actor/' + encodeURIComponent(id));
  renderActor();
  try { state.actor.data = await api(`/v1/actors/${encodeURIComponent(id)}`) } catch (e) { state.actor.error = e.message }
  if (state.actor?.id === id) renderActor();
}
function factRow(label, value) { return value ? `<dt>${label}</dt><dd>${value}</dd>` : '' }
// Entries whose Wikidata label is missing are skipped rather than shown as raw IDs.
const labelList = list => (list || []).map(x => x.label?.[state.lang] || x.label?.en).filter(Boolean).map(esc).join(', ');
function mentionList(mentions, terms, kindFilter, withGroups = false) {
  const kindIcon = { report: 'layers', post: 'radio', document: 'book' };
  const rows = mentions.filter(m => !kindFilter || m.kind === kindFilter);
  return rows.slice(0, 120).map(m => `<article class="mention"><div class="report-kicker">${icon(kindIcon[m.kind] || 'layers')}<span>${esc(m.kind === 'post' ? L('X post', 'Publication X') : m.kind === 'document' ? L('Document', 'Document') : L('Report', 'Article'))}</span><span class="sep">·</span><span>${m.at ? date(m.at, true) : '—'}</span>${m.page ? `<span class="sep">·</span><b>p. ${m.page}</b>` : ''}${m.hits > 1 ? `<span class="sep">·</span><span>${m.hits}×</span>` : ''}</div>
    <h4>${esc(m.title || '')}</h4><p class="post-text" style="margin:4px 0 6px">${highlight(m.snippet || '', terms)}</p>
    <div class="actions">${withGroups && Object.keys(m.groups || {}).length ? `<span class="muted" style="font-size:11.5px">${L('Same sentence', 'Même phrase')}:</span>${Object.keys(m.groups).map(g => `<span class="tag">${esc(groupName(g))}</span>`).join('')}<span style="flex:1"></span>` : ''}${m.kind === 'report' ? `<button class="btn sm" data-act="report" data-arg="${esc(m.ref)}">${L('Open report', 'Ouvrir')}</button>` : m.kind === 'document' ? `<button class="btn sm" data-act="evidence-open" data-arg="${esc(m.ref)}">${L('Open document', 'Ouvrir le document')}</button>` : `<button class="btn sm" data-act="account" data-arg="${esc((m.title || '').replace('@', ''))}">${L('Account dossier', 'Dossier')}</button>`}${m.url ? `<a class="btn sm ghost" href="${esc(m.url)}" target="_blank" rel="noopener noreferrer">${icon('external')}${L('Source', 'Source')}</a>` : ''}</div></article>`).join('') || `<div class="empty">${L('No mentions in collected sources yet.', 'Aucune mention dans les sources collectées.')}</div>`;
}
function mentionTimeline(mentions) {
  const months = countBy(mentions.filter(m => m.at && m.kind !== 'document'), m => m.at.slice(0, 7)).sort((a, b) => a[0].localeCompare(b[0])).slice(-18);
  return months.length > 1 ? `${bars(months.map(([m, n]) => ({ value: n, tip: `${m} · ${n}` })))}<div class="axis"><span>${months[0][0]}</span><span>${months.at(-1)[0]}</span></div>` : '';
}
function renderActor() {
  const s = state.actor; if (!s) return;
  const a = s.data, w = a?.wikidata;
  let body;
  if (s.error) body = `<div class="errorbar">${esc(s.error)}</div>`;
  else if (!a) body = `<div class="loading-line"></div>`;
  else {
    const kinds = countBy(a.mentions, m => m.kind);
    const facts = [
      factRow(L('Born', 'Naissance'), w?.born ? esc(w.born) : (a.sanctions.find(r => r.dob?.length)?.dob || []).map(esc).join(' / ')),
      factRow(L('Birthplace', 'Lieu de naissance'), labelList(w?.birthplace) || (a.sanctions.find(r => r.pob?.length)?.pob || []).map(esc).join(' / ')),
      factRow(L('Nationality', 'Nationalité'), labelList(w?.citizenship) || [...new Set(a.sanctions.flatMap(r => r.nationality || []))].map(esc).join(', ')),
      factRow(L('Positions', 'Fonctions'), (w?.positions || []).map(p => `${esc(p.label?.[state.lang] || p.label?.en || p.id)}${p.start || p.end ? ` <span class="muted">(${esc((p.start || '').slice(0, 4))}–${esc((p.end || '').slice(0, 4))})</span>` : ''}`).join('<br>')),
      factRow(L('Military', 'Armée'), [labelList(w?.military_branch), labelList(w?.rank)].filter(Boolean).join(' · ')),
      factRow(L('Party', 'Parti'), labelList(w?.parties)),
      factRow(L('Member of', 'Membre de'), [labelList(w?.member_of), labelList(w?.affiliations)].filter(Boolean).join(', ')),
      factRow(L('Founded', 'Fondation'), w?.inception ? esc(w.inception) : ''),
      factRow(L('Headquarters', 'Siège'), labelList(w?.headquarters)),
      factRow(L('Leaders', 'Dirigeants'), labelList(w?.leaders)),
      factRow(L('Titles on lists', 'Fonctions (listes)'), [...new Set(a.sanctions.flatMap(r => r.titles || []))].slice(0, 6).map(esc).join(' · ')),
    ].join('');
    body = `<div class="profile-head">${portrait(a, 'xl')}<div style="min-width:0"><div class="actor-badges">${sanctionBadges(a)}${a.curated ? `<span class="badge accent">${L('Key figure', 'Personnalité clé')}</span>` : ''}</div><h1 style="margin:6px 0 4px">${esc(a.name)}</h1><p class="dossier-bio" style="margin:0 0 8px">${esc(actorDesc(a))}</p>
      <div class="actions">${w?.wikipedia?.[state.lang] || w?.wikipedia?.en ? `<a class="btn sm" href="${esc(w.wikipedia[state.lang] || w.wikipedia.en)}" target="_blank" rel="noopener noreferrer">${icon('book')}Wikipedia</a>` : ''}${w ? `<a class="btn sm ghost" href="${esc(w.url)}" target="_blank" rel="noopener noreferrer">Wikidata ${esc(w.qid)}</a>` : ''}${w?.x_username ? `<button class="btn sm" data-act="account" data-arg="${esc(w.x_username)}">${icon('radio')}@${esc(w.x_username)}</button>` : ''}<a class="btn sm ghost" href="https://x.com/search?q=${encodeURIComponent('"' + a.name + '"')}&f=live" target="_blank" rel="noopener noreferrer">${L('Search X', 'Chercher sur X')} ↗</a></div>
      ${w?.match === 'name_title' ? `<p class="notice" style="margin:10px 0 0">${L('Profile matched to Wikidata by name and regional relevance. Verify it is the same person.', 'Profil associé à Wikidata par le nom et la pertinence régionale. Vérifiez qu’il s’agit de la même personne.')}</p>` : ''}${credit(a.image, 'actor', a.id)}</div></div>
      ${a.aliases?.length ? `<details style="margin:12px 0"><summary>${L('Aliases and spellings', 'Alias et graphies')} (${a.aliases.length})</summary><div class="cloud" style="margin-top:8px">${a.aliases.map(x => `<button disabled style="cursor:default;opacity:1">${esc(x)}</button>`).join('')}</div></details>` : ''}
      ${facts ? `<section class="panel" style="padding:14px 16px;margin:12px 0"><dl class="props">${facts}</dl></section>` : ''}
      ${a.sanctions.length ? `<h3 class="section-title">${icon('flag')}${L('Sanctions designations', 'Désignations de sanctions')}</h3>${a.sanctions.map(r => `<section class="panel sanction"><div class="report-kicker"><span class="badge high">${esc(r.list)}</span><b>${esc((a.lists || []).find(l => l.id === r.list)?.label || r.program)}</b><span class="sep">·</span><span class="mono">${esc(r.reference)}</span>${r.listed_on ? `<span class="sep">·</span><span>${L('Listed', 'Inscrit le')} ${esc(r.listed_on)}</span>` : ''}</div>${r.reason ? `<p>${esc(r.reason)}</p>` : ''}<div class="report-kicker">${r.dob?.length ? `<span>${L('DOB', 'Naissance')}: ${r.dob.map(esc).join(' / ')}</span>` : ''}${r.nationality?.length ? `<span>· ${r.nationality.map(esc).join(', ')}</span>` : ''}<span style="flex:1"></span><a href="${esc(r.url)}" target="_blank" rel="noopener noreferrer">${L('Official source', 'Source officielle')} ↗</a></div></section>`).join('')}` : ''}
      ${a.designation_groups?.length ? `<p class="muted" style="font-size:12.5px">${L('Groups named in designation text', 'Groupes cités dans la désignation')}: ${a.designation_groups.map(g => `<span class="tag" style="margin-right:8px">${esc(groupName(g))}</span>`).join('')}</p>` : ''}
      <h3 class="section-title">${icon('activity')}${L('Mentions in collected sources', 'Mentions dans les sources collectées')} <span class="muted num">${a.mentions.length}</span></h3>
      ${mentionTimeline(a.mentions)}
      <div class="chips" style="margin:10px 0"><button class="chip ${!s.kind ? 'active' : ''}" data-act="actor-mkind" data-arg="">${L('All', 'Tout')} <b>${a.mentions.length}</b></button>${kinds.map(([k, n]) => `<button class="chip ${s.kind === k ? 'active' : ''}" data-act="actor-mkind" data-arg="${k}">${k === 'report' ? L('Reports', 'Articles') : k === 'post' ? L('X posts', 'Publications X') : L('Documents', 'Documents')} <b>${n}</b></button>`).join('')}</div>
      <section class="panel">${mentionList(a.mentions, a.terms || [], s.kind)}</section>
      <p class="notice">${L('Matching uses names, aliases and listed terms; a mention can refer to a namesake. Sanctions are reproduced from official sources with attribution.', 'La recherche utilise noms, alias et termes associés ; une mention peut concerner un homonyme. Les sanctions sont reproduites depuis les sources officielles.')}</p>`;
  }
  const top = $('#overlay .drawer')?.scrollTop || 0, had = !!$('#overlay .drawer');
  $('#overlay').innerHTML = `<div class="shade" data-close><section class="drawer" role="dialog" aria-modal="true" aria-label="${esc(a?.name || L('Profile', 'Profil'))}"><div class="drawer-top"><span class="eyebrow">${icon('user')}${L('PROFILE', 'PROFIL')}</span><div class="actions">${a ? `<button class="btn sm" data-act="export-actor">${icon('download')}JSON</button>` : ''}<button class="iconbtn" data-act="close" aria-label="${L('Close', 'Fermer')}">${icon('close')}</button></div></div><div class="drawer-body">${body}</div></section></div>`;
  if (had) $('#overlay .drawer').scrollTop = top; else $('#overlay [data-act="close"]')?.focus();
}

// ---------------------------------------------------------------------------------------------
// Arms & equipment

const REGION_WORDS = ['congo', 'rwanda', 'uganda', 'ouganda', 'burundi', 'tanzania', 'tanzanie', 'south africa', 'afrique du sud', 'malawi', 'angola', 'kenya', 'zambia', 'zambie'];
const regionalLabel = l => REGION_WORDS.some(w => norm(l).includes(w));
const itemImage = (it, cls = '') => it.image?.thumb ? `<img class="${cls}" src="${esc(it.image.thumb)}" alt="" loading="lazy" referrerpolicy="no-referrer">` : it.model?.thumb ? `<img class="${cls}" src="${esc(it.model.thumb)}" alt="" loading="lazy" referrerpolicy="no-referrer">` : `<div class="${cls} placeholder">${icon('target')}</div>`;
function armsView() {
  const d = state.arms, f = state.wf, q = norm(f.q.trim());
  const all = d?.items || [];
  const items = all.filter(i => (!f.category || i.category === f.category) && (!q || norm([i.name, ...i.terms, i.description?.en, i.description?.fr].join(' ')).includes(q)))
    .sort((a, b) => f.sort === 'name' ? a.name.localeCompare(b.name) : (b.mentions?.total || 0) - (a.mentions?.total || 0));
  const catName = id => d?.categories?.find(c => c.id === id)?.[state.lang] || id;
  // Co-mention matrix: equipment rows x the groups most co-mentioned overall.
  const groupTotals = {};
  for (const i of all) for (const [g, n] of Object.entries(i.groups || {})) groupTotals[g] = (groupTotals[g] || 0) + n;
  const cols = Object.entries(groupTotals).sort((a, b) => b[1] - a[1]).slice(0, 8).map(([g]) => g);
  const rows = items.filter(i => cols.some(g => i.groups?.[g]));
  const maxCell = Math.max(1, ...rows.flatMap(i => cols.map(g => i.groups?.[g] || 0)));
  const shade = n => n ? `background: color-mix(in srgb, var(--series-1) ${Math.round(15 + 85 * Math.sqrt(n / maxCell))}%, var(--panel-2))` : '';
  const t = d?.transfers;
  return pageHead(L('ARMS · EQUIPMENT · SUPPLY', 'ARMES · ÉQUIPEMENTS · APPROVISIONNEMENT'), L('Arms & equipment', 'Armes et équipements'), L('Weapon systems documented in our sources, the armed actors named alongside them, and recorded state-to-state transfers.', 'Systèmes d’armes documentés dans nos sources, acteurs armés cités avec eux et transferts entre États.'), `<button class="btn" data-act="refresh-reference">${icon('refresh')}${L('Refresh references', 'Actualiser')}</button>`)
    + `<section class="panel" style="margin-bottom:16px"><div class="toolbar"><div class="chips" style="width:100%"><button class="chip ${!f.category ? 'active' : ''}" data-act="arms-cat" data-arg="">${L('All', 'Tout')} <b>${all.length}</b></button>${(d?.categories || []).map(c => `<button class="chip ${f.category === c.id ? 'active' : ''}" data-act="arms-cat" data-arg="${esc(c.id)}">${esc(c[state.lang])} <b>${all.filter(i => i.category === c.id).length}</b></button>`).join('')}</div>
      <label class="search">${icon('search')}<input id="arms-q" class="input" type="search" data-model="wf.q" value="${esc(f.q)}" placeholder="${L('Search weapons, designations…', 'Rechercher armes, désignations…')}" aria-label="${L('Search equipment', 'Rechercher')}"></label>
      <select class="select" data-model="wf.sort" aria-label="${L('Sort', 'Tri')}"><option value="mentions" ${f.sort === 'mentions' ? 'selected' : ''}>${L('Most mentioned', 'Plus mentionnés')}</option><option value="name" ${f.sort === 'name' ? 'selected' : ''}>${L('Name', 'Nom')}</option></select></div>
      <div class="equip-grid">${items.map(i => `<div class="equip-card" role="button" tabindex="0" data-act="weapon" data-arg="${esc(i.id)}">${itemImage(i, 'equip-img')}<div class="equip-body"><div class="report-kicker"><span>${esc(catName(i.category))}</span>${i.model ? `<span class="badge accent" data-tip="${esc(L('Representative 3D model available', 'Modèle 3D représentatif disponible'))}">3D</span>` : ''}${i.transfers ? `<span class="badge" data-tip="${esc(L('Matching SIPRI transfer records', 'Transferts SIPRI correspondants'))}">SIPRI ${i.transfers}</span>` : ''}</div><h3>${esc(i.name)}</h3><p>${esc(i.description?.[state.lang] || i.description?.en || '')}</p>
        <div class="actor-foot">${Object.entries(i.groups || {}).sort((a, b) => b[1] - a[1]).slice(0, 2).map(([g, n]) => `<span class="tag" data-tip="${esc(L('Named in the same sentence', 'Cité dans la même phrase'))}">${esc(groupName(g))} · ${n}</span>`).join('')}<span class="grow"></span><span class="num" data-tip="${esc(L(`${i.mentions?.report || 0} reports · ${i.mentions?.post || 0} X posts · ${i.mentions?.document || 0} document pages`, `${i.mentions?.report || 0} articles · ${i.mentions?.post || 0} publications · ${i.mentions?.document || 0} pages`))}">${icon('activity')} ${nf(i.mentions?.total || 0)}</span></div></div></div>`).join('') || emptyState(L('No equipment matches', 'Aucun équipement'), '')}</div></section>
    <div class="split"><section class="panel"><div class="panel-head"><h2>${icon('grid')}${L('Who is named alongside which weapons', 'Qui est cité avec quelles armes')}</h2><small>${L('same-sentence co-mentions', 'co-mentions dans la même phrase')}</small></div>
      ${rows.length ? `<div class="matrix-wrap"><table class="matrix"><thead><tr><th></th>${cols.map(g => `<th><span>${esc(groupName(g))}</span></th>`).join('')}</tr></thead><tbody>${rows.map(i => `<tr><th><button class="linkbtn" data-act="weapon" data-arg="${esc(i.id)}">${esc(i.name)}</button></th>${cols.map(g => { const n = i.groups?.[g] || 0; return `<td style="${shade(n)}" data-tip="${esc(`${i.name} · ${groupName(g)}: ${n}`)}">${n || ''}</td>` }).join('')}</tr>`).join('')}</tbody></table></div>` : `<div class="panel-body muted">${L('Co-mentions appear as sources are scanned.', 'Les co-mentions apparaissent au fil de l’analyse des sources.')}</div>`}
      <div class="panel-body notice" style="margin:12px 16px">${L('A co-mention means the weapon and the actor appear in the same sentence. It is a lead for review, not proof of possession, use or supply.', 'Une co-mention signifie que l’arme et l’acteur figurent dans la même phrase. C’est une piste, pas une preuve de possession, d’usage ou de fourniture.')}</div></section>
    <section class="panel"><div class="panel-head"><h2>${icon('globe')}${L('State suppliers · SIPRI', 'Fournisseurs étatiques · SIPRI')}</h2><small>${t?.rows ? `${nf(t.rows)} ${L('records', 'entrées')}` : ''}</small></div><div class="panel-body">
      ${t?.rows ? t.flows.map(fl => { const max = Math.max(1, ...fl.suppliers.map(s => s.tiv || s.orders)); return `<div class="eyebrow" style="margin:10px 0 6px">${L('To', 'Vers')} ${esc(fl.recipient)}</div>${fl.suppliers.slice(0, 8).map(s => `<div class="rank-row" style="cursor:default" data-tip="${esc(`${s.supplier} → ${fl.recipient}: ${s.orders} ${L('orders', 'commandes')} · TIV ${Number(s.tiv).toFixed(1)}`)}"><span>${esc(s.supplier)}</span>${meter(s.tiv || s.orders, max, 'var(--series-2)')}<b>${s.tiv ? Number(s.tiv).toFixed(0) : s.orders}</b></div>`).join('')}` }).join('') + `<p class="notice">${L('Bars show SIPRI trend-indicator values (TIV) of deliveries, a volume measure, not financial value.', 'Les barres montrent les valeurs TIV du SIPRI, une mesure de volume et non de valeur financière.')}</p>`
        : `<p style="margin:0 0 10px;color:var(--text-2);font-size:13px">${L('Import the SIPRI Arms Transfers Database to see which states supplied the DRC, Rwanda, Uganda and Burundi.', 'Importez la base SIPRI pour voir quels États ont fourni la RDC, le Rwanda, l’Ouganda et le Burundi.')}</p><ol class="steps"><li>${L('Open', 'Ouvrez')} <a href="https://armstransfers.sipri.org/" target="_blank" rel="noopener noreferrer">armstransfers.sipri.org</a> → Trade Register.</li><li>${L('Recipients: DR Congo, Rwanda, Uganda, Burundi · all years · export CSV.', 'Destinataires : RD Congo, Rwanda, Ouganda, Burundi · toutes années · export CSV.')}</li><li>${L('Save the file into', 'Enregistrez le fichier dans')} <code>${esc(t?.directory || 'data/sipri')}</code>. ${L('It is imported within a minute.', 'Il est importé en moins d’une minute.')}</li></ol>${(t?.sync || []).filter(x => x.error).map(x => `<p class="ev-error">${esc(x.error)}</p>`).join('')}`}
    </div></section></div>
    <p class="notice">${esc(d?.notice || '')}</p>`;
}
async function openWeapon(id) {
  state.modal = 'weapon'; state.weapon = { id, data: null, show3d: false };
  if (location.hash !== '#weapon/' + id) history.replaceState(null, '', '#weapon/' + encodeURIComponent(id));
  renderWeapon();
  try { state.weapon.data = await api(`/v1/arms/${encodeURIComponent(id)}`) } catch (e) { state.weapon.error = e.message }
  if (state.weapon?.id === id) renderWeapon();
}
function renderWeapon() {
  const s = state.weapon; if (!s) return;
  const d = s.data, it = d?.item, w = d?.wikidata, m = d?.model;
  let body;
  if (s.error) body = `<div class="errorbar">${esc(s.error)}</div>`;
  else if (!d) body = `<div class="loading-line"></div>`;
  else {
    const groups = {};
    for (const x of d.mentions) for (const [g, n] of Object.entries(x.groups || {})) groups[g] = (groups[g] || 0) + n;
    const g = Object.entries(groups).sort((a, b) => b[1] - a[1]);
    const ops = w?.operators || [];
    const regional = ops.filter(o => regionalLabel(o.label?.en || ''));
    body = `<div class="weapon-hero">${d.image?.thumb ? `<img src="${esc(d.image.thumb)}" alt="" referrerpolicy="no-referrer">` : m?.thumb ? `<img src="${esc(m.thumb)}" alt="" referrerpolicy="no-referrer">` : ''}</div>${credit(d.image, 'equipment', it.id)}
      <div class="report-kicker" style="margin-top:12px"><span>${esc(d.category?.[state.lang] || it.category)}</span></div><h1>${esc(it.name)}</h1><p class="dossier-bio">${esc(w?.description?.[state.lang] || w?.description?.en || '')}</p>
      <div class="actions" style="margin-bottom:12px">${w?.wikipedia?.[state.lang] || w?.wikipedia?.en ? `<a class="btn sm" href="${esc(w.wikipedia[state.lang] || w.wikipedia.en)}" target="_blank" rel="noopener noreferrer">${icon('book')}Wikipedia</a>` : ''}${w ? `<a class="btn sm ghost" href="${esc(w.url)}" target="_blank" rel="noopener noreferrer">Wikidata ${esc(w.qid)}</a>` : ''}</div>
      <section class="panel" style="padding:14px 16px;margin-bottom:12px"><dl class="props">${factRow(L('Origin', 'Origine'), labelList(w?.origin))}${factRow(L('Manufacturer', 'Fabricant'), labelList(w?.manufacturer))}${factRow(L('In service since', 'En service depuis'), esc(w?.service_entry || w?.inception || ''))}${factRow(L('Regional operators (Wikidata)', 'Utilisateurs régionaux (Wikidata)'), regional.length ? regional.map(o => `<b>${esc(o.label?.[state.lang] || o.label?.en)}</b>`).join(', ') : '')}${factRow(L('All listed operators', 'Tous les utilisateurs'), ops.length ? `<details><summary>${ops.length}</summary>${labelList(ops)}</details>` : '')}${factRow(L('Search terms', 'Termes recherchés'), it.terms.map(esc).join(', '))}</dl></section>
      ${m ? `<section class="panel model-panel"><div class="panel-head"><h2>${icon('target')}${L('3D model', 'Modèle 3D')}</h2><small>${L('representative of the type', 'représentatif du type')}</small></div>${s.show3d ? `<iframe class="model-frame" title="${esc(m.name)}" src="${esc(m.embed_url)}?autostart=1&ui_theme=${state.theme === 'light' ? 'light' : 'dark'}&dnt=1" allow="autoplay; fullscreen; xr-spatial-tracking" allowfullscreen></iframe>` : `<button class="model-cover" data-act="show-3d">${m.thumb ? `<img src="${esc(m.thumb)}" alt="" referrerpolicy="no-referrer">` : ''}<span class="btn primary">${icon('target')}${L('Load interactive 3D model', 'Charger le modèle 3D')}</span><small>${L('Loads Sketchfab (third-party)', 'Charge Sketchfab (tiers)')}</small></button>`}<div class="credit" style="padding:8px 16px 12px">“${esc(m.name)}” ${L('by', 'par')} <a href="${esc(m.author_url)}" target="_blank" rel="noopener noreferrer">${esc(m.author)}</a> · ${esc(m.license || '')} · <a href="${esc(m.viewer_url)}" target="_blank" rel="noopener noreferrer">Sketchfab</a> · ${L('Not the specific item documented.', 'Pas l’objet précis documenté.')}</div></section>` : ''}
      ${g.length ? `<h3 class="section-title">${icon('users')}${L('Actors named in the same sentence', 'Acteurs cités dans la même phrase')}</h3><section class="panel" style="padding:12px 16px">${g.map(([k, n]) => `<div class="topic-bar"><span class="tag">${esc(groupName(k))}</span>${meter(n, g[0][1], 'var(--series-1)')}<b>${n}</b></div>`).join('')}</section>` : ''}
      ${d.transfers.length ? `<h3 class="section-title">${icon('globe')}${L('SIPRI transfer records', 'Transferts SIPRI')}</h3><section class="panel"><div class="matrix-wrap"><table class="table"><thead><tr><th>${L('Supplier', 'Fournisseur')}</th><th>${L('Recipient', 'Destinataire')}</th><th>${L('Designation', 'Désignation')}</th><th>${L('Ordered', 'Commande')}</th><th>${L('Delivered', 'Livraison')}</th></tr></thead><tbody>${d.transfers.slice(0, 100).map(r => `<tr><td>${esc(r.supplier)}</td><td>${esc(r.recipient)}</td><td>${esc(r.designation || '')}<br><small class="muted">${esc(r.description || '')}</small></td><td>${esc(r.ordered || '')} ${r.order_year ? `<small class="muted">(${esc(r.order_year)})</small>` : ''}</td><td>${esc(r.delivered || '')} ${r.delivery_years ? `<small class="muted">(${esc(r.delivery_years)})</small>` : ''}</td></tr>`).join('')}</tbody></table></div></section>` : ''}
      <h3 class="section-title">${icon('activity')}${L('Mentions in collected sources', 'Mentions dans les sources collectées')} <span class="muted num">${d.mentions.length}</span></h3>
      ${mentionTimeline(d.mentions)}
      <section class="panel" style="margin-top:10px">${mentionList(d.mentions, it.terms, '', true)}</section>
      <p class="notice">${L('Mentions are keyword matches and include general or historical references. Verify each against the source before drawing conclusions.', 'Les mentions sont des correspondances par mots-clés, y compris générales ou historiques. Vérifiez chaque source avant de conclure.')}</p>`;
  }
  const top = $('#overlay .drawer')?.scrollTop || 0, had = !!$('#overlay .drawer');
  $('#overlay').innerHTML = `<div class="shade" data-close><section class="drawer" role="dialog" aria-modal="true" aria-label="${esc(it?.name || L('Equipment', 'Équipement'))}"><div class="drawer-top"><span class="eyebrow">${icon('target')}${L('EQUIPMENT', 'ÉQUIPEMENT')}</span><div class="actions">${d ? `<button class="btn sm" data-act="export-weapon">${icon('download')}JSON</button>` : ''}<button class="iconbtn" data-act="close" aria-label="${L('Close', 'Fermer')}">${icon('close')}</button></div></div><div class="drawer-body">${body}</div></section></div>`;
  if (had) $('#overlay .drawer').scrollTop = top; else $('#overlay [data-act="close"]')?.focus();
}

// ---------------------------------------------------------------------------------------------
// Command palette

function paletteItems(q) {
  const n = norm(q.trim()), out = [];
  const views = NAV().filter(([, , label]) => !n || norm(label).includes(n)).map(([id, ic, label]) => ({ group: L('Go to', 'Aller à'), icon: ic, label, run: () => go(id) }));
  out.push(...views);
  if (n) {
    const h = q.trim().replace(/^@/, '');
    if (validHandle(h)) out.push({ group: L('X accounts', 'Comptes X'), icon: 'user', label: `${L('Open dossier for', 'Ouvrir le dossier de')} @${h}`, run: () => openDossier(h) });
    out.push(...(state.accounts?.items || []).filter(a => norm(a.handle + ' ' + a.name).includes(n.replace(/^@/, ''))).slice(0, 5).map(a => ({ group: L('X accounts', 'Comptes X'), icon: 'user', label: `${a.name} · @${a.handle}`, hint: `${a.posts} ${L('posts', 'publ.')}`, run: () => openDossier(a.handle) })));
    out.push(...(state.actors?.items || []).filter(a => norm([a.name, ...a.aliases].join(' ')).includes(n)).sort((a, b) => (b.mentions.total || 0) - (a.mentions.total || 0)).slice(0, 5).map(a => ({ group: L('Key figures & sanctions', 'Acteurs et sanctions'), icon: a.sanctioned ? 'flag' : 'user', label: a.name, hint: a.sanctioned ? [...new Set(a.sanctions.map(s => s.list))].join(' · ') : actorDesc(a).slice(0, 40), run: () => openActor(a.id) })));
    out.push(...(state.arms?.items || []).filter(i => norm([i.name, ...i.terms].join(' ')).includes(n)).slice(0, 4).map(i => ({ group: L('Arms & equipment', 'Armes et équipements'), icon: 'target', label: i.name, hint: `${i.mentions?.total || 0} ${L('mentions', 'mentions')}`, run: () => openWeapon(i.id) })));
    out.push(...placesList().filter(p => norm(p.name + ' ' + p.province).includes(n)).slice(0, 4).map(p => ({ group: L('Places', 'Lieux'), icon: 'pin', label: p.name, hint: `${p.count} ${L('reports', 'articles')}`, run: () => { state.place = p.name; go('situation') } })));
    out.push(...state.items.filter(a => norm(title(a) + ' ' + a.title).includes(n)).sort((a, b) => b.rank.score - a.rank.score).slice(0, 6).map(a => ({ group: L('Reports', 'Articles'), icon: 'layers', label: title(a), hint: `${a.rank.score} · ${age(a.published_at)}`, run: () => openDetail(a.id) })));
    out.push({ group: L('Search', 'Rechercher'), icon: 'search', label: `${L('Search reports for', 'Rechercher dans les articles')} “${q.trim()}”`, run: () => { state.q = q.trim(); go('reports') } });
    out.push({ group: L('Search', 'Rechercher'), icon: 'radio', label: `${L('Search X posts for', 'Rechercher dans les publications X')} “${q.trim()}”`, run: () => { state.sf.q = q.trim(); go('social') } });
  }
  return out;
}
function openPalette() {
  state.modal = 'palette'; state.palette = { q: '', index: 0, items: paletteItems('') };
  $('#overlay').innerHTML = `<div class="shade dialog-shade" data-close><div class="palette" role="dialog" aria-modal="true" aria-label="${L('Search everything', 'Tout rechercher')}"><div class="palette-input">${icon('search')}<input id="palette-q" placeholder="${L('Search reports, places, @accounts, views…', 'Rechercher articles, lieux, @comptes, vues…')}" autocomplete="off" spellcheck="false" aria-label="${L('Search', 'Rechercher')}"></div><div class="palette-results" id="palette-results"></div><div class="palette-foot"><span><span class="kbd">↑↓</span> ${L('navigate', 'naviguer')}</span><span><span class="kbd">↵</span> ${L('open', 'ouvrir')}</span><span><span class="kbd">esc</span> ${L('close', 'fermer')}</span></div></div></div>`;
  renderPaletteResults(); $('#palette-q').focus();
}
function renderPaletteResults() {
  const p = state.palette; let group = '';
  $('#palette-results').innerHTML = p.items.map((it, i) => `${it.group !== group ? `<div class="palette-group">${esc(group = it.group)}</div>` : ''}<button class="palette-item ${i === p.index ? 'active' : ''}" data-palette="${i}">${icon(it.icon)}<span>${esc(it.label)}</span>${it.hint ? `<small>${esc(it.hint)}</small>` : ''}</button>`).join('') || `<div class="empty">${L('No results', 'Aucun résultat')}</div>`;
  $('#palette-results .active')?.scrollIntoView({ block: 'nearest' });
}
function runPalette(i) { const it = state.palette?.items[i]; if (!it) return; closeOverlay(); it.run() }

// ---------------------------------------------------------------------------------------------
// Overlays, settings, toast, tooltip

function closeOverlay() {
  evidenceGeneration++;
  if (evidenceBlob) { URL.revokeObjectURL(evidenceBlob); evidenceBlob = null }
  if ((state.dossier && hashAccount()) || (state.actor && hashPart('actor')) || (state.weapon && hashPart('weapon'))) history.replaceState(null, '', '#' + state.view);
  state.modal = null; state.detailId = null; state.detail = null; state.dossier = null; state.palette = null; state.actor = null; state.weapon = null;
  $('#overlay').innerHTML = ''; document.body.style.overflow = '';
}
function settings() {
  state.modal = 'settings';
  $('#overlay').innerHTML = `<div class="shade dialog-shade" data-close><form class="dialog" data-form="settings" role="dialog" aria-modal="true" aria-labelledby="settings-title"><h2 id="settings-title">${L('Connection settings', 'Paramètres de connexion')}</h2><div class="fields"><label>${L('API bearer token (if configured)', 'Jeton API (si configuré)')}<input id="token" type="password" value="${esc(sessionStorage.getItem('congo-token') || '')}" autocomplete="off"></label></div><p>${L('Stored only for this browser tab. No token is needed for the default local setup.', 'Conservé uniquement dans cet onglet. Aucun jeton n’est requis pour la configuration locale par défaut.')}</p><div class="actions" style="justify-content:flex-end"><button class="btn" type="button" data-act="close">${L('Cancel', 'Annuler')}</button><button class="btn primary" type="submit">${L('Apply', 'Appliquer')}</button></div></form></div>`;
  $('#token').focus();
}
function toast(text) { const el = $('#toast'); el.textContent = text; el.style.display = 'block'; clearTimeout(toast.timer); toast.timer = setTimeout(() => el.style.display = 'none', 4500) }
function download(name, data) {
  const url = URL.createObjectURL(new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' }));
  const a = document.createElement('a'); a.href = url; a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
}
const tip = $('#tip');
document.addEventListener('mouseover', e => {
  const el = e.target.closest?.('[data-tip]');
  if (!el || !el.dataset.tip) { tip.hidden = true; return }
  tip.textContent = el.dataset.tip; tip.hidden = false;
});
document.addEventListener('mousemove', e => {
  if (tip.hidden) return;
  const w = tip.offsetWidth, h = tip.offsetHeight;
  tip.style.left = Math.min(window.innerWidth - w - 8, e.clientX + 14) + 'px';
  tip.style.top = (e.clientY + h + 24 > window.innerHeight ? e.clientY - h - 12 : e.clientY + 16) + 'px';
});

// ---------------------------------------------------------------------------------------------
// Events (delegated: survive re-renders)

const ACTIONS = {
  'agreement-open': id => openAgreement(id),
  'logout': async () => {await api('/v1/auth/logout',{});sessionStorage.removeItem('congo-token');location.reload()},
  'cost-export': () => download('congo-signal-costs.json',state.costs),
  'cost-save': async provider => {const root=document.getElementById('cost-'+provider);const body={provider,paused:root.querySelector('[name=paused]').checked};for(const key of ['request_usd','daily_usd','monthly_usd','daily_requests'])body[key]=Number(root.querySelector('[name='+key+']').value);state.costs=await api('/v1/costs',body);render()},
  'agreement-analyze': async id => {await api(`/v1/agreements/${id}/analyze`,{});await openAgreement(id)},
  'agreement-review': async id => {await api(`/v1/agreements/${id}/review`,{});await load();await openAgreement(id)},
  'agreement-pdf': arg => agreementPdf(arg),
  'agreement-export': () => download('congo-signal-agreements.json',{metadata:state.agreements?.metadata,items:agreementRows()}),
  nav: v => go(v),
  lang: v => { state.lang = v; store.set('congo-lang', v); markerSignature = ''; render(); if (state.modal === 'detail') renderDetail(); if (state.modal === 'dossier') renderDossier() },
  theme: () => { state.theme = state.theme === 'light' ? 'dark' : 'light'; document.documentElement.dataset.theme = state.theme; store.set('congo-theme', state.theme); render() },
  settings, palette: openPalette, close: closeOverlay,
  reload: () => load(),
  'mineral-layer': d => { state.mineralLayers = state.mineralLayers.includes(d) ? state.mineralLayers.filter(x => x !== d) : [...state.mineralLayers,d]; render() },
  'mineral-select': id => selectMineral(id),
  'mineral-close': () => {state.mineralSelected='';render()},
  'mineral-fit': () => fitMinerals(),
  'mineral-more': () => {state.mineralVisible+=30;render();$('.mineral-directory')?.setAttribute('open','')},
  'mineral-reset': () => {Object.assign(state,{mineralSearch:'',mineralQuery:'',mineralProvince:'',mineralKind:'',mineralSelected:'',mineralVisible:15,mineralLayers:['IPIS','USGS']});render()},
  'mineral-export': () => {const f=selectedMineral();if(f) download('congo-signal-site.json',{...f,metadata:state.geology.metadata})},
  'map-focus': () => {state.mapFocus=!state.mapFocus;render()},
  'geology-archive': () => geologyArchive(),
  'geology-export': () => download('congo-signal-minerals.geojson', {type:'FeatureCollection',metadata:state.geology?.metadata,features:mineralFeatures()}),
  report: id => openDetail(id),
  save: async id => { const a = state.items.find(a => a.id === id); try { a.analyst = await api(`/v1/intelligence/${id}/notes`, { ...a.analyst, saved: !a.analyst.saved }); render() } catch (e) { toast(e.message) } },
  category: k => { state.category = k; state.visible = 20; render() },
  place: name => selectPlace(name),
  'clear-place': () => { state.place = ''; render() },
  reset: () => { Object.assign(state, { q: '', place: '', category: 'all', publisher: '' }); render() },
  more: async () => { state.visible += 20; if (state.visible > state.items.length && state.items.length < state.total && state.limit < 500) { state.limit = Math.min(500, state.limit + 100); await load() } else render() },
  export: () => { download('congo-signal-reports.json', { exported_at: new Date().toISOString(), language: state.lang, verification: 'single_source_unverified', articles: state.view === 'situation' ? situationItems() : libraryItems() }); toast(L('Filtered reports exported', 'Articles filtrés exportés')) },
  collect: async () => { await Promise.all([api('/v1/jobs', { url: 'https://actualite.cd/' }), api('/v1/feeds/refresh', {}).catch(() => null)]); toast(L('Collection queued. New reports appear automatically.', 'Collecte ajoutée. Les nouveaux articles apparaîtront automatiquement.')); await load() },
  batch: async () => {
    if (!state.status.enabled || !state.status.available) throw Error(L('AI is unavailable. Check the server’s AI provider settings.', 'IA indisponible. Vérifiez la configuration IA du serveur.'));
    const todo = (state.view === 'situation' ? situationItems() : ['reports', 'saved'].includes(state.view) ? libraryItems() : [...state.items].sort((a, b) => b.rank.score - a.rank.score)).filter(a => ['not_requested', 'failed'].includes(a.brief.status)).slice(0, 5);
    for (const a of todo) await (a.record_type === 'evidence' ? api(`/v1/evidence/${a.id}/analyze`, {}) : api('/v1/intelligence/analyze', { article_id: a.id, task: 'brief' }));
    toast(L(`${todo.length} report(s) queued for AI analysis`, `${todo.length} article(s) ajouté(s) à la file IA`)); await load();
  },
  'social-topic': id => { state.sf.topic = state.view === 'social' && state.sf.topic === id ? '' : id; state.sf.visible = 40; if (state.view !== 'social') go('social'); else render() },
  'post-more': () => { state.sf.visible += 40; render() },
  'export-posts': () => download('congo-signal-x-posts.json', { exported_at: new Date().toISOString(), filters: state.sf, notice: state.social?.notice, posts: filteredPosts() }),
  account: h => openDossier(h),
  actor: id => openActor(id),
  'actor-kind': k => { state.af.kind = k; state.af.visible = 48; render() },
  'actor-more': () => { state.af.visible += 48; render() },
  'actor-mkind': k => { state.actor.kind = k; renderActor() },
  'export-actor': () => { const a = state.actor?.data; if (a) download(`profile-${a.id}.json`, { exported_at: new Date().toISOString(), ...a }) },
  weapon: id => openWeapon(id),
  'reject-image': async arg => {
    const [kind, id] = arg.split(':');
    await api(`/v1/images/${encodeURIComponent(kind)}/${encodeURIComponent(id)}/reject`, {});
    toast(L('Image rejected; it will not be shown or searched again.', 'Image rejetée ; elle ne sera plus affichée ni recherchée.'));
    await load();
    if (kind === 'actor' && state.actor?.id === id) openActor(id); else if (kind === 'equipment' && state.weapon?.id === id) openWeapon(id);
  },
  'arms-cat': c => { state.wf.category = c; render() },
  'show-3d': () => { state.weapon.show3d = true; renderWeapon() },
  'export-weapon': () => { const d = state.weapon?.data; if (d) download(`equipment-${d.item.id}.json`, { exported_at: new Date().toISOString(), ...d }) },
  'refresh-reference': async () => { const r = await api('/v1/actors/refresh', {}); toast(r.sync_enabled ? L('Reference refresh queued. Lists and profiles update in the background.', 'Actualisation lancée. Listes et profils se mettent à jour en arrière-plan.') : L('Network sync is disabled on the server (REFERENCE_SYNC=0).', 'Synchronisation réseau désactivée sur le serveur (REFERENCE_SYNC=0).')) },
  hashtag: tag => { closeOverlay(); state.sf.q = '#' + tag; state.sf.visible = 40; go('social') },
  copy: async text => { try { await navigator.clipboard.writeText(text); toast(L('Link copied', 'Lien copié')) } catch { toast(text) } },
  history: () => fetchHistory(),
  sample: async id => { const r = await api(`/v1/social/topics/${encodeURIComponent(id)}/sample`, {}); toast(L(`${topicName(id)}: ${r.returned} posts fetched · ${r.new} new`, `${topicName(id)} : ${r.returned} publications · ${r.new} nouvelles`)); state.sf.topic = id; await load() },
  expand: (_, el) => { const t = el.previousElementSibling; t.classList.toggle('clamp'); el.textContent = t.classList.contains('clamp') ? L('Show full post', 'Afficher tout') : L('Show less', 'Réduire') },
  'dossier-topic': t => { state.dossier.topic = t; state.dossier.visible = 30; renderDossier() },
  'dossier-kind': k => { state.dossier.kind = k; state.dossier.visible = 30; renderDossier() },
  'dossier-more': () => { state.dossier.visible += 30; renderDossier() },
  'export-dossier': () => { const d = state.dossier?.data; if (d) download(`x-dossier-${d.account.handle}.json`, { exported_at: new Date().toISOString(), ...d }) },
  tab: k => { state.detailTab = k; renderDetail() },
  ai: async task => { const id = state.detail.article.id; await api('/v1/intelligence/analyze', { article_id: id, task }); state.detail = await api(`/v1/intelligence/${id}`); renderDetail(); load() },
  'detail-save': async () => { const d = state.detail; d.analyst = await api(`/v1/intelligence/${d.article.id}/notes`, { ...d.analyst, saved: !d.analyst.saved }); renderDetail(); load() },
  'evidence-open': id => openEvidence(id),
  'evidence-ai': async id => { await api(`/v1/evidence/${id}/analyze`, {}); await load() },
  'source-refresh': async id => { await api(`/v1/sources/${id}/refresh`, {}); toast(L('Source check queued', 'Vérification ajoutée')); await load() },
  'source-doc': async id => {
    const r = await api(`/v1/evidence?source_id=${encodeURIComponent(id)}&limit=1`);
    if (r.items.length) return openEvidence(r.items[0].document_id);
    const s = state.sources.items.find(s => s.id === id);
    showEvidenceDialog(s.title, `<div class="notice">${L('This PDF has not been downloaded. The publisher is currently unavailable to the collector. Once downloaded, it will open here.', 'Ce PDF n’a pas encore été téléchargé. La source est actuellement inaccessible au collecteur. Après téléchargement, il s’ouvrira ici.')}</div>`);
  },
  layer: k => { state.layers = state.layers.includes(k) ? state.layers.filter(x => x !== k) : [...state.layers, k]; state.visible = 20; render() },
  'all-layers': () => { state.layers = [...LAYERS]; render() },
  window: n => { state.windowDays = Number(n); state.selectedDay = ''; state.visible = 20; render() },
  day: d => { state.selectedDay = state.selectedDay === d ? '' : d; state.visible = 20; render() },
  'all-days': () => { state.selectedDay = ''; render() },
  'clear-situation': () => { state.place = ''; state.selectedDay = ''; render() },
  base: b => { state.basemap = b; render() },
  tilt: () => { state.tilt = !state.tilt; mapInstance?.easeTo({ pitch: state.tilt ? 55 : 0, bearing: state.tilt ? -12 : 0, duration: 500 }); render() },
  'reset-map': () => { state.place = ''; render(); mapInstance?.fitBounds([[12, -13.6], [31.4, 5.5]], { padding: 40, duration: 600 }) },
};
document.addEventListener('click', async e => {
  const pal = e.target.closest('[data-palette]'); if (pal) { runPalette(Number(pal.dataset.palette)); return }
  if (e.target.closest('[data-stop]')) return;
  const el = e.target.closest('[data-act]');
  if (!el) { const shade = e.target.closest('[data-close]'); if (shade && e.target === shade) closeOverlay(); return }
  if (el.tagName === 'A' && el.dataset.act === 'nav') e.preventDefault();
  const fn = ACTIONS[el.dataset.act]; if (!fn) return;
  const busy = el.tagName === 'BUTTON' && !['nav', 'lang', 'theme', 'close', 'tab', 'category', 'layer', 'day', 'window', 'expand', 'actor-kind', 'arms-cat', 'actor-mkind', 'show-3d'].includes(el.dataset.act);
  if (busy) el.disabled = true;
  try { await fn(el.dataset.arg ?? '', el) } catch (err) { toast(err.message) } finally { if (busy && el.isConnected) el.disabled = false }
});
let typing = null;
function setModel(path, value) {
  const [a, b] = path.split('.');
  if (b) state[a][b] = value; else state[a] = value;
  state.visible = 20; state.sf.visible = 40;
  if (path.startsWith('mineral')) state.mineralVisible=15;
  if (path === 'includeRegional') { state.publisher = ''; load(); return }
  render();
}
document.addEventListener('input', e => {
  const el = e.target.closest('[data-model]');
  if (el && el.tagName === 'INPUT' && el.type !== 'checkbox') { clearTimeout(typing); typing = setTimeout(() => setModel(el.dataset.model, el.value), 140); return }
  if (e.target.id === 'palette-q') { state.palette.q = e.target.value; state.palette.items = paletteItems(e.target.value); state.palette.index = 0; renderPaletteResults() }
});
document.addEventListener('change', e => {
  const el = e.target.closest('[data-model]');
  if (el && (el.tagName === 'SELECT' || el.type === 'checkbox')) setModel(el.dataset.model, el.type === 'checkbox' ? el.checked : el.value);
});
document.addEventListener('submit', async e => {
  const form = e.target.closest('[data-form]'); if (!form) return;
  e.preventDefault();
  const kind = form.dataset.form;
  if(kind==='team'){const f=new FormData(form),body={email:f.get('email'),role:f.get('role'),active:f.has('active'),daily_requests:Number(f.get('daily_requests')),daily_usd:Number(f.get('daily_usd')),monthly_usd:Number(f.get('monthly_usd'))};try{state.team=await api('/v1/team',body);toast(L('Membership saved','Membre enregistré'));await load()}catch(err){toast(err.message)}return}
  if (kind === 'lookup') return openDossier($('#lookup-handle').value);
  if (kind === 'settings') { sessionStorage.setItem('congo-token', $('#token').value.trim()); state.config = null; closeOverlay(); return load() }
  if (kind === 'notes') {
    const d = state.detail;
    try { d.analyst = await api(`/v1/intelligence/${d.article.id}/notes`, { saved: d.analyst.saved, review: $('#review').value, note: $('#note').value }); toast(L('Research notes saved', 'Notes de recherche enregistrées')); load() } catch (err) { toast(err.message) }
  }
});
document.addEventListener('keydown', e => {
  if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') { e.preventDefault(); state.modal === 'palette' ? closeOverlay() : openPalette(); return }
  if (e.key === '/' && !state.modal && !e.target.matches('input,textarea,select')) { e.preventDefault(); openPalette(); return }
  if (/^[1-9]$/.test(e.key) && !e.metaKey && !e.ctrlKey && !e.altKey && !state.modal && !e.target.matches('input,textarea,select')) { const v = NAV()[Number(e.key) - 1]; if (v) { go(v[0]); return } }
  if (e.key === 'Escape' && state.modal) { closeOverlay(); return }
  if (state.modal === 'palette' && ['ArrowDown', 'ArrowUp', 'Enter'].includes(e.key)) {
    e.preventDefault(); const p = state.palette;
    if (e.key === 'Enter') return runPalette(p.index);
    p.index = (p.index + (e.key === 'ArrowDown' ? 1 : -1) + p.items.length) % Math.max(1, p.items.length); renderPaletteResults(); return;
  }
  if ((e.key === 'Enter' || e.key === ' ') && e.target.matches('[role="button"][data-act]')) { e.preventDefault(); e.target.click(); return }
  if (e.key === 'Tab' && $('#overlay [role="dialog"]')) {
    const nodes = $$('button,a[href],input,textarea,select', $('#overlay [role="dialog"]')).filter(n => !n.disabled);
    const first = nodes[0], last = nodes.at(-1);
    if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last?.focus() } else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first?.focus() }
  }
});
window.addEventListener('popstate', () => { const v = viewFromHash(); if (v !== state.view) { state.view = v; render() } openFromHash() });

// ---------------------------------------------------------------------------------------------
// Boot

try { state.geometry = await fetch('/drc.geo.json').then(r => r.json()) } catch {}
try { state.geology = await fetch('/v1/geology/sites').then(r => { if (!r.ok) throw Error(r.status); return r.json() }) } catch {}
state.enter = true;
await load();
openFromHash();
setInterval(async () => {
  if (document.hidden || (state.modal && state.modal !== 'detail') || document.activeElement?.matches('input,textarea,select')) return;
  await load();
  if (state.detailId && state.detailTab !== 'notes') {
    try { const d = await api(`/v1/intelligence/${state.detailId}`); if (JSON.stringify(d) !== JSON.stringify(state.detail)) { state.detail = d; renderDetail() } } catch {}
  }
}, 15000);
