'use strict';
// node --test hosts/hearth-theater/electron (apps-create.yml)
const test = require('node:test');
const assert = require('node:assert');
const r = require('./rules');

test('the user agent is plain Chromium of the version drawing', () => {
  assert.strictEqual(r.userAgent('140.0.7339.41'), 'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36');
  assert.ok(!/Electron|Hearth/i.test(r.userAgent('140')));
  assert.match(r.userAgent(undefined), /Chrome\/140\.0\.0\.0/);
});

test('bot checks and captchas are recognized, ordinary pages are not', () => {
  for (const u of ['https://challenges.cloudflare.com/cdn-cgi/challenge-platform/h/b/turnstile/if/ov2/av0/rcv/abc',
    'https://newassets.hcaptcha.com/captcha/v1/abc/static/hcaptcha.html', 'https://www.google.com/recaptcha/api2/anchor?k=x',
    'https://www.recaptcha.net/recaptcha/api2/bframe', 'https://example.org/?__cf_chl_rt_tk=abc', 'https://geo.captcha-delivery.com/captcha/?x=1',
    'https://example.org/cdn-cgi/challenge-platform/h/g/orchestrate/chl_page/v1']) assert.ok(r.isCaptchaUrl(u), u);
  for (const u of ['https://www.google.com/search?q=recaptcha', 'https://example.org/watch/1', 'https://hcaptcha.com.evil.org/', 'not a link', ''])
    assert.ok(!r.isCaptchaUrl(u), u);
  assert.ok(r.isChallenge('https://example.org/', 'Just a moment...'));
  assert.ok(r.isChallenge('https://example.org/', 'Attention Required! | Cloudflare'));
  assert.ok(r.isChallenge('https://example.org/', 'DDoS-Guard'));
  assert.ok(!r.isChallenge('https://example.org/', 'Just a moment of zen – Episode 3'.replace('Just a moment', 'A moment')));
  assert.ok(!r.isChallenge('https://example.org/shows', 'Popular shows'));
});

test('new windows: sign-ins and checks get a window, same-site links stay, ad tabs are refused', () => {
  assert.strictEqual(r.popupPolicy('new-window', 'https://accounts.example.com/o/oauth2/auth', 'https://site.org/'), 'window');
  assert.strictEqual(r.popupPolicy('foreground-tab', 'https://challenges.cloudflare.com/x', 'https://site.org/'), 'window');
  assert.strictEqual(r.popupPolicy('foreground-tab', 'https://www.site.org/watch/2', 'https://site.org/'), 'here');
  assert.strictEqual(r.popupPolicy('foreground-tab', 'https://cdn.site.org/watch/2', 'https://site.org/'), 'here');
  assert.strictEqual(r.popupPolicy('foreground-tab', 'https://ads.example.net/click', 'https://site.org/'), 'deny');
  assert.strictEqual(r.popupPolicy('new-window', 'file:///etc/passwd', 'https://site.org/'), 'deny');
  assert.strictEqual(r.popupPolicy('new-window', 'javascript:alert(1)', 'https://site.org/'), 'deny');
});

test('the log gets addresses without their queries; the core gets only Theater\'s arguments', () => {
  assert.strictEqual(r.logUrl('https://example.org/watch/1?token=abc'), 'https://example.org/watch/1?…');
  assert.strictEqual(r.logUrl('http://127.0.0.1:8770/'), 'http://127.0.0.1:8770/');
  assert.strictEqual(r.logUrl('data:text/html,x'), 'data:…');
  assert.deepStrictEqual(r.coreArgs(['--hearth-theater', '--fullscreen', '--no-sandbox', '--open', 'http://127.0.0.1:8770/', '--read-catalogs']),
    ['--core-stdio', '--fullscreen', '--open', 'http://127.0.0.1:8770/', '--read-catalogs']);
  assert.deepStrictEqual(r.coreArgs(['--open', 'file:///etc/passwd']), ['--core-stdio']);
});

test('sites get full screen, the pointer lock and copying, nothing else', () => {
  for (const p of ['fullscreen', 'pointerLock', 'clipboard-sanitized-write']) assert.ok(r.allowPermission(p), p);
  for (const p of ['media', 'geolocation', 'notifications', 'midi', 'openExternal', 'hid', 'serial']) assert.ok(!r.allowPermission(p), p);
});
