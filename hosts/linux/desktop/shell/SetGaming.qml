// Hearth Settings › Gaming: Steam Game Mode up top, then where Game Mode starts, your games and stores, Remote play,
// game storage, the controller and the handheld. "Start in" lives here (StartInChoice.qml); "Light mode while you play"
// is the Home server's setting, shown here too (the same key through hearth-home-server). Opening it only reads.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Gaming"
    property var steam: null          // steamMode.status
    property var boot: null           // hearth-boot-look status
    property var hhd: null            // hearth-hhd page
    property var hs: null             // hearth-home-server status
    readonly property var hsCfg: hs ? hs.settings : null
    HearthCall { id: core }
    SetJob { id: bootJob; helper: "/usr/libexec/nimbus/hearth-boot-look" }
    SetJob { id: hhdJob; helper: "hearth-hhd" }
    SetJob { id: hsJob; helper: "hearth-home-server" }
    Component.onCompleted: {
        core.call("steamMode.status", {}, v => page.steam = v, () => {});
        bootJob.run(["status"], r => page.boot = r, () => {});
        hhdJob.run(["page"], r => page.hhd = r, () => page.hhd = { running: false });
        hsJob.run(["status"], r => page.hs = r, () => {});
    }
    function movieLabel() {
        if (!boot || !boot.movie) return "";
        if (boot.movie.current === "steam") return "Steam's own";
        const m = (boot.movie.list || []).find(x => x.id === boot.movie.current);
        return m ? m.label : "";
    }

    // ---- Steam Game Mode
    SetBase {
        id: hero
        Layout.fillWidth: true
        Layout.bottomMargin: 6
        implicitHeight: Math.max(104, heroText.implicitHeight + 32)
        radius: SetStyle.bigRadius
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0; color: SetStyle.mix(SetStyle.card, Theme.accent, 0.22) }
            GradientStop { position: 0.6; color: SetStyle.card }
        }
        onActivated: Quickshell.execDetached(["nimbus-mode", "game"])
        Accessible.name: "Switch to Game Mode"
        RowLayout {
            anchors { fill: parent; leftMargin: 20; rightMargin: 20 }
            spacing: 18
            Rectangle {
                Layout.preferredWidth: 56; Layout.preferredHeight: 56; radius: Math.max(8, Theme.r(12))
                color: SetStyle.tint(Theme.accent, 0.18)
                Icon { anchors.centerIn: parent; name: "steam"; size: 30; color: Theme.accent }
            }
            ColumnLayout {
                id: heroText
                Layout.fillWidth: true
                spacing: 3
                Text { text: "Steam Game Mode"; color: Theme.text; font.family: Theme.font; font.pixelSize: 19; font.weight: Font.DemiBold }
                Text { Layout.fillWidth: true; text: "Steam's big-screen mode with the Hearth Sync plugin. Press the Game Mode button any time."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap }
            }
            Rectangle {
                implicitWidth: gmRow.implicitWidth + 32; implicitHeight: 38
                radius: Math.max(6, Theme.r(8))
                color: Theme.accent
                Row {
                    id: gmRow
                    anchors.centerIn: parent
                    spacing: 8
                    Icon { anchors.verticalCenter: parent.verticalCenter; name: "pad"; size: 18; color: Theme.accentInk }
                    Text { anchors.verticalCenter: parent.verticalCenter; text: "Switch to Game Mode"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                }
            }
        }
    }

    SetHeader { text: "Game Mode" }
    StartInChoice {}
    SetLinkCard {
        target: "steam"
        title: "Decky Loader and Hearth Sync"
        sub: "Plugins in Game Mode's Quick Access menu"
        chip: page.steam && page.steam.decky ? (page.steam.decky.installed ? "Installed" : "Not installed") : ""
        chipKind: page.steam && page.steam.decky && page.steam.decky.installed ? "ok" : "off"
    }
    SetLinkCard { target: "startup-animation"; value: page.movieLabel() }

    SetHeader { text: "Your games" }
    SetLinkCard { target: "games" }
    SetLinkCard { target: "remote" }
    SetLinkCard { target: "storage"; title: "Game storage"; sub: "Games by size, moving games between drives, low space warning · System › Storage" }

    SetHeader { text: "Controls & performance" }
    SetLinkCard { target: "controller" }
    SetLinkCard { target: "handheld"; value: page.hhd && page.hhd.running === false ? "No handheld on this PC" : "" }
    SetSwitch {
        visible: !!page.hsCfg
        icon: "server"; title: "Light mode while you play"
        sub: "Home server jobs wait while a game runs. Sharing keeps working."
        checked: !!page.hsCfg && page.hsCfg.lightMode !== false
        onToggled: v => hsJob.run(["set", "lightMode", v ? "on" : "off"], () => hsJob.run(["status"], r => page.hs = r, () => {}), e => SettingsNav.toast(e))
    }
}
