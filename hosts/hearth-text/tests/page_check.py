#!/usr/bin/python3
"""Checks Hearth Text's page (ui/) in Chromium with a stand-in for the Rust side: it starts without script errors,
colors code, types and saves, finds and replaces, goes to a line, shows read-only files with Edit as administrator,
shows encodings and line endings, and asks before throwing changes away.

  python3 tests/page_check.py [--shots DIR]        (needs: pip install playwright; playwright install chromium)
"""
import functools
import http.server
import json
import os
import sys
import threading

from playwright.sync_api import sync_playwright

UI = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'ui')
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from make_samples import SAMPLES  # noqa: E402

SHOTS = sys.argv[sys.argv.index('--shots') + 1] if '--shots' in sys.argv else None


def opened(path, text, **kw):
    d = {'path': path, 'name': path.rsplit('/', 1)[-1], 'text': text, 'encoding': 'utf-8', 'bom': False, 'eol': 'lf', 'mixed_eol': False,
         'binary': False, 'lossy': False, 'size': len(text.encode()), 'mtime': 1000, 'writable': True, 'too_big': False, 'part': None,
         'shown': len(text.encode())}
    d.update(kw)
    return d


FILES = {'/home/me/' + n: opened('/home/me/' + n, t) for n, t in SAMPLES.items()}
FILES['/etc/fstab'] = opened('/etc/fstab', '# /etc/fstab\nUUID=1234 / btrfs defaults 0 0\n', writable=False)
FILES['/home/me/crlf.ini'] = opened('/home/me/crlf.ini', '[section]\nkey = value\n', eol='crlf')
FILES['/home/me/latin1.txt'] = opened('/home/me/latin1.txt', 'Café\n', encoding='latin-1')
FILES['/var/log/huge.log'] = opened('/var/log/huge.log', ''.join('2026-10-22 12:00:%02d INFO [w] line %d\n' % (i % 60, i) for i in range(3000)),
                                    size=62914560, too_big=True, part='start', shown=8388608)

MOCK = r"""
(() => {
  const files = %s;
  const calls = [];
  const handlers = {};
  let saved = {};
  window.__mock = { calls, handlers, saved: () => saved, emit: (n, p) => (handlers[n] || []).forEach(h => h({ payload: p })) };
  window.__TAURI__ = {
    core: { invoke: async (cmd, a) => {
      calls.push([cmd, a]);
      switch (cmd) {
        case 'init': return { launch: { files: [], readonly: false, line: null, new: false }, settings: {}, platform: 'linux', home: '/home/me',
          cwd: '/home/me', version: '0.1.0', admin: true, eol: 'lf', limits: { edit: 12582912, view: 50331648, part: 8388608 } };
        case 'open_file': { const f = files[a.path]; if (!f) throw "“" + a.path.split('/').pop() + "” isn't there any more"; return { ...f, ...(a.encoding ? { encoding: a.encoding } : {}) }; }
        case 'stat_file': { const f = files[a.path]; return f ? { exists: true, size: f.size, mtime: f.mtime, writable: f.writable } : { exists: false, size: 0, mtime: 0, writable: true }; }
        case 'save_file': saved[a.path] = a; if (files[a.path]) files[a.path].mtime += 1; return { exists: true, size: a.text.length, mtime: files[a.path] ? files[a.path].mtime : 5, writable: true };
        case 'pick_open': return ['/home/me/app.json'];
        case 'pick_save': return '/home/me/Documents/untitled.txt';
        case 'settings_save': return null;
        case 'quit': window.__quit = true; return null;
      }
      throw 'unknown command ' + cmd;
    } },
    event: { listen: async (n, h) => { (handlers[n] = handlers[n] || []).push(h); return () => {}; } },
    window: { getCurrentWindow: () => ({ setTitle: async () => {} }) },
  };
})();
""" % json.dumps(FILES)


