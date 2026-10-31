// Local test sites for Theater's catalog reader (tests/fixtures): one with JSON-LD, one with Open Graph cards, one
// with an RSS feed and a sitemap, and one with only plain repeated cards. Each is its own little web server on
// 127.0.0.1; posters (/img/<name>.svg) are drawn on the fly. Nothing here talks to a real site.
//   node tests/serve.mjs 8761     (serves jsonld on 8761, og on 8762, rss on 8763, cards on 8764 until stopped)
import http from 'node:http';
import { existsSync, readFileSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const here = path.dirname(fileURLToPath(import.meta.url));
export const FIXTURES = ['jsonld', 'og', 'rss', 'cards'];
const TYPES = { '.html': 'text/html; charset=utf-8', '.xml': 'application/xml', '.txt': 'text/plain', '.svg': 'image/svg+xml' };

function poster(name) {
  let h = 0; for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  const hue = h % 360, wide = /^(ep|feed|extra|film)-/.test(name);
  const [w, ht] = wide ? [320, 180] : [200, 300];
  const label = name.replace(/[^a-z0-9 -]/gi, '').replace(/-/g, ' ');
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${ht}" viewBox="0 0 ${w} ${ht}"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="hsl(${hue},70%,55%)"/><stop offset="1" stop-color="hsl(${(hue + 50) % 360},60%,22%)"/></linearGradient></defs><rect width="100%" height="100%" fill="url(#g)"/><text x="50%" y="88%" font-family="sans-serif" font-size="${wide ? 22 : 20}" fill="#fff" text-anchor="middle">${label}</text></svg>`;
}

function handler(root) {
  return (req, res) => {
    const u = new URL(req.url, 'http://x');
    const p = decodeURIComponent(u.pathname);
    const img = /^\/img\/([a-z0-9-]+)\.svg$/i.exec(p);
    if (img) { res.writeHead(200, { 'content-type': TYPES['.svg'] }); res.end(poster(img[1])); return; }
    const base = path.join(root, path.normalize(p).replace(/^(\.\.[/\\])+/, ''));
    if (!base.startsWith(root)) { res.writeHead(403); res.end(); return; }
    const tries = [base, base + '.html', path.join(base, 'index.html')];
    const f = tries.find(t => existsSync(t) && statSync(t).isFile());
    if (!f) { res.writeHead(404, { 'content-type': 'text/plain' }); res.end('not found'); return; }
    res.writeHead(200, { 'content-type': TYPES[path.extname(f)] || 'application/octet-stream' });
    res.end(readFileSync(f));
  };
}

/** Starts the four sites; port 0 picks free ports. Returns {name: "http://127.0.0.1:<port>/"} and a close(). */
export async function serveFixtures(basePort = 0) {
  const urls = {}, servers = [];
  for (const [i, name] of FIXTURES.entries()) {
    const s = http.createServer(handler(path.join(here, 'fixtures', name)));
    await new Promise(r => s.listen(basePort ? basePort + i : 0, '127.0.0.1', r));
    servers.push(s);
    urls[name] = `http://127.0.0.1:${s.address().port}/`;
  }
  return { urls, close: () => servers.forEach(s => s.close()) };
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  const { urls } = await serveFixtures(+process.argv[2] || 8761);
  console.log(JSON.stringify(urls));
}
