/* Hearth Browser: what Hearth adds to Mozilla's Firefox, started by hearth.cfg (Firefox's AutoConfig) at every
 * start. The same file on Hearth OS (/usr/lib/hearth-browser/firefox/hearth) and on Windows (Ember's copy of
 * Firefox).
 *
 *  - Hearth's look: theme.css (the ember palette), big.css (bigger buttons and text, for Game Mode and handhelds).
 *  - The start page (start.html) as the home page and on new tabs.
 *  - An on-screen keyboard that works with a controller: the Keyboard button in the toolbar, or Ctrl+Alt+K. While
 *    it's open the arrow keys (and Tab / Shift+Tab) move between its keys, Enter types the one picked, Alt+Left
 *    deletes a letter and Esc closes it; a click or a tap types a key too. Keys are typed with Firefox's own text
 *    input interface (nsITextInputProcessor), so they reach web pages like a real keyboard's.
 *  - In Game Mode (HEARTH_BROWSER_GAME=1): the window fills the screen, new windows open as tabs, and the keyboard
 *    opens by itself when the address bar is picked.
 *
 * The program that starts Firefox (/usr/bin/hearth-browser on Linux, Ember on Windows) says how through these:
 *   HEARTH_BROWSER_GAME=1      Game Mode (or Ember's full-screen Game Mode)
 *   HEARTH_BROWSER_BIG=1       bigger buttons and text
 *   HEARTH_BROWSER_SCALE=1.5   the size of everything (Firefox's layout.css.devPixelsPerPx), with BIG only
 *   HEARTH_BROWSER_ACCENT=#rrggbb   your profile's accent colour
 * Without them (Firefox started some other way) it's the ordinary windowed browser with Hearth's look.
 */

const XHTML = "http://www.w3.org/1999/xhtml";
const BROWSER_DOC = "chrome://browser/content/browser.xhtml";

function env(name) {
  try { return Services.env.get(name) || ""; } catch (e) { return ""; }
}
const GAME = env("HEARTH_BROWSER_GAME") === "1";
const BIG = env("HEARTH_BROWSER_BIG") === "1";
const SCALE = (() => { const s = parseFloat(env("HEARTH_BROWSER_SCALE")); return s >= 1 && s <= 4 ? s : 0; })();
const ACCENT = /^#[0-9a-fA-F]{6}$/.test(env("HEARTH_BROWSER_ACCENT")) ? env("HEARTH_BROWSER_ACCENT") : "";

function report(what, e) {
  try { Cu.reportError("Hearth Browser (" + what + "): " + e); } catch (_) {}
}

/** The start page: hearth/start.html, next to this file. */
function startPage() {
  const f = Services.dirsvc.get("GreD", Ci.nsIFile);
  f.append("hearth");
  f.append("start.html");
  return Services.io.newFileURI(f).spec;
}
const START = (() => { try { return startPage(); } catch (e) { return "about:home"; } })();

// ------------------------------------------------------------------ settings

/** Hearth's defaults. Defaults only: what you change in Firefox's settings stays changed. */
function defaults() {
  const d = Services.prefs.getDefaultBranch("");
  const set = (name, v) => {
    try {
      if (typeof v === "boolean") d.setBoolPref(name, v);
      else if (typeof v === "number") d.setIntPref(name, v);
      else d.setStringPref(name, v);
    } catch (e) { report("default " + name, e); }
  };
  set("browser.startup.homepage", START);
  set("browser.startup.page", 1);
  // Your own userChrome.css / userContent.css in the profile's chrome folder still work.
  set("toolkit.legacyUserProfileCustomizations.stylesheets", true);
  // Dark, like the rest of Hearth (theme.css adds the ember colours on top).
  set("extensions.activeThemeID", "firefox-compact-dark@mozilla.org");
  set("browser.theme.toolbar-theme", 0);
  set("browser.theme.content-theme", 0);
  set("layout.css.prefers-color-scheme.content-override", 0);
  // No welcome tour, sponsored tiles, Firefox account button or bookmarks bar.
  set("browser.aboutwelcome.enabled", false);
  set("browser.newtabpage.activity-stream.showSponsored", false);
  set("browser.newtabpage.activity-stream.showSponsoredTopSites", false);
  set("browser.newtabpage.activity-stream.feeds.section.topstories", false);
  set("identity.fxaccounts.toolbar.enabled", false);
  set("browser.toolbars.bookmarks.visibility", "never");
  set("browser.uitour.enabled", false);
  set("extensions.pocket.enabled", false);
  // Protected video (Netflix, Disney+, Spotify...): Firefox downloads Google's Widevine itself.
  set("media.eme.enabled", true);
  set("media.gmp-widevinecdm.enabled", true);
  set("media.gmp-widevinecdm.visible", true);
  set("hearth.newtab", true);
}

