import { createVoiceCenter } from '/voice.js';
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
  bell: 'M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9 M10.3 21a1.9 1.9 0 0 0 3.4 0',
  mail: 'M4 4h16v16H4z m0 0 8 8 8-8',
  mic: 'M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3Z M19 11a7 7 0 0 1-14 0 M12 18v3 M8.5 21h7',
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

const VIEWS = ['briefing', 'situation', 'events', 'alerts', 'reports', 'social', 'sources', 'actors', 'arms', 'saved', 'method', 'agreements', 'costs', 'team', 'ai-settings', 'ai-command'];
const LEGACY_VIEWS = { map: 'situation', overview: 'briefing' };
const viewFromHash = () => { const h = location.hash.slice(1); if (h.startsWith('report/')) return 'reports'; if (h === 'flights' || h === 'minerals' || h === 'radio' || h.startsWith('radio/') || h.startsWith('mine/') || h.startsWith('focus/')) return 'situation'; if (h.startsWith('account/')) return 'social'; if (h.startsWith('actor/')) return 'actors'; if (h.startsWith('weapon/')) return 'arms'; const v = LEGACY_VIEWS[h] || h; return VIEWS.includes(v) ? v : 'briefing' };
const hashAccount = () => location.hash.startsWith('#account/') ? decodeURIComponent(location.hash.slice(9)) : '';
const hashPart = prefix => location.hash.startsWith(`#${prefix}/`) ? decodeURIComponent(location.hash.slice(prefix.length + 2)) : '';
function openFromHash() { if (hashPart('report')) openDetail(hashPart('report')); else if (hashPart('radio')) { state.radioMode = true; state.flightMode = false; state.mineralMode = false; state.radioSel = hashPart('radio'); render() } else if (hashPart('focus')) focusMineral(hashPart('focus')); else if (hashPart('mine')) { state.mineralMode = true; state.radioMode = false; state.flightMode = false; selectMineral(hashPart('mine')) } else if (hashAccount()) openDossier(hashAccount()); else if (hashPart('actor')) openActor(hashPart('actor')); else if (hashPart('weapon')) openWeapon(hashPart('weapon')) }
const state = {
  aiMode: store.get('congo-ai-mode')==='1'||viewFromHash()==='ai-command', aiMotion: store.get('congo-ai-motion')!=='0',
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

const dateFormats = new Map();
function date(s, short = false) {
  if (!s) return '—';
  // Intl formatters are costly to build; one per locale and style.
  const k = `${locale()}|${short}`;
  if (!dateFormats.has(k)) dateFormats.set(k, new Intl.DateTimeFormat(locale(), { day: '2-digit', month: 'short', ...(short ? {} : { year: 'numeric' }), hour: '2-digit', minute: '2-digit', timeZone: 'Africa/Kinshasa' }));
  const d = new Date(s);
  return Number.isNaN(d.getTime()) ? '—' : dateFormats.get(k).format(d);
}
function dayLabel(d) { return new Date(d + 'T12:00:00Z').toLocaleDateString(locale(), { day: 'numeric', month: 'short', timeZone: 'UTC' }) }
function age(s) {
  if (!s) return '—';
  const m = Math.max(0, Math.floor((Date.now() - Date.parse(s)) / 60000));
  if (m < 1) return L('now', 'à l’instant');
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
/// French for fixed English strings the server sends (notices, flight classification, errors).
/// Unknown strings pass through unchanged, so a new server message is never hidden.
const SERVER_FR = new Map([
  ["Counts include reposts and overlapping topics. They do not establish independent sources, endorsement or verified events. Accounts are attributed by public handle only; no stance, affiliation or automation labels are inferred.", "Les comptages incluent les republications et des thèmes qui se chevauchent. Ils n’établissent ni des sources indépendantes, ni une adhésion, ni des événements vérifiés. Les comptes sont attribués par leur identifiant public uniquement ; aucune position, affiliation ni automatisation n’est déduite."],
  ["Posts shown are those matching the configured topic rules. Matching is keyword-based and does not establish stance, affiliation or coordination.", "Les publications affichées sont celles qui correspondent aux règles thématiques configurées. La correspondance repose sur des mots-clés et n’établit ni position, ni affiliation, ni coordination."],
  ["Sanctions are reproduced from official lists with attribution. Profiles are compiled from Wikidata and cited text; a mention is not evidence of involvement. Name matches across lists and to Wikidata are automated and should be verified.", "Les sanctions sont reproduites depuis les listes officielles, avec attribution. Les profils sont compilés à partir de Wikidata et de textes cités ; une mention n’est pas une preuve d’implication. Les rapprochements de noms entre listes et avec Wikidata sont automatiques et doivent être vérifiés."],
  ["Mentions are text matches in collected sources. An actor named in the same sentence is a co-mention, not proof of possession, use or supply. 3D models are representative of the equipment type, not the specific items documented.", "Les mentions sont des correspondances textuelles dans les sources collectées. Un acteur cité dans la même phrase est une co-mention, pas une preuve de possession, d’usage ou de fourniture. Les modèles 3D représentent le type d’équipement, pas les objets précis documentés."],
  ["Benchmark prices are global reference prices for refined metal, not prices paid at DRC mine sites. Buyers are the first recorded importing countries in customs data (including transit hubs), not end users or companies. Partner-reported imports and DRC-reported exports often disagree; both are shown.", "Les prix de référence sont des cours mondiaux du métal raffiné, pas des prix payés sur les sites miniers de RDC. Les acheteurs sont les premiers pays importateurs enregistrés dans les données douanières (y compris les pays de transit), pas les utilisateurs finaux ni les entreprises. Les importations déclarées par les partenaires et les exportations déclarées par la RDC divergent souvent ; les deux sont affichées."],
  ["Streams are played directly from the broadcasters' servers. Availability changes often; each stream is re-checked every 30 minutes. Station locations are approximate (directory coordinates or the city in the station's listing).", "Les flux sont lus directement depuis les serveurs des diffuseurs. Leur disponibilité change souvent ; chaque flux est revérifié toutes les 30 minutes. La position des stations est approximative (coordonnées de l’annuaire ou ville indiquée dans la fiche)."],
  ["Article pages sit behind a bot challenge and many require a subscription. Collected from the publisher's official feed: title, lede, author, section and publication time, linked to the original.", "Les pages d’articles sont protégées contre les robots et souvent payantes. Collecte via le flux officiel de l’éditeur : titre, chapeau, auteur, rubrique et heure de publication, avec lien vers l’original."],
  ["UN radio in the DRC run by MONUSCO. Read from its news feed (/feed/actualite; the site-wide /rss.xml lists only stale admin pages), then each article page for the full text. robots.txt allows article pages with Crawl-delay: 10, which is honoured. The feed summary is kept when a page cannot be fetched and the page is retried on later polls.", "Radio des Nations unies en RDC, gérée par la MONUSCO. Lecture de son flux d’actualité (/feed/actualite), puis de chaque page d’article pour le texte intégral. Le robots.txt autorise les pages d’articles avec un délai de 10 s, respecté. Le résumé du flux est conservé si une page ne peut être lue, et la page est retentée plus tard."],
  ["Private Kinshasa news site without a feed. New dated articles (/YYYY/MM/DD/slug) are discovered on the homepage and four section pages and fetched for the full text; robots.txt is checked before every request and requests are spaced at least 5 s apart.", "Site d’information privé de Kinshasa, sans flux. Les nouveaux articles datés (/AAAA/MM/JJ/titre) sont repérés sur la page d’accueil et quatre rubriques, puis lus en texte intégral ; le robots.txt est vérifié avant chaque requête, espacées d’au moins 5 s."],
  ["UN OCHA humanitarian reports whose primary country is the DRC, from the ReliefWeb API (an approved appname is required; reliefweb.int pages are never scraped). Each report belongs to its contributing organisation, credited as the source; reports published only as attachments are skipped.", "Rapports humanitaires d’OCHA dont le pays principal est la RDC, via l’API ReliefWeb (un nom d’application approuvé est requis ; les pages de reliefweb.int ne sont jamais aspirées). Chaque rapport appartient à l’organisation qui le publie, citée comme source ; les rapports publiés uniquement en pièce jointe sont ignorés."],
  ["not configured: request an appname at https://apidoc.reliefweb.int/parameters#appname", "non configuré : demandez un nom d’application sur https://apidoc.reliefweb.int/parameters#appname"],
  ["Initial research register, NOT exhaustive. PA-X DRC subset through 2025; selected disclosed mining contracts and infrastructure instruments. Records include amendments, statements and local agreements. Central-government signature verification pending except where explicitly stated.", "Registre de recherche initial, NON exhaustif. Sous-ensemble RDC de PA-X jusqu’en 2025 ; sélection de contrats miniers et d’instruments d’infrastructure publiés. Comprend avenants, déclarations et accords locaux. Vérification des signatures du gouvernement central en attente, sauf mention explicite."],
  ["Signing-term grouping is descriptive, not causal credit. Unknown dates stay unverified. AI drafts require review. Promises and observed outcomes are separate. No evidence is not zero effectiveness.", "Le regroupement par mandat de signature est descriptif et n’attribue aucun mérite causal. Les dates inconnues restent non vérifiées. Les brouillons IA doivent être relus. Engagements et résultats observés sont distincts. Absence de preuve ne signifie pas efficacité nulle."],
  ["No operator, airframe or route information matched", "Aucune information d’opérateur, d’appareil ou de trajet ne correspond"],
  ["Flagged military in the public ADS-B aircraft database", "Signalé comme militaire dans la base publique ADS-B"],
  ["Operator name indicates a military or state air arm", "Le nom de l’opérateur indique une force aérienne militaire ou d’État"],
  ["Callsign uses the United Nations designator", "L’indicatif utilise le préfixe des Nations unies"],
  ["7500 · unlawful interference", "7500 · intervention illicite"], ["7600 · radio failure", "7600 · panne radio"], ["7700 · general emergency", "7700 · urgence générale"], ["Emergency status broadcast", "Statut d’urgence émis"],
  ["ADSB.lol military feed", "Flux militaire ADSB.lol"], ["adsbdb (routes and aircraft)", "adsbdb (trajets et appareils)"],
  ["Voice budget paused, unconfigured, or exhausted. Check Costs & budgets and your member allowance.", "Budget vocal en pause, non configuré ou épuisé. Vérifiez Coûts et budgets et votre allocation de membre."],
  ["actor not found", "Acteur introuvable"], ["equipment not found", "Équipement introuvable"], ["article not found", "Article introuvable"], ["Unknown agreement", "Accord inconnu"], ["PDF not found", "PDF introuvable"], ["Archived PDF is unavailable", "PDF archivé indisponible"], ["Save an API key first", "Enregistrez d’abord une clé API"], ["Provider connection failed", "Échec de connexion au fournisseur"],
  ["Watch not found", "Veille introuvable"], ["Add at least one term", "Ajoutez au moins un terme"], ["Invalid email address", "Adresse e-mail invalide"], ["Invalid Telegram chat id", "Identifiant Telegram invalide"], ["Name must be 1–80 characters", "Le nom doit comporter 1 à 80 caractères"], ["Watch limit reached (100)", "Limite de veilles atteinte (100)"],
  ["Email is not configured (SMTP_HOST, SMTP_FROM)", "L’e-mail n’est pas configuré (SMTP_HOST, SMTP_FROM)"], ["Telegram is not configured (TELEGRAM_BOT_TOKEN)", "Telegram n’est pas configuré (TELEGRAM_BOT_TOKEN)"],
]);
const SERVER_FR_PATTERNS = [
  [/^Operator “(.+)” is a military or state air arm$/, m => `L’opérateur « ${m[1]} » est une force aérienne militaire ou d’État`],
  [/^Operator “(.+)” is a UN or humanitarian air service$/, m => `L’opérateur « ${m[1]} » est un service aérien de l’ONU ou humanitaire`],
  [/^Airframe registered as a freighter variant \((.+)\)$/, m => `Appareil immatriculé en version cargo (${m[1]})`],
  [/^(\S+) operates dedicated freighters$/, m => `${m[1]} exploite des avions-cargos dédiés`],
  [/^Operator “(.+)” is a cargo carrier$/, m => `L’opérateur « ${m[1]} » est un transporteur de fret`],
  [/^(\S+) airframes fly almost exclusively as freighters in the region$/, m => `Les ${m[1]} volent presque exclusivement en version cargo dans la région`],
  [/^Callsign matches (.+) in the public airline directory; passenger or cargo mission unverified$/, m => `L’indicatif correspond à ${m[1]} dans l’annuaire public des compagnies ; mission passagers ou fret non vérifiée`],
  [/^Rate limited; resuming in (\d+) min$/, m => `Limite de requêtes atteinte ; reprise dans ${m[1]} min`],
];
function srv(t) {
  if (!t || state.lang !== 'fr') return t || '';
  if (SERVER_FR.has(t)) return SERVER_FR.get(t);
  for (const [re, f] of SERVER_FR_PATTERNS) { const m = t.match(re); if (m) return f(m) }
  return t;
}
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
    const err = Error(r.status === 401 ? L('This API requires a token. Open connection settings.', 'Cette API nécessite un jeton. Ouvrez les paramètres de connexion.') : (e?.error ? srv(e.error) : `${L('Unable to connect to the API.', 'Impossible de joindre l’API.')} (${r.status})`));
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
    const [data, status, config, ev, sources, social, posts, accounts, actors, arms, feeds, agreements, costs, team, commodities, radio, flights, aiSettings, watches, alerts, events] = await Promise.all([
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
      api('/v1/commodities').catch(() => null),
      api('/v1/radio').catch(() => null),
      api('/v1/flights').catch(() => null),
      state.auth?.user?.role==='owner' ? api('/v1/settings/ai').catch(() => null) : null,
      api('/v1/watches').catch(() => null),
      api('/v1/alerts?limit=200').catch(() => null),
      api(`/v1/events?days=${state.evDays}`).catch(() => null),
    ]);
    if (state.agDetail?.assessment?.status === 'running') state.agDetail = await api('/v1/agreements/'+state.agDetail.id);
    Object.assign(state, { items: data.items, total: data.total, updated: data.generated_at, status, config, ev, sources, social, posts, accounts, actors, arms, feeds, agreements, costs, team, commodities, radio, flights, aiSettings, watches, alerts, events, error: '' });
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
  ...(state.aiMode ? [['ai-command','spark',L('OKAPI / AI','OKAPI / IA')]] : []),
  ['briefing', 'grid', L('Briefing', 'Synthèse'), state.items.filter(a => a.rank.band === 'high').length, true],
  ['situation', 'map', L('Situation room', 'Vue de situation')],
  ['events', 'history', L('Event log', 'Journal des événements'), (state.events?.items || []).filter(e => Date.now() - Date.parse(e.date) < 7 * 864e5).length],
  ['alerts', 'bell', L('Watchlists & alerts', 'Veilles et alertes'), state.alerts?.unread, true],
  ['reports', 'layers', L('Reports', 'Articles'), state.total],
  ['social', 'radio', L('Social intelligence', 'Veille sociale'), state.social?.archive?.posts],
  ['sources', 'book', L('Primary sources', 'Sources primaires'), state.ev?.items?.length],
  ['actors', 'flag', L('Key figures & sanctions', 'Acteurs et sanctions'), state.actors?.items?.filter(a => a.sanctioned).length],
  ['arms', 'target', L('Arms & equipment', 'Armes et équipements'), state.arms?.items?.filter(i => i.mentions?.total).length],
  ['saved', 'bookmark', L('Saved', 'Sauvegardés'), state.items.filter(a => a.analyst.saved).length],
  ['agreements', 'history', L('Agreements', 'Accords'), state.agreements?.items?.length],
  ['team', 'shield', L('Team & access', 'Équipe et accès')],
  ['ai-settings', 'settings', L('AI settings', 'Paramètres IA')],
  ['costs', 'layers', L('Costs & budgets', 'Coûts et budgets')],
  ['method', 'shield', L('Methodology', 'Méthodologie')],
];
const viewLabel = v => v==='ai-command' ? L('AI command center','Centre de commande IA') : NAV().find(n => n[0] === v)?.[2] || v;
function feedStatuses() {
  const lastFetch = state.items.filter(a => a.record_type !== 'evidence' && !a.coverage).map(a => a.fetched_at).filter(Boolean).sort().at(-1);
  const s = state.social, src = state.sources?.items?.filter(x => !x.id.startsWith('discovered-')) || [];
  const failed = src.filter(x => x.status === 'failed').length;
  return [
    [state.error ? 'bad' : 'ok live', L('News collector', 'Collecteur d’articles'), lastFetch ? age(lastFetch) : '—'],
    ...(state.feeds?.items || []).map(f => [!f.sync || f.configured === false ? 'warn' : f.sync.status === 'ok' ? 'ok' : 'bad', f.name, f.configured === false ? L('not configured', 'non configuré') : f.sync?.checked_at ? age(f.sync.checked_at) : state.feeds.poll_seconds ? L('pending', 'en attente') : L('off', 'arrêt')]),
    [!s ? 'bad' : s.enabled && s.configured ? (s.items.some(i => i.error) ? 'warn' : 'ok') : 'warn', 'X / Twitter', !s ? '—' : s.enabled && s.configured ? `${s.requests_today}/${s.daily_request_limit}` : L('paused', 'pause')],
    [state.status.available ? 'ok' : 'warn', L('AI analyst', 'Analyste IA'), state.status.provider || '—'],
    [failed ? 'warn' : 'ok', L('Documents', 'Documents'), src.length ? `${src.length - failed}/${src.length}` : '—'],
  ];
}
const VIEW_CODES = { briefing: 'BRF', situation: 'SIT', events: 'EVT', alerts: 'ALR', reports: 'RPT', social: 'SOC', sources: 'SRC', actors: 'ACT', arms: 'ARM', saved: 'SAV', method: 'MTH' };
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
  const nav = NAV().filter(([v])=>!['costs','ai-settings'].includes(v)||state.auth?.user?.role==='owner');
  return `<div class="app"><aside class="rail" aria-label="${L('Main navigation', 'Navigation principale')}">
    <a class="brand" href="#briefing" data-act="nav" data-arg="briefing"><span class="brand-mark">${icon('globe')}</span><div><strong>CONGO SIGNAL</strong><small>${L('DRC situational awareness', 'Veille de situation · RDC')}</small></div></a>
    <nav>${nav.map(([id, ic, label, count, hot], i) => `${id === 'social' ? `<div class="nav-label">${L('Research', 'Recherche')}</div>` : i === 0 ? `<div class="nav-label">${L('Monitor', 'Veille')}</div>` : ''}<button class="nav-item ${state.view === id ? 'active' : ''}" data-act="nav" data-arg="${id}" ${state.view === id ? 'aria-current="page"' : ''} title="${esc(label)} · ${i + 1}">${icon(ic)}<span>${esc(label)}</span>${count ? `<b class="count ${hot ? 'hot' : ''}">${compact(count)}</b>` : `<kbd class="nav-key">${i + 1}</kbd>`}</button>`).join('')}</nav>
    <div class="rail-bottom"><p>${esc(state.auth?.user?.email||'')} · ${esc(state.auth?.user?.role||'')}</p>${state.auth?.mode==='oidc'?`<button class="btn" data-act="logout">${L('Sign out','Déconnexion')}</button>`:''}<div class="nav-label" style="padding:4px 4px 2px">${L('Collection', 'Collecte')}</div>${feedStatuses().map(([c, label, v]) => `<div class="feed-status"><i class="dot ${c}"></i>${esc(label)}<small>${esc(v)}</small></div>`).join('')}</div>
  </aside><div class="workspace"><div class="chrome">
    <div class="classbar"><span>${L('UNCLASSIFIED // OPEN SOURCE', 'NON CLASSIFIÉ // SOURCES OUVERTES')}</span><span class="classbar-mid">CONGO SIGNAL · ${L('DRC SITUATIONAL AWARENESS', 'VEILLE DE SITUATION RDC')}</span><span>${L('ATTRIBUTED · UNVERIFIED UNLESS STATED', 'ATTRIBUÉ · NON VÉRIFIÉ SAUF MENTION')}</span></div>
    <header class="topbar">
      <div class="crumb"><span class="crumb-code">${VIEW_CODES[state.view] || 'OPS'}</span><strong>${esc(viewLabel(state.view))}</strong></div>
      <button class="omni" data-act="palette" aria-label="${L('Search everything', 'Tout rechercher')}">${icon('search')}<span>${L('Search reports, places, actors, weapons…', 'Rechercher articles, lieux, acteurs, armes…')}</span><kbd>⌘K</kbd></button>
      <div class="clocks" aria-label="${L('Current time', 'Heure actuelle')}"><span><small>UTC</small><b id="clock-utc">${clockText(0)}</b></span><span><small>KIN</small><b id="clock-kin">${clockText(1)}</b></span></div>
      ${onAirChip()}<button class="iconbtn bell ${state.alerts?.unread ? 'has' : ''}" data-act="nav" data-arg="alerts" aria-label="${esc(L(`Alerts: ${state.alerts?.unread || 0} unread`, `Alertes : ${state.alerts?.unread || 0} non lues`))}" title="${esc(L('Watchlists & alerts', 'Veilles et alertes'))}">${icon('bell')}${state.alerts?.unread ? `<b>${state.alerts.unread > 99 ? '99+' : state.alerts.unread}</b>` : ''}</button><span class="live-pill ${state.error ? 'down' : ''}"><i class="dot ${state.error ? 'bad' : 'ok live'}"></i>${state.error ? L('OFFLINE', 'HORS LIGNE') : L('LIVE', 'EN DIRECT')}</span>
      <div class="seg" role="group" aria-label="${L('Language', 'Langue')}"><button data-act="lang" data-arg="en" class="${state.lang === 'en' ? 'active' : ''}">EN</button><button data-act="lang" data-arg="fr" class="${state.lang === 'fr' ? 'active' : ''}">FR</button></div>
      <button class="btn sm ai-mode-switch" data-act="ai-mode" aria-pressed="${state.aiMode}">${icon('spark')}${state.aiMode?L('Exit AI mode','Quitter le mode IA'):L('AI mode','Mode IA')}</button>
      <button class="btn sm" data-act="voice-open">Okapi</button>
      <button class="iconbtn" data-act="theme" aria-label="${L('Toggle theme', 'Changer de thème')}" title="${L('Toggle theme', 'Changer de thème')}">${icon(state.theme === 'light' ? 'moon' : 'sun')}</button>
      <button class="iconbtn" data-act="settings" aria-label="${L('Connection settings', 'Paramètres de connexion')}">${icon('settings')}</button>
    </header>${['situation','ai-command'].includes(state.view) ? '' : wire()}</div>
    <main class="view view-${state.view} ${state.enter ? 'enter' : ''}">${state.error && state.view !== 'situation' ? `<div class="errorbar">${icon('alert')}${esc(state.error)} <button class="linkbtn" data-act="settings">${L('Connection settings', 'Paramètres de connexion')}</button></div>` : ''}${content}</main></div></div>`;
}
/// HH:MM:SS at UTC + offset hours (Kinshasa is UTC+1 year-round).
function clockText(offset) { return new Date(Date.now() + offset * 3600000).toISOString().slice(11, 19) }
setInterval(() => { const u = $('#clock-utc'), k = $('#clock-kin'); if (u) u.textContent = clockText(0); if (k) k.textContent = clockText(1) }, 1000);
const pageHead = (eyebrow, h1, p, actions = '') => `<section class="page-head"><div><div class="eyebrow">${eyebrow}</div><h1>${h1}</h1>${p ? `<p>${p}</p>` : ''}</div><div class="actions">${actions}</div></section>`;
const btnExport = () => `<button class="btn" data-act="export">${icon('download')}${L('Export JSON', 'Exporter JSON')}</button>`;
const btnCollect = () => `<button class="btn" data-act="collect">${icon('refresh')}${L('Collect latest', 'Actualiser la collecte')}</button>`;
const btnBatch = () => `<button class="btn primary" data-act="batch">${icon('spark')}${L('Analyze top 5', 'Analyser les 5 premiers')}</button>`;

/// Stable selector for an element across re-renders (id, else its action/argument or model binding),
/// so keyboard focus survives the full re-render that every state change and poll performs.
function focusKey(el) {
  if (el.id) return `#${CSS.escape(el.id)}`;
  const attr = n => el.getAttribute(n) == null ? '' : `[${n}="${CSS.escape(el.getAttribute(n))}"]`;
  const key = attr('data-act') + attr('data-arg') || attr('data-model') || attr('data-station') || attr('data-palette');
  return key ? el.tagName.toLowerCase() + key : '';
}
let overlayReturn = '';
function render() {
  if (!state.loaded) return;
  if(state.loginRequired){mapHost?.remove();$('#app').innerHTML=`<main class="panel" style="max-width:560px;margin:12vh auto;padding:32px"><h1>Congo Signal</h1><p>${L('Sign in with your invited email to access your workspace.','Connectez-vous avec votre adresse invitée pour accéder à votre espace.')}</p><a class="btn primary" href="/auth/login">${L('Sign in','Se connecter')}</a></main>`;return}
  const active = document.activeElement;
  const keep = active && active !== document.body && !active.closest('#overlay') && active.closest('#app') ? { key: focusKey(active), start: active.selectionStart, end: active.selectionEnd } : null;
  const scrolls = $$('#app [data-scroll]').map(el => [el.dataset.scroll, el.scrollTop]);
  document.documentElement.lang = state.lang;
  document.documentElement.dataset.aiMode=state.aiMode?'on':'off';
  document.documentElement.dataset.aiMotion=state.aiMotion?'on':'off';
  document.documentElement.dataset.theme=state.aiMode?'dark':state.theme;
  mapHost?.remove();
  const views = { briefing: briefingView, situation: situationView, reports: reportsView, saved: reportsView, social: socialView, sources: sourcesView, actors: actorsView, arms: armsView, method: methodView, agreements: agreementsView, costs: costsView, team: teamView, "ai-settings": aiSettingsView, "ai-command": aiCommandView, events: eventsView, alerts: alertsView };
  $('#app').innerHTML = shell(views[state.view]());
  if(state.view==='ai-settings') loadAiModels();
  for (const [k, top] of scrolls) { const el = $(`#app [data-scroll="${k}"]`); if (el) el.scrollTop = top }
  if (keep?.key) { const el = $(`#app ${keep.key}`); if (el) { el.focus({ preventScroll: true }); try { el.setSelectionRange(keep.start, keep.end) } catch {} } }
  mountMap();
  mountHolo();
  state.enter = false;
}
function go(view) {
  if (!VIEWS.includes(view)) return;
  if(view==='ai-command'){state.aiMode=true;store.set('congo-ai-mode','1')}
  if(view!=='situation'){flightMarkers.forEach(m=>m.remove());flightMarkers=[];if(mapInstance)mapInstance.__flightSignature='';}
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
  return pageHead(`${L('DRC', 'RDC')} · ${L('OPEN-SOURCE MONITORING', 'VEILLE EN SOURCES OUVERTES')} · ${esc(today)}`, L('Daily briefing', 'Synthèse du jour'), L('What changed across news, primary documents and public conversation — ranked for analyst attention, linked to every source.', 'Ce qui a changé dans les articles, les documents primaires et la conversation publique — classé pour l’analyste, relié à chaque source.'), `${btnBrief()}${btnExport()}${btnCollect()}${btnBatch()}`)
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
      <div class="toolbar"><label class="search">${icon('search')}<input id="report-q" class="input" type="search" data-model="q" value="${esc(state.q)}" placeholder="${L('Search title, text, places, AI briefs…', 'Rechercher titre, texte, lieux, synthèses…')}" aria-label="${L('Search reports', 'Rechercher')}"></label>${state.q.trim() ? `<button class="btn sm" data-act="watch-quick" data-arg="${esc(`search|${state.q.trim().replace(/\|/g, ' ')}|${state.q.trim()}`)}" title="${L('Alert me when new reports match this search', 'M’alerter quand de nouveaux articles correspondent')}">${icon('bell')}${L('Save search', 'Enregistrer la recherche')}</button>` : ''}
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
      <p class="notice">${esc(srv(s?.notice))} ${L('Refreshing this screen never triggers a paid X request; account histories are fetched only when you ask.', 'Actualiser cet écran ne déclenche aucune requête X payante ; les historiques ne sont récupérés qu’à votre demande.')}</p>`;
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
      <p class="notice">${esc(srv(data.notice))}</p>`;
  }
  const scroller = $('#overlay .drawer'), top = scroller?.scrollTop || 0;
  $('#overlay').innerHTML = `<div class="shade" data-close><section class="drawer" role="dialog" aria-modal="true" aria-label="${esc(L('Account dossier', 'Dossier de compte'))} @${esc(d.handle)}"><div class="drawer-top"><span class="eyebrow">${icon('user')}${L('ACCOUNT DOSSIER', 'DOSSIER DE COMPTE')} · X</span><div class="actions">${a ? `<button class="btn sm" data-act="export-dossier">${icon('download')}JSON</button>` : ''}<a class="btn sm" href="https://x.com/${esc(d.handle)}" target="_blank" rel="noopener noreferrer">${icon('external')}${L('Open on X', 'Ouvrir sur X')}</a><button class="iconbtn" data-act="close" aria-label="${L('Close', 'Fermer')}">${icon('close')}</button></div></div><div class="drawer-body">${body}</div></section></div>`;
  const drawer = $('#overlay .drawer'); if (drawer) drawer.scrollTop = top;
  if (!document.activeElement?.closest('#overlay')) $('#overlay [data-act="close"]')?.focus();
}

