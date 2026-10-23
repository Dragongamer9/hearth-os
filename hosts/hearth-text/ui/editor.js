// The editor component: CodeMirror 6 (vendored in ui/vendor by vendor.sh, loaded as plain ES modules), set up
// the way Hearth Text uses it. app.js only talks to this file, never to CodeMirror directly.
import { EditorState, Compartment, EditorSelection } from './vendor/codemirror-state.js';
import {
  EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection, dropCursor,
  rectangularSelection, crosshairCursor, highlightSpecialChars,
} from './vendor/codemirror-view.js';
import { defaultKeymap, history, historyKeymap, indentWithTab, undo, redo, undoDepth, redoDepth, selectAll } from './vendor/codemirror-commands.js';
import {
  syntaxHighlighting, HighlightStyle, indentOnInput, bracketMatching, foldGutter, foldKeymap, StreamLanguage, indentUnit,
} from './vendor/codemirror-language.js';
import {
  search, SearchQuery, setSearchQuery, getSearchQuery, findNext, findPrevious, replaceNext, replaceAll,
  openSearchPanel, closeSearchPanel, highlightSelectionMatches, selectNextOccurrence,
} from './vendor/codemirror-search.js';
import { closeBrackets, closeBracketsKeymap } from './vendor/codemirror-autocomplete.js';
import { tags as t } from './vendor/lezer-highlight.js';

// ------------------------------------------------------------------ colors (CSS variables set by the theme)
const style = HighlightStyle.define([
  { tag: [t.keyword, t.controlKeyword, t.operatorKeyword, t.definitionKeyword, t.moduleKeyword, t.modifier, t.self], color: 'var(--syn-kw)' },
  { tag: [t.string, t.special(t.string), t.character, t.docString], color: 'var(--syn-str)' },
  { tag: [t.regexp, t.escape], color: 'var(--syn-esc)' },
  { tag: [t.number, t.bool, t.atom, t.null, t.unit, t.color], color: 'var(--syn-num)' },
  { tag: [t.comment, t.lineComment, t.blockComment, t.docComment], color: 'var(--syn-cm)', fontStyle: 'italic' },
  { tag: [t.function(t.variableName), t.function(t.propertyName), t.macroName], color: 'var(--syn-fn)' },
  { tag: [t.typeName, t.className, t.namespace, t.standard(t.typeName)], color: 'var(--syn-type)' },
  { tag: [t.propertyName, t.attributeName, t.labelName], color: 'var(--syn-prop)' },
  { tag: [t.tagName, t.angleBracket], color: 'var(--syn-tag)' },
  { tag: [t.heading, t.heading1, t.heading2, t.heading3], color: 'var(--syn-head)', fontWeight: '700' },
  { tag: t.strong, fontWeight: '700' },
  { tag: t.emphasis, fontStyle: 'italic' },
  { tag: t.strikethrough, textDecoration: 'line-through' },
  { tag: [t.link, t.url], color: 'var(--syn-link)', textDecoration: 'underline' },
  { tag: [t.meta, t.processingInstruction, t.documentMeta, t.annotation], color: 'var(--syn-meta)' },
  { tag: [t.invalid], color: 'var(--syn-err)' },
  { tag: [t.inserted], color: 'var(--syn-ins)' },
  { tag: [t.deleted], color: 'var(--syn-del)' },
  { tag: [t.changed], color: 'var(--syn-num)' },
  { tag: [t.definition(t.variableName), t.definition(t.propertyName)], color: 'var(--syn-def)' },
  { tag: [t.operator, t.punctuation, t.separator, t.bracket], color: 'var(--syn-op)' },
  { tag: [t.monospace], color: 'var(--syn-str)' },
]);

