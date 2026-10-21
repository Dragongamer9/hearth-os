// Hearth OS live player counter (a Cloudflare Worker with a D1 database).
//
//   POST /v1/ping   {"id": "<random install id>", "v": "0.0.20", "k": "amd|nvidia|windows|appimage|live"}
//                   Each install sends this about every 10 minutes while Hearth (or Ember) is running.
//   GET  /v1/stats  {"now": n, "today": n, "month": n, "kinds": {...}, "updated": ms}
//                   now = seen in the last 15 minutes, today = 24 hours, month = 30 days.
//
//   POST /v1/logs   (text) Settings › System › "Send logs": a bug report's logs (personal details already taken
//                   out on the device). Answers {"code": "ABCD-EFGH-JK"}; kept 14 days.
//   POST /v1/vote   {"id": "<device id>", "layout": "x-orbit", "up": true}  upvote (or take back) an experimental layout
//   GET  /v1/votes  {"counts": {"x-orbit": 12, ...}}  everyone's upvotes, live
//   GET  /v1/logs/<code>  the logs, as text, for whoever has the code (the person shows it to the developer).
//   Each log sent is also put in the private GitHub repository LOGS_REPO (a file plus an issue with the code,
//   version, device, the person's note from X-Hearth-Note, and the error lines), once the worker has the secret
//   LOGS_GITHUB_TOKEN (added by hand in Cloudflare's dashboard). The repository is private: logs name devices.
//
// Stored per install: the random id, its kind, its version, when it was first and last seen. No IP
// addresses, names or anything else. Rows not seen for 40 days are deleted every day.

const KINDS = new Set(['amd', 'nvidia', 'windows', 'appimage', 'live', 'anatase', 'ubuntu', 'cachyos', 'peppermint', 'android', 'other']);
const CORS = { 'access-control-allow-origin': '*', 'access-control-allow-methods': 'GET, POST, OPTIONS', 'access-control-allow-headers': 'content-type' };
const json = (o, status = 200, extra = {}) => new Response(JSON.stringify(o), { status, headers: { 'content-type': 'application/json', ...CORS, ...extra } });

