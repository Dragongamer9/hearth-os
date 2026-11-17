// Hearth Settings › System › Your desktop (1.13, docs/DESKTOPS.md; mockup docs/mockups/next-update/
// desktops-1-your-desktop-settings.png): pick the desktop Desktop Mode opens. Mantel (Hearth Classic) is built in; KDE
// Plasma and GNOME are downloaded when picked ("Use KDE Plasma"), can be cancelled while downloading and removed again.
// Everything through hearth-desktops (the session.conf lines through hearth_settings_io; the downloads through the
// root helper). Opening the page only reads; nothing here changes by itself.
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Your desktop"
    property var st: null
    property string msg: ""
    property bool bad: false
    function say(t, b) { msg = t; bad = !!b; }
    property string wantPick: ""      // "Use KDE Plasma" before it was downloaded: picked once it's here (while this page is open)
    function refresh() { job.run(["status"], r => { page.st = r; page.pickWhenReady(); }, e => page.say(e, true)); }
    function pickWhenReady() {
        if (wantPick === "" || !st) return;
        const s = d(wantPick).state;
        if (s === "downloaded") { const w = wantPick; wantPick = ""; run(["pick", w], "Downloaded · Desktop Mode opens " + (w === "kde" ? "KDE Plasma" : "GNOME") + " ✓"); }
        else if (["starting", "downloading", "installing"].indexOf(s) < 0) wantPick = "";
    }
    Component.onCompleted: { refresh(); lookJob.run(["status"], r => page.lookSt = r, () => {}); }
    SetJob { id: job; helper: "hearth-desktops" }
    SetJob { id: act; helper: "hearth-desktops" }
    readonly property bool busy: !!st && ["kde", "gnome"].some(d => ["starting", "downloading", "installing", "removing"].indexOf(((st.desktops || {})[d] || {}).state) >= 0)
    Timer { interval: 1500; repeat: true; running: page.busy && page.visible; onTriggered: if (!job.busy) page.refresh() }
    function d(id) { return st && st.desktops ? (st.desktops[id] || {}) : {}; }
    function run(args, done) { act.run(args, r => { page.st = r; if (done) page.say(done); }, e => { page.say(e, true); page.refresh(); }); }
    readonly property var lines: ({
        classic: "Hearth's own desktop with all of Hearth's apps. Always there, and where Hearth goes back to if another desktop can't start.",
        kde: "KDE's own look and apps (Dolphin, Konsole, System Settings...) with Hearth inside: Game Mode, controller, keyboard, notifications.",
        gnome: "GNOME's own look and apps (Files, Console, GNOME Settings...) with Hearth inside."
    })

    Text {
        Layout.fillWidth: true; Layout.bottomMargin: 8
        text: "Pick the desktop Desktop Mode opens."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap
    }
    SetBanner {
        visible: !!page.st && !!page.st.notice && page.st.notice.text !== ""
        kind: "warn"; icon: "warn"
        title: page.st && page.st.notice ? page.st.notice.text : ""
        sub: page.st && page.st.notice && page.st.notice.kind === "missing" && page.st.reapply === "running"
             ? "It's being put back after the update; the next time you open Desktop Mode it's there."
             : "Your choice stays saved. Pick it again below to try once more, or keep Mantel."
    }

    Repeater {
        model: ["classic", "kde", "gnome"]
        delegate: SetBase {
            id: card
            required property string modelData
            readonly property var info: modelData === "classic" ? ({ state: "downloaded" }) : page.d(modelData)
            readonly property bool chosen: !!page.st && page.st.picked === modelData
            readonly property bool here: info.state === "downloaded"
            readonly property bool moving: ["starting", "downloading", "installing", "removing"].indexOf(info.state) >= 0
            readonly property string name: modelData === "classic" ? "Mantel (Hearth Classic)" : modelData === "kde" ? "KDE Plasma" : "GNOME"
            Layout.fillWidth: true
            implicitHeight: Math.max(96, col.implicitHeight + 36)
            border.width: chosen ? 2 : 1
            border.color: chosen ? Theme.accent : SetStyle.tint(Theme.text, 0.05)
            onActivated: if (here && !chosen) page.run(["pick", modelData], "Desktop Mode opens " + name + " ✓")
            RowLayout {
                anchors { fill: parent; leftMargin: 22; rightMargin: 22 }
                spacing: 18
                Rectangle {   // the radio
                    Layout.alignment: Qt.AlignVCenter
                    width: 22; height: 22; radius: 11; color: "transparent"
                    border.width: 2; border.color: card.chosen ? Theme.accent : Theme.muted
                    Rectangle { anchors.centerIn: parent; width: 10; height: 10; radius: 5; color: Theme.accent; visible: card.chosen }
                }
                Rectangle {   // its letter (no KDE or GNOME logos)
                    Layout.alignment: Qt.AlignVCenter
                    width: 54; height: 54; radius: 12
                    color: card.modelData === "classic" ? "#4a3a24" : card.modelData === "kde" ? "#1f2f4a" : "#1f3a2a"
                    Text { anchors.centerIn: parent; text: card.modelData === "classic" ? "M" : card.modelData === "kde" ? "K" : "G"
                        color: card.modelData === "classic" ? "#f0a840" : card.modelData === "kde" ? "#7fb2f0" : "#8fd89a"
                        font.family: Theme.font; font.pixelSize: 26; font.weight: Font.Bold }
                }
                ColumnLayout {
                    id: col
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignVCenter
                    spacing: 6
                    RowLayout {
                        spacing: 10
                        Text { text: card.name; color: Theme.text; font.family: Theme.font; font.pixelSize: 18; font.weight: Font.DemiBold }
                        SetChip {
                            text: card.modelData === "classic" ? "built in" + (page.st && page.st.now === "classic" ? " · using now" : "")
                                : card.info.state === "downloaded" ? (page.st && page.st.now === card.modelData ? "downloaded · using now" : "downloaded")
                                : card.info.state === "downloading" || card.info.state === "starting" ? "downloading"
                                : card.info.state === "installing" ? "installing"
                                : card.info.state === "removing" ? "removing"
                                : card.info.state === "reapplying" ? "coming back after the update"
                                : card.info.state === "missing" ? "not on this version yet" : ""
                            visible: text !== ""
                            kind: card.modelData === "classic" ? "ok" : "new"
                        }
                        SetChip { visible: (card.info.fails || 0) >= 2; text: "didn't start twice"; kind: "warn" }
                    }
                    Text { Layout.fillWidth: true; text: page.lines[card.modelData]; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap }
                    Rectangle {   // the download's progress
                        visible: card.moving
                        Layout.preferredWidth: Math.min(col.width, 520); height: 8; radius: 4; color: SetStyle.card2
                        Rectangle { width: parent.width * Math.max(0.02, (card.info.percent || 0) / 100); height: parent.height; radius: 4; color: Theme.accent }
                    }
                    Text {
                        visible: card.moving
                        text: (card.info.percent || 0) + "% · made for Hearth OS " + (page.st ? (page.st.version || "") : "")
                              + (card.info.state === "installing" ? " · installing, almost done" : " · the rest keeps working while it downloads")
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
                    }
                    Text {
                        visible: !card.moving && (card.info.error || "") !== ""
                        Layout.fillWidth: true; wrapMode: Text.WordWrap
                        text: "Last try didn't finish: " + (card.info.error || "")
                        color: SetStyle.warn; font.family: Theme.font; font.pixelSize: 12
                    }
                }
                Row {
                    Layout.alignment: Qt.AlignVCenter
                    spacing: 12
                    visible: card.modelData !== "classic"
                    SetButton { visible: card.here && !card.chosen; text: "Remove"
                        onActivated: page.run(["remove", card.modelData], card.name + " removed") }
                    SetButton { visible: card.here && !card.chosen; primary: true; text: "Use " + card.name
                        onActivated: page.run(["pick", card.modelData], "Desktop Mode opens " + card.name + " ✓") }
                    SetButton { visible: !card.here && !card.moving && card.info.state !== "reapplying"; primary: true; text: "Use " + card.name
                        onActivated: { page.wantPick = card.modelData; page.run(["install", card.modelData], "Downloading " + card.name + "… the rest keeps working; it's picked once it's here") } }
                    SetButton { visible: card.moving && card.info.state !== "installing" && card.info.state !== "removing"; text: "Cancel"
                        onActivated: page.run(["cancel", card.modelData], "Stopped") }
                }
            }
        }
    }

    SetSwitch {
        title: "Only show each desktop's own apps"
        sub: "Mantel shows Hearth's apps, KDE shows KDE's, GNOME shows GNOME's. Apps you installed (Bazaar, Flatpak, Steam, .deb) show on every desktop."
        showState: false
        checked: !page.st || page.st.ownApps !== false
        onToggled: on => page.run(["own-apps", on ? "on" : "off"], on ? "Each desktop shows its own apps" : "Every desktop shows every app")
    }
    // 1.14: KDE Plasma and GNOME look like Mantel and follow it (hearth-desktop-look), on unless you turn it off.
    SetJob { id: lookJob; helper: "hearth-desktop-look" }
    property var lookSt: null
    SetSwitch {
        title: "KDE and GNOME look like Mantel"
        sub: "Mantel's look, light or dark, accent, wallpaper, font and its bar and dock, each time KDE or GNOME starts. Change the look in Personalization; off leaves KDE's and GNOME's own settings as they are."
        showState: false
        checked: !page.lookSt || page.lookSt.on !== false
        onToggled: on => lookJob.run([on ? "on" : "off"], r => { page.lookSt = r; SettingsNav.toast(on ? "KDE and GNOME follow Mantel's look" : "KDE and GNOME keep their own look"); }, e => SettingsNav.toast(e))
    }
    SetDropdown {
        title: "Pick at the login screen too"
        sub: "Hearth Classic, KDE Plasma and GNOME are listed when you sign in. Game Mode's power menu › Switch to Desktop opens the one picked here."
        options: [{ value: "save", label: "This one" }, { value: "once", label: "Just that time" }]
        current: page.st ? page.st.loginPick : "save"
        boxWidth: 200
        onChosen: v => page.run(["login", v], v === "save" ? "The login screen's pick becomes the one picked here" : "The login screen's pick is for that sign-in only")
    }
    SetRow {
        title: "Safe start"
        sub: "If KDE or GNOME doesn't start twice in a row, Hearth opens Mantel and tells you why. Your choice stays saved until you change it."
        chip: "always on"; chipKind: "ok"
        interactive: false
    }
    Text { visible: page.msg !== ""; Layout.fillWidth: true; text: page.msg; color: page.bad ? SetStyle.warn : Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
}
