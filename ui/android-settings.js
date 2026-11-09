// Hearth for Android: the device's own settings inside Hearth's Settings (hosts/android AndroidSettings.kt,
// docs/ANDROID.md "Settings"). Loaded by index.html after its main script; does nothing unless the page runs in Hearth
// for Android (ANDROID) and its host answers android.settings.get. index.html calls in through four guarded hooks:
// androidPage() → anPage(), adjustSlider() → anSlide() for 'an:' sliders, act() → anAct() for /^an[A-Z]/ actions, and
// gameOptions() → anGameItems().
//
// Inline where Android allows it (volumes, mute, brightness, auto-brightness, screen timeout, auto-rotate, Do Not
// Disturb, Wi-Fi's network and nearby networks, joining one, Bluetooth's paired devices, pairing, turning Bluetooth on,
// battery and battery saver's state, running in the background). Everything else opens Android's panel (over Hearth)
// or its exact page, from where B (Back) comes straight back to Hearth. Nothing here changes a setting by itself.
(function(){
  if (typeof ANDROID === 'undefined' || !ANDROID) return;
  const AS = window.AS = {s: null, loading: false, nets: null, scanning: false, askedOnce: {}};
  const ANP = window.ANP = {};   // permission texts (from the host's state, SettingsLogic.PERMS)

  function anLoad(){
    if (!host || AS.loading) return;
    AS.loading = true;
    call('android.settings.get').then(r => { AS.s = r; AS.loading = false; anRefresh(); }).catch(() => { AS.loading = false; AS.s = AS.s || false; });
  }
  function anRefresh(){ if (S.view === 'settings') render(null, true); }
  window.anLoad = anLoad;

  // The host tells the page what changed when Hearth is back from one of Android's pages, panels or dialogs.
  if (host) host.addEventListener('message', e => {
    const m = e.data || {};
    if (m.type === 'event' && m.name === 'android.settings' && m.data) { AS.s = m.data; anRefresh(); }
  });

  // ---------------------------------------------------------------- pieces
  const ic = n => I[n] || I.gear;
  const row = (k, icon, title, sub, act, extra, right) => `<button class="row" data-f data-k="${k}" data-act="${act}" ${extra||''}><span class="ricon">${ic(icon)}</span><div class="rt"><b>${title}</b>${sub?`<small>${sub}</small>`:''}</div>${right==null?'<div class="rv">›</div>':right}</button>`;
  const page = (k, icon, title, sub, p, id) => row(k, icon, title, sub, 'anPage', `data-page="${p}"${id?` data-id="${esc(id)}"`:''}`);
  const panel = (k, icon, title, sub, p) => row(k, icon, title, sub, 'anPanel', `data-panel="${p}"`);
  const sw = (k, title, sub, key, on) => `<button class="row" data-f data-k="${k}" data-act="anToggle" data-key="${key}"><div class="rt"><b>${title}</b>${sub?`<small>${sub}</small>`:''}</div><div class="sw ${on?'on':''}"></div></button>`;
  const info = (k, title, val) => `<button class="row" data-f data-k="${k}"><div class="rt"><b>${title}</b></div><div class="rv">${esc(val)}</div></button>`;
  const ask = (perm, k, title, sub) => row(k, 'lock', title, sub || 'Android asks once', 'anAsk', `data-perm="${perm}"`);
  const slider = (key, icon, label, v, min, max, step, shown) => {
    const pct = max > min ? (v - min) / (max - min) * 100 : 0;
    return `<div class="ctl" data-f data-k="an-sl-${key.replace(/\./g,'-')}" data-slider="an:${key}" data-min="${min}" data-max="${max}" data-step="${step}" data-unit="" data-v="${v}">
      <div class="lab">${ic(icon)}${label}<span>${shown}</span></div>
      <div class="slider"><i style="width:${pct}%"></i><b style="left:${pct}%"></b></div></div>`;
  };
  const volShown = (v, max) => max > 0 ? Math.round(v / max * 100) + '%' : '—';
  const STREAMS = [['media', 'vol', 'Media volume'], ['ring', 'bell', 'Ring volume'], ['notification', 'note', 'Notification volume'], ['alarm', 'clock', 'Alarm volume']];
  const DNDS = [['all', 'Off'], ['priority', 'Priority only'], ['alarms', 'Alarms only'], ['none', 'Total silence']];
  const tLabel = s => s <= 0 ? 'Never' : s < 60 ? s + ' seconds' : s < 3600 ? (s / 60 === 1 ? '1 minute' : Math.round(s / 60) + ' minutes') : Math.round(s / 3600) + (s >= 7200 ? ' hours' : ' hour');
  const bars = n => '▂▄▆█'.slice(0, Math.max(0, Math.min(4, n || 0))) || '—';

  // ---------------------------------------------------------------- the pages
  // Returns the page's HTML, or null to keep index.html's own Android page (before the host answers, or an older host).
  window.anPage = function(t, body){
    if (!['wifi', 'bluetooth', 'display', 'power', 'system'].includes(t)) return null;
    if (AS.s == null) { anLoad(); return null; }
    if (!AS.s) return null;
    const s = AS.s, p = s.perms || {};
    if (t === 'wifi') {
      const w = s.wifi || {};
      let h = sgrid(info('an-wifi', 'Wi-Fi', !w.on ? 'Off' : w.ssid && w.ssid !== 'Wi-Fi' ? `Connected · ${w.ssid} · ${bars(w.bars)}` : w.ssid ? `Connected · ${bars(w.bars)}` : 'On · not connected')
        + panel('an-wpanel', 'wifi', 'Wi-Fi', 'Turn it on or off and pick a network, over Hearth', 'wifi')
        + panel('an-inet', 'globe', 'Internet and mobile data', 'Wi-Fi, mobile data and hotspots, over Hearth', 'internet')
        + (w.canSeeName ? '' : ask('location', 'an-wloc', 'Show the network\'s name and nearby networks', 'Android only tells apps with location permission (asked once)'))
        + (s.nfc && s.nfc.supported ? panel('an-nfc', 'phone', 'NFC', s.nfc.on ? 'On' : 'Off', 'nfc') : '')
        + page('an-wset', 'wifi', 'Wi-Fi settings', 'Saved networks and passwords, in Android\'s settings', 'wifi')
        + page('an-air', 'plane', 'Airplane mode', s.airplane ? 'On · in Android\'s settings' : 'In Android\'s settings', 'airplane'));
      if (w.canSeeName) {
        h += shead('Networks nearby', AS.scanning) + sgrid(
          row('an-wscan', 'refresh', AS.scanning ? 'Looking…' : AS.nets ? 'Look again' : 'Find networks', 'Join one with a password typed in Hearth', 'anScan', '', '')
          + (AS.nets || []).map((n, i) => row('an-net' + i, n.security === 'open' ? 'wifi' : 'lock', esc(n.ssid), `${bars(n.bars)} · ${n.security === 'open' ? 'Open' : n.security === 'enterprise' ? 'Work or school (in Android\'s settings)' : 'Password'}${w.ssid === n.ssid ? ' · Connected' : ''}`, 'anJoin', `data-i="${i}"`)).join(''));
        if (AS.nets && !AS.nets.length) h += note('No networks found. Is Wi-Fi on?');
      }
      return h + note('Android doesn\'t let apps switch Wi-Fi, mobile data, NFC or airplane mode themselves: Hearth opens Android\'s panel for those, over Hearth. Press B to come back.');
    }
    if (t === 'bluetooth') {
      const b = s.bt || {};
      if (!b.supported) return sgrid(info('an-bt0', 'Bluetooth', 'Not on this device'));
      let h = sgrid(b.on ? info('an-bton', 'Bluetooth', 'On') : row('an-bton', 'bt', 'Turn Bluetooth on', 'Android asks you to confirm', 'anBtOn')
        + '');
      h += shead('Paired devices') + sgrid(
        (!b.canList ? ask('btConnect', 'an-btl', 'Show your paired devices', 'Android asks once for "Nearby devices"')
          : (b.paired || []).length ? b.paired.map((d, i) => info('an-btd' + i, esc(d.name), d.kind === 'controller' ? 'Controller' : d.kind === 'audio' ? 'Audio' : d.kind === 'input' ? 'Keyboard or mouse' : 'Paired')).join('')
          : info('an-btd', 'Paired devices', b.on ? 'None yet' : 'Bluetooth is off'))
        + row('an-pair', 'pad', 'Pair a controller or headphones', 'Android\'s pairing list opens over Hearth', 'anBtPair')
        + page('an-bt', 'bt', 'Bluetooth settings', 'Forget or rename devices, in Android\'s settings', 'bluetooth'));
      return h + note('To pair a controller, put it in pairing mode (Xbox: hold the pair button; PlayStation: hold Share and the PS button; Switch Pro: hold the sync button), then pick it in the list Android shows.');
    }
    if (t === 'display') {
      const a = (s.audio || {}).streams || {}, d = s.display || {}, z = s.dnd || {};
      let h = shead('Sound') + STREAMS.filter(x => a[x[0]]).map(([k, icn, l]) => slider('vol.' + k, icn, l, a[k].v, 0, a[k].max, 1, volShown(a[k].v, a[k].max))).join('')
        + sgrid(`<button class="row" data-f data-k="a-mute" data-act="anToggle" data-key="mute"><span class="ricon">${a.media && a.media.muted ? I.mute : I.vol}</span><div class="rt"><b>Mute media</b></div><div class="sw ${a.media && a.media.muted ? 'on' : ''}"></div></button>`
          + (z.access ? row('an-dnd', 'moon', 'Do Not Disturb', (DNDS.find(x => x[0] === z.filter) || DNDS[0])[1], 'anDnd', '', `<div class="rv">${(DNDS.find(x => x[0] === z.filter) || DNDS[0])[1]} ›</div>`)
            : ask('dnd', 'an-dnd', 'Do Not Disturb', 'Android asks once for "Do Not Disturb access"'))
          + panel('an-volp', 'sliders', 'All volumes', 'Android\'s volume panel, over Hearth', 'volume')
          + page('an-sound', 'vol', 'Sound settings', 'Ringtones, vibration and sound effects', 'sound'));
      h += shead('Display') + (d.canWrite ? '' : sgrid(ask('writeSettings', 'an-wsperm', 'Let Hearth change brightness, timeout and rotation', 'Android asks once for "Modify system settings"')))
        + slider('brightness', 'sun', 'Brightness', d.brightness || 0, 0, 100, 5, (d.brightness || 0) + '%')
        + sgrid(sw('an-autob', 'Adaptive brightness', 'Android sets the brightness for the light around you', 'autoBrightness', d.auto)
          + row('an-timeout', 'clock', 'Screen timeout', '', 'anTimeout', '', `<div class="rv">${tLabel(d.timeoutSec)} ›</div>`)
          + sw('an-rotate', 'Auto-rotate', 'The screen turns with the device (Hearth works both ways on a tablet)', 'autoRotate', d.rotate)
          + page('an-dark', 'moon', 'Dark mode', 'Hearth is always dark; this is for the rest of Android', 'darkmode')
          + page('an-text', 'text', 'Text and display size', 'Font size and screen zoom, in Android\'s settings', 'textsize')
          + page('an-disp', 'display', 'Display settings', s.samsung ? 'Eye comfort shield, screen mode and more (One UI)' : 'Night light, colours and more', 'display'));
      return h;
    }
    if (t === 'power') {
      const b = s.battery || {};
      return sgrid(info('s-batt', 'Battery', b.pct == null ? '—' : `${b.pct}% · ${b.charging ? 'Charging' : 'On battery'}`)
        + page('an-saver', 'bolt', 'Battery saver', (b.saver ? 'On' : 'Off') + ' · switched in Android\'s settings', 'saver')
        + (b.background ? info('an-bg', 'Runs in the background', 'Allowed')
          : ask('batteryOpt', 'an-bg', 'Let Hearth run in the background', s.samsung ? 'So One UI\'s battery saving doesn\'t close Hearth or Hearth Linux' : 'So Android doesn\'t close Hearth or Hearth Linux'))
        + page('an-batt', 'bolt', 'Battery settings', 'Usage and charging', 'battery'))
        + note(s.samsung ? 'On Samsung: if Hearth still closes in the background, open Battery settings › Background usage limits and take Hearth off the sleeping apps.' : 'Android doesn\'t let apps switch battery saver themselves: it opens in Android\'s settings.');
    }
    if (t === 'system') {
      const a = AND.s || {};
      return shead('Home app') + sgrid(page('an-home', 'home', a.home && a.home.isDefault ? 'Hearth is your home app' : 'Use Hearth as your home app', 'The Home button opens Hearth. Change it back in Android\'s settings at any time.', 'home')
          + ((s.battery || {}).background ? '' : ask('batteryOpt', 'an-bg2', 'Let Hearth run in the background', 'Keeps Hearth from being closed while you play')))
        + shead('Android') + sgrid(page('an-notif', 'bell', 'Hearth\'s notifications', '', 'notifications')
          + page('an-notifs', 'bell', 'Notifications', 'All apps', 'allNotifications')
          + page('an-storage', 'lib', 'Storage', '', 'storage')
          + page('an-date', 'clock', 'Date and time', '', 'date')
          + page('an-lang', 'globe', 'Language', '', 'language')
          + page('an-a11y', 'user', 'Accessibility', '', 'accessibility')
          + page('an-dev', 'gear', 'Developer options', 'Only when they\'re turned on (About › tap Build number 7 times)', 'developer')
          + page('an-aboutdev', 'info', s.samsung ? 'About tablet' : 'About this device', '', 'about')
          + page('an-apps', 'gear', 'Android settings', 'Everything else on this device', 'settings'))
        + note('These open in Android\'s settings. Press B (Back) to come back to Hearth.') + body;
    }
    return null;
  };

  // ---------------------------------------------------------------- changing things
  async function anSet(key, value){
    let r;
    try { r = await call('android.settings.set', {key, value}); } catch (e) { toast(e.message); return false; }
    if (r && r.state) AS.s = r.state;
    if (r && !r.ok && r.need) { anAskSheet(r.need, r.why); return false; }
    return true;
  }

  // Asks Android for a one-time permission, after saying why; a "no" leaves everything as it was.
  function anAskSheet(perm, why){
    if (AS.askedOnce[perm + ':sheet']) return toast(why || 'Android needs your okay first');
    AS.askedOnce[perm + ':sheet'] = 1;
    openSheet({title: 'Android needs your okay', sub: why || '', items: [{label: 'Ask Android', icon: 'lock', run: () => anAsk(perm)}]});
  }
  async function anAsk(perm){
    try {
      const r = await call('android.settings.ask', {perm}, 600000);
      if (r && r.state) AS.s = r.state;
      toast(r && r.granted ? 'Thanks: Hearth can do that now' : 'Not allowed: nothing changed. You can ask again here any time.');
      if (perm === 'location' && r && r.granted) anScan();
    } catch (e) { toast(e.message); }
    anRefresh();
  }
  window.anAsk = anAsk;

  async function anScan(){
    if (AS.scanning) return;
    AS.scanning = true; anRefresh();
    try {
      const r = await call('android.wifi.scan', {}, 30000);
      if (r && r.need) { AS.scanning = false; anRefresh(); return anAskSheet(r.need, r.why); }
      AS.nets = (r && r.networks) || [];
    } catch (e) { toast(e.message); }
    AS.scanning = false; anRefresh();
  }

  async function anJoin(n){
    if (!n) return;
    if (n.security === 'enterprise' || n.security === 'wep') return call('android.openSettings', {page: 'wifi'}).catch(e => toast(e.message));
    let password = '';
    if (n.security !== 'open') {
      password = await openPrompt({title: `Password for ${n.ssid}`, secret: true, mode: 'password', enterLabel: 'Join'});
      if (!password) return;
    }
    try {
      const r = await call('android.wifi.join', {ssid: n.ssid, password, security: n.security}, 600000);
      toast(r && r.suggested ? `Android will join ${n.ssid} when it's in range` : r && r.ok ? `Saved ${n.ssid}: Android is joining it` : 'Not joined: nothing changed');
    } catch (e) { toast(e.message); }
  }

  window.anSlide = function(el, d){
    const key = el.dataset.slider.slice(3), min = +el.dataset.min, max = +el.dataset.max, st = +el.dataset.step;
    const v = Math.max(min, Math.min(max, (+el.dataset.v || 0) + d * st));
    el.dataset.v = v;
    const pct = max > min ? (v - min) / (max - min) * 100 : 0;
    el.querySelector('.slider i').style.width = pct + '%'; el.querySelector('.slider b').style.left = pct + '%';
    el.querySelector('.lab span').textContent = key.startsWith('vol.') ? volShown(v, max) : v + '%';
    debounce('an:' + key, () => anSet(key, v).then(ok => {
      if (!ok) return;
      const s = AS.s || {};
      if (key.startsWith('vol.') && s.audio && s.audio.streams[key.slice(4)]) s.audio.streams[key.slice(4)].v = v;
      if (key === 'brightness' && s.display) s.display.brightness = v;
    }), 120);
    if (typeof blip === 'function') blip(500 + pct * 4, .03);
  };

  window.anAct = function(a, d, el){
    const s = AS.s || {};
    switch (a) {
      case 'anPage': return call('android.openSettings', d.id ? {page: d.page, id: d.id} : {page: d.page}).catch(e => toast(e.message));
      case 'anPanel': return call('android.panel', {name: d.panel}).catch(e => toast(e.message));
      case 'anAsk': return anAsk(d.perm);
      case 'anScan': return anScan();
      case 'anJoin': return anJoin((AS.nets || [])[+d.i]);
      case 'anToggle': {
        const cur = d.key === 'mute' ? !!(s.audio && s.audio.streams.media && s.audio.streams.media.muted)
          : d.key === 'autoBrightness' ? !!(s.display && s.display.auto) : d.key === 'autoRotate' ? !!(s.display && s.display.rotate) : false;
        return anSet(d.key, !cur).then(() => render(el && el.dataset.k, true));
      }
      case 'anDnd': return openSheet({title: 'Do Not Disturb', items: DNDS.map(([v, l]) => ({label: l + ((s.dnd || {}).filter === v ? ' ✓' : ''), icon: 'moon', run: () => anSet('dnd', v).then(() => render('an-dnd', true))}))});
      case 'anTimeout': return openSheet({title: 'Screen timeout', sub: 'How long the screen stays on without a touch or a button', items: ((s.display || {}).timeouts || [15, 30, 60, 120, 300, 600, 1800]).map(t => ({label: tLabel(t) + ((s.display || {}).timeoutSec === t ? ' ✓' : ''), icon: 'clock', run: () => anSet('timeout', t).then(() => render('an-timeout', true))}))});
      case 'anBtOn': return call('android.bt.enable', {}, 600000).then(r => { if (r && r.state) AS.s = r.state; if (r && r.need) anAskSheet(r.need, r.why); anRefresh(); }).catch(e => toast(e.message));
      case 'anBtPair': return call('android.bt.pair', {}, 600000).then(r => {
        if (r && r.need) return anAskSheet(r.need, r.why);
        if (r && r.ok) toast(`Pairing ${r.device}: confirm on the device if it asks`);
        else if (r && r.opened) toast('Pick the device in Android\'s Bluetooth settings, then press B');
      }).catch(e => toast(e.message));
      case 'anUninstall': return call('android.uninstall', {id: d.id}).catch(e => toast(e.message));
    }
  };

  // A game's options (index.html gameOptions): Android apps also get Uninstall (Android's own dialog asks first).
  window.anGameItems = function(g){
    if (!g || g.src === 'emu' || !/^android:/.test(g.id || '')) return [];
    return [{label: 'Uninstall…', icon: 'trash', run: () => call('android.uninstall', {id: g.id}).catch(e => toast(e.message))}];
  };
})();