// ---------------------------------------------------------------------------------------------
// Situation room (map-first)

// Versioned, attributed reference data; independent of the report time window.
state.geology = null; state.mineralLayers = ['IPIS', 'USGS']; state.mineralQuery = '';
state.mineralSearch = ''; state.mineralProvince = ''; state.mineralKind = ''; state.mineralSelected = ''; state.mineralVisible = 15; state.nearbyRadius = '25'; state.mapFocus = false;
/// Filtered mineral sites, memoised on the filters and the dataset: the minerals view, map and
/// selection all ask for it several times per render, and 3,672 sites made each keystroke slow.
let mineralMemo = { src: null, key: '', out: [] };
function mineralFeatures() {
  const src = state.geology?.features || [];
  const key = JSON.stringify([state.mineralSearch, state.mineralLayers, state.mineralQuery, state.mineralFocusLabels, state.mineralProvince, state.mineralKind]);
  if (mineralMemo.src === src && mineralMemo.key === key) return mineralMemo.out;
  const q = norm(state.mineralSearch.trim());
  const out = src.filter(({properties:p}) => state.mineralLayers.includes(p.dataset) &&
    (!state.mineralQuery || p.minerals.includes(state.mineralQuery)) &&
    (!state.mineralFocusLabels?.length || p.minerals.some(m => state.mineralFocusLabels.includes(m))) &&
    (!state.mineralProvince || p.province === state.mineralProvince) &&
    (!state.mineralKind || p.kind === state.mineralKind) &&
    (!q || (p._norm ??= norm([p.name,p.province,...p.minerals].join(' '))).includes(q)));
  mineralMemo = { src, key, out };
  return out;
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
  state.mineralSelected=id; state.mapFocus=false;
  if (state.mineralMode && location.hash !== '#mine/' + encodeURIComponent(id)) history.replaceState(null, '', '#mine/' + encodeURIComponent(id));
  render();
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
  return `<div class="pane-section"><h2>${L('Minerals & geology', 'Minerais et géologie')}</h2>${state.mineralMode ? '' : `<button class="btn primary" data-act="map-mode" data-arg="minerals">${icon('layers')}${L('Open minerals mode', 'Ouvrir le mode minerais')}</button>`}
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
  // isStyleLoaded() is false while any tiles load (often, under the geology layer), so check the
  // style and treat "style not done loading" as retry-on-idle rather than a fatal render error.
  if (!mapInstance || !state.geology) return;
  // getStyle()/addSource throw "Style is not done loading" in that window; the idle event retries.
  try { mountMineralLayers() } catch {}
}
function mountMineralLayers() {
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

// Public civilian aircraft: source positions only, no extrapolation or mission inference.
state.flightMode=location.hash==='#flights';state.flights=null;state.flightQuery='';state.flightCategory='';state.flightSelected='';state.flightApproach=true;
const FLIGHT_CATS=['military','humanitarian','cargo','airline','unknown'];
const flightLabel=k=>({military:L('Military','Militaire'),humanitarian:L('UN / aid','ONU / humanitaire'),cargo:L('Cargo','Cargo'),airline:L('Airline','Compagnie'),unknown:L('Unclassified','Non classé'),cargo_operator:L('Cargo','Cargo')})[k]||k;
const flightColor=f=>({military:'#ff5d6c',humanitarian:'#7ab8ff',cargo:'#e5ad53',cargo_operator:'#e5ad53',airline:'#63cbd0'})[f.category||f]||'#a4acb5';
const flightName=f=>f.callsign||f.registration||f.icao24;
const airportShort=a=>a?(a.iata||a.icao||a.name||'?'):'?';
function flightRows(){const q=norm(state.flightQuery||'');return (state.flights?.items||[]).filter(f=>(Date.now()/1000-f.observed_at)<=600&&(state.flightApproach||f.region!=='approach')&&(!state.flightCategory||f.category===state.flightCategory)&&(!q||norm([f.callsign,f.registration,f.icao24,f.aircraft_type,f.type_name,f.airline,f.owner,f.route?.origin?.city,f.route?.destination?.city,f.route?.origin?.iata,f.route?.destination?.iata].filter(Boolean).join(' ')).includes(q))).sort((a,b)=>FLIGHT_CATS.indexOf(a.category)-FLIGHT_CATS.indexOf(b.category)||(a.region==='drc'?0:1)-(b.region==='drc'?0:1)||flightName(a).localeCompare(flightName(b)))}
const planeSvg=`<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 2c.8 0 1.4.9 1.4 2v5.3l7.6 4.4v2l-7.6-2.3v4.8l2.2 1.6V21L12 20l-3.6 1v-1.2l2.2-1.6v-4.8L3 15.7v-2l7.6-4.4V4c0-1.1.6-2 1.4-2Z"/></svg>`;
function routeCard(f){const r=f.route;if(!r)return `<div class="flight-route none"><span>${L('Route','Trajet')}</span><b>${L('No published route for this callsign','Aucun trajet publié pour cet indicatif')}</b></div>`;const ap=(a,lab)=>`<div><small>${lab}</small><strong>${esc(airportShort(a))}</strong><span>${esc([a.city||a.name,a.country].filter(Boolean).join(' · '))}</span></div>`;return `<div class="flight-route ${r.plausible?'':'doubt'}">${ap(r.origin,L('FROM','DÉPART'))}<i class="fr-line" style="--c:${flightColor(f)}"><em>${planeSvg}</em></i>${ap(r.destination,L('TO','ARRIVÉE'))}<p>${r.plausible?L('Published schedule for this callsign; consistent with the observed position.','Horaire publié pour cet indicatif ; cohérent avec la position observée.'):L('Published schedule does not match the observed position: the callsign may be reused, so treat this route as unverified.','L’horaire publié ne correspond pas à la position observée : l’indicatif peut être réutilisé, trajet non vérifié.')} <a href="${esc(r.source_url)}" target="_blank" rel="noopener noreferrer">adsbdb ↗</a></p></div>`}
function flightDetail(f){if(!f)return `<div class="pane-section muted" style="font-size:12.5px">${L('Select an aircraft on the map or in the list to see its route, operator and classification basis.','Sélectionnez un aéronef sur la carte ou dans la liste pour voir son trajet, son opérateur et la base de classification.')}</div>`;const v=x=>x==null||x===''?'—':x;return `<section class="flight-detail" style="--c:${flightColor(f)}"><div class="fd-head"><span class="fd-icon" style="transform:rotate(${f.track_degrees||0}deg)">${planeSvg}</span><div><div class="eyebrow">${esc(f.icao24)} · ${f.region==='drc'?L('IN DRC AIRSPACE','ESPACE AÉRIEN RDC'):L('APPROACHES','ABORDS')}</div><h2>${esc(flightName(f))}</h2></div></div>${f.emergency?`<p class="fd-emergency">${icon('alert')} ${esc(srv(f.emergency))}</p>`:''}<p class="fd-cat"><b>${esc(flightLabel(f.category))}</b> ${esc(srv(f.classification_basis))}</p><button class="btn sm" data-act="watch-quick" data-arg="${esc(`aircraft|${(f.registration||f.callsign||f.icao24||'').trim()}|${flightName(f)}`)}">${icon('bell')} ${L('Watch this aircraft','Suivre cet aéronef')}</button>${routeCard(f)}<dl class="props">${[[L('Aircraft','Appareil'),v(f.type_name||f.aircraft_type)],[L('Registration','Immatriculation'),v(f.registration)],[L('Registered owner','Propriétaire'),v([f.owner,f.owner_country].filter(Boolean).join(' · '))],[L('Airline (callsign)','Compagnie (indicatif)'),v(f.airline)],[L('Altitude','Altitude'),f.altitude_ft==null?'—':`${nf(Math.round(f.altitude_ft))} ft`],[L('Ground speed','Vitesse sol'),f.ground_speed_knots==null?'—':`${Math.round(f.ground_speed_knots)} kt`],[L('Vertical rate','Taux vertical'),f.vertical_rate_fpm==null?'—':`${f.vertical_rate_fpm>0?'+':''}${Math.round(f.vertical_rate_fpm)} ft/min`],[L('Heading','Cap'),f.track_degrees==null?'—':`${Math.round(f.track_degrees)}°`],[L('Squawk','Transpondeur'),v(f.squawk)],[L('Observed','Observé'),`${(n=>L(`${n} min ago`,`il y a ${n} min`))(Math.max(0,Math.round((Date.now()/1000-f.observed_at)/60)))} · ${(f.sources||[]).map(srv).join(' + ')||'ADS-B'}`]].map(([k,x])=>`<dt>${esc(k)}</dt><dd>${esc(x)}</dd>`).join('')}</dl><p class="muted" style="font-size:11.5px">${L('Classification states what matched (database flag, operator, airframe or callsign). It does not establish the cargo, passengers or mission of this flight.','La classification indique ce qui correspond (base de données, opérateur, appareil ou indicatif). Elle n’établit ni la cargaison, ni les passagers, ni la mission de ce vol.')}</p></section>`}
function flightMap(){if(state.config?.mapbox_public_token&&window.mapboxgl&&!mapFailed)return '<div id="map-slot"></div>';const xy=(lon,lat)=>[25+(lon-12)*18,25+(5.5-lat)*13.8];const path=state.geometry?.features?.[0]?.geometry.coordinates.map(r=>r.map((p,i)=>`${i?'L':'M'}${xy(...p).join(',')}`).join(' ')+'Z').join(' ')||'';return `<div class="static-map"><svg viewBox="0 0 405 290" aria-label="${L('Flight map','Carte des vols')}"><path class="map-country" d="${path}"/>${flightRows().map(f=>{const [x,y]=xy(f.longitude,f.latitude);return `<circle cx="${x}" cy="${y}" r="4" fill="${flightColor(f)}" data-act="flight-select" data-arg="${esc(f.icao24)}" tabindex="0" role="button" aria-label="${esc(flightName(f))}"><title>${esc(flightName(f))}</title></circle>`}).join('')}</svg></div>`}
function flightSourceStatus(d){if(!d)return L('Unavailable','Indisponible');if(!d.enabled)return L('Disabled','Désactivé');return ({ok:L('Scan complete','Scan terminé'),partial:L('Partial scan','Scan partiel'),pending:L('First scan in progress','Premier scan en cours')})[d.status]||L('Sources unavailable','Sources indisponibles')}
function flightsSituation(){const d=state.flights,all=(d?.items||[]).filter(f=>(Date.now()/1000-f.observed_at)<=600&&(state.flightApproach||f.region!=='approach')),rows=flightRows(),selected=rows.find(f=>f.icao24===state.flightSelected)||all.find(f=>f.icao24===state.flightSelected);const count=k=>all.filter(f=>f.category===k).length,inDrc=all.filter(f=>f.region==='drc').length;const next=d?.next_check_at?Math.max(0,Math.round((d.next_check_at-Date.now()/1000)/60)):null;
return `<div class="situation flights-view ${state.mapFocus?'map-focus':''}"><aside class="filters-pane" data-scroll="flights">
 <div class="pane-section"><div class="eyebrow">${L('DRC', 'RDC')} · ADS-B · ${L('LIVE AIR PICTURE','SITUATION AÉRIENNE')}</div><div style="font-size:13px;color:var(--text-2)"><b class="num" style="color:var(--text)">${all.length}</b> ${L('aircraft','aéronefs')} · ${inDrc} ${L('inside the DRC','dans la RDC')}</div>
  <label class="search">${icon('search')}<input class="input" id="flight-search" data-model="flightQuery" value="${esc(state.flightQuery)}" placeholder="${L('Callsign, airline, airport, owner…','Indicatif, compagnie, aéroport, propriétaire…')}"></label>
  <div class="flight-cats"><button class="${state.flightCategory?'':'active'}" data-act="flight-cat" data-arg="">${L('All','Tous')}<b>${all.length}</b></button>${FLIGHT_CATS.map(k=>`<button class="${state.flightCategory===k?'active':''}" data-act="flight-cat" data-arg="${k}" style="--c:${flightColor(k)}"><i></i>${esc(flightLabel(k))}<b>${count(k)}</b></button>`).join('')}</div>
  <label class="check"><input type="checkbox" data-model="flightApproach" ${state.flightApproach?'checked':''}>${L('Include approaches (Kigali, Entebbe, Brazzaville, Ndola…)','Inclure les abords (Kigali, Entebbe, Brazzaville, Ndola…)')}</label></div>
 <div class="pane-section"><h2>${L('Sources','Sources')}<span class="muted" style="font-weight:400;font-size:11px">${esc(flightSourceStatus(d))}</span></h2>${(d?.providers||[]).map(p=>`<div class="feed-status" style="padding:3px 0"><i class="dot ${p.tiles_successful&&p.tiles_successful===p.tiles_attempted?'ok':p.tiles_successful?'warn':'bad'}"></i>${esc(srv(p.name))}<small>${p.tiles_attempted>1?`${p.tiles_successful}/${p.tiles_attempted} · `:''}${p.aircraft} ${L('seen','vus')}</small></div>${(p.errors||[]).slice(0,1).map(e=>`<p class="notice" style="margin:2px 0 6px">${esc(srv(e))}</p>`).join('')}`).join('')||`<p class="muted">${L('Waiting for the first scan.','En attente du premier scan.')}</p>`}
  <p class="muted" style="font-size:11.5px">${d?.checked_at?`${L('Last scan','Dernier scan')} ${age(d.checked_at)}`:''}${next!=null?` · ${L('next in','prochain dans')} ${next} min`:''} · ${nf(d?.requests_today||0)} ${L('free requests today','requêtes gratuites aujourd’hui')}</p></div>
 <div class="pane-section"><h2>${L('Coverage','Couverture')}</h2><p class="flight-coverage">${L('Community ADS-B networks have very few receivers in central Africa, so most aircraft over the DRC are not visible here and an empty map does not mean empty airspace. Positions are never predicted.','Les réseaux ADS-B communautaires ont très peu de récepteurs en Afrique centrale : la plupart des aéronefs au-dessus de la RDC ne sont pas visibles et une carte vide ne signifie pas un ciel vide. Aucune position n’est extrapolée.')}</p><p class="flight-coverage">${L('Military aircraft that broadcast ADS-B are shown and labelled. Aircraft enrolled by their owners in privacy programmes (PIA/LADD) are not shown.','Les aéronefs militaires qui émettent en ADS-B sont affichés et signalés. Les aéronefs inscrits par leurs propriétaires à des programmes de confidentialité (PIA/LADD) ne sont pas affichés.')}</p><p style="font-size:11.5px"><a href="https://api.adsb.lol/docs" target="_blank" rel="noopener">ADSB.lol · ODbL ↗</a> · <a href="https://adsb.fi" target="_blank" rel="noopener">adsb.fi ↗</a> · <a href="https://www.adsbdb.com" target="_blank" rel="noopener">adsbdb ↗</a></p></div>
</aside><section class="stage"><div class="map-area">${flightMap()}
 <div class="hud" aria-hidden="true"><i class="hud-c tl"></i><i class="hud-c tr"></i><i class="hud-c bl"></i><i class="hud-c br"></i></div>
 <div class="map-float tl glass"><strong>${icon('globe')} ${L('Air picture','Situation aérienne')}</strong>${all.length} ${L('aircraft','aéronefs')}${count('military')?` · <span style="color:#ff5d6c">${count('military')} ${L('military','militaires')}</span>`:''}${count('cargo')?` · <span style="color:#e5ad53">${count('cargo')} cargo</span>`:''}</div>
 <div class="map-float tr"><div class="seg glass mode-switch" style="padding:2px">${[['news','News','Actualité'],['minerals','Minerals','Minerais'],['radio','Radio','Radio'],['flights','Flights','Vols']].map(([id,en,fr])=>`<button class="${id==='flights'?'active':''}" data-act="map-mode" data-arg="${id}">${L(en,fr)}</button>`).join('')}</div><button class="btn sm glass" data-act="map-focus">${state.mapFocus?L('Show panels','Afficher les panneaux'):L('Expand map','Agrandir')}</button><button class="btn sm glass" data-act="reset-map">${L('Reset map','Recentrer')}</button></div>
 <div class="map-float bl glass flight-legend">${FLIGHT_CATS.map(k=>`<span><i style="background:${flightColor(k)}"></i>${esc(flightLabel(k))}</span>`).join('')}<span><i class="faded"></i>${L('Approaches','Abords')}</span></div>
</div></section><aside class="rail-pane"><div class="mine-dossier" data-scroll="flight-list">${flightDetail(selected)}<div class="rail-head" style="padding:12px 0 6px"><h2>${L('Observed aircraft','Aéronefs observés')}</h2><small class="muted">${rows.length}</small></div>${rows.map(f=>`<button class="flight-row ${selected?.icao24===f.icao24?'selected':''} ${f.region==='approach'?'approach':''}" data-act="flight-select" data-arg="${esc(f.icao24)}" style="--c:${flightColor(f)}"><span class="fr-plane" style="transform:rotate(${f.track_degrees||0}deg)">${planeSvg}</span><div><strong>${esc(flightName(f))}${f.emergency?' <em class="fr-sos">SOS</em>':''}</strong><small>${f.route?`${esc(airportShort(f.route.origin))} → ${esc(airportShort(f.route.destination))}${f.route.plausible?'':' ?'}`:esc(f.airline||f.owner||f.type_name||f.aircraft_type||'—')}</small></div><span class="fr-meta"><b>${esc(flightLabel(f.category))}</b><small>${f.altitude_ft==null?'—':`FL${String(Math.round(f.altitude_ft/100)).padStart(3,'0')}`}</small></span></button>`).join('')||`<div class="empty" style="padding:18px 0"><strong>${L('No aircraft visible right now','Aucun aéronef visible pour le moment')}</strong><p class="muted" style="font-size:12px">${L('Receiver coverage over the DRC is sparse, especially at night. The next scan runs automatically.','La couverture des récepteurs au-dessus de la RDC est faible, surtout la nuit. Le prochain scan est automatique.')}</p></div>`}</div></aside></div>`}
let flightMarkers=[];
function mountFlights(){const map=mapInstance;if(!map)return;const on=state.flightMode&&state.view==='situation';if(on){try{for(const id of ['mineral-clusters','mineral-counts','mineral-sites','mineral-selection'])if(map.getLayer(id))map.setLayoutProperty(id,'visibility','none')}catch{}}const rows=on?flightRows():[];const sel=rows.find(f=>f.icao24===state.flightSelected);
 // Route of the selected aircraft: origin → observed position → destination.
 try{const coords=sel?.route?[[sel.route.origin.lon,sel.route.origin.lat],[sel.longitude,sel.latitude],[sel.route.destination.lon,sel.route.destination.lat]].filter(p=>Number.isFinite(p[0])&&Number.isFinite(p[1])):[];
  const fc={type:'FeatureCollection',features:coords.length===3?[{type:'Feature',properties:{part:'flown'},geometry:{type:'LineString',coordinates:coords.slice(0,2)}},{type:'Feature',properties:{part:'ahead'},geometry:{type:'LineString',coordinates:coords.slice(1)}},...[[sel.route.origin,coords[0]],[sel.route.destination,coords[2]]].map(([a,c])=>({type:'Feature',properties:{label:`${airportShort(a)} · ${a.city||a.name||''}`},geometry:{type:'Point',coordinates:c}}))]:[]};
  if(map.getSource('flight-route'))map.getSource('flight-route').setData(fc);else{map.addSource('flight-route',{type:'geojson',data:fc});map.addLayer({id:'flight-route-line',type:'line',source:'flight-route',filter:['==',['geometry-type'],'LineString'],paint:{'line-color':'#ffffff','line-width':1.6,'line-opacity':['match',['get','part'],'flown',0.35,0.85],'line-dasharray':[2,2]}});map.addLayer({id:'flight-route-airports',type:'circle',source:'flight-route',filter:['==',['geometry-type'],'Point'],paint:{'circle-radius':5,'circle-color':'#0b1512','circle-stroke-color':'#ffffff','circle-stroke-width':1.5}});map.addLayer({id:'flight-route-labels',type:'symbol',source:'flight-route',filter:['==',['geometry-type'],'Point'],layout:{'text-field':['get','label'],'text-size':11,'text-offset':[0,1.3],'text-anchor':'top'},paint:{'text-color':'#ffffff','text-halo-color':'#000','text-halo-width':1.2}})}}catch{}
 const sig=JSON.stringify([on,state.flightSelected,rows.map(f=>[f.icao24,f.longitude,f.latitude,f.track_degrees,f.category,f.region])]);if(map.__flightSignature===sig)return;map.__flightSignature=sig;flightMarkers.forEach(m=>m.remove());flightMarkers=[];
 for(const f of rows){const el=document.createElement('button');el.className='flight-marker'+(f.icao24===state.flightSelected?' selected':'')+(f.region==='approach'?' approach':'')+(f.emergency?' emergency':'')+(f.category==='military'?' military':'');el.style.setProperty('--c',flightColor(f));el.innerHTML=`<span class="fm-plane" style="transform:rotate(${f.track_degrees||0}deg)">${planeSvg}</span><span class="fm-label">${esc(flightName(f))}${f.route?` <small>${esc(airportShort(f.route.origin))}→${esc(airportShort(f.route.destination))}</small>`:''}</span>`;el.title=`${flightName(f)} · ${flightLabel(f.category)}`;el.setAttribute('aria-label',el.title);el.onclick=()=>selectFlight(f.icao24);flightMarkers.push(new mapboxgl.Marker({element:el,anchor:'left',offset:[-14,0]}).setLngLat([f.longitude,f.latitude]).addTo(map))}}
function selectFlight(id){state.flightSelected=state.flightSelected===id?'':id;render();const f=flightRows().find(f=>f.icao24===state.flightSelected);if(!f||!mapInstance)return;const r=f.route,pts=[[f.longitude,f.latitude],...(r?[[r.origin.lon,r.origin.lat],[r.destination.lon,r.destination.lat]]:[])].filter(p=>Number.isFinite(p[0])&&Number.isFinite(p[1]));if(pts.length>1&&r?.plausible){const b=pts.reduce((b,p)=>b.extend(p),new mapboxgl.LngLatBounds(pts[0],pts[0]));mapInstance.fitBounds(b,{padding:80,maxZoom:6,duration:reducedMotion()?0:900})}else mapInstance.easeTo({center:pts[0],zoom:Math.max(6,mapInstance.getZoom()),duration:reducedMotion()?0:700})}

function situationView() {
  if (state.flightMode) return flightsSituation();
  if (state.radioMode) return radioSituation();
  if (state.mineralMode) return mineralsSituation();
  const items = situationItems(), ps = placesList(situationItems(true));
  const timeline = (() => {
    const counts = new Map(countBy(situationItems(false, true), a => kinDay(a.published_at)));
    const days = [...counts.keys()].sort(); const max = Math.max(1, ...counts.values());
    return `<div class="timeline-head"><strong>${L('Publication timeline', 'Chronologie des publications')}</strong><span>${L('Kinshasa dates (UTC+1)', 'Dates de Kinshasa (UTC+1)')}</span>${state.selectedDay ? `<button class="linkbtn" data-act="all-days">${L('All days', 'Tous les jours')}</button>` : ''}</div><div class="day-bars">${days.map(d => `<button class="day-bar ${state.selectedDay === d ? 'active' : ''}" data-act="day" data-arg="${d}" aria-pressed="${state.selectedDay === d}" data-tip="${esc(`${dayLabel(d)} · ${counts.get(d)} ${L('reports', 'articles')}`)}"><i style="height:${Math.max(6, counts.get(d) / max * 100)}%"></i></button>`).join('') || `<small class="muted">${L('No dated reports in this window.', 'Aucun article daté dans cette période.')}</small>`}</div>`;
  })();
  return `<div class="situation ${state.mapFocus ? 'map-focus' : ''}">
    <aside class="filters-pane" data-scroll="filters">
      <div class="pane-section"><div class="eyebrow">${L('DRC', 'RDC')} · ${L('SITUATION ROOM', 'VUE DE SITUATION')}</div><div style="font-size:13px;color:var(--text-2)"><b class="num" style="color:var(--text)">${nf(items.length)}</b> ${L('reports in view', 'articles affichés')} · ${nf(state.items.length)} ${L('loaded', 'chargés')}</div><label class="search">${icon('search')}<input id="sit-q" class="input" data-model="q" value="${esc(state.q)}" placeholder="${L('Filter reports…', 'Filtrer les articles…')}" aria-label="${L('Filter reports', 'Filtrer les articles')}"></label></div>
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
      <div class="map-float tr"><div class="seg glass mode-switch" style="padding:2px"><button class="active" data-act="map-mode" data-arg="news">${L('News', 'Actualité')}</button><button data-act="map-mode" data-arg="minerals">${L('Minerals', 'Minerais')}</button><button data-act="map-mode" data-arg="radio">${icon('radio')}${L('Radio', 'Radio')}</button><button data-act="map-mode" data-arg="flights">✈ ${L('Flights','Vols')}</button></div><button class="btn sm glass" data-act="map-focus" aria-pressed="${state.mapFocus}">${state.mapFocus ? L('Show panels','Afficher les panneaux') : L('Expand map','Agrandir')}</button><div class="seg glass" style="padding:2px"><button class="${state.basemap === 'base' ? 'active' : ''}" data-act="base" data-arg="base">${L('Map', 'Carte')}</button><button class="${state.basemap === 'satellite' ? 'active' : ''}" data-act="base" data-arg="satellite">${L('Satellite', 'Satellite')}</button></div><button class="btn sm glass" data-act="tilt">${state.tilt ? L('Flat', 'À plat') : L('3D tilt', 'Inclinée')}</button><button class="btn sm glass" data-act="reset-map">${icon('globe')}${L('Reset', 'Recentrer')}</button></div>
      <div class="map-float bl glass">${mapFailed ? L('Interactive map unavailable; showing outline.', 'Carte interactive indisponible ; contour affiché.') : L('Markers are approximate place mentions, not verified incident locations.', 'Les repères sont des lieux cités approximatifs, pas des incidents vérifiés.')}${state.basemap === 'satellite' ? ` ${L('Imagery is not live.', 'Imagerie non en direct.')}` : ''}</div>
    </div><div class="timeline">${timeline}</div></section>
    <aside class="rail-pane">${mineralInspector()}<div class="rail-head"><div class="eyebrow">${state.place ? esc(state.place) : L('Matching reports', 'Articles de la sélection')}</div><div style="display:flex;gap:8px"><select class="select" data-model="sort" style="flex:1" aria-label="${L('Sort', 'Tri')}"><option value="priority" ${state.sort === 'priority' ? 'selected' : ''}>${L('Priority', 'Priorité')}</option><option value="latest" ${state.sort === 'latest' ? 'selected' : ''}>${L('Newest', 'Plus récents')}</option></select><button class="iconbtn" data-act="export" aria-label="${L('Export JSON', 'Exporter JSON')}" title="${L('Export JSON', 'Exporter JSON')}">${icon('download')}</button></div>${state.place || state.selectedDay ? `<div class="chips"><button class="chip active" data-act="clear-situation">${esc([state.place, state.selectedDay && dayLabel(state.selectedDay)].filter(Boolean).join(' · '))} ×</button></div>` : ''}${state.error ? `<div class="errorbar" style="margin:0">${esc(state.error)}</div>` : ''}</div>
      <div class="rail-list compact report-list" data-scroll="rail">${items.slice(0, state.visible).map(a => reportRow(a, true)).join('') || emptyState(L('No reports', 'Aucun article'), L('Select a layer or widen the window.', 'Sélectionnez une couche ou élargissez la période.'))}${items.length > state.visible ? `<div class="loadmore"><button class="btn sm" data-act="more">${L('Load more', 'Charger plus')}</button></div>` : ''}</div>
      <div class="rail-foot"><span>${L('Single source · unverified', 'Source unique · non corroboré')}</span><span class="num">${nf(items.length)}</span></div></aside>
  </div>`;
}
// ---------------------------------------------------------------------------------------------
// Minerals mode: geological base map, mine dossiers and market context, without news layers.

state.mineralMode = location.hash === '#minerals'; state.geoOverlay = true; state.geoOpacity = 0.55; state.relief = true;
state.marketCommodity = 'copper'; state.commodities = null; state.geoUnit = null;
// ICS chronostratigraphic colours, which Macrostrat's geology tiles follow.
const ERA_LEGEND = [['Cenozoic', 'Cénozoïque', '#F2F91D', '0–66 Ma'], ['Mesozoic', 'Mésozoïque', '#67C5CA', '66–252 Ma'], ['Paleozoic', 'Paléozoïque', '#99C08D', '252–539 Ma'], ['Neoproterozoic', 'Néoprotérozoïque', '#FEB342', '539–1000 Ma'], ['Mesoproterozoic', 'Mésoprotérozoïque', '#FDB462', '1.0–1.6 Ga'], ['Paleoproterozoic', 'Paléoprotérozoïque', '#F74370', '1.6–2.5 Ga'], ['Archean', 'Archéen', '#F0047F', '> 2.5 Ga']];
const QUALIFICATION = () => ({ Vert: ['good', L('Green', 'Vert')], Jaune: ['medium', L('Yellow', 'Jaune')], Rouge: ['high', L('Red', 'Rouge')], Bleu: ['accent', L('Blue', 'Bleu')] });
const USGS_STATUS = () => ({ 'C&M': L('Care & maintenance', 'En veille'), 'Assumed Active': L('Assumed active', 'Présumé actif'), Active: L('Active', 'Actif') });
const commodityFor = label => state.commodities?.items?.find(c => c.labels.some(l => norm(l) === norm(label)));
const commodityName = c => c ? c[state.lang] || c.en : '';
const usd = v => v == null ? '—' : '$' + (Math.abs(v) >= 1e5 ? compact(v) : Number(v).toLocaleString(locale(), { maximumFractionDigits: v < 100 ? 2 : 0 }));
/// The commodity's largest traded product line (by partner-reported value).
const mainTrade = c => [...(c?.hs || [])].filter(h => h.buyers).sort((a, b) => b.buyers.total_usd - a.buyers.total_usd)[0] || null;
function priceSummary(c) {
  const p = c?.price, t = mainTrade(c);
  if (p?.latest) return { value: usd(p.latest.value), unit: p.unit.replace('$', ''), note: `${L('World Bank benchmark', 'Référence Banque mondiale')} · ${p.latest.period}`, change: p.change_12m, history: p.history.map(h => h.value) };
  const uv = t?.buyers?.unit_value_usd_per_kg;
  if (uv) return { value: usd(uv), unit: '/kg', note: `${L('Customs unit value', 'Valeur unitaire douanière')} · ${t.code} · ${t.buyers.year}`, change: null, history: [] };
  return null;
}
function changeBadge(ch) {
  if (ch == null) return '';
  const up = ch >= 0;
  return `<em class="chg ${up ? 'up' : 'down'}" data-tip="${esc(L('Change vs. same month last year', 'Variation sur un an'))}">${up ? '▲' : '▼'} ${Math.abs(ch * 100).toFixed(1)}%</em>`;
}
function marketBoard() {
  const items = (state.commodities?.items || []).filter(c => priceSummary(c));
  if (!state.commodities) return `<div class="timeline-head"><strong>${L('Mineral markets', 'Marchés des minerais')}</strong><span>${L('Market data unavailable or still syncing.', 'Données de marché indisponibles ou en cours de synchronisation.')}</span></div>`;
  return `<div class="timeline-head"><strong>${L('Mineral markets', 'Marchés des minerais')}</strong><span>${L('Benchmarks: World Bank Pink Sheet · no-benchmark minerals: UN Comtrade customs unit values', 'Références : Banque mondiale · minerais sans référence : valeurs unitaires douanières UN Comtrade')}</span></div>
    <div class="tickers">${items.map(c => { const s = priceSummary(c); return `<button class="ticker ${(state.focus?.commodity || state.marketCommodity) === c.id ? 'active' : ''}" data-act="focus" data-arg="${esc(c.id)}" data-tip="${esc(L('Map this mineral’s mining areas', 'Cartographier les zones minières de ce minerai'))}"><span class="t-name">${esc(commodityName(c))}</span><span class="t-val"><b>${s.value}</b><small>${esc(s.unit)}</small>${changeBadge(s.change)}</span>${s.history.length > 1 ? spark(s.history.slice(-24), 'var(--signal)', 96, 22) : `<small class="t-note">${esc(s.note)}</small>`}</button>` }).join('') || `<small class="muted">${L('No market data yet.', 'Pas encore de données de marché.')}</small>`}</div>`;
}
function partnerBars(list, year, max) {
  const top = list.partners[0]?.value_usd || 1;
  return `<div class="buyers">${list.partners.slice(0, max).map(p => `<div class="buyer" data-tip="${esc(`${p.partner}: ${usd(p.value_usd)} · ${(p.share * 100).toFixed(1)}%${p.unit_value_usd_per_kg ? ` · ${usd(p.unit_value_usd_per_kg)}/kg` : ''} · ${year}`)}"><span>${esc(p.partner)}</span>${meter(p.value_usd, top, 'var(--signal)')}<b class="num">${usd(p.value_usd)}</b><small class="num">${(p.share * 100).toFixed(0)}%</small></div>`).join('')}</div>`;
}
function buyersBlock(h, max = 8) {
  const b = h?.buyers, d = h?.drc_declared;
  // When the DRC declares far more than importers report, the declared destinations matter too.
  const gap = d && (!b || d.total_usd > 2 * b.total_usd);
  if (!b && !d) return `<p class="muted">${L('No customs records found for this product.', 'Aucune donnée douanière pour ce produit.')}</p>`;
  return `${b ? `<div class="eyebrow" style="margin:0 0 4px">${L('Importers’ declarations', 'Déclarations des importateurs')} · ${b.year}</div>${partnerBars(b, b.year, max)}` : `<p class="muted">${L('No importer reports this product from the DRC.', 'Aucun importateur ne déclare ce produit en provenance de la RDC.')}</p>`}
    ${gap && d.partners.length ? `<div class="eyebrow" style="margin:10px 0 4px">${L('DRC-declared destinations', 'Destinations déclarées par la RDC')} · ${d.year}</div>${partnerBars(d, d.year, Math.min(max, 6))}<p class="gap-note">${L('Large gap between DRC-declared exports and importers’ records: products may be declared under other codes, routed through transit countries, or under-reported.', 'Écart important entre exportations déclarées par la RDC et importations déclarées : autres codes douaniers, pays de transit ou sous-déclaration possibles.')}</p>` : ''}
    <div class="trade-foot">${b ? `<span>${L('Importers report', 'Importations déclarées')} ${b.year}: <b class="num">${usd(b.total_usd)}</b>${b.unit_value_usd_per_kg ? ` · ${usd(b.unit_value_usd_per_kg)}/kg` : ''}</span>` : ''}<span>${d ? `${L('DRC declares', 'La RDC déclare')} ${d.year}: <b class="num">${usd(d.total_usd)}</b>` : L('No DRC-declared exports on record', 'Pas d’exportations déclarées par la RDC')}</span></div>`;
}
function indicativeValue(p, c) {
  const pr = c?.price;
  if (!p.capacity || !pr?.latest) return null;
  const cap = p.capacity, unit = p.capacity_unit;
  const kg = unit === 'Metric tons' ? cap * 1e3 : unit === 'Thousand metric tons' ? cap * 1e6 : unit === 'Kilograms' ? cap : null;
  if (kg != null && pr.per_kg) return kg * pr.per_kg * pr.latest.value;
  if (unit === 'Thousand 42-gallon barrels' && pr.unit === '$/bbl') return cap * 1e3 * pr.latest.value;
  return null;
}
function commodityCard(c, p) {
  if (!c) return '';
  const s = priceSummary(c), v = indicativeValue(p, c), h = mainTrade(c);
  return `<section class="panel market-card"><div class="panel-head"><h2>${esc(commodityName(c))}</h2><button class="linkbtn" data-act="market-open" data-arg="${esc(c.id)}">${L('Full market', 'Marché complet')} →</button></div><div class="panel-body">
    ${s ? `<div class="price-line"><b>${s.value}</b><small>${esc(s.unit)}</small>${changeBadge(s.change)}<span class="grow"></span>${s.history.length > 1 ? spark(s.history.slice(-24), 'var(--signal)', 120, 28) : ''}</div><div class="muted" style="font-size:11.5px">${esc(s.note)}</div>` : `<p class="muted">${L('No public benchmark or customs unit value.', 'Aucune référence publique ni valeur douanière.')}</p>`}
    ${v ? `<div class="value-line" data-tip="${esc(L('Nameplate capacity × latest benchmark price. A ceiling for orientation, not production, revenue or reserves.', 'Capacité nominale × dernier prix de référence. Un plafond indicatif, pas la production, le chiffre d’affaires ni les réserves.'))}"><span>${L('Indicative capacity value', 'Valeur indicative de la capacité')}</span><b class="num">≈ ${usd(v)}${L('/yr', '/an')}</b></div>` : ''}
    <div class="eyebrow" style="margin:12px 0 6px">${L('Who buys DRC', 'Qui achète le')} ${esc(h ? h[state.lang] || h.en : commodityName(c))}</div>${buyersBlock(h, 5)}</div></section>`;
}
function mineDossier(f) {
  const p = f.properties, [lon, lat] = f.geometry.coordinates;
  const commodities = [...new Map(p.minerals.map(m => [commodityFor(m)?.id, commodityFor(m)])).values()].filter(Boolean);
  const q = QUALIFICATION()[p.qualification];
  const facts = [
    factRow(L('Facility type', 'Type d’installation'), esc(p.facility_type || '')),
    factRow(L('Operator', 'Exploitant'), esc(p.operator || '')),
    factRow(L('Owners', 'Propriétaires'), (p.owners || []).map(esc).join('<br>')),
    factRow(L('Status (end 2018)', 'Statut (fin 2018)'), p.status ? `${esc(USGS_STATUS()[p.status] || p.status)}${p.status_note ? ` <span class="muted">· ${esc(p.status_note)}</span>` : ''}` : ''),
    factRow(L('Capacity', 'Capacité'), p.capacity ? `<span class="num">${nf(p.capacity)}</span> ${esc(p.capacity_unit || '')} ${L('per year', 'par an')}` : ''),
    factRow(L('Workers (last visit)', 'Travailleurs (dernière visite)'), p.workers != null ? `<span class="num">${nf(p.workers)}</span>` : ''),
    factRow(L('Mechanisation', 'Mécanisation'), esc(p.mechanisation || '')),
    factRow(L('Mercury use', 'Usage du mercure'), p.mercury == null ? '' : p.mercury ? L('Observed', 'Observé') : L('Not observed', 'Non observé')),
    factRow(L('State services present', 'Services de l’État présents'), (p.state_services || []).map(esc).join(', ')),
    factRow(L('Record date', 'Date de la référence'), `${esc(p.date || L('Unknown', 'Inconnue'))}${p.observations ? ` · ${p.observations} ${L('IPIS visits', 'visites IPIS')}` : ''}`),
    factRow(L('Coordinates', 'Coordonnées'), `<span class="num">${lat.toFixed(5)}, ${lon.toFixed(5)}</span>${p.location_origin ? ` <span class="muted">· ${esc(p.location_origin)}</span>` : ''}`),
  ].join('');
  const nearby = nearbyMineralReports(f);
  return `<div class="mine-dossier" data-scroll="mine">
    <div class="dossier-top"><span class="eyebrow">${esc(p.dataset)} · ${esc(mineralKindLabel(p.kind))}</span><button class="iconbtn" data-act="mineral-close" aria-label="${L('Close', 'Fermer')}">${icon('close')}</button></div>
    <h2 class="mine-name">${esc(p.name)}</h2>
    <div class="mine-place">${esc([p.village, p.territory, p.province].filter(Boolean).join(' · '))}</div>
    <div class="chips" style="margin:10px 0">${p.minerals.map(m => `<button class="chip mineral-chip" data-act="focus" data-arg="${esc(commodityFor(m)?.id || m)}" data-tip="${esc(L('Map all mining areas for this mineral', 'Cartographier toutes les zones de ce minerai'))}">${esc(m)}</button>`).join('')}${q ? `<span class="badge ${q[0]}" data-tip="${esc(L('DRC Ministry of Mines site qualification at the latest IPIS visit (green: validated; yellow: issues to remedy; red: non-compliant).', 'Qualification du site par le ministère des Mines lors de la dernière visite IPIS (vert : validé ; jaune : à corriger ; rouge : non conforme).'))}">${L('Qualification', 'Qualification')} · ${q[1]}</span>` : ''}${p.traceability ? `<span class="badge good" data-tip="${esc(L('Mineral traceability scheme recorded for the site', 'Système de traçabilité enregistré pour le site'))}">${esc(p.traceability)}</span>` : ''}</div>
    ${(p.trade || []).length ? `<section class="route"><div class="eyebrow" style="margin-bottom:8px">${L('Trade route · latest IPIS visit', 'Circuit commercial · dernière visite IPIS')}</div>${p.trade.map(t => `<div class="route-row"><span class="route-node mine">${esc(t.mineral)}</span><i>→</i><span class="route-node" data-tip="${esc(L('Selling points: where the mineral is first sold', 'Points de vente : premier lieu de vente'))}">${esc(t.selling_points || '—')}</span><i>→</i><span class="route-node dest" data-tip="${esc(L('Final destination recorded by IPIS', 'Destination finale enregistrée par IPIS'))}">${esc(t.final_destination || '—')}</span></div>`).join('')}</section>` : ''}
    <section class="panel" style="padding:12px 16px;margin:12px 0"><dl class="props">${facts}</dl></section>
    ${commodities.map(c => commodityCard(c, p)).join('')}
    <section class="panel" style="padding:12px 16px;margin:12px 0"><label class="nearby-label">${L('News mentions within', 'Lieux cités dans un rayon de')}<select class="select" data-model="nearbyRadius">${[10, 25, 50, 100].map(n => `<option value="${n}" ${Number(state.nearbyRadius) === n ? 'selected' : ''}>${n} km</option>`).join('')}</select></label>
      <div class="nearby-reports">${nearby.slice(0, 6).map(({ article: a, place: pl }) => `<button class="mineral-result" data-act="report" data-arg="${esc(a.id)}"><strong>${esc(title(a))}</strong><small>${esc(pl.name)} · ${pl.km.toFixed(1)} km · ${esc(a.source || '')}</small></button>`).join('') || `<p class="muted">${L('No loaded reports mention a place in this radius.', 'Aucun article chargé ne cite de lieu dans ce rayon.')}</p>`}</div>
      <small class="muted">${L('Proximity to a place mention does not establish a link to this site.', 'La proximité d’un lieu cité ne prouve pas un lien avec ce site.')}</small></section>
    <div class="actions"><button class="btn sm" data-act="mineral-select" data-arg="${esc(f.id)}">${icon('pin')}${L('Centre', 'Centrer')}</button><button class="btn sm" data-act="mineral-export">${icon('download')}JSON</button><a class="btn sm ghost" href="${esc(p.reference)}" target="_blank" rel="noopener noreferrer">${icon('external')}${L('Source', 'Source')}</a></div>
    <p class="notice">${esc(p.source || '')} · ${esc(p.license || '')}. ${L('Historical record: current operation, ownership and control are not verified.', 'Référence historique : exploitation, propriété et contrôle actuels non vérifiés.')}</p></div>`;
}
function marketPanel() {
  const all = state.commodities?.items || [], c = all.find(x => x.id === state.marketCommodity) || all[0];
  if (!c) return `<div class="mine-dossier"><p class="muted">${L('Market data is syncing from the World Bank and UN Comtrade.', 'Synchronisation des données de marché (Banque mondiale, UN Comtrade).')}</p></div>`;
  const s = priceSummary(c);
  const siteCount = (state.geology?.features || []).filter(f => f.properties.minerals.some(m => c.labels.some(l => norm(l) === norm(m)))).length;
  return `<div class="mine-dossier" data-scroll="market">
    <div class="eyebrow">${L('MARKET INTELLIGENCE', 'RENSEIGNEMENT DE MARCHÉ')}</div><h2 class="mine-name">${esc(commodityName(c))}</h2>
    <div class="chips" style="margin:8px 0 12px">${all.filter(x => priceSummary(x)).map(x => `<button class="chip ${x.id === c.id ? 'active' : ''}" data-act="market" data-arg="${esc(x.id)}">${esc(commodityName(x))}</button>`).join('')}</div>
    ${s ? `<section class="panel" style="padding:14px 16px;margin-bottom:12px"><div class="price-line big"><b>${s.value}</b><small>${esc(s.unit)}</small>${changeBadge(s.change)}</div><div class="muted" style="font-size:11.5px;margin-bottom:8px">${esc(s.note)}</div>${c.price?.history?.length > 1 ? areaChart(c.price.history.slice(-36).map(h => ({ value: h.value, tip: `${h.period} · ${usd(h.value)} ${c.price.unit.replace('$', '')}` })), 'var(--signal)', 70) : ''}</section>` : ''}
    <div class="actions" style="margin-bottom:12px"><button class="btn sm primary" data-act="focus" data-arg="${esc(c.id)}">${icon('map')}${L('Map', 'Cartographier')} ${nf(siteCount)} ${L('sites & mining areas', 'sites et zones minières')}</button></div>
    ${c.hs.map(h => `<section class="panel market-card"><div class="panel-head"><h2>${esc(h[state.lang] || h.en)}</h2><small>HS ${esc(h.code)}</small></div><div class="panel-body">${buyersBlock(h, 10)}</div></section>`).join('')}
    <p class="notice">${esc(srv(state.commodities?.notice))}</p>
    <p class="muted" style="font-size:11px">${(state.commodities?.sources || []).map(s => `<a href="${esc(s.url)}" target="_blank" rel="noopener noreferrer">${esc(s.name)}</a> · ${esc(s.license)}`).join('<br>')}</p></div>`;
}
function mineralsSituation() {
  const fs = mineralFeatures();
  const sel = selectedMineral();
  return `<div class="situation minerals-mode ${state.mapFocus ? 'map-focus' : ''}">
    <aside class="filters-pane" data-scroll="filters">
      <div class="pane-section"><div class="eyebrow">${L('DRC', 'RDC')} · ${L('MINERALS & GEOLOGY', 'MINERAIS ET GÉOLOGIE')}</div><div style="font-size:13px;color:var(--text-2)"><b class="num" style="color:var(--text)">${nf(fs.length)}</b> ${L('site records in view', 'références affichées')}</div></div>
      <div class="pane-section"><h2>${L('Map layers', 'Couches')}</h2>
        <label class="check"><input type="checkbox" data-model="geoOverlay" ${state.geoOverlay ? 'checked' : ''}>${L('Bedrock geology (Macrostrat)', 'Géologie du socle (Macrostrat)')}</label>
        <label class="range-row">${L('Opacity', 'Opacité')}<input type="range" min="0.2" max="1" step="0.05" value="${state.geoOpacity}" data-act="geo-opacity" aria-label="${L('Geology opacity', 'Opacité de la géologie')}"></label>
        <label class="check"><input type="checkbox" data-model="relief" ${state.relief ? 'checked' : ''}>${L('Terrain relief', 'Relief')}</label>
        <div class="era-legend">${ERA_LEGEND.map(([en, fr, c, span]) => `<div><i style="background:${c}"></i><span>${state.lang === 'fr' ? fr : en}</span><small>${span}</small></div>`).join('')}</div>
        <small class="muted">${L('Colours follow the international chronostratigraphic chart. Click any rock on the map for its unit, lithology and age.', 'Couleurs de la charte chronostratigraphique internationale. Cliquez sur la carte pour l’unité, la lithologie et l’âge.')}</small></div>
      ${geologyPanel()}
    </aside>
    <section class="stage"><div class="map-area">${mapSVG([])}
      <div class="hud" aria-hidden="true"><i class="hud-c tl"></i><i class="hud-c tr"></i><i class="hud-c bl"></i><i class="hud-c br"></i></div>
      <div class="map-float hud-pos glass hud-readout" id="hud-coords" aria-hidden="true">${hudText(state.hud)}</div>
      <div class="map-float tl glass ${state.focus ? 'focus-tag' : ''}" ${state.focus ? `style="--focus:${state.focus.hex}"` : ''}><strong>${sel ? esc(sel.properties.name) : state.focus ? `<i class="swatch" style="background:${state.focus.hex}"></i>${esc(state.focus.name)}` : L('Mineral & geological map', 'Carte minière et géologique')}</strong>${state.focus ? `${nf(state.focus.sites)} ${L('sites', 'sites')} · ${state.focus.areas.length} ${L('mining areas', 'zones minières')} · ${[...new Set(state.focus.areas.flatMap(a => a.provinces))].length} ${L('provinces', 'provinces')}` : `${nf(fs.length)} ${L('sites', 'sites')} · IPIS · USGS · Macrostrat`}</div>
      <div class="map-float tr"><div class="seg glass mode-switch" style="padding:2px"><button data-act="map-mode" data-arg="news">${L('News', 'Actualité')}</button><button class="active" data-act="map-mode" data-arg="minerals">${L('Minerals', 'Minerais')}</button><button data-act="map-mode" data-arg="radio">${icon('radio')}${L('Radio', 'Radio')}</button><button data-act="map-mode" data-arg="flights">✈ ${L('Flights','Vols')}</button></div><button class="btn sm glass" data-act="map-focus" aria-pressed="${state.mapFocus}">${state.mapFocus ? L('Show panels', 'Afficher les panneaux') : L('Expand map', 'Agrandir')}</button><button class="btn sm glass" data-act="tilt">${state.tilt ? L('Flat', 'À plat') : L('3D tilt', 'Inclinée')}</button><button class="btn sm glass" data-act="reset-map">${icon('globe')}${L('Reset', 'Recentrer')}</button></div>
      ${state.geoUnit ? geoUnitCard(state.geoUnit) : `<div class="map-float bl glass">${L('Click the map for bedrock geology; click a site for its dossier. Sites are historical records, not current operations.', 'Cliquez sur la carte pour la géologie ; sur un site pour son dossier. Les sites sont des références historiques.')}</div>`}
    </div><div class="timeline market-board">${marketBoard()}</div></section>
    <aside class="rail-pane">${sel ? mineDossier(sel) : state.focus ? focusPanel() : marketPanel()}</aside>
  </div>`;
}
function geoUnitCard(u) {
  if (u.loading) return `<div class="map-float bl glass geo-card"><div class="loading-line"></div>${L('Reading bedrock geology…', 'Lecture de la géologie…')}</div>`;
  if (u.error || !u.found) return `<div class="map-float bl glass geo-card">${esc(u.error || L('No mapped geological unit here.', 'Aucune unité géologique cartographiée ici.'))}<button class="linkbtn" data-act="geo-close">×</button></div>`;
  const age = u.age || {};
  const span = age.top === age.bottom ? age.top : `${age.bottom || '?'} → ${age.top || '?'}`;
  return `<div class="map-float bl glass geo-card"><div class="actions" style="justify-content:space-between"><span class="eyebrow"><i class="swatch" style="background:${esc(u.color || '#888')}"></i>${L('BEDROCK', 'SOCLE')} · ${esc(u.lat.toFixed(3))}, ${esc(u.lng.toFixed(3))}</span><button class="linkbtn" data-act="geo-close" aria-label="${L('Close', 'Fermer')}">×</button></div>
    <strong>${esc(u.name || u.strat_name || (u.lith ? u.lith[0].toUpperCase() + u.lith.slice(1) : L('Mapped unit', 'Unité cartographiée')))}</strong>
    <div class="geo-facts"><span>${L('Lithology', 'Lithologie')}: <b>${esc(u.lith || '—')}</b></span><span>${L('Age', 'Âge')}: <b>${esc(span || '—')}</b>${age.bottom_ma != null ? ` <small>(${nf(age.bottom_ma)}–${nf(age.top_ma)} Ma)</small>` : ''}</span></div>
    ${u.description ? `<p>${esc(u.description.slice(0, 260))}</p>` : ''}
    <small class="muted">${u.source ? `${esc(u.source.authors || '')} ${esc(u.source.year || '')}, ${esc(u.source.title || u.source.name || '')}. ` : ''}Macrostrat · CC BY 4.0 · ${L('map scale', 'échelle')}: ${esc(u.scale || '—')}</small></div>`;
}
async function geologyAt(lngLat) {
  state.geoUnit = { loading: true };
  render();
  try { state.geoUnit = { ...(await api(`/v1/geology/unit?lat=${lngLat.lat.toFixed(4)}&lng=${lngLat.lng.toFixed(4)}`)), lat: lngLat.lat, lng: lngLat.lng } }
  catch (e) { state.geoUnit = { error: e.message } }
  render();
}
/// Geology raster and hillshade relief, shown only in minerals mode; news markers are cleared.
function mountGeology() {
  const map = mapInstance;
  // isStyleLoaded() stays false while tiles load, so check the style itself and retry on idle.
  if (!map) return;
  try { mountGeologyLayers(map) } catch {}
}
function mountGeologyLayers(map) {
  // Geology and relief sit beneath every Congo Signal layer (outline, focus mask, sites), whatever
  // order those were added in; a mis-ordered stack is corrected on the next mount.
  const ours = ['drc-fill', 'drc-glow', 'drc-border', 'drc-comet-glow', 'drc-comet', 'focus-mask', 'focus-fill', 'focus-glow', 'focus-line', 'focus-active', 'focus-labels', 'mineral-clusters', 'mineral-counts', 'mineral-sites', 'mineral-selection'];
  const order = () => map.getStyle().layers.map(l => l.id);
  const before = order().find(id => ours.includes(id));
  if (!map.getSource('macrostrat')) {
    map.addSource('macrostrat', { type: 'raster', tiles: ['https://tiles.macrostrat.org/carto/{z}/{x}/{y}.png'], tileSize: 256, maxzoom: 10, attribution: '© <a href="https://macrostrat.org" target="_blank" rel="noopener">Macrostrat</a> (CC BY 4.0)' });
    map.addLayer({ id: 'geology', type: 'raster', source: 'macrostrat', paint: { 'raster-opacity': state.geoOpacity, 'raster-saturation': -0.25, 'raster-contrast': 0.05, 'raster-fade-duration': 150 } }, before);
  }
  if (!map.getSource('terrain-dem')) {
    map.addSource('terrain-dem', { type: 'raster-dem', url: 'mapbox://mapbox.mapbox-terrain-dem-v1', tileSize: 512, maxzoom: 14 });
    map.addLayer({ id: 'relief', type: 'hillshade', source: 'terrain-dem', paint: { 'hillshade-exaggeration': 0.55, 'hillshade-shadow-color': '#05080a', 'hillshade-highlight-color': 'rgba(255,255,255,0.18)' } }, before);
  }
  const ids = order();
  if (before && ids.indexOf('geology') > ids.indexOf(before)) { map.moveLayer('geology', before); map.moveLayer('relief', before) }
  const on = state.mineralMode && state.view === 'situation';
  map.setLayoutProperty('geology', 'visibility', on && state.geoOverlay ? 'visible' : 'none');
  map.setPaintProperty('geology', 'raster-opacity', state.geoOpacity);
  map.setLayoutProperty('relief', 'visibility', on && state.relief ? 'visible' : 'none');
  if (map.getLayer('drc-fill')) map.setPaintProperty('drc-fill', 'fill-opacity', on ? 0 : 0.04);
}


// ---------------------------------------------------------------------------------------------
// Mineral focus: group a mineral's sites into mining areas, outline them, spotlight and tour.

// focus-geometry:start — pure functions (tested in tests/mineral_focus.cjs)
const FOCUS_LINK_KM = 20, FOCUS_MIN_SITES = 3;
function kmBetween(a, b) {
  const rad = Math.PI / 180, dlat = (b[1] - a[1]) * rad, dlon = (b[0] - a[0]) * rad;
  const h = Math.sin(dlat / 2) ** 2 + Math.cos(a[1] * rad) * Math.cos(b[1] * rad) * Math.sin(dlon / 2) ** 2;
  return 6371 * 2 * Math.asin(Math.sqrt(Math.min(1, h)));
}
/// Single-linkage clusters: sites within `linkKm` of each other chain into one area (grid-indexed).
function clusterSites(features, linkKm = FOCUS_LINK_KM) {
  const n = features.length, parent = [...Array(n).keys()];
  const find = i => parent[i] === i ? i : (parent[i] = find(parent[i]));
  const cell = linkKm / 111, grid = new Map();
  features.forEach((f, i) => {
    const [x, y] = f.geometry.coordinates, key = `${Math.floor(x / cell)}:${Math.floor(y / cell)}`;
    (grid.get(key) || grid.set(key, []).get(key)).push(i);
  });
  features.forEach((f, i) => {
    const [x, y] = f.geometry.coordinates, cx = Math.floor(x / cell), cy = Math.floor(y / cell);
    for (let dx = -1; dx <= 1; dx++) for (let dy = -1; dy <= 1; dy++) {
      for (const j of grid.get(`${cx + dx}:${cy + dy}`) || []) {
        if (j > i && kmBetween(f.geometry.coordinates, features[j].geometry.coordinates) <= linkKm) {
          const a = find(i), b = find(j); if (a !== b) parent[a] = b;
        }
      }
    }
  });
  const groups = new Map();
  features.forEach((f, i) => { const r = find(i); (groups.get(r) || groups.set(r, []).get(r)).push(f) });
  return [...groups.values()];
}
function convexHull(points) {
  const p = [...new Map(points.map(q => [q.join(','), q])).values()].sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  if (p.length < 3) return p;
  const cross = (o, a, b) => (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
  const lower = [], upper = [];
  for (const q of p) { while (lower.length >= 2 && cross(lower.at(-2), lower.at(-1), q) <= 0) lower.pop(); lower.push(q) }
  for (const q of [...p].reverse()) { while (upper.length >= 2 && cross(upper.at(-2), upper.at(-1), q) <= 0) upper.pop(); upper.push(q) }
  return [...lower.slice(0, -1), ...upper.slice(0, -1)];
}
/// Rounded outline around an area: the convex hull of small circles (radius `padKm`) placed on the
/// hull of its sites — a buffered hull, so single sites and straight lines still get a body.
function areaOutline(points, padKm = 7) {
  const cy = points.reduce((s, q) => s + q[1], 0) / points.length;
  const kx = 1 / (111.32 * Math.cos(cy * Math.PI / 180)), ky = 1 / 110.57;
  const around = convexHull(points).flatMap(q => [...Array(16).keys()].map(k => [q[0] + Math.cos(k / 8 * Math.PI) * padKm * kx, q[1] + Math.sin(k / 8 * Math.PI) * padKm * ky]));
  const ring = convexHull(around).map(q => [Math.round(q[0] * 1e5) / 1e5, Math.round(q[1] * 1e5) / 1e5]);
  return [...ring, ring[0]];
}
function ringAreaKm2(ring) {
  const lat0 = ring.reduce((s, q) => s + q[1], 0) / ring.length, kx = 111.32 * Math.cos(lat0 * Math.PI / 180), ky = 110.57;
  let a = 0;
  for (let i = 0; i < ring.length - 1; i++) a += ring[i][0] * kx * ring[i + 1][1] * ky - ring[i + 1][0] * kx * ring[i][1] * ky;
  return Math.abs(a / 2);
}
const signedArea = ring => ring.slice(0, -1).reduce((s, q, i) => s + q[0] * ring[i + 1][1] - ring[i + 1][0] * q[1], 0) / 2;
/// Mining areas for a mineral. Sites are grouped by territory (IPIS) or province (USGS, which has
/// no territory), then chained within FOCUS_LINK_KM; areas need FOCUS_MIN_SITES sites. Grouping by
/// administrative unit first keeps dense mineral belts from merging into one region-sized blob.
function mineralAreas(features) {
  const groups = new Map();
  for (const f of features) {
    const p = f.properties, key = `${p.province || '—'}|${p.territory || ''}`;
    (groups.get(key) || groups.set(key, []).get(key)).push(f);
  }
  return [...groups.values()].flatMap(g => clusterSites(g)).filter(g => g.length >= FOCUS_MIN_SITES).map(g => {
    const pts = g.map(f => f.geometry.coordinates);
    const ring = areaOutline(pts);
    const top = (field) => [...g.reduce((m, f) => { const v = f.properties[field]; return v ? m.set(v, (m.get(v) || 0) + 1) : m }, new Map())].sort((a, b) => b[1] - a[1]).map(x => x[0]);
    const provinces = top('province'), territories = top('territory');
    const xs = ring.map(q => q[0]), ys = ring.map(q => q[1]);
    return { sites: g, count: g.length, ring, provinces: provinces.length ? provinces : ['—'], province: provinces[0] || '—', territory: territories[0] || null,
      name: territories[0] ? `${territories[0]} · ${provinces[0]}` : provinces[0] || '—',
      bbox: [[Math.min(...xs), Math.min(...ys)], [Math.max(...xs), Math.max(...ys)]], km2: ringAreaKm2(ring),
      centroid: [pts.reduce((s, q) => s + q[0], 0) / pts.length, pts.reduce((s, q) => s + q[1], 0) / pts.length] };
  }).sort((a, b) => b.count - a.count);
}
/// A world polygon with the areas cut out, to dim everything outside them. Overlapping outlines are
/// merged (hull of their union) first: overlapping holes make an invalid polygon that is not drawn.
function spotlightMask(areas) {
  const bbox = r => { const xs = r.map(q => q[0]), ys = r.map(q => q[1]); return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)] };
  const overlap = (a, b) => a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3];
  let rings = areas.map(a => a.ring.slice(0, -1));
  for (let merged = true; merged;) {
    merged = false;
    outer: for (let i = 0; i < rings.length; i++) for (let j = i + 1; j < rings.length; j++) {
      if (overlap(bbox(rings[i]), bbox(rings[j]))) { rings[i] = convexHull([...rings[i], ...rings[j]]); rings.splice(j, 1); merged = true; break outer }
    }
  }
  const outer = [[-180, -85], [180, -85], [180, 85], [-180, 85], [-180, -85]];
  const holes = rings.map(r => [...r, r[0]]).map(r => signedArea(r) > 0 ? [...r].reverse() : r);
  return { type: 'Feature', properties: {}, geometry: { type: 'Polygon', coordinates: [outer, ...holes] } };
}
// focus-geometry:end

