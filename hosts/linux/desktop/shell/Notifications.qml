// Notification server for Desktop Mode: apps' notifications pop up in the top-right corner
// and go away after a few seconds. Click one to run its default action; × dismisses it.
// Past ones are kept for the session in DeskState.history (listed in the clock's panel); Do Not Disturb
// (DeskState.dnd) keeps them from popping up.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import Quickshell.Widgets
import Quickshell.Services.Notifications

Scope {
    NotificationServer {
        id: server
        keepOnReload: false
        actionsSupported: true
        imageSupported: true
        bodyMarkupSupported: true
        // Every notification goes into the history (the clock's panel, ClockPanel.qml). With Do Not Disturb on
        // only urgent ones pop up.
        onNotification: n => {
            DeskState.remember(n);
            if (DeskState.dnd && n.urgency !== NotificationUrgency.Critical) return;
            n.tracked = true;
        }
    }

    PanelWindow {
        visible: server.trackedNotifications.values.length > 0
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "nimbus-notifications"
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; right: true }
        margins { top: 12; right: 12 }
        implicitWidth: 380
        implicitHeight: list.implicitHeight
        color: "transparent"

        Column {
            id: list
            width: parent.width
            spacing: 8

            Repeater {
                model: server.trackedNotifications
                delegate: Rectangle {
                    id: card
                    required property Notification modelData
                    readonly property var acts: Array.from(modelData.actions)
                    readonly property var defaultAction: acts.find(a => a.identifier === "default")

                    width: list.width
                    implicitHeight: row.implicitHeight + 24
                    radius: 14
                    color: Theme.panel
                    border.color: modelData.urgency === NotificationUrgency.Critical ? Theme.danger : Theme.line

                    // Gone after its timeout (or 6 s); critical ones stay until dismissed.
                    Timer {
                        running: card.modelData.urgency !== NotificationUrgency.Critical && !hover.containsMouse
                        // expireTimeout may come in seconds or milliseconds depending on the version; accept both.
                        interval: card.modelData.expireTimeout <= 0 ? 6000 : card.modelData.expireTimeout > 100 ? card.modelData.expireTimeout : card.modelData.expireTimeout * 1000
                        onTriggered: card.modelData.expire()
                    }

                    MouseArea {
                        id: hover
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: {
                            if (card.defaultAction) card.defaultAction.invoke();
                            else card.modelData.dismiss();
                        }
                    }

                    RowLayout {
                        id: row
                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: 12 }
                        spacing: 12

                        IconImage {
                            Layout.alignment: Qt.AlignTop
                            implicitSize: 40
                            visible: source != ""
                            source: card.modelData.image || (card.modelData.appIcon ? Quickshell.iconPath(card.modelData.appIcon, true) : "")
                        }

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 3
                            Text { text: card.modelData.appName; visible: text !== ""; color: Theme.muted; font.pixelSize: 11; font.family: Theme.font }
                            Text { Layout.fillWidth: true; text: card.modelData.summary; color: Theme.text; font.bold: true; font.pixelSize: 14; font.family: Theme.font; elide: Text.ElideRight }
                            Text {
                                Layout.fillWidth: true
                                visible: text !== ""
                                text: card.modelData.body
                                textFormat: Text.StyledText
                                wrapMode: Text.Wrap
                                maximumLineCount: 4
                                elide: Text.ElideRight
                                color: Theme.text
                                opacity: 0.85
                                font.pixelSize: 13
                                font.family: Theme.font
                            }
                            // Other actions as buttons
                            Flow {
                                Layout.fillWidth: true
                                spacing: 6
                                visible: card.acts.some(a => a.identifier !== "default")
                                Repeater {
                                    model: card.acts.filter(a => a.identifier !== "default")
                                    delegate: Rectangle {
                                        required property var modelData
                                        implicitWidth: actText.implicitWidth + 20
                                        implicitHeight: 28
                                        radius: 8
                                        color: actArea.containsMouse ? Theme.panel3 : Theme.panel2
                                        Text { id: actText; anchors.centerIn: parent; text: parent.modelData.text; color: Theme.text; font.pixelSize: 12; font.family: Theme.font }
                                        MouseArea { id: actArea; anchors.fill: parent; hoverEnabled: true; onClicked: parent.modelData.invoke() }
                                    }
                                }
                            }
                        }

                        Text {
                            Layout.alignment: Qt.AlignTop
                            text: "×"
                            color: closeArea.containsMouse ? Theme.text : Theme.muted
                            font.pixelSize: 18
                            MouseArea { id: closeArea; anchors.fill: parent; anchors.margins: -6; hoverEnabled: true; onClicked: card.modelData.dismiss() }
                        }
                    }
                }
            }
        }
    }
}
