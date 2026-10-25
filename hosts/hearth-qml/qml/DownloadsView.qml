// Downloads (the web page's viewDownloads): the current download big with its controls and the speed, what's next,
// and what was installed lately.
import QtQuick

Item {
    id: dv
    readonly property var items: { App.rev; return App.games.filter(App.isActive); }
    readonly property var cur: items.find(x => x.state === "downloading") || items.find(x => x.state !== "paused") || (items.length ? items[0] : null)
    readonly property var next: items.filter(x => x !== cur)
    readonly property var done: { App.rev; return (App.dl.recent || []).map(r => App.g(r.id)).filter(x => x && x.inst).slice(0, 8); }
    readonly property real speed: items.reduce((t, x) => t + (x.state === "downloading" ? (x.speed || 0) : 0), 0)
    function ours(x) { return x && x.installKind === "nimbus"; }
    function gbs(b) { return b > 0 ? (b / 1e9).toFixed(b >= 1e10 ? 0 : 1) + " GB" : ""; }
    function fmtSpeed(b) { return b >= 1e6 ? (b / 1e6).toFixed(1) + " MB/s" : Math.round((b || 0) / 1e3) + " KB/s"; }

    Scroller {
        id: page
        anchors.fill: parent
        contentHeight: col.height + Theme.u(40)
        Column {
            id: col
            x: Theme.gut
            width: page.width - Theme.gut * 2
            topPadding: Theme.u(18)
            spacing: Theme.u(12)

            Rectangle {
                visible: !dv.cur
                width: parent.width
                height: Theme.u(84)
                radius: Theme.radius
                color: Theme.panel
                Column {
                    anchors { left: parent.left; leftMargin: Theme.u(20); verticalCenter: parent.verticalCenter }
                    Text { text: "No downloads"; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(17); font.weight: Font.DemiBold }
                    Text { text: "Everything is up to date"; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(13) }
                }
                Focusable { anchors.fill: parent; navKey: "dl-none"; navFirst: true; ring: true }
            }

            Row {
                visible: !!dv.cur
                width: parent.width
                spacing: Theme.u(16)
                Rectangle {
                    width: parent.width * 0.62
                    height: Theme.u(250)
                    radius: Theme.radius
                    color: Theme.panel
                    clip: true
                    ArtImage { anchors.fill: parent; game: dv.cur; kinds: ["hero", "wide", "cover"]; opacity: 0.35 }
                    Column {
                        anchors { left: parent.left; right: parent.right; bottom: parent.bottom; margins: Theme.u(20) }
                        spacing: Theme.u(8)
                        Text { text: dv.cur ? App.stateLabel(dv.cur) : ""; color: Theme.accent; font.family: Theme.font; font.pixelSize: Theme.u(13); font.weight: Font.Bold; font.capitalization: Font.AllUppercase }
                        Text { text: dv.cur ? dv.cur.title : ""; color: Theme.text; font.family: Theme.display; font.pixelSize: Theme.u(26); font.weight: Font.Bold; width: parent.width; elide: Text.ElideRight }
                        Bar { width: parent.width; value: dv.cur && dv.cur.progress != null ? dv.cur.progress : 0 }
                        Text {
                            color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(13)
                            text: !dv.cur ? "" : [dv.cur.progress != null ? Math.round(dv.cur.progress * 100) + "%" : "",
                                   dv.cur.bytesTotal ? dv.gbs(dv.cur.bytesDone) + " of " + dv.gbs(dv.cur.bytesTotal) : "",
                                   dv.cur.state === "downloading" && dv.cur.speed > 1e4 ? dv.fmtSpeed(dv.cur.speed) : "",
                                   App.sourceName(dv.cur)].filter(s => s).join("  ·  ")
                        }
                        Row {
                            spacing: Theme.u(10)
                            Btn { visible: dv.ours(dv.cur); navKey: "dc-pp"; navFirst: !!dv.cur; icon: dv.cur && dv.cur.state === "paused" ? "play" : "pause"; text: dv.cur && dv.cur.state === "paused" ? "Resume" : "Pause"
                                  onActivated: App.dlCall(dv.cur.state === "paused" ? "game.resumeInstall" : "game.pauseInstall", dv.cur.id) }
                            Btn { visible: dv.ours(dv.cur) || (dv.cur && dv.cur.stage === "Waiting for Steam"); navKey: "dc-x"; kind: "ghost"; icon: "x"; text: "Cancel"
                                  onActivated: App.openSheet({ title: "Cancel " + dv.cur.title + "?", sub: "What it downloaded so far is kept, so installing it again later carries on from there.", items: [{ label: "Cancel download", icon: "x", danger: true, run: () => App.dlCall("game.cancelInstall", dv.cur.id) }] }) }
                            Btn { visible: !!dv.cur && !dv.ours(dv.cur) && dv.cur.src === "steam"; navKey: "dc-steam"; navFirst: !!dv.cur && !dv.ours(dv.cur); kind: "ghost"; icon: "display"; text: "Manage in Steam"; onActivated: App.call("steam.downloads").catch(e => App.toast(e.message)) }
                            Btn { visible: !!dv.cur; navKey: "dc-info"; kind: "ghost"; icon: "info"; text: "Details"; onActivated: App.navigate("game", { gameId: dv.cur.id }) }
                        }
                    }
                }
                Rectangle {
                    width: parent.width * 0.38 - Theme.u(16)
                    height: Theme.u(250)
                    radius: Theme.radius
                    color: Theme.panel
                    Column {
                        anchors { fill: parent; margins: Theme.u(18) }
                        spacing: Theme.u(8)
                        Text { text: "Download speed"; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(12) }
                        Text { text: dv.fmtSpeed(dv.speed); color: Theme.text; font.family: Theme.display; font.pixelSize: Theme.u(30); font.weight: Font.Bold }
                        Item {
                            width: parent.width
                            height: Theme.u(110)
                            readonly property var pts: (App.dl.speeds || []).slice(-40)
                            readonly property real peak: Math.max(1, ...pts.map(p => p[1] || 0))
                            Row {
                                anchors.bottom: parent.bottom
                                spacing: Theme.u(2)
                                Repeater {
                                    model: parent.parent.pts
                                    delegate: Rectangle {
                                        required property var modelData
                                        width: Math.max(2, (col.width * 0.38 - Theme.u(52)) / 40 - Theme.u(2))
                                        height: Math.max(1, (modelData[1] || 0) / parent.parent.peak * Theme.u(110))
                                        anchors.bottom: parent.bottom
                                        radius: Theme.u(2)
                                        color: Theme.alpha(Theme.accent, 0.7)
                                    }
                                }
                            }
                        }
                        Text { text: "Last few minutes"; color: Theme.faint; font.family: Theme.font; font.pixelSize: Theme.u(11) }
                    }
                }
            }

            Heading { visible: dv.next.length > 0; text: "Up next · " + dv.next.length }
            Repeater {
                model: dv.next
                delegate: DlRow {
                    required property var modelData
                    game: modelData
                    sub: App.stateLabel(modelData) + (modelData.bytesTotal ? " · " + dv.gbs(modelData.bytesTotal) : "") + " · " + App.sourceName(modelData)
                }
            }
            Heading { visible: dv.done.length > 0; text: "Recently installed" }
            Repeater {
                model: dv.done
                delegate: DlRow {
                    required property var modelData
                    game: modelData
                    sub: "Installed · " + App.sourceName(modelData)
                }
            }
        }
    }

    component DlRow: Focusable {
        id: row
        property var game: null
        property string sub: ""
        width: parent ? parent.width : 0
        height: Theme.u(72)
        navKey: "dq-" + (game ? game.id : "")
        onActivated: if (game) App.navigate("game", { gameId: game.id })
        Rectangle { anchors.fill: parent; radius: row.radius; color: row.isFocused || row.hovered ? Theme.panel2 : Theme.panel }
        Rectangle {
            id: thumb
            x: Theme.u(10); anchors.verticalCenter: parent.verticalCenter
            width: Theme.u(96); height: Theme.u(54); radius: Theme.u(6); color: Theme.panel3; clip: true
            ArtImage { anchors.fill: parent; game: row.game; kinds: ["wide", "hero", "cover"] }
        }
        Column {
            anchors { left: thumb.right; leftMargin: Theme.u(14); right: parent.right; rightMargin: Theme.u(14); verticalCenter: parent.verticalCenter }
            spacing: Theme.u(4)
            Text { text: row.game ? row.game.title : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: Theme.u(15); font.weight: Font.DemiBold; elide: Text.ElideRight; width: parent.width }
            Text { text: row.sub; color: Theme.dim; font.family: Theme.font; font.pixelSize: Theme.u(12); elide: Text.ElideRight; width: parent.width }
            Bar { visible: App.isActive(row.game) && row.game.progress > 0; width: parent.width; value: row.game && row.game.progress ? row.game.progress : 0 }
        }
    }
}
