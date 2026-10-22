#!/usr/bin/env python3
"""Tries Hearth Browser (Firefox with Hearth's parts) the way people use it, for CI (.github/workflows/hearth-browser.yml).

  hearth-browser-test.py --firefox PATH --out DIR [--mode desktop|game] [--extras]

Starts that Firefox headless with a new profile and drives it through Marionette (Firefox's own remote control,
spoken here directly: a few lines of JSON over TCP, so nothing needs installing), then checks:
  - Hearth's policies are active (and none failed), its start-up script ran, the look, the start page as home page
    and new tab, the Keyboard button
  - the on-screen keyboard types into a web page: arrow keys / Tab move between its keys, Enter types one, Alt+Left
    deletes, as the controller does (in Game Mode it also comes up with the address bar)
  - game mode: the window fills the screen, bigger buttons, new windows as tabs
  - Google's sign-in page opens without "This browser or app may not be secure"
  - with --extras: uBlock Origin installs from Mozilla's add-on site, and Widevine (protected video) downloads
Screenshots (the whole window, with the toolbar and keyboard) and report-<mode>.json go to DIR. Exit 1 when a
required check fails; the extras only warn (they depend on Mozilla's and Google's servers).
"""
import argparse
import base64
import json
import os
import socket
import subprocess
import sys
import tempfile
import time

failures = []
warnings = []


def ok(cond, what, required=True):
    print(('ok: ' if cond else ('FAIL: ' if required else 'warning: ')) + what, flush=True)
    if not cond and os.environ.get('GITHUB_ACTIONS'):
        print('::%s title=Hearth Browser::%s' % ('error' if required else 'warning', what.replace('\n', ' ')[:900]), flush=True)
    if not cond:
        (failures if required else warnings).append(what)
    return cond


class Marionette:
    def __init__(self, port):
        deadline = time.time() + 90
        while True:
            try:
                self.s = socket.create_connection(('127.0.0.1', port), timeout=5)
                break
            except OSError:
                if time.time() > deadline:
                    raise
                time.sleep(0.5)
        self.s.settimeout(180)
        self.buf = b''
        self.id = 0
        hello = self.read()
        print('marionette:', hello, flush=True)

    def read(self):
        while b':' not in self.buf:
            self.buf += self.recv()
        n, _, rest = self.buf.partition(b':')
        n = int(n)
        while len(rest) < n:
            rest += self.recv()
        self.buf = rest[n:]
        return json.loads(rest[:n])

    def recv(self):
        d = self.s.recv(1 << 20)
        if not d:
            raise ConnectionError('Firefox closed the connection')
        return d

    def cmd(self, name, params=None):
        self.id += 1
        body = json.dumps([0, self.id, name, params or {}]).encode()
        self.s.sendall(str(len(body)).encode() + b':' + body)
        while True:
            msg = self.read()
            if msg[0] == 1 and msg[1] == self.id:
                if msg[2]:
                    raise RuntimeError('%s: %s' % (name, msg[2].get('message') or msg[2]))
                return msg[3]

    def js(self, script, *args, timeout=60):
        """Runs an async script: it calls resolve(value) (its last argument) when done."""
        self.cmd('WebDriver:SetTimeouts', {'script': timeout * 1000})
        body = 'const resolve = arguments[arguments.length - 1];\n(async () => { ' + script + ' })().then(resolve, e => resolve({error: String(e)}));'
        return self.cmd('WebDriver:ExecuteAsyncScript', {'script': body, 'args': list(args)}).get('value')

    def context(self, which):
        self.cmd('Marionette:SetContext', {'value': which})

    def shot(self, path):
        data = self.cmd('WebDriver:TakeScreenshot', {'full': False}).get('value')
        with open(path, 'wb') as f:
            f.write(base64.b64decode(data))
        print('screenshot:', path, flush=True)


