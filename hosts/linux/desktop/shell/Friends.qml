// Hearth Friends in Desktop Mode: the same friends, chats and requests as Game Mode's Friends screen, from the same
// Hearth Friends service (hosts/hearth-friends: Matrix, end-to-end encrypted), through `hearth-friends call` and
// `hearth-friends watch`. Opens from the bar's Friends button (or `quickshell ipc call friends toggle`).
// Controller: Up/Down pick (the controller types arrow keys here), A opens a chat or accepts, X declines a request,
// Y writes a message (Mantel's on-screen keyboard), B goes back / closes. Mouse and keyboard work as usual.
// New messages pop up through Mantel's notifications (the service sends them there when Game Mode isn't showing
// its own).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import Quickshell.Wayland

Scope {
    id: fr
    property bool shown: false
    property bool available: true
    property var st: null                  // the service's status
    property var friends: []
    property var chats: []
    property var reqs: []
    property var games: []                 // the library (hearth-games cached), for game invites
    property string room: ""               // the open chat
    property var msgs: []
    property var images: ({})
    property int current: 0
    property int unread: 0
    property string signUser: ""
    property string signServer: "matrix.org"
    property bool pickingGame: false
    readonly property var rows: buildRows()
    readonly property string gamesHelper: decodeURIComponent(Qt.resolvedUrl("hearth-games").toString().replace(/^file:\/\//, ""))

    function open() { shown = true; current = 0; refresh(); }
    function close() { shown = false; room = ""; pickingGame = false; }
    function toggle() { shown ? close() : open(); }
    onShownChanged: SettingsNav.friendsShown = shown
    Component.onCompleted: SettingsNav.friends = fr

    IpcHandler {
        target: "friends"
        function toggle(): void { fr.toggle(); }
        function open(): void { fr.open(); }
        function close(): void { fr.close(); }
    }

    // ------------------------------------------------ talking to the service, one request at a time
    property var queue: []
    function svc(method, args, done) {
        queue = queue.concat([{ method: method, args: args || {}, done: done }]);
        if (!caller.running) next();
    }
    function next() {
        if (!queue.length || caller.running) return;
        const q = queue[0];
        caller.done = q.done;
        caller.command = ["hearth-friends", "call", q.method, JSON.stringify(q.args)];
        queue = queue.slice(1);
        caller.running = true;
    }
    // One call at a time: the answer is handled once the output is read AND the program has exited (starting the
    // next call before the exit was ignored, and answers went to the wrong callers).
    Process {
        id: caller
        property var done: null
        property string out: ""
        property bool read: false
        property bool exited: false
        function finish() {
            if (!read || !exited) return;
            read = false; exited = false;
            const t = out.trim();
            let r = null, err = null;
            if (t.startsWith("error: ")) err = t.slice(7);
            else { try { r = JSON.parse(t); } catch (e) { err = t ? "A garbled answer" : "Hearth Friends isn't installed"; } }
            if (err && err.indexOf("isn't running") < 0 && err !== "Hearth Friends isn't installed") SettingsNav.toast(err);
            if (err === "Hearth Friends isn't installed") fr.available = false;
            const d = done; done = null;
            if (d) d(r, err);
            Qt.callLater(fr.next);
        }
        stdout: StdioCollector { onStreamFinished: { caller.out = text; caller.read = true; caller.finish(); } }
        onExited: (code, status) => { if (code === 127) fr.available = false; caller.exited = true; caller.finish(); }
    }
    // Events: new messages, presence, requests. Restarted if the service restarts.
    Process {
        id: watcher
        running: true
        command: ["hearth-friends", "watch", "mantel"]
        stdout: SplitParser {
            onRead: line => {
                let o = null; try { o = JSON.parse(line); } catch (e) { return; }
                if (o.id !== undefined) { fr.st = o.value || null; fr.refresh(); return; }
                if (o.event === "message" && o.data && o.data.roomId === fr.room) {
                    if (!fr.msgs.some(m => m.id === o.data.msg.id)) fr.msgs = fr.msgs.concat([o.data.msg]);
                    if (fr.shown) fr.svc("read", { roomId: fr.room });
                }
                if (o.event === "status") fr.st = o.data;
                if (["message", "rooms", "presence", "request", "resync", "status"].indexOf(o.event) >= 0) refreshSoon.restart();
            }
        }
        onExited: rewatch.start()
    }
    Timer { id: rewatch; interval: 5000; onTriggered: watcher.running = true }
    Timer { id: refreshSoon; interval: 700; onTriggered: fr.refresh() }
    Process {
        id: gameList
        command: ["python3", fr.gamesHelper, "cached"]
        stdout: StdioCollector { onStreamFinished: { try { fr.games = JSON.parse(text.trim().split("\n").pop()) || []; } catch (e) { fr.games = []; } } }
    }

    function refresh() {
        svc("status", {}, (r, e) => {
            if (!r) return;
            st = r;
            if (!r.signedIn) { friends = []; chats = []; reqs = []; unread = 0; return; }
            svc("friends", {}, f => { if (f) friends = f; });
            svc("chats", {}, c => { if (c) { chats = c; unread = c.filter(x => !x.muted).reduce((n, x) => n + (x.unread || 0), 0); SettingsNav.friendsUnread = unread; } });
            svc("requests", {}, q => { if (q) reqs = (q.requests || []).concat(q.invites || []); });
        });
        if (shown && !gameList.running) gameList.running = true;
    }
    function buildRows() {
        const out = [];
        if (reqs.length) { out.push({ kind: "head", text: "Requests" }); reqs.forEach(r => out.push({ kind: "req", r: r })); }
        if (chats.length) { out.push({ kind: "head", text: "Chats" }); chats.forEach(c => out.push({ kind: "chat", c: c })); }
        const lonely = friends.filter(f => !chats.some(c => c.roomId === f.roomId));
        if (lonely.length) { out.push({ kind: "head", text: "Friends" }); lonely.forEach(f => out.push({ kind: "friend", f: f })); }
        return out;
    }
    function friendLine(f) {
        if (!f) return "";
        if (f.state === "pending") return "Request sent";
        if (f.game) return "Playing " + f.game + (f.joinable ? " · Joinable" : "");
        return f.presence === "online" ? "Online" : f.presence === "unavailable" ? "Away" : "Offline";
    }
    function friendOf(roomId) { return friends.find(f => f.roomId === roomId); }
    function openChat(roomId) {
        room = roomId; msgs = []; pickingGame = false;
        svc("messages", { roomId: roomId }, r => { if (r && fr.room === roomId) { fr.msgs = r.messages || []; fr.svc("read", { roomId: roomId }); } });
        input.forceActiveFocus();
    }
    function activate(i) {
        const row = rows[i];
        if (!row) return;
        if (row.kind === "chat") openChat(row.c.roomId);
        else if (row.kind === "friend") openChat(row.f.roomId);
        else if (row.kind === "req") svc("accept", { roomId: row.r.roomId }, () => fr.refresh());
    }
    function decline(i) { const row = rows[i]; if (row && row.kind === "req") svc("decline", { roomId: row.r.roomId }, () => fr.refresh()); }
    function send() {
        const t = input.text.trim();
        if (!t || !room) return;
        input.text = "";
        svc("send", { roomId: room, text: t });
    }
    function localGame(g) {
        return games.find(x => x.id === g.id) || games.find(x => (x.title || "").toLowerCase() === (g.title || "").toLowerCase());
    }
    function invite(g) {
        pickingGame = false;
        const appId = String(g.id).indexOf(":") > 0 ? String(g.id).split(":")[1] : undefined;
        svc("invite", { roomId: room, game: { title: g.title, store: g.source || "", id: g.id, appId: g.source === "steam" ? appId : undefined } }, r => { if (r) SettingsNav.toast("Invite to " + g.title + " sent"); });
    }

    // ------------------------------------------------ the window
    PanelWindow {
        id: win
        visible: fr.shown
        WlrLayershell.layer: WlrLayer.Overlay
        WlrLayershell.namespace: "hearth-friends"
        WlrLayershell.keyboardFocus: fr.shown ? WlrKeyboardFocus.Exclusive : WlrKeyboardFocus.None
        exclusionMode: ExclusionMode.Ignore
        anchors { top: true; bottom: true; left: true; right: true }
        color: "#b0080b10"
        onVisibleChanged: if (visible) keys.forceActiveFocus()
        MouseArea { anchors.fill: parent; onClicked: fr.close() }

        Rectangle {
            anchors.centerIn: parent
            width: Math.min(parent.width - 48, 1040)
            height: Math.min(parent.height - 96, 680)
            radius: 20
            color: Theme.panel
            border.color: Theme.line
            MouseArea { anchors.fill: parent }

            Item {
                id: keys
                anchors.fill: parent
                focus: true
                Keys.onPressed: e => {
                    switch (e.key) {
                    case Qt.Key_Down: fr.current = Math.min(fr.rows.length - 1, fr.current + 1); while (fr.rows[fr.current] && fr.rows[fr.current].kind === "head" && fr.current < fr.rows.length - 1) fr.current++; break;
                    case Qt.Key_Up: fr.current = Math.max(0, fr.current - 1); while (fr.rows[fr.current] && fr.rows[fr.current].kind === "head" && fr.current > 0) fr.current--; break;
                    case Qt.Key_Return: case Qt.Key_Enter: case Qt.Key_Space: fr.activate(fr.current); break;
                    case Qt.Key_X: case Qt.Key_Delete: fr.decline(fr.current); break;
                    case Qt.Key_Y: case Qt.Key_Tab: if (fr.room) input.forceActiveFocus(); break;
                    case Qt.Key_Escape: if (fr.room) fr.room = ""; else fr.close(); break;
                    default: return;
                    }
                    list.positionViewAtIndex(fr.current, ListView.Contain);
                    e.accepted = true;
                }
            }

            ColumnLayout {
                anchors { fill: parent; margins: 20 }
                spacing: 12
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 10
                    Icon { name: "user"; size: 22 }
                    Text { text: "Friends"; color: Theme.text; font.family: Theme.font; font.pixelSize: 19; font.weight: Font.DemiBold }
                    Text {
                        Layout.fillWidth: true
                        text: fr.st && fr.st.signedIn ? fr.st.userId + " · lives on " + (fr.st.lives || fr.st.homeserver) : "Matrix: the open chat network"
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight
                    }
                    Text { text: "Ⓐ open  Ⓧ decline  Ⓨ write  Ⓑ back"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                }

                // Not signed in, or no service.
                ColumnLayout {
                    visible: !fr.available || !(fr.st && fr.st.signedIn)
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 10
                    Text {
                        Layout.fillWidth: true; wrapMode: Text.WordWrap
                        text: !fr.available ? "Hearth Friends comes with Hearth OS and isn't on this system."
                            : "Chat with friends, invite them to games and see what they're playing, on Matrix. Hearth doesn't run a server: your account lives on matrix.org or any Matrix server you pick. Chats with friends are end-to-end encrypted. New accounts on matrix.org are made at app.element.io; servers that allow it can be signed up to from Game Mode's Friends screen."
                        color: Theme.text; font.family: Theme.font; font.pixelSize: 14
                    }
                    Repeater {
                        model: fr.available ? [["Server", "server"], ["User name or @name:server", "user"], ["Password", "pass"]] : []
                        delegate: Rectangle {
                            required property var modelData
                            Layout.fillWidth: true; implicitHeight: 44; radius: 12; color: Theme.bg2; border.color: fld.activeFocus ? Theme.accent : Theme.line
                            TextInput {
                                id: fld
                                anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                                verticalAlignment: TextInput.AlignVCenter
                                color: Theme.text; font.pixelSize: 15; font.family: Theme.font; clip: true
                                echoMode: modelData[1] === "pass" ? TextInput.Password : TextInput.Normal
                                text: modelData[1] === "server" ? fr.signServer : ""
                                onTextChanged: { if (modelData[1] === "server") fr.signServer = text; if (modelData[1] === "user") fr.signUser = text; if (modelData[1] === "pass") signPass.text = text; }
                                Keys.onReturnPressed: signBtn.go()
                                Text { visible: !parent.text; text: modelData[0]; color: Theme.muted; font: parent.font; anchors.verticalCenter: parent.verticalCenter }
                            }
                        }
                    }
                    TextInput { id: signPass; visible: false }
                    Rectangle {
                        id: signBtn
                        visible: fr.available
                        function go() {
                            if (!fr.signUser || !signPass.text) return;
                            SettingsNav.toast("Signing in…");
                            fr.svc("login", { homeserver: fr.signServer, user: fr.signUser, password: signPass.text }, r => { signPass.text = ""; if (r && r.signedIn) fr.refresh(); });
                        }
                        implicitWidth: 160; implicitHeight: 42; radius: 12; color: Theme.accent
                        Text { anchors.centerIn: parent; text: "Sign in"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 15; font.bold: true }
                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: signBtn.go() }
                    }
                    Item { Layout.fillHeight: true }
                }

                RowLayout {
                    visible: fr.available && fr.st && fr.st.signedIn
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    spacing: 14

                    // Requests, chats and friends.
                    ListView {
                        id: list
                        Layout.preferredWidth: 340
                        Layout.fillHeight: true
                        clip: true
                        spacing: 6
                        model: fr.rows
                        boundsBehavior: Flickable.StopAtBounds
                        delegate: Rectangle {
                            id: row
                            required property var modelData
                            required property int index
                            readonly property bool lit: fr.current === index && modelData.kind !== "head"
                            readonly property var f: modelData.kind === "friend" ? modelData.f : modelData.kind === "chat" ? fr.friendOf(modelData.c.roomId) : null
                            width: ListView.view.width
                            height: modelData.kind === "head" ? 28 : 56
                            radius: 12
                            color: modelData.kind === "head" ? "transparent" : lit ? Theme.panel3 : rowMouse.containsMouse ? Theme.panel2 : Theme.bg2
                            border.width: modelData.kind === "head" ? 0 : 2
                            border.color: lit ? Theme.accent : "transparent"
                            Text {
                                visible: row.modelData.kind === "head"
                                anchors { left: parent.left; bottom: parent.bottom; leftMargin: 4 }
                                text: row.modelData.text || ""
                                color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; font.bold: true
                            }
                            Column {
                                visible: row.modelData.kind !== "head"
                                anchors { left: parent.left; right: badge.left; verticalCenter: parent.verticalCenter; leftMargin: 14; rightMargin: 8 }
                                Text {
                                    width: parent.width; elide: Text.ElideRight
                                    text: row.modelData.kind === "req" ? row.modelData.r.name : row.modelData.kind === "chat" ? row.modelData.c.name : row.modelData.kind === "friend" ? row.modelData.f.name : ""
                                    color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold
                                }
                                Text {
                                    width: parent.width; elide: Text.ElideRight
                                    text: row.modelData.kind === "req" ? (row.modelData.r.direct ? "wants to be friends · Ⓐ accept Ⓧ decline" : "invited you to " + (row.modelData.r.roomName || "a group chat"))
                                        : row.f ? fr.friendLine(row.f) : row.modelData.kind === "chat" ? (row.modelData.c.last ? (row.modelData.c.last.mine ? "You: " : "") + (row.modelData.c.last.body || "") : row.modelData.c.members + " people") : ""
                                    color: row.f && row.f.game ? "#7ad15a" : Theme.muted; font.family: Theme.font; font.pixelSize: 12
                                }
                            }
                            Rectangle {
                                id: badge
                                readonly property int n: row.modelData.kind === "chat" && !row.modelData.c.muted ? (row.modelData.c.unread || 0) : 0
                                visible: n > 0
                                anchors { right: parent.right; verticalCenter: parent.verticalCenter; rightMargin: 12 }
                                width: Math.max(22, bt.implicitWidth + 10); height: 22; radius: 11; color: Theme.accent
                                Text { id: bt; anchors.centerIn: parent; text: String(badge.n); color: Theme.accentInk; font.pixelSize: 11; font.bold: true; font.family: Theme.font }
                            }
                            MouseArea { id: rowMouse; anchors.fill: parent; hoverEnabled: true; enabled: row.modelData.kind !== "head"; cursorShape: Qt.PointingHandCursor; onClicked: { fr.current = row.index; fr.activate(row.index); } }
                        }
                        Text {
                            anchors.centerIn: parent; width: parent.width - 30; horizontalAlignment: Text.AlignHCenter; wrapMode: Text.WordWrap
                            visible: fr.rows.length === 0
                            text: "No friends yet. Add them from Game Mode's Friends screen, or share your address: " + (fr.st ? fr.st.userId : "")
                            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                        }
                    }

                    // The chat.
                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        spacing: 8
                        Text {
                            Layout.fillWidth: true
                            text: fr.room ? ((fr.chats.find(c => c.roomId === fr.room) || {}).name || "Chat") + "  ·  end-to-end encrypted" : "Pick a chat or a friend"
                            color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold; elide: Text.ElideRight
                        }
                        ListView {
                            id: chatList
                            Layout.fillWidth: true
                            Layout.fillHeight: true
                            clip: true
                            spacing: 6
                            model: fr.msgs
                            onCountChanged: positionViewAtEnd()
                            delegate: Rectangle {
                                id: bubble
                                required property var modelData
                                readonly property bool mine: modelData.mine === true
                                readonly property var game: modelData.kind === "game" ? modelData.game : null
                                width: Math.min(ListView.view.width * 0.8, Math.max(body.implicitWidth, who.implicitWidth) + 28)
                                x: mine ? ListView.view.width - width : 0
                                height: col.implicitHeight + 16
                                radius: 12
                                color: mine ? Theme.panel3 : Theme.bg2
                                Column {
                                    id: col
                                    anchors { left: parent.left; right: parent.right; top: parent.top; margins: 8; leftMargin: 14; rightMargin: 14 }
                                    spacing: 4
                                    Text { id: who; text: (bubble.mine ? "You" : (bubble.modelData.senderName || bubble.modelData.sender)) + (bubble.modelData.at ? " · " + new Date(bubble.modelData.at).toLocaleTimeString(Qt.locale(), "hh:mm") : ""); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                                    Text {
                                        id: body
                                        width: Math.min(implicitWidth, chatList.width * 0.8 - 28)
                                        wrapMode: Text.Wrap
                                        textFormat: Text.PlainText
                                        text: bubble.game ? "Game invite: " + bubble.game.title + (bubble.game.server ? " · server " + bubble.game.server : "")
                                            : bubble.modelData.kind === "image" ? (fr.images[bubble.modelData.id] ? "" : "Picture (click to show)")
                                            : (bubble.modelData.body || "")
                                        color: bubble.modelData.kind === "member" || bubble.modelData.kind === "locked" ? Theme.muted : Theme.text; font.family: Theme.font; font.pixelSize: 14
                                    }
                                    Image {
                                        visible: bubble.modelData.kind === "image" && !!fr.images[bubble.modelData.id]
                                        source: fr.images[bubble.modelData.id] || ""
                                        width: Math.min(360, chatList.width * 0.7); fillMode: Image.PreserveAspectFit
                                    }
                                    Rectangle {
                                        visible: bubble.game !== null && !bubble.mine
                                        implicitWidth: playTxt.implicitWidth + 24; implicitHeight: 32; radius: 10; color: Theme.accent
                                        readonly property var local: bubble.game ? fr.localGame(bubble.game) : null
                                        Text { id: playTxt; anchors.centerIn: parent; text: parent.local ? "Play " + parent.local.title : "Not in your library"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 13; font.bold: true }
                                        MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: if (parent.local) Quickshell.execDetached(["python3", fr.gamesHelper, "play", parent.local.id]) }
                                    }
                                }
                                MouseArea {
                                    anchors.fill: parent; z: -1
                                    enabled: bubble.modelData.kind === "image" && !fr.images[bubble.modelData.id]
                                    onClicked: fr.svc("image", { roomId: fr.room, eventId: bubble.modelData.id }, r => { if (r && r.url) { const m = Object.assign({}, fr.images); m[bubble.modelData.id] = r.url; fr.images = m; } })
                                }
                            }
                        }
                        // Games to invite to (from the library).
                        Flow {
                            visible: fr.pickingGame
                            Layout.fillWidth: true
                            spacing: 6
                            Repeater {
                                model: fr.games.slice(0, 10)
                                delegate: Rectangle {
                                    required property var modelData
                                    implicitWidth: gt.implicitWidth + 20; implicitHeight: 32; radius: 10; color: gm.containsMouse ? Theme.panel3 : Theme.bg2
                                    Text { id: gt; anchors.centerIn: parent; text: modelData.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                                    MouseArea { id: gm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: fr.invite(modelData) }
                                }
                            }
                        }
                        RowLayout {
                            visible: fr.room !== ""
                            Layout.fillWidth: true
                            spacing: 8
                            Rectangle {
                                Layout.fillWidth: true; implicitHeight: 44; radius: 12; color: Theme.bg2; border.color: input.activeFocus ? Theme.accent : Theme.line
                                TextInput {
                                    id: input
                                    anchors { fill: parent; leftMargin: 14; rightMargin: 14 }
                                    verticalAlignment: TextInput.AlignVCenter
                                    color: Theme.text; selectionColor: Theme.accent; selectedTextColor: Theme.accentInk
                                    font.pixelSize: 15; font.family: Theme.font; clip: true
                                    property bool typingSent: false
                                    onTextChanged: { const t = text.length > 0; if (t !== typingSent && fr.room) { typingSent = t; fr.svc("typing", { roomId: fr.room, on: t }); } }
                                    Keys.onReturnPressed: fr.send()
                                    Keys.onEnterPressed: fr.send()
                                    Keys.onEscapePressed: keys.forceActiveFocus()
                                    Text { visible: !parent.text; text: "Write a message (Enter sends)"; color: Theme.muted; font: parent.font; anchors.verticalCenter: parent.verticalCenter }
                                }
                            }
                            Rectangle {
                                implicitWidth: 120; implicitHeight: 44; radius: 12; color: inv.containsMouse ? Theme.panel3 : Theme.panel2
                                Text { anchors.centerIn: parent; text: "Invite to play"; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                                MouseArea { id: inv; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: { fr.pickingGame = !fr.pickingGame; if (!gameList.running) gameList.running = true; } }
                            }
                            Rectangle {
                                implicitWidth: 80; implicitHeight: 44; radius: 12; color: Theme.accent
                                Text { anchors.centerIn: parent; text: "Send"; color: Theme.accentInk; font.family: Theme.font; font.pixelSize: 14; font.bold: true }
                                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: fr.send() }
                            }
                        }
                    }
                }
            }
        }
    }
}
