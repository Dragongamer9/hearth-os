/* Hearth Browser: what Hearth adds to Mozilla's Firefox, started by hearth.cfg (Firefox's AutoConfig) at every
 * start. The same file on Hearth OS (/usr/lib/hearth-browser/firefox/hearth) and on Windows (Ember's copy of
 * Firefox).
 *
 *  - Hearth's look: theme.css (the ember palette), big.css (bigger buttons and text, for Game Mode and handhelds).
 *  - The start page (start.html) as the home page and on new tabs.
 *  - Hearth's on-screen keyboard (hearth/osk, the same one as Game Mode's and Desktop Mode's: docs/OSK.md), for a
 *    controller: the Keyboard button in the toolbar, or Ctrl+Alt+K. While it's open the arrow keys (and Tab /
 *    Shift+Tab) move between its keys, Enter types the one picked, Alt+Left deletes a letter and Esc closes it; a
 *    click or a tap types a key too, and it suggests words. Keys are typed with Firefox's own text input interface
 *    (nsITextInputProcessor), so they reach web pages like a real keyboard's.
 *  - In Game Mode (HEARTH_BROWSER_GAME=1): the window fills the screen, new windows open as tabs, the keyboard
 *    opens by itself when the address bar is picked, and the D-pad moves between a page's links, buttons and fields
 *    with Hearth's focus ring (A opens, B goes back: spatial-child.sys.mjs).
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
  // Firefox's first-start window (terms of use, with a choice about data): Hearth's policies already decide.
  set("browser.preonboarding.enabled", false);
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

// ------------------------------------------------------------------ the D-pad in web pages (Game Mode)
// spatial-child.sys.mjs, as window actors in each page's process: arrows move the focus between links, buttons and
// fields (before the page sees them); Esc goes back when the page didn't use it (after the page's own handlers).

function spatial() {
  if (!GAME) return;
  const ring = ACCENT || "#ff8a3d";
  // Hearth's focus ring on whatever the D-pad picked, over the page's own styles (web pages only, not Firefox's window).
  sheet("data:text/css;charset=utf-8," + encodeURIComponent(
    `@-moz-document url-prefix("http:"), url-prefix("https:"), url-prefix("file:") {
       :focus-visible { outline: 3px solid ${ring} !important; outline-offset: 2px !important; }
     }`));
  const done = [];
  const reg = (name, events) => {
    try {
      ChromeUtils.registerWindowActor(name, {
        parent: { esModuleURI: "resource://hearth/spatial-parent.sys.mjs" },
        child: { esModuleURI: "resource://hearth/spatial-child.sys.mjs", events },
        allFrames: false,
        // It runs in web pages' processes on purpose: the child side only reads the page and moves its focus, and
        // the parent side only keeps what it reports (as plain text) for the console and the CI test.
        safeForUntrustedWebProcess: true,
      });
      done.push(name);
    } catch (e) { report("actor " + name, e); done.push(name + ": " + e); }
  };
  reg("HearthSpatial", { keydown: { capture: true } });
  reg("HearthSpatialBack", { keydown: { mozSystemGroup: true } });
  try { Services.prefs.setStringPref("hearth.spatial", done.join("; ")); } catch (e) {}
}

// ------------------------------------------------------------------ the on-screen keyboard
// Hearth's shared keyboard (osk/hearth-osk.js, the same as Game Mode's and Desktop Mode's: docs/OSK.md), loaded into
// the window and drawn under the page. Keys reach the page through Firefox's own text input interface.

const KEYCODES = { Backspace: 8, Tab: 9, Enter: 13, Escape: 27, " ": 32, Home: 36, End: 35, ArrowLeft: 37, ArrowUp: 38, ArrowRight: 39, ArrowDown: 40 };
const CODES = { Backspace: "Backspace", Tab: "Tab", Enter: "Enter", Escape: "Escape", " ": "Space", Home: "Home", End: "End",
                ArrowLeft: "ArrowLeft", ArrowRight: "ArrowRight", ArrowUp: "ArrowUp", ArrowDown: "ArrowDown" };
const KEYNAMES = { enter: "Enter", tab: "Tab", esc: "Escape", left: "ArrowLeft", right: "ArrowRight", up: "ArrowUp", down: "ArrowDown", home: "Home", end: "End" };

/** The shared keyboard's code, in this window (once). */
function oskIn(win) {
  if (!win.HearthOSK) {
    const load = url => Services.scriptloader.loadSubScriptWithOptions(url, { target: win, charset: "UTF-8", ignoreCache: true });
    load("resource://hearth/osk/hearth-osk.js");
    const H = win.HearthOSK;
    let learned = null;
    try { learned = JSON.parse(Services.prefs.getStringPref("hearth.osk.learned", "") || "null"); } catch (e) {}
    H.configure({
      loadWords: lang => { try { load("resource://hearth/osk/words/" + lang + ".js"); } catch (e) { report("words " + lang, e); } },
      save: d => { try { const t = JSON.stringify(d); if (t.length < 400000) Services.prefs.setStringPref("hearth.osk.learned", t); } catch (e) {} },
      learned: learned || undefined,
    });
    // Settings › Keyboard (Hearth OS saves it in ~/.config/nimbus/osk.json).
    try {
      const home = Services.dirsvc.get("Home", Ci.nsIFile).path;
      IOUtils.readUTF8(PathUtils.join(home, ".config", "nimbus", "osk.json")).then(t => H.configure({ settings: JSON.parse(t) })).catch(() => {});
    } catch (e) {}
    const link = win.document.createElementNS(XHTML, "link");
    link.setAttribute("rel", "stylesheet");
    link.setAttribute("href", "resource://hearth/osk/hearth-osk.css");
    (win.document.head || win.document.documentElement).appendChild(link);
  }
  return win.HearthOSK;
}