const look = EditorView.theme({
  '&': { height: '100%', color: 'var(--text)', backgroundColor: 'var(--edbg)', fontSize: 'var(--fs)' },
  '.cm-scroller': { fontFamily: 'var(--mono)', lineHeight: '1.5', overflow: 'auto' },
  '.cm-content': { caretColor: 'var(--accent)', padding: '8px 0 40vh' },
  '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--accent)', borderLeftWidth: '2px' },
  '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': { backgroundColor: 'var(--sel) !important' },
  '.cm-activeLine': { backgroundColor: 'var(--actline)' },
  '.cm-gutters': { backgroundColor: 'var(--edbg)', color: 'var(--muted)', border: 'none', borderRight: '1px solid var(--line)' },
  '.cm-activeLineGutter': { backgroundColor: 'var(--actline)', color: 'var(--text)' },
  '.cm-lineNumbers .cm-gutterElement': { padding: '0 10px 0 14px', minWidth: '3ch' },
  '.cm-foldGutter .cm-gutterElement': { padding: '0 4px', cursor: 'pointer' },
  '.cm-searchMatch': { backgroundColor: 'var(--match)', borderRadius: '2px' },
  '.cm-searchMatch.cm-searchMatch-selected': { backgroundColor: 'var(--match-now)', outline: '1px solid var(--accent)' },
  '.cm-selectionMatch': { backgroundColor: 'var(--selmatch)' },
  '&.cm-focused .cm-matchingBracket': { backgroundColor: 'var(--match)', outline: '1px solid var(--accent)' },
  '.cm-foldPlaceholder': { backgroundColor: 'var(--panel3)', border: 'none', color: 'var(--text)', padding: '0 6px' },
  '.cm-specialChar': { color: 'var(--syn-err)' },
  '.cm-panels': { display: 'none' },
});

// ------------------------------------------------------------------ languages
// Each: id, name, file extensions, whole file names (and patterns), and how to load it. Loaded when first needed.
const legacy = (file, name) => async () => StreamLanguage.define((await import('./vendor/legacy-modes/' + file + '.js'))[name]);
const lang = (file, fn, opts) => async () => (await import('./vendor/' + file + '.js'))[fn](opts);

// Logs: dates and times, ERROR / WARN / INFO / DEBUG, quoted text, [tags] and numbers.
const logMode = {
  name: 'log',
  token(stream) {
    if (stream.match(/^\d{4}-\d\d-\d\d[T ]\d\d:\d\d(:\d\d)?([.,]\d+)?(Z|[+-]\d\d:?\d\d)?/)) return 'meta';
    if (stream.match(/^\d\d:\d\d:\d\d([.,]\d+)?/)) return 'meta';
    if (stream.match(/^(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) [ \d]\d \d\d:\d\d:\d\d/)) return 'meta';
    if (stream.match(/^\b(FATAL|CRITICAL|CRIT|ERROR|ERR|EMERG|ALERT|FAIL(ED|URE)?|PANIC|error|fatal|failed|panicked)\b/)) return 'invalid';
    if (stream.match(/^\b(WARN(ING)?|warning|warn|NOTICE)\b/)) return 'keyword';
    if (stream.match(/^\b(INFO|info|OK|SUCCESS|DONE)\b/)) return 'atom';
    if (stream.match(/^\b(DEBUG|TRACE|VERBOSE|debug|trace)\b/)) return 'comment';
    if (stream.match(/^"(?:[^"\\]|\\.)*"/) || stream.match(/^'(?:[^'\\\n]|\\.)*'/)) return 'string';
    if (stream.match(/^\[[^\]\n]{1,60}\]/)) return 'typeName';
    if (stream.match(/^(https?|file):\/\/\S+/)) return 'link';
    if (stream.match(/^0x[0-9a-fA-F]+\b/) || stream.match(/^\b\d+(\.\d+)?\b/)) return 'number';
    stream.next();
    return null;
  },
};

