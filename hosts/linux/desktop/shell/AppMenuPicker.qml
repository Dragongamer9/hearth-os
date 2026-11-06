// Settings › Appearance › Bar and dock › App menu: the five app menus as picture cards (Hearth, Centred, Tiles,
// Start screen, Classic). What opens with the Hearth flame, the Super key or the controller's Home button; search,
// pins and recent items carry over. A component of its own so the coming Settings redesign can move it as it is.
// Picking one writes only Settings.appMenu. Tiles and Classic are drawn for one full-width bar at the bottom, so
// picking either asks "Match the menu" (One main bar at the bottom, the floating app menu off) or "Keep mine"
// (nothing else changes); nothing changes without that click.
import QtQuick
import QtQuick.Layouts

ColumnLayout {
    id: amp
    Layout.fillWidth: true
    spacing: 10
    property string asking: ""                 // the menu just picked that asks about the bar ("" = not asking)
    readonly property bool barMatches: Theme.oneBar && Theme.barEdge === "bottom" && !Theme.dockFloats
    readonly property var menus: [
        { value: "hearth", label: "Hearth", desc: "Big centred panel with Apps and Games tabs" },
        { value: "centred", label: "Centred", desc: "Pinned apps and Recommended, above the centred dock" },
        { value: "tiles", label: "Tiles", desc: "App list and live tiles in the corner of a full-width bar" },
        { value: "start", label: "Start screen", desc: "Full-screen live tiles; easiest with a controller" },
        { value: "classic", label: "Classic", desc: "Two columns with your places; glassy, from the corner" }
    ]
    function choose(v) {
        Settings.appMenu = v;
        asking = (v === "tiles" || v === "classic") && !barMatches ? v : "";
    }
    function match() {
        Settings.barLayout = "one";
        Settings.barEdge = "bottom";
        Settings.dockFloat = "off";
        asking = "";
    }

    Text {
        Layout.fillWidth: true
        text: "What opens with the Hearth button, the Super key or the controller's Home button. Search, pins and recent items carry over when you switch." + (Settings.appMenu === "" && Theme.appMenuDefault === "start" ? " (Start screen is this handheld's own.)" : "")
        wrapMode: Text.WordWrap; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
    }
    GridLayout {
        Layout.fillWidth: true
        columns: amp.width > 760 ? 5 : amp.width > 480 ? 3 : 2
        columnSpacing: 10
        rowSpacing: 10
        Repeater {
            model: amp.menus
            delegate: MenuCard {}
        }
    }
    // "Bar for this menu": only right after picking Tiles or Classic with a different bar.
    SetBase {
        id: ask
        visible: amp.asking !== ""
        Layout.fillWidth: true
        implicitHeight: askCol.implicitHeight + 28
        usesLeftRight: true
        property int pick: 0
        onAdjusted: d => pick = d > 0 ? 1 : 0
        onActivated: pick === 0 ? amp.match() : (amp.asking = "")
        ColumnLayout {
            id: askCol
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 14; leftMargin: 16; rightMargin: 16 }
            spacing: 10
            Text { text: "Bar to go with the menu"; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
            Text {
                Layout.fillWidth: true; wrapMode: Text.WordWrap
                text: (amp.asking === "tiles" ? "Tiles" : "Classic") + " is drawn for one full-width bar at the bottom with the app menu button on the left. Match the menu: One main bar at the bottom, floating app menu off. Keep mine: your bar stays as it is."
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
            }
            RowLayout {
                spacing: 8
                AskButton { label: "Match the menu"; primary: true; lit: ask.pick === 0; onPicked: amp.match() }
                AskButton { label: "Keep mine"; lit: ask.pick === 1; onPicked: amp.asking = "" }
            }
        }
    }

    component AskButton: Rectangle {
        id: ab
        property string label
        property bool primary: false
        property bool lit: false
        signal picked()
        implicitWidth: abt.implicitWidth + 32; implicitHeight: 38
        radius: Theme.r(10)
        color: primary ? (aba.containsMouse ? Qt.lighter(Theme.accent, 1.1) : Theme.accent) : (aba.containsMouse ? Theme.panel3 : Theme.panel3)
        border.width: lit ? 2 : 0; border.color: Theme.text
        Text { id: abt; anchors.centerIn: parent; text: ab.label; color: ab.primary ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
        MouseArea { id: aba; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: ab.picked() }
    }

    // A menu as a picture card: a sketch of it in your colours, its name and what it's like.
    component MenuCard: SetBase {
        id: card
        required property var modelData
        readonly property string v: modelData.value
        readonly property bool lit: Theme.appMenu === v
        Layout.fillWidth: true
        Layout.preferredWidth: 160
        implicitHeight: 196
        border.color: activeFocus ? Theme.accent : lit ? Qt.lighter(Theme.accent, 1.2) : "transparent"
        onActivated: amp.choose(v)
        Rectangle {
            id: pic
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: 6 }
            height: 100
            radius: Theme.r(8)
            clip: true
            color: Theme.bg2
            Image { anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom } width: parent.width * 1.4; height: parent.height; source: Qt.resolvedUrl("images/ember-glow.png"); opacity: 0.35 }
            // Hearth: the big centred panel with tabs
            Rectangle { visible: card.v === "hearth"; anchors.centerIn: parent; width: parent.width * 0.74; height: parent.height * 0.74; radius: 6; color: Theme.panel2
                Row { x: 8; y: 7; spacing: 3; Rectangle { width: 18; height: 5; radius: 2; color: Theme.accent } Rectangle { width: 18; height: 5; radius: 2; color: Theme.panel3 } }
                Grid { x: 8; y: 18; columns: 6; spacing: 5; Repeater { model: 12; delegate: Rectangle { width: 9; height: 9; radius: 2; color: Theme.panel3 } } }
                Rectangle { x: 8; anchors.bottom: parent.bottom; anchors.bottomMargin: 6; width: 26; height: 7; radius: 3; color: Theme.accent } }
            // Centred: pinned grid + recommended over a centred dock
            Rectangle { visible: card.v === "centred"; anchors.horizontalCenter: parent.horizontalCenter; y: 8; width: parent.width * 0.56; height: parent.height - 26; radius: 6; color: Theme.panel2
                Rectangle { x: 6; y: 5; width: parent.width - 12; height: 6; radius: 3; color: Theme.panel3 }
                Grid { x: 7; y: 16; columns: 6; spacing: 4; Repeater { model: 12; delegate: Rectangle { width: 7; height: 7; radius: 2; color: Theme.accent; opacity: 0.4 + (index % 3) * 0.2 } } }
                Column { x: 7; y: 44; spacing: 3; Repeater { model: 2; delegate: Rectangle { width: 44; height: 4; radius: 2; color: Theme.panel3 } } } }
            Rectangle { visible: card.v === "centred"; anchors { horizontalCenter: parent.horizontalCenter; bottom: parent.bottom; bottomMargin: 4 } width: 56; height: 9; radius: 4; color: Theme.panel3 }
            // Tiles: rail + list + tiles from the corner of a bottom bar
            Rectangle { visible: card.v === "tiles" || card.v === "classic"; anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 9; color: Theme.panel3
                Rectangle { x: 2; y: 1.5; width: 6; height: 6; radius: 3; color: Theme.flame } }
            Rectangle { visible: card.v === "tiles"; x: 4; y: 10; width: parent.width * 0.7; height: parent.height - 22; radius: 3; color: Theme.panel2
                Column { x: 5; y: 6; spacing: 4; Repeater { model: 6; delegate: Rectangle { width: 22; height: 4; radius: 2; color: Theme.panel3 } } }
                Grid { x: 34; y: 6; columns: 3; spacing: 3; Repeater { model: 9; delegate: Rectangle { width: index % 4 === 0 ? 23 : 10; height: 10; color: ["#d9702a", "#2f7fbf", "#6b4fc8", "#1f9a8a", "#c8414b", "#2f9e57"][index % 6] } } } }
            // Start screen: full screen of coloured tiles
            Grid { visible: card.v === "start"; x: 8; y: 18; columns: 7; spacing: 3
                Repeater { model: 21; delegate: Rectangle { width: 16; height: 16; color: ["#d9702a", "#2f7fbf", "#6b4fc8", "#1f9a8a", "#c8414b", "#2f9e57", "#c99316"][(index * 3) % 7] } } }
            Text { visible: card.v === "start"; x: 8; y: 3; text: "Start"; color: Theme.text; font.family: Theme.font; font.pixelSize: 10 }
            // Classic: two columns, glassy
            Rectangle { visible: card.v === "classic"; x: 4; y: 14; width: parent.width * 0.55; height: parent.height - 26; radius: 4
                gradient: Gradient { GradientStop { position: 0; color: Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.55) } GradientStop { position: 1; color: Theme.panel2 } }
                border.color: Qt.rgba(1, 1, 1, 0.3)
                Rectangle { x: 3; y: 3; width: parent.width * 0.55; height: parent.height - 6; radius: 2; color: Theme.panel
                    Column { x: 4; y: 4; spacing: 4; Repeater { model: 6; delegate: Rectangle { width: 26; height: 4; radius: 2; color: Theme.panel3 } } } }
                Column { x: parent.width * 0.62; y: 8; spacing: 4; Repeater { model: 5; delegate: Rectangle { width: 16; height: 3; radius: 1.5; color: Qt.rgba(1, 1, 1, 0.6) } } } }
        }
        Rectangle {   // "In use", on the picture's corner
            visible: card.lit
            anchors { right: pic.right; top: pic.top; margins: 6 }
            implicitWidth: inUse.implicitWidth + 14; implicitHeight: 20; radius: 10
            color: Theme.accent
            Text { id: inUse; anchors.centerIn: parent; text: "In use"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 10; font.bold: true }
        }
        ColumnLayout {
            anchors { left: parent.left; right: parent.right; top: pic.bottom; margins: 10; topMargin: 8 }
            spacing: 2
            Text { Layout.fillWidth: true; text: card.modelData.label; elide: Text.ElideRight; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
            Text { Layout.fillWidth: true; text: card.modelData.desc; wrapMode: Text.WordWrap; maximumLineCount: 3; elide: Text.ElideRight; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
        }
    }
}
