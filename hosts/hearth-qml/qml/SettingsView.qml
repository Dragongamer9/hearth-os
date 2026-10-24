// Settings, as far as the Qt renderer has it: Display › Renderer (the switch between the three renderers), System
// (about, Send logs to the developer, restart), and the way to everything else (the web renderer).
import QtQuick

Item {
    id: sv
    property string tab: "display"
    readonly property var tabs: [["display", "Display", "display"], ["system", "System", "info"], ["more", "Everything else", "gear"]]
    property var logResult: null
    property bool sending: false
    readonly property var names: ({ auto: "Automatic", chromium: "Electron (Chromium)", webkit: "WebKit", qt: "Qt (Experimental)" })
    readonly property var r: App.renderer

    Component.onCompleted: App.call("display.renderer").then(x => { App.renderer = x; }).catch(() => {})
    Connections {
        target: App
        function onTabStepped(dir) {
            if (App.view !== "settings" || App.layers.length) return;
            const i = sv.tabs.findIndex(t => t[0] === sv.tab);
            sv.tab = sv.tabs[(i + dir + sv.tabs.length) % sv.tabs.length][0];
            App.later(30, () => Nav.focusKey("st-" + sv.tab));
        }
    }

    function pickRenderer() {
        const items = [
            ["auto", "Automatic", "Electron (Chromium) on Nvidia graphics, WebKit elsewhere"],
            ["chromium", "Electron (Chromium)", "Smoother on Nvidia graphics"],
            ["webkit", "WebKit", "Lighter; the usual one on AMD and Intel"],
            ["qt", "Qt (Experimental)", "Native Qt Quick screens; not everything is here yet"]
        ].filter(x => x[0] !== "chromium" || !r || r.electron !== false)
         .filter(x => x[0] !== "qt" || !r || r.qt !== false)
         .map(x => ({ label: x[1], sub: x[2], icon: (r && r.pick === x[0]) ? "check" : "display", on: !!(r && r.pick === x[0]),
                      run: () => App.call("display.renderer", { set: x[0] }).then(n => { App.renderer = n; App.toast("Restarting Hearth…"); App.later(600, () => App.call("app.restart").catch(() => {})); }).catch(e => App.toast(e.message)) }));
        App.openSheet({ title: "Renderer", sub: "Which engine draws Hearth's screens. Hearth restarts to change it.", items: items });
    }
    function sendLogs() {
        if (sending) return;
        sending = true;
        App.toast("Sending Hearth's logs…");
        App.log("Send logs to the developer (from the Qt renderer)");
        App.call("logs.sendToDeveloper", { note: "Sent from the Qt renderer (Experimental)" }, 90000).then(x => {
            logResult = x;
            App.openSheet({ title: "Logs sent", sub: "Tell the developer this code: " + (x && x.code ? x.code : "?"), items: [{ label: "OK", icon: "check", run: () => {} }] });
        }).catch(e => App.toast("The logs weren't sent: " + e.message)).then(() => { sending = false; });
    }

    Row {
        anchors.fill: parent
        anchors.leftMargin: Theme.gut
        anchors.topMargin: Theme.u(18)
        spacing: Theme.u(24)
        Column {
            width: Theme.u(230)
            spacing: Theme.u(4)
            Repeater {
                model: sv.tabs
                delegate: Btn {
                    required property var modelData
                    width: parent.width
                    kind: "menu"
                    icon: modelData[2]
                    text: modelData[1]
                    on: sv.tab === modelData[0]
                    navKey: "st-" + modelData[0]
                    navFirst: sv.tab === modelData[0]
                    onActivated: sv.tab = modelData[0]
                    Rectangle { anchors { left: parent.left; verticalCenter: parent.verticalCenter } width: Theme.u(3); height: parent.height * 0.6; radius: 2; color: Theme.accent; visible: parent.on }
                }
            }
        }
        Scroller {
            id: page
            width: parent.width - Theme.u(230) - Theme.u(24) - Theme.gut
            height: parent.height
            contentHeight: content.height + Theme.u(40)
            Column {
                id: content
                width: page.width
                spacing: Theme.u(8)

                // ---- Display
                Heading { visible: sv.tab === "display"; text: "Renderer"; topPadding: 0 }
                SetRow {
                    visible: sv.tab === "display"
                    width: parent.width
                    navKey: "s-rnd"
                    icon: "display"
                    text: "Renderer"
                    sub: "Automatic uses Electron (Chromium) on Nvidia graphics and WebKit everywhere else. Qt is Experimental. Drawing with " + sv.names[sv.r ? sv.r.running : "qt"] + " now."
                    value: (sv.names[sv.r ? sv.r.pick : "qt"] || "") + " ›"
                    onActivated: sv.pickRenderer()
                }
                Text {
                    visible: sv.tab === "display"
                    width: parent.width
                    wrapMode: Text.WordWrap
                    color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(13)
                    text: "If the Qt renderer can't bring Hearth's screen up, or crashes 3 times within a few minutes, Hearth goes back to Automatic by itself and says so when it starts."
                        + (sv.r && sv.r.env ? "\nHEARTH_RENDERER=" + sv.r.env + " is set, and wins over this setting." : "")
                }
                SetRow {
                    visible: sv.tab === "display"
                    width: parent.width
                    navKey: "s-web"
                    icon: "globe"
                    text: "Open in the web renderer"
                    sub: "Starts Hearth again with the web renderer this once (everything is there); this setting stays as it is."
                    onActivated: App.openInWeb()
                }

                // ---- System
                Heading { visible: sv.tab === "system"; text: "About"; topPadding: 0 }
                Rectangle {
                    visible: sv.tab === "system"
                    width: parent.width
                    height: about.implicitHeight + Theme.u(28)
                    radius: Theme.radius
                    color: Theme.panel
                    Column {
                        id: about
                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.u(14) }
                        spacing: Theme.u(6)
                        Repeater {
                            model: [
                                ["Hearth", (App.hostInfo.version || "?") + (App.hostInfo.name ? " · " + App.hostInfo.name : "")],
                                ["Renderer", "Qt Quick " + (typeof hostApp !== "undefined" ? hostApp.qtVersion + " (Experimental), hearth-qml " + hostApp.version : "(Experimental)")],
                                ["Controllers", typeof padInput !== "undefined" && padInput.available ? (padInput.count ? padInput.names : "None connected") : "Read by Hearth (no SDL2 here)"],
                                ["Games", App.games.length + " in your library"]
                            ]
                            delegate: Row {
                                required property var modelData
                                spacing: Theme.u(12)
                                Text { width: Theme.u(130); text: modelData[0]; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(14) }
                                Text { width: about.width - Theme.u(142); text: modelData[1]; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(14); wrapMode: Text.WordWrap }
                            }
                        }
                    }
                }
                Heading { visible: sv.tab === "system"; text: "Send logs" }
                SetRow {
                    visible: sv.tab === "system"
                    width: parent.width
                    navKey: "s-logs"
                    icon: "refresh"
                    text: sv.sending ? "Sending…" : "Send logs to the developer"
                    sub: "Hearth's logs, with the Qt renderer's own (hearth-qml.log), go to the developer. You get a code to tell them."
                        + (sv.logResult && sv.logResult.code ? " Last code: " + sv.logResult.code : "")
                    onActivated: sv.sendLogs()
                }
                SetRow {
                    visible: sv.tab === "system"
                    width: parent.width
                    navKey: "s-restart"
                    icon: "restart"
                    text: "Restart Hearth"
                    sub: "Starts Hearth again (the renderer is decided again)."
                    onActivated: { App.toast("Restarting Hearth…"); App.call("app.restart").catch(e => App.toast(e.message)); }
                }

                // ---- Everything else
                Heading { visible: sv.tab === "more"; text: "Everything else"; topPadding: 0 }
                Rectangle {
                    visible: sv.tab === "more"
                    width: parent.width
                    height: moreCol.implicitHeight + Theme.u(32)
                    radius: Theme.radius
                    color: Theme.panel
                    Column {
                        id: moreCol
                        anchors { left: parent.left; right: parent.right; top: parent.top; margins: Theme.u(16) }
                        spacing: Theme.u(12)
                        Text { text: "Not in the Qt renderer yet"; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(16); font.weight: Font.DemiBold }
                        Text {
                            width: parent.width; wrapMode: Text.WordWrap; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(14)
                            text: "Wi-Fi, Bluetooth, sound, controllers and Handheld Daemon, the keyboard, profiles and accounts, stores and sign-ins, updates, Hearth Link, accessibility and the rest of Settings are in the web renderer for now. They work the same; only the drawing is different."
                        }
                        Btn { text: "Open in the web renderer"; icon: "globe"; navKey: "s-more-web"; navFirst: sv.tab === "more"; onActivated: App.openInWeb() }
                    }
                }
            }
        }
    }
}