const FOCUS_COLORS = { gold: '#f5c542', tin: '#5fd0ff', tantalum: '#c084fc', tungsten: '#9fb4c8', copper: '#ff8a3d', cobalt: '#4f8cff', diamonds: '#e8eef5', zinc: '#a3e635', manganese: '#f472b6', silver: '#cbd5e1', oil: '#facc15' };
let focusTour = 0;
const reducedMotion = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
/// Focus a mineral by commodity id or by a raw site label (e.g. "Tourmaline").
function focusMineral(key) {
  const c = state.commodities?.items?.find(x => x.id === key);
  const labels = c ? c.labels : [key];
  const present = new Set((state.geology?.features || []).flatMap(f => f.properties.minerals));
  const usable = labels.filter(l => present.has(l));
  if (!usable.length) { toast(L('No mapped sites for this mineral.', 'Aucun site cartographié pour ce minerai.')); return }
  const sites = (state.geology?.features || []).filter(f => state.mineralLayers.includes(f.properties.dataset) && f.properties.minerals.some(m => usable.includes(m)));
  const areas = mineralAreas(sites);
  state.focus = { key, commodity: c?.id || null, name: c ? commodityName(c) : key, labels: usable, color: FOCUS_COLORS[c?.id] || 'var(--accent)', hex: FOCUS_COLORS[c?.id] || '#c6f36b', sites: sites.length, areas, active: -1 };
  state.mineralFocusLabels = usable; state.mineralQuery = ''; state.mineralSelected = ''; state.geoUnit = null;
  if (c) state.marketCommodity = c.id;
  state.mineralMode = true; state.radioMode = false; state.flightMode = false;
  history.replaceState(null, '', '#focus/' + encodeURIComponent(key));
  if (state.view !== 'situation') go('situation'); else render();
  playFocusTour();
}
function exitFocus() {
  focusTour++;
  state.focus = null; state.mineralFocusLabels = [];
  history.replaceState(null, '', '#minerals');
  render();
  mapInstance?.easeTo({ pitch: state.tilt ? 55 : 0, bearing: 0, duration: reducedMotion() ? 0 : 900 });
  mapInstance?.fitBounds([[12, -13.6], [31.4, 5.5]], { padding: 40, duration: reducedMotion() ? 0 : 1400 });
}
const moveDone = map => new Promise(r => map.once('moveend', r));
const pause = ms => new Promise(r => setTimeout(r, ms));
function markActiveArea(i) {
  if (!state.focus) return;
  state.focus.active = i;
  $$('[data-area]').forEach(el => el.classList.toggle('active', Number(el.dataset.area) === i));
  try { mapInstance?.setFilter('focus-active', ['==', ['get', 'rank'], i]) } catch {}
}
function allAreasBounds(areas) {
  const xs = areas.flatMap(a => [a.bbox[0][0], a.bbox[1][0]]), ys = areas.flatMap(a => [a.bbox[0][1], a.bbox[1][1]]);
  return [[Math.min(...xs), Math.min(...ys)], [Math.max(...xs), Math.max(...ys)]];
}
/// Overview of all areas, then fly through the largest ones with a slight tilt, then back.
async function playFocusTour(onlyIndex) {
  const token = ++focusTour, map = mapInstance, f = state.focus;
  if (!map || !f?.areas.length) return;
  const still = () => token === focusTour && state.focus === f && mapInstance === map;
  const fast = reducedMotion();
  const fly = async (bounds, opts = {}) => { map.fitBounds(bounds, { padding: 90, maxZoom: 9.5, duration: fast ? 0 : 2200, essential: true, ...opts }); if (!fast) await moveDone(map) };
  if (onlyIndex != null) { markActiveArea(onlyIndex); await fly(f.areas[onlyIndex].bbox, { pitch: fast ? 0 : 40, bearing: fast ? 0 : -12 }); return }
  markActiveArea(-1);
  await fly(allAreasBounds(f.areas), { pitch: 0, bearing: 0, duration: fast ? 0 : 1800, maxZoom: 7 });
  if (fast || f.areas.length < 2 || !still()) return;
  await pause(900);
  for (let i = 0; i < Math.min(6, f.areas.length) && still(); i++) {
    markActiveArea(i);
    await fly(f.areas[i].bbox, { pitch: 42, bearing: (i % 2 ? 14 : -14), duration: 2600 });
    if (!still()) return;
    await pause(1500);
  }
  if (!still()) return;
  markActiveArea(-1);
  await fly(allAreasBounds(f.areas), { pitch: 0, bearing: 0, duration: 2000, maxZoom: 7 });
}
/// Spotlight mask, area fills, glowing outlines and numbered labels for the focused mineral.
function mountFocus() {
  const map = mapInstance; if (!map) return;
  try {
    const f = state.mineralMode && state.view === 'situation' ? state.focus : null;
    const areas = f?.areas || [];
    const polys = { type: 'FeatureCollection', features: areas.map((a, i) => ({ type: 'Feature', properties: { rank: i, label: `${i + 1} · ${a.province} · ${a.count}` }, geometry: { type: 'Polygon', coordinates: [a.ring] } })) };
    const labels = { type: 'FeatureCollection', features: areas.map((a, i) => ({ type: 'Feature', properties: { rank: i, label: `${i + 1}  ${(a.territory || a.province).toUpperCase()}`, sub: `${a.territory ? a.province + ' · ' : ''}${a.count} ${L('sites', 'sites')}` }, geometry: { type: 'Point', coordinates: a.centroid } })) };
    const mask = { type: 'FeatureCollection', features: areas.length ? [spotlightMask(areas)] : [] };
    const signature = JSON.stringify([f?.key, areas.length, f?.hex]);
    if (!map.getSource('focus-areas')) {
      map.addSource('focus-mask', { type: 'geojson', data: mask });
      map.addSource('focus-areas', { type: 'geojson', data: polys });
      map.addSource('focus-labels', { type: 'geojson', data: labels });
      const before = ['mineral-clusters', 'mineral-sites'].find(l => map.getLayer(l));
      map.addLayer({ id: 'focus-mask', type: 'fill', source: 'focus-mask', paint: { 'fill-color': '#02060a', 'fill-opacity': 0.55 } }, before);
      map.addLayer({ id: 'focus-fill', type: 'fill', source: 'focus-areas', paint: { 'fill-color': '#c6f36b', 'fill-opacity': 0.1 } }, before);
      map.addLayer({ id: 'focus-glow', type: 'line', source: 'focus-areas', paint: { 'line-color': '#c6f36b', 'line-width': 9, 'line-blur': 7, 'line-opacity': 0.55 } }, before);
      map.addLayer({ id: 'focus-line', type: 'line', source: 'focus-areas', paint: { 'line-color': '#c6f36b', 'line-width': 1.8 } }, before);
      map.addLayer({ id: 'focus-active', type: 'line', source: 'focus-areas', filter: ['==', ['get', 'rank'], -1], paint: { 'line-color': '#ffffff', 'line-width': 2.6, 'line-dasharray': [2, 1.2] } }, before);
      map.addLayer({ id: 'focus-labels', type: 'symbol', source: 'focus-labels', layout: { 'text-field': ['format', ['get', 'label'], { 'font-scale': 0.95 }, '\n', {}, ['get', 'sub'], { 'font-scale': 0.78 }], 'text-font': ['DIN Pro Medium', 'Arial Unicode MS Regular'], 'text-size': 12, 'text-letter-spacing': 0.08, 'text-allow-overlap': false, 'text-variable-anchor': ['top', 'bottom', 'left', 'right'], 'text-radial-offset': 1.2 }, paint: { 'text-color': '#ffffff', 'text-halo-color': '#02060a', 'text-halo-width': 1.6 } });
      map.__focusSignature = '';
    }
    if (map.__focusSignature !== signature) {
      map.getSource('focus-mask').setData(mask);
      map.getSource('focus-areas').setData(polys);
      map.getSource('focus-labels').setData(labels);
      for (const id of ['focus-fill', 'focus-glow', 'focus-line']) map.setPaintProperty(id, id === 'focus-fill' ? 'fill-color' : 'line-color', f?.hex || '#c6f36b');
      map.__focusSignature = signature;
    }
    map.setFilter('focus-active', ['==', ['get', 'rank'], f?.active ?? -1]);
  } catch {}
}
function focusPanel() {
  const f = state.focus, c = state.commodities?.items?.find(x => x.id === f.commodity);
  const s = c ? priceSummary(c) : null, h = c ? mainTrade(c) : null;
  const provinces = [...new Set(f.areas.flatMap(a => a.provinces))];
  return `<div class="mine-dossier focus-panel" data-scroll="focus" style="--focus:${f.hex}">
    <div class="dossier-top"><span class="eyebrow"><i class="swatch" style="background:${f.hex}"></i>${L('MINERAL FOCUS', 'FOCUS MINERAI')}</span><button class="iconbtn" data-act="focus-exit" aria-label="${L('Exit focus', 'Quitter le focus')}">${icon('close')}</button></div>
    <h2 class="mine-name">${esc(f.name)}</h2>
    <div class="focus-stats"><div><b class="num">${nf(f.sites)}</b><small>${L('sites', 'sites')}</small></div><div><b class="num">${f.areas.length}</b><small>${L('mining areas', 'zones minières')}</small></div><div><b class="num">${provinces.length}</b><small>${L('provinces', 'provinces')}</small></div></div>
    <div class="actions" style="margin:10px 0 14px"><button class="btn sm primary" data-act="focus-tour">${icon('globe')}${L('Replay tour', 'Rejouer la visite')}</button><button class="btn sm" data-act="focus-overview">${L('All areas', 'Toutes les zones')}</button><button class="linkbtn" data-act="focus-exit">${L('Exit focus', 'Quitter')}</button></div>
    <div class="eyebrow" style="margin-bottom:6px">${L('Mining areas · sites within', 'Zones minières · sites à moins de')} ${FOCUS_LINK_KM} km</div>
    <div class="area-list">${f.areas.map((a, i) => { const kinds = countBy(a.sites, x => x.properties.kind).map(([k, n]) => `${n} ${mineralKindLabel(k).toLowerCase()}`).join(' · '); return `<button class="area-row ${f.active === i ? 'active' : ''}" data-area="${i}" data-act="focus-area" data-arg="${i}"><b class="area-rank">${i + 1}</b><div><strong>${esc(a.name)}</strong><small>${esc(kinds)}</small><small class="muted">${esc(a.sites.slice(0, 3).map(x => x.properties.name).join(', '))}${a.count > 3 ? '…' : ''}</small></div><div class="area-n"><b class="num">${nf(a.count)}</b><small class="num">${nf(Math.round(a.km2))} km²</small></div></button>` }).join('') || `<p class="muted">${L('Sites are too scattered to form areas of 3+ sites.', 'Sites trop dispersés pour former des zones de 3 sites ou plus.')}</p>`}</div>
    ${c ? `<section class="panel market-card"><div class="panel-head"><h2>${L('Market', 'Marché')}</h2><button class="linkbtn" data-act="market-open" data-arg="${esc(c.id)}">${L('Full market', 'Marché complet')} →</button></div><div class="panel-body">${s ? `<div class="price-line"><b>${s.value}</b><small>${esc(s.unit)}</small>${changeBadge(s.change)}<span class="grow"></span>${s.history.length > 1 ? spark(s.history.slice(-24), 'var(--signal)', 120, 28) : ''}</div><div class="muted" style="font-size:11.5px">${esc(s.note)}</div>` : ''}<div class="eyebrow" style="margin:12px 0 6px">${L('Who buys', 'Qui achète')}</div>${buyersBlock(h, 5)}</div></section>` : ''}
    <p class="notice">${L('Areas group historical site records by proximity; outlines are drawn around those sites with a small margin. They are not concession boundaries, deposits or zones of control.', 'Les zones regroupent des références historiques par proximité ; les contours entourent ces sites avec une marge. Ce ne sont ni des concessions, ni des gisements, ni des zones de contrôle.')}</p></div>`;
}

