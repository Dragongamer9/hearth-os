// CI only (render.mjs): Hearth Classic's real plugin (src/index.tsx) started the way Decky starts it, over a stand-in
// of Steam's Game Mode (a Library page with tabs, a game page, Steam's Home, the main menu and Quick Access). Steam's
// Home is drawn through the plugin's own route patch (what routerHook.addPatch("/library/home") got), so the home shot
// is the real path. Hearth Music's own focus reader (hosts/decky-hearth-music/src/steam.ts appIdOf) is loaded too, to
// check that it sees the game of a focused Hearth home tile.
import { createRoot } from "react-dom/client";
import { createElement as h, useEffect, useState } from "react";
import plugin from "../../src/index.tsx";
import { appIdOf } from "../../../decky-hearth-music/src/steam.ts";

window.HC_MUSIC_APPID = appIdOf;
const p = plugin();
window.HC_PLUGIN = p;
const scene = window.HC_SCENE || "home";
const caps = ["#5c677d", "#7d8597", "#33415c", "#979dac", "#5c677d", "#33415c", "#7d8597"];

function SteamHome() {
  return h("div", { className: "steam-home" }, h("div", { className: "steam-home-title" }, "Steam's own Home"),
    h("div", { className: "grid" }, ...caps.slice(0, 5).map((c, i) => h("div", { key: i, className: "cap", style: { background: c } }))));
}
function Library() {
  return h("div", { className: "gtp_Page steam-page" },
    h("div", { className: "gtp_Header" }, ...["Great on Deck", "All games", "Installed", "Collections", "Non-Steam"].map((t, i) =>
      h("div", { key: t, className: "gtp_Tab" + (i === 1 ? " gtp_Selected" : "") }, t, " ", h("span", { className: "gtp_Count" }, String(12 + i * 7))))),
    h("div", { className: "lib_Root" }, h("div", { className: "lib_Header" }, "All games"),
      h("div", { className: "grid" }, ...caps.map((c, i) => h("div", { key: i, className: "cap-wrap" + (i === 1 ? " gpfocus" : "") },
        h("div", { className: "cap cap-grey", style: { background: c } }))))));
}
function GamePage() {
  return h("div", { className: "ad_Container steam-page" },
    h("div", { className: "ad_Capsule hero" }, h("div", { className: "ad_Gradient" }), h("div", { className: "hero-title" }, "Ember Knights of Ash")),
    h("div", { className: "ps_PlayBar" }, h("div", { className: "ps_Container bar" },
      h("button", { className: "DialogButton ps_Play ps_Green gpfocus play" }, "Play"),
      h("div", { className: "ps_Stat" }, "Last played Today"), h("div", { className: "ps_Stat" }, "Play time 12.6 hours"),
      h("button", { className: "DialogButton ps_Menu menu" }, "⚙"))),
    h("div", { className: "ad_Content" }, h("div", { className: "ad_Info info" }, h("b", null, "Activity"), h("div", null, "Your friends played this 3 times this week.")),
      h("div", { className: "ad_Links info" }, h("b", null, "Achievements"), h("div", null, "18 of 40 unlocked"))));
}
// Steam's main menu in its own window, as on a device: CSS Loader's "MainMenu" tab, a popup of the main window
// (found through g_PopupManager), with the real structure (#MainNavMenuContainer > .Panel > * > .Panel.Focusable, from
// CSS Loader themes). Here an iframe stands in for that popup window. HC_MENU: "steam" (Steam's structure), "changed"
// (a Steam that changed it: no .Panel). HC_MENUWIDTH: the window's width (a narrow one plays a side-panel-only window).
function openMenuWindow() {
  // Steam's main window keeps an empty place for the side menu while it's open (the blank menu on the owner's device).
  const anchor = document.createElement("div");
  anchor.className = "mm_Anchor";
  anchor.style.cssText = "position:absolute;left:0;top:0;bottom:40px;width:241px;background:#1b2028;z-index:6";
  document.body.appendChild(anchor);
  const f = document.createElement("iframe");
  f.id = "menuwin";
  f.style.cssText = `position:absolute;left:0;top:0;width:${window.HC_MENUWIDTH || 1280}px;height:760px;border:0;z-index:20;background:transparent`;
  document.body.appendChild(f);
  const d = f.contentDocument;
  const running = !!window.HC_RUNNING;
  const items = ["Home", "Library", "Store", "Friends & Chat", "Media", "Downloads", "Settings", "Power"];
  const svg = '<svg viewBox="0 0 24 24" width="22" height="22"><circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" stroke-width="2"/></svg>';
  const entries = items.map((t, i) => `<div class="mm_wrap"><div class="Panel Focusable mm_entry${i === 1 ? " gpfocus" : ""}">${svg}<div class="mm_label">${t}</div></div></div>`).join("");
  const inner = window.HC_MENU === "changed" ? `<div class="mm_list">${entries}</div>` : `<div class="Panel mm_panel">${entries}</div>`;
  d.open();
  d.write(`<!doctype html><html><head><title>MainMenu_uid2</title><style>
body{margin:0;background:transparent;font-family:"Noto Sans",Arial,sans-serif;color:#dcdedf}
.scrim{position:fixed;inset:0;background:rgba(0,0,0,.45)}
#MainNavMenuContainer{position:absolute;left:0;top:0;bottom:0;width:300px;background:#0e141b;box-shadow:8px 0 30px rgba(0,0,0,.55)}
.mm_panel,.mm_list{display:flex;flex-direction:column;padding:${running ? 120 : 18}px 8px 18px}
.mm_entry{display:flex;align-items:center;gap:14px;padding:10px 16px;font-size:17px;border-radius:2px}.mm_entry.gpfocus{background:#3d4450}
.game{position:absolute;left:0;top:0;width:300px;padding:16px;box-sizing:border-box;font-weight:800;font-size:18px;z-index:2}
.cover{position:fixed;inset:0;background:#000;z-index:5000}
</style></head><body><div class="scrim"></div>${running ? '<div class="game">Ember Knights · Resume · Exit game</div>' : ""}${window.HC_COVER ? '<div class="cover"></div>' : ""}<div id="MainNavMenuContainer" class="Focusable">${inner}</div></body></html>`);
  d.close();
  // HC_MENURESIZE "steamclient": the window's own SteamClient.Window can move and resize it (what Steam's popups have);
  // otherwise nothing can (window.resizeTo does nothing to an iframe), as when Steam keeps it narrow.
  if (window.HC_MENURESIZE === "steamclient") {
    const px = (v) => parseFloat(v) || 0;
    f.contentWindow.SteamClient = { Window: {
      GetWindowDimensions: async () => ({ x: px(f.style.left), y: px(f.style.top), width: px(f.style.width), height: px(f.style.height) }),
      // As on the owner's device: the third argument must be a number.
      ResizeTo: (w, h, k) => { if (typeof k !== "number") throw new Error("ResizeTo requires 3 arguments: <x: double> <y: double> <dpi: double>"); f.style.width = w + "px"; f.style.height = h + "px"; },
      MoveTo: (x, y, k) => { if (typeof k !== "number") throw new Error("MoveTo requires 3 arguments: <x: double> <y: double> <dpi: double>"); f.style.left = x + "px"; f.style.top = y + "px"; },
      GetWindowRestoreDetails: async () => JSON.stringify({ l: f.style.left, t: f.style.top, w: f.style.width, h: f.style.height }),
      RestoreWindowSizeAndPosition: (d) => { const o = JSON.parse(d); f.style.left = o.l; f.style.top = o.t; f.style.width = o.w; f.style.height = o.h; },
    } };
  }
  // HC_MENURESIZE "browserview": as on a device, the menu is one of Steam's browser views; the focus controller's
  // active browser view (named MainMenu_uid2) has the BrowserViewPopup with GetBounds/SetBounds that move it.
  if (window.HC_MENURESIZE === "browserview") {
    const px = (v) => parseFloat(v) || 0;
    window.FocusNavController = { m_activeBrowserView: { m_strName: "MainMenu_uid2", m_browserView: {
      GetBounds: () => ({ x: px(f.style.left), y: px(f.style.top), width: px(f.style.width), height: px(f.style.height) }),
      SetBounds: (x, y, w, h) => { f.style.left = x + "px"; f.style.top = y + "px"; f.style.width = w + "px"; f.style.height = h + "px"; },
    } } };
  }
  try { f.contentWindow.focus(); } catch (e) { /* fine */ }
  window.g_PopupManager = { GetPopups: () => [{ m_strName: "MainMenu_uid2", m_popup: f.contentWindow }] };
}
if (window.HC_MENU) openMenuWindow();