export default {
  async fetch(req, env, ctx) {
    const url = new URL(req.url);
    if (req.method === 'OPTIONS') return new Response(null, { headers: CORS });
    if (url.pathname === '/v1/ping' && req.method === 'POST') return ping(req, env);
    if (url.pathname === '/v1/stats' && req.method === 'GET') return stats(req, env, ctx);
    if (url.pathname === '/v1/logs' && req.method === 'POST') return putLogs(req, env, ctx);
    if (url.pathname === '/v1/vote' && req.method === 'POST') return putVote(req, env);
    if (url.pathname === '/v1/votes' && req.method === 'GET') return json({ counts: await voteCounts(env) }, 200, { 'cache-control': 'no-store' });
    if (url.pathname === '/v1/github-errors' && req.method === 'GET') return ghErrors(env);
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

async function putLogs(req, env, ctx) {
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
  let note = '';
  try { note = decodeURIComponent(req.headers.get('x-hearth-note') || ''); } catch (e) { note = ''; }
  note = note.replace(/[\u0000-\u001f]/g, ' ').trim().slice(0, 1000);
  if (env.LOGS_GITHUB_TOKEN) ctx.waitUntil(toGitHub(env, code, ver, kind, body, note).catch(e => ghFailed(env, code, e)));
  return json({ code });
}

// ---------------------------------------------------------------- logs to the private GitHub repository
function b64(text) {
  const bytes = new TextEncoder().encode(text);
  let s = '';
  for (let i = 0; i < bytes.length; i += 0x8000) s += String.fromCharCode.apply(null, bytes.subarray(i, i + 0x8000));
  return btoa(s);
}
async function gh(env, method, path, body) {
  const r = await fetch('https://api.github.com' + path, { method, body: body ? JSON.stringify(body) : undefined, headers: {
    authorization: 'Bearer ' + env.LOGS_GITHUB_TOKEN, accept: 'application/vnd.github+json', 'user-agent': 'hearth-counter',
    'x-github-api-version': '2022-11-28', 'content-type': 'application/json' } });
  if (!r.ok) throw new Error(method + ' ' + path + ': ' + r.status + ' ' + (await r.text()).slice(0, 300));
  return r.json();
}
// The lines worth reading first: errors, failures, crashes (each once, at most 80).
export function problemLines(body) {
  const seen = new Set(), out = [];
  for (const line of body.split('\n')) {
    if (!/(error|failed|panic|couldn't|could not|traceback|exception|segfault|denied|not found|timed out)/i.test(line)) continue;
    const key = line.replace(/^\d{10,}\s*/, '').replace(/\b\d+\b/g, '#').slice(0, 200);
    if (seen.has(key)) continue;
    seen.add(key); out.push(line.slice(0, 300));
    if (out.length >= 80) break;
  }
  return out;
}
// Why a copy didn't reach GitHub (GitHub's answer, never the token), for GET /v1/github-errors.
async function ghFailed(env, code, e) {
  const msg = String(e && e.message || e).slice(0, 500);
  console.log('github: ' + msg);
  try {
    await env.DB.prepare('CREATE TABLE IF NOT EXISTS gh_errors (code TEXT, created INTEGER, error TEXT)').run();
    await env.DB.prepare('INSERT INTO gh_errors (code, created, error) VALUES (?, ?, ?)').bind(code, Date.now(), msg).run();
  } catch (x) { console.log('github error not kept: ' + x); }
}
async function ghErrors(env) {
  try {
    const r = await env.DB.prepare('SELECT code, created, error FROM gh_errors ORDER BY created DESC LIMIT 10').all();
    return json({ errors: r.results || [] }, 200, { 'cache-control': 'no-store' });
  } catch (e) { return json({ errors: [] }, 200, { 'cache-control': 'no-store' }); }
}

// The log file goes up as a Git blob in plain text (no base64: that took too long for big logs), then a commit.
async function putFile(env, repo, path, text, message) {
  const blob = await gh(env, 'POST', `/repos/${repo}/git/blobs`, { content: text, encoding: 'utf-8' });
  for (let attempt = 0; attempt < 3; attempt++) {
    const ref = await gh(env, 'GET', `/repos/${repo}/git/ref/heads/main`);
    const parent = await gh(env, 'GET', `/repos/${repo}/git/commits/${ref.object.sha}`);
    const tree = await gh(env, 'POST', `/repos/${repo}/git/trees`, { base_tree: parent.tree.sha, tree: [{ path, mode: '100644', type: 'blob', sha: blob.sha }] });
    const commit = await gh(env, 'POST', `/repos/${repo}/git/commits`, { message, tree: tree.sha, parents: [ref.object.sha] });
    try { await gh(env, 'PATCH', `/repos/${repo}/git/refs/heads/main`, { sha: commit.sha }); return; }
    catch (e) { if (attempt === 2) throw e; }   // someone else committed in between: again on top of theirs
  }
}

async function toGitHub(env, code, ver, kind, body, note) {
  const repo = env.LOGS_REPO || 'Dragongamer9/hearth-logs';
  const day = new Date().toISOString().slice(0, 10);
  const path = `logs/${day}/${code}.txt`;
  await putFile(env, repo, path, body, `Logs ${code} (Hearth ${ver || '?'})`);
  const head = body.split('\n').slice(0, 12).filter(l => l.trim()).join('\n');
  const device = (body.match(/"device":\s*"([^"]+)"/) || body.match(/^Device:\s*(.+)$/m) || [])[1] || kind;
  const problems = problemLines(body);
  const title = `${code} · Hearth ${ver || '?'} · ${device.slice(0, 60)}` + (note ? ` · ${note.slice(0, 80)}` : '');
  const md = [
    note ? `**What happened:** ${note}` : '_No description given._',
    '', '```', head, '```',
    '', `**Full log:** [${path}](https://github.com/${repo}/blob/main/${path}) · also at \`/v1/logs/${code}\` for 14 days`,
    '', `### Problems found (${problems.length})`, '', '```', problems.join('\n').replace(/```/g, "'''") || '(none matched)', '```',
  ].join('\n').slice(0, 60000);
  await gh(env, 'POST', `/repos/${repo}/issues`, { title, body: md, labels: [ver ? 'v' + ver : 'unknown version', kind] });
}

async function getLogs(url, env) {
  const code = decodeURIComponent(url.pathname.slice('/v1/logs/'.length)).toUpperCase().replace(/[^A-Z0-9-]/g, '');
  if (!/^[A-Z0-9]{4}-[A-Z0-9]{4}-[A-Z0-9]{2}$/.test(code)) return json({ error: 'bad code' }, 400);
  const row = await env.DB.prepare('SELECT body FROM logs WHERE code = ?').bind(code).first();
  if (!row) return json({ error: 'not found (logs are kept 14 days)' }, 404);
  return new Response(row.body, { headers: { 'content-type': 'text/plain; charset=utf-8', 'cache-control': 'no-store', ...CORS } });
}

// ---------------------------------------------------------------- layout upvotes
async function voteCounts(env) {
  const rows = await env.DB.prepare('SELECT layout, COUNT(*) AS n FROM votes GROUP BY layout').all();
  const out = {};
  for (const r of rows.results || []) out[r.layout] = r.n;
  return out;
}

async function putVote(req, env) {
  let b;
  try { b = await req.json(); } catch { return json({ error: 'bad json' }, 400); }
  const id = String(b && b.id || ''), layout = String(b && b.layout || '');
  if (!/^[a-f0-9]{16,64}$/.test(id)) return json({ error: 'bad id' }, 400);
  if (!/^x-[a-z0-9-]{2,30}$/.test(layout)) return json({ error: 'bad layout' }, 400);
  if (b.up) await env.DB.prepare('INSERT OR IGNORE INTO votes (device, layout, at) VALUES (?, ?, ?)').bind(id, layout, Date.now()).run();
  else await env.DB.prepare('DELETE FROM votes WHERE device = ? AND layout = ?').bind(id, layout).run();
  return json({ ok: true, counts: await voteCounts(env) });
}
