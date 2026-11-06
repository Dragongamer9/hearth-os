#!/usr/bin/env python3
"""A small, strict stand-in for Plymouth's script plugin, for Hearth's boot splash themes (no plymouthd, no root):

  - check(path)            parses a theme's .script and returns syntax errors (Plymouth's script language: fun,
                           if/else, while, for, return, break, continue, blocks, hashes, the usual operators)
  - Splash(theme_dir, w, h) runs a theme the way Plymouth does (the script once, then its refresh function 50 times
                           a second, the password, normal and message callbacks when asked) and draws frames with
                           Pillow: the background gradient and every sprite by its z order and opacity

It's stricter than Plymouth: reading a name that was never set, calling something that isn't a function, a missing
picture or arithmetic on nothing are errors here (Plymouth quietly uses NULL, which is how a splash ends up blank).
Settings › Boot & startup's splash previews are drawn with it (tools/make-boot-previews.py), and the tests run every
theme through it (hosts/linux/test/test_plymouth_themes.py).

  python3 tools/plymouth_preview.py THEME_DIR [OUT.png]      check and draw one frame (2 s in) of a theme
"""
import math
import os
import re
import sys

TOKEN = re.compile(r'''
    (?P<ws>[ \t\r\n]+)
  | (?P<comment>\#[^\n]*|//[^\n]*|/\*.*?\*/)
  | (?P<num>\d+\.\d*|\.\d+|\d+)
  | (?P<str>"(?:[^"\\\n]|\\.)*")
  | (?P<id>[A-Za-z_][A-Za-z0-9_]*)
  | (?P<op>==|!=|<=|>=|&&|\|\||\+\+|--|\+=|-=|\*=|/=|%=|[-+*/%<>=!(){}\[\];,.])
''', re.X | re.S)
KEYWORDS = {'if', 'else', 'while', 'do', 'for', 'fun', 'return', 'break', 'continue'}


class ScriptError(Exception):
    pass


def tokenize(src):
    out, pos, line = [], 0, 1
    while pos < len(src):
        m = TOKEN.match(src, pos)
        if not m:
            raise ScriptError(f'line {line}: unexpected character {src[pos]!r}')
        kind = m.lastgroup
        text = m.group()
        if kind == 'str':
            body = text[1:-1]
            text = re.sub(r'\\(.)', lambda e: {'n': '\n', 't': '\t'}.get(e.group(1), e.group(1)), body)
        if kind not in ('ws', 'comment'):
            out.append((kind, text, line))
        line += m.group().count('\n')
        pos = m.end()
    out.append(('eof', '', line))
    return out


