// Add widgets: the gallery (the desktop's right-click menu, Quick Access, `ipc call widgets gallery`). Every widget
// Hearth has, as a card; click Add (or the D-pad and A) to put it on this screen's desktop at a free place, or Add to
// bar (X) for the ones that fit in the bar. Arrange (LB / RB to its button, or the button) moves the ones you have.
// Keys: arrows move, Enter adds, the Menu key (X) adds to the bar, Tab goes to the buttons at the bottom, Esc closes.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Wayland
import "widget-layout.js" as L

PanelWindow {
    id: g
    property bool shown: false
    signal addRequested(string type, bool toBar)
    signal closeRequested()
    signal arrangeRequested()
    visible: shown
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.namespace: "hearth-widget-gallery"
    WlrLayershell.keyboardFocus: shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
    exclusionMode: ExclusionMode.Ignore
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"

    readonly property var cards: L.CATALOG
    readonly property int cols: width > 1500 ? 5 : width > 1100 ? 4 : 3
    property int at: 0
    property string flash: ""
    readonly property var mine: { void WidgetStore.rev; const o = {}; for (const w of WidgetStore.widgets) o[w.type] = (o[w.type] || 0) + 1; return o; }
    onShownChanged: if (shown) { at = 0; keys.forceActiveFocus(); }
    function add(i, bar) {
        const c = cards[i];
        if (!c || (bar && !c.bar)) return;
        addRequested(c.type, bar);
        flash = c.type;
        flashT.restart();
    }
    Timer { id: flashT; interval: 900; onTriggered: g.flash = "" }

    Rectangle { anchors.fill: parent; color: Qt.rgba(0, 0, 0, 0.45); MouseArea { anchors.fill: parent; onClicked: g.closeRequested() } }
    Item {
        id: keys
        anchors.fill: parent
        focus: true
        Keys.onPressed: event => {
            const k = event.key;
            if (k === Qt.Key_Escape) g.closeRequested();
            else if (k === Qt.Key_Left) g.at = L.galleryMove(g.at, -1, 0, g.cols, g.cards.length);
            else if (k === Qt.Key_Right) g.at = L.galleryMove(g.at, 1, 0, g.cols, g.cards.length);
            else if (k === Qt.Key_Up) g.at = L.galleryMove(g.at, 0, -1, g.cols, g.cards.length);
            else if (k === Qt.Key_Down) g.at = L.galleryMove(g.at, 0, 1, g.cols, g.cards.length);
            else if (k === Qt.Key_Return || k === Qt.Key_Enter || k === Qt.Key_Space) g.add(g.at, false);
            else if (k === Qt.Key_Menu || k === Qt.Key_B) g.add(g.at, true);
            else if (k === Qt.Key_Tab || k === Qt.Key_Backtab) g.arrangeRequested();
            else return;
            event.accepted = true;
        }
    }

    Item {
        id: panel
        anchors.centerIn: parent
        width: Math.min(parent.width - 80, g.cols * 250 + 60)
        height: Math.min(parent.height - 80, head.height + gridV.contentHeight + foot.height + 70)
        Frame { anchors.fill: parent; rad: Theme.r(22); solid: true }
        MouseArea { anchors.fill: parent }          // clicks on the panel don't close it
        ColumnLayout {
            anchors { fill: parent; margins: 26 }
            spacing: 14
            ColumnLayout {
                id: head
                Layout.fillWidth: true
                spacing: 2
                Text { text: "Add widgets"; color: Theme.text; font.family: Theme.font; font.pixelSize: 28; font.weight: Font.Bold }
                Text {
                    text: "To this screen's desktop" + (Activities.many ? " on " + Activities.name : "") + ". Hover a widget on the desktop to move, resize or remove it."
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 14
                }
            }
            GridView {
                id: gridV
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                cellWidth: Math.floor(width / g.cols)
                cellHeight: 196
                model: g.cards
                currentIndex: g.at
                delegate: Item {
                    id: card
                    required property var modelData
                    required property int index
                    readonly property bool cur: index === g.at
                    width: gridV.cellWidth; height: gridV.cellHeight
                    Rectangle {
                        anchors { fill: parent; margins: 6 }
                        radius: Theme.r(16)
                        color: card.cur || cm.containsMouse ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16) : Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.05)
                        border.width: card.cur ? Theme.focusWidth : 1
                        border.color: card.cur ? Theme.accent : Theme.line
                        MouseArea { id: cm; anchors.fill: parent; hoverEnabled: true; onEntered: g.at = card.index; onDoubleClicked: g.add(card.index, false) }
                        ColumnLayout {
                            anchors { fill: parent; margins: 14 }
                            spacing: 6
                            RowLayout {
                                spacing: 10
                                Rectangle {
                                    width: 46; height: 46; radius: Theme.r(12)
                                    color: g.flash === card.modelData.type ? Theme.accent : Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.2)
                                    Icon { anchors.centerIn: parent; name: card.modelData.icon; size: 26; color: g.flash === card.modelData.type ? Theme.accentInk : Theme.accent }
                                }
                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 0
                                    Text { Layout.fillWidth: true; text: card.modelData.name; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold; elide: Text.ElideRight }
                                    Text { visible: !!g.mine[card.modelData.type]; text: (g.mine[card.modelData.type] || 0) + " on your desktop"; color: Theme.accent; font.family: Theme.font; font.pixelSize: 11 }
                                }
                            }
                            Text { Layout.fillWidth: true; Layout.fillHeight: true; text: card.modelData.about; wrapMode: Text.WordWrap; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                            RowLayout {
                                spacing: 6
                                WBtn { text: g.flash === card.modelData.type ? "Added" : "Add"; on: true; size: 28; onClicked: g.add(card.index, false) }
                                WBtn { visible: card.modelData.bar; text: "Add to bar"; size: 28; onClicked: g.add(card.index, true) }
                            }
                        }
                    }
                }
            }
            RowLayout {
                id: foot
                Layout.fillWidth: true
                spacing: 10
                Repeater {
                    model: [{ b: "A", c: "#3fae5a", t: "Add" }, { b: "X", c: "#3c7be0", t: "Add to bar" }, { b: "RB", c: "#6c7480", t: "Arrange" }, { b: "B", c: "#d9483b", t: "Close" }]
                    delegate: RowLayout {
                        required property var modelData
                        spacing: 5
                        Rectangle { width: 24; height: 24; radius: 12; color: modelData.c; Text { anchors.centerIn: parent; text: modelData.b; color: "white"; font.family: Theme.font; font.pixelSize: modelData.b.length > 1 ? 9 : 12; font.bold: true } }
                        Text { text: modelData.t; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                    }
                }
                Item { Layout.fillWidth: true }
                WBtn { text: "Arrange widgets"; icon: "grid"; onClicked: g.arrangeRequested() }
                WBtn { text: "Done"; onClicked: g.closeRequested() }
            }
        }
    }
}
