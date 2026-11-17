// Hearth Settings › Notifications: Do Not Disturb, clearing the list, and each app (muting from a pop-up's bell button
// still works and shows here as Pop-ups off; muted apps' notifications still go to the list in the clock's panel). 1.14: each app's own settings
// (allowed, pop-ups, a sound, on the lock screen, priority): hearth-notify writes them (notify-apps.json) only when you
// change one; pop-ups stay desk.json's muted list as before.
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
    // ------------------------------------------------ each app (1.14)
    SetJob { id: job; helper: "hearth-notify" }
    property string message: ""
    function setRule(app, key, value) {
        message = "";
        job.run(["set", app, key, value], () => {}, e => page.message = e, "", "set");
    }
    // Every app Hearth knows sent notifications: this session's, muted ones and ones with their own settings.
    readonly property var knownApps: {
        const seen = [];
        const add = a => { if (a && seen.indexOf(a) < 0) seen.push(a); };
        for (const h of DeskState.history) add(h.app);
        for (const m of DeskState.muted) add(m);
        for (const k of Object.keys(NotifyRules.apps)) add(k);
        return seen.sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase()));
    }
    function ruleText(app) {
        const r = NotifyRules.rule(app);
        if (!r.allow) return "Off: nothing from it is shown or kept";
        return [DeskState.isMuted(app) ? "No pop-ups" : "Pop-ups", r.sound ? "sound" : "", r.lockScreen ? "on the lock screen" : "",
                r.priority === "high" ? "high priority" : r.priority === "low" ? "list only" : ""].filter(x => x).join(" · ");
    }
    SetCardGroup {
        title: "Each app"
        Text {
            visible: page.knownApps.length === 0
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "Apps show here once they've sent a notification."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
        Repeater {
            model: page.knownApps
            delegate: SetExpander {
                id: ax
                required property string modelData
                readonly property var r: NotifyRules.rule(modelData)
                icon: r.allow ? "bell" : "bell-off"
                title: modelData
                sub: page.ruleText(modelData)
                SetSwitch {
                    title: "Notifications"; sub: "Off: nothing from this app is shown or kept, urgent ones neither"
                    checked: ax.r.allow
                    onToggled: v => page.setRule(ax.modelData, "allow", v ? "on" : "off")
                }
                SetSwitch {
                    visible: ax.r.allow
                    title: "Pop-ups"; sub: "Off: they only go to the list"
                    checked: !DeskState.isMuted(ax.modelData)
                    onToggled: v => DeskState.setMuted(ax.modelData, !v)
                }
                SetSwitch {
                    visible: ax.r.allow
                    title: "Play a sound"
                    checked: ax.r.sound
                    onToggled: v => page.setRule(ax.modelData, "sound", v ? "on" : "off")
                }
                SetSwitch {
                    visible: ax.r.allow
                    title: "Show on the lock screen"; sub: "Its name and the title, nothing more"
                    checked: ax.r.lockScreen
                    onToggled: v => page.setRule(ax.modelData, "lockScreen", v ? "on" : "off")
                }
                SetChoice {
                    visible: ax.r.allow
                    title: "Priority"; sub: "High pops up even with Do not disturb on; Low goes straight to the list"
                    options: [{ value: "low", label: "Low" }, { value: "normal", label: "Normal" }, { value: "high", label: "High" }]
                    current: ax.r.priority
                    onChosen: v => page.setRule(ax.modelData, "priority", v)
                }
            }
        }
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: Theme.danger; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
}