// ---------------------------------------------------------------------------------------------
// Event log, watchlists and alerts, exportable briefs.

state.watches = null; state.alerts = null; state.events = null; state.watchEdit = null; state.watchResults = null; state.alertsUnread = false;
state.evDays = 30; state.evType = ''; state.evProvince = ''; state.evQ = ''; state.evDay = ''; state.evOpen = ''; state.briefPeriod = 'daily';
const EVENT_TYPES = {
  armed_clash: ['Armed clash', 'Affrontement armé', '#ff5d6c'], attack_on_civilians: ['Attack on civilians', 'Attaque contre des civils', '#ff8a3d'],
  explosive_or_drone: ['Drone, shelling, explosion', 'Drone, bombardement, explosion', '#ffc65c'], abduction: ['Abduction', 'Enlèvement', '#c99bff'],
  territorial_control: ['Territorial control', 'Contrôle territorial', '#e5ad53'], displacement: ['Displacement', 'Déplacement', '#82e3f5'],
  sexual_violence: ['Sexual violence', 'Violences sexuelles', '#ff6fb5'], protest_unrest: ['Protest, unrest', 'Manifestation, troubles', '#7ab8ff'],
  arrest_detention: ['Arrest, detention', 'Arrestation, détention', '#a4acb5'], health_outbreak: ['Disease outbreak', 'Épidémie', '#6affb9'], disaster: ['Disaster', 'Catastrophe', '#5fa8d3'],
};
const evLabel = k => EVENT_TYPES[k] ? L(EVENT_TYPES[k][0], EVENT_TYPES[k][1]) : k;
const evColor = k => EVENT_TYPES[k]?.[2] || '#a4acb5';
const WATCH_KINDS = {
  search: ['Words or phrases', 'Mots ou expressions', 'search'], place: ['Places', 'Lieux', 'pin'], actor: ['Key figures', 'Personnalités', 'flag'],
  sanctioned: ['Any sanctioned person', 'Toute personne sanctionnée', 'shield'], military_flight: ['Military flights', 'Vols militaires', 'globe'],
  aircraft: ['Aircraft (callsign, registration…)', 'Aéronefs (indicatif, immatriculation…)', 'globe'], brief: ['Daily or weekly brief', 'Synthèse quotidienne ou hebdomadaire', 'book'],
};
const watchKind = k => WATCH_KINDS[k] ? L(WATCH_KINDS[k][0], WATCH_KINDS[k][1]) : k;

// --- Event log ---------------------------------------------------------------------------------
function eventRows(withDay = true) {
  const q = norm(state.evQ.trim());
  return (state.events?.items || []).filter(e => (!state.evType || e.kind === state.evType) && (!state.evProvince || e.province === state.evProvince)
    && (!withDay || !state.evDay || e.date === state.evDay)
    && (!q || norm([e.place, e.province, ...(e.actors || []), ...e.sources.map(s => `${s.title} ${s.excerpt}`)].join(' ')).includes(q)));
}
const evDays = () => { const n = Math.min(state.evDays, 90); return [...Array(n)].map((_, i) => new Date(Date.now() - (n - 1 - i) * 864e5).toISOString().slice(0, 10)) };
function eventMap(rows) {
  const xy = (lon, lat) => [20 + (lon - 12) * 18, 16 + (5.6 - lat) * 18];
  const ring = state.geometry?.features?.[0]?.geometry?.coordinates || [];
  const path = ring.map(r => r.map((p, i) => `${i ? 'L' : 'M'}${xy(...p).map(v => v.toFixed(1)).join(',')}`).join(' ') + 'Z').join(' ');
  const dots = [...rows].sort((a, b) => (b.fatalities || 0) - (a.fatalities || 0)).map(e => {
    const [x, y] = xy(e.lon, e.lat), r = 3.5 + Math.sqrt(e.fatalities || 0) * 1.6 + (e.reports - 1) * 1.2;
    return `<circle class="ev-dot ${state.evOpen === e.id ? 'open' : ''}" cx="${x.toFixed(1)}" cy="${y.toFixed(1)}" r="${r.toFixed(1)}" fill="${evColor(e.kind)}" data-act="ev-open" data-arg="${esc(e.id)}" tabindex="0" role="button" aria-label="${esc(`${e.date} · ${evLabel(e.kind)} · ${e.place}`)}" data-tip="${esc(`${e.date} · ${evLabel(e.kind)} · ${e.place}${e.fatalities ? ` · ${e.fatalities} ${L('deaths reported', 'morts rapportés')}` : ''}`)}"/>`;
  }).join('');
  return `<svg class="ev-map" viewBox="0 0 400 380" role="img" aria-label="${L('Map of extracted incidents', 'Carte des incidents extraits')}"><path class="map-country" d="${path}"/>${dots}</svg>`;
}
function eventsView() {
  const d = state.events, all = d?.items || [], rows = eventRows(), dayless = eventRows(false), days = evDays();
  const per = days.map(day => { const m = {}; for (const e of dayless) if (e.date === day) m[e.kind] = (m[e.kind] || 0) + 1; return m });
  const max = Math.max(1, ...per.map(m => Object.values(m).reduce((a, b) => a + b, 0)));
  const provinces = [...new Set(all.map(e => e.province))].sort();
  const kinds = Object.keys(EVENT_TYPES).filter(k => all.some(e => e.kind === k));
  const deaths = rows.reduce((n, e) => n + (e.fatalities || 0), 0), corroborated = rows.filter(e => e.outlets > 1 || e.reports > 1).length;
  const byDate = [...rows.reduce((m, e) => m.set(e.date, [...(m.get(e.date) || []), e]), new Map())];
  const card = e => `<article class="ev-card ${state.evOpen === e.id ? 'open' : ''}" id="ev-${esc(e.id)}" style="--c:${evColor(e.kind)}">
    <header><span class="ev-type">${esc(evLabel(e.kind))}</span><strong>${esc(e.place)}</strong><small>${esc(e.province)}</small>${e.fatalities ? `<b class="ev-toll">${e.fatalities} ${L('deaths reported', 'morts rapportés')}</b>` : ''}${e.reports > 1 ? `<span class="badge">${e.reports} ${L('reports', 'articles')} · ${e.outlets} ${L('outlets', 'médias')}</span>` : ''}</header>
    ${e.actors?.length ? `<div class="chips">${e.actors.map(a => `<span class="chip">${esc(a)}</span>`).join('')}</div>` : ''}
    <p class="ev-excerpt">“${esc(e.sources[0]?.excerpt || '')}”</p>
    <details ${state.evOpen === e.id ? 'open' : ''}><summary>${L('Sources', 'Sources')} (${e.sources.length})</summary>${e.sources.map(s => `<div class="ev-source"><div><strong>${esc(s.title)}</strong><small>${esc(s.source || '')} · ${esc(String(s.published_at || '').slice(0, 10))}${s.fatalities ? ` · ${s.fatalities} ${L('deaths', 'morts')}` : ''}</small><p>“${esc(s.excerpt)}”</p></div><div class="actions"><button class="btn sm" data-act="report" data-arg="${esc(s.article_id)}">${L('Open report', 'Ouvrir')}</button>${s.url ? `<a class="btn sm ghost" href="${esc(s.url)}" target="_blank" rel="noopener noreferrer">${L('Original', 'Original')} ↗</a>` : ''}</div></div>`).join('')}</details>
    <div class="ev-foot"><button class="linkbtn" data-act="watch-quick" data-arg="${esc(`place|${e.place}|${e.place}`)}">${icon('bell')} ${L('Watch', 'Suivre')} ${esc(e.place)}</button></div></article>`;
  return pageHead(`${L('DRC', 'RDC')} · ${L('EVENT LOG', 'JOURNAL DES ÉVÉNEMENTS')}`, L('Incidents over time', 'Incidents dans le temps'), L('Dated incidents extracted from every collected report: what happened, where, when, who was named and the death toll reported. Reports of the same incident are merged so you can see corroboration.', 'Incidents datés extraits de chaque article collecté : quoi, où, quand, qui est cité et le bilan rapporté. Les articles d’un même incident sont regroupés pour montrer la corroboration.'),
    `<div class="seg">${[7, 30, 90].map(n => `<button class="${state.evDays === n ? 'active' : ''}" data-act="ev-days" data-arg="${n}" aria-pressed="${state.evDays === n}">${n} ${L('d', 'j')}</button>`).join('')}</div><button class="btn" data-act="ev-play" aria-pressed="${!!evPlayer}">${evPlayer ? '❚❚ ' + L('Pause', 'Pause') : '▶ ' + L('Play timeline', 'Lire la chronologie')}</button>`)
    + `<div class="kpis ev-kpis">${[[L('Incidents', 'Incidents'), rows.length], [L('Deaths reported', 'Morts rapportés'), deaths], [L('Places', 'Lieux'), new Set(rows.map(e => e.place)).size], [L('Corroborated', 'Corroborés'), corroborated]].map(([l, v]) => `<div class="panel kpi"><div class="kpi-label">${l}</div><div class="kpi-value">${nf(v)}</div></div>`).join('')}</div>
    <div class="ev-layout">
      <section class="panel ev-map-panel"><div class="panel-head"><h2>${icon('map')} ${state.evDay ? esc(dayLabel(state.evDay)) : L('All days in range', 'Toute la période')}</h2>${state.evDay ? `<button class="linkbtn" data-act="ev-day" data-arg="">${L('All days', 'Tous les jours')}</button>` : ''}</div>${eventMap(rows)}
        <div class="ev-timeline" role="group" aria-label="${L('Incidents per day', 'Incidents par jour')}">${days.map((day, i) => { const m = per[i], total = Object.values(m).reduce((a, b) => a + b, 0); return `<button class="ev-day ${state.evDay === day ? 'active' : ''}" data-act="ev-day" data-arg="${day}" aria-pressed="${state.evDay === day}" aria-label="${esc(`${dayLabel(day)}: ${total}`)}" data-tip="${esc(`${dayLabel(day)} · ${total} ${L('incidents', 'incidents')}`)}"><span>${Object.keys(EVENT_TYPES).filter(k => m[k]).map(k => `<i style="height:${(m[k] / max * 100).toFixed(1)}%;background:${evColor(k)}"></i>`).join('')}</span></button>` }).join('')}</div>
        <div class="ev-axis"><span>${esc(dayLabel(days[0]))}</span><span>${esc(dayLabel(days.at(-1)))}</span></div>
        <p class="notice">${esc(L(d?.notice || '', 'Incidents extraits automatiquement des textes (mots-indices, lieux du gazetier, dates relatives à la publication). Non vérifiés : consultez les extraits sources. Les articles de même type, lieu et jour sont regroupés ; le bilan est le plus élevé rapporté par une source.'))}</p></section>
      <section class="ev-list">
        <div class="toolbar"><label class="search">${icon('search')}<input id="ev-q" class="input" type="search" data-model="evQ" value="${esc(state.evQ)}" placeholder="${L('Place, armed group, word…', 'Lieu, groupe armé, mot…')}" aria-label="${L('Search incidents', 'Rechercher des incidents')}"></label>
          <select class="select" data-model="evProvince" aria-label="${L('Province', 'Province')}"><option value="">${L('All provinces', 'Toutes les provinces')}</option>${provinces.map(p => `<option ${state.evProvince === p ? 'selected' : ''}>${esc(p)}</option>`).join('')}</select></div>
        <div class="flight-cats ev-types"><button class="${state.evType ? '' : 'active'}" data-act="ev-type" data-arg="" aria-pressed="${!state.evType}">${L('All', 'Tous')}<b>${dayless.length}</b></button>${kinds.map(k => `<button class="${state.evType === k ? 'active' : ''}" data-act="ev-type" data-arg="${k}" aria-pressed="${state.evType === k}" style="--c:${evColor(k)}"><i></i>${esc(evLabel(k))}<b>${(state.events?.items || []).filter(e => e.kind === k).length}</b></button>`).join('')}</div>
        ${byDate.map(([day, list]) => `<h3 class="ev-date">${esc(dayLabel(day))}<small>${list.length}</small></h3>${list.map(card).join('')}`).join('') || emptyState(L('No incidents match', 'Aucun incident ne correspond'), d ? L('Widen the period or clear filters.', 'Élargissez la période ou effacez les filtres.') : L('The event log is being built from collected reports.', 'Le journal est en cours de construction.'))}
      </section></div>`;
}
let evPlayer = null;
function toggleEvPlay() {
  if (evPlayer) { clearInterval(evPlayer); evPlayer = null; render(); return }
  const days = evDays(); let i = Math.max(0, days.indexOf(state.evDay) + 1); if (i >= days.length) i = 0;
  state.evDay = days[i]; render();
  evPlayer = setInterval(() => {
    if (state.view !== 'events') { clearInterval(evPlayer); evPlayer = null; return }
    i++; if (i >= days.length) { clearInterval(evPlayer); evPlayer = null; render(); return }
    state.evDay = days[i]; render();
  }, reducedMotion() ? 1600 : 800);
}