/** What depends on how it was started: sizes for Game Mode, new windows as tabs. Set at every start, and taken
 *  back when it's started the other way. */
function modePrefs() {
  const p = Services.prefs;
  try {
    p.setBoolPref("hearth.game", GAME);
    if (BIG) {
      p.setIntPref("browser.uidensity", 2);   // Firefox's own "touch" density
      if (SCALE) p.setStringPref("layout.css.devPixelsPerPx", String(SCALE));
      else if (p.getBoolPref("hearth.scale.applied", false)) p.clearUserPref("layout.css.devPixelsPerPx");
      p.setBoolPref("hearth.scale.applied", !!SCALE);
      p.setBoolPref("hearth.big.applied", true);
    } else if (p.getBoolPref("hearth.big.applied", false)) {
      p.clearUserPref("browser.uidensity");
      if (p.getBoolPref("hearth.scale.applied", false)) p.clearUserPref("layout.css.devPixelsPerPx");
      p.setBoolPref("hearth.big.applied", false);
      p.setBoolPref("hearth.scale.applied", false);
    }
    if (GAME) {
      // Pop-ups (a site's "Sign in with Google") open as tabs: a second window would hide behind Hearth.
      p.setIntPref("browser.link.open_newwindow", 3);
      p.setIntPref("browser.link.open_newwindow.restriction", 0);
      p.setBoolPref("browser.fullscreen.autohide", false);
      p.setBoolPref("hearth.game.applied", true);
    } else if (p.getBoolPref("hearth.game.applied", false)) {
      p.clearUserPref("browser.link.open_newwindow.restriction");
      p.clearUserPref("browser.fullscreen.autohide");
      p.setBoolPref("hearth.game.applied", false);
    }
  } catch (e) { report("mode", e); }
}

// ------------------------------------------------------------------ look

function sheet(spec) {
  try {
    const sss = Cc["@mozilla.org/content/style-sheet-service;1"].getService(Ci.nsIStyleSheetService);
    const uri = Services.io.newURI(spec);
    if (!sss.sheetRegistered(uri, sss.USER_SHEET)) sss.loadAndRegisterSheet(uri, sss.USER_SHEET);
  } catch (e) { report("style " + spec.slice(0, 40), e); }
}

function styles() {
  sheet("resource://hearth/theme.css");
  if (BIG) sheet("resource://hearth/big.css");
  if (ACCENT) {
    const css = `@-moz-document url("${BROWSER_DOC}"), url("${START}") { :root { --hearth-user-accent: ${ACCENT} !important; } }`;
    sheet("data:text/css;charset=utf-8," + encodeURIComponent(css));
  }
}

// ------------------------------------------------------------------ the on-screen keyboard

const K = (v, label, w) => ({ t: "key", v, label, w: w || 1 });
const T = (v, w) => ({ t: "text", v, label: v, w: w || 1 });
const C = (v, label, w) => ({ t: "cmd", v, label, w: w || 1 });
const chars = s => [...s].map(ch => ({ t: "char", v: ch, label: ch, w: 1 }));
const BOTTOM = other => [C("symbols", other, 1.5), T(".com", 1.5), K(" ", "Space", 4), K("ArrowLeft", "◀"), K("ArrowRight", "▶"), C("close", "Close", 1.5)];
const LAYOUTS = {
  abc: [
    [...chars("1234567890"), K("Backspace", "⌫", 1.5)],
    chars("qwertyuiop@"),
    [...chars("asdfghjkl-"), K("Enter", "Enter", 1.5)],
    [C("shift", "⇧", 1.5), ...chars("zxcvbnm,./")],
    BOTTOM("&123"),
  ],
  sym: [
    [...chars("!@#$%^&*()"), K("Backspace", "⌫", 1.5)],
    chars("~`_=+[]{}\\|"),
    [...chars(";:'\"<>?€£¥"), K("Enter", "Enter", 1.5)],
    [C("shift", "⇧", 1.5), ...chars("§°•¡¿«»©±÷")],
    BOTTOM("ABC"),
  ],
};
const KEYCODES = { Backspace: 8, Enter: 13, " ": 32, ArrowLeft: 37, ArrowRight: 39 };
const CODES = { Backspace: "Backspace", Enter: "Enter", " ": "Space", ArrowLeft: "ArrowLeft", ArrowRight: "ArrowRight" };

