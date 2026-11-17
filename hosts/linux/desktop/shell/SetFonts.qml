// Hearth Settings › Personalization › Fonts, cursor & icons: the font apps use and its size, the fixed-width font
// (terminals, code), font smoothing and hinting, font DPI, the mouse cursor and its size, and the icon theme, for Qt/KDE
// and GTK apps alike (hearth-lookset). Saved in ~/.config/nimbus/fonts-cursor-icons.json only when you pick something;
// "Hearth's default" takes your choice out again. The look's own colours stay in Personalization.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Fonts, cursor & icons"
    property var st: null              // hearth-lookset status
    property string message: ""
    property bool messageBad: false
    readonly property var e: st ? st.effective : ({})
    readonly property var s: st ? st.saved : ({})
    function opts(list, def) { return [{ value: "", label: "Hearth's default (" + def + ")" }].concat(list.map(f => typeof f === "string" ? ({ value: f, label: f }) : ({ value: f.id, label: f.name }))); }

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["status"], r => page.st = r, e => page.say(e, true)); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-lookset" }
    // One setting at a time; "" (Hearth's default) takes it out.
    function pick(key, v, ok) {
        say("");
        const req = {}; req[key] = v === "" ? null : v;
        job.run(["set"], r => { page.st = Object.assign({}, page.st, { saved: r.saved, effective: r.effective }); page.say(ok || "Saved. Apps opened from now on use it."); },
                e => page.say(e, true), JSON.stringify(req), "set");
    }

    SetCardGroup {
        title: "Fonts"
        SetDropdown {
            icon: "doc"; title: "Font"; sub: "Menus, buttons and text in apps"
            options: page.st ? page.opts(page.st.fonts, page.st.defaults.font) : []
            current: page.s.font || ""
            boxWidth: 300
            onChosen: v => page.pick("font", v)
        }
        SetSlider {
            icon: "doc"; title: "Font size"
            readonly property real shown: sizeTimer.key === "fontSize" && (sizeTimer.running || job.busy) ? sizeTimer.val : (page.e.fontSize || 10)
            value: (shown - 6) / 18; step: 1 / 36
            valueText: shown + " pt"
            onMoved: v => sizeTimer.restartWith("fontSize", Math.round((6 + v * 18) * 2) / 2)
        }
        SetDropdown {
            icon: "kbd"; title: "Fixed-width font"; sub: "Terminals and code"
            options: page.st ? page.opts(page.st.monoFonts, page.st.defaults.mono) : []
            current: page.s.mono || ""
            boxWidth: 300
            onChosen: v => page.pick("mono", v)
        }
        SetSlider {
            icon: "kbd"; title: "Fixed-width font size"
            readonly property real shown: sizeTimer.key === "monoSize" && (sizeTimer.running || job.busy) ? sizeTimer.val : (page.e.monoSize || 10)
            value: (shown - 6) / 18; step: 1 / 36
            valueText: shown + " pt"
            onMoved: v => sizeTimer.restartWith("monoSize", Math.round((6 + v * 18) * 2) / 2)
        }
        SetExpander {
            icon: "gear"; title: "Font smoothing and DPI"; sub: "For sharp or soft text on your screen"
            SetChoice {
                title: "Smoothing"
                options: [{ value: "grayscale", label: "Normal" }, { value: "rgba", label: "Subpixel (LCD)" }, { value: "none", label: "Off" }]
                current: page.e.antialias || "grayscale"
                onChosen: v => page.pick("antialias", v)
            }
            SetChoice {
                title: "Hinting"
                options: [{ value: "none", label: "None" }, { value: "slight", label: "Slight" }, { value: "medium", label: "Medium" }, { value: "full", label: "Full" }]
                current: page.e.hinting || "slight"
                onChosen: v => page.pick("hinting", v)
            }
            SetChoice {
                title: "Font DPI"; sub: "Bigger text without changing the display's scale"
                options: [{ value: 96, label: "96 (normal)" }, { value: 108, label: "108" }, { value: 120, label: "120" }, { value: 144, label: "144" }]
                current: page.e.dpi || 96
                onChosen: v => page.pick("dpi", v === 96 ? "" : v)
            }
        }
    }
    // Sizes wait until you stop moving the slider, then save once.
    Timer {
        id: sizeTimer
        interval: 700
        property string key: ""
        property real val: 0
        function restartWith(k, v) { key = k; val = v; restart(); }
        onTriggered: page.pick(key, val === 10 ? "" : val)
    }
    SetCardGroup {
        title: "Cursor"
        SetDropdown {
            icon: "mouse"; title: "Cursor"
            options: page.st ? page.opts(page.st.cursors, "Breeze") : []
            current: page.s.cursor || ""
            boxWidth: 300
            onChosen: v => page.pick("cursor", v, "Saved. The cursor changes in apps as they open; sign out and in for everything.")
        }
        SetChoice {
            icon: "mouse"; title: "Cursor size"
            options: [{ value: 24, label: "Normal" }, { value: 32, label: "Large" }, { value: 48, label: "Larger" }, { value: 64, label: "Huge" }]
            current: page.e.cursorSize || 24
            onChosen: v => page.pick("cursorSize", v === 24 ? "" : v, "Saved. Sign out and in for every app to use it.")
        }
    }
    SetCardGroup {
        title: "Icons"
        SetDropdown {
            icon: "grid"; title: "Icon theme"; sub: "App icons in Qt/KDE and GTK apps (Hearth's own icons stay)"
            options: page.st ? page.opts(page.st.icons, "follows light or dark") : []
            current: page.s.icons || ""
            boxWidth: 300
            onChosen: v => page.pick("icons", v)
        }
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
}
