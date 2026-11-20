// Hearth Games: "Sharper graphics" (vkBasalt; hearth-core vkbasalt.rs) on a game's page, and the launch options to copy
// for Steam's and Faugus Launcher's own games on the Stores tab. Off unless you turn it on; every change is saved at
// once through Hearth's per-game options (game_options.rs) and the game's vkBasalt.conf is made from them. Works with
// the mouse (switches, the strength slider) and the controller (A on a switch, − and + beside the slider).
// games.js calls VKB.mount(game, dialog) from a game's page and VKB.stores(cards) from the Stores tab.
'use strict';
const VKB = (() => {
  const { esc } = KIT;
  const V = {};
  // Its look, beside games.html's own (.extras, .sw, .line).
  const css = document.createElement('style');
  css.textContent = `
.vkb { background:rgba(255,255,255,.04); border:1px solid rgba(255,255,255,.08); border-radius:min(14px, var(--corner, 14px)); padding:.8em 1em; margin:0 0 1em; }
.vkb h3 { margin:0 0 .5em; font-size:1em; }
.vkb .line { display:flex; align-items:center; gap:1em; padding:.5em 0; border-top:1px solid rgba(255,255,255,.06); }
.vkb .line:first-of-type { border-top:0; }
.vkb .line > span:first-child { flex:1; display:flex; flex-direction:column; }
.vkb .line small { opacity:.7; font-size:.82em; margin-top:.15em; }
.vkb .line.dim > span:first-child { opacity:.5; }
.vkb .sw[disabled], .vkb .btn[disabled], .vkb input[disabled] { opacity:.4; cursor:default; }
.vkb-slider { display:flex; align-items:center; gap:.5em; flex:none; }
.vkb-slider .btn { min-width:2.7em; justify-content:center; font-size:1.1em; padding:.3em .6em; }
.vkb-slider input { width:min(14em, 32vw); accent-color:var(--accent, #f0a840); height:2em; border-radius:99px; }
.vkb-pct { min-width:3.2em; text-align:right; font-variant-numeric:tabular-nums; }
.vkb-key { display:inline-block; padding:.05em .5em; border-radius:6px; border:1px solid var(--line); background:var(--panel3); font:inherit; font-weight:700; }
.vkb-pad { display:block; margin-top:.35em; }
.vkb-pad b { display:inline-grid; place-items:center; min-width:1.5em; height:1.5em; border-radius:50%; background:var(--panel3); color:var(--text); font-size:.85em; }
.vkb-missing { color:var(--flame, #f0a840); }
.vkb-card { grid-column:1/-1; }
.vkb-card code { display:block; margin-top:.3em; padding:.45em .6em; border-radius:8px; background:var(--bg2); border:1px solid var(--line); font-size:.85em; word-break:break-all; user-select:all; }
.vkb-copy { display:flex; gap:.8em; align-items:center; padding:.45em 0; }
.vkb-copy > span { flex:1; min-width:0; display:flex; flex-direction:column; }
.vkb-copy small { color:var(--muted); font-size:.8em; }`;
  document.head.appendChild(css);
  // Calls go to the app; the preview in a plain browser (no app) answers from V.demo.
  const call = (method, args = {}) => KIT.live ? KIT.invoke('call', { method, args }) : Promise.resolve(V.demo(method, args));
  const sw = (on, what, label, off) => `<button class="sw f ${on ? 'on' : ''}" data-vkb="${what}" role="switch" aria-checked="${!!on}" aria-label="${esc(label)}" ${off ? 'disabled' : ''}><i></i></button>`;
  const key = k => `<kbd class="vkb-key">${esc(String(k || 'Home').replace('_', ' '))}</kbd>`;

  // Copy to the clipboard: the web view's own, or the old way (a selected text box) where that isn't allowed.
  async function copy(text) {
    try { await navigator.clipboard.writeText(text); return true; } catch (_) { /* below */ }
    const t = document.createElement('textarea'); t.value = text; t.setAttribute('readonly', ''); t.style.cssText = 'position:fixed;left:-99em';
    document.body.appendChild(t); t.select();
    let ok = false; try { ok = document.execCommand('copy'); } catch (_) { ok = false; }
    t.remove(); return ok;
  }

  // ------------------------------------------------------------------ a game's page
  const canHave = g => g && g.installed && g.id !== 'battlenet:app';
  function html(g, s) {
    if (s.error) return `<h3>Sharper graphics</h3><p class="bad small">${esc(s.error)}</p>`;
    const missing = !s.available;
    const dim = !s.on || missing;
    const pct = Math.round(s.strength ?? 40);
    const why = missing
      ? `<p class="note vkb-missing">vkBasalt isn't installed on this device, so this can't be turned on here.${s.on ? ' It stays saved as on and works once vkBasalt is installed.' : ''}</p>` : '';
    const via = g.source === 'battlenet' ? '<p class="note">Battle.net games start through Faugus Launcher: Hearth passes this on to the game.</p>' : '';
    const pad = KIT.usingPad();
    return `<h3>Sharper graphics</h3>
      <div class="line"><span><b>Sharpen the picture</b><small>vkBasalt sharpens the picture while you play (AMD's FidelityFX CAS). It works with Vulkan and DirectX games through Proton.</small></span>
        ${sw(s.on, 'on', `Sharper graphics ${s.on ? 'on' : 'off'}`, missing && !s.on)}</div>
      <div class="line vkb-sub ${dim ? 'dim' : ''}"><span><b>Sharpening strength</b><small>From a light touch to as sharp as it goes</small></span>
        <span class="vkb-slider"><button class="btn f" data-vkb="minus" aria-label="Less sharpening" ${dim ? 'disabled' : ''}>−</button>
        <input type="range" class="f" data-vkb="strength" min="0" max="100" step="5" value="${pct}" aria-label="Sharpening strength" ${dim ? 'disabled' : ''}>
        <button class="btn f" data-vkb="plus" aria-label="More sharpening" ${dim ? 'disabled' : ''}>+</button><b class="vkb-pct">${pct} %</b></span></div>
      <div class="line vkb-sub ${dim ? 'dim' : ''}"><span><b>Smoother edges</b><small>SMAA: smooths jagged edges a little</small></span>${sw(s.smaa, 'smaa', 'Smoother edges', dim)}</div>
      <div class="line vkb-sub ${dim ? 'dim' : ''}"><span><b>Richer colour</b><small>A light vibrance and colour boost</small></span>${sw(s.vibrance, 'vibrance', 'Richer colour', dim)}</div>
      <p class="note">While you play, press ${key(s.key)} to turn the effects off and on.${pad ? ' <span class="vkb-pad"><b>A</b> turns a switch on or off · <b>−</b> <b>+</b> change the strength</span>' : ''}</p>
      ${why}${via}`;
  }
  async function draw(g, box, s) {
    box.innerHTML = html(g, s);
    box.dataset.state = JSON.stringify({ on: !!s.on, strength: s.strength, smaa: !!s.smaa, vibrance: !!s.vibrance, available: !!s.available });
  }
  async function save(g, box, s, change) {
    try { const n = await call('game.setVkbasalt', Object.assign({ id: g.id }, change)); Object.assign(s, n); }
    catch (e) { KIT.toast(KIT.err(e)); }
    const had = document.activeElement?.dataset?.vkb;
    draw(g, box, s);
    if (had) box.querySelector(`[data-vkb="${had}"]:not([disabled])`)?.focus() || box.querySelector('[data-vkb="on"]')?.focus();
    if ('on' in change) KIT.toast(s.on ? `Sharper graphics on for ${g.title}: from its next start` : `Sharper graphics off for ${g.title}`);
  }
  V.mount = async (g, d) => {
    if (!canHave(g) || !d) return;
    const row = d.querySelector('.row');
    const box = document.createElement('section');
    box.className = 'vkb';
    box.innerHTML = '<h3>Sharper graphics</h3><p class="msg small">Looking…</p>';
    if (row) row.parentNode.insertBefore(box, row); else (d.querySelector('.dlg') || d).appendChild(box);
    let s;
    try { s = await call('game.vkbasalt', { id: g.id }); } catch (e) { s = { error: KIT.err(e) }; }
    if (!document.body.contains(box)) return;
    draw(g, box, s);
    box.addEventListener('click', e => {
      const b = e.target.closest('button[data-vkb]'); if (!b || b.disabled || s.error) return;
      e.stopPropagation();
      const w = b.dataset.vkb;
      if (w === 'on' || w === 'smaa' || w === 'vibrance') return save(g, box, s, { [w]: !s[w] });
      const n = Math.max(0, Math.min(100, Math.round(s.strength ?? 40) + (w === 'plus' ? 10 : -10)));
      if (n !== s.strength) save(g, box, s, { strength: String(n) });
    });
    // The slider: saved when you let go (not while it moves).
    box.addEventListener('input', e => { const r = e.target.closest('input[data-vkb="strength"]'); if (r) box.querySelectorAll('.vkb-pct').forEach(p => { p.textContent = `${r.value} %`; }); });
    box.addEventListener('change', e => { const r = e.target.closest('input[data-vkb="strength"]'); if (r) save(g, box, s, { strength: String(r.value) }); });
  };

  // ------------------------------------------------------------------ Stores tab: Steam's and Faugus Launcher's own games
  V.stores = async cards => {
    if (!cards || cards.querySelector('.vkb-card')) return;
    const c = document.createElement('div');
    c.className = 'panel card vkb-card';
    c.innerHTML = '<h3>Sharper graphics in other launchers</h3><p class="msg small">Looking…</p>';
    cards.appendChild(c);
    let i = V.info;   // asked once a window: the Stores tab is drawn again often
    try { if (!i) i = V.info = await call('vkbasalt.info'); } catch (e) { c.innerHTML = `<h3>Sharper graphics in other launchers</h3><p class="bad small">${esc(KIT.err(e))}</p>`; return; }
    const line = (label, text, what, sub) => `<div class="vkb-copy"><span><b>${esc(label)}</b>${sub ? `<small>${esc(sub)}</small>` : ''}<code>${esc(text)}</code></span><button class="btn f" data-vkbcopy="${what}">Copy</button></div>`;
    c.innerHTML = `<h3>Sharper graphics in other launchers</h3>
      <p class="msg small">Games here have it on their own page. For Steam's games and the games you added in Faugus Launcher, add this to the game's launch options yourself: Hearth never changes Steam's or Faugus' settings.</p>
      ${i.available ? '' : '<p class="note vkb-missing">vkBasalt isn\'t installed on this device, so these do nothing here yet.</p>'}
      ${line('Steam: Properties › Launch options', i.steamOption, 'steam')}
      ${line('Steam, with Hearth\'s sharpening', i.steamOptionHearth, 'steam-hearth', 'Uses Hearth\'s settings file (sharpening at 40 %), made when you copy this')}
      ${line('Faugus Launcher: the game › Edit › Launch arguments', i.faugusArg, 'faugus', 'Faugus Launcher has its own field for this; Hearth can\'t see the games you add there')}
      <p class="note">While you play, press ${key(i.key)} to turn the effects off and on.</p>`;
    c.addEventListener('click', async e => {
      const b = e.target.closest('[data-vkbcopy]'); if (!b) return;
      e.stopPropagation();
      let info = i;
      if (b.dataset.vkbcopy === 'steam-hearth') { try { info = V.info = await call('vkbasalt.shared'); } catch (err) { return KIT.toast(KIT.err(err)); } }
      const text = { steam: info.steamOption, 'steam-hearth': info.steamOptionHearth, faugus: info.faugusArg }[b.dataset.vkbcopy];
      KIT.toast(await copy(text) ? 'Copied: paste it in the game\'s launch options' : 'Couldn\'t copy: select the text and copy it yourself');
    });
  };

  // ------------------------------------------------------------------ preview without the app (CI's pictures)
  const P = new URLSearchParams(location.search);
  const demoState = { on: P.get('vkbon') !== '0', strength: 60, smaa: true, vibrance: false, key: 'Home' };
  V.demo = (m, a) => {
    const available = P.get('missing') !== '1';
    const conf = `/home/you/.config/nimbus/vkbasalt/${String(a.id || 'game').replace(/[^A-Za-z0-9._-]/g, '_')}.conf`;
    const st = () => Object.assign({ available, conf, steamOption: 'ENABLE_VKBASALT=1 %command%', faugusArg: 'ENABLE_VKBASALT=1' }, demoState);
    if (m === 'game.vkbasalt') return st();
    if (m === 'game.setVkbasalt') { for (const k of ['on', 'smaa', 'vibrance']) if (k in a) demoState[k] = !!a[k]; if ('strength' in a) demoState.strength = +a.strength; return st(); }
    if (m === 'vkbasalt.info' || m === 'vkbasalt.shared') {
      const f = '/home/you/.config/nimbus/vkbasalt/steam.conf';
      return { available, key: 'Home', steamOption: 'ENABLE_VKBASALT=1 %command%', faugusArg: 'ENABLE_VKBASALT=1',
               steamOptionHearth: `ENABLE_VKBASALT=1 VKBASALT_CONFIG_FILE="${f}" %command%` };
    }
    return {};
  };
  // ?tab=stores&vkbcard=1: the Stores tab scrolled to the launch options (&pad=1: its first Copy focused).
  if (!KIT.live && P.get('vkbcard')) {
    let n = 0;
    const show = () => {
      const c = document.querySelector('.vkb-card [data-vkbcopy]');
      if (!c) { if (++n < 80) setTimeout(show, 100); return; }
      if (P.get('pad') === '1') { KIT.pad = true; c.focus(); }
      document.querySelector('.vkb-card').scrollIntoView({ block: 'end' });
      document.body.dataset.vkbReady = '1';
    };
    setTimeout(show, 300);
  }
  // ?vkb=<game id>: that game's page scrolled to Sharper graphics (&pad=1: as with a controller, the switch focused).
  if (!KIT.live && P.get('vkb')) {
    let tries = 0;
    const open = () => {
      const g = typeof game === 'function' ? game(P.get('vkb')) : null;   // games.js's
      if (!g) { if (++tries < 50) setTimeout(open, 100); return; }
      if (P.get('pad') === '1') KIT.pad = true;
      openGame(g);
      const wait = () => {
        const box = document.querySelector('.vkb [data-vkb="on"]');
        if (!box) { if (++tries < 100) setTimeout(wait, 100); return; }
        document.querySelector('.vkb').scrollIntoView({ block: 'end' });
        if (P.get('pad') === '1') { box.focus(); document.querySelector('.vkb').scrollIntoView({ block: 'end' }); }
        document.body.dataset.vkbReady = '1';
      };
      wait();
    };
    setTimeout(open, 150);
  }
  return V;
})();