CHROME_INFO = r'''
  const { HearthBrowser } = ChromeUtils.importESModule("resource://hearth/hearth.sys.mjs");
  for (let i = 0; i < 100 && !Services.prefs.getBoolPref("hearth.keyboard.ready", false); i++) await new Promise(r => setTimeout(r, 200));
  const p = Services.prefs;
  const get = n => { const t = p.getPrefType(n); return t === p.PREF_BOOL ? p.getBoolPref(n) : t === p.PREF_INT ? p.getIntPref(n) : t === p.PREF_STRING ? p.getStringPref(n) : null; };
  let newTab = null;
  try { newTab = ChromeUtils.importESModule("resource:///modules/AboutNewTab.sys.mjs").AboutNewTab.newTabURL; } catch (e) { newTab = String(e); }
  const errors = Services.console.getMessageArray().map(m => m.message || String(m))
    .filter(m => /Hearth Browser|[Pp]olic(y|ies)/.test(m)).slice(-20);
  return {
    version: Services.appinfo.version,
    policies: Services.policies.status === Services.policies.ACTIVE,
    active: Object.keys(Services.policies.getActivePolicies() || {}),
    hearth: { start: HearthBrowser.START, game: HearthBrowser.GAME, big: HearthBrowser.BIG, scale: HearthBrowser.SCALE },
    prefs: Object.fromEntries(["hearth.started", "hearth.keyboard.ready", "browser.startup.homepage", "media.eme.enabled",
      "toolkit.legacyUserProfileCustomizations.stylesheets", "datareporting.healthreport.uploadEnabled", "app.update.enabled",
      "browser.uidensity", "layout.css.devPixelsPerPx", "browser.link.open_newwindow.restriction", "extensions.activeThemeID"].map(n => [n, get(n)])),
    newTab,
    keyboardButton: !!document.getElementById("hearth-keyboard-button"),
    keyboard: !!document.getElementById("hearth-osk"),
    window: [window.outerWidth, window.outerHeight, window.screen.width, window.screen.height],
    errors,
  };
'''

# A hardware-like key press (what a keyboard, or the controller through Hearth Input, sends), through its own
# text input processor, so it goes through Firefox's shortcuts like a real key.
CHROME_KEY = r'''
  const [key, code, keyCode, alt] = arguments;
  const tip = Components.classes["@mozilla.org/text-input-processor;1"].createInstance(Components.interfaces.nsITextInputProcessor);
  if (!tip.beginInputTransaction(window, () => true)) return "no input transaction";
  const altKey = new window.KeyboardEvent("", { key: "Alt", code: "AltLeft", keyCode: 18 });
  if (alt) tip.keydown(altKey);
  const e = new window.KeyboardEvent("", { key, code, keyCode });
  tip.keydown(e); tip.keyup(e);
  if (alt) tip.keyup(altKey);
  await new Promise(r => setTimeout(r, 400));
  const { HearthBrowser } = ChromeUtils.importESModule("resource://hearth/hearth.sys.mjs");
  const kb = HearthBrowser.keyboardOf(window);
  return { key, open: kb.isOpen, r: kb.r, c: kb.c, heard: kb.heard.slice(-4), focus: String(document.activeElement && document.activeElement.localName) };
'''


PAGE = 'data:text/html;charset=utf-8,' + ('<title>Typing</title><body style="background:%23222;color:%23eee;font:24px sans-serif">'
                                          '<p>Typed with Hearth Browser\'s keyboard:</p><input id="i" style="font-size:28px;width:80%25"></body>')