def main():
    handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=os.path.abspath(UI))
    handler.log_message = lambda *a: None
    srv = http.server.ThreadingHTTPServer(('127.0.0.1', 0), handler)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    url = 'http://127.0.0.1:%d/index.html' % srv.server_address[1]
    errors, fails = [], []
    if SHOTS:
        os.makedirs(SHOTS, exist_ok=True)

    def check(cond, what):
        print(('ok    ' if cond else 'FAIL  ') + what)
        if not cond:
            fails.append(what)

    with sync_playwright() as p:
        b = p.chromium.launch()
        pg = b.new_page(viewport={'width': 1100, 'height': 720})
        pg.on('pageerror', lambda e: errors.append(str(e)))
        pg.on('console', lambda m: m.type == 'error' and errors.append(m.text))
        pg.add_init_script(MOCK)
        pg.goto(url)
        pg.wait_for_function('window.__hearthText')
        shot = lambda n: SHOTS and pg.screenshot(path=os.path.join(SHOTS, n + '.png'))
        ht = 'window.__hearthText'
        status = lambda: pg.inner_text('#status')

        check(pg.is_visible('#start'), 'an empty tab shows the start card')
        shot('01-start')

        pg.evaluate(ht + ".openPath('/home/me/sample.sh')")
        pg.wait_for_timeout(600)
        check('Shell script' in status(), 'sample.sh is highlighted as a shell script')
        colored = pg.evaluate("[...document.querySelectorAll('.ed:not([hidden]) .cm-line span')].filter(s => getComputedStyle(s).color !== getComputedStyle(document.querySelector('.ed:not([hidden]) .cm-line')).color).length")
        check(colored > 10, 'shell script has colored words (%d)' % colored)
        check(not pg.is_visible('#start'), 'the start card goes away once a file is open (it replaced the empty tab)')
        check(pg.evaluate(ht + '.tabs.length') == 1, 'the empty tab was reused')
        shot('02-shell')

        pg.evaluate(ht + ".openPath('/home/me/app.py')")
        pg.wait_for_timeout(500)
        check('Python' in status(), 'app.py is Python')
        shot('03-python')
        pg.click('.ed:not([hidden]) .cm-content')
        pg.keyboard.press('Control+End')
        pg.keyboard.type('# added\n')
        pg.wait_for_timeout(300)
        check(pg.is_visible('.tab.active.dirty .dot'), 'a changed file shows a dot on its tab')
        pg.keyboard.press('Control+s')
        pg.wait_for_timeout(300)
        saved = pg.evaluate("window.__mock.saved()['/home/me/app.py']")
        check(bool(saved) and saved['text'].endswith('# added\n') and saved['eol'] == 'lf' and saved['encoding'] == 'utf-8', 'Ctrl+S saves the text with its encoding and line endings')
        check(not pg.is_visible('.tab.active.dirty'), 'saved: the dot goes away')

        # Undo and redo.
        pg.keyboard.type('xyz')
        pg.keyboard.press('Control+z')
        pg.wait_for_timeout(200)
        check('xyz' not in pg.evaluate(ht + '.current().ed.text()'), 'Ctrl+Z undoes')
        pg.keyboard.press('Control+y')
        pg.wait_for_timeout(200)
        check('xyz' in pg.evaluate(ht + '.current().ed.text()'), 'Ctrl+Y redoes')
        pg.keyboard.press('Control+z')

        # Find and replace (match case, regex).
        pg.keyboard.press('Control+h')
        pg.fill('#findInput', 'size')
        pg.wait_for_timeout(300)
        check('found' in pg.inner_text('#findCount') or ' of ' in pg.inner_text('#findCount'), 'find counts matches: ' + pg.inner_text('#findCount'))
        shot('04-find-replace')
        pg.click('#optRegex')
        pg.fill('#findInput', r'siz(e)s?')
        pg.fill('#replaceInput', 'SIZE$1')
        pg.wait_for_timeout(200)
        pg.click('#replaceAll')
        pg.wait_for_timeout(300)
        txt = pg.evaluate(ht + '.current().ed.text()')
        check('SIZEe' in txt and 'sizes' not in txt, 'replace all with a regular expression and a group')
        pg.click('#optRegex')
        pg.click('#optCase')
        pg.fill('#findInput', 'OSERROR')
        pg.wait_for_timeout(200)
        check('Not found' in pg.inner_text('#findCount'), 'match case')
        pg.click('#optCase')
        pg.fill('#findInput', 'oserror')
        pg.wait_for_timeout(200)
        check('Not found' not in pg.inner_text('#findCount'), 'ignore case finds OSError')
        pg.keyboard.press('Escape')
        check(not pg.is_visible('#findbar'), 'Esc closes find')

        # Go to line.
        pg.keyboard.press('Control+g')
        pg.wait_for_timeout(200)
        shot('05-go-to-line')
        pg.fill('#dlgInput', '12')
        pg.keyboard.press('Enter')
        pg.wait_for_timeout(200)
        check('Ln 12,' in status(), 'go to line 12')

        # Read-only system file and Edit as administrator.
        pg.evaluate(ht + ".openPath('/etc/fstab')")
        pg.wait_for_timeout(400)
        check(pg.is_visible('#banner') and 'permission' in pg.inner_text('#bannerText'), 'a file you can\'t write says so')
        check(pg.is_visible('#stRO'), 'status shows Read-only')
        pg.click('.ed:not([hidden]) .cm-content')
        pg.keyboard.type('nope')
        check('nope' not in pg.evaluate(ht + '.current().ed.text()'), 'read-only text can\'t be typed into')
        shot('06-read-only')
        pg.click('#bannerButtons button:has-text("Edit as administrator")')
        pg.wait_for_timeout(200)
        shot('07-admin-question')
        pg.click('#dlgButtons button:has-text("Edit as administrator")')
        pg.wait_for_timeout(200)
        check(pg.is_visible('#stAdmin'), 'editing as administrator is shown')
        pg.click('.ed:not([hidden]) .cm-content')
        pg.keyboard.press('Control+End')
        pg.keyboard.type('# new line\n')
        pg.keyboard.press('Control+s')
        pg.wait_for_timeout(300)
        s = pg.evaluate("window.__mock.saved()['/etc/fstab']")
        check(bool(s) and s['admin'] is True, 'saving as administrator goes through the admin path')
        shot('08-admin')

        # Encodings and line endings.
        pg.evaluate(ht + ".openPath('/home/me/crlf.ini')")
        pg.wait_for_timeout(400)
        check('CRLF' in status() and 'INI' in status(), 'CRLF and INI shown: ' + status())
        pg.click('#stEol')
        pg.click('#menu button:has-text("LF (Linux")')
        pg.wait_for_timeout(300)
        check('LF' in status() and 'CRLF' not in status() and pg.is_visible('.tab.active.dirty'), 'line endings can be changed (and it counts as a change)')
        pg.keyboard.press('Control+s')
        pg.wait_for_timeout(300)
        check(pg.evaluate("window.__mock.saved()['/home/me/crlf.ini'].eol") == 'lf', 'saved with LF')
        pg.evaluate(ht + ".openPath('/home/me/latin1.txt')")
        pg.wait_for_timeout(300)
        check('Latin-1' in status(), 'Latin-1 shown')
        pg.click('#stEnc')
        shot('09-encoding-menu')
        pg.click('#menu button:has-text("Save as UTF-8")')
        pg.wait_for_timeout(200)
        check('UTF-8' in status(), 'encoding can be changed')

        # A big log: only part is shown.
        pg.evaluate(ht + ".openPath('/var/log/huge.log')")
        pg.wait_for_timeout(500)
        check('only its first' in pg.inner_text('#bannerText') and 'Log' in status(), 'a huge log opens in part, read-only, as a log')
        shot('10-big-log')

        # Menu with the keyboard.
        pg.keyboard.press('F10')
        pg.wait_for_timeout(200)
        check(pg.is_visible('#menu'), 'F10 opens the menu')
        shot('11-menu')
        pg.keyboard.press('ArrowDown')
        pg.keyboard.press('ArrowDown')
        focused = pg.evaluate('document.activeElement.textContent')
        check('Open recent' in focused, 'arrows move through the menu (%s)' % focused)
        pg.keyboard.press('ArrowRight')
        pg.wait_for_timeout(100)
        check('app.py' in pg.inner_text('#menu'), 'recent files submenu')
        pg.keyboard.press('Escape')
        pg.keyboard.press('Escape')
        check(not pg.is_visible('#menu'), 'Esc closes the menu')

        # Unsaved changes: closing a tab asks; closing the window asks.
        pg.evaluate(ht + ".openPath('/home/me/notes.md')")
        pg.wait_for_timeout(300)
        check('Markdown' in status(), 'notes.md is Markdown')
        pg.click('.ed:not([hidden]) .cm-content')
        pg.keyboard.type('more')
        pg.wait_for_timeout(300)
        pg.keyboard.press('Control+w')
        pg.wait_for_timeout(200)
        check(pg.is_visible('#shade') and 'Save the changes' in pg.inner_text('#dialog'), 'closing a changed tab asks')
        shot('12-unsaved')
        pg.keyboard.press('Escape')
        pg.wait_for_timeout(100)
        check(pg.evaluate(ht + '.current().name') == 'notes.md', 'Cancel keeps the tab')
        pg.evaluate("window.__mock.emit('close-requested')")
        pg.wait_for_timeout(200)
        check(pg.is_visible('#shade'), 'closing the window asks')
        pg.click('#dlgButtons button:has-text("Don\'t save")')
        pg.wait_for_timeout(300)
        check(pg.evaluate('window.__quit === true'), 'Don\'t save closes the window')

        # Themes and narrow windows.
        pg.evaluate("document.documentElement.classList.add('light')")
        pg.set_viewport_size({'width': 480, 'height': 760})
        pg.wait_for_timeout(200)
        shot('13-narrow')
        b.close()
    srv.shutdown()
    check(not errors, 'no script errors' + ('' if not errors else ': ' + ' | '.join(errors[:5])))
    if fails:
        print('\n%d failed' % len(fails))
        sys.exit(1)
    print('\nall passed')


if __name__ == '__main__':
    main()