class Keyboard {
  constructor(win) {
    this.win = win;
    this.doc = win.document;
    this.tip = null;
    this.heard = [];   // the last keys it acted on (for the CI test)
    this.typing = false;
    this.shift = false;
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
    const anchor = doc.getElementById("browser");
    if (anchor) anchor.after(box); else doc.documentElement.appendChild(box);
    this.box = box;
    try {
      const H = oskIn(this.win);
      this.engine = H.createEngine({
        host: "browser",
        adapter: {
          context: () => this.addressBarFocused() ? (() => { try { const f = this.win.gURLBar.inputField; return f.value.slice(0, f.selectionStart); } catch (e) { return undefined; } })() : undefined,
          insert: text => this.typeText(text),
          deleteBack: n => { for (let i = 0; i < n; i++) this.typeKey("Backspace"); },
          key: name => {
            if (name === "paste") return this.win.goDoCommand && this.win.goDoCommand("cmd_paste");
            const k = KEYNAMES[name]; if (!k) return;
            const inAddressBar = this.addressBarFocused();
            this.typeKey(k);
            if (k === "Enter" && inAddressBar) this.close();
          },
        },
        onHide: () => this.close(),
      });
      this.view = H.mount(box, this.engine, { doc });
      // The controller reaches it as keys (Hearth Input: arrows, Enter, Alt+Left, Esc), so no button hints on the keys.
      this.engine.setHints(false);
      if (BIG) H.setSetting("size", "l");
    } catch (e) { report("keyboard", e); }

    // Keys: always Ctrl+Alt+K (open/close); the rest only while the keyboard is open. Caught on the window first
    // (capturing, in Firefox's system group), so they never reach the web page or Firefox's own shortcuts.
    this.onKey = e => this.key(e);
    this.win.addEventListener("keydown", this.onKey, { capture: true, mozSystemGroup: true });
  }

  get isOpen() { return !this.box.hidden; }
  // The picked key (rows from the top: 0 is the number row) and the letters or symbols (for the CI test).
  get r() { return this.engine ? this.engine.state.sel.r : 0; }
  set r(v) { if (this.engine) this.engine.state.sel.r = v; }
  get c() { return this.engine ? this.engine.state.sel.c : 0; }
  set c(v) { if (this.engine) this.engine.state.sel.c = v; }
  get layout() { return this.engine && this.engine.state.panel === "sym" ? "sym" : "abc"; }
  set layout(v) { if (this.engine && (v === "sym") !== (this.engine.state.panel === "sym")) this.engine.setPanel(v === "sym" ? "sym" : "letters"); }
  render() { if (this.view) this.view.render(); }

  addressBarFocused() {
    try { const u = this.win.gURLBar; return !!u && (u.focused || this.doc.activeElement === u.inputField); } catch (e) { return false; }
  }

