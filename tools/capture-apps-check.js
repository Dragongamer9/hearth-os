// Checks the pages of Hearth Recorder, Hearth Camera and Hearth Share (hosts/hearth-{recorder,camera,share}/ui)
// in a browser, as the apps show them without their Rust side (each page has a preview mode with sample data):
// no script errors, the controls work with the keyboard, and a screenshot of each screen into shots/.
// Run: node tools/capture-apps-check.js  (needs the playwright package and its Chromium).
'use strict';
const path = require('path');
const fs = require('fs');
const { chromium } = require('playwright');

const root = path.join(__dirname, '..');
const out = path.join(root, 'shots');
fs.mkdirSync(out, { recursive: true });

const steps = {
  recorder: [
    ['recorder-screen', async p => {}],
    ['recorder-recording', async p => { await p.click('[data-act="record"]'); await p.waitForSelector('#timer'); }],
    ['recorder-stopped', async p => { await p.keyboard.press('Escape'); await p.click('[data-act="stop"]'); await p.waitForSelector('[data-act="record"]'); }],
    ['recorder-replay', async p => { await p.click('[data-act="replay"]'); await p.waitForSelector('[data-act="save"]'); await p.click('[data-act="save"]'); }],
    ['recorder-sound', async p => { await p.click('[data-act="stop"]'); await p.keyboard.press('PageDown'); await p.waitForSelector('[data-act="sound"]'); await p.click('[data-act="sound"]'); await p.waitForSelector('#level'); }],
    ['recorder-delete', async p => { await p.click('[data-act="stop"]'); await p.waitForSelector('[data-act="sound"]'); await p.click('[data-act="delete"]'); await p.waitForSelector('#layer .dlg'); }],
  ],
  camera: [
    ['camera-photo', async p => { await p.waitForSelector('#preview'); }],
    ['camera-timer', async p => { await p.click('[data-act="timer"][data-v="3"]'); await p.click('[data-act="shoot"]'); await p.waitForTimeout(800); }],
    ['camera-video', async p => { await p.waitForTimeout(3000); await p.click('[data-act="mode"][data-v="video"]'); await p.click('[data-act="shoot"]'); await p.waitForSelector('#timer'); }],
  ],
  share: [
    ['share-start', async p => {}],
    ['share-browser', async p => { await p.click('[data-act="web-start"]'); await p.waitForSelector('#url'); }],
    ['share-moonlight', async p => { await p.click('[data-act="tab"][data-tab="moonlight"]'); await p.waitForTimeout(300); }],
  ],
};

(async () => {
  const browser = await chromium.launch();
  let failed = 0;
  for (const [app, list] of Object.entries(steps)) {
    const file = path.join(root, 'hosts', 'hearth-' + app, 'ui', 'index.html');
    if (!fs.existsSync(file)) { console.log(`skip ${app}: no page`); continue; }
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    const errors = [];
    page.on('pageerror', e => errors.push(e.message));
    page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
    await page.goto('file://' + file);
    await page.waitForTimeout(700);
    for (const [name, step] of list) {
      try { await step(page); await page.waitForTimeout(400); }
      catch (e) { errors.push(`${name}: ${e.message.split('\n')[0]}`); }
      await page.screenshot({ path: path.join(out, name + '.png') });
    }
    // The keyboard moves the focus ring around without errors.
    for (const k of ['ArrowDown', 'ArrowRight', 'ArrowDown', 'ArrowLeft', 'ArrowUp', 'PageDown', 'PageUp', 'Escape']) await page.keyboard.press(k);
    const focused = await page.evaluate(() => document.activeElement && document.activeElement.classList.contains('f'));
    if (!focused) errors.push('nothing has the focus ring after moving with the keyboard');
    const bad = errors.filter(e => !/preview|ERR_FILE_NOT_FOUND|net::/.test(e));
    console.log(`${app}: ${bad.length ? 'FAILED\n  ' + bad.join('\n  ') : 'ok'}`);
    if (bad.length) failed++;
    await page.close();
  }
  await browser.close();
  process.exit(failed ? 1 : 0);
})();
