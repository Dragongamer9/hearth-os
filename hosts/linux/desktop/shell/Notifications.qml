// Notification server for Desktop Mode: apps' notifications pop up under the clock pill (Mantel 2026; the top-right
// corner with the classic taskbar) and go away after a few seconds. Click one to run its default action; × removes it.
// 1.14 (the same on KWin and labwc):
//   * Every one goes into the history (DeskState.history, the clock's panel), kept across restarts, and stays alive
//     there so its buttons still work later (DeskState.invoke).
//   * Do Not Disturb (DeskState.dnd, your switch) and automatic quiet (NotifyHub.autoReason: a full-screen app in
//     front, the screen shared or recorded, a schedule) keep them from popping up; automatic quiet counts what came
//     in and says so when it's over. Muting an app (the bell on its pop-up) and each app's own settings (NotifyRules).
//   * Progress: a notification's "value" hint (a portal's or an app's download) is a bar on it; KDE's job progress
//     (org.kde.JobViewServer, NotifyHub.qml) is in the clock's panel and the bar's tray, and a finished job leaves
//     a notification here with a sound.
//   * Hearth's own warnings (NotifyRules.isHearthWarning) can never be blocked: not by an app rule, muting, Do Not
//     Disturb, automatic quiet or low priority. They always pop up and play their sound.
//   * Sounds: Sounds.qml (Settings › Sounds).
//   quickshell ipc -p <shell folder> call notify state|popups|history|fullscreen on|off|sharing on|off|test <summary> <body>
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import Quickshell.Widgets
import Quickshell.Services.Notifications

