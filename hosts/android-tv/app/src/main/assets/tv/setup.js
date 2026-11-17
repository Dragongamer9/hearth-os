/* First start (mockup 1): Welcome, Your PC (Hearth Link and My Media), Apps for Home, Clean up the TV, Done.
   Settings › Set up again runs it again. */
(function () {
  'use strict';
  var T = window.hearthTV, E = T.esc;
  var SU = { step: 0, pc: 'looking', pcName: '', servers: [], picked: null, apps: [], jobs: null, run: -1, code: '', again: false };
  var JOBS = [['ads', 'Ads and sponsored rows'], ['acr', 'Screen tracking (ACR)'], ['tcl', 'TCL Channel and extras'], ['home', 'Make Hearth TV the home screen (Google TV kept as backup)'], ['watch', 'Watch for things turning back on']];
  function findPc() {
    SU.pc = 'looking';
    T.Media.jfServers().then(function (s) {
      SU.servers = s;
      var peer = T.data.linkPeer;
      SU.pcName = (s[0] && s[0].name) || (peer && peer.name) || '';
      SU.pc = T.Media.ready ? 'connected' : s.length ? 'found' : 'none';
      if (T.cur === 'setup' && !T.layer) T.render();
    });
    if (T.n('linkAvailable')) T.call('link', 'pair.open', JSON.stringify({ mine: true })).then(function (r) { var d = r && r.ok ? r.data : {}; SU.code = String(d.code || d.lanCode || '').replace(/\D/g, ''); if (T.cur === 'setup' && !T.layer) T.render(); });
  }
  T.screen('setup', {
    first: 'next',
    enter: function (arg) {
      SU.again = arg === 'again'; SU.step = 0; SU.run = -1;
      var all = T.json('apps') || [], home = T.homeApps().home.map(function (a) { return a.pkg; });
      SU.apps = all; SU.picked = {}; (home.length ? home : all.slice(0, 6).map(function (a) { return a.pkg; })).forEach(function (p) { SU.picked[p] = true; });
      SU.jobs = { ads: true, acr: true, tcl: true, home: true, watch: true };
    },
    back: function () { if (SU.step > 0 && SU.run < 0) { SU.step--; T.render(); return true; } return !SU.again; },
    leave: function () { T.call('link', 'pair.close', '{}'); },
    render: function () {
      var names = ['Welcome', 'Your PC', 'Apps', 'Clean up', 'Done'], wp = T.lookWallpaper || 'theme/wallpapers/slate-horizon.jpg';
      var out = '<div class="screen" style="display:block"><img class="art" src="' + E(wp) + '" alt=""><div style="position:absolute;inset:0;background:rgba(14,18,26,.55)"></div>' +
        '<div class="on-panel" style="position:absolute;left:50%;top:50%;transform:translate(-50%,-50%);width:1240px;background:rgba(27,40,56,.94);border:1px solid var(--line);border-radius:8px;padding:52px 64px;box-sizing:border-box;display:flex;flex-direction:column;gap:26px">' +
        '<div style="display:flex;align-items:center;gap:12px">' + T.ICON.flame + '<span style="font-size:24px;font-weight:700">Hearth TV</span><div style="flex:1"></div>';
      names.forEach(function (n, i) { out += '<span style="font-size:16px;font-weight:600;padding:6px 14px;border-radius:16px;' + (i === SU.step ? 'background:var(--panel3);color:var(--text)' : i < SU.step ? 'color:var(--ok)' : 'color:#5d6b78') + '">' + n + '</span>'; });
      out += '</div>';
      if (SU.step === 0) {
        out += '<span style="font-size:60px;font-weight:700;letter-spacing:-.02em;line-height:1.05">Welcome to Hearth TV</span><span class="soft" style="font-size:24px;line-height:1.5;max-width:900px">A cleaner home for your shows: your apps, your PC\'s library and your saved links in one place, with the ads and tracking switched off. It takes about two minutes.</span><div style="display:flex;gap:22px;padding-top:6px">';
        [['No ads', 'Sponsored rows and promoted apps switched off.'], ['No tracking', 'TCL\'s screen tracking switched off.'], ['Nothing deleted', 'Everything can be put back in one step.']].forEach(function (p) { out += '<div class="panel" style="flex:1;padding:22px;display:flex;flex-direction:column;gap:8px"><span style="font-size:21px;font-weight:700">' + p[0] + '</span><span class="muted" style="font-size:17px;line-height:1.45">' + p[1] + '</span></div>'; });
        out += '</div>';
      } else if (SU.step === 1) {
        var state = { looking: 'Looking…', found: 'Found', none: 'Not found', connected: '✓ Connected' }[SU.pc];
        out += '<span style="font-size:48px;font-weight:700">Connect to your PC</span><span class="soft" style="font-size:22px;line-height:1.5">Hearth TV plays your shows and movies straight from your PC through Hearth Link. Nothing is copied to the TV.</span><div style="display:flex;gap:22px">' +
          '<button data-f data-key="pc" data-act="pc" style="flex:1;display:flex;align-items:center;gap:22px;padding:24px;border-radius:6px;background:var(--panel);border:2px solid ' + (SU.pc === 'connected' ? 'var(--ok)' : 'var(--focus)') + '"><span style="width:64px;height:64px;border-radius:32px;background:var(--panel3);display:flex;align-items:center;justify-content:center;flex:none">' + T.ICON.pc + '</span>' +
          '<span style="flex:1;display:flex;flex-direction:column;gap:4px;text-align:left"><span style="font-size:24px;font-weight:700">' + E(SU.pcName || 'Your PC') + '</span><span class="muted" style="font-size:17px">' + (SU.servers.length ? 'Hearth OS · Hearth Server · found on your network' : SU.pc === 'looking' ? 'Looking on your network…' : 'Turn the PC on, or connect later in Settings') + '</span></span><span style="font-size:19px;font-weight:700;color:' + (SU.pc === 'connected' ? 'var(--ok)' : 'var(--focus)') + '">' + state + '</span></button>' +
          '<div class="panel" style="width:360px;flex:none;padding:22px;box-sizing:border-box;display:flex;flex-direction:column;gap:6px;align-items:center;text-align:center"><span class="muted" style="font-size:17px">Or type this code in Hearth Link</span><span style="font-size:52px;font-weight:700;letter-spacing:.14em;color:var(--focus)">' + (SU.code ? E(SU.code.slice(0, 3) + ' ' + SU.code.slice(3)) : '······') + '</span></div></div>' +
          '<button data-f data-key="skip" data-act="skip" style="align-self:flex-start;font-size:18px;color:var(--muted);text-decoration:underline">Skip: I\'ll connect later in Settings</button>';
      } else if (SU.step === 2) {
        out += '<span style="font-size:48px;font-weight:700">Pick your apps for Home</span><span class="soft" style="font-size:22px">The rest stay installed under Apps. You can change this any time.</span><div data-scrolly style="display:grid;grid-template-columns:repeat(5,minmax(0,1fr));gap:16px;max-height:470px;overflow:hidden;padding:8px;margin:-8px">';
        SU.apps.forEach(function (a, i) { var v = !!SU.picked[a.pkg]; out += '<button class="tile" data-f data-key="ap' + i + '" data-act="app" data-arg="' + E(a.pkg) + '" style="height:104px;background:' + T.tileColor(a.pkg) + ';flex-direction:column;gap:6px;opacity:' + (v ? 1 : .55) + (v ? ';outline:2px solid var(--ok);outline-offset:3px' : '') + '"><span style="font-size:20px;font-weight:700">' + E(a.label) + '</span><span style="font-size:15px;font-weight:400;color:' + (v ? 'var(--ok)' : '#c3ccd4') + '">' + (v ? '✓ On Home' : 'Not on Home') + '</span></button>'; });
        out += '</div>';
      } else if (SU.step === 3) {
        var cleaned = SU.run >= JOBS.length;
        out += '<span style="font-size:48px;font-weight:700">' + (cleaned ? 'Your TV is clean' : 'Clean up the TV') + '</span><span class="soft" style="font-size:22px">' + (cleaned ? 'All done. The next screen is the last one.' : 'Pick what to switch off. Each one can be turned back on later.') + '</span><div style="display:flex;flex-direction:column;gap:8px">';
        JOBS.forEach(function (j, i) {
          var v = SU.jobs[j[0]], done = SU.run > i, doing = SU.run === i;
          out += '<button data-f data-key="j' + i + '" data-act="job" data-arg="' + j[0] + '" style="display:flex;align-items:center;gap:18px;height:64px;padding:0 22px;border-radius:var(--r);background:var(--panel);text-align:left"><span style="width:32px;height:32px;border-radius:6px;border:2px solid ' + (v ? 'var(--focus)' : '#5d6b78') + ';background:' + (v ? 'var(--focus)' : 'transparent') + ';display:flex;align-items:center;justify-content:center;font-size:18px;font-weight:700;color:var(--ink);flex:none">' + (v ? '✓' : '') + '</span>' +
            '<span style="flex:1;font-size:20px;font-weight:600">' + E(j[1]) + '</span><span style="font-size:18px;font-weight:600;color:' + (done && SU.result[j[0]] !== false ? 'var(--ok)' : 'var(--muted)') + '">' + (!v ? 'Skipped' : done ? (SU.result[j[0]] === false ? 'Needs your OK on the TV' : 'Done') : doing ? 'Working…' : SU.run >= 0 ? 'Waiting' : '') + '</span></button>';
        });
        out += '</div><span class="muted" style="font-size:17px">Nothing is deleted. Google TV\'s home stays installed as a backup, and Settings › TV Cleanup can put everything back.</span>';
      } else {
        var n = Object.keys(SU.picked).filter(function (k) { return SU.picked[k]; }).length;
        out += '<span style="font-size:60px;font-weight:700;letter-spacing:-.02em">You\'re all set</span><span class="soft" style="font-size:24px;line-height:1.5;max-width:900px">The Home button on your remote now opens Hearth TV. Your settings are saved and never change by themselves.</span><div style="display:flex;gap:22px">';
        [['Your PC', T.Media.ready ? (SU.pcName || 'Your PC') + ' connected' : 'Not connected yet'], ['Apps on Home', n + ' apps + My Media and Theater'], ['TV Cleanup', JOBS.filter(function (j) { return SU.jobs[j[0]] && SU.result && SU.result[j[0]] !== false; }).length + ' of 5 done']].forEach(function (x) { out += '<div class="panel" style="flex:1;padding:22px;display:flex;flex-direction:column;gap:6px"><span class="muted" style="font-size:17px">' + x[0] + '</span><span style="font-size:22px;font-weight:700">' + E(x[1]) + '</span></div>'; });
        out += '</div>';
      }
      var running = SU.run >= 0 && SU.run < JOBS.length;
      out += '<div style="display:flex;gap:14px;padding-top:8px">' + (SU.step > 0 && SU.step < 4 && !running ? '<button class="btn" style="height:64px;padding:0 30px" data-f data-key="back" data-act="back">Back</button>' : '') + '<div style="flex:1"></div>';
      var label = [SU.again ? 'Start again' : 'Start', SU.pc === 'connected' ? 'Next' : 'Connect', 'Next', SU.run >= JOBS.length ? 'Next' : running ? 'Cleaning up…' : 'Clean up my TV', 'Go to Home'][SU.step];
      out += '<button class="btn primary" style="height:64px;padding:0 40px' + (running ? ';opacity:.7' : '') + '" data-f data-key="next" data-act="next">' + label + '</button></div></div></div>';
      return out;
    },
    act: function (act, arg) {
      if (act === 'back') { SU.step--; T.render(); return true; }
      if (act === 'skip') { SU.step = 2; T.render('next'); return true; }
      if (act === 'pc') { connect(); return true; }
      if (act === 'app') { SU.picked[arg] = !SU.picked[arg]; T.render(); return true; }
      if (act === 'job') { if (SU.run < 0) { SU.jobs[arg] = !SU.jobs[arg]; T.render(); } return true; }
      if (act === 'next') {
        if (SU.step === 0) { SU.step = 1; findPc(); T.render('pc'); return true; }
        if (SU.step === 1) { if (SU.pc === 'connected') { SU.step = 2; T.render('next'); } else connect(); return true; }
        if (SU.step === 2) {
          var list = SU.apps.filter(function (a) { return SU.picked[a.pkg]; }).map(function (a) { return a.pkg; });
          T.set('homeApps', list); SU.step = 3; T.render('next'); return true;
        }
        if (SU.step === 3) { if (SU.run >= JOBS.length) { SU.step = 4; T.render('next'); } else if (SU.run < 0) cleanUp(); return true; }
        if (SU.step === 4) { T.set('setupDone', true); T.stack = []; T.go('home', null, { replace: true }); return true; }
      }
      return false;
    }
  });
  function connect() {
    T.mediaSignIn();
    var wait = setInterval(function () { if (T.Media.ready) { clearInterval(wait); SU.pc = 'connected'; if (T.cur === 'setup') T.render('next'); } }, 1500);
    setTimeout(function () { clearInterval(wait); }, 600000);
  }
  function cleanUp() {
    SU.run = 0; SU.result = {}; T.render('next');
    var want = Object.assign({}, T.settings.cleanup);
    var step = function () {
      if (SU.run >= JOBS.length) { T.set('cleanup', want); T.render('next'); return; }
      var j = JOBS[SU.run][0];
      if (!SU.jobs[j]) { SU.run++; T.render('next'); setTimeout(step, 300); return; }
      if (j === 'watch') { want.watch = true; SU.result.watch = true; SU.run++; T.render('next'); setTimeout(step, 300); return; }
      T.call('cleanupSet', j, false).then(function (r) {
        SU.result[j] = !!(r && r.ok); if (r && r.ok) want[j] = false;
        if (r && r.adb === false) T.toast('The TV needs to allow Hearth TV once: pick "Always allow" on the TV, then press Clean up again');
        SU.run++; T.render('next'); setTimeout(step, 400);
      });
    };
    step();
  }
})();
