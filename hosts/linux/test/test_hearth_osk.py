"""Hearth's on-screen keyboard (ui/osk, docs/OSK.md): every copy is the same as ui/osk (tools/osk-sync.py), the
engine stays plain ES2017 (Qt's QML engine runs it in Desktop Mode), the word lists read the way Keyboard.qml reads
them, and Desktop Mode's pieces are wired up. Run: python3 -m unittest discover -s test -p 'test_*.py'"""
import os
import re
import subprocess
import sys
import unittest

sys.dont_write_bytecode = True
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', '..'))
OSK = os.path.join(ROOT, 'ui', 'osk')
SHELL = os.path.join(ROOT, 'hosts', 'linux', 'desktop', 'shell')


def read(*p):
    with open(os.path.join(*p), encoding='utf-8') as f:
        return f.read()


def code_only(js):
    """The JavaScript without comments, strings and regular expressions (roughly), to look for syntax."""
    out, i, n = [], 0, len(js)
    while i < n:
        c = js[i]
        if js.startswith('//', i):
            j = js.find('\n', i)
            i = n if j < 0 else j
        elif js.startswith('/*', i):
            j = js.find('*/', i + 2)
            i = n if j < 0 else j + 2
        elif c in '\'"`':
            j = i + 1
            while j < n and js[j] != c:
                j += 2 if js[j] == '\\' else 1
            out.append('""')
            i = j + 1
        elif c == '/' and re.search(r'[(,=:!&|?{};\n]\s*$', ''.join(out[-3:]) or '\n'):
            j = i + 1
            while j < n and js[j] != '/' and js[j] != '\n':
                j += 2 if js[j] == '\\' else 1
            out.append('/re/')
            i = j + 1
        else:
            out.append(c)
            i += 1
    return ''.join(out)


