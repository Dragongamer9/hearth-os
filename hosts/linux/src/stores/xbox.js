'use strict';
// Xbox / Microsoft account sign-in (Microsoft account → Xbox Live): your gamertag and gamerpic, and the games
// you've played on Xbox. Port of the Windows host's Stores/XboxStore.cs. On Linux, Xbox games stream
// (Xbox Cloud Gaming); the played list marks which cloud games are yours and when you last played them.

const { OwnedCache, secure, https, ms } = require('./common');
const { log } = require('../util');

// The public client id that Xbox apps and many open-source Xbox tools use, with Microsoft's desktop redirect page.
const CLIENT_ID = '000000004C12AE6F';
const SCOPE = 'service::user.auth.xboxlive.com::MBI_SSL';
const REDIRECT = 'https://login.live.com/oauth20_desktop.srf';
const LOGIN_URL = `https://login.live.com/oauth20_authorize.srf?client_id=${CLIENT_ID}&response_type=code&display=touch` +
  `&scope=${encodeURIComponent(SCOPE)}&redirect_uri=${encodeURIComponent(REDIRECT)}`;
const CACHE = 'xbox-titles.json';

async function post(url, body, headers = {}) {
  const r = await fetch(url, { method: 'POST', headers, body, signal: AbortSignal.timeout(30000) });
  const text = await r.text();
  return { ok: r.ok, status: r.status, text, json: (() => { try { return JSON.parse(text); } catch { return null; } })() };
}

class XboxStore {
  constructor() {
    this.owned = OwnedCache.load(CACHE);
    this.saved = null;
    this.xsts = null;
    try { const raw = secure.load('xbox'); if (raw) this.saved = JSON.parse(raw); } catch {}
  }

  get signedIn() { return !!this.saved; }
  get gamertag() { return this.saved ? this.saved.gamertag || null : null; }
  get gamerpic() { return this.saved ? this.saved.gamerpic || null : null; }
  persist() { if (this.saved) secure.save('xbox', JSON.stringify(this.saved)); }

  async signIn(signInWindow) {
    let error = null;
    const code = await signInWindow({
      title: 'Sign in with your Microsoft account (Xbox)', url: LOGIN_URL,
      onNavigate: url => {
        if (!url.toLowerCase().startsWith(REDIRECT)) return null;
        const q = new URL(url).searchParams;
        error = q.get('error_description') || q.get('error');
        return q.get('code') || '';
      },
    });
    if (error) throw new Error('Microsoft sign-in failed: ' + error);
    if (!code) return false;
    const t = await this.msaToken({ grant_type: 'authorization_code', code });
    this.saved = { refreshToken: t.refresh };
    this.xsts = null;
    await this.xstsFrom(t.access);
    await this.loadProfile();
    this.persist();
    return true;
  }

  signOut() {
    this.saved = null; this.xsts = null;
    secure.remove('xbox');
    OwnedCache.remove(CACHE);
    this.owned = new OwnedCache();
  }

  async msaToken(form) {
    const body = new URLSearchParams({ ...form, client_id: CLIENT_ID, redirect_uri: REDIRECT, scope: SCOPE });
    const r = await post('https://login.live.com/oauth20_token.srf', body, { 'content-type': 'application/x-www-form-urlencoded' });
    const j = r.json || {};
    if (!r.ok || !j.access_token || !j.refresh_token) {
      throw new Error('Your Microsoft sign-in expired. Sign in again under Library › Stores. ' + (j.error_description || ''));
    }
    return { access: j.access_token, refresh: j.refresh_token };
  }