  textInput() {
    if (!this.tip) this.tip = Cc["@mozilla.org/text-input-processor;1"].createInstance(Ci.nsITextInputProcessor);
    return this.tip.beginInputTransaction(this.win, () => true) ? this.tip : null;
  }

  /** One key down and up, through Firefox's text input interface: to whatever has the keyboard focus (the address
   *  bar, or the text field in the page). The keyboard doesn't catch its own keys meanwhile. */
  dispatch(init) {
    const tip = this.textInput();
    if (!tip) return;
    this.typing = true;
    try {
      const e = new this.win.KeyboardEvent("", init);
      tip.keydown(e);
      tip.keyup(e);
    } catch (e) {
      report("typing", e);
    } finally {
      this.typing = false;
    }
  }
  typeChar(ch) { this.dispatch({ key: ch }); }
  typeKey(name) { this.dispatch({ key: name, code: CODES[name] || "", keyCode: KEYCODES[name] || 0 }); }
  /** Text: one plain character as a key press; anything else (emoji, a picked word, ".com") committed at once. */
  typeText(text) {
    if (text.length === 1) return this.typeChar(text);
    const tip = this.textInput();
    if (!tip) return;
    this.typing = true;
    try { tip.commitCompositionWith(text); } catch (e) { report("typing", e); for (const ch of text) this.typeChar(ch); } finally { this.typing = false; }
  }

  /** A key pressed on a keyboard (or sent by the controller through Hearth Input). Acted on once the key press is
   *  over: typing from inside it would find Firefox's text input busy. */
  key(e) {
    if (this.typing || e.defaultPrevented) return;
    let act = null;
    if (e.key.toLowerCase() === "k" && e.ctrlKey && e.altKey && !e.shiftKey && !e.metaKey) act = "toggle";
    else if (this.isOpen && !e.ctrlKey && !e.metaKey) {
      const plain = !e.altKey;
      act = e.key === "ArrowUp" && plain ? "up" : e.key === "ArrowDown" && plain ? "down"
        : e.key === "ArrowLeft" && plain ? "left" : e.key === "ArrowRight" && plain ? "right"
        : e.key === "ArrowLeft" && e.altKey ? "backspace"
        : e.key === "Tab" && plain ? (e.shiftKey ? "up" : "down")
        : e.key === "Enter" && plain ? "press" : e.key === "Escape" && plain ? "close" : null;
    }
    if (!act) return;
    e.preventDefault();
    e.stopPropagation();
    this.heard.push(act);
    this.heard.splice(0, this.heard.length - 20);
    const pad = b => () => { if (this.engine) this.engine.handle(b, "press"); };
    const acts = { toggle: () => this.toggle(), up: pad("up"), down: pad("down"), left: pad("left"), right: pad("right"),
                   press: pad("a"), backspace: pad("x"), close: () => this.close() };
    this.win.setTimeout(acts[act], 0);
  }

  open() {
    if (this.isOpen) return;
    this.box.hidden = false;
    if (this.engine) { const sel = this.engine.state.sel; this.engine.reset(this.addressBarFocused() ? "url" : "text", this.addressBarFocused() ? "Go" : ""); this.engine.state.sel = sel; }
    this.render();
    this.button(true);
    this.told(true);
  }
  close() {
    if (!this.isOpen) return;
    this.box.hidden = true;
    this.button(false);
    this.told(false);
  }
  /** Pages' D-pad navigation (spatial-child.sys.mjs) leaves the keys alone while the keyboard is open. */
  told(open) { try { Services.prefs.setBoolPref("hearth.osk.open", open); } catch (e) {} }
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
      // Windows: maximized (Windows keeps its own edges right); gamescope has no window manager, so the size itself.
      if (Services.appinfo.OS === "WINNT") { if (win.windowState !== win.STATE_MAXIMIZED) win.maximize(); return; }
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
  try { Services.prefs.setBoolPref("hearth.osk.open", false); } catch (e) {}
  styles();
  spatial();
  Services.obs.addObserver(win => setUpWindow(win), "browser-delayed-startup-finished");
  Services.prefs.setIntPref("hearth.started", Math.floor(Date.now() / 1000));
}

// For the CI test (tools/hearth-browser-test.py): the start page's address and the keyboard of a window.
export const HearthBrowser = { START, GAME, BIG, SCALE, keyboardOf };
