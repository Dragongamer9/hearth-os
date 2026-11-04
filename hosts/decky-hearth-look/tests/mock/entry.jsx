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
function Menu() {
  const running = !!window.HC_RUNNING;
  const items = ["Home", "Library", "Store", "Media", "Downloads", "Friends", "Settings", "Power"];
  return h("div", { className: "mm_Anchor" }, h("div", { className: "menu-panel" },
    running ? h("div", { className: "mm_AppColumn" }, h("div", { className: "mm_GameBg" }), h("div", { className: "mm_Logo game-logo" }, "Ember Knights"),
      h("div", { className: "mm_Controls" }, h("div", { className: "mm_Item" }, "Resume"), h("div", { className: "mm_Item" }, "Exit game"))) : null,
    h("div", { className: "mm_NavColumn" }, h("div", { className: "mm_NavBox" },
      ...items.map((t, i) => h("div", { key: t, className: "mm_Item" + (i === 1 ? " gpfocus" : "") + (i === 0 ? " mm_Active" : "") }, t))))));
}
function Main() {
  const [, bump] = useState(0);
  useEffect(() => { const t = setInterval(() => bump((n) => n + 1), 250); return () => clearInterval(t); }, []);
  if (scene === "library") return h(Library);
  if (scene === "game") return h(GamePage);
  const patch = window.HC_PATCHES.get("/library/home");
  const route = { children: h(SteamHome) };
  return patch ? patch(route).children : route.children;
}
const qam = window.HC_QAM !== false;
createRoot(document.getElementById("root")).render(
  h("div", { className: "frame" },
    h("div", { className: "main" }, h(Main)),
    window.HC_MENU ? h(Menu) : null,
    qam ? h("div", { className: "qam" },
      h("div", { className: "qam-tabs" }, h("div", { className: "qam-tab" }, "⚙"), h("div", { className: "qam-tab on" }, p.icon), h("div", { className: "qam-tab" }, "⟳")),
      h("div", { className: "qam-body" }, h("div", { className: "qam-title" }, p.titleView), h("div", { className: "ScrollPanel" }, p.content))) : null,
    h("div", { className: "steam-footer" }, "A Select   B Back   ☰ Menu")));
