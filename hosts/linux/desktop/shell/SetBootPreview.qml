// Settings › Boot & startup's preview: plays a boot splash or a startup animation over the page, with its own
// buttons. Looking changes nothing; only "Use this" (or "Use as a set") chooses, through the page's own functions.
// Controller: Left/Right (or Up/Down) move between the buttons, A (Enter) presses one, B (Esc) closes the preview and
// goes back to the card it came from.
//   The splash previews are drawn at build time from each theme's own Plymouth script (tools/plymouth_preview.py;
// plymouthd is never run here). The startup animations play as a flip book; "Full video" plays the WebM itself with
// QtMultimedia when the system has it (SetBootVideo.qml, loaded only on request).
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: pv
    property var boot: null             // SetBoot.qml: pickSplash(), pickMovie(), pickSet(), pairAvailable(), st, busy
    property string kind: ""            // "splash" or "movie"
    property var item: null
    property var geo: null
    property Item back: null            // the card to go back to
    property bool fullVideo: false
    readonly property bool shown: item !== null
    readonly property bool hasPicture: !!geo && !!item && !!item.preview && !!item.preview.sheet
    readonly property bool inUse: !!boot && !!boot.st && !!item
        && (kind === "splash" ? boot.st.splash.current === item.id : boot.st.movie.current === item.id)
    readonly property bool canUse: !!boot && !!boot.st && !!item && !boot.busy
        && (kind === "movie" || (boot.st.splash.canChange && item.available))
    readonly property bool canSet: canUse && !!item && !!item.pair && !!boot.st && boot.st.splash.canChange
        && boot.pairAvailable(kind, item.pair)
    visible: shown
    color: Qt.rgba(Theme.bg.r, Theme.bg.g, Theme.bg.b, 0.97)
    z: 50

    function open(k, it, from) {
        kind = k; item = it; back = from; fullVideo = false;
        Qt.callLater(() => (useBtn.visible && useBtn.enabled ? useBtn : closeBtn).forceActiveFocus());
    }
    function step(from, d) {
        const all = [useBtn, setBtn, fullBtn, closeBtn].filter(x => x.visible && x.enabled);
        const n = all[Math.max(0, Math.min(all.length - 1, all.indexOf(from) + d))];
        if (n) n.forceActiveFocus();
    }
    function close() {
        // Focus goes back to the card first, then the preview hides (a hidden button keeping the focus would leave
        // the controller with nothing to move).
        const b = back;
        if (b) b.forceActiveFocus();
        item = null; fullVideo = false;
    }

    // Clicks and the wheel stop here (the page underneath doesn't scroll or react).
    MouseArea { anchors.fill: parent; hoverEnabled: true; onWheel: w => w.accepted = true }

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(pv.width - 48, 720)
        spacing: 12
        Text {
            Layout.fillWidth: true
            text: pv.item ? (pv.kind === "splash" ? "Boot splash: " : "Startup animation: ") + pv.item.label : ""
            color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold; elide: Text.ElideRight
        }
        Rectangle {
            id: screen
            Layout.alignment: Qt.AlignHCenter
            readonly property real w: Math.max(160, Math.min(parent.width, (pv.height - 250) * 16 / 9))
            Layout.preferredWidth: w
            Layout.preferredHeight: w * 9 / 16
            color: "#0b1016"
            border.width: 1; border.color: Theme.line
            SetBootFrames {
                id: flip
                anchors { fill: parent; margins: 1 }
                visible: pv.hasPicture
                sheet: pv.hasPicture ? pv.item.preview.sheet : ""
                frames: pv.hasPicture ? pv.item.preview.frames : 1
                poster: pv.hasPicture ? pv.item.preview.poster : 0
                columns: pv.geo ? pv.geo.columns : 8
                frameWidth: pv.geo ? pv.geo.frameWidth : 400
                frameHeight: pv.geo ? pv.geo.frameHeight : 225
                fps: pv.geo ? pv.geo.fps : 10
                playing: pv.shown
            }
            Loader {
                id: video
                anchors { fill: parent; margins: 1 }
                active: pv.shown && pv.fullVideo && pv.kind === "movie" && !!pv.item && !!pv.item.file
                source: active ? "SetBootVideo.qml" : ""
                onLoaded: item.file = pv.item.file
            }
            Text {
                visible: !pv.hasPicture
                anchors.centerIn: parent
                text: "No preview for this one in this version"
                color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
            }
        }
        Text {
            Layout.fillWidth: true
            text: !pv.item ? "" : pv.item.about + (pv.kind === "splash"
                ? ". On your screen it fills the screen and keeps going until Hearth OS has started; drawn here from the splash's own script."
                : ". Steam Game Mode plays it once as it starts.")
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
        Text {
            Layout.fillWidth: true
            text: pv.inUse ? "This is the one in use."
                : pv.kind === "splash" && pv.boot && pv.boot.st && !pv.boot.st.splash.canChange ? "The boot splash can't be changed on this edition: this is only a look."
                : "Only a preview: nothing changes until you choose Use this."
            color: pv.inUse ? Theme.flame : Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
        Text {
            Layout.fillWidth: true
            visible: video.status === Loader.Error || (video.item && video.item.failed)
            text: "The full video can't play here (no video player on this system); the preview above is the same animation."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
        RowLayout {
            Layout.topMargin: 4
            spacing: 10
            PvButton {
                id: useBtn
                host: pv
                visible: !pv.inUse
                enabled: pv.canUse
                label: "Use this"
                primary: true
                onPressed: {
                    const it = pv.item, k = pv.kind;
                    pv.close();
                    if (k === "splash") pv.boot.pickSplash(it.id, it.label); else pv.boot.pickMovie(it.id, it.label);
                }
            }
            PvButton {
                id: setBtn
                host: pv
                visible: pv.canSet
                label: pv.kind === "splash" ? "Use with its startup animation" : "Use with its boot splash"
                onPressed: {
                    const it = pv.item, k = pv.kind;
                    pv.close();
                    pv.boot.pickSet(k === "splash" ? it.id : it.pair, k === "splash" ? it.pair : it.id, it.label);
                }
            }
            PvButton {
                id: fullBtn
                host: pv
                visible: pv.kind === "movie" && !!pv.item && !!pv.item.file && !pv.fullVideo
                label: "Full video"
                onPressed: { closeBtn.forceActiveFocus(); pv.fullVideo = true; }
            }
            PvButton {
                id: closeBtn
                host: pv
                label: "Close"
                onPressed: pv.close()
            }
        }
    }

    component PvButton: Rectangle {
        id: b
        property string label
        property bool primary: false
        signal pressed()
        implicitWidth: t.implicitWidth + 36
        implicitHeight: 42
        radius: Theme.r(10)
        opacity: enabled ? 1 : 0.45
        activeFocusOnTab: true          // hidden or greyed-out buttons are skipped by Qt anyway
        color: primary && enabled ? Theme.accent : (hov.containsMouse ? Theme.panel3 : Theme.panel2)
        border.width: 2
        border.color: activeFocus ? (primary ? Theme.text : Theme.accent) : "transparent"
        Accessible.role: Accessible.Button
        Accessible.name: label
        Accessible.onPressAction: if (enabled) pressed()
        Text { id: t; anchors.centerIn: parent; text: b.label; color: b.primary && b.enabled ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
        MouseArea { id: hov; anchors.fill: parent; hoverEnabled: true; enabled: b.enabled; cursorShape: Qt.PointingHandCursor; onClicked: b.pressed() }
        property var host: null         // the preview (inline components don't see the file's ids)
        Keys.onPressed: e => {
            switch (e.key) {
            case Qt.Key_Left: case Qt.Key_Up: case Qt.Key_Backtab: b.host.step(b, -1); break;
            case Qt.Key_Right: case Qt.Key_Down: case Qt.Key_Tab: b.host.step(b, 1); break;
            case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: if (b.enabled) b.pressed(); break;
            case Qt.Key_Escape: case Qt.Key_Back: b.host.close(); break;
            default: return;
            }
            e.accepted = true;
        }
    }
}