/** Keys the keyboard listens to while it's open (reserved: they come here before the web page). */
const NAV = [
  ["up", "VK_UP", ""], ["down", "VK_DOWN", ""], ["left", "VK_LEFT", ""], ["right", "VK_RIGHT", ""],
  ["down", "VK_TAB", ""], ["up", "VK_TAB", "shift"], ["press", "VK_RETURN", ""], ["backspace", "VK_LEFT", "alt"],
  ["close", "VK_ESCAPE", ""],
];

class Keyboard {
  constructor(win) {
    this.win = win;
    this.doc = win.document;
    this.layout = "abc";
    this.shift = false;
    this.r = 1;
    this.c = 0;
    this.tip = null;
    this.heard = [];   // the last keys it acted on (for the CI test)
    this.build();
  }

  build() {
    const doc = this.doc;
    const box = doc.createElementNS(XHTML, "div");
    box.id = "hearth-osk";
    box.hidden = true;
    box.setAttribute("role", "group");
    box.setAttribute("aria-label", "On-screen keyboard");
    // Clicking a key mustn't take the focus from the text field it types into.
    box.addEventListener("mousedown", e => e.preventDefault());
    box.addEventListener("click", e => {
      const key = e.target.closest(".hearth-key");
      if (!key) return;
      this.r = +key.dataset.r; this.c = +key.dataset.c;
      this.press();
    });
    const anchor = doc.getElementById("browser");
    if (anchor) anchor.after(box); else doc.documentElement.appendChild(box);
    this.box = box;

    // Keys: always Ctrl+Alt+K (open/close); the rest only while the keyboard is open, in a keyset of their own
    // that's put in when it opens and taken out when it closes (Firefox reads a keyset's keys once, so switching
    // single keys on and off doesn't work).
    const commands = doc.createXULElement("commandset");
    commands.id = "hearth-osk-commands";
    const command = (name, fn) => {
      const cmd = doc.createXULElement("command");
      cmd.id = "hearth-osk-cmd-" + name;
      // Acted on once the key press is over: typing from inside it would find Firefox's text input busy.
      cmd.addEventListener("command", () => { this.heard.push(name); this.heard.splice(0, this.heard.length - 20); this.win.setTimeout(fn, 0); });
      commands.appendChild(cmd);
    };
    command("toggle", () => this.toggle());
    const acts = { up: () => this.move(-1, 0), down: () => this.move(1, 0), left: () => this.move(0, -1), right: () => this.move(0, 1),
                   press: () => this.press(), backspace: () => this.typeKey("Backspace"), close: () => this.close() };
    for (const [name, fn] of Object.entries(acts)) command(name, fn);
    doc.documentElement.appendChild(commands);
    doc.documentElement.appendChild(this.keyset("hearth-osk-toggle", [["toggle", { key: "K", modifiers: "accel,alt" }]]));
    this.render();
  }

  get rows() { return LAYOUTS[this.layout]; }
  get isOpen() { return !this.box.hidden; }

  render() {
    const doc = this.doc;
    this.box.replaceChildren();
    this.rows.forEach((row, r) => {
      const line = doc.createElementNS(XHTML, "div");
      line.className = "hearth-row";
      row.forEach((k, c) => {
        const el = doc.createElementNS(XHTML, "div");
        el.className = "hearth-key" + (k.t === "char" ? "" : " hearth-key-wide") + (r === this.r && c === this.c ? " hearth-picked" : "")
          + (k.t === "cmd" && k.v === "shift" && this.shift ? " hearth-on" : "");
        el.style.flexGrow = String(k.w);
        el.dataset.r = r; el.dataset.c = c;
        el.setAttribute("role", "button");
        el.textContent = k.t === "char" && this.shift ? k.label.toUpperCase() : k.label;
        line.appendChild(el);
      });
      this.box.appendChild(line);
    });
  }

