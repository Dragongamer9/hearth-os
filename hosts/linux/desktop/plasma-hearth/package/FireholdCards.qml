// Firehold on Plasma's notification cards (the shared org.hearthos.notifications, style "firehold"; mockup
// docs/mockups/firehold-plasma/09-notification-cards.png): at the top right under the bar, each with its app, time,
// text, a picture or a reply box when it has one, and its buttons. An urgent one (critical, or Hearth's own
// warning) stays until you answer it; the others go after their time (6 s unless the app says) and stay in the list.
// Four at most, then "N more". Mantel's rules decide what comes in (Notifications.qml, DeskState's history); this
// only draws the cards, and holds them back during Do not disturb or a full-screen game (FhState.qml).
import QtQuick
import QtQuick.Layouts
import "fh.js" as F

LayerPanel {
    id: fc
    scopeName: "hearth-firehold-cards"
    anchors.top: true
    anchors.right: true
    margins.top: (Hearth.panelEdge === 0 ? Hearth.panelSize : 0) + 12
    margins.right: 14
    implicitWidth: 556
    implicitHeight: Math.max(1, col.implicitHeight)
    visible: Hearth.style === "firehold" && cards.length > 0
    color: "transparent"
    wantsKeys: typingIn !== -1
    readonly property string fontName: F.font(Qt.fontFamilies())
    property var cards: []                  // history keys, newest first
    property int typingIn: -1
    signal openList()

    Connections {
        target: DeskState
        function onPopped(h) {
            const n = DeskState.liveOf(h.key);
            const urgent = h.urgency === "critical" || (!!n && NotifyRules.isHearthWarning(n));
            FhState.askDnd(() => {
                if (!FhState.cardFor(h, urgent)) { DeskState.unpop(h.key); return; }
                if (fc.cards.indexOf(h.key) < 0) fc.cards = [h.key].concat(fc.cards);
            });
        }
        // Gone from the list (Clear all, × in the list): gone here too.
        function onHistoryChanged() { const ks = DeskState.history.map(x => x.key); fc.cards = fc.cards.filter(k => ks.indexOf(k) >= 0); }
    }
    function drop(key) { cards = cards.filter(k => k !== key); }

    Column {
        id: col
        width: parent.width
        spacing: 12
        Repeater {
            model: fc.cards.slice(0, 4)
            delegate: Rectangle {
                id: card
                required property var modelData
                required property int index
                readonly property var h: { void DeskState.history; return DeskState.item(modelData); }
                readonly property var n: DeskState.liveOf(modelData)
                readonly property bool urgent: !!h && (h.urgency === "critical" || (!!n && NotifyRules.isHearthWarning(n)))
                readonly property var acts: n ? Array.from(n.actions || []).filter(a => a.identifier !== "default" && a.text) : []
                readonly property var defaultAct: n ? Array.from(n.actions || []).find(a => a.identifier === "default") : null
                readonly property string image: h && h.image ? (String(h.image).startsWith("/") ? "file://" + h.image : h.image) : ""
                width: col.width
                visible: !!h
                height: visible ? body.implicitHeight + 38 : 0
                radius: 14
                color: urgent ? "#2a1d17" : "#241e1b"
                border.width: urgent ? 1.5 : 1
                border.color: urgent ? F.accent : F.line
                Timer {
                    running: !card.urgent && !hover.containsMouse && fc.typingIn !== card.index
                    readonly property real t: card.n ? card.n.expireTimeout : 0
                    interval: t <= 0 ? 6000 : t > 100 ? t : t * 1000
                    onTriggered: { fc.drop(card.modelData); DeskState.unpop(card.modelData); }
                }
                MouseArea {
                    id: hover
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.LeftButton | Qt.RightButton
                    onClicked: mouse => {
                        if (mouse.button === Qt.RightButton) { menu.visible = !menu.visible; return; }
                        if (card.defaultAct || (card.h && card.h.entry)) { DeskState.invoke(card.modelData, "default"); fc.drop(card.modelData); }
                    }
                }
                // The app's tile: its picture, else its letter
                Rectangle {
                    id: tileR
                    x: 18; y: 20
                    width: 40; height: 40; radius: 10
                    color: F.tint(card.h ? card.h.app : "")
                    AppIcon {
                        id: ai
                        anchors.centerIn: parent
                        size: 24
                        icon: card.h ? card.h.icon : ""
                        appId: card.h ? card.h.entry : ""
                        genericFallback: false
                        visible: hasIcon
                    }
                    Text { anchors.centerIn: parent; visible: !ai.visible; text: F.initial(card.h ? (card.urgent ? card.h.summary : (card.h.app || card.h.summary)) : ""); color: "#ffffff"; font.pixelSize: 17; font.weight: Font.Bold; font.family: fc.fontName }
                }
                ColumnLayout {
                    id: body
                    anchors { left: tileR.right; leftMargin: 14; right: parent.right; rightMargin: 18 }
                    y: 18
                    spacing: 4
                    RowLayout {
                        Layout.fillWidth: true
                        Text { Layout.fillWidth: true; text: (card.h ? card.h.app : "") + " · " + (card.h ? F.ago(card.h.time, false) : ""); color: F.muted; font.pixelSize: 14; font.family: fc.fontName; elide: Text.ElideRight }
                        Text { text: "···"; color: F.muted; font.pixelSize: 15; font.family: fc.fontName
                            MouseArea { anchors.fill: parent; anchors.margins: -6; onClicked: menu.visible = !menu.visible } }
                        Item { Layout.preferredWidth: 10 }
                        Text { text: "×"; color: F.muted; font.pixelSize: 19; font.family: fc.fontName
                            MouseArea { anchors.fill: parent; anchors.margins: -6; onClicked: { fc.drop(card.modelData); DeskState.unpop(card.modelData); } } }
                    }
                    Text { Layout.fillWidth: true; text: card.h ? card.h.summary : ""; color: F.text; font.pixelSize: 17; font.weight: Font.DemiBold; font.family: fc.fontName; wrapMode: Text.Wrap; maximumLineCount: 2; elide: Text.ElideRight }
                    Text { Layout.fillWidth: true; visible: text !== ""; text: card.h ? card.h.body : ""; color: "#ddd2ca"; font.pixelSize: 15; font.family: fc.fontName; wrapMode: Text.Wrap; maximumLineCount: 3; elide: Text.ElideRight }
                    Text { visible: card.urgent; text: "Stays until you answer"; color: F.accent; font.pixelSize: 14; font.family: fc.fontName }
                    // A picture (a recorded clip, a screenshot)
                    Rectangle {
                        readonly property var live: card.image === "" && card.n && card.n.imageObj ? card.n.imageObj : null
                        visible: (card.image !== "" && pic.status === Image.Ready) || (!!live && liveImg.status === Loader.Ready)
                        Layout.topMargin: 6
                        Layout.preferredWidth: 220; Layout.preferredHeight: 124
                        radius: 8; clip: true; color: F.tile
                        Image { id: pic; anchors.fill: parent; visible: card.image !== ""; source: card.image; fillMode: Image.PreserveAspectCrop; asynchronous: true; sourceSize: Qt.size(440, 248) }
                        // Plasma's own picture of it (image-data, or an image-path it loaded): CardImage.qml.
                        Loader { id: liveImg; anchors.fill: parent; active: !!parent.live; source: "CardImage.qml"; onLoaded: item.img = Qt.binding(() => parent.live) }
                    }
                    // A reply box (messages)
                    Rectangle {
                        visible: !!card.n && card.n.hasInlineReply === true
                        Layout.fillWidth: true; Layout.topMargin: 6
                        implicitHeight: 38; radius: 8
                        color: "#2e2724"
                        border.width: input.activeFocus ? 1.5 : 0; border.color: F.accent
                        TextInput {
                            id: input
                            anchors { fill: parent; leftMargin: 12; rightMargin: 12 }
                            verticalAlignment: TextInput.AlignVCenter
                            color: F.text; font.pixelSize: 14; font.family: fc.fontName
                            onActiveFocusChanged: fc.typingIn = activeFocus ? card.index : (fc.typingIn === card.index ? -1 : fc.typingIn)
                            onAccepted: card.reply()
                            Text { anchors.verticalCenter: parent.verticalCenter; visible: input.text === ""; text: (card.n && card.n.inlineReplyPlaceholder ? card.n.inlineReplyPlaceholder : "Reply...") + " (Enter sends)"; color: F.muted; font: input.font }
                        }
                        MouseArea { anchors.fill: parent; visible: !input.activeFocus; onClicked: { fc.typingIn = card.index; input.forceActiveFocus(); } }
                    }
                    // Its buttons: the first is the main one
                    Flow {
                        Layout.fillWidth: true; Layout.topMargin: 8
                        visible: card.acts.length > 0 || (!!card.n && card.n.hasInlineReply === true)
                        spacing: 8
                        FhButton { visible: !!card.n && card.n.hasInlineReply === true; primary: true; text: "Reply"; onPressed: card.reply() }
                        Repeater {
                            model: card.acts
                            delegate: FhButton {
                                required property var modelData
                                required property int index
                                primary: index === 0 && !(card.n && card.n.hasInlineReply === true)
                                pad: 13
                                text: modelData.text
                                onPressed: { DeskState.invoke(card.modelData, modelData.identifier); fc.drop(card.modelData); }
                            }
                        }
                    }
                }
                function reply() {
                    const t = input.text.trim();
                    if (!t || !card.n) return;
                    card.n.sendInlineReply(t);
                    input.text = ""; input.focus = false; fc.typingIn = -1;
                    fc.drop(card.modelData);
                }
                // ···: mute this app for an hour (this session) or for good (Settings › Notifications turns it back)
                Rectangle {
                    id: menu
                    visible: false
                    z: 5
                    x: parent.width - width - 14; y: 44
                    width: 240; height: mcol.implicitHeight + 12; radius: 10
                    color: F.tile; border.color: F.line
                    Column {
                        id: mcol
                        x: 6; y: 6; width: parent.width - 12
                        Repeater {
                            model: [{ t: "Mute " + (card.h ? card.h.app : "") + " for 1 hour", w: "hour" }, { t: "Mute " + (card.h ? card.h.app : ""), w: "good" }, { t: "Notification settings", w: "settings" }]
                            delegate: Rectangle {
                                required property var modelData
                                width: mcol.width; height: 34; radius: 7
                                color: mma.containsMouse ? F.seg : "transparent"
                                Text { x: 10; anchors.verticalCenter: parent.verticalCenter; text: parent.modelData.t; color: F.text; font.pixelSize: 14; font.family: fc.fontName }
                                MouseArea {
                                    id: mma; anchors.fill: parent; hoverEnabled: true
                                    onClicked: {
                                        const w = parent.modelData.w, app = card.h ? card.h.app : "";
                                        menu.visible = false;
                                        if (w === "hour") FhState.muteHour(app);
                                        else if (w === "good") DeskState.setMuted(app, true);
                                        else SettingsNav.openPage("notifications");
                                        fc.drop(card.modelData);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // "2 more · …  Show"
        Rectangle {
            visible: fc.cards.length > 4
            width: col.width; height: 44; radius: 12
            color: "#241e1b"; border.color: F.line
            readonly property var rest: { void DeskState.history; return fc.cards.slice(4).map(k => DeskState.item(k)).filter(x => x); }
            Text {
                anchors { left: parent.left; leftMargin: 18; right: showT.left; rightMargin: 12; verticalCenter: parent.verticalCenter }
                textFormat: Text.StyledText
                text: "<b>" + (fc.cards.length - 4) + " more</b> · " + parent.rest.slice(0, 3).map(h => h.summary).join(" · ")
                color: F.muted; font.pixelSize: 14; font.family: fc.fontName; elide: Text.ElideRight
            }
            Text { id: showT; anchors { right: parent.right; rightMargin: 18; verticalCenter: parent.verticalCenter }
                text: "Show"; color: F.accent; font.pixelSize: 14; font.weight: Font.DemiBold; font.family: fc.fontName
                MouseArea { anchors.fill: parent; anchors.margins: -6; onClicked: fc.openList() } }
        }
    }
}
