// Hearth Settings › Phone › Messages (docs/PHONE-LINK.md): your phone's text messages (KDE Connect's SMS, Android):
// the conversations, newest first, with names from the phone's contacts; open one to read it and reply; or start a
// new message to a number. Nothing is saved on this PC: the phone keeps them. Through hearth-phone (SetPhone.qml).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Messages"
    property var st: null
    property var threads: []
    property var messages: []
    property int open: -1              // the conversation shown, by thread id
    property string msg: ""
    property bool bad: false
    readonly property var s: st && st.state ? st.state : null
    readonly property var phones: s ? (s.devices || []).filter(d => d.paired) : []
    readonly property var phone: phones.find(d => d.reachable) || phones[0] || null
    readonly property var thread: threads.find(t => t.thread === open) || null

    function say(t, b) { msg = t; bad = !!b; }
    function refresh() {
        job.run(["state"], r => {
            page.replaceList("st", r);
            if (!page.phone) return;
            job.run(["call", "sms_threads"], t => page.replaceList("threads", t.list || []), null, JSON.stringify({ id: page.phone.id }));
            if (page.open >= 0) job.run(["call", "sms_thread"], m => page.replaceList("messages", m.list || []), null, JSON.stringify({ id: page.phone.id, thread: page.open }));
        }, e => page.replaceList("st", { error: e }));
    }
    function call(cmd, args, ok) { act.run(["call", cmd], () => { if (ok) page.say(ok); page.refresh(); }, e => page.say(e, true), JSON.stringify(args || {})); }
    function when(ms) {
        if (!ms) return "";
        const d = new Date(ms), now = new Date();
        return d.toDateString() === now.toDateString() ? Qt.formatTime(d, "hh:mm") : Qt.formatDate(d, "d MMM");
    }
    // `hearth-settings phone-messages:<thread>` opens at that conversation.
    function takeAnchor() { if (/^\d+$/.test(SettingsNav.pageAnchor)) { open = parseInt(SettingsNav.pageAnchor); SettingsNav.pageAnchor = ""; messages = []; refresh(); } }
    Connections { target: SettingsNav; function onPageAnchorChanged() { page.takeAnchor(); } }
    Component.onCompleted: {
        takeAnchor();
        refresh();
        Qt.callLater(() => { if (page.phone) page.call("sms_refresh", { id: page.phone.id }); });
    }
    Timer { interval: 3000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !act.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-phone" }
    SetJob { id: act; helper: "hearth-phone" }

    Text {
        visible: !page.phone
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "Pair your phone first (Settings › Phone)."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text { visible: page.msg !== ""; text: page.msg; color: page.bad ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: 13; Layout.fillWidth: true; Layout.leftMargin: 4; wrapMode: Text.WordWrap }

    // ---- one conversation
    SetHeader { visible: !!page.thread; text: page.thread ? (page.thread.names || page.thread.addresses || []).join(", ") : "" }
    SetRow { visible: !!page.thread; icon: "back"; title: "All conversations"; chevron: false; onActivated: { page.open = -1; page.messages = []; } }
    Repeater {
        model: page.thread ? page.messages.slice(-30) : []
        delegate: Item {
            required property var modelData
            Layout.fillWidth: true
            implicitHeight: bubble.implicitHeight + 6
            Rectangle {
                id: bubble
                anchors.right: modelData.sent ? parent.right : undefined
                anchors.left: modelData.sent ? undefined : parent.left
                width: Math.min(parent.width * 0.72, body.implicitWidth + 28)
                implicitHeight: body.implicitHeight + meta.implicitHeight + 20
                radius: Theme.r(14)
                color: modelData.sent ? Theme.accent : SetStyle.card
                border.width: modelData.sent ? 0 : 1
                border.color: SetStyle.tint(Theme.text, 0.05)
                Column {
                    anchors { fill: parent; margins: 10; leftMargin: 14; rightMargin: 14 }
                    spacing: 2
                    Text { id: body; width: Math.min(implicitWidth, bubble.parent.width * 0.72 - 28); text: modelData.body; wrapMode: Text.Wrap; color: modelData.sent ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 14 }
                    Text { id: meta; text: page.when(modelData.date); color: modelData.sent ? Theme.accentInk : Theme.muted; opacity: 0.8; font.family: Theme.font; font.pixelSize: 11 }
                }
            }
        }
    }
    SetField {
        id: reply
        visible: !!page.thread
        placeholder: "Text message"
        onSubmitted: t => { if (t.trim()) { page.call("sms_send", { id: page.phone.id, thread: page.open, to: page.thread.addresses, text: t.trim() }, "Sent"); reply.text = ""; } }
    }

    // ---- every conversation
    SetHeader { visible: !!page.phone && !page.thread; text: "Conversations on " + (page.phone ? page.phone.name : "") }
    Text {
        visible: !!page.phone && !page.thread && !page.threads.length
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.phone && page.phone.reachable ? "Loading… On the phone, KDE Connect asks to read your messages the first time: allow it." : "They show here while the phone is connected."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.phone && !page.thread ? page.threads.slice(0, 40) : []
        delegate: SetRow {
            required property var modelData
            icon: "chat"
            title: (modelData.names || modelData.addresses || []).join(", ")
            sub: (modelData.sent ? "You: " : "") + modelData.last
            value: page.when(modelData.date)
            badge: modelData.unread ? modelData.unread + " new" : ""
            onActivated: { page.open = modelData.thread; page.messages = []; page.refresh(); }
        }
    }
    SetHeader { visible: !!page.phone && !page.thread; text: "New message" }
    SetField { id: newTo; visible: !!page.phone && !page.thread; placeholder: "To: phone number" }
    SetField {
        id: newBody
        visible: !!page.phone && !page.thread
        placeholder: "Message"
        onSubmitted: t => { if (newTo.text.trim() && t.trim()) { page.call("sms_send", { id: page.phone.id, to: newTo.text.trim(), text: t.trim() }, "Sent to " + newTo.text.trim()); newBody.text = ""; } else page.say("Type a number and a message", true); }
    }
}
