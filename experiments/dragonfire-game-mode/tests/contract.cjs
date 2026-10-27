// Minimal dependency-free source contract: complements eventual real Gamescope tests.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const html = fs.readFileSync(path.join(__dirname, '..', 'index.html'), 'utf8');
for (const method of ['library.get', 'game.launch']) {
  assert.ok(html.includes(method), 'Missing bridge method ' + method);
}
assert.match(html, /preview\s*=\s*true/, 'Preview must be visibly distinguished');
assert.match(html, /preview\s*\|\|\s*!selected/, 'Preview must prohibit launching');
assert.match(html, /navigator\.getGamepads/, 'Controller interaction missing');
assert.match(html, /buttons\[12\]/, 'D-pad up missing');
assert.match(html, /buttons\[13\]/, 'D-pad down missing');
assert.match(html, /buttons\[1\]/, 'B / Back missing');
assert.match(html, /moveFocus\(step\)/, 'Gamepad focus navigation missing');
assert.match(html, /window\.nimbusHost/, 'Hearth host connection missing');
assert.match(html, /window\.chrome\?\.webview/, 'Windows host connection missing');
assert.match(html, /role="status"/, 'Accessible feedback missing');
console.log('Dragonfire Game Mode source contract OK');
