// Hearth Theater's catalog reader. main.rs opens a site you added in a hidden window of Theater's own web view (so
// your sign-ins and cookies are the browser's own) with this script, and it reads what the page itself lists, the
// way a browser shows it: schema.org JSON-LD first, then each listed page's Open Graph card, the site's RSS/Atom feeds,
// its sitemap, and last rows of "cards" (a link with a picture and a name). It reads the site's own browse pages
// (Popular, New, Movies...) with the same session, finds the site's search (OpenSearch or its search form), and
// hands back rows of titles named after the site's own sections. It never touches video, logins or paywalls: it only
// lists links the page shows; watching happens on the site itself, in the viewer.
//   window.__hearthReaderMode: "catalog" (default) or "search" (only read this page: a search's results).
//   window.__hearthReaderSink(result): tests take the result here; in Theater it goes back by hearth-theater://read.
(() => {
  'use strict';
  if (window.top !== window || window.__hearthReader) return;
  window.__hearthReader = 1;
  const MODE = window.__hearthReaderMode || 'catalog';
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const clean = s => String(s == null ? '' : s).replace(/\s+/g, ' ').trim();
  const hostOf = u => { try { return new URL(u).hostname.replace(/^www\./, ''); } catch { return ''; } };
  const HOST = hostOf(location.href);
  const sameSite = u => { const h = hostOf(u); return !!h && (h === HOST || h.endsWith('.' + HOST) || HOST.endsWith('.' + h)); };
  const abs = (u, base) => {
    if (!u || typeof u !== 'string' || /^\s*(javascript|data|mailto|tel):/i.test(u)) return '';
    try { const x = new URL(u.trim(), base); if (!/^https?:$/.test(x.protocol)) return ''; x.hash = ''; return x.href; } catch { return ''; }
  };
  // Links that are never a title.
  const NOT_TITLE = /\/(login|log-in|signin|sign-in|signup|sign-up|register|join|account|profile|settings|logout|privacy|terms|legal|cookies?|help|support|faq|about|contact|careers|jobs|press|cart|checkout|subscribe|pricing|plans|gift|redeem|download|apps?)(\/|$|\?|\.)/i;
  const BROWSE = /\b(popular|trending|new|newest|latest|recent|recently|browse|catalog|catalogue|explore|discover|top|movies?|films?|series|shows?|anime|tv|genres?|episodes|videos|simulcasts?|seasonal|airing)\b/i;
  const KIND = { Movie: 'movie', MovieSeries: 'series', TVSeries: 'series', TVSeason: 'series', CreativeWorkSeries: 'series', CreativeWorkSeason: 'series',
    TVEpisode: 'episode', Episode: 'episode', VideoObject: 'video', Clip: 'video', TVClip: 'video', MovieClip: 'video' };
  const OG_KIND = { 'video.movie': 'movie', 'video.tv_show': 'series', 'video.episode': 'episode', 'video.other': 'video', video: 'video' };
  const epCount = t => { const m = /(\d{1,4})\s*(?:episodes?|eps?\b\.?|folgen|épisodes|episodios)/i.exec(t || ''); return m ? +m[1] : 0; };
  const yearOf = s => { const m = /\b(19\d\d|20\d\d)\b/.exec(String(s || '')); return m ? m[1] : ''; };
  const imgOf = v => typeof v === 'string' ? v : Array.isArray(v) ? imgOf(v[0]) : v && typeof v === 'object' ? (v.url || v.contentUrl || v['@id'] || '') : '';
  const sources = new Set();

  async function get(url, ms = 9000) {
    if (!url || !sameSite(url)) throw new Error('other site');
    const c = new AbortController(); const t = setTimeout(() => c.abort(), ms);
    try {
      const r = await fetch(url, { credentials: 'include', signal: c.signal, redirect: 'follow' });
      if (!r.ok) throw new Error('status ' + r.status);
      return (await r.text()).slice(0, 3e6);
    } finally { clearTimeout(t); }
  }
  const parse = (text, type = 'text/html') => new DOMParser().parseFromString(text, type);

  // ------------------------------------------------------------------------------------------- 1. JSON-LD
  function ldRows(doc, base, fallback) {
    const rows = [];
    const row = name => { let r = rows.find(x => x.name === name); if (!r) rows.push(r = { name, items: [] }); return r; };
    const item = (o, name) => {
      if (!o || typeof o !== 'object') return;
      const types = [].concat(o['@type'] || []);
      const kind = types.map(t => KIND[t]).find(Boolean) || '';
      const url = abs(o.url || (typeof o['@id'] === 'string' && /^https?:/.test(o['@id']) ? o['@id'] : '') || o.mainEntityOfPage?.['@id'] || '', base);
      const title = clean(o.name || o.headline || o.alternateName);
      if (!url || !title) return;
      row(name).items.push({ title, url, image: abs(imgOf(o.image || o.thumbnailUrl || o.thumbnail), base), kind,
        episodes: +o.numberOfEpisodes || 0, year: yearOf(o.datePublished || o.startDate || o.uploadDate || o.dateCreated), desc: clean(o.description).slice(0, 300) });
    };
    const walk = (n, name, depth = 0) => {
      if (!n || typeof n !== 'object' || depth > 6) return;
      if (Array.isArray(n)) { n.forEach(x => walk(x, name, depth + 1)); return; }
      if (n['@graph']) walk(n['@graph'], name, depth + 1);
      const types = [].concat(n['@type'] || []);
      if (types.includes('ItemList') || types.includes('OfferCatalog')) {
        const nm = clean(n.name) || name;
        [].concat(n.itemListElement || []).forEach(el => {
          let t = el && (el.item || el);
          if (typeof t === 'string') t = { url: t, name: el.name };
          if (t && !t.name && el.name) t = { ...t, name: el.name };
          if (t && !t.url && el.url) t = { ...t, url: el.url };
          if (t && !t.image && el.image) t = { ...t, image: el.image };
          item(t, nm);
        });
      } else if (types.some(t => KIND[t])) item(n, name);
    };
    doc.querySelectorAll('script[type="application/ld+json"]').forEach(s => { try { walk(JSON.parse(s.textContent), fallback); } catch { } });
    if (rows.some(r => r.items.length)) sources.add('json-ld');
    return rows;
  }

  // ------------------------------------------------------------------------------------------- 2. Open Graph
  function ogOf(doc, base) {
    const meta = k => clean(doc.querySelector(`meta[property="${k}"],meta[name="${k}"]`)?.getAttribute('content'));
    const ld = ldRows(doc, base, '').flatMap(r => r.items)[0];
    const title = ld?.title || meta('og:title') || meta('twitter:title') || clean(doc.querySelector('title')?.textContent);
    const image = ld?.image || abs(meta('og:image') || meta('og:image:url') || meta('twitter:image') || meta('twitter:image:src'), base);
    const kind = ld?.kind || OG_KIND[meta('og:type')] || '';
    const oembed = doc.querySelector('link[type="application/json+oembed"]')?.getAttribute('href');
    return { title, image, kind, episodes: ld?.episodes || 0, year: ld?.year || '', desc: ld?.desc || meta('og:description').slice(0, 300), oembed: abs(oembed, base),
      og: !!(meta('og:title') || meta('og:image')) };
  }

  // ------------------------------------------------------------------------------------------- 3. RSS / Atom
  async function feedRows(doc, base) {
    const rows = [];
    const links = [...doc.querySelectorAll('link[rel~="alternate"][type*="rss"], link[rel~="alternate"][type*="atom"]')].map(l => abs(l.getAttribute('href'), base)).filter(Boolean);
    for (const href of [...new Set(links)].slice(0, 3)) {
      try {
        const x = parse(await get(href), 'application/xml');
        if (x.querySelector('parsererror')) continue;
        const tag = (el, name) => el.getElementsByTagName(name)[0];
        const text = (el, name) => clean(tag(el, name)?.textContent);
        const chan = x.querySelector('channel, feed');
        const name = chan ? clean([...chan.children].find(c => c.localName === 'title')?.textContent) : '';
        const items = [...x.querySelectorAll('item, entry')].slice(0, 60).map(e => {
          const linkEl = [...e.getElementsByTagName('link')].find(l => !l.getAttribute('rel') || l.getAttribute('rel') === 'alternate');
          const link = linkEl ? (linkEl.getAttribute('href') || clean(linkEl.textContent)) : '';
          let image = tag(e, 'media:thumbnail')?.getAttribute('url') || [...e.getElementsByTagName('media:content')].find(m => /image/.test(m.getAttribute('medium') || m.getAttribute('type') || ''))?.getAttribute('url')
            || [...e.getElementsByTagName('enclosure')].find(m => /^image\//.test(m.getAttribute('type') || ''))?.getAttribute('url') || tag(e, 'itunes:image')?.getAttribute('href') || '';
          const body = text(e, 'description') || text(e, 'content') || text(e, 'summary');
          if (!image) { const m = /<img[^>]+src=["']([^"']+)/i.exec(tag(e, 'description')?.textContent || tag(e, 'content')?.textContent || ''); if (m) image = m[1]; }
          return { title: text(e, 'title'), url: abs(link, href), image: abs(image, href), kind: '', episodes: 0,
            year: yearOf(text(e, 'pubDate') || text(e, 'published') || text(e, 'updated')), desc: clean(body.replace(/<[^>]+>/g, ' ')).slice(0, 300) };
        }).filter(i => i.title && i.url);
        if (items.length) { rows.push({ name, items }); sources.add('rss'); }
      } catch { }
    }
    return rows;
  }

  // ------------------------------------------------------------------------------------------- 5. cards
  const HEAD = 'h1,h2,h3,h4';
  const headText = h => clean(h.textContent).slice(0, 60);
  function sectionName(a, card) {
    let p = a.parentElement;
    for (let depth = 0; p && p !== a.ownerDocument.body && depth < 9; depth++, p = p.parentElement) {
      const label = p.matches('section,[role=region],ul,ol') ? clean(p.getAttribute('aria-label')) : '';
      if (label) return label.slice(0, 60);
      const cands = [...p.querySelectorAll(`:scope > :is(${HEAD}), :scope > header :is(${HEAD}), :scope > :is(div,span):first-child > :is(${HEAD})`)]
        .filter(h => !h.closest('a') && !h.contains(a) && !(card && card.contains(h)) && (h.compareDocumentPosition(a) & Node.DOCUMENT_POSITION_FOLLOWING)
          // a heading inside another card (a box with a link or a picture) is that card's name, not the section's
          && (h.parentElement === p || h.closest('header') || !h.parentElement.querySelector('a[href], img')));
      if (cands.length) { const t = headText(cands[cands.length - 1]); if (t) return t; }
      let s = p.previousElementSibling;
      for (let i = 0; s && i < 2; i++, s = s.previousElementSibling) {
        if (s.matches(HEAD)) { const t = headText(s); if (t) return t; }
        if (s.matches('header,div') && !s.querySelector('img,a img')) { const h = s.querySelector(HEAD); if (h) { const t = headText(h); if (t) return t; } }
      }
    }
    return '';
  }
  function imgSrc(img, base) {
    if (!img) return '';
    const srcset = img.getAttribute('srcset') || img.getAttribute('data-srcset') || '';
    const first = srcset.split(',').map(s => s.trim().split(/\s+/)[0]).filter(Boolean).pop();
    return abs(img.currentSrc || img.getAttribute('src') && !/^data:/.test(img.getAttribute('src')) && img.getAttribute('src') || img.getAttribute('data-src') || img.getAttribute('data-lazy-src') || img.getAttribute('data-original') || first || '', base);
  }
  function bgSrc(el, base) {
    const m = /url\((['"]?)([^'")]+)\1\)/.exec(el?.getAttribute?.('style') || '');
    return m ? abs(m[2], base) : '';
  }
  // The rows of cards on a page: links to this site with a picture and a name, grouped by the section heading they
  // sit under; lists of three or more plain links under a heading count too (their pictures come from their pages).
  function cardRows(doc, base, fallback) {
    const groups = new Map();
    const seen = new Set();
    const anchors = [...doc.querySelectorAll('a[href]')].filter(a => !a.closest('nav,footer,[role=navigation],[role=contentinfo],[aria-hidden=true]') && !(a.closest('header') && !a.closest('main,article,section')));
    for (const a of anchors) {
      const url = abs(a.getAttribute('href'), base);
      if (!url || !sameSite(url) || seen.has(url) || NOT_TITLE.test(new URL(url).pathname)) continue;
      const path = new URL(url).pathname;
      if (path === '/' || url === base) continue;
      const card = a.closest('li,article,[class*="card" i],[class*="tile" i],[class*="item" i],[class*="poster" i]') || null;
      const box = card && card.querySelectorAll('a[href]').length <= 3 ? card : a;
      const img = a.querySelector('img') || box.querySelector('img');
      const image = imgSrc(img, base) || bgSrc(a, base) || bgSrc(a.querySelector('[style*="url("]'), base) || bgSrc(box.querySelector?.('[style*="url("]'), base);
      const titleEl = box.querySelector('h1,h2,h3,h4,h5,[class*="title" i],[class*="name" i]');
      const title = clean(a.getAttribute('title') || a.getAttribute('aria-label') || (titleEl && !titleEl.contains(a) ? titleEl.textContent : '') || a.textContent || img?.getAttribute('alt'));
      if (!title || title.length < 2 || title.length > 140) continue;
      seen.add(url);
      const name = sectionName(a, card) || fallback;
      if (!groups.has(name)) groups.set(name, []);
      groups.get(name).push({ title: title.replace(/\s*\d{1,4}\s*(episodes?|eps?\.?)\s*$/i, '').trim() || title, url, image, kind: '',
        episodes: epCount(box.textContent), year: '', desc: '', linkOnly: !image });
    }
    const rows = [];
    for (const [name, items] of groups) {
      const pics = items.filter(i => !i.linkOnly);
      // Plain links only count as a row when they are a list (3+) under a real heading; pictures need 2+.
      const keep = pics.length >= 2 ? items.filter(i => !i.linkOnly || items.length - pics.length >= 3) : items.length >= 3 && name !== fallback ? items : [];
      if (keep.length) rows.push({ name, items: keep });
    }
    if (rows.length) sources.add('cards');
    return rows;
  }

  // ------------------------------------------------------------------------------------------- 4. sitemap
  async function sitemapItems() {
    const origin = location.origin;
    let maps = [];
    try { maps = (await get(origin + '/robots.txt', 5000)).split('\n').map(l => /^\s*sitemap:\s*(\S+)/i.exec(l)?.[1]).filter(Boolean).map(u => abs(u, origin)).filter(sameSite); } catch { }
    if (!maps.length) maps = [origin + '/sitemap.xml'];
    const locs = [];
    for (const m of maps.slice(0, 2)) {
      try {
        const x = parse(await get(m), 'application/xml');
        if (x.querySelector('sitemapindex')) {
          const kids = [...x.querySelectorAll('sitemap > loc')].map(l => clean(l.textContent));
          const pick = kids.filter(k => /video|show|series|movie|film|anime|title|episode|tv/i.test(k)).concat(kids).slice(0, 2);
          for (const k of pick) { try { locs.push(...[...parse(await get(abs(k, m)), 'application/xml').querySelectorAll('url > loc')].map(l => abs(clean(l.textContent), k))); } catch { } }
        } else locs.push(...[...x.querySelectorAll('url > loc')].map(l => abs(clean(l.textContent), m)));
      } catch { }
    }
    const urls = [...new Set(locs)].filter(u => u && sameSite(u) && /\/(watch|title|titles|movie|movies|film|films|series|show|shows|anime|episode|episodes|video|videos|tv)\b/i.test(new URL(u).pathname) && !NOT_TITLE.test(u));
    if (urls.length) sources.add('sitemap');
    return urls.slice(0, 36).map(url => ({ title: '', url, image: '', kind: '', episodes: 0, year: '', desc: '', linkOnly: true }));
  }

  // ------------------------------------------------------------------------------------------- search
  async function findSearch(doc, base) {
    const os = doc.querySelector('link[rel="search"][type="application/opensearchdescription+xml"]');
    if (os) {
      try {
        const x = parse(await get(abs(os.getAttribute('href'), base)), 'application/xml');
        const u = [...x.getElementsByTagName('Url')].find(u => /html/.test(u.getAttribute('type') || ''));
        const t = u?.getAttribute('template');
        if (t && t.includes('{searchTerms}')) { sources.add('opensearch'); return t; }
      } catch { }
    }
    for (const f of doc.querySelectorAll('form')) {
      if ((f.getAttribute('method') || 'get').toLowerCase() !== 'get') continue;
      const inp = f.querySelector('input[type="search"], input[name="q"], input[name="query"], input[name="s"], input[name="search"], input[name="keyword"], input[name="term"]');
      if (!inp || !inp.getAttribute('name')) continue;
      const action = abs(f.getAttribute('action') || base, base);
      if (!action || !sameSite(action)) continue;
      const u = new URL(action); u.search = '';
      return `${u.href}?${encodeURIComponent(inp.getAttribute('name'))}={searchTerms}`;
    }
    return '';
  }

  // ------------------------------------------------------------------------------------------- reading
  function hush() {
    document.querySelectorAll('video,audio').forEach(m => { try { m.muted = true; m.pause(); } catch { } });
  }
  document.addEventListener('play', e => { try { e.target.muted = true; e.target.pause(); } catch { } }, true);
  async function settle(ms) {
    if (document.readyState !== 'complete') await new Promise(r => addEventListener('load', r, { once: true }));
    hush();
    await sleep(ms);
    // Lazy pictures and rows load as the page scrolls.
    for (let i = 1; i <= 4; i++) { scrollTo(0, document.body.scrollHeight * i / 4); await sleep(250); }
    scrollTo(0, 0); hush();
  }
  // Rows merged by name, titles once per row.
  function merge(into, rows) {
    for (const r of rows) {
      let t = into.find(x => x.name === r.name);
      if (!t) into.push(t = { name: r.name, items: [] });
      for (const i of r.items) {
        const have = t.items.find(x => x.url === i.url);
        if (!have) t.items.push(i);
        else for (const k of ['image', 'kind', 'episodes', 'year', 'desc']) if (!have[k] && i[k]) have[k] = i[k];
      }
    }
  }
  async function pool(list, n, fn) {
    let i = 0;
    await Promise.all(Array.from({ length: n }, async () => { while (i < list.length) { const x = list[i++]; try { await fn(x); } catch { } } }));
  }
  // Titles found without a picture or a name: their own page's JSON-LD / Open Graph (or oEmbed) fills them in.
  async function enrich(items, max) {
    const need = items.filter(i => i.linkOnly || !i.image || !i.title).slice(0, max);
    await pool(need, 4, async i => {
      const d = parse(await get(i.url, 7000));
      const o = ogOf(d, i.url);
      if (o.og) sources.add('open-graph');
      if (!o.image && o.oembed) { try { const j = JSON.parse(await get(o.oembed, 5000)); o.image = abs(j.thumbnail_url, i.url); o.title ||= clean(j.title); sources.add('oembed'); } catch { } }
      if (!i.title || i.linkOnly) i.title = o.title || i.title;
      i.image ||= o.image; i.kind ||= o.kind; i.episodes ||= o.episodes; i.year ||= o.year; i.desc ||= o.desc;
      if (o.title || o.image) i.linkOnly = false;
    });
  }
  const signedOut = () => !!document.querySelector('input[type="password"]');

  async function catalog() {
    await settle(1800);
    const base = location.href;
    const rows = [];
    merge(rows, ldRows(document, base, ''));
    merge(rows, cardRows(document, base, ''));
    merge(rows, await feedRows(document, base));
    const search = await findSearch(document, base);
    // The site's own browse pages (Popular, New, Movies, Genres...), read with the same session.
    const browse = [];
    // Menu links (nav, header) named or addressed like a browse page, and "See all" / "View all" / "More" links.
    for (const a of document.querySelectorAll('a[href]')) {
      const u = abs(a.getAttribute('href'), base); const text = clean(a.textContent || a.getAttribute('aria-label'));
      if (!u || !sameSite(u) || u === base || NOT_TITLE.test(u) || a.querySelector('img') || browse.some(b => b.url === u)) continue;
      const menu = a.closest('nav,header,[role=navigation],[role=menu],[role=menubar],[role=tablist]');
      const more = /^(see|view|show) (all|more)\b|^more\b|^all\b/i.test(text);
      if (text && text.length <= 30 && ((menu && (BROWSE.test(text) || BROWSE.test(new URL(u).pathname))) || (more && BROWSE.test(new URL(u).pathname)))) {
        browse.push({ url: u, text: more ? (sectionName(a, null) || text) : text });
      }
      if (browse.length >= 5) break;
    }
    await pool(browse, 3, async b => {
      const d = parse(await get(b.url));
      const r = ldRows(d, b.url, b.text).concat(cardRows(d, b.url, b.text));
      merge(rows, r);
      merge(rows, await feedRows(d, b.url));
    });
    let all = rows.flatMap(r => r.items);
    if (new Set(all.filter(i => !i.linkOnly).map(i => i.url)).size < 12) {
      const sm = await sitemapItems();
      const known = new Set(all.map(i => i.url));
      merge(rows, [{ name: '', items: sm.filter(i => !known.has(i.url)) }]);
      all = rows.flatMap(r => r.items);
    }
    await enrich(all, 40);
    const out = rows.map(r => ({ name: r.name, items: r.items.filter(i => i.title && !(i.linkOnly && !i.image && !i.desc && r.name === '')).map(({ linkOnly, ...i }) => i) })).filter(r => r.items.length);
    return { rows: out, search, signin: signedOut() && out.length < 2, sources: [...sources], title: clean(document.title) };
  }
  async function searchPage() {
    await settle(1200);
    const rows = [];
    merge(rows, ldRows(document, location.href, ''));
    merge(rows, cardRows(document, location.href, ''));
    const items = rows.flatMap(r => r.items);
    await enrich(items, 12);
    return { items: items.filter(i => i.title).map(({ linkOnly, ...i }) => i) };
  }

  // ------------------------------------------------------------------------------------------- handing it back
  // In Theater the result goes out in pieces as hearth-theater://read/<n>/<count>?d=... "navigations" that main.rs
  // catches and cancels; it answers each piece with __hearthReaderAck() before the next is sent.
  function send(obj) {
    if (typeof window.__hearthReaderSink === 'function') { window.__hearthReaderSink(obj); return; }
    // Cut by characters (not bytes), so no piece splits a character; each piece is encoded on its own.
    const chars = Array.from(JSON.stringify(obj)), size = 16000, parts = [];
    for (let i = 0; i < chars.length; i += size) parts.push(encodeURIComponent(chars.slice(i, i + size).join('')));
    if (!parts.length) parts.push('');
    let n = 0;
    window.__hearthReaderAck = () => { if (n < parts.length) { const k = n++; location.href = `hearth-theater://read/${k}/${parts.length}?d=${parts[k]}`; } };
    window.__hearthReaderAck();
  }
  (MODE === 'search' ? searchPage() : catalog())
    .then(send, e => send({ rows: [], items: [], error: String(e && e.message || e) }));
})();
