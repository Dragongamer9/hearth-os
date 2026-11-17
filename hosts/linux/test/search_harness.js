// Loads Hearth's QML JavaScript libraries (".pragma library" with ".import "x.js" as X") in node, the way QML does:
// each file in its own context, its imports as names in it. For the search plugins' tests (test_search_plugins.py).
//   node search_harness.js <runners.js> <job.json>   job: {calls: [[module, fn, ...args]]} → [results]
const fs = require('fs');
const path = require('path');
const vm = require('vm');
const cache = {};
function load(file) {
  file = path.resolve(file);
  if (cache[file]) return cache[file];
  const ctx = vm.createContext({ console });
  cache[file] = ctx;
  let src = fs.readFileSync(file, 'utf8').replace(/^\.pragma library\s*$/m, '');
  src = src.replace(/^\.import\s+"([^"]+)"\s+as\s+(\w+)\s*$/mg, (m, f, name) => { ctx[name] = load(path.join(path.dirname(file), f)); return ''; });
  vm.runInContext(src, ctx, { filename: file });
  return ctx;
}
const dir = path.dirname(process.argv[2]);
const job = JSON.parse(fs.readFileSync(process.argv[3], 'utf8'));
const out = job.calls.map(c => { const m = load(path.join(dir, c[0])); return JSON.parse(JSON.stringify(m[c[1]].apply(null, c.slice(2)) ?? null)); });
console.log(JSON.stringify(out));