  /** Moves the pick; up and down go to the key nearest the middle of the one picked now. */
  move(dr, dc) {
    const rows = this.rows;
    if (dc) {
      const n = rows[this.r].length;
      this.c = (this.c + dc + n) % n;
    } else {
      const mid = (row, c) => { let x = 0; for (let i = 0; i < c; i++) x += row[i].w; const all = row.reduce((s, k) => s + k.w, 0); return (x + row[c].w / 2) / all; };
      const at = mid(rows[this.r], this.c);
      this.r = (this.r + dr + rows.length) % rows.length;
      let best = 0, bestD = 9;
      rows[this.r].forEach((_, c) => { const d = Math.abs(mid(rows[this.r], c) - at); if (d < bestD) { bestD = d; best = c; } });
      this.c = best;
    }
    this.render();
  }

  press() {
    const k = this.rows[this.r] && this.rows[this.r][this.c];
    if (!k) return;
    if (k.t === "char") {
      this.typeChar(this.shift ? k.v.toUpperCase() : k.v);
      if (this.shift) { this.shift = false; this.render(); }
    } else if (k.t === "text") {
      for (const ch of k.v) this.typeChar(ch);
    } else if (k.t === "key") {
      const inAddressBar = this.addressBarFocused();
      this.typeKey(k.v);
      if (k.v === "Enter" && inAddressBar) this.close();
    } else if (k.v === "shift") {
      this.shift = !this.shift; this.render();
    } else if (k.v === "symbols") {
      this.layout = this.layout === "abc" ? "sym" : "abc";
      this.r = Math.min(this.r, this.rows.length - 1);
      this.c = Math.min(this.c, this.rows[this.r].length - 1);
      this.render();
    } else if (k.v === "close") {
      this.close();
    }
  }

  addressBarFocused() {
    try { const u = this.win.gURLBar; return !!u && (u.focused || this.doc.activeElement === u.inputField); } catch (e) { return false; }
  }

  textInput() {
    if (!this.tip) this.tip = Cc["@mozilla.org/text-input-processor;1"].createInstance(Ci.nsITextInputProcessor);
    return this.tip.beginInputTransaction(this.win, () => true) ? this.tip : null;
  }

  /** One key down and up, through Firefox's text input interface: to whatever has the keyboard focus (the address
   *  bar, or the text field in the page). The keyboard's own keys are off meanwhile, or they'd catch it. */
  dispatch(init) {
    const tip = this.textInput();
    if (!tip) return;
    this.navKeys(false);
    try {
      const e = new this.win.KeyboardEvent("", init);
      tip.keydown(e);
      tip.keyup(e);
    } catch (e) {
      report("typing", e);
    } finally {
      this.navKeys(this.isOpen);
    }
  }
  typeChar(ch) { this.dispatch({ key: ch }); }
  typeKey(name) { this.dispatch({ key: name, code: CODES[name] || "", keyCode: KEYCODES[name] || 0 }); }

  /** A keyset of reserved keys (they come here before the web page), each running one of the commands above. */
  keyset(id, keys) {
    const set = this.doc.createXULElement("keyset");
    set.id = id;
    for (const [name, attrs] of keys) {
      const key = this.doc.createXULElement("key");
      for (const [k, v] of Object.entries(attrs)) if (v) key.setAttribute(k, v);
      key.setAttribute("command", "hearth-osk-cmd-" + name);
      key.setAttribute("reserved", "true");
      set.appendChild(key);
    }
    return set;
  }

  navKeys(on) {
    const old = this.doc.getElementById("hearth-osk-nav");
    if (on && !old) this.doc.documentElement.appendChild(this.keyset("hearth-osk-nav", NAV.map(([name, keycode, modifiers]) => [name, { keycode, modifiers }])));
    else if (!on && old) old.remove();
  }

