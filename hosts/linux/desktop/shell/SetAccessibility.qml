// Hearth Settings › Accessibility: the magnifier (labwc's own: Super+= / Super+- / Super+Esc), colour filters,
// the screen reader (Orca, Super+Alt+S), and bigger text, high contrast and a bigger pointer. Saved by hearth-access
// in ~/.config/nimbus/accessibility.json, which Game Mode's Settings › Accessibility shares (its zoom and colour
// filters are gamescope's; Desktop Mode can only invert colours, see hearth-access). With Mantel on KWin the same
// settings go through KWin's effects (hearth-access's kwin mode: zoom / magnifier, invert, colour-blindness correction,
// greyscale on KWin 6.6), and KWin's own extras are here too, in their one home (kwinrc, kaccessrc through
// hearth-kwin-settings): how the zoom follows, sticky / bounce / slow keys, mouse keys and the visual bell. Under labwc
// those show greyed, as needing Mantel on KWin.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Accessibility"
    property var st: null                // hearth-access status
    readonly property var s: st ? st.settings : ({})
    readonly property var av: st ? st.available : ({})
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    Component.onCompleted: job.run(["status"], r => page.st = r, e => page.say(e, true))
    SetJob { id: job; helper: "hearth-access" }
    KwinSettings { id: kw }
    readonly property bool onKwin: !!page.av.kwin
    readonly property string kwinNote: kw.live ? "" : ". Needs Mantel on KWin"
    // KWin's version as numbers (6.3 → 603), for the parts only newer KWins have.
    readonly property int kwinVer: { const m = String((kw.st && kw.st.version) || "").match(/^(\d+)\.(\d+)/); return m ? parseInt(m[1]) * 100 + parseInt(m[2]) : 0; }
    function needs(ver, label) { return kw.live && kwinVer > 0 && kwinVer < ver ? ". Needs KWin " + Math.floor(ver / 100) + "." + (ver % 100) + " (this is " + kw.st.version + ")" : page.kwinNote; }
    function can(ver) { return kw.live && (kwinVer === 0 || kwinVer >= ver); }

    // Changes one or more settings: shown at once, saved (and applied) by hearth-access.
    property var pending: ({})
    function change(o, now) {
        const next = Object.assign({}, page.s, o);
        page.st = Object.assign({}, page.st, { settings: next });
        page.pending = Object.assign({}, page.pending, o);
        if (now) save.triggered(); else save.restart();
    }
    Timer {
        id: save
        interval: 500
        onTriggered: {
            const o = page.pending;
            page.pending = ({});
            if (!Object.keys(o).length) return;
            job.run(["set", JSON.stringify(o)], r => {
                page.st = r;
                const p = r.problems || {};
                const keys = Object.keys(p);
                page.say(keys.length ? p[keys[0]] : "", keys.length > 0);
            }, e => page.say(e, true), "", "save");
        }
    }
    readonly property var filterNames: ({ none: "Off", grayscale: "Greyscale", invert: "Inverted colours",
        "invert-brightness": "Inverted brightness", protanopia: "Red-green (protanopia)",
        deuteranopia: "Green-red (deuteranopia)", tritanopia: "Blue-yellow (tritanopia)" })
    readonly property var filterSubs: ({ grayscale: "Shows everything in shades of grey", invert: "Light becomes dark and colours flip",
        "invert-brightness": "Light becomes dark; colours stay much the same",
        protanopia: "Moves reds where they're easier to tell apart", deuteranopia: "Moves greens where they're easier to tell apart",
        tritanopia: "Moves blues and yellows where they're easier to tell apart" })

    // ------------------------------------------------ magnifier
    SetHeader { text: "Magnifier" + (job.what === "save" ? " · saving…" : "") }
    Text {
        visible: !!page.st && !page.onKwin && !page.av.labwcMagnifier
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "This labwc (" + (page.av.labwc || "unknown version") + ") has no magnifier; it came in labwc 0.7.3."
        color: Theme.danger; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    SetRow {
        icon: "search"; title: "Zoom in now"
        sub: page.onKwin ? "Super+= zooms in, Super+- zooms out, Super+0 goes back to normal (KWin's zoom)"
                         : "Super+= zooms in, Super+- zooms out (and off), Super+Esc turns it off or back on. The view follows the pointer"
        chevron: false
        onActivated: job.run(["magnifier", "in"], () => {}, e => page.say(e, true))
    }
    SetChoice {
        title: "What it magnifies"
        options: [{ value: "full", label: "The whole screen" }, { value: "lens", label: "A box around the pointer" }]
        current: page.s.magnifierSize || "full"
        onChosen: v => page.change({ magnifierSize: v })
    }
    SetSlider {
        icon: "search"; title: "Zoom when Super+Esc turns it on"
        readonly property real lo: 1.25
        readonly property real hi: 8
        value: ((page.s.magnifierZoom || 2) - lo) / (hi - lo)
        step: 0.25 / (hi - lo)
        valueText: (page.s.magnifierZoom || 2).toFixed(2).replace(/0$/, "") + "×"
        onMoved: v => page.change({ magnifierZoom: Math.round((lo + v * (hi - lo)) * 4) / 4 })
    }
    SetChoice {
        enabled: kw.live; opacity: kw.live ? 1 : 0.45
        icon: "mouse"; title: "The zoomed view follows the pointer"
        sub: "How the view moves when the pointer nears its edge" + page.kwinNote
        options: [{ value: 0, label: "Smoothly" }, { value: 1, label: "Pointer in the middle" }, { value: 2, label: "Pushed at the edge" }, { value: 3, label: "Stays put" }]
        current: kw.n("kwinrc", "Effect-zoom", "MouseTracking", 0)
        onChosen: v => kw.set("kwinrc", "Effect-zoom", "MouseTracking", v)
    }
    SetSwitch {
        enabled: kw.live; opacity: kw.live ? 1 : 0.45
        icon: "kbd"; title: "Follow the keyboard focus"
        sub: "The view moves to the window or button you move to with the keyboard" + page.kwinNote
        checked: kw.b("kwinrc", "Effect-zoom", "EnableFocusTracking", false)
        onToggled: v => kw.set("kwinrc", "Effect-zoom", "EnableFocusTracking", v)
    }
    SetSwitch {
        enabled: kw.live; opacity: kw.live ? 1 : 0.45
        icon: "kbd"; title: "Follow the text cursor"
        sub: "The view keeps up with what you type" + page.kwinNote
        checked: kw.b("kwinrc", "Effect-zoom", "EnableTextCaretTracking", false)
        onToggled: v => kw.set("kwinrc", "Effect-zoom", "EnableTextCaretTracking", v)
    }
    SetChoice {
        enabled: kw.live; opacity: kw.live ? 1 : 0.45
        icon: "mouse"; title: "The pointer while zoomed"
        sub: page.kwinNote.replace(/^\. /, "")
        options: [{ value: 0, label: "Zoomed too" }, { value: 1, label: "Normal size" }, { value: 2, label: "Hidden" }]
        current: kw.n("kwinrc", "Effect-zoom", "MousePointer", 0)
        onChosen: v => kw.set("kwinrc", "Effect-zoom", "MousePointer", v)
    }
    SetChoice {
        title: "Each Super+= or Super+- step"
        options: [{ value: 0.25, label: "Small" }, { value: 0.5, label: "Medium" }, { value: 1, label: "Big" }]
        current: page.s.magnifierStep || 0.5
        onChosen: v => page.change({ magnifierStep: v })
    }

    // ------------------------------------------------ colour filters
    SetHeader { text: "Colour filters" }
    Repeater {
        model: page.av.filters || ["none", "grayscale", "invert", "invert-brightness", "protanopia", "deuteranopia", "tritanopia"]
        delegate: SetSwitch {
            required property var modelData
            visible: modelData !== "none"
            readonly property bool here: (page.av.desktopFilters || ["none", "invert"]).indexOf(modelData) >= 0
            // With Mantel on KWin: KWin's effects show it (greyscale from KWin 6.6).
            readonly property bool onKwinToo: (page.av.kwinFilters || []).indexOf(modelData) >= 0
            icon: "palette"; title: page.filterNames[modelData] || modelData
            sub: (page.filterSubs[modelData] || "") + (here ? "" : page.onKwin
                ? (modelData === "grayscale" ? ". Needs KWin 6.6 in Desktop Mode" : ". KWin has no effect for it: Game Mode only")
                : (modelData !== "invert-brightness" ? ". Not in Desktop Mode on labwc: Mantel on KWin shows it" : ". Not in Desktop Mode yet: its window manager can't show it"))
            checked: page.s.filter === modelData
            onToggled: v => page.change({ filter: v ? modelData : "none" }, true)
        }
    }
    SetSlider {
        visible: page.onKwin && ["protanopia", "deuteranopia", "tritanopia", "grayscale"].indexOf(page.s.filter) >= 0
        icon: "palette"; title: "How strong"
        value: kw.n("kwinrc", "Effect-colorblindnesscorrection", "Intensity", 1)
        onMoved: v => kw.set("kwinrc", "Effect-colorblindnesscorrection", "Intensity", Math.max(0.1, Math.round(v * 20) / 20))
    }
    Text {
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.onKwin ? "Super+Ctrl+I also inverts the colours (KWin's own shortcut)."
                          : "In Desktop Mode colours are inverted with the screen's colour ramp, so Night light has to be off for it."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }

    // ------------------------------------------------ screen reader
    SetHeader { text: "Screen reader" }
    SetSwitch {
        icon: "vol"; title: "Screen reader (Orca)"
        sub: !page.st ? "Checking…" : !page.av.orca ? "Orca isn't installed" : "Reads out what's on the screen and what you type. Super+Alt+S turns it on or off"
        checked: !!page.s.screenReader
        onToggled: v => page.change({ screenReader: v }, true)
    }

    // ------------------------------------------------ keyboard and mouse help (KWin's, kaccessrc)
    SetHeader { text: "Typing and the mouse" + (kw.job.what === "save" ? " · saving…" : "") }
    SetSwitch {
        enabled: kw.live; opacity: kw.live ? 1 : 0.45
        icon: "kbd"; title: "Sticky keys"
        sub: "Press Shift, Ctrl, Alt or Super, then the other key: no need to hold them together" + page.kwinNote
        checked: kw.b("kaccessrc", "Keyboard", "StickyKeys", false)
        onToggled: v => kw.set("kaccessrc", "Keyboard", "StickyKeys", v)
    }
    SetSwitch {
        visible: kw.b("kaccessrc", "Keyboard", "StickyKeys", false)
        enabled: kw.live
        icon: "lock"; title: "Press twice to lock"
        sub: "A key pressed twice stays down until pressed again"
        checked: kw.b("kaccessrc", "Keyboard", "StickyKeysLatch", true)
        onToggled: v => kw.set("kaccessrc", "Keyboard", "StickyKeysLatch", v)
    }
    SetSwitch {
        visible: kw.b("kaccessrc", "Keyboard", "StickyKeys", false)
        enabled: kw.live
        icon: "kbd"; title: "Turn off when two keys are held together"
        checked: kw.b("kaccessrc", "Keyboard", "StickyKeysAutoOff", false)
        onToggled: v => kw.set("kaccessrc", "Keyboard", "StickyKeysAutoOff", v)
    }
    SetSwitch {
        enabled: kw.live; opacity: kw.live ? 1 : 0.45
        icon: "kbd"; title: "Bounce keys"
        sub: "A key pressed again straight away is ignored (for shaky hands)" + page.kwinNote
        checked: kw.b("kaccessrc", "Keyboard", "BounceKeys", false)
        onToggled: v => kw.set("kaccessrc", "Keyboard", "BounceKeys", v)
    }
    SetSlider {
        visible: kw.b("kaccessrc", "Keyboard", "BounceKeys", false)
        enabled: kw.live
        icon: "clock"; title: "Ignore it for"
        value: kw.n("kaccessrc", "Keyboard", "BounceKeysDelay", 500) / 2000
        step: 0.025
        valueText: Math.round(value * 2000) + " ms"
        onMoved: v => kw.set("kaccessrc", "Keyboard", "BounceKeysDelay", Math.max(50, Math.round(v * 2000)))
    }
    SetSwitch {
        enabled: page.can(606); opacity: page.can(606) ? 1 : 0.45
        icon: "kbd"; title: "Slow keys"
        sub: "A key counts only when held a moment" + page.needs(606)
        checked: kw.b("kaccessrc", "Keyboard", "SlowKeys", false)
        onToggled: v => kw.set("kaccessrc", "Keyboard", "SlowKeys", v)
    }
    SetSlider {
        visible: kw.b("kaccessrc", "Keyboard", "SlowKeys", false)
        enabled: page.can(606)
        icon: "clock"; title: "Hold for"
        value: kw.n("kaccessrc", "Keyboard", "SlowKeysDelay", 500) / 2000
        step: 0.025
        valueText: Math.round(value * 2000) + " ms"
        onMoved: v => kw.set("kaccessrc", "Keyboard", "SlowKeysDelay", Math.max(50, Math.round(v * 2000)))
    }
    SetSwitch {
        enabled: page.can(605); opacity: page.can(605) ? 1 : 0.45
        icon: "mouse"; title: "Mouse keys"
        sub: "Move the pointer with the number keys (5 clicks)" + page.needs(605)
        checked: kw.b("kaccessrc", "Mouse", "MouseKeys", false)
        onToggled: v => kw.set("kaccessrc", "Mouse", "MouseKeys", v)
    }
    SetSlider {
        visible: kw.b("kaccessrc", "Mouse", "MouseKeys", false)
        enabled: page.can(605)
        icon: "mouse"; title: "Pointer speed with the keys"
        value: kw.n("kaccessrc", "Mouse", "MKMaxSpeed", 30) / 100
        valueText: Math.round(value * 100)
        onMoved: v => kw.set("kaccessrc", "Mouse", "MKMaxSpeed", Math.max(5, Math.round(v * 100)))
    }
    SetSwitch {
        enabled: kw.live; opacity: kw.live ? 1 : 0.45
        icon: "bell"; title: "Flash the screen for the alert sound"
        sub: "When an app beeps, the screen flashes too (the visual bell)" + page.kwinNote
        checked: kw.b("kaccessrc", "Bell", "VisibleBell", false)
        onToggled: v => kw.set("kaccessrc", "Bell", "VisibleBell", v)
    }
    SetChoice {
        visible: kw.b("kaccessrc", "Bell", "VisibleBell", false)
        enabled: kw.live
        icon: "bell"; title: "Flash"
        options: [{ value: false, label: "In red" }, { value: true, label: "Invert the screen" }]
        current: kw.b("kaccessrc", "Bell", "VisibleBellInvert", false)
        onChosen: v => kw.set("kaccessrc", "Bell", "VisibleBellInvert", v)
    }
    SetSwitch {
        enabled: kw.live; opacity: kw.live ? 1 : 0.45
        icon: "vol"; title: "Play the alert sound"
        sub: page.kwinNote.replace(/^\. /, "")
        checked: kw.b("kaccessrc", "Bell", "SystemBell", true)
        onToggled: v => kw.set("kaccessrc", "Bell", "SystemBell", v)
    }

    // ------------------------------------------------ text, contrast, pointer
    SetHeader { text: "Text and pointer" }
    SetChoice {
        icon: "file"; title: "Text size"
        sub: "For apps and window titles. Apps open now use it after you sign out and in again"
        options: [{ value: 1, label: "Default" }, { value: 1.15, label: "Larger" }, { value: 1.3, label: "Large" }, { value: 1.5, label: "Largest" }]
        current: page.s.textScale || 1
        onChosen: v => page.change({ textScale: v }, true)
    }
    SetSwitch {
        icon: "sun"; title: "High contrast"
        sub: "Apps with GTK use their high-contrast look (after signing out and in)"
        checked: !!page.s.highContrast
        onToggled: v => page.change({ highContrast: v }, true)
    }
    SetSwitch {
        icon: "mouse"; title: "Large pointer"
        sub: "A pointer twice the size (after signing out and in)"
        checked: !!page.s.largePointer
        onToggled: v => page.change({ largePointer: v }, true)
    }

    // ------------------------------------------------ motion (moved here from Appearance: the same setting, Settings.reduceMotion)
    SetHeader { text: "Motion" }
    SetSwitch { icon: "motion"; title: "Reduce motion"; sub: "Still wallpapers and fewer animations (animated ones also hold still on battery and behind fullscreen games)"; checked: Settings.reduceMotion; onToggled: v => Settings.reduceMotion = v }

    // ------------------------------------------------ More
    // (The archived Game Mode's own zoom, gameZoom, isn't offered: Steam Game Mode has its own magnifier in Steam's
    // Settings › Accessibility. hearth-access still keeps the setting. docs/SETTINGS-MOVE.md.)
    SetHeader { text: "More" }
    SetRow {
        icon: "kbd"; title: "Emoji and symbols"; sub: "Super+. opens the picker, in any app"; chevron: false
        onActivated: Quickshell.execDetached(["hearth-emoji"])
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: kw.message !== "" && kw.live
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: kw.message; color: kw.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
