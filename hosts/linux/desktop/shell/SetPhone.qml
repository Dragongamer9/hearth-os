// Hearth Settings › Bluetooth & devices › Phone (docs/PHONE-LINK.md): your phone linked to this PC with the free KDE
// Connect app (Android or iPhone), through Hearth Connect's own service (hosts/hearth-connect, KDE Connect's protocol).
// Phone link on or off; pairing (with the key both screens show); your phones with battery and signal; Ring, Ping,
// clipboard, send files, take a photo, browse the phone's files; the phone's music; its notifications with Reply; calls
// (Mute ringer); what each phone may do here; Run commands; text messages (SetPhoneMessages.qml). Through hearth-phone,
// which only asks the service: every choice is the service's, saved safely by it (settings never change by themselves).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Phone"
    property var st: null              // hearth-phone state
    property string msg: ""
    property bool bad: false
    property string sel: ""            // the phone picked in Your phones
    readonly property var s: st && st.state ? st.state : null
    readonly property bool on: !!st && st.running
    readonly property var devices: s ? s.devices || [] : []
    readonly property var phones: devices.filter(d => d.paired)
    readonly property var nearby: devices.filter(d => !d.paired && d.reachable)
    readonly property var asking: devices.filter(d => d.pair && d.pair.state === "asking")
    readonly property var phone: phones.find(d => d.id === sel) || phones.find(d => d.reachable) || phones[0] || null
    readonly property var plug: phone && phone.plugins ? phone.plugins : ({})
    readonly property var opts: phone && phone.options ? phone.options : ({})
    readonly property bool linked: !!phone && phone.reachable

    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["state"], r => page.replaceList("st", r), e => page.replaceList("st", { error: e })); }
    function call(cmd, args, ok, done) {
        act.run(["call", cmd], r => { if (ok) page.say(ok); if (done) done(r); page.refresh(); }, e => { page.say(e, true); page.refresh(); }, JSON.stringify(args || {}));
    }
    function battery(d) {
        if (!d || !d.battery || d.battery.charge < 0) return "";
        return d.battery.charge + "%" + (d.battery.charging ? " charging" : "");
    }
    function signal(d) {
        const sg = d && d.plugins ? d.plugins.signal : null;
        if (!sg) return "";
        const k = Object.keys(sg)[0];
        if (k === undefined) return "";
        const x = sg[k];
        return (x.networkType || "") + (x.signalStrength !== undefined ? " " + "▂▄▆█".slice(0, Math.max(0, Math.min(4, x.signalStrength))) : "");
    }
    function phoneLine(d) {
        if (!d.reachable) return "Not connected · it connects by itself when it's on the same Wi-Fi";
        return ["Connected", battery(d) ? "battery " + battery(d) : "", signal(d)].filter(x => x).join(" · ");
    }
    // Send files…: Hearth Files' Open dialog (picker.qml, as every app's), then the files go to the phone.
    readonly property string runDir: Quickshell.env("XDG_RUNTIME_DIR") || "/tmp"
    readonly property string shellDir: decodeURIComponent(Qt.resolvedUrl(".").toString().replace(/^file:\/\//, "")).replace(/\/$/, "")
    Process {
        id: pick
        property string target: ""
        stdout: StdioCollector {
            onStreamFinished: {
                let r = null;
                try { r = JSON.parse(text.trim() || "{}"); } catch (e) { r = null; }
                const paths = r && r.paths ? r.paths.map(p => String(p).replace(/^file:\/\//, "")) : [];
                if (paths.length) page.call("share", { id: pick.target, paths: paths }, paths.length === 1 ? "Sending 1 file…" : "Sending " + paths.length + " files…");
            }
        }
    }
    function sendFiles() {
        if (pick.running || !phone) return;
        const req = { kind: "open", title: "Send to " + phone.name, accept: "Send", multiple: true, directory: false, filters: [], filter: -1,
                      folder: Quickshell.env("HOME") || "/", name: "", files: [], choices: [], app: "", parent: "", raw: [] };
        const rq = runDir + "/hearth-phone-pick-req.json", rs = runDir + "/hearth-phone-pick-res.json";
        pick.target = phone.id;
        pick.command = ["sh", "-c", 'rm -f "$3"; printf "%s" "$1" > "$2" && env HEARTH_PICK_REQUEST="$2" HEARTH_PICK_RESULT="$3" quickshell -p "$4" >/dev/null 2>&1; cat "$3" 2>/dev/null; rm -f "$2" "$3"',
                        "sh", JSON.stringify(req), rq, rs, shellDir + "/picker.qml"];
        pick.running = true;
    }

    // `hearth-settings phone:notifications` / `phone:commands` / `phone:pair` open at that part (a notification's Reply
    // button, the phone's "add commands"): its first card takes focus once the state is in, and the page scrolls to it.
    property string anchor: ""
    Component.onCompleted: { if (["notifications", "commands", "pair"].includes(SettingsNav.pageAnchor)) { anchor = SettingsNav.pageAnchor; SettingsNav.pageAnchor = ""; } refresh(); }
    onStChanged: if (anchor !== "" && on) Qt.callLater(() => {
        const to = { notifications: notifHead, commands: cmdName, pair: ipField }[anchor];
        anchor = "";
        if (to && to.visible) { if (to.focusInput) to.focusInput(); else to.forceActiveFocus(); }
    })
    Timer { interval: 2000; running: page.visible; repeat: true; onTriggered: if (!job.busy && !act.busy) page.refresh() }
    SetJob { id: job; helper: "hearth-phone" }
    SetJob { id: act; helper: "hearth-phone" }

    Text {
        Layout.fillWidth: true
        Layout.bottomMargin: 4
        text: "Get your phone's notifications and messages here, share files and the clipboard both ways, use the phone as a touchpad or a presentation remote, and more. Install the free KDE Connect app on your Android phone or iPhone, and have both on the same Wi-Fi."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetBanner {
        visible: !!page.st && !page.st.installed
        kind: "bad"; title: "Hearth Connect isn't installed"; sub: "It comes with Hearth OS. Updating Hearth puts it back."
    }
    SetSwitch {
        visible: !!page.st && page.st.installed
        icon: "phone"; title: "Phone link"
        sub: page.on ? "Phones on your Wi-Fi find this PC as " + (page.s ? page.s.name : "") : "Off: this PC doesn't listen for phones"
        checked: page.on
        onToggled: v => act.run([v ? "on" : "off"], r => { page.replaceList("st", r); page.say(v ? "Phone link is on" : "Phone link is off"); }, e => { page.say(e, true); page.refresh(); })
    }
    Text { visible: page.msg !== ""; text: page.msg; color: page.bad ? Theme.danger : Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.fillWidth: true; Layout.leftMargin: 4 }

    // ---- someone asking to pair, and calls
    Repeater {
        model: page.on ? page.asking : []
        delegate: ColumnLayout {
            required property var modelData
            Layout.fillWidth: true
            spacing: 6
            SetBanner {
                icon: "lock"; kind: "info"; action: "Accept"
                title: modelData.name + " wants to pair"
                sub: "Check that the phone shows " + (modelData.pair.key || "") + ", then accept. Only accept phones you know."
                onActivated: page.call("accept", { id: modelData.id }, "Paired with " + modelData.name)
            }
            SetRow { icon: "close"; title: "Decline"; sub: "Don't pair with " + modelData.name; onActivated: page.call("reject", { id: modelData.id }, "Declined") }
        }
    }
    SetBanner {
        visible: page.on && !!page.plug.call
        icon: "phone"; kind: "info"; action: "Mute ringer"
        title: page.plug.call ? (page.plug.call.state === "ringing" ? page.plug.call.who + " is calling" : "In a call with " + page.plug.call.who) : ""
        sub: page.phone ? "On " + page.phone.name + (page.opts.pauseOnCall ? " · what was playing here is paused until the call ends" : "") : ""
        onActivated: page.call("mute_call", { id: page.phone.id }, "Ringer muted")
    }

    // ---- your phones
    SetHeader { text: "Your phones"; visible: page.on }
    Repeater {
        model: page.on ? page.phones : []
        delegate: SetRow {
            required property var modelData
            icon: "phone"; title: modelData.name
            sub: page.phoneLine(modelData)
            chip: modelData.reachable ? "Connected" : "Away"; chipKind: modelData.reachable ? "ok" : "off"
            value: page.phones.length > 1 && page.phone && page.phone.id === modelData.id ? "Shown below" : ""
            onActivated: page.sel = modelData.id
        }
    }
    Text {
        visible: page.on && !page.phones.length
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "No phone yet. Open KDE Connect on your phone, pick " + (page.s ? page.s.name : "this PC") + " and tap Pair. It shows up here to accept."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    // ---- the phone shown
    SetHeader { text: page.phone ? page.phone.name : ""; visible: page.on && !!page.phone }
    Flow {
        visible: page.on && !!page.phone
        Layout.fillWidth: true
        spacing: 8
        SetButton { text: "Ring"; icon: "bell"; enabled: page.linked; onActivated: page.call("ring", { id: page.phone.id }, page.phone.name + " is ringing") }
        SetButton { text: "Ping"; icon: "sparkle"; enabled: page.linked; onActivated: page.call("ping", { id: page.phone.id }, "Pinged " + page.phone.name) }
        SetButton { text: "Send clipboard"; icon: "clipboard"; enabled: page.linked; onActivated: page.call("clipboard", { id: page.phone.id }, "Clipboard sent") }
        SetButton { text: "Send files…"; icon: "share"; enabled: page.linked; onActivated: page.sendFiles() }
        SetButton { text: "Take a photo"; icon: "cam"; enabled: page.linked; onActivated: page.call("photo", { id: page.phone.id }, "Take the photo on " + page.phone.name + "; it comes to Downloads") }
        SetButton { text: page.plug.sftp && page.plug.sftp.mounted ? "Open its files" : "Browse its files"; icon: "folder"; enabled: page.linked
            onActivated: {
                if (page.plug.sftp && page.plug.sftp.mounted) Quickshell.execDetached(["hearth-filemanager", page.plug.sftp.mounted]);
                else page.call("browse", { id: page.phone.id }, "Asking " + page.phone.name + " to share its files… allow it on the phone if it asks");
            }
        }
    }
    SetRow {
        visible: page.on && !!page.plug.sftp && (!!page.plug.sftp.mounted || page.plug.sftp.waiting || !!page.plug.sftp.error)
        icon: "folder"
        title: page.plug.sftp && page.plug.sftp.mounted ? "The phone's files are open" : page.plug.sftp && page.plug.sftp.waiting ? "Opening the phone's files…" : "Couldn't open the phone's files"
        sub: page.plug.sftp ? (page.plug.sftp.mounted ? (page.plug.sftp.folders || []).map(f => f.name).join(", ") + " · select to open in Hearth Files" : page.plug.sftp.error || "Allow it on the phone if it asks") : ""
        chip: page.plug.sftp && page.plug.sftp.mounted ? "Open" : ""
        onActivated: if (page.plug.sftp && page.plug.sftp.mounted) Quickshell.execDetached(["hearth-filemanager", page.plug.sftp.mounted])
    }
    SetRow {
        visible: page.on && !!page.plug.sftp && !!page.plug.sftp.mounted
        icon: "close"; title: "Close the phone's files"; chevron: false
        onActivated: page.call("unmount", { id: page.phone.id }, "Closed")
    }
    // The phone's music: what plays on it, controlled from here.
    Repeater {
        model: page.on && page.phone ? page.phone.players.slice(0, 2) : []
        delegate: ColumnLayout {
            required property var modelData
            readonly property var now: page.phone && page.phone.playing ? page.phone.playing[modelData] || ({}) : ({})
            Layout.fillWidth: true
            spacing: 6
            SetRow {
                icon: "headphones"; chevron: false
                title: now.title || now.nowPlaying || modelData
                sub: [now.artist, modelData + " on " + page.phone.name].filter(x => x).join(" · ")
                chip: now.isPlaying ? "Playing" : "Paused"; chipKind: now.isPlaying ? "ok" : "off"
            }
            Row {
                spacing: 8
                SetButton { text: "Previous"; onActivated: page.call("media", { id: page.phone.id, player: modelData, action: "Previous" }) }
                SetButton { text: now.isPlaying ? "Pause" : "Play"; primary: true; onActivated: page.call("media", { id: page.phone.id, player: modelData, action: "PlayPause" }) }
                SetButton { text: "Next"; onActivated: page.call("media", { id: page.phone.id, player: modelData, action: "Next" }) }
            }
        }
    }
    SetRow {
        visible: page.on && !!page.phone
        icon: "chat"; title: "Messages"
        sub: page.plug.sms && page.plug.sms.threads ? page.plug.sms.threads + " conversations · read them and send a text from here" : "Read your text messages and send one from here (Android)"
        chevron: true
        onActivated: SettingsNav.openPage("phone-messages")
    }

    // ---- its notifications, with Reply
    SetHeader { text: "Notifications from " + (page.phone ? page.phone.name : ""); visible: page.on && !!page.phone && page.opts.notifications !== false }
    SetRow {
        id: notifHead
        visible: page.on && !!page.phone && page.opts.notifications !== false && (page.phone.notifications || []).length > 0
        icon: "bell"; chevron: false
        title: (page.phone ? (page.phone.notifications || []).length : 0) + " on " + (page.phone ? page.phone.name : "")
        sub: "Open one to reply, use its buttons or dismiss it (on the phone too)"
        onActivated: if (page.phone) page.call("refresh", {})
    }
    Text {
        visible: page.on && !!page.phone && page.opts.notifications !== false && !(page.phone.notifications || []).length
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.linked ? "None right now. On the phone, KDE Connect asks to read notifications the first time: allow it." : "They show here while the phone is connected."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.on && page.phone && page.opts.notifications !== false ? (page.phone.notifications || []).slice(0, 8) : []
        delegate: SetExpander {
            id: nx
            required property var modelData
            icon: "bell"
            title: [modelData.appName, modelData.title].filter(x => x).join(": ")
            sub: modelData.text
            value: modelData.replyId ? "Reply" : ""
            SetField {
                visible: !!nx.modelData.replyId
                placeholder: "Reply to " + (nx.modelData.title || nx.modelData.appName)
                onSubmitted: t => { if (t.trim()) { page.call("reply", { id: page.phone.id, replyId: nx.modelData.replyId, message: t.trim() }, "Reply sent"); text = ""; } }
            }
            Repeater {
                model: nx.modelData.actions || []
                delegate: SetRow {
                    required property var modelData
                    icon: "forward"; title: modelData; chevron: false
                    onActivated: page.call("action", { id: page.phone.id, notification: nx.modelData.id, action: modelData }, modelData)
                }
            }
            SetRow {
                visible: nx.modelData.dismissable !== false
                icon: "close"; title: "Dismiss"; sub: "On the phone too"; chevron: false
                onActivated: page.call("dismiss", { id: page.phone.id, notification: nx.modelData.id })
            }
        }
    }

    // ---- what the phone may do
    SetHeader { text: "What " + (page.phone ? page.phone.name : "") + " can do"; visible: page.on && !!page.phone }
    Repeater {
        model: page.on && page.phone ? [
            { key: "notifications", icon: "bell", title: "Show its notifications here", sub: "With Reply for messages" },
            { key: "clipboard", icon: "clipboard", title: "Share the clipboard", sub: "Copy on one, paste on the other" },
            { key: "input", icon: "mouse", title: "Touchpad, keyboard and presentation remote", sub: "The phone moves the pointer and types here" },
            { key: "pauseOnCall", icon: "pause", title: "Pause media during calls", sub: "Plays again when the call ends" },
            { key: "commands", icon: "bolt", title: "Run commands", sub: "Only the commands below" },
            { key: "autoAccept", icon: "file", title: "Save its files without asking", sub: "Into " + (page.s ? page.s.folder : "Downloads") }
        ] : []
        delegate: SetSwitch {
            required property var modelData
            icon: modelData.icon; title: modelData.title; sub: modelData.sub
            checked: page.opts[modelData.key] === true
            onToggled: v => page.call("option", { id: page.phone.id, key: modelData.key, on: v }, modelData.title + (v ? ": on" : ": off"))
        }
    }

    // ---- run commands
    SetHeader { text: "Run commands"; visible: page.on }
    Text {
        visible: page.on
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: "Commands your phone can start with a tap (KDE Connect › Run command). Only these: the phone can't send its own."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Repeater {
        model: page.on && page.s ? page.s.commands || [] : []
        delegate: SetExpander {
            id: cx
            required property var modelData
            icon: "bolt"; title: modelData.name; sub: modelData.command
            SetRow { icon: "trash"; danger: true; title: "Remove " + cx.modelData.name; chevron: false; onActivated: page.call("command_remove", { key: cx.modelData.key }, "Removed") }
        }
    }
    SetField { id: cmdName; visible: page.on; placeholder: "New command's name (Lock the screen)" }
    SetField { id: cmdLine; visible: page.on; placeholder: "Its command (loginctl lock-session)"; onSubmitted: addCmd.activated() }
    SetButton {
        id: addCmd
        visible: page.on
        text: "Add command"; icon: "plus"
        onActivated: { if (cmdName.text.trim() && cmdLine.text.trim()) { page.call("command_add", { name: cmdName.text.trim(), command: cmdLine.text.trim() }, "Added " + cmdName.text.trim()); cmdName.text = ""; cmdLine.text = ""; } else page.say("Type a name and a command", true); }
    }

    // ---- pair a new phone
    SetHeader { text: "Pair a new phone"; visible: page.on }
    Repeater {
        model: page.on ? page.nearby.filter(d => !(d.pair && d.pair.state === "asking")) : []
        delegate: SetRow {
            required property var modelData
            icon: "phone"; title: modelData.name
            sub: modelData.pair && modelData.pair.state === "requested" ? "Waiting for the phone… check it shows " + modelData.pair.key : "Select to pair"
            onActivated: page.call("pair", { id: modelData.id }, "Asked " + modelData.name + ": accept on the phone")
        }
    }
    SetRow {
        visible: page.on
        icon: "help"; chevron: false; interactive: false
        title: "Phone not listed?"
        sub: "Open KDE Connect on the phone (same Wi-Fi), or add this PC there by its address" + (page.s && page.s.tcpPort ? "" : "") + ". Some networks (guest Wi-Fi) keep devices apart."
    }
    SetField {
        id: ipField
        visible: page.on
        placeholder: "Or type the phone's address (192.168.1.23)"
        onSubmitted: t => { if (t.trim()) { page.call("add_ip", { ip: t.trim() }, "Looking for it at " + t.trim()); ipField.text = ""; } }
    }
    SetRow {
        visible: page.on && !!page.s
        icon: "folder"; title: "Files from your phone"; sub: page.s ? page.s.folder : ""
        onActivated: Quickshell.execDetached(["hearth-filemanager", page.s.folder])
    }
    SetRow {
        visible: page.on && !!page.phone
        icon: "trash"; danger: true; chevron: false
        title: page.phone ? "Forget " + page.phone.name : ""
        sub: "Unpairs it. Pair again from the phone to use it here"
        onActivated: page.call("forget", { id: page.phone.id }, "Forgot it")
    }
}
