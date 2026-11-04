// CI only (render.mjs): Hearth Classic's real plugin (src/index.tsx) started the way Decky starts it, over a stand-in
// of Steam's Game Mode (a Library page with tabs, Steam's Home, the main menu and Quick Access). Steam's Home is drawn
// through the plugin's own route patch (what routerHook.addPatch("/library/home") got), so the home shot is the real path.
import { createRoot } from "react-dom/client";
import { createElement as h, useEffect, useState } from "react";
import plugin from "../../src/index.tsx";

const p = plugin();
window.HC_PLUGIN = p;
const scene = window.HC_SCENE || "home";

function SteamHome() {
  return h("div", { className: "steam-home" }, h("div", { className: "steam-home-title" }, "Steam's own Home"),
    h("div", { className: "grid" }, ...["#5c677d", "#7d8597", "#33415c", "#979dac", "#5c677d"].map((c, i) => h("div", { key: i, className: "cap", style: { background: c } }))));
}
function Main() {
  const [, bump] = useState(0);
  useEffect(() => { const t = setInterval(() => bump((n) => n + 1), 250); return () => clearInterval(t); }, []);
  if (scene === "library") {
    return h("div", { className: "gtp_Page steam-page" },
      h("div", { className: "gtp_Header" }, ...["Great on Deck", "All games", "Installed", "Collections", "Non-Steam"].map((t, i) => h("div", { key: t, className: "gtp_Tab" + (i === 1 ? " gtp_Selected" : "") }, t))),
      h("div", { className: "grid" }, ...["#5c677d", "#7d8597", "#33415c", "#979dac", "#5c677d", "#33415c", "#7d8597"].map((c, i) => h("div", { key: i, className: "cap" + (i === 1 ? " gpfocus" : ""), style: { background: c } }))));
  }
  const patch = window.HC_PATCHES.get("/library/home");
  const route = { children: h(SteamHome) };
  return patch ? patch(route).children : route.children;
}
const qam = window.HC_QAM !== false;
createRoot(document.getElementById("root")).render(
  h("div", { className: "frame" },
    h("div", { className: "main" }, h(Main)),
    scene === "library" ? h("div", { className: "mainmenu" }, h("div", { className: "mainmenu-logo" }, h("span", null, "STEAM")),
      ...["Home", "Library", "Store", "Media", "Downloads", "Friends", "Settings", "Power"].map((t, i) => h("div", { key: t, className: "menu-entry" + (i === 1 ? " gpfocus" : "") }, t))) : null,
    qam ? h("div", { className: "qam" },
      h("div", { className: "qam-tabs" }, h("div", { className: "qam-tab" }, "⚙"), h("div", { className: "qam-tab on" }, p.icon), h("div", { className: "qam-tab" }, "⟳")),
      h("div", { className: "qam-body" }, h("div", { className: "qam-title" }, p.titleView), h("div", { className: "ScrollPanel" }, p.content))) : null,
    h("div", { className: "steam-footer" }, "A Select   B Back   ☰ Menu")));