// --- Watchlists and alerts ---------------------------------------------------------------------
const blankWatch = (kind = 'search') => ({ id: '', name: '', kind, termsText: '', region: 'drc', period: 'daily', hour: 6, orgs: false, email: '', telegram: '', active: true });
function watchToEdit(w) {
  const names = w.kind === 'actor' ? w.terms.map(id => state.actors?.items?.find(a => a.id === id)?.name || id) : w.terms;
  return { id: w.id, name: w.name, kind: w.kind, termsText: names.join(', '), region: w.options?.region || 'drc', period: w.options?.period || 'daily', hour: w.options?.hour_utc ?? 6, orgs: !!w.options?.include_organisations, email: w.notify?.email || '', telegram: w.notify?.telegram || '', active: w.active };
}
function actorIdFor(name) {
  const f = norm(name.trim()); const items = state.actors?.items || [];
  return (items.find(a => norm(a.name) === f) || items.find(a => (a.aliases || []).some(x => norm(x) === f)) || items.find(a => a.id === name.trim()))?.id;
}
async function saveWatch(edit) {
  let terms = edit.termsText.split(',').map(t => t.trim()).filter(Boolean);
  if (edit.kind === 'actor') {
    const ids = terms.map(actorIdFor);
    const missing = terms.filter((_, i) => !ids[i]);
    if (missing.length) throw Error(L(`Unknown key figure: ${missing.join(', ')}. Pick names from the list.`, `Personnalité inconnue : ${missing.join(', ')}. Choisissez dans la liste.`));
    terms = ids;
  }
  const body = { id: edit.id || null, name: edit.name.trim() || terms.slice(0, 3).join(', ') || watchKind(edit.kind), kind: edit.kind, terms, options: { region: edit.region, period: edit.period, hour_utc: Number(edit.hour), include_organisations: !!edit.orgs }, notify: { email: edit.email.trim() || null, telegram: edit.telegram.trim() || null }, active: edit.active };
  await api('/v1/watches', body);
}
async function refreshWatches() {
  const [watches, alerts] = await Promise.all([api('/v1/watches').catch(() => null), api('/v1/alerts?limit=200').catch(() => null)]);
  Object.assign(state, { watches, alerts }); render();
}
function watchForm() {
  const e = state.watchEdit; if (!e) return '';
  const ch = state.watches?.channels || {};
  const placeList = [...new Set([...placesList().map(p => p.name), ...(state.events?.items || []).map(x => x.place)])].sort();
  const needsTerms = ['search', 'place', 'actor', 'aircraft'].includes(e.kind);
  return `<section class="panel watch-form"><div class="panel-head"><h2>${icon('bell')} ${e.id ? L('Edit watch', 'Modifier la veille') : L('New watch', 'Nouvelle veille')}</h2><button class="iconbtn" data-act="watch-cancel" aria-label="${L('Cancel', 'Annuler')}">${icon('close')}</button></div><div class="panel-body form-grid">
    <label>${L('What to watch', 'Que suivre')}<select class="select" data-model="watchEdit.kind">${Object.keys(WATCH_KINDS).map(k => `<option value="${k}" ${e.kind === k ? 'selected' : ''}>${esc(watchKind(k))}</option>`).join('')}</select></label>
    <label>${L('Name', 'Nom')}<input class="input" id="watch-name" data-model="watchEdit.name" value="${esc(e.name)}" maxlength="80" placeholder="${L('e.g. Goma and M23', 'ex. Goma et M23')}"></label>
    ${needsTerms ? `<label class="wide">${{ search: L('Words or phrases, comma-separated (acronyms like M23 match in capitals only)', 'Mots ou expressions, séparés par des virgules (les sigles comme M23 en majuscules uniquement)'), place: L('Places, comma-separated', 'Lieux, séparés par des virgules'), actor: L('Key figures, comma-separated', 'Personnalités, séparées par des virgules'), aircraft: L('Callsigns, registrations, ICAO hex or type codes (prefixes work: ETH, UNO)', 'Indicatifs, immatriculations, code OACI ou type (préfixes acceptés : ETH, UNO)') }[e.kind]}<input class="input" id="watch-terms" data-model="watchEdit.termsText" value="${esc(e.termsText)}" list="watch-suggest" autocomplete="off"></label>
      <datalist id="watch-suggest">${(e.kind === 'actor' ? (state.actors?.items || []).map(a => a.name) : e.kind === 'place' ? placeList : []).slice(0, 600).map(v => `<option value="${esc(v)}">`).join('')}</datalist>` : ''}
    ${e.kind === 'sanctioned' ? `<label class="check wide"><input type="checkbox" data-model="watchEdit.orgs" ${e.orgs ? 'checked' : ''}>${L('Also sanctioned organisations and armed groups (M23, RDF… appear in most posts)', 'Inclure aussi les organisations et groupes armés sanctionnés (M23, RDF… présents dans la plupart des publications)')}</label>` : ''}
    ${e.kind === 'military_flight' ? `<label>${L('Area', 'Zone')}<select class="select" data-model="watchEdit.region"><option value="drc" ${e.region === 'drc' ? 'selected' : ''}>${L('Inside the DRC', 'Dans la RDC')}</option><option value="any" ${e.region === 'any' ? 'selected' : ''}>${L('DRC and approaches', 'RDC et abords')}</option></select></label>` : ''}
    ${e.kind === 'brief' ? `<label>${L('Frequency', 'Fréquence')}<select class="select" data-model="watchEdit.period"><option value="daily" ${e.period === 'daily' ? 'selected' : ''}>${L('Daily', 'Quotidienne')}</option><option value="weekly" ${e.period === 'weekly' ? 'selected' : ''}>${L('Weekly (Mondays)', 'Hebdomadaire (lundi)')}</option></select></label><label>${L('Hour (UTC; Kinshasa = UTC+1)', 'Heure (UTC ; Kinshasa = UTC+1)')}<input class="input" type="number" min="0" max="23" data-model="watchEdit.hour" value="${esc(e.hour)}"></label>` : ''}
    <div class="wide watch-channels"><div class="eyebrow">${L('Also notify me by', 'Me prévenir aussi par')}</div>
      <label>${L('Email', 'E-mail')}<input class="input" id="watch-email" type="email" data-model="watchEdit.email" value="${esc(e.email)}" placeholder="${esc(/@/.test(state.watches?.email_default || '') ? state.watches.email_default : 'analyst@example.org')}"></label>
      <label>${L('Telegram chat id', 'Identifiant de discussion Telegram')}<span class="inline"><input class="input" id="watch-telegram" data-model="watchEdit.telegram" value="${esc(e.telegram)}" placeholder="123456789"><button class="btn sm" data-act="telegram-chats" ${ch.telegram ? '' : 'disabled'}>${L('Find my chat', 'Trouver ma discussion')}</button></span></label>
      ${state.telegramChats ? `<div class="chips">${state.telegramChats.chats.map(c => `<button class="chip" data-act="telegram-pick" data-arg="${esc(c.id)}">${esc(c.label)} · ${esc(c.id)}</button>`).join('') || `<span class="muted">${L(`No chats yet. Send any message to @${state.telegramChats.bot || 'your bot'} on Telegram, then try again.`, `Aucune discussion. Envoyez un message à @${state.telegramChats.bot || 'votre bot'} sur Telegram puis réessayez.`)}</span>`}</div>` : ''}
      <div class="actions"><button class="btn sm" data-act="watch-test" data-arg="email" ${ch.email ? '' : 'disabled'}>${L('Test email', 'Tester l’e-mail')}</button><button class="btn sm" data-act="watch-test" data-arg="telegram" ${ch.telegram ? '' : 'disabled'}>${L('Test Telegram', 'Tester Telegram')}</button></div>
      ${!ch.email || !ch.telegram ? `<p class="notice">${!ch.email ? L('Email needs SMTP settings on the server (SMTP_HOST, SMTP_PORT, SMTP_USERNAME, SMTP_PASSWORD, SMTP_FROM). ', 'L’e-mail nécessite des paramètres SMTP sur le serveur (SMTP_HOST, SMTP_PORT, SMTP_USERNAME, SMTP_PASSWORD, SMTP_FROM). ') : ''}${!ch.telegram ? L('Telegram needs a bot token on the server (TELEGRAM_BOT_TOKEN, from @BotFather). ', 'Telegram nécessite un jeton de bot sur le serveur (TELEGRAM_BOT_TOKEN, via @BotFather). ') : ''}${L('Alerts always appear in this inbox.', 'Les alertes apparaissent toujours dans cette boîte.')}</p>` : ''}</div>
    <div class="wide actions"><button class="btn primary" data-act="watch-save">${icon('check')} ${L('Save watch', 'Enregistrer')}</button>${e.kind !== 'brief' && e.kind !== 'military_flight' ? `<span class="muted" style="font-size:12px">${L('New matches alert you from now on; past matches are under Results.', 'Les nouvelles correspondances déclenchent une alerte ; les anciennes sont dans Résultats.')}</span>` : ''}</div>
  </div></section>`;
}
function alertOpenArgs(a) { return `${a.id}|${a.kind}|${a.ref}` }
function alertRow(a) {
  const d = a.detail || {}, k = a.kind;
  const ic = { article: 'layers', post: 'radio', flight: 'globe', brief: 'book' }[k] || 'bell';
  const deliv = Object.entries(a.delivery || {}).filter(([c]) => c !== 'at').map(([c, v]) => `<span class="badge ${String(v).startsWith('failed') ? 'high' : ''}" data-tip="${esc(String(v))}">${c === 'email' ? L('email', 'e-mail') : c} ${String(v).startsWith('failed') ? '✕' : '✓'}</span>`).join('');
  return `<button class="alert-row ${a.read ? '' : 'unread'}" data-act="alert-open" data-arg="${esc(alertOpenArgs(a))}"><span class="alert-icon">${icon(ic)}</span><div><strong>${esc(a.title)}</strong><small>${esc(a.watch_name || '')}${(d.matched || []).length ? ` · ${esc(d.matched.join(', '))}` : ''}${d.source ? ` · ${esc(d.source)}` : ''} · ${age(a.created_at)}</small>${d.snippet ? `<p>${esc(d.snippet)}</p>` : d.summary ? `<p>${esc(d.summary)}</p>` : d.basis ? `<p>${esc(d.basis)}</p>` : ''}</div><span class="alert-meta">${deliv}</span></button>`;
}
function alertsView() {
  const ws = state.watches?.items || [], al = state.alerts?.items || [], unread = state.alerts?.unread || 0;
  const list = state.alertsUnread ? al.filter(a => !a.read) : al;
  const r = state.watchResults;
  return pageHead(`${L('MONITOR', 'VEILLE')} · ${L('WATCHLISTS', 'LISTES DE SUIVI')}`, L('Watchlists & alerts', 'Veilles et alertes'), L('Save a search, a place, a key figure, any sanctioned name, military flights or aircraft, and get an alert when they appear. Briefs can be delivered daily or weekly.', 'Enregistrez une recherche, un lieu, une personnalité, tout nom sanctionné, les vols militaires ou un aéronef, et recevez une alerte dès qu’ils apparaissent. Les synthèses peuvent être envoyées chaque jour ou chaque semaine.'),
    `<button class="btn primary" data-act="watch-new">${icon('bell')} ${L('New watch', 'Nouvelle veille')}</button>`)
    + `<div class="watch-layout"><div class="stack">${watchForm()}
      <section class="panel"><div class="panel-head"><h2>${L('Your watches', 'Vos veilles')}</h2><small>${ws.length}</small></div>
        ${ws.map(w => `<div class="watch-row ${w.active ? '' : 'paused'}"><span class="watch-icon">${icon(WATCH_KINDS[w.kind]?.[2] || 'bell')}</span><div><strong>${esc(w.name)}</strong><small>${esc(watchKind(w.kind))}${w.kind === 'sanctioned' ? (w.options?.include_organisations ? L(' · people and organisations', ' · personnes et organisations') : L(' · people', ' · personnes')) : ''}${w.terms.length ? ' · ' + esc((w.kind === 'actor' ? w.terms.map(id => state.actors?.items?.find(a => a.id === id)?.name || id) : w.terms).join(', ')) : ''}${w.kind === 'brief' ? ` · ${w.options.period === 'weekly' ? L('weekly', 'hebdomadaire') : L('daily', 'quotidienne')} ${String(w.options.hour_utc).padStart(2, '0')}:00 UTC` : ''}</small>
          <small class="watch-stats">${nf(w.hits)} ${L('alerts', 'alertes')}${w.unread ? ` · <b>${w.unread} ${L('unread', 'non lues')}</b>` : ''}${w.notify?.email ? ` · ${icon('mail')}${esc(w.notify.email)}` : ''}${w.notify?.telegram ? ` · Telegram` : ''}${w.active ? '' : ` · ${L('paused', 'en pause')}`}</small></div>
          <div class="actions">${['search', 'place', 'actor', 'sanctioned', 'military_flight', 'aircraft'].includes(w.kind) ? `<button class="btn sm" data-act="watch-results" data-arg="${esc(w.id)}">${L('Results', 'Résultats')}</button>` : ''}<button class="btn sm ghost" data-act="watch-edit" data-arg="${esc(w.id)}">${L('Edit', 'Modifier')}</button><button class="btn sm ghost" data-act="watch-toggle" data-arg="${esc(w.id)}">${w.active ? L('Pause', 'Pause') : L('Resume', 'Reprendre')}</button><button class="iconbtn" data-act="watch-delete" data-arg="${esc(w.id)}" aria-label="${L('Delete watch', 'Supprimer la veille')}">${icon('close')}</button></div></div>`).join('') || `<div class="panel-body">${emptyState(L('No watches yet', 'Aucune veille'), L('Try Goma, M23, a key figure, or every sanctioned name.', 'Essayez Goma, M23, une personnalité ou tous les noms sanctionnés.'), `<div class="actions" style="justify-content:center">${[['search', 'Goma, M23'], ['sanctioned', ''], ['military_flight', ''], ['brief', '']].map(([k, t]) => `<button class="btn sm" data-act="watch-template" data-arg="${k}|${esc(t)}">${esc(watchKind(k))}${t ? `: ${esc(t)}` : ''}</button>`).join('')}</div>`)}</div>`}</section></div>
      <div class="stack">${r ? `<section class="panel"><div class="panel-head"><h2>${L('Results', 'Résultats')} · ${esc(r.name)}</h2><small>${r.data ? `${r.data.items.length} · ${esc(r.data.scope)}` : '…'}</small><button class="iconbtn" data-act="watch-results-close" aria-label="${L('Close results', 'Fermer les résultats')}">${icon('close')}</button></div>${r.data ? r.data.items.map(x => `<button class="alert-row" data-act="${x.kind === 'article' ? 'report' : x.kind === 'flight' ? 'nav' : 'open-url'}" data-arg="${esc(x.kind === 'article' ? x.ref : x.kind === 'flight' ? 'flights' : x.url || '')}"><span class="alert-icon">${icon(x.kind === 'post' ? 'radio' : x.kind === 'flight' ? 'globe' : 'layers')}</span><div><strong>${esc(x.title)}</strong><small>${esc((x.matched || []).join(', '))}${x.source ? ' · ' + esc(x.source) : ''}${x.at && typeof x.at === 'string' ? ' · ' + age(x.at) : ''}</small>${x.snippet ? `<p>${esc(x.snippet)}</p>` : ''}</div></button>`).join('') || `<div class="panel-body muted">${L('No past matches.', 'Aucune correspondance passée.')}</div>` : '<div class="loading-line"></div>'}</section>` : ''}
      <section class="panel"><div class="panel-head"><h2>${icon('bell')} ${L('Alert inbox', 'Boîte d’alertes')}</h2><div class="seg"><button class="${state.alertsUnread ? '' : 'active'}" data-act="alerts-filter" data-arg="all" aria-pressed="${!state.alertsUnread}">${L('All', 'Toutes')}</button><button class="${state.alertsUnread ? 'active' : ''}" data-act="alerts-filter" data-arg="unread" aria-pressed="${state.alertsUnread}">${L('Unread', 'Non lues')} ${unread}</button></div>${unread ? `<button class="linkbtn" data-act="alerts-read-all">${L('Mark all read', 'Tout marquer comme lu')}</button>` : ''}</div>
        ${list.map(alertRow).join('') || `<div class="panel-body muted">${ws.length ? L('No alerts yet. Matches appear here as new reports, posts and flights arrive.', 'Aucune alerte. Les correspondances apparaissent ici à l’arrivée de nouveaux articles, publications et vols.') : L('Create a watch to start receiving alerts.', 'Créez une veille pour recevoir des alertes.')}</div>`}</section></div></div>`;
}
async function quickWatch(arg) {
  const [kind, term, name] = arg.split('|');
  const label = name || term;
  await api('/v1/watches', { name: label, kind, terms: term ? [term] : [], options: {}, notify: {} });
  toast(L(`Watching ${label}. Manage alerts in Watchlists.`, `Suivi de ${label} activé. Gérez les alertes dans Veilles.`));
  refreshWatches();
}

// --- Brief export ------------------------------------------------------------------------------
function authHeaders() { const t = sessionStorage.getItem('congo-token'); return t ? { Authorization: `Bearer ${t}` } : {} }
async function downloadBrief(format) {
  const period = state.briefPeriod, lang = state.lang;
  // Open the window inside the click so popup blockers allow it, then fill it.
  const win = format === 'pdf' ? window.open('', '_blank') : null;
  const r = await fetch(`/v1/brief?period=${period}&lang=${lang}&format=${format === 'pdf' ? 'html' : 'docx'}${format === 'pdf' ? '&print=true' : ''}`, { headers: authHeaders() });
  if (!r.ok) { win?.close(); throw Error(L('The brief could not be generated.', 'La synthèse n’a pas pu être générée.') + ` (${r.status})`) }
  if (win) { const html = await r.text(); win.document.open(); win.document.write(html); win.document.close(); return }
  const blob = await r.blob(), a = document.createElement('a');
  a.href = URL.createObjectURL(blob); a.download = (r.headers.get('content-disposition') || '').match(/filename="([^"]+)"/)?.[1] || `congo-signal-${period}-brief.docx`;
  document.body.append(a); a.click(); a.remove(); setTimeout(() => URL.revokeObjectURL(a.href), 4000);
}
const btnBrief = () => `<div class="brief-export" role="group" aria-label="${L('Export brief', 'Exporter la synthèse')}"><select class="select" data-model="briefPeriod" aria-label="${L('Brief period', 'Période de la synthèse')}"><option value="daily" ${state.briefPeriod === 'daily' ? 'selected' : ''}>${L('Daily brief', 'Synthèse du jour')}</option><option value="weekly" ${state.briefPeriod === 'weekly' ? 'selected' : ''}>${L('Weekly brief', 'Synthèse de la semaine')}</option></select><button class="btn" data-act="brief-dl" data-arg="docx">${icon('download')}Word</button><button class="btn" data-act="brief-dl" data-arg="pdf">${icon('download')}PDF</button></div>`;

// ---------------------------------------------------------------------------------------------
// AI command center: a live holographic DRC rendered from the collection itself (report
// hotspots as light columns, co-mention links as arcs, mineral sites as a point cloud,
// radio stations as pulses), framed by an intercept feed, province pressure and the Okapi bar.

