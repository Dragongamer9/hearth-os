// Hearth Settings › Notifications: Do Not Disturb, clearing the list, and each app (muting from a pop-up's bell button
// still works and shows here as Pop-ups off; muted apps' notifications still go to the list in the clock's panel). 1.14: each app's own settings
// (allowed, pop-ups, a sound, on the lock screen, priority): hearth-notify writes them (notify-apps.json) only when you
// change one; pop-ups stay desk.json's muted list as before. 1.14 (round 2): quiet by itself (a full-screen app, sharing
// the screen, a schedule; NotifyHub.qml, notify.json, written only when you change one), the history kept across
// restarts, and Hearth's own warnings, which nothing here can block.
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
            visible: NotifyHub.autoReason !== "" && !DeskState.dnd
            icon: "moon"; title: "Quiet right now"; chevron: false; interactive: false
            sub: "Pop-ups are quiet " + NotifyHub.autoLabel + (NotifyHub.missed ? ". " + NotifyHub.missed + " waiting in the list" : "") + ". Your Do not disturb switch isn't changed"
        }
        SetRow {
            icon: "bell"; title: "Clear the notification list"
            sub: DeskState.history.length === 1 ? "1 notification" : DeskState.history.length + " notifications"
            chevron: false
            onActivated: DeskState.clearHistory()
        }
    }
    // ------------------------------------------------ quiet by itself, the history (1.14)
    readonly property var times: { const o = []; for (let h = 0; h < 24; h++) for (const m of ["00", "30"]) { const v = (h < 10 ? "0" : "") + h + ":" + m; o.push({ value: v, label: v }); } return o; }
    SetCardGroup {
        title: "Quiet by itself"
        SetSwitch {
            icon: "pad"; title: "While a full-screen app is in front"
            sub: "Games and videos: pop-ups wait in the list, and you're told how many came in when you leave full screen"
            checked: NotifyHub.pref("autoFullscreen")
            onToggled: v => NotifyHub.setPref("autoFullscreen", v)
        }
        SetSwitch {
            icon: "share"; title: "While you share or record your screen"
            sub: "Screen sharing in a call, Hearth's recorder, Remote desktop: nobody else sees your notifications"
            checked: NotifyHub.pref("autoSharing")
            onToggled: v => NotifyHub.setPref("autoSharing", v)
        }
        SetSwitch {
            icon: "clock"; title: "On a schedule"
            sub: "Every day between the times below. Press Super+Alt+N any time for Do not disturb"
            checked: NotifyHub.pref("schedule")
            onToggled: v => NotifyHub.setPref("schedule", v)
        }
        SetDropdown {
            visible: NotifyHub.pref("schedule")
            icon: "moon"; title: "From"; boxWidth: 140
            options: page.times; current: NotifyHub.pref("scheduleFrom")
            onChosen: v => NotifyHub.setPref("scheduleFrom", v)
        }
        SetDropdown {
            visible: NotifyHub.pref("schedule")
            icon: "sun"; title: "Until"; boxWidth: 140
            options: page.times; current: NotifyHub.pref("scheduleTo")
            onChosen: v => NotifyHub.setPref("scheduleTo", v)
        }
        SetRow {
            icon: "warn"; title: "Hearth's own warnings always show"; chevron: false; interactive: false
            sub: "Low space, a low battery, a settings file put back: they pop up even when it's quiet, and can't be muted"
        }
    }
    SetCardGroup {
        title: "History"
        SetSwitch {
            icon: "list"; title: "Keep the list after a restart"
            sub: "The last 100, with their buttons while their app can still answer. Off: the list starts empty each time (what's kept now is cleared)"
            checked: NotifyHub.pref("keepHistory")
            onToggled: v => NotifyHub.setPref("keepHistory", v)
        }
        SetRow {
            icon: "vol"; title: "Sounds"; sub: "Hearth's own short sounds, the freedesktop set, or none, and which events play one"
            chevron: true
            onActivated: SettingsNav.openPage("sounds")
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
                    visible: !NotifyRules.isHearth(ax.modelData)
                    title: "Notifications"; sub: "Off: nothing from this app is shown or kept, urgent ones neither"
                    checked: ax.r.allow
                    onToggled: v => page.setRule(ax.modelData, "allow", v ? "on" : "off")
                }
                SetRow {
                    visible: NotifyRules.isHearth(ax.modelData)
                    interactive: false; chevron: false
                    title: "Always allowed"; sub: "Hearth's own messages (warnings, pairing requests, settings put back) can't be turned off; its urgent ones always pop up"
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
