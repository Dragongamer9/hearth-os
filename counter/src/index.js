// Hearth OS live player counter (a Cloudflare Worker with a D1 database).
//
//   POST /v1/ping   {"id": "<random install id>", "v": "0.0.20", "k": "amd|nvidia|windows|appimage|live"}
//                   Each install sends this about every 10 minutes while Hearth (or Ember) is running.
//   GET  /v1/stats  {"now": n, "today": n, "month": n, "kinds": {...}, "updated": ms}
//                   now = seen in the last 15 minutes, today = 24 hours, month = 30 days.
//
//   POST /v1/logs   (text) Settings › System › "Send logs": a bug report's logs (personal details already taken
//                   out on the device). Answers {"code": "ABCD-EFGH-JK"}; kept 14 days.
//   GET  /v1/logs/<code>  the logs, as text, for whoever has the code (the person shows it to the developer).
//
// Stored per install: the random id, its kind, its version, when it was first and last seen. No IP
// addresses, names or anything else. Rows not seen for 40 days are deleted every day.

const KINDS = new Set(['amd', 'nvidia', 'windows', 'appimage', 'live']);
const CORS = { 'access-control-allow-origin': '*', 'access-control-allow-methods': 'GET, POST, OPTIONS', 'access-control-allow-headers': 'content-type' };
const json = (o, status = 200, extra = {}) => new Response(JSON.stringify(o), { status, headers: { 'content-type': 'application/json', ...CORS, ...extra } });

export default {
  async fetch(req, env, ctx) {
    const url = new URL(req.url);
    if (req.method === 'OPTIONS') return new Response(null, { headers: CORS });
    if (url.pathname === '/v1/ping' && req.method === 'POST') return ping(req, env);
    if (url.pathname === '/v1/stats' && req.method === 'GET') return stats(req, env, ctx);
    if (url.pathname === '/v1/logs' && req.method === 'POST') return putLogs(req, env);
    if (url.pathname.startsWith('/v1/logs/') && req.method === 'GET') return getLogs(url, env);
    if (url.pathname === '/') return json({ service: 'Hearth OS player counter', stats: '/v1/stats' });
    return json({ error: 'not found' }, 404);
  },

  // Daily: forget installs not seen for 40 days.
  async scheduled(_event, env) {
    await env.DB.prepare('DELETE FROM pings WHERE last < ?').bind(Date.now() - 40 * 864e5).run();
    await env.DB.prepare('DELETE FROM logs WHERE created < ?').bind(Date.now() - 14 * 864e5).run();
  },
};

async function ping(req, env) {
  let b;
  try { b = await req.json(); } catch { return json({ error: 'bad json' }, 400); }
  const id = String(b && b.id || '');
  const kind = KINDS.has(b && b.k) ? b.k : 'other';
  const ver = String(b && b.v || '').slice(0, 20).replace(/[^\w.+-]/g, '');
  if (!/^[a-f0-9]{16,64}$/.test(id)) return json({ error: 'bad id' }, 400);
  const now = Date.now();
  // One write per install per ~4 minutes at most (keeps well inside the free plan).
  await env.DB.prepare(
    `INSERT INTO pings (id, kind, version, first, last) VALUES (?1, ?2, ?3, ?4, ?4)
     ON CONFLICT(id) DO UPDATE SET kind = ?2, version = ?3, last = ?4 WHERE pings.last < ?4 - 240000`
  ).bind(id, kind, ver, now).run();
  return json({ ok: true });
}

async function stats(req, env, ctx) {
  const cache = caches.default;
  const key = new Request(new URL('/v1/stats', req.url).toString());
  const hit = await cache.match(key);
  if (hit) return hit;
  const now = Date.now();
  const rows = await env.DB.prepare(
    `SELECT kind,
            SUM(CASE WHEN last > ?1 THEN 1 ELSE 0 END) AS now,
            SUM(CASE WHEN last > ?2 THEN 1 ELSE 0 END) AS today,
            SUM(CASE WHEN last > ?3 THEN 1 ELSE 0 END) AS month
     FROM pings GROUP BY kind`
  ).bind(now - 15 * 60e3, now - 864e5, now - 30 * 864e5).all();
  const out = { now: 0, today: 0, month: 0, kinds: {}, updated: now };
  for (const r of rows.results || []) {
    out.now += r.now || 0; out.today += r.today || 0; out.month += r.month || 0;
    out.kinds[r.kind] = { now: r.now || 0, today: r.today || 0, month: r.month || 0 };
  }
  const res = json(out, 200, { 'cache-control': 'public, max-age=60' });
  ctx.waitUntil(cache.put(key, res.clone()));
  return res;
}

// ---------------------------------------------------------------- bug-report logs
const MAX_LOG = 1800000;                       // D1 keeps rows up to about 2 MB
const CODE_CHARS = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';   // no 0/O or 1/I to mix up

function newCode() {
  const b = new Uint8Array(10); crypto.getRandomValues(b);
  const c = [...b].map(x => CODE_CHARS[x % CODE_CHARS.length]).join('');
  return `${c.slice(0, 4)}-${c.slice(4, 8)}-${c.slice(8)}`;
}

async function putLogs(req, env) {
  const len = Number(req.headers.get('content-length') || 0);
  if (len > MAX_LOG) return json({ error: 'too big' }, 413);
  const body = await req.text();
  if (!body || body.length > MAX_LOG) return json({ error: body ? 'too big' : 'empty' }, body ? 413 : 400);
  if (!body.startsWith('Hearth logs')) return json({ error: 'not a Hearth log' }, 400);
  const ver = String(req.headers.get('x-hearth-version') || '').slice(0, 20).replace(/[^\w.+-]/g, '');
  const kind = KINDS.has(req.headers.get('x-hearth-kind')) ? req.headers.get('x-hearth-kind') : 'other';
  // At most 20 a day from everyone together would already be a lot for a beta: refuse floods.
  const recent = await env.DB.prepare('SELECT COUNT(*) AS n FROM logs WHERE created > ?').bind(Date.now() - 3600e3).first();
  if (recent && recent.n > 60) return json({ error: 'busy, try again later' }, 429);
  const code = newCode();
  await env.DB.prepare('INSERT INTO logs (code, created, version, kind, body) VALUES (?, ?, ?, ?, ?)').bind(code, Date.now(), ver, kind, body).run();
  return json({ code });
}

async function getLogs(url, env) {
  const code = decodeURIComponent(url.pathname.slice('/v1/logs/'.length)).toUpperCase().replace(/[^A-Z0-9-]/g, '');
  if (!/^[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{2}$/.test(code)) return json({ error: 'bad code' }, 400);
  const row = await env.DB.prepare('SELECT body FROM logs WHERE code = ?').bind(code).first();
  if (!row) return json({ error: 'not found (logs are kept 14 days)' }, 404);
  return new Response(row.body, { headers: { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store', ...CORS } });
}