const holoLayerList = () => state.holoLayers ??= (store.get('congo-holo-layers') || 'signals,links,minerals,radio').split(',').filter(Boolean);
const HOLO_LAYERS = () => [['signals', L('Signal columns', 'Colonnes de signaux'), '#6affb9'], ['links', L('Co-mention links', 'Liens de co-citation'), '#82e3f5'], ['minerals', L('Mineral sites', 'Sites miniers'), '#e0b252'], ['radio', L('Radio stations', 'Stations de radio'), '#c99bff']];
const frac = x => x - Math.floor(x);
const bandHex = s => s >= 70 ? '#ff5d6c' : s >= 50 ? '#ffc65c' : '#6affb9';
/// Places aggregated across loaded reports: count, peak priority, latest headline.
function holoHotspots() {
  const m = new Map();
  for (const a of state.items) for (const p of a.places || []) {
    if (!Number.isFinite(p.lon) || !Number.isFinite(p.lat)) continue;
    const h = m.get(p.name) || { name: p.name, province: p.province, lon: p.lon, lat: p.lat, count: 0, peak: 0, latest: null };
    h.count++; h.peak = Math.max(h.peak, a.rank?.score || 0);
    if (!h.latest || String(a.published_at) > String(h.latest.published_at)) h.latest = a;
    m.set(p.name, h);
  }
  return [...m.values()].sort((a, b) => b.count - a.count).slice(0, 48);
}
/// Pairs of places named together in priority reports (score ≥ 55), strongest first.
function holoLinks(spots) {
  const at = new Map(spots.map(s => [s.name, s])), m = new Map();
  for (const a of state.items) {
    if ((a.rank?.score || 0) < 55) continue;
    const names = [...new Set((a.places || []).map(p => p.name))].filter(n => at.has(n)).slice(0, 5);
    for (let i = 0; i < names.length; i++) for (let j = i + 1; j < names.length; j++) {
      const k = [names[i], names[j]].sort().join('|'); m.set(k, (m.get(k) || 0) + 1);
    }
  }
  return [...m].sort((a, b) => b[1] - a[1]).slice(0, 40).map(([k, n]) => { const [a, b] = k.split('|'); return { a: at.get(a), b: at.get(b), n } });
}
const holo = { canvas: null, raf: 0, t0: performance.now(), hover: null, spots: [], links: [], screen: [], size: [0, 0], dpr: 1, ro: null, sig: '' };
function holoData() {
  const sig = `${state.items.length}|${state.updated}|${state.geology?.features?.length}|${state.radio?.items?.length}`;
  if (holo.sig === sig) return;
  holo.sig = sig; holo.spots = holoHotspots(); holo.links = holoLinks(holo.spots);
  holo.minerals = (state.geology?.features || []).map(f => f.geometry.coordinates);
  holo.radio = (state.radio?.items || []).filter(s => s.location && s.health?.ok).map(s => [s.location.lon, s.location.lat]);
  // Densify the border so the travelling light moves smoothly.
  const ring = state.geometry?.features?.[0]?.geometry?.coordinates?.[0] || [];
  holo.border = ring.flatMap((p, i) => { const q = ring[(i + 1) % ring.length]; return [0, .25, .5, .75].map(t => [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]) });
}
function mountHolo() {
  const c = $('#holo');
  if (!c) { cancelAnimationFrame(holo.raf); holo.raf = 0; holo.canvas = null; holo.ro?.disconnect(); return }
  if (holo.canvas === c) return;
  holo.canvas = c; holoData();
  holo.ro?.disconnect(); holo.ro = new ResizeObserver(() => { holoResize(); if (!holoAnimated()) holoFrame(performance.now()) }); holo.ro.observe(c);
  c.onmousemove = e => { const r = c.getBoundingClientRect(); holoPick(e.clientX - r.left, e.clientY - r.top) };
  c.onmouseleave = () => { holo.hover = null; holoTip() };
  c.onclick = () => { const h = holo.hover; if (!h) return; state.place = h.name; state.radioMode = false; state.mineralMode = false; go('situation') };
  holoResize(); cancelAnimationFrame(holo.raf); holo.raf = requestAnimationFrame(holoLoop);
}
const holoAnimated = () => state.aiMotion && !reducedMotion();
function holoResize() {
  const c = holo.canvas; if (!c) return;
  const r = c.getBoundingClientRect(); holo.dpr = Math.min(2, devicePixelRatio || 1); holo.size = [r.width, r.height];
  c.width = Math.round(r.width * holo.dpr); c.height = Math.round(r.height * holo.dpr);
}
let holoLast = 0;
function holoLoop(now) {
  if (!holo.canvas?.isConnected) { holo.raf = 0; return }
  // ~40 fps is plenty for slow motion and halves the work on laptops.
  if (now - holoLast > 24) { holoLast = now; holoFrame(now) }
  holo.raf = holoAnimated() ? requestAnimationFrame(holoLoop) : 0;
}
function holoPick(x, y) {
  let best = null, bd = 22;
  for (const s of holo.screen) { const d = Math.hypot(s.x - x, s.y - y); if (d < bd) { bd = d; best = s } }
  if (best?.spot !== holo.hover) { holo.hover = best?.spot || null; holoTip(best); if (!holoAnimated()) holoFrame(performance.now()) }
  holo.canvas.style.cursor = best ? 'pointer' : '';
}
function holoTip(s) {
  const tip = $('#holo-tip'); if (!tip) return;
  if (!s) { tip.hidden = true; return }
  const h = s.spot, a = h.latest;
  tip.hidden = false; tip.style.left = `${s.x}px`; tip.style.top = `${s.y}px`;
  tip.innerHTML = `<small>${esc(h.province || '')}</small><strong>${esc(h.name)}</strong><span class="holo-tip-n"><b style="color:${bandHex(h.peak)}">${h.count}</b> ${L('reports', 'articles')} · ${L('peak', 'pic')} ${h.peak}</span>${a ? `<p>${esc(title(a))}</p><em>${esc(a.source || '')} · ${age(a.published_at)}</em>` : ''}<i>${L('Click to open in the situation room', 'Cliquez pour ouvrir dans la vue de situation')}</i>`;
}
function holoFrame(now) {
  const c = holo.canvas; if (!c) return;
  const ctx = c.getContext('2d'), [w, h] = holo.size, t = Math.max(0, now - holo.t0) / 1000;
  ctx.setTransform(holo.dpr, 0, 0, holo.dpr, 0, 0); ctx.clearRect(0, 0, w, h);
  // Fit the country into the free space between the floating panels, title and command bar:
  // project the border (and the tallest column) at both ends of the turntable swing, once per size.
  const wide = w > 1100, box = wide ? [22 + 330 + 24, 210, w - 22 - 290 - 24, h - 228] : [16, 16, w - 16, h - 16];
  const pitch = 0.9, D = 46, cp = Math.cos(pitch), sp = Math.sin(pitch), SWING = 0.38, BIAS = -0.06;
  const raw = (lon, lat, z, yw) => { const x = lon - 23.4, y = lat + 4.1, c = Math.cos(yw), s = Math.sin(yw), xr = x * c - y * s, yr = x * s + y * c, f = D / (D + yr * sp - z * cp); return [xr * f, -(yr * cp + z * sp) * f, f] };
  const fitKey = `${w}x${h}|${(holo.border || []).length}|${holo.spots.length}`;
  if (holo.fitKey !== fitKey && holo.border?.length) {
    const zTop = Math.max(0, ...holo.spots.map(s => 0.5 + Math.log2(s.count + 1) * 0.85));
    let x0 = Infinity, x1 = -Infinity, y0 = Infinity, y1 = -Infinity;
    for (const yw of [BIAS - SWING, BIAS, BIAS + SWING]) for (const [lon, lat] of holo.border) for (const z of [0, zTop * 0.6]) { const [x, y] = raw(lon, lat, z, yw); x0 = Math.min(x0, x); x1 = Math.max(x1, x); y0 = Math.min(y0, y); y1 = Math.max(y1, y) }
    const S = Math.min((box[2] - box[0]) / (x1 - x0), (box[3] - box[1]) / (y1 - y0));
    holo.fitKey = fitKey; holo.fit = { S, cx: (box[0] + box[2]) / 2 - (x0 + x1) / 2 * S, cy: (box[1] + box[3]) / 2 - (y0 + y1) / 2 * S };
  }
  const { S, cx, cy } = holo.fit || { S: Math.min(w, h) / 24, cx: w / 2, cy: h / 2 };
  const yaw = Math.sin(t * 0.09) * SWING + BIAS;
  const cy0 = Math.cos(yaw), sy0 = Math.sin(yaw);
  // World: degrees about the DRC's centre; z up (degrees of "height").
  const P = (lon, lat, z = 0) => {
    const x = lon - 23.4, y = lat + 4.1, xr = x * cy0 - y * sy0, yr = x * sy0 + y * cy0;
    const f = D / (D + yr * sp - z * cp);
    return [cx + xr * f * S, cy - (yr * cp + z * sp) * f * S, f];
  };
  const on = k => holoLayerList().includes(k);
  ctx.lineCap = 'round'; ctx.lineJoin = 'round';
  // Ground grid every 2°, fading with distance.
  ctx.lineWidth = 1;
  for (let lon = 12; lon <= 32; lon += 2) { const a = P(lon, -14), b = P(lon, 6); ctx.strokeStyle = 'rgba(106,255,185,.055)'; ctx.beginPath(); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.stroke() }
  for (let lat = -14; lat <= 6; lat += 2) { const a = P(12, lat), b = P(32, lat); ctx.strokeStyle = 'rgba(106,255,185,.055)'; ctx.beginPath(); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.stroke() }
  // Radar sweep around the centre of the country.
  const sweep = t * 0.55, R = 13;
  const o = P(23.4, -4.1);
  for (let i = 0; i < 26; i++) {
    const a0 = sweep - i * 0.035, a1 = a0 - 0.035, p0 = P(23.4 + Math.cos(a0) * R, -4.1 + Math.sin(a0) * R), p1 = P(23.4 + Math.cos(a1) * R, -4.1 + Math.sin(a1) * R);
    ctx.fillStyle = `rgba(106,255,185,${0.07 * (1 - i / 26)})`; ctx.beginPath(); ctx.moveTo(o[0], o[1]); ctx.lineTo(p0[0], p0[1]); ctx.lineTo(p1[0], p1[1]); ctx.closePath(); ctx.fill();
  }
  for (const rr of [4, 8, 12]) { ctx.strokeStyle = 'rgba(106,255,185,.07)'; ctx.setLineDash([2, 6]); ctx.beginPath(); for (let i = 0; i <= 64; i++) { const a = i / 64 * Math.PI * 2, p = P(23.4 + Math.cos(a) * rr, -4.1 + Math.sin(a) * rr); i ? ctx.lineTo(p[0], p[1]) : ctx.moveTo(p[0], p[1]) } ctx.stroke(); ctx.setLineDash([]) }
  const swept = (lon, lat) => { const a = Math.atan2(lat + 4.1, lon - 23.4); let d = (sweep - a) % (Math.PI * 2); if (d < 0) d += Math.PI * 2; return d < 0.9 ? 1 - d / 0.9 : 0 };
  // Country slab: underside, walls, top outline with glow and a travelling light.
  const B = holo.border || [];
  if (B.length) {
    const path = z => { ctx.beginPath(); B.forEach((p, i) => { const q = P(p[0], p[1], z); i ? ctx.lineTo(q[0], q[1]) : ctx.moveTo(q[0], q[1]) }); ctx.closePath() };
    path(-0.7); ctx.strokeStyle = 'rgba(106,255,185,.12)'; ctx.lineWidth = 1; ctx.stroke();
    ctx.strokeStyle = 'rgba(106,255,185,.06)';
    for (let i = 0; i < B.length; i += 6) { const a = P(B[i][0], B[i][1], 0), b = P(B[i][0], B[i][1], -0.7); ctx.beginPath(); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.stroke() }
    path(0); const g = ctx.createLinearGradient(0, cy - 8 * S, 0, cy + 8 * S); g.addColorStop(0, 'rgba(106,255,185,.10)'); g.addColorStop(1, 'rgba(130,227,245,.025)'); ctx.fillStyle = g; ctx.fill();
    ctx.shadowColor = 'rgba(106,255,185,.8)'; ctx.shadowBlur = 14; ctx.strokeStyle = 'rgba(150,255,205,.75)'; ctx.lineWidth = 1.4; ctx.stroke(); ctx.shadowBlur = 0;
    for (const k of [0, 0.5]) {
      const head = Math.floor(frac(t * 0.06 + k) * B.length);
      for (let i = 0; i < 40; i++) { const a = B[(head - i + B.length) % B.length], b = B[(head - i - 1 + B.length) % B.length], p = P(a[0], a[1]), q = P(b[0], b[1]); ctx.strokeStyle = `rgba(210,255,235,${0.9 * (1 - i / 40)})`; ctx.lineWidth = 2.6 * (1 - i / 40) + 0.6; ctx.beginPath(); ctx.moveTo(p[0], p[1]); ctx.lineTo(q[0], q[1]); ctx.stroke() }
    }
  }
  // Mineral point cloud, brightened as the sweep passes.
  if (on('minerals')) for (const [lon, lat] of holo.minerals || []) {
    const p = P(lon, lat), k = swept(lon, lat);
    ctx.fillStyle = k ? `rgba(255,214,120,${0.3 + 0.6 * k})` : 'rgba(224,178,82,.28)';
    const s = (k ? 2.2 : 1.3) * p[2]; ctx.fillRect(p[0] - s / 2, p[1] - s / 2, s, s);
  }
  // Radio stations: expanding rings.
  if (on('radio')) for (const [lon, lat] of holo.radio || []) {
    const p = P(lon, lat);
    for (const k of [0, 0.5]) { const ph = frac(t * 0.5 + k); ctx.strokeStyle = `rgba(201,155,255,${0.7 * (1 - ph)})`; ctx.lineWidth = 1.2; ctx.beginPath(); ctx.ellipse(p[0], p[1], 4 + ph * 22, (4 + ph * 22) * cp, 0, 0, Math.PI * 2); ctx.stroke() }
    ctx.fillStyle = '#d9bcff'; ctx.beginPath(); ctx.arc(p[0], p[1], 2.4, 0, Math.PI * 2); ctx.fill();
  }
  const height = s => 0.5 + Math.log2(s.count + 1) * 0.85;
  // Co-mention arcs with travelling sparks.
  if (on('links')) holo.links.forEach((l, i) => {
    const d = Math.hypot(l.a.lon - l.b.lon, l.a.lat - l.b.lat), hz = Math.max(0.6, d * 0.32) + height(l.a) * 0.3;
    const pt = u => P(l.a.lon + (l.b.lon - l.a.lon) * u, l.a.lat + (l.b.lat - l.a.lat) * u, Math.sin(Math.PI * u) * hz);
    const hot = holo.hover && (holo.hover === l.a || holo.hover === l.b);
    ctx.strokeStyle = hot ? 'rgba(130,227,245,.85)' : `rgba(130,227,245,${0.14 + Math.min(0.3, l.n * 0.05)})`; ctx.lineWidth = hot ? 1.6 : 1;
    ctx.beginPath(); for (let k = 0; k <= 24; k++) { const p = pt(k / 24); k ? ctx.lineTo(p[0], p[1]) : ctx.moveTo(p[0], p[1]) } ctx.stroke();
    const u = frac(t * 0.22 + i * 0.137), p = pt(u);
    ctx.fillStyle = 'rgba(200,245,255,.95)'; ctx.shadowColor = '#82e3f5'; ctx.shadowBlur = 10; ctx.beginPath(); ctx.arc(p[0], p[1], 1.8, 0, Math.PI * 2); ctx.fill(); ctx.shadowBlur = 0;
  });
  // Signal columns, far to near so near ones draw on top.
  holo.screen = [];
  if (on('signals')) {
    const spots = holo.spots.map(s => ({ s, base: P(s.lon, s.lat), top: P(s.lon, s.lat, height(s)) })).sort((a, b) => a.base[2] - b.base[2]);
    for (const { s, base, top } of spots) {
      const col = bandHex(s.peak), hov = holo.hover === s, pulse = 0.5 + 0.5 * Math.sin(t * 2.2 + s.lon);
      const rr = (3 + Math.log2(s.count + 1) * 2.2) * base[2];
      ctx.strokeStyle = col + (hov ? 'ff' : '66'); ctx.lineWidth = 1; ctx.beginPath(); ctx.ellipse(base[0], base[1], rr, rr * cp, 0, 0, Math.PI * 2); ctx.stroke();
      if (s.peak >= 70) { const ph = frac(t * 0.7 + s.lat); ctx.strokeStyle = `rgba(255,93,108,${0.6 * (1 - ph)})`; ctx.beginPath(); ctx.ellipse(base[0], base[1], rr + ph * 26, (rr + ph * 26) * cp, 0, 0, Math.PI * 2); ctx.stroke() }
      const g = ctx.createLinearGradient(base[0], base[1], top[0], top[1]); g.addColorStop(0, col + '10'); g.addColorStop(1, col + (hov ? 'ff' : 'dd'));
      ctx.strokeStyle = g; ctx.lineWidth = (hov ? 4 : 2.4) * base[2]; ctx.beginPath(); ctx.moveTo(base[0], base[1]); ctx.lineTo(top[0], top[1]); ctx.stroke();
      ctx.shadowColor = col; ctx.shadowBlur = hov ? 22 : 12 + pulse * 6; ctx.fillStyle = hov ? '#fff' : col; ctx.beginPath(); ctx.arc(top[0], top[1], (hov ? 4 : 2.6) * base[2], 0, Math.PI * 2); ctx.fill(); ctx.shadowBlur = 0;
      holo.screen.push({ x: top[0], y: top[1], spot: s });
    }
    // Labels for the strongest places and the hovered one.
    ctx.font = '500 10.5px "IBM Plex Mono", ui-monospace, monospace'; ctx.textBaseline = 'middle';
    // Strongest places first; a label is skipped when it would overlap one already placed.
    const placed = [], order = [...holo.screen].sort((a, b) => (b.spot === holo.hover) - (a.spot === holo.hover) || b.spot.count - a.spot.count).slice(0, 12);
    for (const { x, y, spot } of order) {
      const text = `${spot.name.toUpperCase()}  ${spot.count}`, tw = ctx.measureText(text).width, right = x < cx + 6 * S, lx = right ? x + 18 : x - 18 - tw - 10, ly = y - 14;
      const box = [lx - 4, ly - 18, tw + 16, 18];
      if (placed.some(b => box[0] < b[0] + b[2] && b[0] < box[0] + box[2] && box[1] < b[1] + b[3] && b[1] < box[1] + box[3])) continue;
      placed.push(box);
      ctx.strokeStyle = 'rgba(160,255,210,.45)'; ctx.lineWidth = 1; ctx.beginPath(); ctx.moveTo(x + (right ? 3 : -3), y - 2); ctx.lineTo(right ? lx - 4 : lx + tw + 10, ly); ctx.lineTo(right ? lx + tw + 10 : lx - 4, ly); ctx.stroke();
      ctx.fillStyle = 'rgba(3,12,10,.72)'; ctx.fillRect(lx - 2, ly - 16, tw + 12, 14);
      ctx.fillStyle = spot === holo.hover ? '#ffffff' : '#c9ffe4'; ctx.fillText(text, lx + 3, ly - 9);
    }
  }
  // Compass readout fixed to the projection.
  const n = P(23.4, 7.2); ctx.fillStyle = 'rgba(160,255,210,.55)'; ctx.font = '500 10px "IBM Plex Mono", monospace'; ctx.textAlign = 'center'; ctx.fillText('N', n[0], n[1]); ctx.textAlign = 'left';
}
function aiCommandView() {
  holoData();
  const items = state.items, now = Date.now();
  const recent = [...items].sort((a, b) => String(b.published_at).localeCompare(String(a.published_at))).slice(0, 14);
  const day = 864e5, within = (a, d) => now - Date.parse(a.published_at) < d * day;
  const high24 = items.filter(a => within(a, 1) && a.rank?.band === 'high').length;
  const days = Array.from({ length: 14 }, (_, i) => kinDay(new Date(now - (13 - i) * day).toISOString()));
  const perDay = (list, field) => { const m = new Map(); for (const x of list) { const k = kinDay(x[field]); m.set(k, (m.get(k) || 0) + 1) } return days.map(d => m.get(d) || 0) };
  const reportDays = perDay(items, 'published_at'), postDays = perDay(state.posts?.items || [], 'created_at');
  const provinces = [...items.filter(a => within(a, 7)).reduce((m, a) => { for (const pv of new Set((a.places || []).map(p => p.province).filter(Boolean))) { const v = m.get(pv) || { n: 0, score: 0 }; v.n++; v.score += a.rank?.score || 0; m.set(pv, v) } return m }, new Map())].sort((a, b) => b[1].n - a[1].n).slice(0, 8);
  const pmax = Math.max(1, ...provinces.map(p => p[1].n));
  const radioLive = (state.radio?.items || []).filter(s => s.health?.ok).length;
  const metrics = [
    [L('Reports', 'Articles'), state.total, spark(reportDays, '#6affb9', 120, 30), 'nav', 'reports', L('14 days', '14 jours')],
    [L('Social posts', 'Publications'), state.social?.archive?.posts ?? state.posts?.items?.length ?? 0, spark(postDays, '#82e3f5', 120, 30), 'nav', 'social', L('14 days', '14 jours')],
    [L('Mineral sites', 'Sites miniers'), state.geology?.features?.length ?? 0, '', 'map-mode', 'minerals', 'IPIS · USGS'],
    [L('Radio on air', 'Radios à l’antenne'), radioLive, '', 'map-mode', 'radio', `${state.radio?.items?.length || 0} ${L('listed', 'répertoriées')}`],
    [L('Agreements', 'Accords'), state.agreements?.items?.length ?? 0, '', 'nav', 'agreements', L('register', 'registre')],
  ];
  const prompts = [L('What changed in the last 24 hours?', 'Qu’est-ce qui a changé en 24 heures ?'), L('Show me the mineral sites', 'Montre-moi les sites miniers'), L('Summarize social intelligence', 'Résume la veille sociale'), L('Where is fighting reported?', 'Où signale-t-on des combats ?')];
  const corner = '<i class="hc tl"></i><i class="hc tr"></i><i class="hc bl"></i><i class="hc br"></i>';
  return `<div class="holo-stage">
  <canvas id="holo" class="holo-canvas" role="img" aria-label="${L('Holographic map of the DRC built from the collection: report hotspots, co-mention links, mineral sites and radio stations.', 'Carte holographique de la RDC construite à partir de la collection : points chauds, liens de co-citation, sites miniers et radios.')}"></canvas>
  <div class="holo-vignette" aria-hidden="true"></div><div class="holo-scan" aria-hidden="true"></div>
  <div class="holo-tip" id="holo-tip" hidden></div>
  <header class="holo-title"><div class="holo-kicker"><i class="dot ok live"></i>OKAPI · ${L('INTELLIGENCE SURFACE', 'SURFACE DE RENSEIGNEMENT')}</div>
    <h1><span class="holo-num">${nf(items.length)}</span> ${L('reports', 'articles')} <span class="holo-sep">/</span> <span class="holo-num">${holo.spots.length}</span> ${L('places', 'lieux')} <span class="holo-sep">/</span> <span class="holo-num ${high24 ? 'hot' : ''}">${high24}</span> ${L('high-priority today', 'prioritaires aujourd’hui')}</h1>
    <p>${L('Every light on this surface is a source in your collection. Hover a column to read it; click to open the place.', 'Chaque lumière correspond à une source de votre collection. Survolez une colonne pour la lire ; cliquez pour ouvrir le lieu.')}</p></header>
  <aside class="holo-panel holo-feed">${corner}<div class="holo-panel-head"><span>${L('INTERCEPT FEED', 'FLUX D’INTERCEPTION')}</span><b>${L('LATEST', 'DERNIERS')}</b></div>
    ${recent.map((a, i) => `<button class="holo-intercept" data-act="report" data-arg="${esc(a.id)}" style="--i:${i}"><time>${esc(age(a.published_at))}</time><i style="background:${bandHex(a.rank?.score || 0)}"></i><div><strong>${esc(title(a))}</strong><small>${esc(a.source || '')}${a.places?.[0] ? ' · ' + esc(a.places[0].name) : ''}</small></div></button>`).join('') || `<p class="muted">${L('No reports loaded.', 'Aucun article chargé.')}</p>`}</aside>
  <aside class="holo-panel holo-side">${corner}<div class="holo-panel-head"><span>${L('PROVINCE PRESSURE', 'PRESSION PAR PROVINCE')}</span><b>7 ${L('DAYS', 'JOURS')}</b></div>
    ${provinces.map(([pv, v]) => `<button class="holo-bar" data-act="holo-province" data-arg="${esc(pv)}"><span>${esc(pv)}</span><i><em style="width:${(v.n / pmax * 100).toFixed(0)}%;background:${bandHex(v.score / v.n)}"></em></i><b>${v.n}</b></button>`).join('') || `<p class="muted">${L('No placed reports this week.', 'Aucun article localisé cette semaine.')}</p>`}
    <div class="holo-panel-head" style="margin-top:18px"><span>${L('LAYERS', 'COUCHES')}</span><b>${L('HOLOGRAM', 'HOLOGRAMME')}</b></div>
    ${HOLO_LAYERS().map(([k, label, col]) => `<label class="holo-layer"><input type="checkbox" data-act="holo-layer" data-arg="${k}" ${holoLayerList().includes(k) ? 'checked' : ''}><i style="background:${col};box-shadow:0 0 10px ${col}"></i>${esc(label)}</label>`).join('')}
    <div class="holo-legend"><span><i style="background:#ff5d6c"></i>${L('High', 'Élevée')}</span><span><i style="background:#ffc65c"></i>${L('Medium', 'Moyenne')}</span><span><i style="background:#6affb9"></i>${L('Routine', 'Courante')}</span></div>
    <button class="holo-motion" data-act="ai-motion" aria-pressed="${!state.aiMotion}">${state.aiMotion ? '❚❚ ' + L('Pause motion', 'Suspendre') : '▶ ' + L('Resume motion', 'Reprendre')}</button></aside>
  <form class="holo-command" data-form="okapi-ask">${corner}<span class="holo-orb" aria-hidden="true"><i></i><i></i><i></i></span><label class="sr-only" for="okapi-q">${L('Ask Okapi', 'Demander à Okapi')}</label><input id="okapi-q" name="q" autocomplete="off" maxlength="1000" placeholder="${L('Ask Okapi about the DRC…', 'Interrogez Okapi sur la RDC…')}"><button type="submit" class="holo-send">${L('Ask', 'Demander')} ${icon('arrow')}</button><button type="button" class="holo-mic" data-act="voice-listen" aria-label="${L('Voice mode: talk to Okapi', 'Mode vocal : parlez à Okapi')}" title="${L('Voice mode: talk to Okapi', 'Mode vocal : parlez à Okapi')}">${icon('mic')}<span>${L('Voice', 'Voix')}</span></button>
    <div class="holo-prompts">${prompts.map(q => `<button type="button" data-act="ai-prompt" data-arg="${esc(q)}">${esc(q)}</button>`).join('')}</div></form>
  <div class="holo-metrics">${metrics.map(([label, n, chart, act, arg, sub]) => `<button data-act="${act}" data-arg="${arg}">${corner}<small>${esc(label)}</small><strong>${nf(n)}</strong><span class="holo-metric-sub">${esc(sub)}</span>${chart}</button>`).join('')}</div>
  </div>`;
}
// ---------------------------------------------------------------------------------------------
// Live radio: stations on the map, a persistent player, and news from the station's area.

