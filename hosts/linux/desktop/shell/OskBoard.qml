// Hearth's on-screen keyboard, drawn for Desktop Mode: the same keyboard as Game Mode's and the apps' (ui/osk,
// docs/OSK.md). It draws what the keyboard's engine (osk/hearth-osk.js) says, `vm` = engine.view(), and reports taps:
// the owner (Keyboard.qml, or Hearth Files' own window in Game Mode) runs the engine and does the typing.
// Hearth's touches: the ember glow that glides to the picked key with a short trail of sparks, the Ember Wheel.
import QtQuick
import QtQuick.Layouts

Item {
    id: board
    property var vm: null
    property bool overGame: false
    signal tapped(int r, int c)
    signal longPressed(int r, int c)
    signal altPicked(int i)

    readonly property var s: vm && vm.settings ? vm.settings : ({})
    readonly property var themes: ({
        hearth: { bg: "#17171c", bg2: "#1f1f25", key: "#2b2b32", mod: "#222228", text: "#f3efe8", muted: "#a7a3ad", sel: "#f6f2ea", selText: "#141217", accent: Theme.accent, ink: Theme.accentInk },
        ember: { bg: "#120c0a", bg2: "#1d1310", key: "#2a1a14", mod: "#1f1410", text: "#ffeadb", muted: "#c9a18a", sel: "#ffd9bd", selText: "#2a1206", accent: "#ff7a2f", ink: "#1d0b02" },
        slate: { bg: "#1b2028", bg2: "#222833", key: "#2d3542", mod: "#252c37", text: "#eef2f7", muted: "#9aa6b6", sel: "#f2f6fb", selText: "#111722", accent: Theme.accent, ink: Theme.accentInk },
        daylight: { bg: "#e9e6e1", bg2: "#dedad3", key: "#ffffff", mod: "#d4d0c8", text: "#1d1b20", muted: "#6b6670", sel: "#1d1b20", selText: "#ffffff", accent: Theme.accent, ink: Theme.accentInk }
    })
    readonly property var th: themes[s.theme] || themes.hearth
    readonly property int rowCount: vm && vm.rows ? vm.rows.length : 5
    readonly property real gap: s.size === "s" ? 5 : s.size === "l" ? 8 : 7
    readonly property real keyH: Math.round(Math.max(30, Math.min(s.size === "s" ? 54 : s.size === "l" ? 86 : 72, (height - 26 - topH - gap * rowCount) / rowCount)))
    readonly property real topH: Math.round((s.size === "s" ? 46 : s.size === "l" ? 72 : 58) * 0.66)
    readonly property real fontPx: Math.round(keyH * 0.36)
    readonly property bool split: s.shape === "split" && vm && vm.panel !== "num"
    readonly property var modKeys: ["bksp", "enter", "shift", "sym", "abc", "sym2", "emoji", "clip", "layout", "left", "right", "up", "down", "hide", "tab", "esc", "ctrl", "alt", "home", "end", "paste", "prev", "next", "page", "cat"]
    property Item selItem: null
    // Where the picked key is in the keys' area (its row's place in the column, its place in the row): bound to their
    // positions, so the ember follows when the keys are laid out again.
    readonly property bool selPlaced: !!(selItem && selItem.parent && selItem.parent.parent)
    readonly property real selX: selPlaced ? selItem.x + selItem.parent.x + selItem.parent.parent.x : 0
    readonly property real selY: selPlaced ? selItem.y + selItem.parent.y + selItem.parent.parent.y : 0

    function label(k) {
        if (!vm) return "";
        if (k.t === "ch") return (vm.shift || vm.caps) && k.v !== "ß" ? k.v.toUpperCase() : k.v;
        if (k.t === "shift" && vm.caps) return "⇪";
        return k.label || "";
    }
    function glyph(k) {
        if (!vm || !vm.hints || !vm.glyphs || (k.t === "shift" && k.v === "r")) return "";
        if (s.fast === "split") { if (k.t === "shift") return "LB"; if (k.t === "sym" || k.t === "abc") return "RB"; if (k.t === "left" || k.t === "right") return ""; }
        return vm.glyphs[k.t] || "";
    }
    function halfOf(row, side) { return row.filter(k => k.half === side); }

    Rectangle {
        id: bgRect
        anchors.fill: parent
        radius: 20
        color: board.overGame ? Qt.rgba(Qt.color(board.th.bg).r, Qt.color(board.th.bg).g, Qt.color(board.th.bg).b, 0.82) : board.th.bg
        border.color: Qt.rgba(1, 1, 1, 0.06)
        Rectangle {        // the ember theme's warm glow from below
            visible: board.s.theme === "ember"
            anchors { left: parent.left; right: parent.right; bottom: parent.bottom }
            height: parent.height * 0.7
            radius: parent.radius
            gradient: Gradient { GradientStop { position: 0; color: "transparent" } GradientStop { position: 1; color: "#3a1a0c" } }
        }
    }

    ColumnLayout {
        id: col
        anchors { fill: parent; leftMargin: 12; rightMargin: 12; topMargin: 8; bottomMargin: 12 }
        spacing: board.gap

        // the strip: word suggestions, or what the controller does
        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: false
            Layout.preferredHeight: board.topH
            Layout.maximumHeight: board.topH
            spacing: 6
            Repeater {
                model: board.vm ? board.vm.suggestions : []
                delegate: Rectangle {
                    id: sug
                    required property string modelData
                    required property int index
                    readonly property bool sel: board.vm.active && board.vm.sel.r === -1 && board.vm.sel.c === index
                    Layout.fillWidth: true
                    Layout.preferredHeight: board.topH
                    radius: height / 2
                    color: sel ? board.th.sel : board.th.bg2
                    border.width: sel ? 3 : 0
                    border.color: board.th.accent
                    Text {
                        anchors.centerIn: parent
                        width: parent.width - 16
                        horizontalAlignment: Text.AlignHCenter
                        elide: Text.ElideRight
                        text: sug.modelData
                        color: sug.sel ? board.th.selText : sug.index === 0 ? board.th.accent : board.th.text
                        font.family: Theme.font; font.pixelSize: Math.round(board.fontPx * 0.9); font.weight: Font.DemiBold
                    }
                    MouseArea { anchors.fill: parent; onClicked: board.tapped(-1, sug.index) }
                }
            }
            Text {
                visible: !board.vm || !board.vm.suggestions.length
                Layout.fillWidth: true
                elide: Text.ElideRight
                color: board.th.muted
                font.family: Theme.font; font.pixelSize: 12
                text: !board.vm || !board.vm.hints ? "" : board.s.fast === "wheel" ? "Left stick: Ember Wheel · A B X Y type its letters · right stick moves the pointer, RT clicks"
                    : board.s.fast === "split" ? "Point with both sticks · LT / RT type" : "A type · X ⌫ · Y space · LT ⇧ · LB RB move · ☰ Enter · B hide · hold A for accents · RT clicks"
            }
            Rectangle {
                Layout.preferredWidth: layTxt.implicitWidth + 16
                Layout.preferredHeight: 22
                radius: 6
                color: board.th.bg2
                Text { id: layTxt; anchors.centerIn: parent; text: !board.vm ? "" : board.vm.panel === "emoji" ? "Emoji" : board.vm.panel === "clip" ? "Clipboard" : board.vm.layoutShort; color: board.th.muted; font.pixelSize: 12; font.weight: Font.Bold; font.letterSpacing: 1 }
            }
        }

        // the keys
        Item {
            id: keysArea
            Layout.fillWidth: true
            Layout.fillHeight: true
            Column {
                id: rowsCol
                width: parent.width
                spacing: board.gap
                Repeater {
                    model: board.vm ? board.vm.rows : []
                    delegate: Item {
                        id: line
                        required property var modelData
                        required property int index
                        width: rowsCol.width
                        height: board.keyH
                        Row {
                            id: fullRow
                            visible: !board.split
                            anchors.fill: parent
                            spacing: board.gap
                            readonly property real total: line.modelData.reduce((a, k) => a + k.w, 0)
                            Repeater {
                                model: board.split ? [] : line.modelData
                                delegate: OskKey { owner: board; row: line.index; key: modelData; col: index; width: (fullRow.width - board.gap * (line.modelData.length - 1)) * modelData.w / fullRow.total }
                            }
                        }
                        Row {
                            id: leftHalf
                            visible: board.split
                            anchors { left: parent.left; top: parent.top; bottom: parent.bottom }
                            width: parent.width * 0.38
                            spacing: board.gap
                            readonly property var keys: board.split ? board.halfOf(line.modelData, "l") : []
                            readonly property real total: keys.reduce((a, k) => a + k.w, 0)
                            Repeater {
                                model: leftHalf.keys
                                delegate: OskKey { owner: board; row: line.index; key: modelData; col: modelData.col; width: (leftHalf.width - board.gap * (leftHalf.keys.length - 1)) * modelData.w / leftHalf.total }
                            }
                        }
                        Row {
                            id: rightHalf
                            visible: board.split
                            anchors { right: parent.right; top: parent.top; bottom: parent.bottom }
                            width: parent.width * 0.38
                            spacing: board.gap
                            readonly property var keys: board.split ? board.halfOf(line.modelData, "r") : []
                            readonly property real total: keys.reduce((a, k) => a + k.w, 0)
                            Repeater {
                                model: rightHalf.keys
                                delegate: OskKey { owner: board; row: line.index; key: modelData; col: modelData.col; width: (rightHalf.width - board.gap * (rightHalf.keys.length - 1)) * modelData.w / rightHalf.total }
                            }
                        }
                    }
                }
            }

            // Hearth's ember: a warm glow that glides to the picked key
            Rectangle {
                id: ember
                visible: board.selPlaced && board.vm && board.vm.active && board.vm.sel.r >= 0 && !(board.vm.wheel && board.vm.wheel.active)
                x: board.selX - 3; y: board.selY - 3
                width: board.selItem ? board.selItem.width + 6 : 0
                height: board.selItem ? board.selItem.height + 6 : 0
                radius: 13
                color: "transparent"
                border.width: 3
                border.color: board.th.accent
                z: 3
                Behavior on x { enabled: board.s.trail !== false; NumberAnimation { duration: 130; easing.type: Easing.OutCubic } }
                Behavior on y { enabled: board.s.trail !== false; NumberAnimation { duration: 130; easing.type: Easing.OutCubic } }
                Behavior on width { NumberAnimation { duration: 130 } }
                Rectangle {   // the warmth under it
                    anchors.fill: parent
                    anchors.margins: -6
                    radius: 18
                    z: -1
                    color: board.th.accent
                    opacity: 0.22
                    SequentialAnimation on opacity { loops: Animation.Infinite; running: ember.visible
                        NumberAnimation { to: 0.32; duration: 1100; easing.type: Easing.InOutSine }
                        NumberAnimation { to: 0.18; duration: 1300; easing.type: Easing.InOutSine } }
                }
                onXChanged: if (board.s.trail !== false && visible) sparks.spark()
            }
            Item {
                id: sparks
                anchors.fill: parent
                z: 2
                property real lastSpark: 0
                function spark() {
                    const now = Date.now();
                    if (now - lastSpark < 70) return;
                    lastSpark = now;
                    sparkC.createObject(sparks, { x: ember.x + ember.width / 2, y: ember.y + ember.height / 2, color: board.th.accent });
                }
                Component {
                    id: sparkC
                    Rectangle {
                        id: sp
                        width: 9; height: 9; radius: 5
                        opacity: 0.9
                        ParallelAnimation {
                            running: true
                            NumberAnimation { target: sp; property: "opacity"; to: 0; duration: 450 }
                            NumberAnimation { target: sp; property: "y"; to: sp.y - 14; duration: 450 }
                            NumberAnimation { target: sp; property: "scale"; to: 0.3; duration: 450 }
                            onFinished: sp.destroy()
                        }
                    }
                }
            }

            // accents (hold A, View, or a long touch)
            Rectangle {
                id: pop
                visible: !!(board.vm && board.vm.popup)
                z: 5
                readonly property var alts: visible ? board.vm.popup.alts : []
                width: Math.min(keysArea.width, (board.keyH + 6) * alts.length + 12)
                height: board.keyH + 12
                radius: 14
                color: board.th.bg2
                border.color: Qt.rgba(1, 1, 1, 0.08)
                x: board.selPlaced ? Math.max(0, Math.min(keysArea.width - width, board.selX + board.selItem.width / 2 - width / 2)) : 0
                y: board.selPlaced ? Math.max(-height / 2, board.selY - height - 6) : 0
                Row {
                    anchors.centerIn: parent
                    spacing: 6
                    Repeater {
                        model: pop.alts
                        delegate: Rectangle {
                            required property string modelData
                            required property int index
                            readonly property bool sel: board.vm.popup && board.vm.popup.i === index
                            width: board.keyH; height: board.keyH; radius: 10
                            color: sel ? board.th.sel : board.th.key
                            border.width: sel ? 3 : 0
                            border.color: board.th.accent
                            Text { anchors.centerIn: parent; text: parent.modelData; color: parent.sel ? board.th.selText : board.th.text; font.pixelSize: board.fontPx; font.weight: Font.DemiBold }
                            MouseArea { anchors.fill: parent; onClicked: board.altPicked(parent.index) }
                        }
                    }
                }
            }

            // the Ember Wheel: eight coals around the hearth; the tilted one glows with its letters on Y X B A
            Item {
                id: wheel
                readonly property var w: board.vm ? board.vm.wheel : null
                visible: !!(w && w.active)
                z: 6
                width: Math.min(300, keysArea.height * 1.1); height: width
                anchors.centerIn: parent
                Rectangle { anchors.fill: parent; radius: width / 2; color: Qt.rgba(Qt.color(board.th.bg).r, Qt.color(board.th.bg).g, Qt.color(board.th.bg).b, 0.9) }
                Rectangle {
                    width: parent.width * 0.22; height: width; radius: width / 2; anchors.centerIn: parent
                    gradient: Gradient { GradientStop { position: 0; color: "#ffd9a8" } GradientStop { position: 0.5; color: board.th.accent } GradientStop { position: 1; color: "#6a2a0a" } }
                }
                Repeater {
                    model: wheel.w ? wheel.w.petals : []
                    delegate: Rectangle {
                        id: petal
                        required property string modelData
                        required property int index
                        readonly property bool on: wheel.w && wheel.w.petal === index
                        readonly property real ang: index * Math.PI / 4
                        width: wheel.width * 0.26; height: width; radius: width / 2
                        x: wheel.width / 2 + Math.sin(ang) * wheel.width * 0.36 - width / 2
                        y: wheel.height / 2 - Math.cos(ang) * wheel.height * 0.36 - height / 2
                        scale: on ? 1.45 : 1
                        z: on ? 2 : 1
                        color: on ? board.th.sel : board.th.key
                        border.width: on ? 3 : 1
                        border.color: on ? board.th.accent : Qt.rgba(1, 1, 1, 0.07)
                        Behavior on scale { NumberAnimation { duration: 100 } }
                        Repeater {
                            model: 4
                            delegate: Text {
                                required property int index
                                readonly property string ch: petal.modelData.charAt(index)
                                text: board.vm && (board.vm.shift || board.vm.caps) ? ch.toUpperCase() : ch
                                color: petal.on ? board.th.selText : board.th.muted
                                font.pixelSize: 12; font.weight: Font.Bold
                                x: index === 1 ? petal.width * 0.08 : index === 2 ? petal.width * 0.92 - width : (petal.width - width) / 2
                                y: index === 0 ? petal.height * 0.06 : index === 3 ? petal.height * 0.94 - height : (petal.height - height) / 2
                            }
                        }
                    }
                }
            }
        }
    }
}
