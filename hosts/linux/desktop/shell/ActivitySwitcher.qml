// The activity switcher (Activities.qml): a card for each activity (its wallpaper, name, how many windows and
// widgets are on it) and New activity. Opens from the bar's activity button, Quick Access, Super+A (labwc / KWin
// key bindings call `ipc call activities switcher`) and with the controller.
//   ← → / LB RB   pick          A / Enter   switch to it (on New activity: make one)
//   X / Menu key  change it: name, icon, wallpaper, its own pinned apps, delete     B / Esc   close
// Everything here is something you choose; each change merges one activity's key into activities.json.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import "activities.js" as A

PanelWindow {
    id: sw
    property bool shown: false
    function open() { at = Math.max(0, Activities.list.findIndex(a => a.id === Activities.current)); editing = false; shown = true; }
    function close() { shown = false; editing = false; }
    Component.onCompleted: Activities.switcher = sw
    screen: Quickshell.screens.length ? Quickshell.screens[0] : null
    visible: shown
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-activities"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    readonly property var acts: Activities.list
    readonly property int count: acts.length + 1          // + New activity
    property int at: 0
    property bool editing: false
    property int editRow: 0                                // 0 name, 1 icon, 2 wallpaper, 3 pins, 4 delete
    property int editCol: 0
    readonly property var sel: at < acts.length ? acts[at] : null
    readonly property var walls: ["", "builtin:slate-dusk", "builtin:slate-horizon", "builtin:slate-nebula", "builtin:slate-facets", "anim:lava", "anim:aurora"]
    onShownChanged: if (shown) keys.forceActiveFocus()
    onEditingChanged: { editRow = 0; editCol = 0; if (editing) nameField.text = sel ? sel.name : ""; else keys.forceActiveFocus(); }
    function wallLabel(w) { return w === "" ? "Desktop's own" : w.startsWith("anim:") ? w.slice(5).replace(/^./, c => c.toUpperCase()) : w.startsWith("builtin:") ? w.slice(8).replace("slate-", "Slate ").replace(/\b./g, c => c.toUpperCase()) : w.split("/").pop(); }
    function wallUrl(w) { return w.startsWith("builtin:") ? Qt.resolvedUrl("images/wallpapers/" + w.slice(8) + ".jpg") : w.startsWith("/") ? "file://" + w : ""; }
    function widgetsOn(id) { void WidgetStore.rev; return WidgetStore.widgets.filter(w => w.activity === id).length; }
    function choose() {
        if (at >= acts.length) { const id = Activities.create("", ""); at = Math.max(0, Activities.list.findIndex(a => a.id === id)); editing = true; return; }
        Activities.switchTo(sel.id);
        close();
    }

    Item {
        id: keys
        anchors.fill: parent
        focus: true
        Keys.onPressed: event => {
            const k = event.key;
            if (sw.editing) {
                if (k === Qt.Key_Escape || k === Qt.Key_Menu) sw.editing = false;
                else if (k === Qt.Key_Up) sw.editRow = Math.max(0, sw.editRow - 1);
                else if (k === Qt.Key_Down) sw.editRow = Math.min(sw.sel && sw.sel.id !== "default" ? 4 : 3, sw.editRow + 1);
                else if (k === Qt.Key_Left) sw.editCol = Math.max(0, sw.editCol - 1);
                else if (k === Qt.Key_Right) sw.editCol = Math.min(sw.editRow === 1 ? A.ICONS.length - 1 : sw.editRow === 2 ? sw.walls.length - 1 : 0, sw.editCol + 1);
                else if (k === Qt.Key_Return || k === Qt.Key_Enter) {
                    if (!sw.sel) return;
                    if (sw.editRow === 0) nameField.input.forceActiveFocus();
                    else if (sw.editRow === 1) Activities.setIcon(sw.sel.id, A.ICONS[sw.editCol]);
                    else if (sw.editRow === 2) Activities.setWallpaper(sw.sel.id, sw.walls[sw.editCol]);
                    else if (sw.editRow === 3) Activities.setOwnPins(sw.sel.id, !Array.isArray(sw.sel.pins));
                    else if (sw.editRow === 4) { Activities.remove(sw.sel.id); sw.editing = false; sw.at = 0; }
                }
                else return;
                event.accepted = true;
                return;
            }
            if (k === Qt.Key_Escape) sw.close();
            else if (k === Qt.Key_Left || k === Qt.Key_Backtab) sw.at = (sw.at + sw.count - 1) % sw.count;
            else if (k === Qt.Key_Right || k === Qt.Key_Tab) sw.at = (sw.at + 1) % sw.count;
            else if (k === Qt.Key_Return || k === Qt.Key_Enter || k === Qt.Key_Space) sw.choose();
            else if (k === Qt.Key_Menu || k === Qt.Key_F2) { if (sw.sel) sw.editing = true; }
            else return;
            event.accepted = true;
        }
    }
    Rectangle { anchors.fill: parent; color: Qt.rgba(0, 0, 0, 0.5); MouseArea { anchors.fill: parent; onClicked: sw.close() } }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, cardsRow.implicitWidth + 20)
        spacing: 22
        Text { Layout.alignment: Qt.AlignHCenter; text: "Activities"; color: "white"; font.family: Theme.font; font.pixelSize: 30; font.weight: Font.Bold }
        Text { Layout.alignment: Qt.AlignHCenter; text: "Each has its own windows, wallpaper, widgets and pinned apps"; color: "#dddddd"; font.family: Theme.font; font.pixelSize: 14 }
        Flickable {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: Math.min(sw.width - 100, cardsRow.implicitWidth)
            Layout.preferredHeight: 236
            contentWidth: cardsRow.implicitWidth
            clip: true
            Row {
                id: cardsRow
                spacing: 18
                Repeater {
                    model: sw.count
                    delegate: Item {
                        id: card
                        required property int index
                        readonly property var a: index < sw.acts.length ? sw.acts[index] : null
                        readonly property bool cur: index === sw.at
                        readonly property bool on: a !== null && a.id === Activities.current
                        width: 280; height: 230
                        Rectangle {
                            anchors.fill: parent
                            radius: Theme.r(18)
                            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.96)
                            border.width: card.cur ? 3 : 1
                            border.color: card.cur ? Theme.accent : Theme.line
                            scale: card.cur && !Settings.reduceMotion ? 1.04 : 1
                            Behavior on scale { NumberAnimation { duration: 120 } }
                            Rectangle {   // the wallpaper
                                id: thumb
                                anchors { left: parent.left; right: parent.right; top: parent.top; margins: 10 }
                                height: 130
                                radius: Theme.r(12)
                                clip: true
                                gradient: Gradient {
                                    GradientStop { position: 0; color: card.a && card.a.wallpaper === "anim:lava" ? "#5a1408" : card.a && card.a.wallpaper === "anim:aurora" ? "#0e1a2a" : Theme.bg2 }
                                    GradientStop { position: 1; color: card.a && card.a.wallpaper === "anim:lava" ? "#f07a2a" : card.a && card.a.wallpaper === "anim:aurora" ? "#3fae8a" : Theme.accent }
                                }
                                Image { anchors.fill: parent; visible: card.a !== null && status === Image.Ready; source: card.a ? sw.wallUrl(card.a.wallpaper) : ""; fillMode: Image.PreserveAspectCrop; sourceSize.width: 400; asynchronous: true }
                                Icon { anchors.centerIn: parent; visible: card.a === null; name: "plus"; size: 48; color: "white" }
                                Rectangle {
                                    visible: card.on
                                    anchors { top: parent.top; right: parent.right; margins: 8 }
                                    width: nowT.implicitWidth + 16; height: 22; radius: 11; color: Theme.accent
                                    Text { id: nowT; anchors.centerIn: parent; text: "You're here"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 11; font.bold: true }
                                }
                            }
                            RowLayout {
                                anchors { left: parent.left; right: parent.right; top: thumb.bottom; margins: 14; topMargin: 12 }
                                spacing: 10
                                Rectangle {
                                    width: 40; height: 40; radius: 20
                                    color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.2)
                                    Icon { anchors.centerIn: parent; name: card.a ? card.a.icon : "plus"; size: 22; color: Theme.accent }
                                }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 1
                                    Text { Layout.fillWidth: true; text: card.a ? card.a.name : "New activity"; color: Theme.text; font.family: Theme.font; font.pixelSize: 18; font.weight: Font.DemiBold; elide: Text.ElideRight }
                                    Text {
                                        Layout.fillWidth: true
                                        text: card.a ? Activities.windowsOn(card.a.id) + " windows · " + sw.widgetsOn(card.a.id) + " widgets" + (Array.isArray(card.a.pins) ? " · own apps" : "") : "Work, Games, Study…"
                                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight
                                    }
                                }
                            }
                            MouseArea {
                                anchors.fill: parent
                                hoverEnabled: true
                                acceptedButtons: Qt.LeftButton | Qt.RightButton
                                onEntered: if (!sw.editing) sw.at = card.index
                                onClicked: mouse => { sw.at = card.index; if (mouse.button === Qt.RightButton) { if (sw.sel) sw.editing = true; } else sw.choose(); }
                            }
                        }
                    }
                }
            }
        }

        // ---- change the picked activity
        Rectangle {
            visible: sw.editing && sw.sel !== null
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: 760
            Layout.preferredHeight: editBox.implicitHeight + 36
            radius: Theme.r(18)
            color: Qt.rgba(Theme.panel.r, Theme.panel.g, Theme.panel.b, 0.97)
            border.width: 1; border.color: Theme.line
            MouseArea { anchors.fill: parent }
            ColumnLayout {
                id: editBox
                anchors { fill: parent; margins: 18 }
                spacing: 12
                RowLayout {
                    spacing: 10
                    Text { Layout.preferredWidth: 120; text: "Name"; color: sw.editRow === 0 ? Theme.accent : Theme.muted; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                    WField {
                        id: nameField
                        Layout.preferredWidth: 360
                        onAccepted: { if (sw.sel) Activities.rename(sw.sel.id, text); keys.forceActiveFocus(); }
                    }
                }
                RowLayout {
                    spacing: 10
                    Text { Layout.preferredWidth: 120; text: "Icon"; color: sw.editRow === 1 ? Theme.accent : Theme.muted; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                    Repeater {
                        model: A.ICONS
                        delegate: WBtn {
                            required property var modelData
                            required property int index
                            icon: modelData; size: 36
                            on: sw.sel && sw.sel.icon === modelData
                            border.width: sw.editRow === 1 && sw.editCol === index ? Theme.focusWidth : 0
                            onClicked: if (sw.sel) Activities.setIcon(sw.sel.id, modelData)
                        }
                    }
                }
                RowLayout {
                    spacing: 10
                    Text { Layout.preferredWidth: 120; text: "Wallpaper"; color: sw.editRow === 2 ? Theme.accent : Theme.muted; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                    Repeater {
                        model: sw.walls
                        delegate: Rectangle {
                            required property var modelData
                            required property int index
                            width: 76; height: 50; radius: Theme.r(8)
                            clip: true
                            gradient: Gradient {
                                GradientStop { position: 0; color: modelData === "anim:lava" ? "#5a1408" : modelData === "anim:aurora" ? "#0e1a2a" : Theme.bg2 }
                                GradientStop { position: 1; color: modelData === "anim:lava" ? "#f07a2a" : modelData === "anim:aurora" ? "#3fae8a" : Theme.accent }
                            }
                            border.width: (sw.sel && sw.sel.wallpaper === modelData) || (sw.editRow === 2 && sw.editCol === index) ? 3 : 0
                            border.color: sw.editRow === 2 && sw.editCol === index ? "white" : Theme.accent
                            Image { anchors.fill: parent; anchors.margins: parent.border.width; source: sw.wallUrl(modelData); visible: status === Image.Ready; fillMode: Image.PreserveAspectCrop; sourceSize.width: 160; asynchronous: true }
                            Text { anchors { bottom: parent.bottom; horizontalCenter: parent.horizontalCenter; bottomMargin: 2 } text: sw.wallLabel(modelData); color: "white"; font.family: Theme.font; font.pixelSize: 9; style: Text.Outline; styleColor: "#aa000000"; width: parent.width - 4; elide: Text.ElideRight; horizontalAlignment: Text.AlignHCenter }
                            MouseArea { anchors.fill: parent; onClicked: if (sw.sel) Activities.setWallpaper(sw.sel.id, modelData) }
                        }
                    }
                }
                RowLayout {
                    spacing: 10
                    Text { Layout.preferredWidth: 120; text: "Pinned apps"; color: sw.editRow === 3 ? Theme.accent : Theme.muted; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                    WBtn { text: "The taskbar's own"; on: sw.sel && !Array.isArray(sw.sel.pins); border.width: sw.editRow === 3 ? Theme.focusWidth : 0; onClicked: if (sw.sel) Activities.setOwnPins(sw.sel.id, false) }
                    WBtn { text: "Its own"; on: sw.sel && Array.isArray(sw.sel.pins); onClicked: if (sw.sel) Activities.setOwnPins(sw.sel.id, true) }
                    Text { text: "Pin and unpin while you're on it"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                }
                RowLayout {
                    visible: sw.sel && sw.sel.id !== "default"
                    spacing: 10
                    Text { Layout.preferredWidth: 120; text: ""; }
                    WBtn { text: "Delete this activity"; icon: "trash"; border.width: sw.editRow === 4 ? Theme.focusWidth : 0; onClicked: { Activities.remove(sw.sel.id); sw.editing = false; sw.at = 0; } }
                    Text { text: "Its windows and widgets move to Main"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                }
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 16
            Repeater {
                model: sw.editing ? [{ b: "D-pad", c: "#6c7480", t: "Choose" }, { b: "A", c: "#3fae5a", t: "Set" }, { b: "B", c: "#d9483b", t: "Back" }]
                                  : [{ b: "LB RB", c: "#6c7480", t: "Pick" }, { b: "A", c: "#3fae5a", t: "Switch" }, { b: "X", c: "#3c7be0", t: "Change" }, { b: "B", c: "#d9483b", t: "Close" }]
                delegate: RowLayout {
                    required property var modelData
                    spacing: 5
                    Rectangle { width: Math.max(24, bl.implicitWidth + 10); height: 24; radius: 12; color: modelData.c; Text { id: bl; anchors.centerIn: parent; text: modelData.b; color: "white"; font.family: Theme.font; font.pixelSize: modelData.b.length > 2 ? 9 : 12; font.bold: true } }
                    Text { text: modelData.t; color: "#dddddd"; font.family: Theme.font; font.pixelSize: 13 }
                }
            }
        }
    }
}