state.radioMode = location.hash === '#radio' || location.hash.startsWith('#radio/'); state.radio = null; state.radioNow = ''; state.radioSel = ''; state.radioStatus = 'idle';
state.radioQ = ''; state.radioLang = ''; state.radioLiveOnly = true; state.radioPlace = '';
const radioAudio = new Audio();
radioAudio.preload = 'none';
radioAudio.hidden = true; document.body.append(radioAudio); // outside #app, so re-renders never touch it
radioAudio.volume = Number(store.get('congo-radio-volume') || 0.8);
radioAudio.addEventListener('playing', () => { state.radioStatus = 'playing'; refreshRadioChrome() });
radioAudio.addEventListener('waiting', () => { state.radioStatus = 'loading'; refreshRadioChrome() });
radioAudio.addEventListener('pause', () => { if (state.radioStatus !== 'error') state.radioStatus = 'idle'; refreshRadioChrome() });
radioAudio.addEventListener('error', () => {
  if (!state.radioNow) return;
  const code = radioAudio.error?.code;
  console.warn('radio stream error', radioAudio.currentSrc, code, radioAudio.error?.message);
  state.radioStatus = 'error'; refreshRadioChrome();
  toast(code === 4 ? L('Your browser cannot decode this stream’s format. Try another station or browser.', 'Le navigateur ne sait pas lire le format de ce flux. Essayez une autre station ou un autre navigateur.')
    : L('This stream is not responding right now. Try another station.', 'Ce flux ne répond pas pour le moment. Essayez une autre station.'));
});
const stationById = id => state.radio?.items?.find(s => s.id === id);
const stationLive = s => s?.health?.ok === true;
const stationPlace = s => s?.location?.place ? `${s.location.place}${s.location.province && s.location.province !== s.location.place ? ' · ' + s.location.province : ''}` : L('National & online', 'National et en ligne');
function radioStations() {
  const q = norm(state.radioQ.trim());
  return (state.radio?.items || []).filter(s => (!state.radioLiveOnly || stationLive(s)) && (!state.radioLang || s.languages?.includes(state.radioLang))
    && (!state.radioPlace || (s.location?.place || '') === state.radioPlace)
    && (!q || norm([s.name, s.city, s.location?.place, ...(s.tags || []), ...(s.languages || [])].join(' ')).includes(q)));
}
async function playStation(id) {
  const s = stationById(id); if (!s) return;
  if (state.radioNow === id && !radioAudio.paused) { radioAudio.pause(); state.radioStatus = 'idle'; render(); return }
  // Start playback first, synchronously inside the click: some browsers (Safari) only honour
  // play() while the user gesture is fresh, and a full re-render beforehand can use that up.
  radioAudio.src = s.stream;
  const started = radioAudio.play();
  state.radioNow = id; state.radioSel = id; state.radioStatus = 'loading'; radioHash(id);
  render();
  try { await started } catch (err) {
    if (state.radioNow !== id || err?.name === 'AbortError') return; // superseded by another station
    console.warn('radio playback failed', s.stream, err);
    state.radioStatus = 'error'; refreshRadioChrome();
    toast(err?.name === 'NotAllowedError'
      ? L('Your browser blocked audio playback. Allow audio for this site (or turn off auto-play blocking) and press play again.', 'Le navigateur a bloqué la lecture audio. Autorisez le son pour ce site puis relancez la lecture.')
      : L(`This stream could not be played (${err?.name || 'error'}). Try another station.`, `Ce flux n’a pas pu être lu (${err?.name || 'erreur'}). Essayez une autre station.`));
    return;
  }
  api(`/v1/radio/${encodeURIComponent(id)}/listen`, {}).catch(() => {});
}
const radioHash = id => { if (state.radioMode && state.view === 'situation') history.replaceState(null, '', '#radio/' + encodeURIComponent(id)) };
function stopRadio() { state.radioNow = ''; state.radioStatus = 'idle'; radioAudio.pause(); radioAudio.removeAttribute('src'); radioAudio.load(); render() }
const eqBars = (on, n = 4) => `<span class="eq ${on ? 'on' : ''}" aria-hidden="true">${'<i></i>'.repeat(n)}</span>`;
/// Topbar "on air" chip, visible on every view while a station plays.
function onAirChip() {
  const s = stationById(state.radioNow); if (!s) return '';
  const label = state.radioStatus === 'playing' ? L('ON AIR', 'EN DIRECT') : state.radioStatus === 'loading' ? L('TUNING', 'RÉGLAGE') : state.radioStatus === 'error' ? L('NO SIGNAL', 'PAS DE SIGNAL') : L('PAUSED', 'EN PAUSE');
  return `<div class="onair ${state.radioStatus}" id="onair"><button class="onair-main" data-act="radio-open" title="${esc(s.name)}">${eqBars(state.radioStatus === 'playing')}<span class="onair-label">${label}</span><strong>${esc(s.name)}</strong><small>${esc(s.location?.place || '')}</small></button><button class="onair-btn" data-act="radio-toggle" aria-label="${state.radioStatus === 'playing' ? L('Pause', 'Pause') : L('Play', 'Lecture')}">${state.radioStatus === 'playing' ? '❚❚' : '▶'}</button><button class="onair-btn" data-act="radio-stop" aria-label="${L('Stop', 'Arrêter')}">■</button></div>`;
}
/// Update player chrome without a full re-render (audio events fire often).
function refreshRadioChrome() {
  const chip = $('#onair'); if (chip) chip.outerHTML = onAirChip();
  const np = $('#now-playing'); if (np && state.radioSel) np.outerHTML = nowPlaying(stationById(state.radioSel));
  $$('[data-station]').forEach(el => { const on = el.dataset.station === state.radioNow; el.classList.toggle('playing', on && state.radioStatus === 'playing'); el.classList.toggle('tuning', on && state.radioStatus === 'loading'); el.querySelector('.eq')?.classList.toggle('on', on && state.radioStatus === 'playing') });
  radioMarkers.forEach(m => m.getElement().classList.toggle('playing', m.__place === stationById(state.radioNow)?.location?.place && state.radioStatus === 'playing'));
}
function newsNear(lon, lat, km = 60) {
  return state.items.map(a => {
    const d = a.places.filter(p => Number.isFinite(p.lon)).map(p => ({ p, km: distanceKm([lon, lat], [p.lon, p.lat]) })).sort((x, y) => x.km - y.km)[0];
    return d && d.km <= km ? { a, ...d } : null;
  }).filter(Boolean).sort((x, y) => String(y.a.published_at).localeCompare(String(x.a.published_at)));
}
function nowPlaying(s) {
  if (!s) return `<div class="now-playing empty" id="now-playing"><div class="eyebrow">${L('LIVE RADIO', 'RADIO EN DIRECT')}</div><h2 class="mine-name">${L('Choose a station', 'Choisissez une station')}</h2><p class="muted">${L('Pick a station on the map, in the list or on the dial. Radio Okapi is the national news network.', 'Choisissez une station sur la carte, dans la liste ou sur le cadran. Radio Okapi est le réseau national d’information.')}</p>${stationById('radio-okapi') ? `<button class="btn primary" data-act="radio-play" data-arg="radio-okapi">▶ ${L('Listen to Radio Okapi', 'Écouter Radio Okapi')}</button>` : ''}</div>`;
  const on = state.radioNow === s.id, playing = on && state.radioStatus === 'playing', h = s.health || {};
  return `<div class="now-playing ${playing ? 'playing' : ''}" id="now-playing">
    <div class="np-head">${s.favicon ? `<img class="np-logo" src="${esc(s.favicon)}" alt="" referrerpolicy="no-referrer" loading="lazy" onerror="this.remove()">` : `<span class="np-logo ph">${icon('radio')}</span>`}<div style="min-width:0"><div class="eyebrow">${stationLive(s) ? `<i class="dot ok live"></i>${L('LIVE STREAM', 'FLUX EN DIRECT')}` : `<i class="dot bad"></i>${L('OFF AIR', 'HORS ANTENNE')}`}</div><h2 class="mine-name">${esc(s.name)}</h2><div class="mine-place">${icon('pin')} ${esc(stationPlace(s))}</div></div></div>
    <div class="np-visual">${eqBars(playing, 24)}</div>
    <div class="np-controls"><button class="np-play ${playing ? 'on' : ''}" data-act="radio-play" data-arg="${esc(s.id)}" aria-label="${playing ? L('Pause', 'Pause') : L('Play', 'Lecture')}" ${stationLive(s) || on ? '' : 'disabled'}>${playing ? '❚❚' : on && state.radioStatus === 'loading' ? '…' : '▶'}</button>
      <label class="np-volume">${icon('radio')}<input type="range" min="0" max="1" step="0.05" value="${radioAudio.volume}" data-act="radio-volume" aria-label="${L('Volume', 'Volume')}"></label>${on ? `<button class="btn sm ghost" data-act="radio-stop">■ ${L('Stop', 'Arrêter')}</button>` : ''}</div>
    <div class="chips" style="margin:12px 0">${(s.languages || []).map(l => `<span class="chip">${esc(l)}</span>`).join('')}${(s.tags || []).slice(0, 5).map(t => `<span class="badge">${esc(t)}</span>`).join('')}</div>
    <dl class="props np-props">${factRow(L('Coverage', 'Couverture'), esc(s.coverage || ''))}${factRow(L('Stream', 'Flux'), `${esc(h.content_type || s.codec || '—')}${h.icy_bitrate || s.bitrate ? ` · ${esc(h.icy_bitrate || s.bitrate)} kbps` : ''}${h.latency_ms ? ` · ${h.latency_ms} ms` : ''}`)}${factRow(L('Last checked', 'Dernière vérification'), s.checked_at ? `${age(s.checked_at)}${h.error ? ` · <span class="ev-error">${esc(h.error)}</span>` : ''}` : '—')}${factRow(L('Source', 'Source'), s.source === 'curated' ? esc(s.source_note || L('Official station stream', 'Flux officiel de la station')) : 'Radio Browser')}${factRow(L('Website', 'Site web'), s.homepage ? `<a href="${esc(s.homepage)}" target="_blank" rel="noopener noreferrer">${esc(s.homepage.replace(/^https?:\/\//, '').slice(0, 40))} ↗</a>` : '')}</dl>
    ${s.stream.startsWith('http:') && location.protocol === 'https:' ? `<p class="notice">${L('This station only offers an unencrypted (http) stream, which browsers block on secure pages.', 'Cette station ne propose qu’un flux non chiffré (http), bloqué par les navigateurs sur les pages sécurisées.')}</p>` : ''}</div>`;
}
function radioSituation() {
  const all = state.radio?.items || [], list = radioStations(), live = all.filter(stationLive);
  const sel = stationById(state.radioSel);
  const groups = [...list.reduce((m, s) => { const k = stationPlace(s); return m.set(k, [...(m.get(k) || []), s]) }, new Map())].sort((a, b) => (a[0].startsWith(L('National', 'National')) ? 1 : 0) - (b[0].startsWith(L('National', 'National')) ? 1 : 0) || b[1].length - a[1].length);
  const langs = [...new Set(all.flatMap(s => s.languages || []))].sort();
  const near = sel?.location ? newsNear(sel.location.lon, sel.location.lat) : [];
  const nearbyStations = sel?.location ? all.filter(s => s.id !== sel.id && s.location && distanceKm([sel.location.lon, sel.location.lat], [s.location.lon, s.location.lat]) <= 150) : [];
  const row = s => `<button class="station ${state.radioSel === s.id ? 'selected' : ''} ${state.radioNow === s.id && state.radioStatus === 'playing' ? 'playing' : ''}" data-station="${esc(s.id)}" data-act="radio-select" data-arg="${esc(s.id)}">${eqBars(state.radioNow === s.id && state.radioStatus === 'playing', 3)}<div style="min-width:0"><strong>${esc(s.name)}</strong><small>${esc((s.languages || []).slice(0, 3).join(' · ') || (s.tags || []).slice(0, 2).join(' · '))}</small></div><i class="dot ${stationLive(s) ? 'ok' : 'bad'}" title="${stationLive(s) ? L('Live', 'En direct') : L('Off air', 'Hors antenne')}"></i></button>`;
  return `<div class="situation radio-mode ${state.mapFocus ? 'map-focus' : ''}">
    <aside class="filters-pane" data-scroll="radio-list">
      <div class="pane-section"><div class="eyebrow">${L('DRC', 'RDC')} · ${L('LIVE RADIO', 'RADIO EN DIRECT')}</div><div style="font-size:13px;color:var(--text-2)"><b class="num" style="color:var(--text)">${live.length}</b> ${L('stations on air', 'stations à l’antenne')} · ${all.length} ${L('listed', 'répertoriées')}</div>
        <label class="search">${icon('search')}<input id="radio-q" class="input" type="search" data-model="radioQ" value="${esc(state.radioQ)}" placeholder="${L('Station, city, language…', 'Station, ville, langue…')}" aria-label="${L('Search stations', 'Rechercher des stations')}"></label>
        <select class="select" data-model="radioLang" aria-label="${L('Language', 'Langue')}"><option value="">${L('All languages', 'Toutes les langues')}</option>${langs.map(l => `<option value="${esc(l)}" ${state.radioLang === l ? 'selected' : ''}>${esc(l)}</option>`).join('')}</select>
        <label class="check"><input type="checkbox" data-model="radioLiveOnly" ${state.radioLiveOnly ? 'checked' : ''}>${L('Live streams only', 'Flux en direct uniquement')}</label>
        ${state.radioPlace ? `<div class="chips"><button class="chip active" data-act="radio-place" data-arg="">${icon('pin')}${esc(state.radioPlace)} ×</button></div>` : ''}</div>
      ${groups.map(([place, ss]) => `<div class="pane-section station-group"><h2>${esc(place)}<span>${ss.length}</span></h2>${ss.map(row).join('')}</div>`).join('') || `<div class="pane-section"><p class="muted">${state.radio ? L('No stations match.', 'Aucune station ne correspond.') : L('Station directory is syncing.', 'Synchronisation du répertoire.')}</p></div>`}
    </aside>
    <section class="stage"><div class="map-area">${mapSVG([])}
      <div class="hud" aria-hidden="true"><i class="hud-c tl"></i><i class="hud-c tr"></i><i class="hud-c bl"></i><i class="hud-c br"></i></div>
      <div class="map-float hud-pos glass hud-readout" id="hud-coords" aria-hidden="true">${hudText(state.hud)}</div>
      <div class="map-float tl glass"><strong>${icon('radio')} ${L('Live radio', 'Radio en direct')}</strong>${live.length} ${L('stations on air', 'à l’antenne')} · ${new Set(live.map(s => s.location?.place).filter(Boolean)).size} ${L('cities on the map', 'villes sur la carte')}</div>
      <div class="map-float tr"><div class="seg glass mode-switch" style="padding:2px"><button data-act="map-mode" data-arg="news">${L('News', 'Actualité')}</button><button data-act="map-mode" data-arg="minerals">${L('Minerals', 'Minerais')}</button><button class="active" data-act="map-mode" data-arg="radio">${icon('radio')}${L('Radio', 'Radio')}</button><button data-act="map-mode" data-arg="flights">✈ ${L('Flights','Vols')}</button></div><button class="btn sm glass" data-act="map-focus" aria-pressed="${state.mapFocus}">${state.mapFocus ? L('Show panels', 'Afficher les panneaux') : L('Expand map', 'Agrandir')}</button><button class="btn sm glass" data-act="reset-map">${icon('globe')}${L('Reset', 'Recentrer')}</button></div>
      <div class="map-float bl glass">${L('Streams play directly from the broadcasters. Locations are the station’s listed city, not transmitter coverage.', 'Les flux sont lus directement depuis les diffuseurs. La position est la ville déclarée, pas la zone de couverture.')}</div>
    </div><div class="timeline dial"><div class="timeline-head"><strong>${L('Dial', 'Cadran')}</strong><span>${L('Live stations · click to tune', 'Stations en direct · cliquez pour écouter')}</span></div><div class="tickers dial-row">${live.map(s => `<button class="ticker dial-station ${state.radioNow === s.id ? 'active' : ''}" data-station="${esc(s.id)}" data-act="radio-play" data-arg="${esc(s.id)}"><span class="t-name">${esc(s.location?.place || L('Online', 'En ligne'))}</span><span class="t-val"><b>${esc(s.name.length > 22 ? s.name.slice(0, 21) + '…' : s.name)}</b></span>${eqBars(state.radioNow === s.id && state.radioStatus === 'playing', 12)}</button>`).join('')}</div></div></section>
    <aside class="rail-pane"><div class="mine-dossier" data-scroll="np">${nowPlaying(sel)}
      ${sel?.location ? `<section class="panel market-card"><div class="panel-head"><h2>${L('News from', 'Actualités de')} ${esc(sel.location.place || '')}</h2><small>${L('within 60 km', 'dans un rayon de 60 km')}</small></div>${near.slice(0, 8).map(({ a, p, km }) => `<button class="mineral-result" data-act="report" data-arg="${esc(a.id)}"><strong>${esc(title(a))}</strong><small>${esc(p.name)} · ${km.toFixed(0)} km · ${age(a.published_at)} · ${esc(a.source || '')}</small></button>`).join('') || `<div class="panel-body muted">${L('No loaded reports mention this area.', 'Aucun article chargé ne cite cette zone.')}</div>`}</section>` : ''}
      ${nearbyStations.length ? `<section class="panel market-card"><div class="panel-head"><h2>${L('Other stations nearby', 'Autres stations proches')}</h2></div><div class="panel-body">${nearbyStations.map(row).join('')}</div></section>` : ''}
      <p class="notice">${esc(srv(state.radio?.notice))}</p></div></aside>
  </div>`;
}
let radioMarkers = [];
function clearRadioMarkers() { radioMarkers.forEach(m => m.remove()); radioMarkers = []; if (mapInstance) mapInstance.__radioSignature = '' }
/// One marker per station location; playing location pulses. Mine layers are hidden in this mode.
function mountRadio() {
  const map = mapInstance; if (!map) return;
  const on = state.radioMode && state.view === 'situation';
  try { for (const id of ['mineral-clusters', 'mineral-counts', 'mineral-sites', 'mineral-selection']) if (map.getLayer(id)) map.setLayoutProperty(id, 'visibility', on ? 'none' : 'visible') } catch {}
  if (!on) { if (radioMarkers.length) clearRadioMarkers(); return }
  const places = [...radioStations().filter(s => s.location).reduce((m, s) => { const k = s.location.place || `${s.location.lat},${s.location.lon}`; const g = m.get(k) || { place: k, lat: s.location.lat, lon: s.location.lon, stations: [] }; g.stations.push(s); return m.set(k, g) }, new Map()).values()];
  const signature = JSON.stringify(places.map(p => [p.place, p.stations.map(s => s.id)]));
  if (map.__radioSignature === signature) { refreshRadioChrome(); return }
  clearRadioMarkers(); map.__radioSignature = signature;
  for (const p of places) {
    const el = document.createElement('button');
    const live = p.stations.filter(stationLive).length;
    el.className = 'radio-marker' + (live ? '' : ' off') + (state.radioPlace === p.place ? ' selected' : '');
    el.innerHTML = `<span class="rm-dot">${icon('radio')}</span><span class="rm-label"><b>${esc(p.place)}</b><small>${live} ${L('live', 'en direct')}</small></span>`;
    el.setAttribute('aria-label', `${p.place}: ${p.stations.map(s => s.name).join(', ')}`);
    el.dataset.tip = p.stations.map(s => `${stationLive(s) ? '●' : '○'} ${s.name}`).join('\n');
    el.onclick = () => { state.radioPlace = state.radioPlace === p.place ? '' : p.place; const first = p.stations.find(stationLive) || p.stations[0]; if (first) state.radioSel = first.id; render(); map.flyTo({ center: [p.lon, p.lat], zoom: Math.max(map.getZoom(), 6.5), duration: reducedMotion() ? 0 : 1400 }) };
    const m = new mapboxgl.Marker({ element: el, anchor: 'left' }).setLngLat([p.lon, p.lat]).addTo(map);
    m.__place = p.place;
    radioMarkers.push(m);
  }
  refreshRadioChrome();
}

function mapSVG(ps) {
  if (state.config?.mapbox_public_token && window.mapboxgl && !mapFailed) return '<div id="map-slot"></div>';
  const xy = (lon, lat) => [25 + (lon - 12) * 18, 25 + (5.5 - lat) * 13.8];
  const path = state.geometry ? state.geometry.features[0].geometry.coordinates.map(r => r.map((p, i) => `${i ? 'L' : 'M'}${xy(...p).join(',')}`).join(' ') + 'Z').join(' ') : '';
  return `<div class="static-map"><svg viewBox="0 0 405 290" role="img" aria-label="${L('Map of place mentions', 'Carte des lieux cités')}"><path class="map-country" d="${path}"/>${mineralFeatures().map(f => { const [x,y] = xy(...f.geometry.coordinates); return `<circle data-act="mineral-select" data-arg="${esc(f.id)}" role="button" aria-label="${esc(f.properties.name)}" cx="${x}" cy="${y}" r="${state.mineralSelected === f.id ? 3 : 1.2}" fill="${f.properties.dataset === 'IPIS' ? '#d9a441' : '#9b8afb'}"><title>${esc(f.properties.name)} · ${esc(f.properties.minerals.join(' · '))} · ${esc(f.properties.date || '2021 compilation')} · ${esc(f.properties.dataset)}</title></circle>` }).join('')}${ps.map(p => { const [x, y] = xy(p.lon, p.lat); return `<g class="map-place ${state.place === p.name ? 'active' : ''}" data-act="place" data-arg="${esc(p.name)}" tabindex="0" role="button" aria-label="${esc(p.name)}, ${p.count}"><title>${esc(p.name)} · ${p.count}</title><circle cx="${x}" cy="${y}" r="${Math.min(4 + Math.sqrt(p.count), 10)}"/><text x="${x + 9}" y="${y - 7}">${esc(p.name)}</text></g>` }).join('')}</svg></div>`;
}
// Preserve one map instance through re-renders.
let mapHost = null, mapInstance = null, mapMarkers = [], mapFailed = false, markerSignature = '';
// ---------------------------------------------------------------------------------------------
// Animated national border: breathing glow, two light pulses travelling the perimeter, crisp line.

const BORDER_LAP_MS = 16000, BORDER_FPS = 30;
let borderFrame = 0, borderLast = 0;
function mountBorder() {
  const map = mapInstance;
  if (!map || !state.geometry || map.getSource('drc-outline')) return;
  const accent = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim() || '#c8dd91';
  // line-progress needs line geometry with metrics, so trace the polygon rings as lines.
  const rings = state.geometry.features.flatMap(f => f.geometry.type === 'Polygon' ? f.geometry.coordinates : f.geometry.coordinates.flat());
  const trace = { type: 'FeatureCollection', features: rings.map(r => ({ type: 'Feature', properties: {}, geometry: { type: 'LineString', coordinates: r } })) };
  map.addSource('drc-outline', { type: 'geojson', data: state.geometry });
  map.addSource('drc-trace', { type: 'geojson', data: trace, lineMetrics: true });
  map.addLayer({ id: 'drc-fill', type: 'fill', source: 'drc-outline', paint: { 'fill-color': accent, 'fill-opacity': 0.04 } });
  map.addLayer({ id: 'drc-glow', type: 'line', source: 'drc-trace', layout: { 'line-join': 'round' }, paint: { 'line-color': accent, 'line-width': 8, 'line-blur': 7, 'line-opacity': 0.22 } });
  map.addLayer({ id: 'drc-border', type: 'line', source: 'drc-outline', layout: { 'line-join': 'round' }, paint: { 'line-color': accent, 'line-width': 1.3, 'line-opacity': 0.45 } });
  map.addLayer({ id: 'drc-comet-glow', type: 'line', source: 'drc-trace', layout: { 'line-join': 'round', 'line-cap': 'round' }, paint: { 'line-width': 16, 'line-blur': 10, 'line-gradient': borderGradient(0, accent, 0.9) } });
  map.addLayer({ id: 'drc-comet', type: 'line', source: 'drc-trace', layout: { 'line-join': 'round', 'line-cap': 'round' }, paint: { 'line-width': 3.4, 'line-gradient': borderGradient(0, '#f4ffd6', 1) } });
  map.__borderAccent = accent;
  if (!borderFrame && !reducedMotion()) borderFrame = requestAnimationFrame(animateBorder);
}
/// Gradient along the border with two comets (heads at p and p + 0.5) fading over a tail.
function borderGradient(p, color, alpha) {
  const tail = 0.09, stops = new Map([[0, 0], [1, 0]]);
  const rgba = a => { const c = color.replace('#', ''); const n = parseInt(c.length === 3 ? [...c].map(x => x + x).join('') : c, 16); return `rgba(${n >> 16 & 255},${n >> 8 & 255},${n & 255},${(a * alpha).toFixed(3)})` };
  const mark = (pos, a) => { const q = Math.min(1, Math.max(0, pos)); stops.set(+q.toFixed(4), Math.max(stops.get(+q.toFixed(4)) || 0, a)) };
  for (const head of [p, (p + 0.5) % 1]) {
    for (let k = 0; k <= 6; k++) {
      const pos = head - tail * k / 6, a = Math.pow(1 - k / 6, 1.6);
      if (pos >= 0) mark(pos, a); else mark(pos + 1, a);  // wrap the tail across the ring start
    }
    mark(head + 0.004, 0);
  }
  const sorted = [...stops].sort((a, b) => a[0] - b[0]);
  return ['interpolate', ['linear'], ['line-progress'], ...sorted.flatMap(([pos, a]) => [pos, rgba(a)])];
}
function animateBorder(t) {
  borderFrame = 0;
  const map = mapInstance;
  if (!map || reducedMotion()) return;
  borderFrame = requestAnimationFrame(animateBorder);
  if (t - borderLast < 1000 / BORDER_FPS || document.hidden) return;
  borderLast = t;
  try {
    if (!map.getLayer('drc-comet')) return;
    const p = (t % BORDER_LAP_MS) / BORDER_LAP_MS, accent = map.__borderAccent || '#c6f36b';
    map.setPaintProperty('drc-comet', 'line-gradient', borderGradient(p, '#f4ffd6', 1));
    map.setPaintProperty('drc-comet-glow', 'line-gradient', borderGradient(p, accent, 0.9));
    map.setPaintProperty('drc-glow', 'line-opacity', 0.16 + 0.12 * (0.5 + 0.5 * Math.sin(t / 1400)));
  } catch {}
}

/// Cursor position and camera readout for the map HUD.
function hudText(h = {}) {
  const deg = (v, pos, neg, w) => v == null ? '—'.padEnd(w + 7) : `${Math.abs(v).toFixed(4).padStart(w + 5, '0')}°${v >= 0 ? pos : neg}`;
  return `LAT ${deg(h.lat, 'N', 'S', 2)}  LON ${deg(h.lon, 'E', 'W', 3)}  Z ${(h.zoom ?? 4).toFixed(1)}  P ${Math.round(h.pitch ?? 0)}°`;
}
const mapStyle = () => 'mapbox://styles/mapbox/' + (state.basemap === 'satellite' ? 'satellite-streets-v12' : !state.aiMode && state.theme === 'light' ? 'light-v11' : 'dark-v11');
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
      mapInstance.on('idle', mountGeology);
      mapInstance.on('idle', mountFocus);
      mapInstance.on('idle', mountRadio);
      mapInstance.on('idle', mountFlights);
      // Any manual pan or zoom hands control back to the analyst.
      for (const ev of ['dragstart', 'wheel', 'touchstart']) mapInstance.on(ev, () => { focusTour++ });
      mapInstance.on('click', 'mineral-sites', mineralPopup);
      mapInstance.on('click', 'mineral-clusters', mineralCluster);
      for (const layer of ['mineral-sites','mineral-clusters']) {
        mapInstance.on('mouseenter',layer,()=>{ mapInstance.getCanvas().style.cursor='pointer' });
        mapInstance.on('mouseleave',layer,()=>{ mapInstance.getCanvas().style.cursor='' });
      }
      mapInstance.on('click', e => {
        if (!state.mineralMode) return;
        const layers = ['mineral-sites', 'mineral-clusters'].filter(l => mapInstance.getLayer(l));
        if (layers.length && mapInstance.queryRenderedFeatures(e.point, { layers }).length) return;
        geologyAt(e.lngLat);
      });
      mapInstance.on('style.load', () => {
        mountMinerals();
        setTimeout(() => { mountGeology(); mountFocus() }, 0);
        mountBorder();
      });
    } catch { mapFailed = true; mapInstance?.remove(); mapInstance = null; mapHost = null; render(); return }
  }
  if (mapInstance.__style !== mapStyle()) { mapInstance.__style = mapStyle(); mapInstance.setStyle(mapStyle()) }
  requestAnimationFrame(() => mapInstance?.resize());
  mountMinerals();
  mountGeology();
  mountFocus();
  mountRadio();
  mountFlights();
  if (state.mineralMode || state.radioMode || state.flightMode) { mapMarkers.forEach(m => m.remove()); mapMarkers = []; markerSignature = ''; return }
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

const AI_NAMES={claude:'Claude · Anthropic',openai:'OpenAI',deepseek:'DeepSeek',kimi:'Kimi · Moonshot',soniox:'Soniox · Speech recognition',soniox_tts:'Soniox · Voice output',ollama:'Ollama · Local'};
const aiModelLists = new Map();
const aiModelPending = new Set();
const aiModelErrors = new Map();
const discoverableAi = ['claude','openai','deepseek','kimi'];
function aiModelOptions(provider, selected) {
  const verified=aiModelLists.get(provider);
  const models=[...new Set([selected,...(verified||[])])];
  return models.map(id=>{
    const unconfirmed=verified&&!verified.includes(id);
    return `<option value="${esc(id)}" ${id===selected?'selected':''} ${unconfirmed?'disabled':''}>${esc(id)}${unconfirmed?' · '+L('not available / unsupported','indisponible / non compatible'):id===selected?' · '+L('saved','enregistré'):''}</option>`;
  }).join('');
}

function aiModelNotice(p) {
  if(aiModelPending.has(p.id))return L('Loading available models…','Chargement des modèles disponibles…');
  if(aiModelErrors.has(p.id))return aiModelErrors.get(p.id);
  if(aiModelLists.has(p.id))return L('Account-visible models filtered for this integration. Select one, then Save. Paid generation has not been tested.','Modèles du compte filtrés pour cette intégration. Choisissez puis enregistrez. Génération payante non testée.');
  if(discoverableAi.includes(p.id))return p.configured?L('Available models load automatically.','Les modèles disponibles se chargent automatiquement.'):L('Save your API key to load available models.','Enregistrez votre clé API pour charger les modèles disponibles.');
  return L('Currently configured model.','Modèle actuellement configuré.');
}
async function fetchAiModels(provider, force=false) {
  if(aiModelPending.has(provider)||(!force&&(aiModelLists.has(provider)||aiModelErrors.has(provider))))return;
  aiModelPending.add(provider);aiModelErrors.delete(provider);
  const status=()=>document.getElementById('ai-check-'+provider);
  if(status())status().textContent=L('Loading available models…','Chargement des modèles disponibles…');
  try{
    const r=await api('/v1/settings/ai/check',{provider});
    const models=[...new Set(r.models)].sort();
    aiModelLists.set(provider,models);
    const select=document.getElementById('ai-model-'+provider);
    if(select)select.innerHTML=aiModelOptions(provider,select.value);
    if(!models.length)aiModelErrors.set(provider,L('No compatible models returned. Your saved model is preserved.','Aucun modèle compatible reçu. Votre modèle enregistré est conservé.'));
  }catch(e){aiModelErrors.set(provider,e.message)}
  finally{
    aiModelPending.delete(provider);
    const p=state.aiSettings?.providers.find(p=>p.id===provider);
    if(status()&&p)status().textContent=aiModelNotice(p);
  }
}
function loadAiModels(){
  for(const p of state.aiSettings?.providers||[])if(p.configured&&discoverableAi.includes(p.id))void fetchAiModels(p.id);
}

function aiSettingsView(){
 const c=state.aiSettings;
 if(state.auth?.user?.role!=='owner')return pageHead('AI',L('Owner access required','Accès propriétaire requis'),'');
 if(!c)return pageHead('AI',L('Settings unavailable','Paramètres indisponibles'),L('Reload to try again.','Rechargez pour réessayer.'));
 const options=selected=>c.providers.filter(p=>p.text_engine).map(p=>`<option value="${p.id}" ${p.id===selected?'selected':''}>${AI_NAMES[p.id]}</option>`).join('');
 return pageHead('OKAPI & AI',L('AI settings','Paramètres IA'),L('Shared workspace configuration. Keys stay on the server. Changes apply to new requests; queued jobs keep their selected model.','Configuration partagée. Les clés restent sur le serveur. Les changements concernent les nouvelles requêtes ; les tâches en file gardent leur modèle.'))+
 `<form class="panel fields ai-settings-panel" data-form="ai-engine">
 <h2>${L('Active engines','Moteurs actifs')}</h2>
 <label>${L('Article & agreement analysis','Analyse des articles et accords')}<select name="analysis_provider">${options(c.analysis_provider)}</select></label>
 <label>Okapi<select name="okapi_provider">${options(c.okapi_provider)}</select></label>
 <label class="check"><input type="checkbox" name="enabled" ${c.enabled?'checked':''}>${L('Enable AI processing','Activer le traitement IA')}</label>
 <p>${L('Saving a key does not enable spending. Configure each provider in Costs & budgets. OpenAI web research remains a separate OpenAI feature. Use a chat model supporting JSON output.','Enregistrer une clé n’active pas les dépenses. Configurez chaque fournisseur dans Coûts et budgets. La recherche web OpenAI reste une fonction OpenAI distincte. Utilisez un modèle de conversation compatible JSON.')}</p>
 <div><button class="btn primary" type="submit">${L('Save engines','Enregistrer les moteurs')}</button> <button class="btn" type="button" data-act="nav" data-arg="costs">${L('Costs & budgets','Coûts et budgets')}</button></div></form>
 <div class="ai-provider-grid">${c.providers.map(p=>`<form class="panel fields ai-settings-panel" data-form="ai-provider" data-provider="${p.id}">
 <h2>${AI_NAMES[p.id]}</h2>
 <p>${p.id==='ollama'?L('Requires a local Ollama server','Nécessite un serveur Ollama local'):p.configured?L('Key configured','Clé configurée'):L('No key configured','Aucune clé configurée')}</p>
 <label>${L('Model','Modèle')}<select name="model" id="ai-model-${p.id}" required>${aiModelOptions(p.id,p.model)}</select></label>
 ${['ollama','soniox_tts'].includes(p.id)?(p.id==='soniox_tts'?'<p>'+L('Uses the Soniox speech recognition key.','Utilise la clé Soniox de reconnaissance vocale.')+'</p>':''):`<label>${L('New API key (leave blank to keep current key)','Nouvelle clé API (laisser vide pour conserver)')}<input name="api_key" type="password" autocomplete="new-password" spellcheck="false" maxlength="4096"></label>`}
 <div class="actions"><button class="btn primary" type="submit">${L('Save','Enregistrer')}</button>
 ${p.text_engine&&p.id!=='ollama'?`<button class="btn" type="button" data-act="ai-check" data-arg="${p.id}">${L('Refresh models','Actualiser les modèles')}</button>`:''}
 ${!['ollama','soniox_tts'].includes(p.id)?`<button class="btn" type="button" data-act="ai-remove" data-arg="${p.id}">${L('Remove key','Supprimer la clé')}</button>`:''}</div>
 <p role="status" id="ai-check-${p.id}">${esc(aiModelNotice(p))}</p></form>`).join('')}</div>`;
}
function costsView(){
 const c=state.costs;
 return pageHead(L('API OPERATIONS','OPÉRATIONS API'),L('Costs & budgets','Coûts et budgets'),L('Persistent controls for Claude, OpenAI research and X. Periods reset in UTC.','Contrôles persistants pour Claude, la recherche OpenAI et X. Périodes en UTC.'))+
 `<p class="panel">${L('These are your planning allowances, not provider invoices. Set an allowance per request and daily/monthly limits before enabling. Actual charges may differ; configure billing limits with each provider too. Historical spend, Mapbox browser usage, hosting and other applications are excluded. Existing X limits also apply. Pausing affects new requests, not requests already sent.','Ces montants sont vos provisions, pas les factures des fournisseurs. Définissez une provision par requête et des limites journalières/mensuelles avant activation. Les frais réels peuvent différer ; configurez aussi les limites des fournisseurs. Historique, Mapbox du navigateur, hébergement et autres applications exclus. Les limites X existantes restent actives. La pause concerne les nouvelles requêtes.')}</p>`+
 (c?.items||[]).map(p=>`<section class="panel" style="padding:20px;margin-bottom:16px" id="cost-${p.provider}"><h2>${esc(p.provider)} · ${p.paused?L('Paused','En pause'):p.unlimited?L('Enabled · No budget cap','Activé · Sans plafond'):L('Enabled','Activé')}</h2><p>${p.requests_today}/${p.unlimited?'∞':p.daily_requests} ${L('requests today','requêtes aujourd’hui')} · $${p.allocated_today_usd.toFixed(4)} / $${p.unlimited?'∞':p.daily_usd.toFixed(2)} ${L('allocated today','provisionnés aujourd’hui')} · $${p.allocated_month_usd.toFixed(4)} / $${p.unlimited?'∞':p.monthly_usd.toFixed(2)} ${L('this month','ce mois')}</p><div class="toolbar">${[['request_usd',L('USD allowance / request','Provision USD / requête')],['daily_usd',L('Daily USD limit','Limite USD / jour')],['monthly_usd',L('Monthly USD limit','Limite USD / mois')],['daily_requests',L('Daily request cap','Requêtes max / jour')]].map(([key,label])=>`<label>${label}<input class="input" name="${key}" type="number" min="0" step="${key==='daily_requests'?'1':'0.000001'}" value="${p[key]}"></label>`).join('')}<label><input name="unlimited" type="checkbox" ${p.unlimited?'checked':''}> ${L('No provider budget or request cap','Aucun plafond fournisseur de budget ou de requêtes')}</label><label><input name="paused" type="checkbox" ${p.paused?'checked':''}> ${L('Pause paid requests','Suspendre les requêtes payantes')}</label><button class="btn" data-act="cost-save" data-arg="${p.provider}">${L('Save limits','Enregistrer')}</button></div></section>`).join('')+
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
 <details class="panel" style="padding:16px"><summary>${L('Coverage, rights and methodology','Couverture, droits et méthode')}</summary><p>${esc(srv(state.agreements?.metadata.coverage))}</p><p>${esc(srv(state.agreements?.metadata.methodology))}</p><p>${esc(state.agreements?.metadata.pax_license)}</p><p>${L('State signatures, SOE contracts and local accords require separate verification. Date-based term assignment does not establish who negotiated, ratified or implemented an agreement. Scanned documents may require OCR.','Les signatures étatiques, contrats d’entreprises publiques et accords locaux nécessitent des vérifications distinctes. Le mandat associé à une date ne détermine pas qui a négocié, ratifié ou exécuté un accord. Les scans peuvent nécessiter une reconnaissance de texte.')}</p></details>`;
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
      ${(state.feeds?.items || []).map(f => `<div class="source-row"><a href="https://${esc(f.site)}/" target="_blank" rel="noopener noreferrer">${esc(f.name)}</a><span class="feed-status" style="padding:0"><i class="dot ${!f.sync || f.configured === false ? 'warn' : f.sync.status === 'ok' ? 'ok' : 'bad'}"></i>${{ feed_full_text: L('Official feed + article pages · full text', 'Flux officiel + pages · texte intégral'), listing: L('Section pages · full text', 'Pages de rubrique · texte intégral'), reliefweb_api: L('ReliefWeb API · full text', 'API ReliefWeb · texte intégral') }[f.mode] || L('Official feed · headline + lede', 'Flux officiel · titre + chapeau')}<small>${nf(f.collected)} ${L('collected', 'collectés')}${f.sync?.checked_at ? ` · ${age(f.sync.checked_at)}` : ''}</small></span>${f.configured === false ? `<small class="muted">${L('Not configured: request a ReliefWeb appname and set RELIEFWEB_APPNAME.', 'Non configuré : demandez un appname ReliefWeb et définissez RELIEFWEB_APPNAME.')} <a href="https://apidoc.reliefweb.int/parameters#appname" target="_blank" rel="noopener noreferrer">apidoc.reliefweb.int ↗</a></small>` : ''}${f.sync?.detail?.items != null ? `<small class="muted">${L('Last poll', 'Dernier relevé')}: ${f.sync.detail.not_modified ? L('no change', 'inchangé') : `${f.sync.detail.items} ${L('items', 'éléments')} · ${f.sync.detail.drc} RDC · ${f.sync.detail.regional} ${L('regional', 'régional')}${f.sync.detail.full_text != null ? ` · ${f.sync.detail.full_text} ${L('full text', 'texte intégral')}` : ''}`}</small>` : ''}${f.sync?.error ? `<span class="ev-error">${esc(f.sync.error)}</span>` : ''}<small class="muted">${esc(srv(f.access_note))}</small></div>`).join('')}
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
  $('#overlay').innerHTML = `<div class="shade" data-close><section class="drawer" role="dialog" aria-modal="true" aria-busy="true" aria-label="${L('Report', 'Article')}"><div class="drawer-top"><span class="muted">${L('Loading…', 'Chargement…')}</span><button class="iconbtn" data-act="close" aria-label="${L('Close', 'Fermer')}">${icon('close')}</button></div><div class="drawer-body"><div class="loading-line"></div></div></section></div>`;
  $('#overlay [data-act="close"]')?.focus();
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
  if (!document.activeElement?.closest('#overlay')) $('#overlay [data-act="close"]')?.focus();
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
    <p class="notice">${esc(srv(d?.notice))}</p>`;
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
  $('#overlay').innerHTML = `<div class="shade" data-close><section class="drawer" role="dialog" aria-modal="true" aria-label="${esc(a?.name || L('Profile', 'Profil'))}"><div class="drawer-top"><span class="eyebrow">${icon('user')}${L('PROFILE', 'PROFIL')}</span><div class="actions">${a ? `<button class="btn sm" data-act="watch-quick" data-arg="${esc(`actor|${a.id}|${a.name}`)}">${icon('bell')}${L('Watch', 'Suivre')}</button><button class="btn sm" data-act="export-actor">${icon('download')}JSON</button>` : ''}<button class="iconbtn" data-act="close" aria-label="${L('Close', 'Fermer')}">${icon('close')}</button></div></div><div class="drawer-body">${body}</div></section></div>`;
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
    <p class="notice">${esc(srv(d?.notice))}</p>`;
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
  // Return focus to whatever opened the overlay (re-rendered elements are found by key).
  if (overlayReturn) { const back = $(`#app ${overlayReturn}`); overlayReturn = ''; back?.focus({ preventScroll: true }) }
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
  'ai-mode': () => {state.aiMode=!state.aiMode;store.set('congo-ai-mode',state.aiMode?'1':'0');if(state.aiMode)go('ai-command');else if(state.view==='ai-command')go('briefing');else render();},
  'ai-motion': () => {state.aiMotion=!state.aiMotion;store.set('congo-ai-motion',state.aiMotion?'1':'0');render();},
  'ai-prompt': prompt => voiceCenter.open(prompt),
  'holo-layer': (k, el) => { state.holoLayers = el.checked ? [...new Set([...holoLayerList(), k])] : holoLayerList().filter(x => x !== k); store.set('congo-holo-layers', state.holoLayers.join(',')); if (!holoAnimated()) holoFrame(performance.now()) },
  'holo-province': pv => { state.q = pv; state.place = ''; state.radioMode = false; state.mineralMode = false; go('situation') },
  'voice-open': () => voiceCenter.open(),
  'voice-listen': () => voiceCenter.listen(),
  'logout': async () => {voiceCenter.close();await api('/v1/auth/logout',{});sessionStorage.removeItem('congo-token');location.reload()},
  'ai-check': provider => fetchAiModels(provider,true),
  'ai-remove': async provider => {if(!confirm(L('Remove this shared provider key?','Supprimer cette clé partagée ?')))return;state.aiSettings=await api('/v1/settings/ai',{provider,remove_key:true});aiModelLists.delete(provider);aiModelErrors.delete(provider);render();},
  'cost-export': () => download('congo-signal-costs.json',state.costs),
  'cost-save': async provider => {const root=document.getElementById('cost-'+provider);const body={provider,paused:root.querySelector('[name=paused]').checked,unlimited:root.querySelector('[name=unlimited]').checked};for(const key of ['request_usd','daily_usd','monthly_usd','daily_requests'])body[key]=Number(root.querySelector('[name='+key+']').value);state.costs=await api('/v1/costs',body);render()},
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
  'mineral-close': () => { state.mineralSelected = ''; if (location.hash.startsWith('#mine/')) history.replaceState(null, '', '#minerals'); render() },
  'mineral-fit': () => fitMinerals(),
  'mineral-more': () => {state.mineralVisible+=30;render();$('.mineral-directory')?.setAttribute('open','')},
  'mineral-reset': () => {Object.assign(state,{mineralSearch:'',mineralQuery:'',mineralProvince:'',mineralKind:'',mineralSelected:'',mineralVisible:15,mineralLayers:['IPIS','USGS']});render()},
  'mineral-export': () => {const f=selectedMineral();if(f) download('congo-signal-site.json',{...f,metadata:state.geology.metadata})},
  'map-focus': () => {state.mapFocus=!state.mapFocus;render()},
  'flight-select': id => selectFlight(id),
  'flight-cat': k => { state.flightCategory = k; render() },
  'ev-days': async n => { state.evDays = Number(n); state.evDay = ''; state.events = await api(`/v1/events?days=${state.evDays}`); render() },
  'ev-day': d => { state.evDay = state.evDay === d ? '' : d; render() },
  'ev-type': k => { state.evType = k; render() },
  'ev-play': () => toggleEvPlay(),
  'ev-open': id => { state.evOpen = state.evOpen === id ? '' : id; render(); if (state.evOpen) $(`#ev-${CSS.escape(id)}`)?.scrollIntoView({ block: 'nearest', behavior: reducedMotion() ? 'auto' : 'smooth' }) },
  'watch-new': () => { state.watchEdit = blankWatch(); state.telegramChats = null; render(); $('#watch-name')?.focus() },
  'watch-template': arg => { const [kind, terms] = arg.split('|'); state.watchEdit = { ...blankWatch(kind), termsText: terms || '' }; render(); $('#watch-name')?.focus() },
  'watch-edit': id => { const w = state.watches?.items?.find(x => x.id === id); if (w) { state.watchEdit = watchToEdit(w); state.telegramChats = null; render() } },
  'watch-cancel': () => { state.watchEdit = null; render() },
  'watch-save': async () => { await saveWatch(state.watchEdit); state.watchEdit = null; toast(L('Watch saved', 'Veille enregistrée')); await refreshWatches() },
  'watch-toggle': async id => { const w = state.watches?.items?.find(x => x.id === id); if (!w) return; await saveWatch({ ...watchToEdit(w), active: !w.active }); await refreshWatches() },
  'watch-delete': async id => { if (!confirm(L('Delete this watch and its alerts?', 'Supprimer cette veille et ses alertes ?'))) return; await fetch(`/v1/watches/${encodeURIComponent(id)}`, { method: 'DELETE', headers: { ...authHeaders(), ...(state.auth?.csrf ? { 'X-CSRF-Token': state.auth.csrf } : {}) } }); if (state.watchResults?.id === id) state.watchResults = null; await refreshWatches() },
  'watch-results': async id => { const w = state.watches?.items?.find(x => x.id === id); state.watchResults = { id, name: w?.name || '', data: null }; render(); state.watchResults.data = await api(`/v1/watches/${encodeURIComponent(id)}/results`); render() },
  'watch-results-close': () => { state.watchResults = null; render() },
  'watch-test': async channel => { const e = state.watchEdit, target = channel === 'email' ? (e.email || (/@/.test(state.watches?.email_default || '') ? state.watches.email_default : '')) : e.telegram; if (!target) throw Error(channel === 'email' ? L('Enter an email address first.', 'Saisissez d’abord une adresse e-mail.') : L('Enter a Telegram chat id first.', 'Saisissez d’abord un identifiant Telegram.')); await api('/v1/watches/test', { channel, target }); toast(L('Test sent. Check your inbox or Telegram.', 'Test envoyé. Vérifiez votre messagerie ou Telegram.')) },
  'telegram-chats': async () => { state.telegramChats = await api('/v1/watches/telegram-chats'); render() },
  'telegram-pick': id => { state.watchEdit.telegram = id; state.telegramChats = null; render() },
  'watch-quick': arg => quickWatch(arg),
  'alerts-filter': f => { state.alertsUnread = f === 'unread'; render() },
  'alerts-read-all': async () => { await api('/v1/alerts/read', { all: true }); await refreshWatches() },
  'alert-open': async arg => {
    const [id, kind, ref] = arg.split('|'), a = state.alerts?.items?.find(x => String(x.id) === id);
    if (a && !a.read) { api('/v1/alerts/read', { ids: [Number(id)] }).catch(() => {}); a.read = true; state.alerts.unread = Math.max(0, state.alerts.unread - 1) }
    if (kind === 'article') return openDetail(ref);
    if (kind === 'post') { const url = a?.detail?.url; if (url) window.open(url, '_blank', 'noopener'); return render() }
    if (kind === 'flight') { state.flightSelected = a?.detail?.icao24 || ''; state.flightMode = true; state.radioMode = false; state.mineralMode = false; history.replaceState(null, '', '#flights'); return go('situation') }
    if (kind === 'brief') { state.briefPeriod = a?.detail?.period || 'daily'; return downloadBrief('docx') }
    render();
  },
  'open-url': url => { if (/^https?:/.test(url)) window.open(url, '_blank', 'noopener') },
  'brief-dl': format => downloadBrief(format),
  'map-mode': m => { state.flightMode = m === 'flights'; state.radioMode = m === 'radio'; if (state.radioMode && !state.radioSel) state.radioSel = state.radioNow || (stationById('radio-okapi') ? 'radio-okapi' : ''); state.mineralMode = m === 'minerals'; if (!state.mineralMode) { focusTour++; state.focus = null; state.mineralFocusLabels = [] } state.geoUnit = null; history.replaceState(null, '', state.mineralMode ? '#minerals' : state.radioMode ? '#radio' : state.flightMode ? '#flights' : '#situation'); if (state.view !== 'situation') go('situation'); else render() },
  market: id => { state.marketCommodity = id; state.mineralSelected = ''; render() },
  'market-open': id => { state.marketCommodity = id; state.mineralSelected = ''; render() },
  'market-sites': id => {
    const c = state.commodities?.items?.find(x => x.id === id); if (!c) return;
    const present = new Set((state.geology?.features || []).flatMap(f => f.properties.minerals));
    state.mineralQuery = c.labels.find(l => present.has(l)) || ''; state.mineralSelected = ''; render(); fitMinerals();
  },
  'geo-close': () => { state.geoUnit = null; render() },
  focus: key => focusMineral(key),
  'focus-exit': () => exitFocus(),
  'focus-tour': () => playFocusTour(),
  'focus-area': i => playFocusTour(Number(i)),
  'focus-overview': () => { focusTour++; markActiveArea(-1); if (state.focus?.areas.length) mapInstance?.fitBounds(allAreasBounds(state.focus.areas), { padding: 90, maxZoom: 7, pitch: 0, bearing: 0, duration: reducedMotion() ? 0 : 1600 }) },
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
  'radio-select': id => { state.radioSel = id; radioHash(id); const s = stationById(id); render(); if (s?.location) mapInstance?.flyTo({ center: [s.location.lon, s.location.lat], zoom: Math.max(mapInstance.getZoom(), 6), duration: reducedMotion() ? 0 : 1200 }) },
  'radio-play': id => playStation(id),
  'radio-toggle': () => { if (radioAudio.paused) playStation(state.radioNow); else radioAudio.pause() },
  'radio-stop': () => stopRadio(),
  'radio-open': () => { state.flightMode=false; state.radioMode = true; state.mineralMode = false; state.radioSel = state.radioNow; history.replaceState(null, '', '#radio'); if (state.view !== 'situation') go('situation'); else render() },
  'radio-place': p => { state.radioPlace = p; render() },
  'reset-map': () => { state.place = ''; render(); mapInstance?.fitBounds([[12, -13.6], [31.4, 5.5]], { padding: 40, duration: 600 }) },
};
document.addEventListener('click', async e => {
  const pal = e.target.closest('[data-palette]'); if (pal) { runPalette(Number(pal.dataset.palette)); return }
  if (e.target.closest('[data-stop]')) return;
  const el = e.target.closest('[data-act]');
  if (el && !el.closest('#overlay') && el.closest('#app')) overlayReturn = focusKey(el);
  if (!el) { const shade = e.target.closest('[data-close]'); if (shade && e.target === shade) closeOverlay(); return }
  if (el.tagName === 'A' && el.dataset.act === 'nav') e.preventDefault();
  const fn = ACTIONS[el.dataset.act]; if (!fn) return;
  const busy = el.tagName === 'BUTTON' && !['nav', 'lang', 'theme', 'close', 'tab', 'category', 'layer', 'day', 'window', 'expand', 'actor-kind', 'arms-cat', 'actor-mkind', 'show-3d', 'map-mode', 'market', 'market-open', 'geo-close', 'focus', 'focus-exit', 'focus-tour', 'focus-area', 'focus-overview', 'radio-select', 'radio-play', 'radio-toggle', 'radio-stop', 'radio-open', 'radio-place', 'ev-day', 'ev-type', 'ev-play', 'ev-open', 'watch-new', 'watch-template', 'watch-edit', 'watch-cancel', 'watch-results-close', 'alerts-filter', 'alert-open', 'open-url', 'telegram-pick', 'brief-dl'].includes(el.dataset.act);
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
  if (e.target.dataset?.act === 'radio-volume') { radioAudio.volume = Number(e.target.value); store.set('congo-radio-volume', e.target.value); return }
  if (e.target.dataset?.act === 'geo-opacity') { state.geoOpacity = Number(e.target.value); if (mapInstance?.getLayer('geology')) mapInstance.setPaintProperty('geology', 'raster-opacity', state.geoOpacity); return }
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
  if(kind==='ai-engine'||kind==='ai-provider'){
    const f=new FormData(form);const body=kind==='ai-engine'?{analysis_provider:f.get('analysis_provider'),okapi_provider:f.get('okapi_provider'),enabled:f.has('enabled')}:{provider:form.dataset.provider,model:f.get('model')};
    if(kind==='ai-provider'&&!body.model)delete body.model;
    if(kind==='ai-provider' && f.get('api_key')?.trim())body.api_key=f.get('api_key').trim();
    const button=form.querySelector('[type=submit]');button.disabled=true;
    try{state.aiSettings=await api('/v1/settings/ai',body);if(body.api_key){aiModelLists.delete(body.provider);aiModelErrors.delete(body.provider)}form.querySelector('[name=api_key]')?.value && (form.querySelector('[name=api_key]').value='');render();toast(L('AI settings saved','Paramètres IA enregistrés'))}catch(err){toast(err.message)}finally{button.disabled=false}return;
  }
  if(kind==='team'){const f=new FormData(form),body={email:f.get('email'),role:f.get('role'),active:f.has('active'),daily_requests:Number(f.get('daily_requests')),daily_usd:Number(f.get('daily_usd')),monthly_usd:Number(f.get('monthly_usd'))};try{state.team=await api('/v1/team',body);toast(L('Membership saved','Membre enregistré'));await load()}catch(err){toast(err.message)}return}
  if (kind === 'okapi-ask') { const q = form.q.value.trim(); voiceCenter.open(q || undefined); form.q.value = ''; return }
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
window.addEventListener('popstate', () => { state.flightMode = location.hash === '#flights'; state.mineralMode = /^#(minerals|mine\/|focus\/)/.test(location.hash); state.radioMode = /^#radio/.test(location.hash); state.view = viewFromHash(); render(); openFromHash() });

// ---------------------------------------------------------------------------------------------
// Boot

const voiceCenter = createVoiceCenter({api,L,currentView:()=>state.view==='ai-command'?'briefing':state.view,execute:async command=>{
  if(command.type==='navigate'){
    if(['minerals','flights','radio','situation'].includes(command.target)){
      if(command.target==='minerals')Object.assign(state,{mineralSearch:command.query||'',mineralQuery:'',mineralProvince:'',mineralKind:'',mineralSelected:'',mineralFocusLabels:[],mineralLayers:['IPIS','USGS']});
      state.view='situation';ACTIONS['map-mode'](command.target==='situation'?'news':command.target);
    }else go(command.target);
    return L('Opened ','Page ouverte : ')+command.target;
  }
  if(command.type==='search'){state.q=command.query;state.place='';state.category='all';state.publisher='';state.visible=20;go('reports');return L('Searching loaded reports for: ','Recherche dans les articles chargés : ')+command.query;}
  if(command.type==='brief'){
    const rows=[...state.items].sort((a,b)=>b.rank.score-a.rank.score).slice(0,3);go('briefing');
    return rows.length?L('Top reports in the loaded collection. These are source claims, not independently verified. ','Principaux articles chargés. Déclarations des sources, non vérifiées indépendamment. ')+rows.map(a=>title(a)+' ('+(a.source_name||a.publisher||'source linked in report')+')').join('. '):L('No reports loaded.','Aucun article chargé.');
  }
  if(command.type==='collect'){
    const results=await Promise.allSettled([api('/v1/jobs',{url:'https://actualite.cd/'}),api('/v1/feeds/refresh',{})]);
    const queued=results.filter(r=>r.status==='fulfilled').length;
    const failures=results.filter(r=>r.status==='rejected').map(r=>r.reason.message);
    if(!queued)throw Error(failures.join(' · '));
    await load();return L(`Collection worker: ${queued}/2 requests accepted. Articles will appear as jobs complete.`,`Collecteur : ${queued}/2 demandes acceptées. Les articles apparaîtront après traitement.`)+(failures.length?' '+failures.join(' · '):'');
  }
  if(command.type==='analyze'){
    const rows=(state.view==='situation'?situationItems():['reports','saved'].includes(state.view)?libraryItems():[...state.items].sort((a,b)=>b.rank.score-a.rank.score)).filter(a=>['not_requested','failed'].includes(a.brief?.status)).slice(0,5);
    let queued=0;
    for(const a of rows){try{await(a.record_type==='evidence'?api(`/v1/evidence/${a.id}/analyze`,{}):api('/v1/intelligence/analyze',{article_id:a.id,task:'brief'}));queued++;}catch(e){throw Error(L(`${queued} queued before stopping: `,`${queued} ajouté(s) avant arrêt : `)+e.message);}}
    await load();return L(`Analysis worker: ${queued} reports queued. Completion is shown in report AI briefs. Existing budgets apply.`,`Analyse : ${queued} articles ajoutés à la file. Consultez les synthèses IA pour les résultats. Budgets existants appliqués.`);
  }
  throw Error('Unsupported command');
}});

try { state.geometry = await fetch('/drc.geo.json').then(r => r.json()) } catch {}
try { state.geology = await fetch('/v1/geology/sites').then(r => { if (!r.ok) throw Error(r.status); return r.json() }) } catch {}
state.enter = true;
await load();
openFromHash();
document.addEventListener('visibilitychange',()=>{document.documentElement.dataset.pageHidden=String(document.hidden)});
// Full refresh every minute (17 endpoints); unread alerts every 20 s with a one-row request.
setInterval(async () => {
  if (document.hidden) return;
  try { const a = await api('/v1/alerts?unread=true&limit=1'); if (state.alerts && a.unread !== state.alerts.unread) { state.alerts = await api('/v1/alerts?limit=200'); state.watches = await api('/v1/watches').catch(() => state.watches); render() } } catch {}
}, 20000);
setInterval(async () => {
  if (['ai-settings','ai-command'].includes(state.view) || document.hidden || (state.modal && state.modal !== 'detail') || document.activeElement?.matches('input,textarea,select')) return;
  await load();
  if (state.detailId && state.detailTab !== 'notes') {
    try { const d = await api(`/v1/intelligence/${state.detailId}`); if (JSON.stringify(d) !== JSON.stringify(state.detail)) { state.detail = d; renderDetail() } } catch {}
  }
}, 60000);
