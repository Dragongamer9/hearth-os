/* Settings (mockup 8), every page: Home screen (with all of Hearth OS's looks), TV Cleanup, TV inputs, Profiles,
   My Media, Hearth Link, Screensaver, Picture and sound, Remote and controllers, Set up again, About. */
(function () {
  'use strict';
  var T = window.hearthTV, E = T.esc;
  var PAGES = ['Home screen', 'TV Cleanup', 'TV inputs', 'Profiles', 'My Media', 'Hearth Link', 'Screensaver', 'Picture and sound', 'Remote and controllers', 'Set up again', 'About'];
  var ST = { page: 'TV Cleanup', clean: null, adb: null, checking: false, update: null, peers: null, ctrls: null, views: null };

  // ---------- Typing a name or a password with the remote ----------
  T.askText = function (title, initial, done, opts) {
    opts = opts || {};
    var text = initial || '', caps = false, sym = false;
    T.open({
      id: 'text', first: 'tk' + (opts.digits ? '1' : 'a'),
      render: function () {
        var h = '<div class="dim"><div class="dialog on-panel" style="width:1100px"><span style="font-size:30px;font-weight:700">' + E(title) + '</span>' +
          '<div style="height:72px;border-radius:var(--r);background:var(--bg);border:2px solid var(--focus);display:flex;align-items:center;padding:0 20px;font-size:30px;font-weight:600">' + E(opts.secret ? text.replace(/./g, '●') : text) + '<span style="display:inline-block;width:3px;height:32px;background:var(--focus);margin-left:2px"></span></div>' +
          '<div style="display:grid;grid-template-columns:repeat(13,minmax(0,1fr));gap:8px">';
        var chars = opts.digits ? '1234567890' : sym ? '!#$%^&*()+=?,;\'"~`[]{}<>|\\1234567890-_.@:/ ' : 'abcdefghijklmnopqrstuvwxyz1234567890-_.@:/ ';
        chars.split('').forEach(function (c) { var s = caps ? c.toUpperCase() : c; h += '<button class="key" data-f data-key="tk' + (c === ' ' ? 'sp' : sym ? 's' + chars.indexOf(c) : c) + '" data-act="c" data-arg="' + E(s) + '">' + (c === ' ' ? '␣' : E(s)) + '</button>'; });
        h += '</div><div style="display:flex;gap:12px">' + (opts.digits ? '' : (sym ? '' : '<button class="btn small" data-f data-key="tkCaps" data-act="caps">' + (caps ? 'abc' : 'ABC') + '</button>') + '<button class="btn small" data-f data-key="tkSym" data-act="sym">' + (sym ? 'abc' : '#+=') + '</button>') +
          '<button class="btn small" data-f data-key="tkDel" data-act="del">⌫ Delete</button><div style="flex:1"></div><button class="btn small" data-f data-key="tkCancel" data-act="close">Cancel</button><button class="btn small primary" data-f data-key="tkOk" data-act="ok">' + E(opts.ok || 'Done') + '</button></div></div></div>';
        return h;
      },
      key: function (k, e) { if (e && k.length === 1) { text += k; T.renderLayer(); return true; } if (e && k === 'Backspace') { text = text.slice(0, -1); T.renderLayer(); return true; } return false; },
      act: function (act, arg) {
        if (act === 'c') text += arg; else if (act === 'del') text = text.slice(0, -1); else if (act === 'caps') caps = !caps; else if (act === 'sym') sym = !sym;
        else if (act === 'ok') { T.close(); done(text); return true; } else return false;
        T.renderLayer(); return true;
      }
    });
  };
  /** A list to pick from (a look, a sub-theme, a wallpaper...). */
  T.pickFrom = function (title, items, cur, done) {
    T.open({
      id: 'pick', first: 'pi' + Math.max(0, items.findIndex(function (x) { return x.id === cur; })),
      render: function () {
        var h = '<div class="dim"><div class="dialog on-panel" style="width:1000px;max-height:900px"><span style="font-size:30px;font-weight:700">' + E(title) + '</span><div data-scrolly style="display:grid;grid-template-columns:repeat(' + (items.some(function (x) { return x.img; }) ? 3 : 2) + ',minmax(0,1fr));gap:12px;overflow:hidden;padding:8px;margin:-8px">';
        items.forEach(function (x, i) {
          h += '<button data-f data-key="pi' + i + '" data-act="p" data-arg="' + i + '" style="display:flex;flex-direction:column;gap:6px;padding:16px;border-radius:var(--r);background:' + (x.id === cur ? 'var(--panel3)' : 'var(--panel2)') + ';text-align:left">' +
            (x.img ? '<span style="height:120px;border-radius:var(--r);position:relative;overflow:hidden;display:block;background:var(--bg)"><img class="art" src="' + E(x.img) + '" alt="" loading="lazy"></span>' : '') +
            (x.swatch ? '<span style="display:flex;gap:6px">' + x.swatch.map(function (c) { return '<span style="width:28px;height:28px;border-radius:14px;background:' + E(c) + '"></span>'; }).join('') + '</span>' : '') +
            '<span style="font-size:20px;font-weight:700">' + E(x.label) + (x.id === cur ? ' ✓' : '') + '</span>' + (x.desc ? '<span class="muted" style="font-size:16px">' + E(x.desc) + '</span>' : '') + '</button>';
        });
        return h + '</div><div style="display:flex;justify-content:flex-end"><button class="btn small" data-f data-key="piBack" data-act="close">↩ Back</button></div></div></div>';
      },
      act: function (act, arg) { if (act === 'p') { T.close(); done(items[Number(arg)]); return true; } return false; }
    });
  };

  // ---------- Rows ----------
  var ROWS = [];
  function head(n) { ROWS.push({ kind: 'head', name: n }); }
  function toggle(name, why, on, fn, opts) { ROWS.push(Object.assign({ kind: 'toggle', name: name, why: why, on: !!on, fn: fn }, opts || {})); }
  function choice(name, why, val, fn) { ROWS.push({ kind: 'choice', name: name, why: why, val: val, fn: fn }); }
  function action(name, why, val, fn, opts) { ROWS.push(Object.assign({ kind: 'action', name: name, why: why, val: val, fn: fn }, opts || {})); }
  function note(t) { ROWS.push({ kind: 'note', name: t }); }
  function cycle(path, opts, labels) {
    var parts = path.split('.'), cur = parts.length > 1 ? T.settings[parts[0]][parts[1]] : T.settings[path];
    var i = opts.indexOf(cur), next = opts[(i + 1) % opts.length];
    T.set(path, next); T.toast((labels ? labels[next] : next) + ' · saved');
  }

  function pageRows(page) {
    ROWS = [];
    var s = T.settings;
    if (page === 'Home screen') {
      head('Look');
      choice('Look', 'Every Hearth OS look' + (T.palettes && T.palettes.gold ? ', Gold included' : ''), T.lookName(), function () {
        var items = T.looks().map(function (l) { var c = T.palettes[l.id].dark || {}; return { id: l.id, label: l.label, desc: l.desc, swatch: [c.bg, c.panel3 || c.panel2, c.ember, c.flame] }; });
        T.pickFrom('Look', items, s.look, function (x) { T.set('look', x.id); T.set('sub', ''); T.applyLook(); T.render(); T.toast('Look: ' + x.label + ' · saved'); });
      });
      var look = T.palettes && T.palettes[s.look], subs = (look && look.subThemes || []).concat((T.palettes && T.palettes.blackSet) || []);
      choice('Sub-theme', 'A variation of the look: its own colours and wallpaper', (subs.filter(function (x) { return x.id === s.sub; })[0] || { label: 'The look\'s own' }).label, function () {
        var items = [{ id: '', label: 'The look\'s own', desc: '' }].concat(subs.map(function (x) { var c = x.dark || {}; return { id: x.id, label: x.label, desc: x.desc || '', swatch: [c.bg, c.panel3 || c.panel2, c.ember, c.flame].filter(Boolean) }; }));
        T.pickFrom('Sub-theme', items, s.sub, function (x) { T.set('sub', x.id); T.applyLook(); T.render(); T.toast('Sub-theme: ' + x.label + ' · saved'); });
      });
      choice('Dark or light', '', s.mode === 'light' ? 'Light' : 'Dark', function () { T.set('mode', s.mode === 'light' ? 'dark' : 'light'); T.applyLook(); T.render(); });
      choice('Accent colour', 'The colour of buttons and the focus ring', ((T.palettes && T.palettes.accents || []).filter(function (a) { return a.color === s.accent; })[0] || { name: 'The look\'s own' }).name, function () {
        var items = [{ id: '', label: 'The look\'s own' }].concat(((T.palettes && T.palettes.accents) || []).map(function (a) { return { id: a.color, label: a.name, swatch: [a.color] }; }));
        T.pickFrom('Accent colour', items, s.accent, function (x) { T.set('accent', x.id); T.applyLook(); T.render(); });
      });
      action('Background behind Home', 'Shown when the featured banner is off or empty', wallName(), function () {
        var items = [{ id: '', label: 'The look\'s own', img: T.wallpaperUrl(lookWall()) }].concat(WALLS.map(function (w) { return { id: w, label: w.split('/').pop().replace(/^(builtin:|tv:)/, '').replace(/[-_]/g, ' '), img: T.wallpaperUrl(w) }; }));
        T.pickFrom('Background behind Home', items, s.wallpaper, function (x) { T.set('wallpaper', x.id); T.applyLook(); T.render(); });
      }, { img: T.lookWallpaper });
      head('What Home shows');
      toggle('Featured banner', 'The big picture at the top', s.home.hero, function (v) { T.set('home.hero', v); });
      choice('Featured banner changes every', 'Stops while you are on it', { 7: '7 seconds', 15: '15 seconds', 0: 'Never' }[s.home.rotate] || '7 seconds', function () { cycle('home.rotate', [7, 15, 0], { 7: '7 seconds', 15: '15 seconds', 0: 'Never' }); });
      toggle('Your apps row', '', s.home.rowApps, function (v) { T.set('home.rowApps', v); });
      toggle('Keep watching row', 'Your PC and Theater', s.home.rowKeep, function (v) { T.set('home.rowKeep', v); });
      toggle('Recently added on your PC', '', s.home.rowNew, function (v) { T.set('home.rowNew', v); });
      toggle('Theater row', 'Your saved links', s.home.rowTheater, function (v) { T.set('home.rowTheater', v); });
      note('Changes show up on Home right away.');
    }
    else if (page === 'TV inputs') {
      var ins = T.json('inputs') || [];
      head('Inputs');
      if (!ins.length) note('This TV didn\'t list any inputs to Hearth TV.');
      ins.forEach(function (x) {
        if (x.kind === 'tuner') return;
        var cur = (s.inputs.names || {})[x.id] || x.custom || x.label;
        choice(x.label, (x.state === 'signal' ? 'Signal' : x.state === 'standby' ? 'Standby' : 'No signal') + ' · pick a name for it', cur, function () {
          var names = [x.custom || x.label, 'Nvidia PC', 'Game console', 'Cable box', 'Blu-ray player', 'Soundbar', 'Type a name…'];
          var i = names.indexOf(cur), nx = names[(i + 1) % names.length];
          var setName = function (n) { var all = Object.assign({}, s.inputs.names || {}); all[x.id] = n; T.set('inputs.names', all); T.toast(x.label + ' is now "' + n + '"'); T.render(); };
          if (nx === 'Type a name…') T.askText('A name for ' + x.label, '', function (t) { if (t.trim()) setName(t.trim()); }); else setName(nx);
        });
      });
      ins.forEach(function (x) {
        if (x.kind === 'tuner') return;
        toggle('Hide ' + ((s.inputs.names || {})[x.id] || x.label) + ' from the input list', x.kind === 'av' ? 'Rarely used' : '', (s.inputs.hidden || {})[x.id], function (v) { var all = Object.assign({}, s.inputs.hidden || {}); all[x.id] = v; T.set('inputs.hidden', all); });
      });
      if (ins.some(function (x) { return x.kind === 'tuner'; })) action('Live TV (antenna)', 'Opens the TV\'s Live TV app', 'Scan again ›', function () { T.n('liveTv'); });
      head('Switching');
      toggle('Inputs button on Home', 'Also on the remote\'s Input button', s.inputs.button, function (v) { T.set('inputs.button', v); });
      toggle('Switch by itself when a device turns on', 'Uses HDMI-CEC: turn on the console or PC and the TV follows', s.inputs.autoSwitch, function (v) {
        T.set('inputs.autoSwitch', v);
        T.call('tvSetting', 'hdmi_control_enabled', v ? '1' : '0').then(function (r) { if (!r || !r.adb) T.toast('Saved. The TV\'s own HDMI-CEC switch needs network debugging (Settings › TV Cleanup)'); });
      });
      toggle('Come back to Hearth TV when that device turns off', '', s.inputs.backHome, function (v) { T.set('inputs.backHome', v); });
      choice('When the TV turns on, start on', '', s.inputs.startOn === 'last' ? 'The last input used' : 'Hearth TV', function () { T.set('inputs.startOn', s.inputs.startOn === 'last' ? 'hearth' : 'last'); });
      note('Names you pick show on the Inputs screen. Picture settings for each input stay in the TV\'s own menu.');
    }
    else if (page === 'Profiles') {
      head('Who uses this TV');
      T.profiles().forEach(function (p, i) {
        action(p.name, (i === 0 ? 'Owner · can change settings' : p.kids ? 'Kids · only shows rated for kids' : 'Own Keep watching and pinned shows') + (T.currentProfile().id === p.id ? ' · watching now' : ''), 'Edit ›', function () { editProfile(p); }, { swatch: p.color || T.PROFILE_COLORS[i % 6] });
      });
      action('Add a profile', '', '+', function () {
        T.askText('Name for the new profile', '', function (n) {
          if (!n.trim()) return;
          var ps = T.profiles().slice(); ps.push({ id: 'p' + Date.now().toString(36), name: n.trim(), color: T.PROFILE_COLORS[ps.length % 6], kids: false }); T.set('profiles', ps); T.toast('Added ' + n.trim()); T.render();
        });
      });
      head('Safety');
      toggle('Kids profile', 'This profile only: shows rated for kids; no Settings, no app installs', T.currentProfile().kids, function (v) {
        var cur = T.currentProfile(); if (T.profiles()[0].id === cur.id && v) { T.toast('The owner profile can\'t be a kids profile'); return false; }
        T.set('profiles', T.profiles().map(function (p) { return p.id === cur.id ? Object.assign({}, p, { kids: v }) : p; }));
      });
      toggle('PIN for Settings', 'Asks for a 4-digit PIN before anything here changes, and to leave a kids profile', s.pinOn && s.pin, function (v) {
        if (v) { T.askPin(function (pin) { T.set('pin', pin); T.set('pinOn', true); T.toast('PIN set'); T.render(); }, true); return false; }
        T.set('pinOn', false);
      });
    }
    else if (page === 'My Media') {
      var M = T.Media, sess = M.session();
      head('Your library');
      action(sess ? (sess.name || 'Nvidia PC') : 'Not connected', sess ? (sess.app === 'plex' ? 'Plex' : 'Jellyfin (Hearth Server)') + (sess.user ? ' · signed in as ' + sess.user : '') + ' · ' + sess.server.replace(/^https?:\/\//, '') : 'Sign in to see your PC\'s shows, movies and music', sess ? 'Connected' : 'Connect ›', function () { mediaSignIn(); }, { good: !!sess });
      choice('Library app on the PC', 'Hearth TV reads it directly; no app needed on the TV', s.media.app === 'plex' ? 'Plex' : 'Jellyfin', function () { T.set('media.app', s.media.app === 'plex' ? 'jellyfin' : 'plex'); T.toast('Now sign in to ' + (s.media.app === 'plex' ? 'Jellyfin' : 'Plex')); });
      var libs = s.media.libraries || {};
      ['Shows', 'Movies', 'Music'].forEach(function (k) { toggle(k, 'Show this kind of item in My Media', libs[k] !== false, function (v) { var all = Object.assign({}, libs); all[k] = v; T.set('media.libraries', all); }); });
      head('Playing');
      choice('Quality', 'Original plays the file as it is when the network allows', { original: 'Original (best)', '1080p': '1080p', '720p': '720p' }[s.media.quality], function () { cycle('media.quality', ['original', '1080p', '720p'], { original: 'Original (best)', '1080p': '1080p', '720p': '720p' }); });
      toggle('Lower quality when Wi-Fi is weak', 'The PC sends a smaller stream instead of stopping to load', s.media.autoQuality, function (v) { T.set('media.autoQuality', v); });
      choice('Subtitles by default', 'Each show remembers its own choice after that', { off: 'Off', english: 'English', spanish: 'Spanish' }[s.media.subs] || 'Off', function () { cycle('media.subs', ['off', 'english', 'spanish'], { off: 'Off', english: 'English', spanish: 'Spanish' }); });
      if (T.n('tailscaleInstalled')) toggle('Turn on Tailscale when Hearth TV starts', 'So your PC is reachable after the TV restarts (TCL\'s TVs don\'t let Tailscale start by itself)', s.media.tailscale !== false, function (v) { T.set('media.tailscale', v); if (v) T.n('tailscaleConnect'); });
      toggle('Wake the PC when needed', T.state.pcMac ? 'Turns the PC on over the network when you press Play' : 'Turns the PC on over the network (Hearth TV learns the PC\'s address the first time it\'s on)', s.media.wake, function (v) { T.set('media.wake', v); if (v) learnMac(); });
      if (sess) action('Sign out', 'This TV forgets the sign-in; your library stays on the PC', '', function () { T.Media.signOut(); T.toast('Signed out of My Media'); T.render(); });
      note('Nothing is copied to the TV: shows stream from your PC.');
    }
    else if (page === 'Hearth Link') {
      if (!T.n('linkAvailable')) { note('Hearth Link isn\'t in this build of Hearth TV.'); return ROWS; }
      var d = T.data.link || {}, me = d.me || d.self || {};
      head('This TV');
      action('Name', 'How it shows on your other devices', (me.name || d.name || 'Living room TV') + ' ›', function () {
        T.askText('Name for this TV', me.name || d.name || 'Living room TV', function (n) { if (!n.trim()) return; T.call('link', 'settings.set', JSON.stringify({ name: n.trim() })).then(function () { T.toast('This TV is now "' + n.trim() + '"'); refreshLink(); }); });
      });
      toggle('Allow "Send to TV"', 'Links and videos sent from your devices open in Theater', s.link.sendTo, function (v) { T.set('link.sendTo', v); });
      head('Your devices');
      var peers = (d.peers || []).filter(function (p) { return !p.friend; });
      if (!peers.length) note(d.peers ? 'No devices paired yet.' : 'Hearth Link is starting…');
      peers.forEach(function (p) { action(p.name, (p.lan ? 'On this network' : p.online ? 'Over the internet' : 'Last seen ' + ago(p.lastSeen)), p.online ? 'Online' : 'Offline', function () { deviceMenu(p); }, { good: p.online }); });
      action('Pair a device', 'Your own device, or a friend\'s (kept separate)', '+', pairDialog);
      toggle('Friends can send to this TV', 'Only friends you paired', s.link.friends, function (v) { T.set('link.friends', v); });
      var friends = (d.peers || []).filter(function (p) { return p.friend; });
      if (friends.length) { head('Friends\' devices'); friends.forEach(function (p) { action(p.name, 'A friend\'s device', p.online ? 'Online' : 'Offline', function () { deviceMenu(p); }); }); }
      note('A device paired as "my device" joins all your devices; a friend\'s device only sees this TV.');
    }
    else if (page === 'Screensaver') {
      head('Screensaver');
      choice('Start after', 'Only when nothing is playing', { 5: '5 minutes', 10: '10 minutes', 30: '30 minutes', 0: 'Never' }[s.saver.after], function () { cycle('saver.after', [5, 10, 30, 0], { 5: '5 minutes', 10: '10 minutes', 30: '30 minutes', 0: 'Never' }); T.idle(); });
      choice('Pictures', 'Stills from your shows and movies, your Hearth wallpapers, or space photos', { media: 'My Media stills', wallpapers: 'Hearth wallpapers', space: 'Space photos' }[s.saver.pictures], function () { cycle('saver.pictures', ['media', 'wallpapers', 'space'], { media: 'My Media stills', wallpapers: 'Hearth wallpapers', space: 'Space photos' }); });
      toggle('Show the clock', '', s.saver.clock, function (v) { T.set('saver.clock', v); });
      action('Preview the screensaver', 'Opens it now', 'Preview ›', function () { T.go('screensaver'); });
      action('Use it as the TV\'s own screensaver', 'The TV\'s Settings › Screensaver: pick Hearth TV', 'Open ›', function () { T.n('openSystem', 'screensaver'); });
    }
    else if (page === 'Picture and sound') {
      head('In Hearth TV\'s player');
      toggle('Match the video\'s frame rate', 'Smoother movies on this TV', s.player.frame, function (v) { T.set('player.frame', v); });
      choice('HDR', '', s.player.hdr === 'off' ? 'Always off' : 'Automatic', function () { T.set('player.hdr', s.player.hdr === 'off' ? 'auto' : 'off'); });
      toggle('Night mode', 'Quieter loud scenes', s.player.night, function (v) { T.set('player.night', v); });
      toggle('Clearer voices', 'Lifts speech above music and effects', s.player.dialog, function (v) { T.set('player.dialog', v); });
      choice('Sound comes out of', '', s.player.out === 'arc' ? 'Soundbar (HDMI ARC)' : 'TV speakers', function () {
        var arc = s.player.out !== 'arc'; T.set('player.out', arc ? 'arc' : 'tv');
        T.call('tvSetting', 'hdmi_system_audio_control', arc ? '1' : '0').then(function (r) { if (!r || !r.adb) T.toast('Saved. Switching the TV\'s sound needs network debugging (Settings › TV Cleanup)'); });
      });
      head('The TV itself');
      action('TCL picture and sound settings', 'Brightness, picture mode, motion: these stay in the TV\'s own menu', 'Open ›', function () { T.n('openSystem', 'picture'); });
      note('Hearth TV never changes the TV\'s own picture settings.');
    }
    else if (page === 'Remote and controllers') {
      head('Connected');
      var cs = ST.ctrls || T.json('controllers') || [];
      if (!cs.length) note('Nothing connected besides the TV\'s own buttons.');
      cs.forEach(function (c) { action(c.name, c.pad ? 'Controller' : 'Remote', 'Connected', function () { T.toast(c.name + ' is connected'); }, { good: true }); });
      action('Add a controller', 'Hold the pair button on the controller', '+', function () { T.n('openSystem', 'bluetooth'); });
      head('Buttons');
      action('Button guide', 'What each button does in Hearth TV', 'View ›', buttonGuide);
      toggle('Turn on with the TV', 'The TV remote also works devices plugged in over HDMI', T.n('tvSettingGet', 'hdmi_control_auto_wakeup_enabled') !== '0', function (v) {
        T.call('tvSetting', 'hdmi_control_auto_wakeup_enabled', v ? '1' : '0').then(function (r) { if (!r || !r.adb) T.toast('This needs network debugging (Settings › TV Cleanup)'); else T.toast('Saved'); });
      });
    }
    else if (page === 'About') {
      var dev = T.json('device') || {}, u = ST.update;
      head('Hearth TV');
      action('Version', 'You pick the version number for each release', T.n('version') || '1.0', function () { T.toast('Hearth TV ' + (T.n('version') || '')); });
      action('Check for updates', u ? (u.error ? u.error : u.newer ? 'Hearth TV ' + u.latest + ' is ready' : 'You have the newest Hearth TV') : 'Hearth TV updates itself from inside the app', u ? (u.newer ? 'Install ›' : u.error ? 'Try again' : 'Up to date') : 'Check ›', function () {
        if (u && u.newer) { T.toast('Downloading Hearth TV ' + u.latest + '…'); T.call('updateInstall', u.url).then(function (r) { if (r && r.message) T.toast(r.message); }); return; }
        T.toast('Checking for Hearth TV updates…');
        T.call('updateCheck').then(function (r) { ST.update = r || { error: 'Couldn\'t check' }; T.render(); });
      }, { good: u && !u.newer && !u.error, dataUpdate: true });
      toggle('Install updates by themselves', 'Off: asks first', s.updates.auto, function (v) { T.set('updates.auto', v); });
      head('This TV');
      action('Model', 'From the TV\'s Settings › System › About', ((dev.maker || '') + ' ' + (dev.model || '')).trim() || '—', function () {});
      action('Android TV OS build', 'Android ' + (dev.android || ''), dev.build || '—', function () {});
      head('Pictures');
      action('Shows\', movies\' and music pictures', 'From your library (Jellyfin or Plex on your PC)', '', function () {});
      action('Looks and wallpapers', 'Hearth OS\'s own (Settings › Home screen)', '', function () {});
    }
    return ROWS;
  }
  var WALLS = ['builtin:slate-horizon', 'builtin:slate-dusk', 'builtin:slate-ridges', 'builtin:slate-nebula', 'builtin:slate-facets', 'builtin:slate-contours',
    'tv:theme/backgrounds/hearth-embers.jpg', 'tv:theme/backgrounds/hearth-cabin.jpg', 'tv:theme/backgrounds/hearth-peaks.jpg', 'tv:theme/backgrounds/hearth-fireside.jpg', 'tv:theme/backgrounds/hearth-lanterns.jpg', 'tv:theme/backgrounds/hearth-ridge.jpg'];
  function lookWall() { var l = T.palettes && T.palettes[T.settings.look]; return l ? l.wallpaper : ''; }
  function wallName() { var w = T.settings.wallpaper; return w ? w.split('/').pop().replace(/^(builtin:|tv:)/, '').replace(/\.jpg$/, '').replace(/[-_]/g, ' ') : 'The look\'s own'; }
  function ago(t) { if (!t) return 'never'; var m = Math.round((Date.now() - t) / 60000); return m < 60 ? m + ' min ago' : m < 1440 ? Math.round(m / 60) + ' h ago' : Math.round(m / 1440) + ' days ago'; }
  function learnMac() {
    var sess = T.Media.session(); if (!sess) return;
    var host = sess.server.replace(/^https?:\/\//, '').split(/[:/]/)[0];
    T.call('learnMac', host).then(function (mac) { if (mac && /^[0-9a-f:]{17}$/i.test(mac)) { T.setState({ pcMac: mac }); T.toast('Hearth TV can now wake your PC'); } });
  }
  T.learnMac = learnMac;

  function editProfile(p) {
    T.open({
      id: 'prof', first: 'peName',
      render: function () {
        return '<div class="dim"><div class="dialog on-panel" style="width:700px"><span style="font-size:30px;font-weight:700">' + E(p.name) + '</span>' +
          '<button class="srow" data-f data-key="peName" data-act="name"><span style="flex:1" class="n">Name</span><span class="v">' + E(p.name) + ' ›</span></button>' +
          '<button class="srow" data-f data-key="peColor" data-act="color"><span style="flex:1" class="n">Colour</span><span style="width:36px;height:36px;border-radius:18px;background:' + E(p.color || T.PROFILE_COLORS[0]) + '"></span></button>' +
          '<button class="srow" data-f data-key="peMedia" data-act="media"><span style="flex:1"><span class="n" style="display:block">My Media sign-in</span><span class="w">' + (T.state.media && T.state.media['p:' + p.id] ? 'Its own account' : 'Shares the TV\'s') + '</span></span><span class="v">›</span></button>' +
          (T.profiles()[0].id !== p.id ? '<button class="btn small danger" data-f data-key="peDel" data-act="del">Remove this profile</button>' : '') +
          '<button class="btn small" data-f data-key="peBack" data-act="close">Done</button></div></div>';
      },
      act: function (act) {
        var save = function (np) { p = np; T.set('profiles', T.profiles().map(function (x) { return x.id === p.id ? p : x; })); };
        if (act === 'name') { T.close(); T.askText('Name', p.name, function (n) { if (n.trim()) save(Object.assign({}, p, { name: n.trim() })); T.render(); editProfile(p); }); return true; }
        if (act === 'color') { var i = T.PROFILE_COLORS.indexOf(p.color); save(Object.assign({}, p, { color: T.PROFILE_COLORS[(i + 1) % T.PROFILE_COLORS.length] })); T.renderLayer('peColor'); return true; }
        if (act === 'media') { T.close(); if (T.currentProfile().id !== p.id) { T.toast('Switch to ' + p.name + ' (Who\'s watching) to sign it in'); return true; } mediaSignIn(true); return true; }
        if (act === 'del') { T.set('profiles', T.profiles().filter(function (x) { return x.id !== p.id; })); if (T.state.profile === p.id) T.setState({ profile: T.profiles()[0].id }); T.close(); T.render(); T.toast('Removed ' + p.name); return true; }
        return false;
      }
    });
  }

  // ---------- My Media sign-in (Jellyfin: Quick Connect or name and password; Plex: plex.tv/link) ----------
  function mediaSignIn(forProfile) {
    var M = T.Media, app = T.settings.media.app;
    if (app === 'plex') {
      M.plexStart().then(function (pin) {
        var stop = false;
        T.open({ id: 'plex', first: 'plBack', onClose: function () { stop = true; },
          render: function () { return '<div class="dim"><div class="dialog on-panel" style="width:820px;align-items:center;text-align:center"><span style="font-size:32px;font-weight:700">Sign in to Plex</span><span class="soft" style="font-size:20px">On your phone or PC, open <b>plex.tv/link</b> and type this code</span><span style="font-size:84px;font-weight:700;letter-spacing:.18em;color:var(--focus)">' + E(pin.code) + '</span><span class="muted" style="font-size:17px">Waiting for Plex…</span><button class="btn small" data-f data-key="plBack" data-act="close">Cancel</button></div></div>'; } });
        var poll = function () { if (stop) return; M.plexPoll(pin, forProfile).then(function (ok) { if (ok) { T.close(); T.toast('Signed in to Plex'); T.render(); } else setTimeout(poll, 2500); }).catch(function (e) { T.close(); T.toast('Plex: ' + e.message); }); };
        setTimeout(poll, 2500);
      }).catch(function (e) { T.toast('Couldn\'t reach Plex: ' + e.message); });
      return;
    }
    T.toast('Looking for Jellyfin on your network…');
    M.jfServers().then(function (servers) {
      var list = servers.map(function (x) { return { id: x.url, label: x.name, desc: x.url }; });
      list.push({ id: 'type', label: 'Type the address', desc: 'Your PC\'s address, for example 192.168.1.20 (http:// and :8096 are added for you)' });
      T.pickFrom('Your PC\'s Jellyfin', list, '', function (x) {
        var go = function (server) { server = server.replace(/\/$/, ''); if (!/^https?:\/\//i.test(server)) server = 'http://' + server; if (!/:\d+$/.test(server.replace(/^https?:\/\//i, '')) && !/^https:/i.test(server)) server += ':8096'; M.serverName = x.label; signInJellyfin(server, forProfile); };
        if (x.id === 'type') T.askText('Jellyfin\'s address', '', function (t) { if (t.trim()) go(t.trim()); }); else go(x.id);
      });
    });
  }
  T.mediaSignIn = mediaSignIn;
  function signInJellyfin(server, forProfile) {
    var M = T.Media;
    M.jfQuickStart(server).then(function (q) {
      var stop = false;
      T.open({ id: 'qc', first: 'qcPw', onClose: function () { stop = true; },
        render: function () {
          return '<div class="dim"><div class="dialog on-panel" style="width:900px;align-items:center;text-align:center"><span style="font-size:32px;font-weight:700">Sign in to Jellyfin</span>' +
            '<span class="soft" style="font-size:20px;line-height:1.5">On your PC: Jellyfin (Hearth Server) › your profile › <b>Quick Connect</b>, and type this code</span>' +
            '<span style="font-size:84px;font-weight:700;letter-spacing:.18em;color:var(--focus)">' + E(q.Code) + '</span><span class="muted" style="font-size:17px">Waiting for your PC…</span>' +
            '<div style="display:flex;gap:12px"><button class="btn small" data-f data-key="qcPw" data-act="pw">Use a name and password instead</button><button class="btn small" data-f data-key="qcBack" data-act="close">Cancel</button></div></div></div>';
        },
        act: function (act) { if (act === 'pw') { T.close(); passwordSignIn(server, forProfile); return true; } return false; }
      });
      var poll = function () { if (stop) return; M.jfQuickPoll(server, q.Secret, forProfile).then(function (ok) { if (ok) { T.close(); T.toast('Signed in to your PC\'s Jellyfin'); learnMac(); T.render(); } else setTimeout(poll, 2500); }).catch(function () { setTimeout(poll, 4000); }); };
      setTimeout(poll, 2500);
    }).catch(function () { passwordSignIn(server, forProfile); });
  }
  function passwordSignIn(server, forProfile) {
    T.askText('Jellyfin user name', '', function (user) {
      T.askText('Password for ' + user, '', function (pw) {
        T.Media.jfLogin(server, user, pw, forProfile).then(function () { T.toast('Signed in as ' + user); learnMac(); T.render(); }).catch(function (e) { T.toast(e.message); });
      }, { secret: true, ok: 'Sign in' });
    });
  }

  // ---------- Hearth Link ----------
  function refreshLink() { return T.call('link', 'status', '{}').then(function (r) { if (r && r.ok) T.data.link = r.data; if (T.cur === 'settings' && !T.layer) T.render(); }); }
  T.pairDialog = pairDialog;
  function pairDialog() {
    var mine = true, code = '', stop = false, other = null, status = '';
    var open = function () { T.call('link', 'pair.open', JSON.stringify({ mine: mine })).then(function (r) { var d = r && r.ok ? r.data : {}; code = d.code || d.lanCode || (d.pairing && d.pairing.code) || ''; T.renderLayer(); }); };
    T.open({
      id: 'pair', first: 'pkMine', onClose: function () { stop = true; T.call('link', 'pair.close', '{}'); },
      render: function () {
        var c = String(code).replace(/\D/g, '');
        return '<div class="dim"><div class="dialog on-panel" role="dialog" aria-label="Pair a device" style="width:880px;align-items:center;text-align:center"><span style="font-size:32px;font-weight:700">Pair with Hearth Link</span>' +
          '<span class="soft" style="font-size:19px">On your PC, phone or handheld: Hearth Link › Add a device › type this code</span>' +
          '<span style="font-size:84px;font-weight:700;letter-spacing:.18em;font-variant-numeric:tabular-nums;color:var(--focus);padding:10px 0">' + (c ? E(c.slice(0, 3) + ' ' + c.slice(3)) : '······') + '</span>' +
          '<div style="display:flex;gap:12px"><button class="chip' + (mine ? ' on' : '') + '" style="height:52px;padding:0 24px;border-radius:26px" data-f data-key="pkMine" data-act="mine">My device</button><button class="chip' + (!mine ? ' on' : '') + '" style="height:52px;padding:0 24px;border-radius:26px" data-f data-key="pkFriend" data-act="friend">A friend\'s device</button></div>' +
          '<span class="muted" style="font-size:16px">' + E(status || (mine ? 'Joins your set: it connects to all your other paired devices too' : 'Kept separate: it can only send to this TV')) + '</span>' +
          '<div style="display:flex;gap:12px;margin-top:8px"><button class="btn small" data-f data-key="pkType" data-act="type">Type the other device\'s code instead</button><button class="btn small" data-f data-key="pkClose" data-act="close">Close</button></div></div></div>';
      },
      act: function (act) {
        if (act === 'mine' || act === 'friend') { mine = act === 'mine'; open(); return true; }
        if (act === 'type') {
          var near = ((T.data.link || {}).nearby || []).filter(function (n) { return n.pairing; });
          var pick = function (dev) { T.close(); T.askText('The 6-digit code ' + dev.name + ' shows', '', function (t) { T.toast('Pairing with ' + dev.name + '…'); T.call('link', 'pair.join', JSON.stringify({ id: dev.id, addr: dev.addr, code: t.replace(/\D/g, ''), mine: mine })).then(function (r) { T.toast(r && r.ok ? 'Paired ✓ with ' + dev.name : (r && r.error) || 'Didn\'t pair'); refreshLink(); }); }, { digits: true, ok: 'Pair' }); };
          if (!near.length) { status = 'On the other device: Settings › Hearth Link › Pair a device first'; T.renderLayer(); refreshLink(); return true; }
          if (near.length === 1) pick(near[0]); else T.pickFrom('Which device?', near.map(function (n) { return { id: n.id, label: n.name, desc: [n.kind, n.edition].filter(Boolean).join(' · '), n: n }; }), '', function (x) { pick(x.n); });
          return true;
        }
        return false;
      }
    });
    open();
    var poll = function () { if (stop) return; T.call('link', 'status', '{}').then(function (r) { var d = r && r.ok ? r.data : null; if (d) { T.data.link = d; var ev = d.pairEvent || d.event; if (ev && ev.text) { status = ev.text; T.renderLayer(); } if (ev && /Paired/.test(ev.text || '')) { setTimeout(function () { T.close(); T.render(); }, 1500); return; } } setTimeout(poll, 2000); }); };
    setTimeout(poll, 2000);
  }
  function deviceMenu(p) {
    T.open({ id: 'dev', first: 'dvBack',
      render: function () { return '<div class="dim"><div class="dialog on-panel" style="width:640px"><span style="font-size:30px;font-weight:700">' + E(p.name) + '</span><span class="muted" style="font-size:18px">' + (p.friend ? 'A friend\'s device' : 'Your device') + (p.introducedBy ? ' · paired through ' + E(p.introducedBy) : '') + '</span><button class="btn small danger" data-f data-key="dvUnpair" data-act="unpair">Unpair</button><button class="btn small" data-f data-key="dvBack" data-act="close">↩ Back</button></div></div>'; },
      act: function (act) { if (act === 'unpair') { T.call('link', 'unpair', JSON.stringify({ id: p.id })).then(function () { T.close(); T.toast('Unpaired ' + p.name); refreshLink(); }); return true; } return false; }
    });
  }
  function buttonGuide() {
    var rows = [['OK / A', 'Open, play, change a setting'], ['◀ ▶ ▲ ▼', 'Move around · in the player, skip 1 minute with the buttons, 10 s with ◀ ▶ in Theater'], ['Back / B', 'Back one step · closes a pop-up'], ['Home', 'Always comes back to Hearth TV\'s Home'], ['☰ Menu / Start', 'Options: pin, mark watched, remove'], ['Input', 'Switch input'], ['Mic / Search', 'Search (hold the mic to talk)'], ['Play/Pause / X', 'Play or pause']];
    T.open({ id: 'guide', first: 'gBack', render: function () { return '<div class="dim"><div class="dialog on-panel" style="width:1000px"><span style="font-size:30px;font-weight:700">Button guide</span>' + rows.map(function (r) { return '<div style="display:flex;gap:24px;font-size:20px;padding:6px 0"><span style="width:240px;font-weight:700">' + E(r[0]) + '</span><span class="soft">' + E(r[1]) + '</span></div>'; }).join('') + '<button class="btn small" data-f data-key="gBack" data-act="close">Done</button></div></div>'; } });
  }

  // ---------- TV Cleanup ----------
  var GROUPS = [
    { id: 'ads', name: 'Ads and sponsored rows', why: 'Google TV and TCL promotions on the home screen and in menus', bad: true },
    { id: 'acr', name: 'Screen tracking (ACR)', why: 'TCL reading what is on screen to target ads', bad: true },
    { id: 'tcl', name: 'TCL Channel and extras', why: 'Free ad channels and preinstalled TCL apps', bad: true },
    { id: 'home', name: 'Google TV home screen', why: 'Kept installed as a backup if Hearth TV ever fails to start', bad: true, onLabel: 'Home screen', offLabel: 'Backup only' },
    { id: 'voice', name: 'Voice search on the remote', why: 'The mic button on the remote' },
    { id: 'updates', name: 'System updates', why: 'Google and TCL updates; Hearth TV checks after each one' }
  ];
  function cleanupHtml() {
    var st = ST.clean || {}, dirty = GROUPS.filter(function (g) { return g.bad && st[g.id] && st[g.id].present && st[g.id].on; });
    var checked = T.state.cleanupChecked ? 'Last checked ' + ago(T.state.cleanupChecked) + '.' : 'Last checked today, after the TV started.';
    var ok = !dirty.length;
    var h = '<section class="panel on-panel" style="padding:24px 32px;display:flex;align-items:center;gap:26px"><span style="width:64px;height:64px;border-radius:32px;background:' + (ok ? 'rgba(76,195,138,.16)' : 'rgba(245,165,36,.16)') + ';display:flex;align-items:center;justify-content:center;flex:none;font-size:30px;font-weight:700;color:' + (ok ? 'var(--ok)' : 'var(--warn)') + '">' + (ok ? '✓' : '!') + '</span>' +
      '<div style="flex:1;display:flex;flex-direction:column;gap:6px"><span style="font-size:26px;font-weight:700">' + (ok ? 'Your TV is clean' : dirty.length === 4 ? 'Everything is back the way TCL shipped it' : dirty.length + (dirty.length === 1 ? ' thing is' : ' things are') + ' turned back on') + '</span>' +
      '<span class="muted" style="font-size:18px">' + E(ok ? 'No ads, no sponsored rows, no screen tracking. ' + checked : dirty.map(function (g) { return g.name; }).join(', ')) + '</span></div>' +
      '<button class="btn small" style="min-width:220px" data-f data-key="cCheck" data-act="cCheck">' + (ST.checking ? 'Checking…' : 'Check again now') + '</button></section>';
    if (ST.adb === false) h += '<div class="panel" style="padding:16px 24px;border-color:var(--warn);font-size:18px;line-height:1.5" class="soft">Hearth TV can\'t reach the TV\'s network debugging, so switches here can\'t change the TV yet. Turn on <b>Settings › System › Developer options › Network debugging</b> on the TV (or run the install script from your laptop again), then press Check again now and pick <b>Always allow</b>.</div>';
    GROUPS.forEach(function (g) {
      var gs = st[g.id] || {}, present = g.id === 'updates' || gs.present !== false || !T.N, on = g.id === 'updates' ? true : gs.on !== undefined ? gs.on : !g.bad;
      var label = !present ? 'Not on this TV' : on ? (g.onLabel || 'On') : (g.offLabel || 'Off');
      var bad = g.bad && on && present;
      h += '<button class="srow" data-f data-key="cg' + g.id + '" data-act="cToggle" data-arg="' + g.id + '" aria-label="' + E(g.name + ': ' + label) + '"><span style="flex:1;display:flex;flex-direction:column;gap:4px"><span class="n">' + E(g.name) + '</span><span class="w">' + E(g.why) + '</span></span>' +
        '<span style="font-size:17px;font-weight:600;min-width:170px;text-align:right;color:' + (bad ? 'var(--warn)' : on && present ? 'var(--ok)' : 'var(--muted)') + '">' + label + '</span><span class="sw' + (on && present ? ' on' : '') + (bad ? ' warn' : '') + '"></span></button>';
    });
    var w = T.settings.cleanup.watch !== false;
    h += '<button class="srow" data-f data-key="cgwatch" data-act="cWatch"><span style="flex:1;display:flex;flex-direction:column;gap:4px"><span class="n">Watch for things turning back on</span><span class="w">Checks each time the TV starts and after updates</span></span><span style="font-size:17px;font-weight:600;min-width:170px;text-align:right;color:' + (w ? 'var(--ok)' : 'var(--muted)') + '">' + (w ? 'On' : 'Off') + '</span><span class="sw' + (w ? ' on' : '') + '"></span></button>';
    h += '<div style="display:flex;align-items:center;gap:24px;padding-top:6px"><span class="soft" style="flex:1;font-size:18px;line-height:1.5">Nothing is deleted, only switched off. If a Google or TCL update turns something back on, Hearth TV switches it off again and tells you.</span><button class="btn small danger" data-f data-key="cRestore" data-act="cRestore">Put everything back</button></div>';
    return h;
  }
  function cleanupSet(g, on) {
    T.call('cleanupSet', g, on).then(function (r) {
      if (r && r.adb === false) { ST.adb = false; T.toast(r.message); T.render(); return; }
      var want = Object.assign({}, T.settings.cleanup); want[g] = on; T.set('cleanup', want);
      ST.clean = T.json('cleanupStatus'); T.render(); var gr = GROUPS.filter(function (x) { return x.id === g; })[0];
      T.toast((gr ? gr.name : g) + ' turned ' + (on ? 'on' : 'off') + (r && r.message ? ' · ' + r.message : ' · saved'));
    });
  }

  // ---------- The page ----------
  T.screen('settings', {
    first: 'nTV Cleanup',
    enter: function (page) {
      if (page && PAGES.indexOf(page) >= 0) ST.page = page;
      ST.clean = T.json('cleanupStatus'); ST.ctrls = T.json('controllers');
      T.call('adbReachable').then(function (r) { ST.adb = r === true; if (T.cur === 'settings' && ST.page === 'TV Cleanup' && !T.layer) T.render(); });
      if (T.n('linkAvailable')) refreshLink();
    },
    render: function () {
      var out = '<div class="screen"><div style="flex:1;display:flex;min-height:0"><nav aria-label="Settings" style="width:440px;flex:none;background:var(--bg2);padding:40px 28px;box-sizing:border-box;display:flex;flex-direction:column;gap:4px"><span style="font-size:34px;font-weight:700;padding:0 20px 20px">Settings</span>';
      PAGES.forEach(function (p) { out += '<button data-f data-key="n' + p + '" data-act="page" data-arg="' + p + '" style="height:56px;padding:0 20px;border-radius:var(--r);font-size:20px;text-align:left;' + (ST.page === p ? 'background:var(--panel3);font-weight:700' : 'color:var(--soft)') + '">' + p + '</button>'; });
      out += '</nav><main data-scrolly style="flex:1;padding:40px 72px 0;display:flex;flex-direction:column;gap:10px;min-width:0;overflow:hidden"><h1 style="margin:0 0 10px;font-size:40px;font-weight:700">' + E(ST.page) + '</h1>';
      if (ST.page === 'TV Cleanup') out += cleanupHtml();
      else {
        var rows = pageRows(ST.page);
        rows.forEach(function (r, i) {
          if (r.kind === 'head') { out += '<span class="h3">' + E(r.name) + '</span>'; return; }
          if (r.kind === 'note') { out += '<span class="muted" style="font-size:17px;line-height:1.5;padding-top:8px">' + E(r.name) + '</span>'; return; }
          var val = r.kind === 'toggle' ? (r.on ? 'On' : 'Off') : r.val;
          var col = r.kind === 'toggle' ? (r.on ? 'var(--ok)' : 'var(--muted)') : r.good ? 'var(--ok)' : 'var(--soft)';
          out += '<button class="srow" data-f data-key="r' + i + '" data-act="row" data-arg="' + i + '"' + (r.dataUpdate ? ' data-update' : '') + '>' +
            (r.img ? '<span style="width:128px;height:72px;border-radius:var(--r);overflow:hidden;position:relative;flex:none;display:block;background:var(--bg)"><img class="art" src="' + E(r.img) + '" alt=""></span>' : '') +
            (r.swatch ? '<span style="width:44px;height:44px;border-radius:22px;background:' + E(r.swatch) + ';flex:none"></span>' : '') +
            '<span style="flex:1;display:flex;flex-direction:column;gap:4px"><span class="n">' + E(r.name) + '</span>' + (r.why ? '<span class="w">' + E(r.why) + '</span>' : '') + '</span>' +
            '<span class="v" style="color:' + col + '">' + E(val || '') + '</span>' +
            (r.kind === 'toggle' ? '<span class="sw' + (r.on ? ' on' : '') + '"></span>' : r.kind === 'choice' ? '<span class="muted" style="font-size:22px">›</span>' : '') + '</button>';
        });
        ST.rows = rows;
      }
      out += '<div style="height:40px;flex:none"></div></main></div>' + T.footer([['OK', 'Change'], ['▲▼', 'Move'], ['↩', 'Back to Home', 'go', 'home']], 'Settings are saved and never change by themselves') + '</div>';
      return out;
    },
    act: function (act, arg) {
      if (act === 'page') {
        if (arg === 'Set up again') { T.go('setup', 'again'); return true; }
        ST.page = arg; if (arg === 'Hearth Link') refreshLink(); if (arg === 'Remote and controllers') ST.ctrls = T.json('controllers');
        T.render('n' + arg); return true;
      }
      if (act === 'row') {
        var r = ST.rows[Number(arg)]; if (!r || !r.fn) return true;
        var res = r.kind === 'toggle' ? r.fn(!r.on) : r.fn();
        if (res !== false && r.kind === 'toggle') T.toast(r.name + ' ' + (!r.on ? 'on' : 'off') + ' · saved');
        if (!T.layer) T.render('r' + arg); return true;
      }
      if (act === 'cCheck') {
        ST.checking = true; T.render('cCheck');
        T.call('cleanupCheck').then(function (r) { ST.checking = false; ST.adb = r && r.adb; ST.clean = T.json('cleanupStatus'); T.loadSettings(); T.toast(r && r.fixed && r.fixed.length ? 'Switched off again: ' + r.fixed.join(', ') : 'Check finished · nothing new turned on'); T.render('cCheck'); });
        return true;
      }
      if (act === 'cToggle' && arg === 'updates') { T.toast('Hearth TV never switches off the TV\'s own updates: it checks after each one instead'); return true; }
      if (act === 'cToggle') { var gs = (ST.clean || {})[arg] || {}; if (gs.present === false) { T.toast('Nothing from this group is on this TV'); return true; } cleanupSet(arg, !gs.on); return true; }
      if (act === 'cWatch') { T.set('cleanup.watch', !(T.settings.cleanup.watch !== false)); T.render('cgwatch'); return true; }
      if (act === 'cRestore') {
        T.open({ id: 'restore', first: 'rsKeep',
          render: function () { return '<div class="dim"><div class="dialog on-panel" role="dialog" aria-label="Put everything back?" style="width:760px"><span style="font-size:32px;font-weight:700">Put everything back?</span><span class="soft" style="font-size:19px;line-height:1.5">This turns the ads, TCL\'s screen tracking and TCL\'s extras back on, and makes Google TV the home screen again. Hearth TV stays installed, so you can clean up again any time.</span><div style="display:flex;gap:14px;padding-top:12px"><button class="btn primary" style="flex:1" data-f data-key="rsKeep" data-act="close">Keep it clean</button><button class="btn danger" style="flex:1" data-f data-key="rsDo" data-act="do">Put everything back</button></div></div></div>'; },
          act: function (a) { if (a === 'do') { T.close(); ['ads', 'acr', 'tcl', 'home', 'voice', 'updates'].forEach(function (g) { cleanupSet(g, true); }); T.toast('Everything is back · Google TV is the home screen again'); return true; } return false; }
        });
        return true;
      }
      return false;
    }
  });
})();
