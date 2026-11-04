// CI only (render.mjs): Hearth Eras' real plugin (src/index.tsx) started the way Decky starts it, over a stand-in of
// Steam's Game Mode (Steam's Home, a Library page with tabs, a game page, the keyboard, Quick Access). Steam's Home is
// drawn through the plugin's own route patch (what routerHook.addPatch("/library/home") got), so the home shots are the
// real path. SteamClient.Input stands in for Steam's controller state (window.HE_PAD sends button states).
import { createRoot } from "react-dom/client";
import { createElement as h, useEffect, useState } from "react";
import plugin from "../../src/index.tsx";
import { appIdOf } from "../../../decky-hearth-music/src/steam.ts";

window.HE_MUSIC_APPID = appIdOf;
window.SteamClient = { Input: { RegisterForControllerStateChanges(cb) { window.HE_PAD = cb; return { unregister() { window.HE_PAD = null; } }; } } };
const p = plugin();
window.HE_PLUGIN = p;
const scene = window.HE_SCENE || "home";
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
        h("div", { className: "cap cap-grey", style: { background: c } }))))),
    window.HE_KEYBOARD ? h("div", { className: "vk_Container keyboard" }, ...["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"].map((r, n) =>
      h("div", { key: r, className: "krow" }, ...r.split("").map((k, i) => h("div", { key: k, className: "key" + (n === 1 && i === 3 ? " gpfocus" : "") }, k))))) : null,
    window.HE_SPINNER ? h("div", { className: "throbber spinner" }) : null);
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
function Main() {
  const [, bump] = useState(0);
  useEffect(() => { const t = setInterval(() => bump((n) => n + 1), 250); return () => clearInterval(t); }, []);
  if (scene === "library") return h(Library);
  if (scene === "game") return h(GamePage);
  const patch = window.HE_PATCHES.get("/library/home");
  const route = { children: h(SteamHome) };
  return patch ? patch(route).children : route.children;
}
const qam = window.HE_QAM !== false;
createRoot(document.getElementById("root")).render(
  h("div", { className: "frame" },
    h("div", { className: "main" }, h(Main)),
    qam ? h("div", { className: "qam" },
      h("div", { className: "qam-tabs" }, h("div", { className: "qam-tab" }, "⚙"), h("div", { className: "qam-tab on" }, p.icon), h("div", { className: "qam-tab" }, "⟳")),
      h("div", { className: "qam-body" }, h("div", { className: "qam-title" }, p.titleView), h("div", { className: "ScrollPanel" }, p.content))) : null,
    h("div", { className: "steam-footer" }, "A Select   B Back   ☰ Menu")));