# ------------------------------------------------------------------ parser (tuples: (kind, line, ...))
class Parser:
    def __init__(self, src):
        self.t = tokenize(src)
        self.i = 0

    def peek(self, k=0):
        return self.t[self.i + k]

    def take(self, text=None):
        tok = self.t[self.i]
        if text is not None and tok[1] != text or (text is not None and tok[0] == 'str'):
            raise ScriptError(f'line {tok[2]}: expected {text!r}, found {tok[1] or "the end"!r}')
        self.i += 1
        return tok

    def is_(self, text):
        tok = self.t[self.i]
        return tok[0] in ('op', 'id') and tok[1] == text

    def program(self):
        body = []
        while self.peek()[0] != 'eof':
            body.append(self.statement())
        return ('block', 1, body)

    def statement(self):
        tok = self.peek()
        ln = tok[2]
        if self.is_('{'):
            self.take('{')
            body = []
            while not self.is_('}'):
                if self.peek()[0] == 'eof':
                    raise ScriptError(f'line {ln}: a {{ is never closed')
                body.append(self.statement())
            self.take('}')
            return ('block', ln, body)
        if self.is_(';'):
            self.take(';')
            return ('block', ln, [])
        if tok[0] == 'id' and tok[1] == 'if':
            self.take(); self.take('('); c = self.expr(); self.take(')')
            a = self.statement()
            b = None
            if self.is_('else'):
                self.take(); b = self.statement()
            return ('if', ln, c, a, b)
        if tok[0] == 'id' and tok[1] == 'while':
            self.take(); self.take('('); c = self.expr(); self.take(')')
            return ('while', ln, c, self.statement())
        if tok[0] == 'id' and tok[1] == 'do':
            self.take(); s = self.statement(); self.take('while'); self.take('('); c = self.expr(); self.take(')'); self.take(';')
            return ('do', ln, s, c)
        if tok[0] == 'id' and tok[1] == 'for':
            self.take(); self.take('(')
            a = self.expr(); self.take(';'); c = self.expr(); self.take(';'); s = self.expr(); self.take(')')
            return ('for', ln, a, c, s, self.statement())
        if tok[0] == 'id' and tok[1] == 'fun':
            self.take()
            name = self.take()
            if name[0] != 'id' or name[1] in KEYWORDS:
                raise ScriptError(f'line {ln}: a function needs a name')
            params = self.params()
            if not self.is_('{'):
                raise ScriptError(f'line {ln}: a function\'s body goes in {{ }}')
            return ('fun', ln, name[1], params, self.statement())
        if tok[0] == 'id' and tok[1] == 'return':
            self.take()
            e = None if self.is_(';') else self.expr()
            self.take(';')
            return ('return', ln, e)
        if tok[0] == 'id' and tok[1] in ('break', 'continue'):
            self.take(); self.take(';')
            return (tok[1], ln)
        e = self.expr()
        self.take(';')
        return ('expr', ln, e)

    def params(self):
        self.take('(')
        out = []
        while not self.is_(')'):
            p = self.take()
            if p[0] != 'id' or p[1] in KEYWORDS:
                raise ScriptError(f'line {p[2]}: bad parameter {p[1]!r}')
            out.append(p[1])
            if not self.is_(')'):
                self.take(',')
        self.take(')')
        return out

    def expr(self):
        return self.assign()

    def assign(self):
        left = self.binary(0)
        if self.peek()[0] == 'op' and self.peek()[1] in ('=', '+=', '-=', '*=', '/=', '%='):
            op = self.take()
            if left[0] not in ('var', 'field', 'index'):
                raise ScriptError(f'line {op[2]}: can\'t assign to that')
            return ('assign', op[2], op[1], left, self.assign())
        return left

    LEVELS = [('||',), ('&&',), ('==', '!='), ('<', '>', '<=', '>='), ('+', '-'), ('*', '/', '%')]

    def binary(self, level):
        if level == len(self.LEVELS):
            return self.unary()
        left = self.binary(level + 1)
        while self.peek()[0] == 'op' and self.peek()[1] in self.LEVELS[level]:
            op = self.take()
            left = ('bin', op[2], op[1], left, self.binary(level + 1))
        return left

    def unary(self):
        tok = self.peek()
        if tok[0] == 'op' and tok[1] in ('!', '-', '+'):
            self.take()
            return ('unary', tok[2], tok[1], self.unary())
        if tok[0] == 'op' and tok[1] in ('++', '--'):
            self.take()
            target = self.unary()
            if target[0] not in ('var', 'field', 'index'):
                raise ScriptError(f'line {tok[2]}: {tok[1]} needs a variable')
            return ('pre', tok[2], tok[1], target)
        return self.postfix()

    def postfix(self):
        e = self.primary()
        while True:
            tok = self.peek()
            if tok[0] != 'op':
                return e
            if tok[1] == '.':
                self.take()
                name = self.take()
                if name[0] != 'id':
                    raise ScriptError(f'line {tok[2]}: a name goes after "."')
                e = ('field', tok[2], e, name[1])
            elif tok[1] == '[':
                self.take(); idx = self.expr(); self.take(']')
                e = ('index', tok[2], e, idx)
            elif tok[1] == '(':
                self.take()
                args = []
                while not self.is_(')'):
                    args.append(self.expr())
                    if not self.is_(')'):
                        self.take(',')
                self.take(')')
                e = ('call', tok[2], e, args)
            elif tok[1] in ('++', '--'):
                if e[0] not in ('var', 'field', 'index'):
                    raise ScriptError(f'line {tok[2]}: {tok[1]} needs a variable')
                self.take()
                e = ('post', tok[2], tok[1], e)
            else:
                return e

    def primary(self):
        tok = self.take()
        kind, text, ln = tok
        if kind == 'num':
            return ('num', ln, float(text))
        if kind == 'str':
            return ('str', ln, text)
        if kind == 'id':
            if text in KEYWORDS:
                if text == 'fun':
                    params = self.params()
                    return ('lambda', ln, params, self.statement())
                raise ScriptError(f'line {ln}: unexpected {text!r}')
            return ('var', ln, text)
        if kind == 'op' and text == '(':
            e = self.expr()
            self.take(')')
            return e
        raise ScriptError(f'line {ln}: unexpected {text or "end of file"!r}')


