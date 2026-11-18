// Hearth Settings › Personalization › Virtual desktops (Mantel on KWin, docs/KWIN-SETTINGS.md): how many desktops and
// their names (KWin's VirtualDesktopManager over D-Bus, which keeps them in kwinrc [Desktops]; straight into kwinrc when
// KWin isn't running), how many rows, going round from the last to the first ([Windows] RollOverDesktops), the
// switching animation (slide / fade effects), the desktop's name shown on switching (desktopchangeosd) and the Desktop
// Cube (kwin-addons' cube effect, Debian's kdeplasma-addons: os-bases/peppermint/build.sh "optional kwin-addons").
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Virtual desktops"
    KwinSettings { id: kw; onLoaded: page.load() }
    property var ds: ({ desktops: [], rows: 1 })
    function load() { kw.job.run(["desktops"], r => page.ds = r, e => kw.say(e, true)); }
    function change(o) { kw.job.run(["desktops-set"], r => page.ds = r, e => { kw.say(e, true); page.load(); }, JSON.stringify(o), "save"); }
    readonly property int count: (ds.desktops || []).length

    KwinGate {
        kw: kw
        what: "Virtual desktops"

        SetHeader { text: "Desktops" + (kw.job.what === "save" ? " · saving…" : "") }
        SetSlider {
            icon: "desktops"; title: "How many"
            sub: "Super+Ctrl+Left / Right switch between them; three or four fingers sideways on a touchpad"
            value: (page.count - 1) / 19
            step: 1 / 19
            valueText: String(page.count)
            onMoved: v => { const c = Math.max(1, Math.min(20, Math.round(1 + v * 19))); if (c !== page.count) page.change({ count: c }); }
        }
        Repeater {
            model: page.ds.desktops || []
            delegate: ColumnLayout {
                required property var modelData
                required property int index
                Layout.fillWidth: true
                spacing: 2
                Text { text: "Desktop " + (index + 1); color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; Layout.leftMargin: 4; Layout.topMargin: 4 }
                SetField {
                    text: modelData.name
                    placeholder: "Desktop " + (index + 1)
                    onSubmitted: t => { if (t !== modelData.name) page.change({ names: { [modelData.id || String(index + 1)]: t } }); }
                }
            }
        }
        SetChoice {
            icon: "grid"; title: "Rows"
            sub: "Desktops laid out in a grid: Super+Ctrl+Up / Down move between rows"
            options: [1, 2, 3, 4].map(n => ({ value: n, label: String(n), disabled: n > page.count }))
            current: Math.min(page.ds.rows || 1, Math.max(1, page.count))
            onChosen: v => page.change({ rows: v })
        }
        SetSwitch {
            icon: "sync"; title: "Go round"
            sub: "From the last desktop, the next one is the first"
            checked: kw.b("kwinrc", "Windows", "RollOverDesktops", true)
            onToggled: v => kw.set("kwinrc", "Windows", "RollOverDesktops", v)
        }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "Every screen switches together: KWin 6 has one set of desktops for all screens."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }

        SetHeader { text: "Switching" }
        SetChoice {
            icon: "motion"; title: "Animation"
            options: [{ value: "slide", label: "Slide", disabled: !kw.hasEffect("slide") },
                      { value: "fadedesktop", label: "Fade", disabled: !kw.hasEffect("fadedesktop") },
                      { value: "none", label: "None" }]
            current: kw.effectOn("slide") ? "slide" : kw.effectOn("fadedesktop") ? "fadedesktop" : "none"
            onChosen: v => {
                const g = {};
                if (kw.effectOn("slide") !== (v === "slide")) g.slideEnabled = v === "slide";
                if (kw.effectOn("fadedesktop") !== (v === "fadedesktop")) g.fadedesktopEnabled = v === "fadedesktop";
                kw.setMany("kwinrc", { Plugins: g });
            }
        }
        SetSwitch {
            visible: kw.effectOn("slide")
            icon: "image"; title: "The wallpaper slides too"
            checked: kw.b("kwinrc", "Effect-slide", "SlideBackground", true)
            onToggled: v => kw.set("kwinrc", "Effect-slide", "SlideBackground", v)
        }
        SetSlider {
            visible: kw.effectOn("slide")
            icon: "desktops"; title: "Gap between desktops while sliding"
            value: kw.n("kwinrc", "Effect-slide", "HorizontalGap", 45) / 200
            valueText: Math.round(value * 200) + " px"
            onMoved: v => kw.setMany("kwinrc", { "Effect-slide": { HorizontalGap: Math.round(v * 200), VerticalGap: Math.round(v * 200 * 20 / 45) } })
        }
        SetSwitch {
            icon: "info"; title: "Show the desktop's name when switching"
            checked: kw.effectOn("desktopchangeosd")
            onToggled: v => kw.setEffect("desktopchangeosd", v)
        }
        SetLinkCard { target: "kwin-edges"; title: "Switch by pushing against the screen's edge"; sub: "In Screen edges" }

        // The Desktop Cube (KWin 6: kwin-addons, [Plugins] cubeEnabled; its shortcut "Cube", Super+C).
        SetHeader { text: "Desktop cube" }
        SetSwitch {
            icon: "box"; title: "Desktop cube"
            sub: kw.hasEffect("cube") ? "Super+C puts your desktops on the sides of a cube: drag to spin it, click a side to go there"
                                      : "Not in this KWin: the cube comes with the kwin-addons package, which isn't installed"
            checked: kw.effectOn("cube"); enabled: kw.hasEffect("cube")
            onToggled: v => kw.setEffect("cube", v)
        }
        SetRow {
            visible: kw.effectOn("cube") && kw.hasEffect("cube")
            icon: "box"; title: "Open the cube now"
            sub: page.count < 3 ? "It needs three desktops or more to make a cube" : "Esc or Super+C closes it"
            chevron: false
            enabled: page.count >= 3        // without KWin running it says so (hearth-kwin-settings shortcut)
            onActivated: kw.shortcut("cube")
        }
    }
}