  open() {
    if (this.isOpen) return;
    this.box.hidden = false;
    this.navKeys(true);
    this.render();
    this.button(true);
  }
  close() {
    if (!this.isOpen) return;
    this.box.hidden = true;
    this.navKeys(false);
    this.shift = false;
    this.button(false);
  }
  toggle() { if (this.isOpen) this.close(); else this.open(); }
  button(on) {
    const b = this.doc.getElementById("hearth-keyboard-button");
    if (b) { if (on) b.setAttribute("checked", "true"); else b.removeAttribute("checked"); }
  }
}

// One keyboard per window, found by the window's number (the same window can reach here through different
// wrappers, so not by the object).
const boards = new Map();
function keyboardOf(win) {
  const id = win.docShell.outerWindowID;
  let k = boards.get(id);
  if (!k) {
    k = new Keyboard(win);
    boards.set(id, k);
    win.addEventListener("unload", () => boards.delete(id), { once: true });
  }
  return k;
}

/** Firefox's toolbar module (moved to moz-src:// in newer versions). */
function customizableUI() {
  for (const url of ["moz-src:///browser/components/customizableui/CustomizableUI.sys.mjs", "resource:///modules/CustomizableUI.sys.mjs"]) {
    try { return ChromeUtils.importESModule(url).CustomizableUI; } catch (e) {}
  }
  throw new Error("Firefox's CustomizableUI module wasn't found");
}

let buttonMade = false;
function keyboardButton() {
  if (buttonMade) return;
  try {
    const CustomizableUI = customizableUI();
    CustomizableUI.createWidget({
      id: "hearth-keyboard-button",
      type: "button",
      label: "Keyboard",
      tooltiptext: "On-screen keyboard (Ctrl+Alt+K)",
      defaultArea: CustomizableUI.AREA_NAVBAR,
      onCommand: e => keyboardOf(e.target.ownerGlobal).toggle(),
    });
    buttonMade = true;
  } catch (e) { report("keyboard button", e); }
}

// ------------------------------------------------------------------ windows

function newTabPage() {
  if (!Services.prefs.getBoolPref("hearth.newtab", true)) return;
  try {
    const { AboutNewTab } = ChromeUtils.importESModule("resource:///modules/AboutNewTab.sys.mjs");
    if (AboutNewTab.newTabURL !== START) AboutNewTab.newTabURL = START;
  } catch (e) { report("new tab page", e); }
}

function fillScreen(win) {
  const fit = () => {
    try {
      if (win.closed || win.fullScreen) return;
      if (win.outerWidth >= win.screen.width && win.outerHeight >= win.screen.height) return;
      win.moveTo(0, 0);
      win.resizeTo(win.screen.width, win.screen.height);
    } catch (e) { report("window size", e); }
  };
  fit();
  win.setTimeout(fit, 600);
  win.setTimeout(fit, 2000);
}

function setUpWindow(win) {
  try {
    if (win.document.documentURI !== BROWSER_DOC) return;
    newTabPage();
    keyboardButton();
    // The start page on a new tab leaves the address bar empty, as Firefox's own does.
    try { if (Array.isArray(win.gInitialPages) && !win.gInitialPages.includes(START)) win.gInitialPages.push(START); } catch (e) {}
    const kb = keyboardOf(win);
    if (GAME) {
      fillScreen(win);
      // Picking the address bar with a controller: the keyboard comes up with it.
      try { win.gURLBar.inputField.addEventListener("focus", () => kb.open()); } catch (e) { report("address bar", e); }
    }
    Services.prefs.setBoolPref("hearth.keyboard.ready", true);
  } catch (e) { report("window", e); }
}

let started = false;
/** Called by hearth.cfg, before any window is open. */
export function start() {
  if (started) return;
  started = true;
  defaults();
  modePrefs();
  styles();
  Services.obs.addObserver(win => setUpWindow(win), "browser-delayed-startup-finished");
  Services.prefs.setIntPref("hearth.started", Math.floor(Date.now() / 1000));
}

// For the CI test (tools/hearth-browser-test.py): the start page's address and the keyboard of a window.
export const HearthBrowser = { START, GAME, BIG, SCALE, keyboardOf };
