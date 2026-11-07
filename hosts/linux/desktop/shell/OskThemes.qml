pragma Singleton

// The looks of Mantel's on-screen keyboard (OskBoard.qml, OskKey.qml): only colours, corners and lettering, so typing
// with the controller and touch is the same in every one. Picked in Settings › Keyboard (SetKeyboard.qml) and kept as
// "deskTheme" in ~/.config/nimbus/osk.json (the shared keyboard's settings file; the engine, osk/hearth-osk.js, keeps
// its own "theme" for Hearth's apps and ignores this key). Without a pick: Steam-style, or the keyboard's older
// look when one was chosen before (Ember glow, Slate, Daylight).
//   steam     Steam-style: Mantel's Steam look, with your accent, dark or light as Settings › Appearance says (default)
//   warm      Hearth warm: firelit browns and an ember glow, dark or light as Appearance says
//   contrast  High contrast: black and white with outlined keys and a yellow (light mode: blue) selection
//   frost     Frosted: see-through glass panels, thin outlines, your accent
//   pocket    Pocket: a handheld skin, round keys and bigger, bolder letters for thumbs
//   hearth, ember, slate, daylight   the keyboard's first looks (dark only, Daylight light)
// A look's fields: bg bg2 key mod text muted sel selText accent ink, and optionally kr (key corner, -1 = round),
// br (board corner), kb / kbc (key outline width / colour), bbc (board outline), fw (letter weight), fs (letter size
// factor), glow (a warm glow from below), alpha (the board's opacity).
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root
    property string chosen: ""           // osk.json's deskTheme

    readonly property var list: [
        { id: "steam", label: "Steam-style", sub: "Follows Appearance: your accent, dark or light" },
        { id: "warm", label: "Hearth warm", sub: "Firelit browns and an ember glow; dark or light" },
        { id: "contrast", label: "High contrast", sub: "Bold outlines, easiest to read" },
        { id: "frost", label: "Frosted", sub: "See-through glass, thin outlines" },
        { id: "pocket", label: "Pocket", sub: "Round keys, big letters for thumbs" },
        { id: "hearth", label: "Classic dark", sub: "The keyboard's first look" },
        { id: "ember", label: "Ember glow", sub: "Dark, with a warm glow" },
        { id: "slate", label: "Slate", sub: "Dark blue-grey" },
        { id: "daylight", label: "Daylight", sub: "Light" }
    ]

    function pal(look) { return ((Theme.palettes || {})[look] || {})[Theme.mode] || {}; }
    function a(c, al) { const q = Qt.color(c); return Qt.rgba(q.r, q.g, q.b, al); }
    function tint(base, c, al) { return Qt.tint(base, a(c, al)); }

    readonly property var themes: {
        const dark = Theme.dark;
        const st = pal("steam"), hw = pal("hearth");
        const acc = Theme.accent, ink = Theme.accentInk;
        const warmAcc = Theme.look === "hearth" ? Theme.accent : (hw.ember || "#ff8a3d");
        const stKey = st.panel2 || "#24303f";
        const hwKey = hw.panel2 || "#261d18";
        return {
            steam: { bg: dark ? (st.bg || "#171a21") : (st.bg2 || "#dbe2e9"), bg2: dark ? (st.bg2 || "#1b2838") : (st.panel3 || "#d2dce6"),
                     key: dark ? stKey : (st.panel || "#f4f7fa"), mod: dark ? (st.panel || "#1e2633") : (st.panel3 || "#d2dce6"),
                     text: st.text || "#dce5ed", muted: st.muted || "#8f98a0", sel: tint(dark ? stKey : "#ffffff", acc, dark ? 0.45 : 0.3),
                     selText: st.text || "#dce5ed", accent: acc, ink: ink, kr: 4, br: 6, bbc: dark ? a("#ffffff", 0.06) : a("#000000", 0.08), fw: Font.Medium },
            warm: { bg: hw.bg || "#0f0b09", bg2: hw.bg2 || "#17110e", key: hwKey, mod: hw.panel || "#1c1512", text: hw.text || "#f4ece3",
                    muted: hw.muted || "#a69484", sel: tint(hwKey, warmAcc, dark ? 0.5 : 0.35), selText: hw.text || "#f4ece3",
                    accent: warmAcc, ink: hw.ink || "#2a1206", kr: 12, br: 22, glow: dark ? "#3a1a0c" : "#f3d2b4", bbc: a(warmAcc, 0.18) },
            contrast: dark
                ? { bg: "#000000", bg2: "#141414", key: "#000000", mod: "#000000", text: "#ffffff", muted: "#e6e6e6", sel: "#ffd400", selText: "#000000",
                    accent: "#ffd400", ink: "#000000", kr: 6, br: 8, kb: 2, kbc: "#ffffff", mbc: "#9a9a9a", bbc: "#ffffff", fw: Font.Bold, fs: 1.08 }
                : { bg: "#ffffff", bg2: "#ededed", key: "#ffffff", mod: "#ffffff", text: "#000000", muted: "#1a1a1a", sel: "#0047d6", selText: "#ffffff",
                    accent: "#0047d6", ink: "#ffffff", kr: 6, br: 8, kb: 2, kbc: "#000000", mbc: "#5a5a5a", bbc: "#000000", fw: Font.Bold, fs: 1.08 },
            frost: { bg: dark ? a(Theme.panel, 0.55) : a("#ffffff", 0.55), bg2: dark ? a("#ffffff", 0.08) : a("#ffffff", 0.6),
                     key: dark ? a("#ffffff", 0.09) : a("#ffffff", 0.7), mod: dark ? a("#ffffff", 0.04) : a("#ffffff", 0.4),
                     text: Theme.text, muted: Theme.muted, sel: a(acc, 0.9), selText: ink, accent: acc, ink: ink,
                     kr: 14, br: 24, kb: 1, kbc: dark ? a("#ffffff", 0.14) : a("#000000", 0.08), bbc: dark ? a("#ffffff", 0.18) : a("#ffffff", 0.8), fw: Font.Normal },
            pocket: { bg: dark ? "#0d0f14" : "#eef1f5", bg2: dark ? "#171a22" : "#e1e6ed", key: dark ? "#1d212b" : "#ffffff", mod: dark ? "#14171e" : "#d7dde6",
                      text: dark ? "#f2f5f9" : "#121620", muted: dark ? "#8e97a6" : "#5d6676", sel: acc, selText: ink, accent: acc, ink: ink,
                      kr: -1, br: 28, fw: Font.Bold, fs: 1.18, bbc: "transparent" },
            hearth: { bg: "#17171c", bg2: "#1f1f25", key: "#2b2b32", mod: "#222228", text: "#f3efe8", muted: "#a7a3ad", sel: "#f6f2ea", selText: "#141217", accent: acc, ink: ink },
            ember: { bg: "#120c0a", bg2: "#1d1310", key: "#2a1a14", mod: "#1f1410", text: "#ffeadb", muted: "#c9a18a", sel: "#ffd9bd", selText: "#2a1206", accent: "#ff7a2f", ink: "#1d0b02", glow: "#3a1a0c" },
            slate: { bg: "#1b2028", bg2: "#222833", key: "#2d3542", mod: "#252c37", text: "#eef2f7", muted: "#9aa6b6", sel: "#f2f6fb", selText: "#111722", accent: acc, ink: ink },
            daylight: { bg: "#e9e6e1", bg2: "#dedad3", key: "#ffffff", mod: "#d4d0c8", text: "#1d1b20", muted: "#6b6670", sel: "#1d1b20", selText: "#ffffff", accent: acc, ink: ink }
        };
    }
    // The look in use: the pick from Settings › Keyboard; else the engine's older pick (not its default, "hearth");
    // else Steam-style.
    function resolve(engineTheme) {
        if (chosen !== "" && themes[chosen]) return chosen;
        if (engineTheme && engineTheme !== "hearth" && themes[engineTheme]) return engineTheme;
        return "steam";
    }

    FileView {
        path: (Quickshell.env("XDG_CONFIG_HOME") || (Quickshell.env("HOME") + "/.config")) + "/nimbus/osk.json"
        watchChanges: true
        // Not there yet is normal (nothing saved yet): only real read errors are printed.
        printErrors: false
        onFileChanged: reload()
        onLoaded: { try { const o = JSON.parse(text()); root.chosen = o && typeof o.deskTheme === "string" ? o.deskTheme : ""; } catch (e) { root.chosen = ""; } }
        onLoadFailed: error => { if (typeof FileViewError === "undefined" || error !== FileViewError.FileNotFound) console.warn("OskThemes: couldn't read " + path + ": " + error); root.chosen = ""; }
    }
}
