// NODE_PATH must resolve Playwright. Uses the installed Chromium, never invokes AI.
const assert = require('node:assert/strict');
const { spawn } = require('node:child_process');
const path = require('node:path');
const { chromium } = require('playwright');
(async () => {
  const server = spawn('python3', ['-u', 'server.py', '--no-browser'], {
    cwd: path.resolve(__dirname, '..'), stdio: ['ignore', 'pipe', 'pipe']
  });
  let browser;
  try {
    const url = await new Promise((resolve, reject) => {
      let output = '';
      const timer = setTimeout(() => reject(Error('Server startup timed out')), 10000);
      server.once('error', error => { clearTimeout(timer); reject(error); });
      server.once('exit', () => { clearTimeout(timer); reject(Error('Server exited before startup')); });
      server.stdout.on('data', chunk => {
        output += chunk;
        const match = output.match(/http:\/\/127\.0\.0\.1:\d+\/\?token=[\w-]+/);
        if (match) { clearTimeout(timer); resolve(match[0]); }
      });
    });
    browser = await chromium.launch({ executablePath: process.env.HEARTH_CHROMIUM || '/usr/bin/chromium' });
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
    await page.goto(url);
    await page.waitForFunction(() => document.querySelectorAll('#agents .agent').length === 4);
    assert.equal(await page.locator('#games .game').count(), 5);
    assert.ok(await page.locator('#system').innerText());
    await page.locator('#refresh').click();
    await page.waitForFunction(() => document.querySelector('#error').textContent === '');
    assert.equal(await page.locator('#chosen').innerText(), 'No agent selected');
    assert.deepEqual(errors, []);
    await page.screenshot({ path: process.env.HEARTH_SCREENSHOT || '/tmp/hearth-ai-command-center.png', fullPage: true });
    console.log('ok: live dashboard loads providers, diagnostics and gaming tools under CSP; no AI invoked');
  } finally {
    if (browser) await browser.close();
    server.kill('SIGTERM');
  }
})().catch(error => { console.error(error.message); process.exitCode = 1; });
