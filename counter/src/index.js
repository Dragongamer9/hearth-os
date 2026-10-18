// Hearth OS live player counter (a Cloudflare Worker with a D1 database).
//
//   POST /v1/ping   {"id": "<random install id>", "v": "0.0.20", "k": "amd|nvidia|windows|appimage|live"}
//                   Each install sends this about every 10 minutes while Hearth (or Ember) is running.
//   GET  /v1/stats  {"now": n, "today": n, "month": n, "kinds": {...}, "updated": ms}
//                   now = seen in the last 15 minutes, today = 24 hours, month = 30 days.
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
    if (url.pathname === '/') return json({ service: 'Hearth OS player counter', stats: '/v1/stats' });
    return json({ error: 'not found' }, 404);
  },

  // Daily: forget installs not seen for 40 days.
  async scheduled(_event, env) {
    await env.DB.prepare('DELETE FROM pings WHERE last < ?').bind(Date.now() - 40 * 864e5).run();
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
