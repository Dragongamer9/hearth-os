// Hearth Settings › Apps › Window rules (Mantel on KWin, docs/KWIN-SETTINGS.md): Hearth's editor for KWin's window rules
// (kwinrulesrc). Each rule matches an app (its window class) and, if you like, a window title, and sets where the window
// goes, its size, desktop and screen, keep above or below, no frame, see-through, full screen, tearing for games and more,
// either once when it opens, always (forced) or remembered. Every change writes only that rule's changed keys; KWin
// reads the rules again at once.
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Window rules"
    KwinSettings { id: kw; onLoaded: page.load() }
    property var rules: []
    property var desktopsList: []
    property string openId: ""
    function load() {
        kw.job.run(["rules"], r => page.rules = r.rules || [], e => kw.say(e, true));
        kw.job.run(["desktops"], r => page.desktopsList = r.desktops || [], () => {});
    }
    function change(id, set, after) {
        kw.job.run(["rule-set"], r => { page.rules = r.rules || []; if (after) after(r.id); },
            e => kw.say(e, true), JSON.stringify({ id: id, set: set }), "save");
    }
    function remove(id) { kw.job.run(["rule-delete", id], r => page.rules = r.rules || [], e => kw.say(e, true), "", "save"); }
    // How a rule is used (KWin's Rules: 2 force, 3 apply when it opens, 4 remember, 5 apply now, 6 force until it closes)
    readonly property var hows: [{ value: 3, label: "When it opens" }, { value: 2, label: "Always" }, { value: 4, label: "Remember" }]
    readonly property var matches: [{ value: 0, label: "Any" }, { value: 1, label: "Is" }, { value: 2, label: "Has" }, { value: 3, label: "Pattern" }]
    readonly property var toggles: [
        { key: "above", label: "Keep above other windows", icon: "pin" },
        { key: "below", label: "Keep below other windows", icon: "window" },
        { key: "noborder", label: "No title bar and frame", icon: "window" },
        { key: "fullscreen", label: "Full screen", icon: "display" },
        { key: "maximizehoriz", label: "Maximized", icon: "window", also: "maximizevert" },
        { key: "minimize", label: "Minimized", icon: "window" },
        { key: "skiptaskbar", label: "Leave out of the bar", icon: "menu" },
        { key: "skipswitcher", label: "Leave out of Alt+Tab", icon: "grid" },
        { key: "tearing", label: "Allow tearing (less lag in games)", icon: "pad" },
        { key: "adaptivesync", label: "Variable refresh rate", icon: "display" },
        { key: "acceptfocus", label: "Can take keyboard focus", icon: "kbd" },
        { key: "closeable", label: "Can be closed", icon: "close" }
    ]
    function summary(r) {
        const v = r.values;
        const app = v.wmclass ? v.wmclass : "any app";
        const n = Object.keys(v).filter(k => k.endsWith("rule") && parseInt(v[k]) > 1).length;
        return app + (v.title ? " · title " + (parseInt(v.titlematch) === 1 ? "is" : "has") + " “" + v.title + "”" : "") + " · " + n + (n === 1 ? " setting" : " settings");
    }

    KwinGate {
        kw: kw
        what: "Window rules"

        SetHeader { text: "Your rules" + (kw.job.what === "save" ? " · saving…" : "") }
        Text {
            visible: page.rules.length === 0 && kw.ready
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "No rules yet. A rule makes an app's windows open where, how big and how you want, every time."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
        Repeater {
            model: page.rules
            delegate: SetExpander {
                id: ex
                required property var modelData
                readonly property var r: modelData
                readonly property var v: modelData.values
                function val(k, d) { return v[k] !== undefined ? v[k] : d; }
                function on(k) { return parseInt(val(k + "rule", 0)) > 1; }
                icon: "window"
                title: val("Description", "Window rule")
                sub: page.summary(modelData)
                chip: val("Enabled", "true") === "false" ? "Off" : ""
                chipKind: "off"
                open: page.openId === modelData.id
                SetField {
                    Layout.leftMargin: 56
                    text: ex.val("Description", "")
                    placeholder: "Name of this rule"
                    onSubmitted: t => page.change(ex.r.id, { Description: t || "Window rule" })
                }
                SetSwitch {
                    title: "Use this rule"
                    checked: ex.val("Enabled", "true") !== "false"
                    onToggled: c => page.change(ex.r.id, { Enabled: c ? null : false })
                }
                SetRow {
                    icon: "search"; title: "Pick the app's window"
                    sub: kw.running ? "Then click the window: its app and title are filled in" : "Needs KWin running"
                    enabled: kw.running
                    onActivated: kw.job.run(["pick-window"], w => page.change(ex.r.id, { wmclass: w.wmclass, wmclassmatch: 1, wmclasscomplete: false }), e => kw.say(e, true))
                }
                SetChoice {
                    title: "App (window class)"
                    sub: ex.val("wmclass", "") || "Every app"
                    options: page.matches
                    current: parseInt(ex.val("wmclassmatch", 0))
                    onChosen: m => page.change(ex.r.id, { wmclassmatch: m })
                }
                SetField {
                    Layout.leftMargin: 56
                    visible: parseInt(ex.val("wmclassmatch", 0)) > 0
                    text: ex.val("wmclass", "")
                    placeholder: "firefox, steam, org.kde.dolphin…"
                    onSubmitted: t => page.change(ex.r.id, { wmclass: t })
                }
                SetChoice {
                    title: "Window title"
                    sub: ex.val("title", "") || "Any title"
                    options: page.matches
                    current: parseInt(ex.val("titlematch", 0))
                    onChosen: m => page.change(ex.r.id, { titlematch: m })
                }
                SetField {
                    Layout.leftMargin: 56
                    visible: parseInt(ex.val("titlematch", 0)) > 0
                    text: ex.val("title", "")
                    placeholder: "Words in the title"
                    onSubmitted: t => page.change(ex.r.id, { title: t })
                }
                // ---- what it sets
                Repeater {
                    model: page.toggles
                    delegate: ColumnLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: 0
                        SetSwitch {
                            inset: true
                            title: modelData.label
                            sub: ex.on(modelData.key) ? (ex.val(modelData.key, "false") === "true" ? "Yes" : "No") + ", " + (page.hows.find(h => h.value === parseInt(ex.val(modelData.key + "rule", 2))) || page.hows[1]).label.toLowerCase() : "Not set by this rule"
                            checked: ex.on(modelData.key)
                            onToggled: c => {
                                const s = {};
                                for (const k of [modelData.key].concat(modelData.also ? [modelData.also] : [])) {
                                    s[k] = c ? true : null;
                                    s[k + "rule"] = c ? 2 : null;
                                }
                                page.change(ex.r.id, s);
                            }
                        }
                        SetChoice {
                            inset: true
                            visible: ex.on(modelData.key)
                            title: modelData.label + ": set to"
                            options: [{ value: true, label: "Yes" }, { value: false, label: "No" }]
                            current: ex.val(modelData.key, "false") === "true"
                            onChosen: c => {
                                const s = {};
                                for (const k of [modelData.key].concat(modelData.also ? [modelData.also] : [])) s[k] = c;
                                page.change(ex.r.id, s);
                            }
                        }
                        SetChoice {
                            inset: true
                            visible: ex.on(modelData.key)
                            title: modelData.label + ": when"
                            options: page.hows
                            current: parseInt(ex.val(modelData.key + "rule", 2))
                            onChosen: c => {
                                const s = {};
                                for (const k of [modelData.key].concat(modelData.also ? [modelData.also] : [])) s[k + "rule"] = c;
                                page.change(ex.r.id, s);
                            }
                        }
                    }
                }
                Repeater {
                    model: [{ key: "position", label: "Position", hint: "x,y in pixels, e.g. 0,0" }, { key: "size", label: "Size", hint: "width,height, e.g. 1280,720" }]
                    delegate: ColumnLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: 0
                        SetSwitch {
                            inset: true
                            title: modelData.label
                            sub: ex.on(modelData.key) ? ex.val(modelData.key, "") : "Not set by this rule"
                            checked: ex.on(modelData.key)
                            onToggled: c => page.change(ex.r.id, { [modelData.key]: c ? (modelData.key === "size" ? "1280,720" : "0,0") : null, [modelData.key + "rule"]: c ? 3 : null })
                        }
                        SetField {
                            Layout.leftMargin: 56
                            visible: ex.on(modelData.key)
                            text: ex.val(modelData.key, "")
                            placeholder: modelData.hint
                            onSubmitted: t => page.change(ex.r.id, { [modelData.key]: t.replace(/\s/g, "") })
                        }
                        SetChoice {
                            inset: true
                            visible: ex.on(modelData.key)
                            title: modelData.label + ": how"
                            options: page.hows
                            current: parseInt(ex.val(modelData.key + "rule", 3))
                            onChosen: c => page.change(ex.r.id, { [modelData.key + "rule"]: c })
                        }
                    }
                }
                SetSwitch {
                    inset: true
                    title: "On a desktop"
                    sub: ex.on("desktops") ? ((page.desktopsList.find(d => d.id === ex.val("desktops", "")) || {}).name || "All desktops") : "Not set by this rule"
                    checked: ex.on("desktops")
                    onToggled: c => page.change(ex.r.id, { desktops: c ? ((page.desktopsList[0] || {}).id || "") : null, desktopsrule: c ? 3 : null })
                }
                SetChoice {
                    inset: true
                    visible: ex.on("desktops")
                    title: "Which desktop"
                    options: page.desktopsList.filter(d => d.id).map(d => ({ value: d.id, label: d.name })).concat([{ value: "", label: "All" }])
                    current: ex.val("desktops", "")
                    onChosen: c => page.change(ex.r.id, { desktops: c })
                }
                SetSwitch {
                    inset: true
                    title: "On a screen"
                    sub: ex.on("screen") ? "Screen " + (parseInt(ex.val("screen", 0)) + 1) : "Not set by this rule"
                    checked: ex.on("screen")
                    onToggled: c => page.change(ex.r.id, { screen: c ? 0 : null, screenrule: c ? 3 : null })
                }
                SetChoice {
                    inset: true
                    visible: ex.on("screen")
                    title: "Which screen"
                    options: [0, 1, 2, 3].map(i => ({ value: i, label: String(i + 1) }))
                    current: parseInt(ex.val("screen", 0))
                    onChosen: c => page.change(ex.r.id, { screen: c })
                }
                Repeater {
                    model: [{ key: "opacityactive", label: "How solid it is in front" }, { key: "opacityinactive", label: "How solid it is behind others" }]
                    delegate: ColumnLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: 0
                        SetSwitch {
                            inset: true
                            title: modelData.label
                            sub: ex.on(modelData.key) ? ex.val(modelData.key, 100) + "%" : "Not set by this rule"
                            checked: ex.on(modelData.key)
                            onToggled: c => page.change(ex.r.id, { [modelData.key]: c ? 90 : null, [modelData.key + "rule"]: c ? 2 : null })
                        }
                        SetSlider {
                            inset: true
                            visible: ex.on(modelData.key)
                            title: modelData.label
                            value: parseInt(ex.val(modelData.key, 100)) / 100
                            onMoved: v => slow.go(ex.r.id, modelData.key, Math.max(10, Math.round(v * 100)))
                        }
                    }
                }
                SetButton {
                    Layout.leftMargin: 56; Layout.topMargin: 6; Layout.bottomMargin: 8
                    text: "Delete this rule"; icon: "trash"; ghost: true
                    onActivated: page.remove(ex.r.id)
                }
                Text {
                    visible: (ex.r.other || []).length > 0
                    Layout.fillWidth: true; Layout.leftMargin: 56; Layout.bottomMargin: 8
                    text: "Also has settings made in KDE's own editor (" + (ex.r.other || []).join(", ") + "): kept as they are."
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
                }
            }
        }
        // Sliders move in small steps: save once they stop.
        Timer {
            id: slow
            interval: 600
            property var args: null
            function go(id, key, value) { args = [id, key, value]; restart(); }
            onTriggered: if (args) page.change(args[0], { [args[1]]: args[2] })
        }
        SetButton {
            Layout.topMargin: 8
            text: "New rule"; icon: "plus"; primary: true
            onActivated: page.change("", { Description: "New rule" }, id => page.openId = id)
        }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4; Layout.topMargin: 6
            text: "An app's window class is usually its name (firefox, steam). “Pick the app's window” finds it for you."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
    }
}