export const LANGUAGES = [
  { id: 'text', name: 'Plain text', exts: ['txt', 'text'], load: null },
  { id: 'shell', name: 'Shell script', exts: ['sh', 'bash', 'zsh', 'ksh', 'ash', 'bats', 'command', 'install', 'ebuild'],
    files: ['.bashrc', '.bash_profile', '.bash_logout', '.profile', '.zshrc', '.zprofile', '.zshenv', '.xprofile', '.xinitrc', 'PKGBUILD', 'APKBUILD', 'configure', '.envrc'],
    shebang: /^#!.*\b(ba|z|k|da|a)?sh\b/, load: legacy('shell', 'shell') },
  { id: 'python', name: 'Python', exts: ['py', 'pyw', 'pyi', 'wsgi'], files: ['SConstruct', 'SConscript'], shebang: /^#!.*\bpython/, load: lang('codemirror-lang-python', 'python') },
  { id: 'javascript', name: 'JavaScript', exts: ['js', 'mjs', 'cjs'], shebang: /^#!.*\b(node|deno|bun)\b/, load: lang('codemirror-lang-javascript', 'javascript') },
  { id: 'jsx', name: 'JavaScript (JSX)', exts: ['jsx'], load: lang('codemirror-lang-javascript', 'javascript', { jsx: true }) },
  { id: 'typescript', name: 'TypeScript', exts: ['ts', 'mts', 'cts'], load: lang('codemirror-lang-javascript', 'javascript', { typescript: true }) },
  { id: 'tsx', name: 'TypeScript (TSX)', exts: ['tsx'], load: lang('codemirror-lang-javascript', 'javascript', { typescript: true, jsx: true }) },
  { id: 'json', name: 'JSON', exts: ['json', 'jsonc', 'json5', 'geojson', 'webmanifest', 'har', 'ipynb', 'code-workspace'], files: ['.babelrc', '.eslintrc', 'flake.lock', 'composer.lock'], load: lang('codemirror-lang-json', 'json') },
  { id: 'yaml', name: 'YAML', exts: ['yaml', 'yml'], files: ['.clang-format', '.clang-tidy'], load: lang('codemirror-lang-yaml', 'yaml') },
  { id: 'toml', name: 'TOML', exts: ['toml'], files: ['Cargo.lock', 'Pipfile', 'poetry.lock', 'uv.lock'], load: legacy('toml', 'toml') },
  { id: 'ini', name: 'INI / config', exts: ['ini', 'conf', 'cfg', 'cnf', 'desktop', 'service', 'socket', 'timer', 'target', 'mount', 'automount', 'path', 'slice', 'network', 'netdev', 'link', 'properties', 'directory', 'theme', 'inf', 'reg', 'url', 'flatpakref', 'flatpakrepo', 'container', 'kdl', 'editorconfig', 'gitconfig', 'npmrc', 'pacnew', 'repo', 'list'],
    files: ['.gitconfig', '.editorconfig', '.npmrc', 'mimeapps.list', 'user-dirs.dirs', 'pacman.conf', 'os-release', 'environment', '.gitmodules', '.env'], load: legacy('properties', 'properties') },
  { id: 'xml', name: 'XML', exts: ['xml', 'xsd', 'xsl', 'xslt', 'svg', 'plist', 'xaml', 'csproj', 'vcxproj', 'props', 'targets', 'resx', 'config', 'manifest', 'metainfo', 'appdata', 'rss', 'atom', 'qrc', 'ui', 'policy', 'kml', 'gpx', 'nuspec'], load: lang('codemirror-lang-xml', 'xml') },
  { id: 'html', name: 'HTML', exts: ['html', 'htm', 'xhtml', 'vue', 'svelte'], load: lang('codemirror-lang-html', 'html') },
  { id: 'css', name: 'CSS', exts: ['css', 'scss', 'less', 'qss'], load: lang('codemirror-lang-css', 'css') },
  { id: 'markdown', name: 'Markdown', exts: ['md', 'markdown', 'mdown', 'mkd', 'mdx'], files: ['README', 'CHANGELOG'], load: lang('codemirror-lang-markdown', 'markdown') },
  { id: 'cpp', name: 'C / C++', exts: ['c', 'h', 'cc', 'cpp', 'cxx', 'c++', 'hh', 'hpp', 'hxx', 'h++', 'ino', 'inl', 'ipp', 'm', 'mm', 'glsl', 'vert', 'frag', 'hlsl', 'cu'], load: lang('codemirror-lang-cpp', 'cpp') },
  { id: 'rust', name: 'Rust', exts: ['rs'], load: lang('codemirror-lang-rust', 'rust') },
  { id: 'csharp', name: 'C#', exts: ['cs', 'csx', 'cake'], load: legacy('clike', 'csharp') },
  { id: 'java', name: 'Java', exts: ['java'], load: legacy('clike', 'java') },
  { id: 'kotlin', name: 'Kotlin', exts: ['kt', 'kts'], load: legacy('clike', 'kotlin') },
  { id: 'go', name: 'Go', exts: ['go'], load: legacy('go', 'go') },
  { id: 'lua', name: 'Lua', exts: ['lua'], shebang: /^#!.*\blua/, load: legacy('lua', 'lua') },
  { id: 'perl', name: 'Perl', exts: ['pl', 'pm'], shebang: /^#!.*\bperl/, load: legacy('perl', 'perl') },
  { id: 'ruby', name: 'Ruby', exts: ['rb', 'gemspec', 'rake'], files: ['Gemfile', 'Rakefile', 'Vagrantfile'], shebang: /^#!.*\bruby/, load: legacy('ruby', 'ruby') },
  { id: 'powershell', name: 'PowerShell', exts: ['ps1', 'psm1', 'psd1'], shebang: /^#!.*\bpwsh/, load: legacy('powershell', 'powerShell') },
  { id: 'sql', name: 'SQL', exts: ['sql'], load: legacy('sql', 'standardSQL') },
  { id: 'cmake', name: 'CMake', exts: ['cmake'], files: ['CMakeLists.txt'], load: legacy('cmake', 'cmake') },
  { id: 'dockerfile', name: 'Dockerfile', exts: ['dockerfile'], files: ['Dockerfile', 'Containerfile'], load: legacy('dockerfile', 'dockerFile') },
  { id: 'nginx', name: 'nginx', exts: [], files: ['nginx.conf'], load: legacy('nginx', 'nginx') },
  { id: 'diff', name: 'Diff / patch', exts: ['diff', 'patch', 'rej'], load: legacy('diff', 'diff') },
  { id: 'log', name: 'Log', exts: ['log', 'out', 'err', 'journal'], files: ['dmesg', 'syslog', 'messages'], load: async () => StreamLanguage.define(logMode) },
];
const byId = Object.fromEntries(LANGUAGES.map(l => [l.id, l]));

/// The language for a file name (and its first line, for scripts without an extension).
export function detectLanguage(name, firstLine = '') {
  const base = (name || '').split(/[\\/]/).pop();
  const lower = base.toLowerCase();
  for (const l of LANGUAGES) if (l.files && l.files.some(f => f === base || f.toLowerCase() === lower)) return l.id;
  if (/^dockerfile|^containerfile/i.test(base)) return 'dockerfile';
  if (/\.log(\.\d+)?$/i.test(lower) || /\.log\.(old|bak)$/i.test(lower)) return 'log';
  // backup copies: notes.txt~, config.ini.bak, nginx.conf.orig
  const stem = lower.replace(/(~|\.bak|\.orig|\.old|\.save|\.dist|\.example|\.sample|\.in|\.pacnew|\.rpmnew|\.dpkg-dist)$/, '');
  const ext = stem.includes('.') ? stem.split('.').pop() : '';
  if (ext) for (const l of LANGUAGES) if (l.exts.includes(ext)) return l.id;
  if (firstLine.startsWith('#!')) for (const l of LANGUAGES) if (l.shebang && l.shebang.test(firstLine)) return l.id;
  if (/^<\?xml|^<svg/.test(firstLine)) return 'xml';
  if (/^<!doctype html|^<html/i.test(firstLine)) return 'html';
  if (/^\[[\w .:"-]+\]\s*$/.test(firstLine) && !ext) return 'ini';
  if (/^(diff --git|--- |Index: )/.test(firstLine)) return 'diff';
  return 'text';
}
export const languageName = id => (byId[id] || byId.text).name;

/// Whether the text indents with tabs or spaces, and how many.
export function detectIndent(text) {
  const lines = text.slice(0, 200000).split('\n', 2000);
  let tabs = 0, spaces = 0;
  const widths = {};
  let prev = 0;
  for (const ln of lines) {
    if (ln.startsWith('\t')) { tabs++; continue; }
    const m = /^( +)\S/.exec(ln);
    if (!m) continue;
    spaces++;
    const d = Math.abs(m[1].length - prev);
    prev = m[1].length;
    if (d > 0 && d <= 8) widths[d] = (widths[d] || 0) + 1;
  }
  if (tabs > spaces) return { tabs: true, width: 4 };
  const best = Object.entries(widths).sort((a, b) => b[1] - a[1])[0];
  return { tabs: false, width: best ? +best[0] : 4 };
}

// ------------------------------------------------------------------ one editor
/// An editor in `parent`. `o`: doc, lang, readOnly, wrap, lineNumbers, big (no colors or bracket matching:
/// a very big file stays quick), indent {tabs, width}, onChange(view), onCursor(view).
export function createEditor(parent, o) {
  const cLang = new Compartment(), cWrap = new Compartment(), cRO = new Compartment(), cNums = new Compartment(), cIndent = new Compartment();
  const indentExt = ind => [indentUnit.of(ind.tabs ? '\t' : ' '.repeat(ind.width)), EditorState.tabSize.of(ind.width)];
  const numbers = on => on ? [lineNumbers(), highlightActiveLineGutter(), o.big ? [] : foldGutter({ openText: '▾', closedText: '▸' })] : [];
  const extensions = [
    cNums.of(numbers(o.lineNumbers !== false)),
    highlightSpecialChars(),
    history(),
    drawSelection(),
    dropCursor(),
    EditorState.allowMultipleSelections.of(true),
    o.big ? [] : [indentOnInput(), bracketMatching(), closeBrackets(), highlightSelectionMatches()],
    syntaxHighlighting(style, { fallback: true }),
    rectangularSelection(),
    crosshairCursor(),
    highlightActiveLine(),
    search({ createPanel: () => { const dom = document.createElement('div'); dom.className = 'hidden-search-panel'; return { dom }; } }),
    keymap.of([{ key: 'Mod-d', run: selectNextOccurrence, preventDefault: true }, ...closeBracketsKeymap, ...defaultKeymap, ...historyKeymap, ...foldKeymap, indentWithTab]),
    cLang.of([]),
    cWrap.of(o.wrap ? EditorView.lineWrapping : []),
    cRO.of(EditorState.readOnly.of(!!o.readOnly)),
    cIndent.of(indentExt(o.indent || { tabs: false, width: 4 })),
    look,
    EditorView.updateListener.of(u => {
      if (u.docChanged && o.onChange) o.onChange(u.view);
      if ((u.selectionSet || u.docChanged) && o.onCursor) o.onCursor(u.view);
    }),
    EditorView.contentAttributes.of({ spellcheck: 'false', autocorrect: 'off', autocapitalize: 'off' }),
  ];
  const view = new EditorView({ parent, state: EditorState.create({ doc: o.doc || '', extensions }) });
  let langSeq = 0;
  const ed = {
    view,
    lang: 'text',
    async setLanguage(id) {
      const l = byId[id] || byId.text;
      ed.lang = l.id;
      const seq = ++langSeq;
      let ext = [];
      if (l.load && !o.big) {
        try { ext = await l.load(); } catch (e) { console.warn('language', id, e); ext = []; }
      }
      if (seq === langSeq) view.dispatch({ effects: cLang.reconfigure(ext) });
    },
    setWrap(on) { view.dispatch({ effects: cWrap.reconfigure(on ? EditorView.lineWrapping : []) }); },
    setLineNumbers(on) { view.dispatch({ effects: cNums.reconfigure(numbers(on)) }); },
    setReadOnly(on) { view.dispatch({ effects: cRO.reconfigure(EditorState.readOnly.of(!!on)) }); },
    setIndent(ind) { view.dispatch({ effects: cIndent.reconfigure(indentExt(ind)) }); },
    setText(text) { view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } }); },
    text() { return view.state.doc.toString(); },
    focus() { view.focus(); },
    destroy() { view.destroy(); },
  };
  ed.setLanguage(o.lang || 'text');
  return ed;
}

// ------------------------------------------------------------------ find and replace
/// The search in `view` (and its highlighting) set to `q` = {search, replace, caseSensitive, regexp, wholeWord}.
/// Returns whether the search is valid (a broken regular expression isn't).
export function setFind(view, q) {
  const query = new SearchQuery({ search: q.search || '', replace: q.replace || '', caseSensitive: !!q.caseSensitive, regexp: !!q.regexp, wholeWord: !!q.wholeWord });
  openSearchPanel(view);
  view.dispatch({ effects: setSearchQuery.of(query) });
  return query.valid || !q.search;
}
export function stopFind(view) { closeSearchPanel(view); }

/// How many matches there are, and which one is selected (counting stops at 9999).
export function countMatches(view) {
  const query = getSearchQuery(view.state);
  if (!query.search || !query.valid) return { total: 0, current: 0 };
  const sel = view.state.selection.main;
  const cur = query.getCursor(view.state);
  let total = 0, current = 0;
  for (let r = cur.next(); !r.done; r = cur.next()) {
    total++;
    if (r.value.from === sel.from && r.value.to === sel.to) current = total;
    if (total >= 9999) break;
  }
  return { total, current };
}
export const findNextMatch = view => findNext(view);
export const findPrevMatch = view => findPrevious(view);
export const replaceOne = view => replaceNext(view);
export const replaceEvery = view => replaceAll(view);

// ------------------------------------------------------------------ small commands
export function gotoLine(view, line, col = 1) {
  const doc = view.state.doc;
  const ln = doc.line(Math.max(1, Math.min(doc.lines, line | 0)));
  const pos = Math.min(ln.to, ln.from + Math.max(0, (col | 0) - 1));
  view.dispatch({ selection: EditorSelection.cursor(pos), effects: EditorView.scrollIntoView(pos, { y: 'center' }) });
  view.focus();
}
export function cursorInfo(view) {
  const s = view.state, r = s.selection.main, ln = s.doc.lineAt(r.head);
  const chosen = s.selection.ranges.reduce((n, x) => n + (x.to - x.from), 0);
  return { line: ln.number, col: r.head - ln.from + 1, lines: s.doc.lines, selected: chosen, cursors: s.selection.ranges.length };
}
export const doUndo = view => undo(view);
export const doRedo = view => redo(view);
export const doSelectAll = view => selectAll(view);
export const canUndo = view => undoDepth(view.state) > 0;
export const canRedo = view => redoDepth(view.state) > 0;
export { EditorView };
