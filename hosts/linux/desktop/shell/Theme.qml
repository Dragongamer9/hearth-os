pragma Singleton

// Mantel's palette. Settings › Appearance picks the look (Settings.look: "hearth" = warm, firelit darks with an
// ember accent; "steam" = Steam-style deep blue-slate, flat panels, square-ish corners, a bright blue accent),
// dark or light (Settings.mode) and the accent (Settings.accent: a colour, or "" for the look's own; with the
// Hearth look your profile's accent from ~/.local/share/nimbus/library.json is used then, as before).
// The colours come from ../theme/palettes.json (shared with theme/hearth-theme, which gives labwc's window
// decorations and Qt / GTK / KDE apps the same look); the Hearth dark values below are used until it's read.
import QtQuick
import Quickshell
import Quickshell.Io

Singleton {
    id: root
    property var palettes: ({})
    readonly property string look: palettes[Settings.look] ? Settings.look : "steam"
    // Mode "Sunset" (off unless you turn it on, Settings.sunset): light from Settings.sunsetLight to sunsetDark, dark
    // the rest of the day; Settings.mode itself is never changed (theme/hearth-theme works it out the same way).
    SystemClock { id: sunClock; precision: SystemClock.Minutes }
    function minutesOf(t, d) { const m = /^([01]?\d|2[0-3]):([0-5]\d)$/.exec(t || ""); return m ? parseInt(m[1]) * 60 + parseInt(m[2]) : d; }
    readonly property string sunsetMode: {
        if (!Settings.sunset) return "";
        const now = sunClock.date, m = now.getHours() * 60 + now.getMinutes();
        const a = minutesOf(Settings.sunsetLight, 420), b = minutesOf(Settings.sunsetDark, 1170);
        return (a <= b ? (m >= a && m < b) : (m >= a || m < b)) ? "light" : "dark";
    }
    readonly property string mode: sunsetMode !== "" ? sunsetMode : Settings.mode === "light" ? "light" : "dark"
    readonly property bool dark: mode === "dark"
    readonly property bool steam: look === "steam"
    readonly property var lookDef: palettes[look] || ({})
    readonly property var p: lookDef[mode] || ({})
    function c(k, d) { return p[k] || d; }
    // A colour as a CSS string, for Canvas drawing ("rgba(r,g,b,a)"; alpha from the colour unless given).
    function css(col, a) { return "rgba(" + Math.round(col.r * 255) + "," + Math.round(col.g * 255) + "," + Math.round(col.b * 255) + "," + (a === undefined ? col.a : a) + ")"; }

    readonly property color bg: c("bg", "#0f0b09")
    readonly property color bg2: c("bg2", "#17110e")
    readonly property color panel: c("panel", "#1c1512")
    readonly property color panel2: c("panel2", "#261d18")
    readonly property color panel3: c("panel3", "#33271f")
    readonly property color line: c("line", "#3b2e26")
    readonly property color text: c("text", "#f4ece3")
    readonly property color muted: c("muted", "#a69484")
    readonly property color danger: c("danger", "#ff5d5d")
    readonly property color ember: c("ember", "#ff8a3d")       // the look's own accent
    readonly property color flame: c("flame", "#ffd166")       // its second, brighter highlight
    property string profileAccent: ""
    readonly property bool validAccent: /^#[0-9a-fA-F]{6}$/.test(Settings.accent || "")
    // New installs (desktop.json made with "defaults": 2, from /etc/skel) start Steam-style with the Sky accent; everyone
    // else keeps the look's own. Only a default, filled in when read: the accent you see is never written for you.
    readonly property bool skyDefault: steam && Settings.defaultsGen >= 2
    readonly property color lookAccent: skyDefault ? "#66c0f4" : ember
    readonly property string lookAccentName: skyDefault ? "Sky" : steam ? "Steam blue" : look === "hearth" && profileAccent ? "Your profile's" : "The look's own"
    readonly property color accent: validAccent ? Settings.accent : (look === "hearth" && profileAccent) ? profileAccent : lookAccent
    // Text on an accent background: dark on light accents, white on dark ones.
    readonly property color accentInk: (accent.r * 0.2126 + accent.g * 0.7152 + accent.b * 0.0722) > 0.45 ? ((lookDef.dark || {}).ink || "#2a1206") : "#ffffff"
    // Frosted panels over the wallpaper (Steam-style: nearly solid, flat).
    // Settings › Appearance › Panel transparency: the look's own, or solid / frosted / clear for every look.
    readonly property real glassAlpha: Settings.panelGlass === "solid" ? 0.985 : Settings.panelGlass === "frosted" ? 0.9
        : Settings.panelGlass === "clear" ? 0.62 : (p.glass || 0.92)
    readonly property color glass: Qt.rgba(panel.r, panel.g, panel.b, glassAlpha)

    // ---- Mantel 2026: how the look draws the bar, dock and islands (theme/palettes.json style tokens; docs/MANTEL.md).
    // Your own Settings › Appearance › Bar and dock choices win over the look's.
    readonly property var looks: Object.keys(palettes).filter(k => !k.startsWith("_") && palettes[k] && palettes[k].dark)
        .sort((a, b) => (palettes[a].order || 99) - (palettes[b].order || 99))
    readonly property string lookIslands: ["floating", "bar", "shelf"].indexOf(lookDef.islandStyle) >= 0 ? lookDef.islandStyle : (steam ? "bar" : "floating")
    readonly property string islandStyle: Settings.barStyle === "floating" || Settings.barStyle === "bar" ? Settings.barStyle : lookIslands
    readonly property bool thin: islandStyle === "bar" && lookDef.thin === true && Settings.barStyle !== "bar"
    readonly property string frame: ["plain", "chamfer", "neon", "wood", "frost"].indexOf(lookDef.frame) >= 0 ? lookDef.frame : "plain"
    readonly property bool dockAutoHide: Settings.dockMode === "autohide" || (Settings.dockMode !== "shown" && lookDef.dockAutoHide === true)
    readonly property bool deskClock: Settings.deskClock === "on" || (Settings.deskClock !== "off" && lookDef.deskClock === true)
    readonly property bool perfIsland: lookDef.perfIsland === true
    readonly property bool edgeGlow: lookDef.edgeGlow === true
    readonly property bool scanlines: lookDef.scanlines === true
    readonly property bool hints: lookDef.hints === true
    readonly property string lookLabel: lookDef.label || look

    // ---- Bar layout (Settings › Appearance › Bar and dock; docs/MANTEL.md). Nothing set = today's Mantel 2026:
    // the top bar and the floating dock. "One main bar" puts everything in one bar on the edge you pick; with the
    // floating app menu off, the app menu button and your apps move into the main bar and there's no dock.
    readonly property bool oneBar: Settings.barLayout === "one"
    readonly property string barEdge: oneBar ? (["bottom", "top", "left", "right"].indexOf(Settings.barEdge) >= 0 ? Settings.barEdge : "bottom") : "top"
    readonly property bool barVertical: barEdge === "left" || barEdge === "right"
    readonly property bool dockFloats: Settings.dockFloat !== "off"
    // Auto-hide hides the dock (as before) and, in One main bar or when the apps are in the bar, the main bar too.
    readonly property bool barAutoHide: dockAutoHide && (oneBar || !dockFloats)
    // The main bar's style: the look's (floating islands, one full-width bar); Console's big status row only sits over
    // its shelf of tiles, so a bar that holds the apps or isn't at the top is a full-width bar there.
    readonly property string mainStyle: islandStyle === "shelf" && (oneBar || !dockFloats) ? "bar" : islandStyle
    // When this changes, shell.qml builds the bar and the dock again (so nothing keeps space reserved on the old edge).
    readonly property string barKey: barEdge + "/" + (dockFloats ? "dock" : "nodock")

    // ---- App menu (Settings › Appearance › Bar and dock › App menu): Hearth (the centred launcher), Centred,
    // Tiles, Start screen or Classic. Not chosen: Hearth, except Start screen on a handheld that was installed with
    // a Hearth OS that has the five menus (/etc/nimbus/installed.json "appMenus", written by the installer). The
    // default is only worked out here, when read; it's never written, so existing devices keep the Hearth menu.
    readonly property var appMenus: ["hearth", "centred", "tiles", "start", "classic"]
    property bool handheld: false            // a handheld PC, by name (as hearth-link's device_kind)
    property bool installedWithMenus: false
    readonly property string appMenuDefault: handheld && installedWithMenus ? "start" : "hearth"
    readonly property string appMenu: appMenus.indexOf(Settings.appMenu) >= 0 ? Settings.appMenu : appMenuDefault
    readonly property var handheldNames: ["jupiter", "galileo", "rog ally", "rc71l", "rc72l", "legion go", "83e1", "claw", "ayaneo", "onexplayer", "gpd win", "zotac gaming zone"]
    property string dmiProduct: ""
    function checkHandheld() {
        const k = Quickshell.env("HEARTH_LINK_KIND") || "";
        if (["handheld", "laptop", "desktop", "tablet", "tv"].indexOf(k) >= 0) { handheld = k === "handheld"; return; }
        const p = dmiProduct.toLowerCase();
        handheld = handheldNames.some(h => p.indexOf(h) >= 0);
    }
    Process {
        command: ["sh", "-c", "cat /sys/class/dmi/id/sys_vendor /sys/class/dmi/id/product_name /sys/class/dmi/id/board_name 2>/dev/null | tr '\\n' ' '"]
        running: true
        stdout: StdioCollector { onStreamFinished: { root.dmiProduct = text; root.checkHandheld(); } }
    }
    FileView {
        path: "/etc/nimbus/installed.json"
        blockLoading: true
        onLoaded: { try { root.installedWithMenus = (JSON.parse(text()).appMenus || 0) >= 1; } catch (e) { root.installedWithMenus = false; } }
    }
    // ---- Right-click menu (Settings › Personalization › Bar and dock › Right-click menu; PopMenu.qml, Hearth Files):
    // "today" (the 268 px menu, the default), "windows" (Windows 11 style), "compact" (Compact list) or "tiles" (Tiles
    // and groups). Not chosen = today's; only read here, never written for you.
    readonly property var menuStyles: ["today", "windows", "compact", "tiles"]
    readonly property string menuStyle: menuStyles.indexOf(Settings.menuStyle) >= 0 ? Settings.menuStyle : "today"
    // The wallpaper on show: yours, or the look's own ("" = Hearth's fireside).
    readonly property string wall: Settings.wallpaper !== "" ? Settings.wallpaper : (lookDef.wallpaper || "")
    // A controller focus ring: crisp, in the accent.
    readonly property int focusWidth: 2

    // Slow, calm motion everywhere.
    readonly property int quick: 140
    readonly property int calm: 260

    readonly property string font: lookDef.font || "sans-serif"
    readonly property int radius: r(14)
    // A corner radius in this look: Hearth's soft corners as they are, Steam-style ones capped at 2–4 px.
    function r(n) { const cap = lookDef.corner || 0; return cap > 0 ? Math.min(n, cap) : n; }

    FileView {
        // (Qt.resolvedUrl can't reach outside the shell's folder: Quickshell answers qrc:/qs-blackhole there.)
        path: Quickshell.shellDir + "/../theme/palettes.json"
        blockLoading: true
        onLoaded: { try { root.palettes = JSON.parse(text()); } catch (e) { console.warn("Theme: palettes.json: " + e); } }
    }

    // Apps and window decorations follow (theme/apply.sh runs theme/hearth-theme, which rewrites the Qt / GTK / KDE
    // colours and labwc's theme and asks labwc to reload). Once things settle, not on every step of a change.
    readonly property string key: look + "/" + mode + "/" + accent
    property string appliedKey: ""
    onKeyChanged: if (Settings.loaded) applyTimer.restart()
    Timer {
        id: applyTimer
        interval: 600
        onTriggered: {
            if (root.appliedKey === "" ) { root.appliedKey = root.key; return; }   // the first settle: labwc's autostart already ran it
            if (root.appliedKey === root.key) return;
            root.appliedKey = root.key;
            Quickshell.execDetached(["sh", Quickshell.shellDir + "/../theme/apply.sh"]);
        }
    }
    Connections {
        target: Settings
        function onLoadedChanged() { if (Settings.loaded) applyTimer.restart(); }
    }

    FileView {
        path: Quickshell.env("HOME") + "/.local/share/nimbus/library.json"
        watchChanges: true
        onFileChanged: reload()
        onLoaded: {
            try {
                const a = JSON.parse(text()).profile?.accent;
                if (/^#[0-9a-fA-F]{6}$/.test(a || "")) root.profileAccent = a;
            } catch (e) {}
        }
    }
}
