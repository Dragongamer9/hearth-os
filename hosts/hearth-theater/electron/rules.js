'use strict';
// Hearth Theater's window app: the decisions that don't need Electron, so node can test them (rules.test.js).

/** The browser Theater says it is: Chromium of the version actually drawing (Electron's own name left out, as any
 *  Chromium-based browser's). Sites and bot checks see the same Chromium in the user agent and its client hints. */
function userAgent(chromeVersion) {
  const major = (String(chromeVersion || '').match(/^\d+/) || ['140'])[0];
  return `Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/${major}.0.0.0 Safari/537.36`;
}

// Bot checks and captchas: Theater never runs its own scripts on these pages or in these frames (the reader waits
// for the check to pass; the viewer's play/pause and fill leave them alone), so they see a plain browser.
const CAPTCHA_HOSTS = ['challenges.cloudflare.com', 'hcaptcha.com', 'recaptcha.net', 'arkoselabs.com', 'funcaptcha.com',
  'captcha-delivery.com', 'perimeterx.net', 'px-cloud.net', 'geetest.com', 'mtcaptcha.com', 'friendlycaptcha.eu', 'friendlycaptcha.com'];
const CHALLENGE_TITLE = /^\s*(just a moment|attention required|one more step|checking your browser|checking if the site connection is secure|verifying you are human|verify you are human|ddos-guard|security check|are you a robot|human verification|bot verification)\b/i;

function hostOf(u) { try { return new URL(u).hostname.toLowerCase(); } catch { return ''; } }

/** A captcha's or bot check's own address (its frame, or the page it sends you to). */
function isCaptchaUrl(u) {
  const h = hostOf(u);
  if (!h) return false;
  if (CAPTCHA_HOSTS.some(c => h === c || h.endsWith('.' + c))) return true;
  let p = '';
  try { const x = new URL(u); p = x.pathname + x.search; } catch { return false; }
  if (/(^|\.)(google\.com|gstatic\.com)$/.test(h) && /^\/recaptcha\//.test(p)) return true;
  return /\/cdn-cgi\/challenge-platform\//.test(p) || /[?&]__cf_chl_/.test(p);
}

/** A page that is a bot check right now ("Just a moment...", Cloudflare's or DDoS-Guard's interstitial...). */
function isChallenge(url, title) { return isCaptchaUrl(url) || CHALLENGE_TITLE.test(String(title || '')); }

/** What to do with a page's request for a new window: sign-ins and bot checks open popups ("window", a real window
 *  with the same cookies, closing itself when done); a link to the same site opens in the viewer ("here"); other
 *  new tabs (mostly ads) are refused ("deny"). */
function popupPolicy(disposition, url, openerUrl) {
  if (!/^https?:\/\//i.test(String(url || ''))) return 'deny';
  if (disposition === 'new-window') return 'window';
  if (isCaptchaUrl(url)) return 'window';
  const a = hostOf(url).replace(/^www\./, ''), b = hostOf(openerUrl).replace(/^www\./, '');
  if (a && b && (a === b || a.endsWith('.' + b) || b.endsWith('.' + a))) return 'here';
  return 'deny';
}

/** A web address for the log: its origin and path only (a query can hold sign-in codes). */
function logUrl(u) {
  try {
    const x = new URL(u);
    if (!/^https?:$/.test(x.protocol)) return x.protocol + '…';
    return x.origin + x.pathname.slice(0, 120) + (x.search ? '?…' : '');
  } catch { return '(not a web address)'; }
}

/** The core's arguments from the window app's: Theater's own only. */
function coreArgs(argv) {
  const out = ['--core-stdio'];
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--fullscreen' || a === '--read-catalogs') out.push(a);
    else if (a === '--open' && argv[i + 1] && /^https?:\/\//.test(argv[i + 1])) out.push(a, argv[++i]);
  }
  return out;
}

/** Permissions a site may have in Theater: full screen, the pointer lock and copying. Not the camera, microphone,
 *  location, notifications or anything else. */
function allowPermission(p) { return ['fullscreen', 'pointerLock', 'clipboard-sanitized-write'].includes(p); }

module.exports = { userAgent, isCaptchaUrl, isChallenge, popupPolicy, logUrl, coreArgs, allowPermission };