def parse(src):
    return Parser(src).program()


def check(path):
    """Syntax errors in a .script file (empty list: it parses)."""
    try:
        with open(path, encoding='utf-8') as f:
            parse(f.read())
    except (ScriptError, UnicodeDecodeError, OSError) as e:
        return [f'{os.path.basename(path)}: {e}']
    return []


# ------------------------------------------------------------------ interpreter
class Hash(dict):
    pass


class Func:
    def __init__(self, name, params, body):
        self.name, self.params, self.body = name, params, body


class Native:
    """A built-in: callable (fn), with members (Image.Text, Math.Sin ...)."""
    def __init__(self, name, fn=None, members=None):
        self.name, self.fn, self.members = name, fn, members or {}


class Img:
    def __init__(self, pil):
        self.pil = pil


class Spr:
    def __init__(self, splash, img=None):
        self.img, self.x, self.y, self.z, self.opacity = img, 0.0, 0.0, 0.0, 1.0
        self.order = len(splash.sprites)
        splash.sprites.append(self)


class _Return(Exception):
    def __init__(self, v):
        self.v = v


class _Break(Exception):
    pass


class _Continue(Exception):
    pass


def _key(v):
    if isinstance(v, float):
        return str(int(v)) if v == int(v) else repr(v)
    return str(v)


def _font(size):
    from PIL import ImageFont
    for p in ('/usr/share/fonts/opentype/inter/Inter-Regular.otf', '/usr/share/fonts/truetype/inter/Inter-Regular.ttf',
              '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'):
        if os.path.isfile(p):
            return ImageFont.truetype(p, size)
    return ImageFont.load_default()