def run_mode(ff, out, mode, extras, port):
    game = mode == 'game'
    prof = tempfile.mkdtemp(prefix='hearth-browser-test-')
    with open(os.path.join(prof, 'user.js'), 'w') as f:
        f.write('user_pref("marionette.port", %d);\n' % port)
        # Marionette's own automation defaults would replace Hearth's home page and new tab.
        f.write('user_pref("remote.prefs.recommended", false);\n')
    env = dict(os.environ)
    for k in ('HEARTH_BROWSER_GAME', 'HEARTH_BROWSER_BIG', 'HEARTH_BROWSER_SCALE', 'HEARTH_BROWSER_ACCENT'):
        env.pop(k, None)
    env.update({'MOZ_HEADLESS': '1', 'MOZ_HEADLESS_WIDTH': '1920' if game else '1280', 'MOZ_HEADLESS_HEIGHT': '1080' if game else '800'})
    if game:
        env.update({'HEARTH_BROWSER_GAME': '1', 'HEARTH_BROWSER_BIG': '1', 'HEARTH_BROWSER_SCALE': '1.5'})
    log = open(os.path.join(out, 'firefox-%s.log' % mode), 'w')
    proc = subprocess.Popen([ff, '--headless', '--marionette', '-remote-allow-system-access', '--no-remote', '--profile', prof], env=env, stdout=log, stderr=subprocess.STDOUT)
    report = {'mode': mode}
    try:
        m = Marionette(port)
        m.cmd('WebDriver:NewSession', {'capabilities': {'alwaysMatch': {'acceptInsecureCerts': False}}})
        m.context('chrome')
        info = m.js(CHROME_INFO, timeout=60)
        report['chrome'] = info
        print(json.dumps(info, indent=2), flush=True)
        if os.environ.get('GITHUB_ACTIONS'):
            print('::notice title=Hearth Browser %s::%s' % (mode, json.dumps(info)[:3000]), flush=True)
        ok(info.get('policies') is True, '[%s] Hearth\'s policies are active' % mode)
        for name in ('DisableTelemetry', 'EncryptedMediaExtensions', 'ExtensionSettings', 'DisableProfileImport', 'UserMessaging'):
            ok(name in (info.get('active') or []), '[%s] policy %s is applied' % (mode, name))
        pol_errors = [e for e in info.get('errors') or [] if 'olic' in e and ('rror' in e or 'fail' in e.lower() or 'nvalid' in e or 'nknown' in e)]
        ok(not pol_errors, '[%s] no policy errors %s' % (mode, pol_errors[:3]))
        hb_errors = [e for e in info.get('errors') or [] if 'Hearth Browser' in e]
        ok(not hb_errors, '[%s] Hearth\'s start-up script reported no errors %s' % (mode, hb_errors[:3]))
        prefs = info.get('prefs') or {}
        ok(bool(prefs.get('hearth.started')), '[%s] Hearth\'s start-up script ran (hearth.cfg)' % mode)
        ok(prefs.get('hearth.keyboard.ready') is True, '[%s] the window got Hearth\'s parts' % mode)
        ok(str(prefs.get('browser.startup.homepage', '')).endswith('/hearth/start.html'), '[%s] the start page is the home page' % mode)
        ok(str(info.get('newTab', '')).endswith('/hearth/start.html'), '[%s] the start page is the new tab' % mode)
        ok(prefs.get('media.eme.enabled') is True, '[%s] protected video (DRM) is on' % mode)
        ok(prefs.get('datareporting.healthreport.uploadEnabled') is not True, '[%s] telemetry is off' % mode)
        ok(info.get('keyboardButton') and info.get('keyboard'), '[%s] the Keyboard button and the keyboard are there' % mode)
        if game:
            ok(prefs.get('browser.uidensity') == 2 and prefs.get('layout.css.devPixelsPerPx') == '1.5', '[game] bigger buttons and text')
            ok(prefs.get('browser.link.open_newwindow.restriction') == 0, '[game] new windows open as tabs')
        else:
            ok(prefs.get('browser.uidensity') in (None, 0), '[desktop] normal size')

        # The start page.
        m.context('content')
        m.cmd('WebDriver:Navigate', {'url': info['hearth']['start']})
        title = m.cmd('WebDriver:GetTitle').get('value')
        links = m.js('return document.querySelectorAll("nav a").length;')
        ok(title == 'New tab' and links >= 8, '[%s] the start page shows (%r, %s sites)' % (mode, title, links))
        m.context('chrome')
        if game:
            w = m.js('await new Promise(r => setTimeout(r, 2500)); return [window.outerWidth, window.outerHeight, window.screen.width, window.screen.height];')
            report['window'] = w
            ok(w[0] >= w[2] and w[1] >= w[3], '[game] the window fills the screen %s' % w)
        m.shot(os.path.join(out, '%s-1-start.png' % mode))

        # The on-screen keyboard, typing into a web page.
        m.context('content')
        m.cmd('WebDriver:Navigate', {'url': PAGE})
        m.js('document.getElementById("i").focus(); return true;')
        m.context('chrome')
        m.js('gBrowser.selectedBrowser.focus(); await new Promise(r => setTimeout(r, 300)); return true;')
        m.js('gBrowser.selectedBrowser.focus(); await new Promise(r => setTimeout(r, 300)); '
             'const { HearthBrowser } = ChromeUtils.importESModule("resource://hearth/hearth.sys.mjs"); '
             'const kb = HearthBrowser.keyboardOf(window); kb.layout = "abc"; kb.r = 1; kb.c = 0; kb.open(); kb.render(); return kb.isOpen;')
        steps = []
        for key in (('ArrowDown', 'ArrowDown', 40, False), ('Return', None, None, None)):
            if key[0] == 'Return':
                steps.append(m.js(CHROME_KEY, 'Enter', 'Enter', 13, False))
            else:
                steps.append(m.js(CHROME_KEY, *key))
        report['keys'] = steps
        print('keys:', steps, flush=True)
        m.context('content')
        v1 = m.js('return document.getElementById("i").value;')
        ok(v1 == 'a', '[%s] keyboard: Down then Enter types "a" into the page (got %r; %s)' % (mode, v1, steps))
        m.context('chrome')
        m.js(CHROME_KEY, 'ArrowLeft', 'ArrowLeft', 37, True)
        m.context('content')
        v2 = m.js('return document.getElementById("i").value;')
        ok(v2 == '', '[%s] keyboard: Alt+Left (B) deletes it (got %r)' % (mode, v2))
        m.context('chrome')
        m.js('const { HearthBrowser } = ChromeUtils.importESModule("resource://hearth/hearth.sys.mjs"); const kb = HearthBrowser.keyboardOf(window); '
             'kb.shift = true; kb.typeChar("H"); kb.shift = false; for (const ch of "earth @ 7") kb.typeChar(ch); kb.typeKey("Backspace"); kb.typeChar("1"); kb.render(); return true;')
        m.context('content')
        v3 = m.js('return document.getElementById("i").value;')
        ok(v3 == 'Hearth @ 1', '[%s] keyboard: typing letters, a space, @ and Backspace (got %r)' % (mode, v3))
        m.context('chrome')
        m.shot(os.path.join(out, '%s-2-keyboard.png' % mode))
        closed = m.js(CHROME_KEY, 'Escape', 'Escape', 27, False) and m.js(
            'const { HearthBrowser } = ChromeUtils.importESModule("resource://hearth/hearth.sys.mjs"); return !HearthBrowser.keyboardOf(window).isOpen;')
        ok(closed is True, '[%s] keyboard: Esc closes it' % mode)
        m.context('content')
        m.js('document.getElementById("i").focus(); return true;')
        m.context('chrome')
        # With the keyboard closed, keys reach the page again.
        m.js(CHROME_KEY, 'x', 'KeyX', 88, False)
        m.context('content')
        v4 = m.js('return document.getElementById("i").value;')
        ok(v4.endswith('x'), '[%s] with the keyboard closed, keys reach the page (got %r)' % (mode, v4))
        if game:
            m.context('chrome')
            auto = m.js('gURLBar.focus(); await new Promise(r => setTimeout(r, 500)); '
                        'const { HearthBrowser } = ChromeUtils.importESModule("resource://hearth/hearth.sys.mjs"); return HearthBrowser.keyboardOf(window).isOpen;')
            ok(auto is True, '[game] the keyboard comes up with the address bar')
            m.shot(os.path.join(out, 'game-3-address-bar.png'))
            m.js('const { HearthBrowser } = ChromeUtils.importESModule("resource://hearth/hearth.sys.mjs"); HearthBrowser.keyboardOf(window).close(); return true;')

        # Google's sign-in page.
        m.context('content')
        try:
            m.cmd('WebDriver:Navigate', {'url': 'https://accounts.google.com/'})
            time.sleep(4)
            g = m.js('return {url: location.href, title: document.title, text: (document.body && document.body.innerText || "").slice(0, 4000)};')
            report['google'] = g
            with open(os.path.join(out, '%s-google.txt' % mode), 'w', encoding='utf-8') as f:
                f.write(json.dumps(g, indent=2, ensure_ascii=False))
            text = (g.get('text') or '').lower()
            ok('not be secure' not in text and 'browser or app may not' not in text, '[%s] Google sign-in: no "This browser or app may not be secure"' % mode)
            ok('sign in' in text or 'email' in text, '[%s] Google sign-in page shows its form (%r)' % (mode, g.get('title')), required=False)
            if extras:
                drm = m.js('try { await navigator.requestMediaKeySystemAccess("com.widevine.alpha", [{initDataTypes: ["cenc"], '
                           'videoCapabilities: [{contentType: \'video/mp4; codecs="avc1.42E01E"\'}]}]); return "widevine available"; } catch (e) { return String(e); }', timeout=30)
                report['drm-before-download'] = drm
                print('DRM on the page (before Widevine is downloaded):', drm, flush=True)
        except Exception as e:
            ok(False, '[%s] Google sign-in page (%s)' % (mode, e), required=False)
        m.context('chrome')
        m.shot(os.path.join(out, '%s-3-google.png' % mode))

        if extras:
            ub = m.js('const { AddonManager } = ChromeUtils.importESModule("resource://gre/modules/AddonManager.sys.mjs"); '
                      'for (let i = 0; i < 90; i++) { const a = await AddonManager.getAddonByID("uBlock0@raymondhill.net"); '
                      'if (a) return {name: a.name, version: a.version, active: a.isActive}; await new Promise(r => setTimeout(r, 1000)); } return null;', timeout=120)
            report['ublock'] = ub
            ok(bool(ub and ub.get('active')), 'uBlock Origin is installed from Mozilla\'s add-on site (%s)' % ub, required=False)
            wv = m.js('try { const { GMPInstallManager } = ChromeUtils.importESModule("resource://gre/modules/GMPInstallManager.sys.mjs"); '
                      'const r = await new GMPInstallManager().simpleCheckAndInstall(); '
                      'return {result: JSON.stringify(r), version: Services.prefs.getStringPref("media.gmp-widevinecdm.version", "")}; } catch (e) { return {error: String(e)}; }', timeout=240)
            report['widevine'] = wv
            ok(bool(wv and wv.get('version')), 'Widevine (protected video) downloads (%s)' % wv, required=False)
        try:
            m.cmd('Marionette:Quit', {'flags': ['eForceQuit']})
        except Exception:
            pass
    except Exception as e:
        ok(False, '[%s] the test ran (%s)' % (mode, e))
    finally:
        try:
            proc.wait(20)
        except subprocess.TimeoutExpired:
            proc.kill()
        log.close()
        with open(os.path.join(out, 'report-%s.json' % mode), 'w') as f:
            json.dump(report, f, indent=2)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--firefox', required=True)
    ap.add_argument('--out', required=True)
    ap.add_argument('--mode', choices=['desktop', 'game', 'both'], default='both')
    ap.add_argument('--extras', action='store_true')
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    modes = ['desktop', 'game'] if a.mode == 'both' else [a.mode]
    for i, mode in enumerate(modes):
        run_mode(a.firefox, a.out, mode, a.extras and i == 0, 2828 + i)
    print('\n%d failed, %d warnings' % (len(failures), len(warnings)))
    for w in warnings:
        print('warning: ' + w)
    for f in failures:
        print('FAILED: ' + f)
    with open(os.path.join(a.out, 'summary.txt'), 'w') as f:
        f.write('failed:\n' + '\n'.join(failures) + '\nwarnings:\n' + '\n'.join(warnings) + '\n')
    sys.exit(1 if failures else 0)


if __name__ == '__main__':
    main()
