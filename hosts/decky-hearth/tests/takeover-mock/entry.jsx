// CI only (render-takeover.mjs): draws the takeover's real pages (src/pages.tsx) with the stand-ins for Decky, and
// Hearth's parts of Steam's main menu, Quick Access and in-game section (src/patches.tsx, qam.tsx, ingame.tsx) over
// static stand-ins for Steam's own menu and Quick Access frame.
import { createRoot } from "react-dom/client";
import { FaBell, FaCog, FaFire, FaGamepad, FaPlug, FaTachometerAlt, FaUserFriends } from "react-icons/fa";
import { HearthHome, LibraryPage, StorePage, setBrand } from "../../src/pages";
import { HearthRowView } from "../../src/row";
import { HearthMenuItems } from "../../src/patches";
import { QamPanel } from "../../src/qam";
import { NowPlayingPage } from "../../src/ingame";
import { ModsPage, RecompsPage, StorageManagePage } from "../../src/manage";
import { startFront } from "../../src/core";
import { setShortcutMap } from "../../src/state";
import { rowGames } from "./stub-api.js";

setBrand({ css: "", logo: window.HEARTH_LOGO, wallpaper: window.HEARTH_WALL });
// A game of Hearth's running through its Steam shortcut (Decky's Router.MainRunningApp, stub-ui.jsx).
setShortcutMap({ "epic:alan-wake-2": 3000000001 });
if (window.HEARTH_RUNNING) startFront();
const steamHome = <div style={{ padding: 60, color: "#fff" }}>Steam's own Home (shown if Hearth's can't be)</div>;
// The older Hearth row over Steam's Home (Hearth's home screen off), over a stand-in for Steam's Home.
const row = <HearthRowView games={rowGames} brand={{ logo: window.HEARTH_LOGO, wallpaper: window.HEARTH_WALL }} onPlay={() => {}}>{steamHome}</HearthRowView>;

// A game behind Steam's menus (a gradient standing in for it).
const game = <div style={{ position: "absolute", inset: 0, background: "linear-gradient(135deg,#1b2a3a,#3a1c2c 60%,#0c1016)", filter: "brightness(.55)" }}>
  <div style={{ position: "absolute", left: 60, bottom: 50, color: "rgba(255,255,255,.25)", fontSize: 64, fontWeight: 800 }}>ALAN WAKE 2</div></div>;

// Steam's main menu (the Steam / Xbox button), static: Steam's own entries, then Hearth's part.
const steamMenu = (
  <div style={{ position: "absolute", inset: 0 }}>
    {game}
    <div className="steam-menu">
      <div className="steam-menu-logo">STEAM</div>
      {["Home", "Library", "Store", "Media", "Downloads", "Friends & Chat", "Settings", "Power"].map((t) => <div key={t} className="steam-menu-entry">{t}</div>)}
      <HearthMenuItems />
    </div>
  </div>
);

// Steam's Quick Access (the … button), static: its tab icons (Steam's, Decky's plug, Hearth's three) and Hearth's panel.
function Qam(props) {
  const icons = [[FaBell], [FaUserFriends], [FaCog], [FaTachometerAlt], [FaGamepad], [FaPlug], [FaFire, "hearth"], [FaTachometerAlt, "perf"], [FaGamepad, "controller"]];
  const title = { hearth: "Hearth", perf: "Hearth Performance", controller: "Hearth Controller" }[props.which];
  return (
    <div style={{ position: "absolute", inset: 0 }}>
      {game}
      <div className="qam">
        <div className="qam-tabs">{icons.map(([I, k], i) => <div key={i} className={"qam-tab" + (k ? " hearth" : "") + (k === props.which ? " on" : "")}><I /></div>)}</div>
        <div className="qam-body"><div className="qam-title">{title}</div><QamPanel which={props.which} /></div>
      </div>
    </div>
  );
}

const pages = {
  home: <HearthHome steam={steamHome} />, store: <StorePage />, library: <LibraryPage />, row,
  "steam-menu": steamMenu, "qam-hearth": <Qam which="hearth" />, "qam-perf": <Qam which="perf" />, "qam-controller": <Qam which="controller" />,
  ingame: <div style={{ position: "absolute", inset: 0 }}><NowPlayingPage /></div>,
  mods: <ModsPage />, storage: <StorageManagePage />, recomps: <RecompsPage />,
};
createRoot(document.getElementById("root")).render(pages[window.HEARTH_PAGE] || steamHome);