class Splash:
    REFRESH_HZ = 50

    def __init__(self, theme_dir, width=1280, height=800, strict=True):
        name = os.path.basename(os.path.normpath(theme_dir))
        self.dir = theme_dir
        self.w, self.h = width, height
        self.strict = strict
        self.errors = []
        self.used = set()           # the built-ins the script used ("Math.Sin", "Sprite.SetOpacity" ...)
        self.sprites = []
        self.top = self.bottom = (0.0, 0.0, 0.0)
        self.callbacks = {}
        self.steps = 0
        self.g = Hash()
        self._install()
        with open(os.path.join(theme_dir, name + '.script'), encoding='utf-8') as f:
            tree = parse(f.read())
        self._run_top(tree)

    # ---- errors
    def err(self, ln, msg):
        text = f'line {ln}: {msg}'
        if text not in self.errors:
            self.errors.append(text)
        if self.strict:
            raise ScriptError(text)

    def _run_top(self, tree):
        try:
            self.exec(tree, self.g)
        except ScriptError:
            pass
        except (_Return, _Break, _Continue):
            pass

    # ---- built-ins
    def _install(self):
        from PIL import Image as PImage
        S = self

        def use(n, f):
            def w(ln, args):
                S.used.add(n)
                return f(ln, args)
            return w

        def num(ln, args, i, what):
            v = args[i] if i < len(args) else None
            if not isinstance(v, float):
                S.err(ln, f'{what} needs a number (got {v!r})')
                return 0.0
            return v

        def image_load(ln, args):
            if not args or not isinstance(args[0], str):
                S.err(ln, 'Image() needs a file name'); return None
            p = os.path.join(S.dir, args[0])
            if not os.path.isfile(p):
                S.err(ln, f'Image("{args[0]}"): no such picture in the theme'); return None
            return Img(PImage.open(p).convert('RGBA'))

        def image_text(ln, args):
            text = args[0] if args else ''
            if not isinstance(text, str):
                text = _key(text) if text is not None else ''
            rgba = [num(ln, args, i, 'Image.Text colour') if i < len(args) else 1.0 for i in range(1, 5)]
            size = 16
            if len(args) > 5 and isinstance(args[5], str):
                m = re.search(r'(\d+)\s*$', args[5])
                size = int(m.group(1)) if m else 16
            f = _font(int(size * 1.33))
            from PIL import ImageDraw
            box = f.getbbox(text or ' ')
            im = PImage.new('RGBA', (max(1, box[2] + 2), max(1, int(size * 1.33 * 1.25))), (0, 0, 0, 0))
            ImageDraw.Draw(im).text((0, 0), text, font=f, fill=tuple(int(max(0, min(1, c)) * 255) for c in rgba))
            return Img(im)

        self.g['Image'] = Native('Image', use('Image', image_load), {'Text': Native('Image.Text', use('Image.Text', image_text))})

        def sprite_new(ln, args):
            img = args[0] if args else None
            if args and not isinstance(img, Img):
                S.err(ln, 'Sprite() needs a picture (the picture is missing)')
                img = None
            return Spr(S, img)

        self.g['Sprite'] = Native('Sprite', use('Sprite', sprite_new))

        def window_color(which):
            def f(ln, args):
                c = tuple(num(ln, args, i, 'Window colour') for i in range(3))
                setattr(S, which, c)
            return f
        self.g['Window'] = Native('Window', None, {
            'GetWidth': Native('Window.GetWidth', use('Window.GetWidth', lambda ln, a: float(S.w))),
            'GetHeight': Native('Window.GetHeight', use('Window.GetHeight', lambda ln, a: float(S.h))),
            'GetX': Native('Window.GetX', use('Window.GetX', lambda ln, a: 0.0)),
            'GetY': Native('Window.GetY', use('Window.GetY', lambda ln, a: 0.0)),
            'SetBackgroundTopColor': Native('w', use('Window.SetBackgroundTopColor', window_color('top'))),
            'SetBackgroundBottomColor': Native('w', use('Window.SetBackgroundBottomColor', window_color('bottom'))),
        })

        def m1(n, f):
            return Native('Math.' + n, use('Math.' + n, lambda ln, a: float(f(num(ln, a, 0, 'Math.' + n)))))

        def m2(n, f):
            return Native('Math.' + n, use('Math.' + n, lambda ln, a: float(f(num(ln, a, 0, 'Math.' + n), num(ln, a, 1, 'Math.' + n)))))
        self.g['Math'] = Native('Math', None, {
            'Int': m1('Int', math.floor), 'Sin': m1('Sin', math.sin), 'Cos': m1('Cos', math.cos), 'Abs': m1('Abs', abs),
            'Sqrt': m1('Sqrt', lambda v: math.sqrt(max(0, v))), 'Min': m2('Min', min), 'Max': m2('Max', max),
            'Pi': 3.14159265358979,
        })

        def setcb(n):
            def f(ln, args):
                if not args or not isinstance(args[0], Func):
                    S.err(ln, f'Plymouth.Set{n}Function needs a function')
                    return None
                S.callbacks[n] = args[0]
            return Native('Plymouth.Set' + n + 'Function', use('Plymouth.Set' + n + 'Function', f))
        self.g['Plymouth'] = Native('Plymouth', None, {
            'Set' + n + 'Function': setcb(n) for n in ('Refresh', 'DisplayPassword', 'DisplayNormal', 'Message',
                                                       'BootProgress', 'Quit', 'DisplayQuestion', 'RootMounted',
                                                       'KeyboardInput', 'UpdateStatus', 'DisplayMessage', 'HideMessage')})
        self.g['Plymouth'].members['GetMode'] = Native('Plymouth.GetMode', use('Plymouth.GetMode', lambda ln, a: 'boot'))

    def member(self, ln, obj, name):
        if isinstance(obj, Native):
            if name in obj.members:
                return obj.members[name]
            self.err(ln, f'{obj.name} has no {name}')
            return None
        if isinstance(obj, Img):
            S = self

            def scale(ln, a):
                w, h = int(self._num(ln, a, 0)), int(self._num(ln, a, 1))
                if w < 1 or h < 1:
                    S.err(ln, f'Image.Scale to {w}x{h}')
                    return None
                return Img(obj.pil.resize((w, h), 2))
            methods = {'Scale': scale, 'GetWidth': lambda ln, a: float(obj.pil.width),
                       'GetHeight': lambda ln, a: float(obj.pil.height)}
            if name not in methods:
                self.err(ln, f'pictures have no {name} (here)')
                return None
            return Native('Image.' + name, (lambda f, n: lambda ln, a: (S.used.add('Image.' + n), f(ln, a))[1])(methods[name], name))
        if isinstance(obj, Spr):
            S = self

            def pos(ln, a):
                obj.x, obj.y = self._num(ln, a, 0), self._num(ln, a, 1)
                if len(a) > 2:
                    obj.z = self._num(ln, a, 2)

            def setimg(ln, a):
                if not isinstance(a[0] if a else None, Img):
                    S.err(ln, 'SetImage needs a picture')
                    return
                obj.img = a[0]

            def setop(ln, a):
                obj.opacity = self._num(ln, a, 0)
            methods = {'SetPosition': pos, 'SetImage': setimg, 'SetOpacity': setop,
                       'SetX': lambda ln, a: setattr(obj, 'x', self._num(ln, a, 0)),
                       'SetY': lambda ln, a: setattr(obj, 'y', self._num(ln, a, 0)),
                       'SetZ': lambda ln, a: setattr(obj, 'z', self._num(ln, a, 0)),
                       'GetImage': lambda ln, a: obj.img}
            if name not in methods:
                self.err(ln, f'sprites have no {name} (here)')
                return None
            return Native('Sprite.' + name, (lambda f, n: lambda ln, a: (S.used.add('Sprite.' + n), f(ln, a))[1])(methods[name], name))
        if isinstance(obj, Hash):
            if name in obj:
                return obj[name]
            self.err(ln, f'.{name} was never set')
            return None
        self.err(ln, f'.{name} of {type(obj).__name__ if obj is not None else "nothing"}')
        return None

    def _num(self, ln, a, i):
        v = a[i] if i < len(a) else None
        if not isinstance(v, float):
            self.err(ln, f'needs a number (got {v!r})')
            return 0.0
        return v

    # ---- statements
    def exec(self, node, local):
        k = node[0]
        if k == 'block':
            for s in node[2]:
                self.exec(s, local)
        elif k == 'expr':
            self.eval(node[2], local)
        elif k == 'if':
            if self.truth(self.eval(node[2], local)):
                self.exec(node[3], local)
            elif node[4] is not None:
                self.exec(node[4], local)
        elif k == 'while':
            n = 0
            while self.truth(self.eval(node[2], local)):
                n += 1
                if n > 100000:
                    self.err(node[1], 'a loop that never ends')
                    return
                try:
                    self.exec(node[3], local)
                except _Break:
                    break
                except _Continue:
                    continue
        elif k == 'do':
            n = 0
            while True:
                n += 1
                if n > 100000:
                    self.err(node[1], 'a loop that never ends')
                    return
                try:
                    self.exec(node[2], local)
                except _Break:
                    break
                except _Continue:
                    pass
                if not self.truth(self.eval(node[3], local)):
                    break
        elif k == 'for':
            self.eval(node[2], local)
            n = 0
            while self.truth(self.eval(node[3], local)):
                n += 1
                if n > 100000:
                    self.err(node[1], 'a loop that never ends')
                    return
                try:
                    self.exec(node[5], local)
                except _Break:
                    break
                except _Continue:
                    pass
                self.eval(node[4], local)
        elif k == 'fun':
            self.g[node[2]] = Func(node[2], node[3], node[4])
        elif k == 'return':
            raise _Return(self.eval(node[2], local) if node[2] is not None else None)
        elif k == 'break':
            raise _Break()
        elif k == 'continue':
            raise _Continue()

    @staticmethod
    def truth(v):
        if v is None:
            return False
        if isinstance(v, float):
            return v != 0
        if isinstance(v, str):
            return v != ''
        return True

    # ---- references (for assignment): (hash, key)
    def ref(self, node, local):
        k = node[0]
        if k == 'var':
            name = node[2]
            if name == 'global':
                self.err(node[1], 'can\'t assign to global itself')
            if name in local:
                return local, name
            if name in self.g:
                return self.g, name
            return local, name
        base = node[2]
        if base[0] == 'var' and base[2] in ('global', 'local'):
            container = self.g if base[2] == 'global' else local
        else:
            h, key = self.ref(base, local)
            container = h.get(key)
            if container is None:
                container = Hash()
                h[key] = container
            if not isinstance(container, Hash):
                self.err(node[1], 'can\'t set a field on that')
                return Hash(), '_'
        key = node[3] if k == 'field' else _key(self.eval(node[3], local))
        return container, key

    def eval(self, node, local):
        k = node[0]
        ln = node[1]
        if k == 'num':
            return node[2]
        if k == 'str':
            return node[2]
        if k == 'var':
            name = node[2]
            if name == 'global':
                return self.g
            if name == 'local':
                return local
            if name in local:
                return local[name]
            if name in self.g:
                return self.g[name]
            self.err(ln, f'{name} was never set')
            return None
        if k == 'field':
            return self.member(ln, self.eval(node[2], local), node[3])
        if k == 'index':
            base = self.eval(node[2], local)
            key = _key(self.eval(node[3], local))
            if isinstance(base, Hash) and key in base:
                return base[key]
            self.err(ln, f'[{key}] was never set')
            return None
        if k == 'lambda':
            return Func('<fun>', node[2], node[3])
        if k == 'call':
            fn = self.eval(node[2], local)
            args = [self.eval(a, local) for a in node[3]]
            return self.call(ln, fn, args)
        if k == 'assign':
            h, key = self.ref(node[3], local)
            v = self.eval(node[4], local)
            if node[2] != '=':
                v = self.arith(ln, node[2][0], h.get(key), v)
            h[key] = v
            return v
        if k in ('pre', 'post'):
            h, key = self.ref(node[3], local)
            old = h.get(key)
            new = self.arith(ln, '+' if node[2] == '++' else '-', old, 1.0)
            h[key] = new
            return new if k == 'pre' else old
        if k == 'unary':
            v = self.eval(node[3], local)
            if node[2] == '!':
                return 0.0 if self.truth(v) else 1.0
            if not isinstance(v, float):
                self.err(ln, f'{node[2]} of {v!r}')
                return 0.0
            return -v if node[2] == '-' else v
        if k == 'bin':
            op = node[2]
            if op == '&&':
                return 1.0 if self.truth(self.eval(node[3], local)) and self.truth(self.eval(node[4], local)) else 0.0
            if op == '||':
                return 1.0 if self.truth(self.eval(node[3], local)) or self.truth(self.eval(node[4], local)) else 0.0
            a, b = self.eval(node[3], local), self.eval(node[4], local)
            if op in ('==', '!='):
                eq = a == b
                return 1.0 if eq == (op == '==') else 0.0
            if op in ('<', '>', '<=', '>='):
                if not (isinstance(a, float) and isinstance(b, float)):
                    self.err(ln, f'{a!r} {op} {b!r}')
                    return 0.0
                return 1.0 if {'<': a < b, '>': a > b, '<=': a <= b, '>=': a >= b}[op] else 0.0
            return self.arith(ln, op, a, b)
        self.err(ln, f'unknown {k}')
        return None

    def arith(self, ln, op, a, b):
        if op == '+' and (isinstance(a, str) or isinstance(b, str)) and a is not None and b is not None:
            return (a if isinstance(a, str) else _key(a)) + (b if isinstance(b, str) else _key(b))
        if not (isinstance(a, float) and isinstance(b, float)):
            self.err(ln, f'{a!r} {op} {b!r}')
            return 0.0
        if op == '+':
            return a + b
        if op == '-':
            return a - b
        if op == '*':
            return a * b
        if op == '/':
            if b == 0:
                self.err(ln, 'division by zero')
                return 0.0
            return a / b
        if op == '%':
            if b == 0:
                self.err(ln, 'division by zero')
                return 0.0
            return math.fmod(a, b)
        self.err(ln, f'unknown operator {op}')
        return 0.0

    def call(self, ln, fn, args):
        if isinstance(fn, Native):
            if fn.fn is None:
                self.err(ln, f'{fn.name} isn\'t a function')
                return None
            return fn.fn(ln, args)
        if isinstance(fn, Func):
            local = Hash()
            for i, p in enumerate(fn.params):
                local[p] = args[i] if i < len(args) else None
            try:
                self.exec(fn.body, local)
            except _Return as r:
                return r.v
            return None
        self.err(ln, f'calling {fn!r}, which isn\'t a function')
        return None

    # ---- what Plymouth does after the script ran
    def callback(self, name, *args):
        f = self.callbacks.get(name)
        if f is None:
            return False
        try:
            self.call(0, f, list(args))
        except ScriptError:
            pass
        except (_Break, _Continue):
            self.errors.append(f'{name}: break or continue outside a loop')
        return True

    def advance(self, seconds):
        for _ in range(int(round(seconds * self.REFRESH_HZ))):
            self.steps += 1
            self.callback('Refresh')

    def password(self, prompt='', bullets=0):
        return self.callback('DisplayPassword', prompt, float(bullets))

    def normal(self):
        return self.callback('DisplayNormal')

    def message(self, text):
        return self.callback('Message', text)

    def draw(self):
        import numpy as np
        from PIL import Image as PImage
        t = np.linspace(0, 1, self.h)[:, None, None]
        top, bottom = np.array(self.top) * 255, np.array(self.bottom) * 255
        bg = top * (1 - t) + bottom * t
        canvas = PImage.fromarray(np.repeat(bg, self.w, axis=1).clip(0, 255).astype('uint8'), 'RGB').convert('RGBA')
        for s in sorted(self.sprites, key=lambda s: (s.z, s.order)):
            if s.img is None or s.opacity <= 0.002:
                continue
            im = s.img.pil
            if s.opacity < 1:
                im = im.copy()
                a = min(1.0, s.opacity)
                im.putalpha(im.split()[3].point(lambda v: int(v * a)))
            x, y = int(math.floor(s.x)), int(math.floor(s.y))
            if x >= self.w or y >= self.h or x + im.width <= 0 or y + im.height <= 0:
                continue
            layer = PImage.new('RGBA', canvas.size, (0, 0, 0, 0))
            layer.paste(im, (x, y))
            canvas = PImage.alpha_composite(canvas, layer)
        return canvas.convert('RGB')


if __name__ == '__main__':
    d = sys.argv[1]
    name = os.path.basename(os.path.normpath(d))
    errs = check(os.path.join(d, name + '.script'))
    if errs:
        print('\n'.join(errs)); sys.exit(1)
    s = Splash(d)
    s.advance(2.0)
    s.password('', 3); s.normal(); s.message('Checking the disk')
    print('errors:', s.errors or 'none')
    print('uses:', ', '.join(sorted(s.used)))
    if len(sys.argv) > 2:
        s.draw().save(sys.argv[2])
    sys.exit(1 if s.errors else 0)