  /** Microsoft account token → Xbox user token → Xbox Live (XSTS) token. */
  async xstsFrom(msaAccess) {
    const hdr = { 'content-type': 'application/json', 'x-xbl-contract-version': '1' };
    let userToken = null;
    // The ticket format depends on how the token was issued; try the known forms.
    for (const ticket of [msaAccess, 't=' + msaAccess, 'd=' + msaAccess]) {
      const r = await post('https://user.auth.xboxlive.com/user/authenticate', JSON.stringify({
        RelyingParty: 'http://auth.xboxlive.com', TokenType: 'JWT',
        Properties: { AuthMethod: 'RPS', SiteName: 'user.auth.xboxlive.com', RpsTicket: ticket },
      }), hdr);
      if (r.ok && r.json && r.json.Token) { userToken = r.json.Token; break; }
    }
    if (!userToken) throw new Error("Xbox Live didn't accept this Microsoft account.");
    const x = await post('https://xsts.auth.xboxlive.com/xsts/authorize', JSON.stringify({
      RelyingParty: 'http://xboxlive.com', TokenType: 'JWT', Properties: { UserTokens: [userToken], SandboxId: 'RETAIL' },
    }), hdr);
    if (!x.ok) {
      throw new Error(x.text.includes('2148916233') ? "This Microsoft account doesn't have an Xbox profile yet. Create one at xbox.com, then try again."
        : x.text.includes('2148916238') ? 'This is a child account. An adult needs to add it to a Microsoft family first.'
        : `Xbox Live sign-in failed (${x.status}).`);
    }
    const xui = (((x.json || {}).DisplayClaims || {}).xui || [])[0] || {};
    if (!x.json.Token || !xui.uhs || !xui.xid) throw new Error('Xbox Live sign-in failed (no profile in the response).');
    this.xsts = { uhs: xui.uhs, token: x.json.Token, xuid: xui.xid, notAfter: ms(x.json.NotAfter) || Date.now() + 4 * 3600e3 };
    if (this.saved) { this.saved.xuid = xui.xid; this.saved.gamertag = this.saved.gamertag || xui.gtg || null; }
  }

  async ensureXsts() {
    if (!this.saved) throw new Error('Sign in to Xbox first.');
    if (this.xsts && this.xsts.notAfter > Date.now() + 5 * 60e3) return;
    const t = await this.msaToken({ grant_type: 'refresh_token', refresh_token: this.saved.refreshToken });
    this.saved.refreshToken = t.refresh; // Microsoft rotates refresh tokens
    this.persist();
    await this.xstsFrom(t.access);
  }

  async get(url, contract) {
    await this.ensureXsts();
    const r = await fetch(url, {
      headers: { authorization: `XBL3.0 x=${this.xsts.uhs};${this.xsts.token}`, 'x-xbl-contract-version': contract, 'accept-language': 'en-US' },
      signal: AbortSignal.timeout(30000),
    });
    if (!r.ok) throw new Error(`Xbox Live answered ${r.status}`);
    return r.json();
  }

  async loadProfile() {
    try {
      const j = await this.get(`https://profile.xboxlive.com/users/xuid(${this.xsts.xuid})/profile/settings?settings=GameDisplayPicRaw,Gamertag,ModernGamertag`, '2');
      for (const s of ((j.profileUsers || [])[0] || {}).settings || []) {
        if (!s.value) continue;
        if (s.id === 'GameDisplayPicRaw') this.saved.gamerpic = https(s.value);
        if (s.id === 'ModernGamertag') this.saved.gamertag = s.value;
        if (s.id === 'Gamertag') this.saved.gamertag = this.saved.gamertag || s.value;
      }
    } catch (e) { log('xbox profile: ' + e.message); }
  }

  /** Games you've played with this Xbox profile (title history), with Xbox art. */
  async refreshOwned() {
    await this.ensureXsts();
    if (!this.saved.gamerpic) { await this.loadProfile(); this.persist(); }
    const j = await this.get(`https://titlehub.xboxlive.com/users/xuid(${this.xsts.xuid})/titles/titlehistory/decoration/image,detail?maxItems=1000`, '2');
    const cache = new OwnedCache({ fetched: Date.now() });
    for (const t of j.titles || []) {
      if (!t.name || t.type !== 'Game') continue;
      const o = { id: t.pfn || t.titleId, title: t.name, art: {}, added: 0, lastPlayed: ms((t.titleHistory || {}).lastTimePlayed), note: null };
      const imgs = (t.images || []).filter(i => i.url);
      const img = (...types) => { for (const ty of types) { const f = imgs.find(i => i.type === ty); if (f) return f.url; } return null; };
      const c = img('Poster', 'BoxArt') || t.displayImage; if (c) o.art.cover = https(c);
      const h = img('SuperHeroArt', 'TitledHeroArt', 'BrandedKeyArt'); if (h) o.art.hero = https(h);
      const w = img('BrandedKeyArt', 'TitledHeroArt', 'SuperHeroArt'); if (w) o.art.wide = https(w);
      const l = img('Logo'); if (l) o.art.logo = https(l);
      cache.games.push(o);
    }
    cache.save(CACHE);
    this.owned = cache;
  }
}

module.exports = { XboxStore, LOGIN_URL };
