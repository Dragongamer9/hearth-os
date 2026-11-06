// Hearth Settings › Notifications: Do Not Disturb, the apps whose notifications don't pop up (muted from a pop-up's
// bell button; they still go to the list in the clock's panel), and clearing that list.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Notifications"

    SetCardGroup {
        title: "Pop-ups and the list"
        SetSwitch {
            icon: "bell-off"; title: "Do not disturb"
            sub: "Notifications go straight to the list without popping up (urgent ones still do). Right-click the bell on the taskbar to switch it, or press Super+N for the list"
            checked: DeskState.dnd
            onToggled: v => DeskState.setDnd(v)
        }
        SetRow {
            icon: "bell"; title: "Clear the notification list"
            sub: DeskState.history.length === 1 ? "1 notification" : DeskState.history.length + " notifications"
            chevron: false
            onActivated: DeskState.clearHistory()
        }
    }
    SetCardGroup {
        title: "Muted apps"
        Text {
            visible: DeskState.muted.length === 0
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "None. To mute an app, point at one of its pop-ups and click the bell with a line through it."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
        Repeater {
            model: DeskState.muted
            delegate: SetSwitch {
                required property var modelData
                icon: "bell"; title: modelData
                sub: "Pop-ups are off; turn this on to see them again"
                checked: false
                onToggled: v => { if (v) DeskState.setMuted(modelData, false); }
            }
        }
    }
    function recentApps() {
        const seen = [];
        for (const h of DeskState.history) if (h.app && seen.indexOf(h.app) < 0 && !DeskState.isMuted(h.app)) seen.push(h.app);
        return seen.slice(0, 12);
    }
    // Apps that sent something this session and aren't muted yet, to mute without waiting for a pop-up.
    SetCardGroup {
        title: "Recently heard from"
        visible: page.recentApps().length > 0
        Repeater {
            model: page.recentApps()
            delegate: SetSwitch {
                required property var modelData
                icon: "bell"; title: modelData
                sub: "Pop-ups are on"
                checked: true
                onToggled: v => { if (!v) DeskState.setMuted(modelData, true); }
            }
        }
    }
}