Scope {
    id: root
    function decide(n) {
        // Each app's settings (1.14, NotifyRules.qml): not allowed = not kept or shown at all (urgent ones too);
        // low priority = only the list; high = pops up even with Do Not Disturb; a sound when it pops up.
        const rule = NotifyRules.rule(n.appName);
        const critical = n.urgency === NotificationUrgency.Critical;
        const warn = NotifyRules.isHearthWarning(n);
        if (!rule.allow && !warn) { n.dismiss(); return; }
        n.tracked = true;      // kept alive while it's in the history, so its buttons still work from there
        const item = DeskState.remember(n, rule.lockScreen);
        if (!item) { n.dismiss(); return; }
        const urgent = critical || rule.priority === "high";
        let pop = true;
        if (warn) pop = true;
        else if (rule.priority === "low" && !critical) pop = false;
        else if (DeskState.isMuted(n.appName) && !urgent) pop = false;
        else if (DeskState.dnd && !urgent) pop = false;
        else if (NotifyHub.autoReason !== "" && !critical) { pop = false; NotifyHub.noteMissed(); }
        if (!pop) return;
        DeskState.popup(item.key);
        if (warn) Sounds.play(/battery/i.test(n.appIcon + " " + n.summary) ? "battery-low" : critical ? "dialog-error" : "dialog-warning");
        else if (critical && !DeskState.dnd) Sounds.play("dialog-error");
        else if (rule.sound && !NotifyHub.quiet) Sounds.play("message-new-instant");
    }
    Binding { target: DeskState; property: "keepHistory"; value: NotifyHub.pref("keepHistory") }
    NotificationServer {
        id: server
        keepOnReload: false
        actionsSupported: true
        imageSupported: true
        bodyMarkupSupported: true
        persistenceSupported: true
        onNotification: n => root.decide(n)
        // Reply boxes on pop-ups (NotifReply.qml; your phone's messages, docs/PHONE-LINK.md). Set only where this
        // Quickshell has it, so an older one still starts.
        Component.onCompleted: if ("inlineReplySupported" in server) server.inlineReplySupported = true
    }

    // ---- a finished copy or download (KDE's jobs) and automatic quiet ending: a notification of Hearth's own
    Connections {
        target: NotifyHub
        function onJobFinished(j) {
            if (j.state === "cancelled") return;
            const ok = j.state === "done";
            const e = j.entry ? AppEntries.byId(j.entry) : null;
            const key = DeskState.rememberOwn({ app: e ? e.name : (j.app || "Files"), icon: j.icon || (e ? e.icon : ""), entry: e ? j.entry : "",
                summary: ok ? (j.title || "Done") + " finished" : (j.title || "A job") + " didn't finish",
                body: ok ? (j.detail || j.dest || "") : (j.error || "It stopped with an error"), urgency: ok ? "normal" : "critical" });
            if (!NotifyHub.quiet || !ok) DeskState.popup(key);
            Sounds.play(ok ? "complete" : "dialog-error");
        }
        function onQuietEnded(count, why) {
            const key = DeskState.rememberOwn({ app: "Hearth", icon: "", summary: count === 1 ? "1 notification came in" : count + " notifications came in",
                body: "They stayed quiet " + why + ". They're in the notification centre.", urgency: "low" });
            DeskState.popup(key);
        }
    }

    PanelWindow {
        id: popups
        property bool replying: false      // typing in a pop-up's Reply box (NotifReply.qml): the keyboard is ours till then
        readonly property var keys: DeskState.popups.slice(0, 4)
        visible: keys.length > 0 && !SettingsNav.fireholdCards
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "nimbus-notifications"
        // Never takes the keyboard (or a WebKit app's controller) from the window in front: docs/CONTROLLER-FOCUS.md
        WlrLayershell.keyboardFocus: popups.replying ? WlrKeyboardFocus.OnDemand : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        // Mantel 2026: under the clock pill in the middle of the top (several stack); with the classic taskbar, the
        // top-right corner as before. With the main bar on another edge: the top-right corner, clear of it.
        anchors { top: true; right: DeskState.classicBar || DeskState.barEdge !== "top" }
        margins { top: DeskState.topInset > 0 ? DeskState.topInset + 2 : 12; right: 12 + DeskState.rightInset }
        implicitWidth: DeskState.classicBar ? 380 : 420
        implicitHeight: Math.max(1, list.implicitHeight)
        color: "transparent"

        Column {
            id: list
            width: parent.width
            spacing: 8

            Repeater {
                model: popups.keys
                delegate: Rectangle {
                    id: card
                    required property var modelData       // the history key
                    readonly property var n: DeskState.liveOf(modelData)
                    readonly property var h: { void DeskState.history; return DeskState.item(modelData); }
                    readonly property var acts: n ? Array.from(n.actions) : []
                    readonly property var defaultAction: acts.find(a => a.identifier === "default")
                    readonly property bool critical: n ? n.urgency === NotificationUrgency.Critical : (!!h && h.urgency === "critical")
                    readonly property bool warn: n ? NotifyRules.isHearthWarning(n) : false
                    readonly property string app: n ? n.appName : h ? h.app : ""
                    readonly property int progress: n ? DeskState.hintValue(n) : h ? h.progress : -1

                    width: list.width
                    visible: !!n || !!h
                    implicitHeight: visible ? row.implicitHeight + 24 : 0
                    radius: Theme.r(14)
                    color: Theme.panel
                    border.width: card.warn ? 2 : 1
                    border.color: card.critical || card.warn ? Theme.danger : Theme.line

                    // Gone after its timeout (or 6 s): it stays in the history, buttons and all. Critical ones and
                    // Hearth's warnings stay until you close them.
                    Timer {
                        running: !card.critical && !card.warn && !hover.containsMouse && !replyBox.typing
                        // expireTimeout may come in seconds or milliseconds depending on the version; accept both.
                        readonly property real t: card.n ? card.n.expireTimeout : 0
                        interval: t <= 0 ? 6000 : t > 100 ? t : t * 1000
                        onTriggered: DeskState.unpop(card.modelData)
                    }

                    MouseArea {
                        id: hover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: {
                            if (card.defaultAction) DeskState.invoke(card.modelData, "default");
                            else DeskState.unpop(card.modelData);
                        }
                    }

                    RowLayout {
                        id: row
                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12 }
                        spacing: 12

                        AppIcon {
                            Layout.alignment: Qt.AlignTop
                            size: 40
                            genericFallback: false
                            visible: hasIcon
                            source: card.n && card.n.image ? AppIcons.fromIcon(card.n.image) : card.h && card.h.image ? AppIcons.fromIcon(card.h.image) : ""
                            icon: card.n ? (card.n.appIcon || "") : card.h ? (card.h.icon || "") : ""
                            appId: card.n ? (card.n.desktopEntry || "") : card.h ? (card.h.entry || "") : ""
                        }
                        Icon { Layout.alignment: Qt.AlignTop; visible: card.warn; name: "warn"; size: 24; color: Theme.danger }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 3
                            Text { text: card.app; visible: text !== ""; color: card.warn ? Theme.danger : Theme.muted; font.pixelSize: 11; font.family: Theme.font; font.bold: card.warn }
                            Text { Layout.fillWidth: true; text: card.n ? card.n.summary : card.h ? card.h.summary : ""; color: Theme.text; font.bold: true; font.pixelSize: 14; font.family: Theme.font; elide: Text.ElideRight }
                            Text {
                                Layout.fillWidth: true
                                visible: text !== ""
                                text: card.n ? card.n.body : card.h ? card.h.body : ""
                                textFormat: card.n ? Text.StyledText : Text.PlainText
                                wrapMode: Text.Wrap
                                maximumLineCount: 4
                                elide: Text.ElideRight
                                color: Theme.text
                                opacity: 0.85
                                font.pixelSize: 13
                                font.family: Theme.font
                            }
                            // Progress (a "value" hint: a portal's or an app's download, a file being sent)
                            ProgressLine { visible: card.progress >= 0; value: card.progress }
                            // A Reply box (NotifReply.qml), for notifications that take a reply.
                            NotifReply {
                                id: replyBox
                                notification: card.n
                                onTypingChanged: popups.replying = typing
                                onSent: DeskState.forget(card.modelData, false)
                            }
                            // Other actions as buttons
                            Flow {
                                Layout.fillWidth: true
                                spacing: 6
                                visible: card.acts.some(a => a.identifier !== "default")
                                Repeater {
                                    model: card.acts.filter(a => a.identifier !== "default")
                                    delegate: ActButton {
                                        required property var modelData
                                        text: modelData.text
                                        onPicked: DeskState.invoke(card.modelData, modelData.identifier)
                                    }
                                }
                            }
                        }

                        // Mute this app: its notifications stop popping up (they still go to the list). Never on
                        // Hearth's own warnings.
                        Item {
                            Layout.alignment: Qt.AlignTop
                            implicitWidth: 18; implicitHeight: 18
                            visible: card.app !== "" && !card.warn && !NotifyRules.isHearth(card.app) && (hover.containsMouse || muteArea.containsMouse || closeArea.containsMouse)
                            Icon { anchors.centerIn: parent; name: "bell-off"; size: 16; color: muteArea.containsMouse ? Theme.text : Theme.muted }
                            MouseArea {
                                id: muteArea
                                anchors.fill: parent; anchors.margins: -6
                                hoverEnabled: true
                                onClicked: {
                                    DeskState.setMuted(card.app, true);
                                    SettingsNav.toast(card.app + " won't pop up any more. Settings › Notifications turns it back on");
                                    DeskState.unpop(card.modelData);
                                }
                            }
                        }
                        Text {
                            Layout.alignment: Qt.AlignTop
                            text: "×"
                            color: closeArea.containsMouse ? Theme.text : Theme.muted
                            font.pixelSize: 18
                            MouseArea { id: closeArea; anchors.fill: parent; anchors.margins: -6; hoverEnabled: true; onClicked: DeskState.forget(card.modelData, true) }
                        }
                    }
                }
            }
        }
    }

    component ActButton: Rectangle {
        property string text: ""
        signal picked()
        implicitWidth: actText.implicitWidth + 20
        implicitHeight: 28
        radius: Theme.r(8)
        color: actArea.containsMouse ? Theme.panel3 : Theme.panel2
        Text { id: actText; anchors.centerIn: parent; text: parent.text; color: Theme.text; font.pixelSize: 12; font.family: Theme.font }
        MouseArea { id: actArea; anchors.fill: parent; hoverEnabled: true; onClicked: parent.picked() }
    }
    component ProgressLine: Item {
        property int value: 0
        Layout.fillWidth: true
        Layout.topMargin: 4
        implicitHeight: 6
        Rectangle { anchors.fill: parent; radius: 3; color: Theme.panel3 }
        Rectangle { height: parent.height; radius: 3; width: parent.width * Math.max(0, Math.min(100, parent.value)) / 100; color: Theme.accent }
    }

    IpcHandler {
        target: "notify"
        function state(): string {
            return JSON.stringify({ dnd: DeskState.dnd, auto: NotifyHub.autoReason, quiet: NotifyHub.quiet, missed: NotifyHub.missed,
                popups: DeskState.popups.length, history: DeskState.history.length, live: Object.keys(DeskState.live).length,
                jobs: NotifyHub.jobs.length, jobServer: NotifyHub.jobServer, badges: Object.keys(NotifyHub.badges), keys: NotifyHub.keys,
                layout: NotifyHub.layoutName, disks: NotifyHub.disks.length, printJobs: NotifyHub.printJobs.length, camera: NotifyHub.cameraApps.length,
                mic: NotifyHub.micApps, background: NotifyHub.backgroundApps.length, sharing: NotifyHub.sharing });
        }
        function history(): string { return JSON.stringify(DeskState.history.map(h => ({ key: h.key, app: h.app, summary: h.summary, actions: (DeskState.liveOf(h.key) ? Array.from(DeskState.liveOf(h.key).actions).map(a => a.text) : []), restored: h.restored === true }))); }
        function popupsShown(): int { return DeskState.popups.length; }
        function unpopAll(): void { DeskState.popups = []; }
        function fullscreen(on: string): void { NotifyHub.testFullscreen = on === "on"; }
        function sharing(on: string): void { NotifyHub.testSharing = on === "on"; }
        function invoke(key: string, action: string): bool { return DeskState.invoke(Number(key), action); }
    }
}