class Copies(unittest.TestCase):
    def test_every_copy_is_ui_osk(self):
        r = subprocess.run([sys.executable, os.path.join(ROOT, 'tools', 'osk-sync.py'), '--check'], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_pages_load_it(self):
        self.assertIn('osk/hearth-osk.js', read(ROOT, 'ui', 'index.html'))
        self.assertIn('osk/hearth-osk.css', read(ROOT, 'ui', 'index.html'))
        for app in ('hearth-maps/ui/index.html', 'hearth-contacts/ui/index.html', 'hearth-time/ui/notes.html', 'hearth-time/ui/calendar.html', 'hearth-disks/ui/index.html'):
            self.assertIn('osk/hearth-osk.js', read(ROOT, 'hosts', app), app)


class PlainJavaScript(unittest.TestCase):
    """Qt's JavaScript engine (Desktop Mode) runs the whole file: nothing newer than it knows."""

    def test_no_newer_syntax(self):
        js = code_only(read(OSK, 'hearth-osk.js'))
        self.assertNotRegex(js, r'\?\.(?![0-9])', 'optional chaining (?.)')
        self.assertNotIn('??', js, 'nullish coalescing (??)')
        self.assertNotIn('class ', js, 'classes (and their fields): plain functions only')
        self.assertNotIn('import ', js)
        self.assertNotIn('export ', js)

    def test_one_global(self):
        js = read(OSK, 'hearth-osk.js')
        self.assertTrue(js.lstrip().startswith('/*'))
        self.assertIn('var HearthOSK = (function () {', js)


class Words(unittest.TestCase):
    def test_word_lists(self):
        files = sorted(f for f in os.listdir(os.path.join(OSK, 'words')) if f.endswith('.js'))
        langs = re.findall(r"lang: '([a-z]+)'", read(OSK, 'hearth-osk.js'))
        self.assertGreaterEqual(len(files), 10)
        for lang in set(langs):
            self.assertIn(lang + '.js', files, 'no word list for ' + lang)
        for f in files:
            text = read(OSK, 'words', f)
            # The way Keyboard.qml reads it (a FileView, the quoted part).
            m = re.search(r'HearthOSK\.words\("([a-z]+)",\s*"([^"]*)"\)', text)
            self.assertIsNotNone(m, f)
            self.assertEqual(m.group(1) + '.js', f)
            words = m.group(2).split(' ')
            self.assertGreater(len(words), 3000, f)
            self.assertFalse([w for w in words if not w or '\\' in w], f)
            self.assertIn('FrequencyWords', text)   # the source and its licence are named

    def test_no_swear_words_suggested(self):
        en = read(OSK, 'words', 'en.js')
        for w in ('fuck', 'shit', 'bitch'):
            self.assertNotRegex(en, r'[ "]%s[ "]' % w)


class DesktopMode(unittest.TestCase):
    def test_keyboard_runs_the_shared_engine(self):
        kb = read(SHELL, 'Keyboard.qml')
        self.assertIn('import "osk/hearth-osk.js" as Osk', kb)
        self.assertIn('OskBoard', kb)
        self.assertIn('"wtype"', kb)
        self.assertIn('host: "mantel"', kb)
        self.assertIn('osk.json', kb)
        self.assertIn('osk-words.json', kb)
        pad = read(SHELL, 'Pad.qml')
        for t in ('"oskpad"', '"oskaxes"'):
            self.assertIn(t, pad)
        self.assertIn('oskWinOpen', read(SHELL, 'picker.qml'))
        self.assertIn('OskBoard', read(SHELL, 'FileManager.qml'))

    def test_board_draws_every_part(self):
        b = read(SHELL, 'OskBoard.qml')
        for part in ('suggestions', 'popup', 'wheel', 'split', 'ember', 'sparks'):
            self.assertIn(part, b)

    def test_browser_uses_it(self):
        mjs = read(ROOT, 'os', 'system_files', 'usr', 'share', 'hearth-browser', 'firefox', 'hearth', 'hearth.sys.mjs')
        self.assertIn('resource://hearth/osk/hearth-osk.js', mjs)
        self.assertIn('createEngine', mjs)



class GameModeController(unittest.TestCase):
    """Game Mode's controller loop (ui/index.html pollPadsNow) feeds the keyboard; tools/ui-check.js drives it with a
    pretend controller. These catch the 0.6.5 break without a browser."""

    def body(self):
        html = read(ROOT, 'ui', 'index.html')
        i = html.index('function pollPadsNow(')
        return html[i:html.index('\nfunction ', i + 10)]

    def test_no_name_declared_twice(self):
        # A const declared in the loop with the name of one outside it (const ax = ... after ax[n] was used) throws
        # "Cannot access 'ax' before initialization" on every frame the keyboard is up.
        code = code_only(self.body())
        # Each declaration with the blocks it is in (the open braces' positions): the same name in a block inside
        # another one that declares it = shadowing (sibling blocks are fine).
        stacks, st = {}, []
        for pos, c in enumerate(code):
            if c == '{':
                st.append(pos)
            elif c == '}' and st:
                st.pop()
            stacks[pos] = tuple(st)
        flat = []
        for m in re.finditer(r'\b(?:const|let|var)\s+', code):
            at = stacks[m.start()]
            k, depth, part = m.end(), 0, ''
            while k < len(code):
                c = code[k]
                if c in '([{':
                    depth += 1
                elif c in ')]}':
                    if depth == 0:
                        break
                    depth -= 1
                elif depth == 0 and c in ';\n':
                    break
                part += c if depth == 0 else ''
                k += 1
            for piece in part.split(','):
                n = piece.split('=')[0].strip()
                if re.fullmatch(r'[A-Za-z_$][\w$]*', n):
                    flat.append((n, at))
        twice = sorted(set(n for n, d in flat if any(n == n2 and len(d2) > len(d) and d2[:len(d)] == d for n2, d2 in flat)))
        self.assertEqual(twice, [], 'a name declared again in an inner block of pollPadsNow (shadowing)')

    def test_logs_which_way_presses_come(self):
        html = read(ROOT, 'ui', 'index.html')
        self.assertIn("oskLogPath('gamepad'", html)
        self.assertIn("oskLogPath('host'", html)
        self.assertIn('keyboard has the selection', html)

    def test_hearth_input_quits_on_signals(self):
        src = read(SHELL, 'hearth-input')
        self.assertNotIn('lambda *_: sys.exit(0)', src)
        self.assertIn('os._exit(0)', src)


if __name__ == '__main__':
    unittest.main()
