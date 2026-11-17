// Hearth Desktop Mode on KWin: live window previews for Mantel's dock and taskbar (KWin declarative script
// "hearthpreviews"). Mantel is a separate program and can't draw another app's window, so KWin draws the previews:
// resting the pointer on an app in the dock (Dock.qml) or the bar's apps (TaskStrip.qml) asks for them through
// WinList.qml and hearth-kwin-bridge, and this script shows a card with a live thumbnail of each of the app's windows
// (KWin's WindowThumbnail: they move as the window does), its title and a close button, in Mantel's colours (the
// bridge sends ~/.config/hearth/theme.json's). Click a preview: that window comes to the front; middle-click or the X:
// it closes. Leaving both the app's button and the card hides it.
//
//   Mantel --preview / previewHide--> hearth-kwin-bridge --reply to NextPreview()--> this script
//
// The bridge loads it (Scripting.loadDeclarativeScript) next to hearthwindows; without the bridge it just waits, and
// without this script Mantel keeps its plain labels. labwc has no way to show another app's window, so Mantel only
// asks on KWin.
import QtQuick
import QtQuick.Window as QW
import org.kde.kwin

Item {
    id: root
    property var wins: []                 // KWin's windows on the card
    property var cols: ({ panel: "#16181d", panel2: "#20232b", panel3: "#2a2e38", text: "#eceef2", muted: "#9aa1ad",
                          accent: "#ff7a3d", line: "#2c3038", r: 12, font: "sans-serif" })
    property string forApp: ""            // the button it's showing for (Mantel's key), so a second ask doesn't redo it
    readonly property int thumbH: 132
    readonly property int thumbMaxW: 236

    function findWin(id) {
        const st = Workspace.stackingOrder;
        for (let i = 0; i < st.length; i++) if (String(st[i].internalId) === String(id)) return st[i];
        return null;
    }
    function screenRect() {
        try {
            const s = Workspace.activeScreen;
            if (s && s.geometry && s.geometry.width > 0) return s.geometry;
        } catch (e) {}
        return { x: 0, y: 0, width: 1920, height: 1080 };
    }
    function show(c) {
        const list = [];
        for (const id of (c.ids || [])) { const w = findWin(id); if (w && !w.deleted) list.push(w); }
        if (!list.length) { console.warn("hearthpreviews: none of these windows is here: " + JSON.stringify(c.ids)); hideNow(); return; }
        if (c.cols) cols = Object.assign({}, cols, c.cols);
        wins = list.slice(0, 6);
        forApp = String(c.key || "");
        hideTimer.stop();
        placeLater.cmd = c;
        placeLater.restart();             // once the card has its size
        pop.visible = true;
    }
    // Where: over (or beside) the button the pointer rests on. Mantel says which edge its bar is on, how far the
    // pointer is from the button's inner side (near) and from its middle (along); at: a fixed place (checks).
    function place(c) {
        const s = screenRect(), p = Workspace.cursorPos || { x: s.x + s.width / 2, y: s.y + s.height };
        const W = card.width, H = card.height, gap = 12;
        let x, y;
        if (c.at) { x = c.at.x - W / 2; y = c.at.y - H; }
        else if (c.edge === "top") { x = p.x + (c.along || 0) - W / 2; y = p.y + (c.near || 0) + gap; }
        else if (c.edge === "left") { x = p.x + (c.near || 0) + gap; y = p.y + (c.along || 0) - H / 2; }
        else if (c.edge === "right") { x = p.x - (c.near || 0) - gap - W; y = p.y + (c.along || 0) - H / 2; }
        else { x = p.x + (c.along || 0) - W / 2; y = p.y - (c.near || 0) - gap - H; }
        pop.x = Math.round(Math.max(s.x + 8, Math.min(x, s.x + s.width - W - 8)));
        pop.y = Math.round(Math.max(s.y + 8, Math.min(y, s.y + s.height - H - 8)));
    }
    function hideNow() { pop.visible = false; wins = []; forApp = ""; }
    function run(c) {
        if (c.op === "preview") show(c);
        else if (c.op === "previewHide") { if (c.now) hideNow(); else if (!cardHover.hovered) hideTimer.restart(); }
    }
    Timer { id: placeLater; interval: 1; property var cmd: ({}); onTriggered: root.place(cmd) }
    Timer { id: hideTimer; interval: 350; onTriggered: if (!cardHover.hovered) root.hideNow() }

    // The bridge answers NextPreview() when Mantel asks for something (or after 20 s with nothing): ask again each time.
    DBusCall {
        id: next
        service: "org.hearth.Windows"
        path: "/org/hearth/Windows"
        dbusInterface: "org.hearth.Windows"
        method: "NextPreview"
        onFinished: returnValue => {
            let list = [];
            try {
                const v = Array.isArray(returnValue) ? returnValue[0] : returnValue;
                list = JSON.parse(String(Array.isArray(v) ? v[0] : v));
            } catch (e) { console.warn("hearthpreviews: the bridge's answer: " + e); }
            for (const c of list) {
                try { root.run(c); } catch (e) { console.warn("hearthpreviews: " + c.op + ": " + e); }
            }
            again.restart();
        }
        onFailed: retry.restart()
    }
    Timer { id: again; interval: 5; onTriggered: next.call() }
    Timer { id: retry; interval: 5000; onTriggered: next.call() }
    Component.onCompleted: next.call()
    Connections {
        target: Workspace
        function onWindowRemoved(w) {
            if (root.wins.indexOf(w) < 0) return;
            const rest = root.wins.filter(x => x !== w);
            if (rest.length) root.wins = rest; else root.hideNow();
        }
    }

    // QtQuick's Window (org.kde.kwin has a Window type of its own: KWin's windows, which can't be made here).
    QW.Window {
        id: pop
        visible: false
        flags: Qt.BypassWindowManagerHint | Qt.FramelessWindowHint | Qt.WindowDoesNotAcceptFocus
        color: "transparent"
        width: card.width
        height: card.height
        title: "Hearth window previews"

        Rectangle {
            id: card
            width: Math.max(160, row.width + 20)
            height: row.height + 20
            radius: root.cols.r
            color: root.cols.panel
            border.width: 1
            border.color: root.cols.line
            HoverHandler { id: cardHover; onHoveredChanged: if (!hovered) hideTimer.restart(); else hideTimer.stop() }

            Row {
                id: row
                x: 10; y: 10
                spacing: 10
                Repeater {
                    model: root.wins
                    delegate: Rectangle {
                        id: cell
                        required property var modelData
                        readonly property real ratio: {
                            const g = modelData.frameGeometry;
                            return g && g.height > 0 ? Math.max(0.45, Math.min(2.4, g.width / g.height)) : 1.6;
                        }
                        readonly property int tw: Math.round(Math.min(root.thumbMaxW, root.thumbH * ratio))
                        readonly property int th: Math.round(Math.min(root.thumbH, tw / ratio))
                        readonly property bool front: Workspace.activeWindow === modelData
                        width: tw + 12
                        height: th + 44
                        radius: Math.max(4, root.cols.r - 4)
                        color: cellMouse.containsMouse ? root.cols.panel3 : root.cols.panel2
                        border.width: front ? 2 : 0
                        border.color: root.cols.accent

                        Text {
                            x: 10; y: 9
                            width: parent.width - 44
                            text: cell.modelData.caption || "Window"
                            elide: Text.ElideRight
                            color: root.cols.text
                            font.family: root.cols.font; font.pixelSize: 12; font.weight: cell.front ? Font.DemiBold : Font.Normal
                        }
                        Item {
                            id: holder
                            x: 6; y: 36
                            width: cell.tw; height: cell.th
                            Rectangle { anchors.fill: parent; radius: 4; color: Qt.rgba(0, 0, 0, 0.25) }
                            // Made here rather than written out, so a KWin that names the window property
                            // differently (client / wId) still shows it.
                            Component.onCompleted: {
                                try {
                                    const t = Qt.createQmlObject('import QtQuick; import org.kde.kwin; WindowThumbnail { anchors.fill: parent }', holder, "thumb");
                                    try { t.client = cell.modelData; } catch (e) {}
                                    try { if (!t.client) t.wId = cell.modelData.internalId; } catch (e) {}
                                } catch (e) { console.warn("hearthpreviews: no thumbnail: " + e); }
                            }
                        }
                        MouseArea {
                            id: cellMouse
                            anchors.fill: parent
                            hoverEnabled: true
                            acceptedButtons: Qt.LeftButton | Qt.MiddleButton
                            onClicked: mouse => {
                                const w = cell.modelData;
                                if (mouse.button === Qt.MiddleButton) { w.closeWindow(); return; }
                                w.minimized = false;
                                Workspace.activeWindow = w;
                                root.hideNow();
                            }
                        }
                        Rectangle {   // close
                            x: parent.width - width - 6; y: 6
                            width: 24; height: 24; radius: 12
                            visible: cellMouse.containsMouse || closeMouse.containsMouse
                            color: closeMouse.containsMouse ? "#e5484d" : root.cols.panel3
                            Text { anchors.centerIn: parent; text: "✕"; color: root.cols.text; font.pixelSize: 11; font.family: root.cols.font }
                            MouseArea { id: closeMouse; anchors.fill: parent; hoverEnabled: true; onClicked: cell.modelData.closeWindow() }
                        }
                    }
                }
            }
        }
    }
}