function Dialog() {
  const row = (label, desc, ctl, focus) => h("div", { className: "gd_Field dlg-row" + (focus ? " gpfocus" : "") },
    h("div", null, h("div", { className: "qam-label" }, label), h("div", { className: "qam-desc" }, desc)), ctl);
  return h("div", { className: "steam-page dlg-page" }, h("div", { className: "dlg-side" }, ...["System", "Display", "Audio", "Controller", "Downloads", "Storage"].map((t, i) =>
      h("div", { key: t, className: "dlg-tab" + (i === 1 ? " on" : "") }, t))),
    h("div", { className: "gd_Content dlg" }, h("div", { className: "dlg-title" }, "Display"),
      row("Brightness", "Screen brightness", h("div", { className: "slider" }, h("b", { style: { width: "62%" } }))),
      row("Adaptive brightness", "Changes with the light around you", h("div", { className: "qam-toggle on" }, h("span", { className: "qam-switch" }, h("i"))), true),
      row("Resolution", "What games see", h("button", { className: "DialogButton gd_Button" }, "1280 × 800 ▾")),
      row("Night mode", "Warmer colours in the evening", h("div", { className: "qam-toggle" }, h("span", { className: "qam-switch" }, h("i")))),
      h("div", { className: "dlg-btns" }, h("button", { className: "DialogButton gd_Button" }, "Reset"), h("button", { className: "DialogButton gd_Button" }, "Done"))));
}
function Main() {
  const [, bump] = useState(0);
  useEffect(() => { const t = setInterval(() => bump((n) => n + 1), 250); return () => clearInterval(t); }, []);
  if (scene === "library") return h(Library);
  if (scene === "game") return h(GamePage);
  if (scene === "dialog") return h(Dialog);
  const patch = window.HC_PATCHES.get("/library/home");
  const route = { children: h(SteamHome) };
  return patch ? patch(route).children : route.children;
}
const qam = window.HC_QAM !== false;
createRoot(document.getElementById("root")).render(
  h("div", { className: "frame" },
    h("div", { className: "main" }, h(Main)),
    qam ? h("div", { className: "qam" },
      h("div", { className: "qam-tabs" }, h("div", { className: "qam-tab" }, "⚙"), h("div", { className: "qam-tab on" }, p.icon), h("div", { className: "qam-tab" }, "⟳")),
      h("div", { className: "qam-body" }, h("div", { className: "qam-title" }, p.titleView), h("div", { className: "ScrollPanel" }, p.content))) : null,
    h("div", { className: "steam-footer" }, "A Select   B Back   ☰ Menu")));
