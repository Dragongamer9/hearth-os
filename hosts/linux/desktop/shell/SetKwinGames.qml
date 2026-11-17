// Hearth Settings › Gaming › Full-screen games & X11 apps (Mantel on KWin, docs/KWIN-SETTINGS.md): tearing in full-screen
// games (kwinrc [Compositing] AllowTearing; per game with a window rule), which keys older X11 apps (Discord's
// push-to-talk, OBS) may see while another app is in front ([Xwayland] XwaylandEavesdrops / ...Mouse), and whether X11
// apps scale themselves or are scaled by KWin (kdeglobals [KScreen] XwaylandClientsScale; KWin then sets [Xwayland] Scale).
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Full-screen games & X11 apps"
    KwinSettings { id: kw }

    KwinGate {
        kw: kw
        what: "Tearing and X11 app settings"

        SetHeader { text: "Full-screen games" + (kw.job.what === "save" ? " · saving…" : "") }
        SetSwitch {
            icon: "pad"; title: "Allow tearing in full-screen games"
            sub: "Games that ask for it show each frame the moment it's ready: less lag, with a tear line now and then. Others stay smooth"
            checked: kw.b("kwinrc", "Compositing", "AllowTearing", true)
            onToggled: v => kw.set("kwinrc", "Compositing", "AllowTearing", v)
        }
        SetLinkCard { target: "kwin-rules"; title: "Tearing for one game"; sub: "A window rule can turn tearing on or off for one app" }
        SetSwitch {
            icon: "display"; title: "Screen edges over full-screen games"
            sub: "Off: hot corners never go off while a game is full screen"
            checked: kw.b("kwinrc", "ScreenEdges", "RemainActiveOnFullscreen", false)
            onToggled: v => kw.set("kwinrc", "ScreenEdges", "RemainActiveOnFullscreen", v)
        }

        SetHeader { text: "Older X11 apps" }
        SetChoice {
            icon: "kbd"; title: "Keys X11 apps can see while you're in another app"
            sub: "For push-to-talk and hotkeys in Discord, OBS and other X11 apps. Wayland apps never see them"
            options: [{ value: "None", label: "None" }, { value: "Modifiers", label: "Shift, Ctrl, Alt, Super" },
                      { value: "Combinations", label: "Those with another key" }, { value: "All", label: "Every key" }]
            current: kw.v("kwinrc", "Xwayland", "XwaylandEavesdrops", "Combinations")
            onChosen: v => kw.set("kwinrc", "Xwayland", "XwaylandEavesdrops", v)
        }
        SetSwitch {
            icon: "mouse"; title: "Mouse buttons too"
            sub: "X11 apps also see mouse buttons pressed in other apps"
            checked: kw.b("kwinrc", "Xwayland", "XwaylandEavesdropsMouse", false)
            onToggled: v => kw.set("kwinrc", "Xwayland", "XwaylandEavesdropsMouse", v)
        }
        SetChoice {
            icon: "display"; title: "Scaling X11 apps"
            sub: kw.b("kdeglobals", "KScreen", "XwaylandClientsScale", true)
                 ? "Apps draw themselves at your screen's scale: sharp, though a few older ones look small"
                 : "KWin enlarges them: the right size everywhere, a little blurry on a scaled screen"
            options: [{ value: true, label: "Apps scale themselves" }, { value: false, label: "KWin scales them" }]
            current: kw.b("kdeglobals", "KScreen", "XwaylandClientsScale", true)
            onChosen: v => kw.set("kdeglobals", "KScreen", "XwaylandClientsScale", v)
        }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "Most games, Steam and Proton use X11 through Xwayland."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
    }
}
