// Hearth Settings › Wi-Fi (NetworkManager, through nmcli): the network you're on (name, signal, address; Properties,
// Disconnect, Forget), the Wi-Fi switch, the networks nearby, and Add a hidden network. Nothing here writes Hearth's
// own settings: every change is NetworkManager's, and only when you ask for it.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Wi-Fi"
    property bool on: false
    property var nets: []          // [{ssid, signal, secure, inUse, freq, security}]
    property string ip: ""         // the Wi-Fi device's address while connected
    property string device: ""     // the Wi-Fi device (wlan0)
    property var props: ({})       // Properties: {IP4.ADDRESS, IP4.GATEWAY, IP4.DNS, GENERAL.HWADDR, IP6.ADDRESS}
    property bool showProps: false
    property bool forgetAsk: false // Forget asks once more first
    property bool hiddenOpen: false
    readonly property var current: nets.find(n => n.inUse) || null
    property var saved: []         // the saved networks' SSIDs
    property var savedIds: ({})    // SSID: its saved connection's UUID
    property bool scanning: false
    property string asking: ""     // the network we're asking a password for
    property string busy: ""
    property string message: ""
    // A short list (the network you're on, saved ones, then the strongest others, up to 6); the rest behind
    // "Show all networks". Folds closed again when you leave the page.
    property bool showAll: false
    onVisibleChanged: if (!visible) { showAll = false; showProps = false; forgetAsk = false; hiddenOpen = false; }
    readonly property var firstNets: nets.filter(n => n.inUse || saved.includes(n.ssid) || busy === n.ssid)
    readonly property var otherNets: nets.filter(n => !(n.inUse || saved.includes(n.ssid) || busy === n.ssid))
    readonly property var shownNets: showAll ? firstNets.concat(otherNets) : firstNets.concat(otherNets.slice(0, Math.max(0, 6 - firstNets.length)))

    // nmcli -t escapes ':' inside fields as '\:'
    function fields(line) {
        const out = []; let cur = "";
        for (let i = 0; i < line.length; i++) {
            const c = line[i];
            if (c === "\\" && i + 1 < line.length) { cur += line[++i]; continue; }
            if (c === ":") { out.push(cur); cur = ""; continue; }
            cur += c;
        }
        out.push(cur);
        return out;
    }
    function refresh() { radio.running = true; savedProc.running = true; scan.running = true; scanning = true; addr.running = true; }
    Component.onCompleted: refresh()
    // Not while a password is being typed (the field would be rebuilt under you).
    Timer { interval: 15000; running: page.visible && page.asking === ""; repeat: true; onTriggered: page.refresh() }

    Process {
        id: radio
        command: ["nmcli", "radio", "wifi"]
        stdout: StdioCollector { onStreamFinished: page.on = text.trim() === "enabled" }
    }
    Process {
        id: savedProc
        // Each saved Wi-Fi connection as UUID:SSID. The SSID is the connection's own (802-11-wireless.ssid), not
        // its name, so a renamed connection still counts as saved; its name only stands in when there's no SSID.
        command: ["sh", "-c", 'nmcli -t -f UUID,TYPE,NAME connection show | while IFS=: read -r u t n; do case "$t" in *wireless*) s=$(nmcli --escape yes -g 802-11-wireless.ssid connection show uuid "$u" 2>/dev/null); printf "%s:%s\\n" "$u" "${s:-$n}";; esac; done']
        stdout: StdioCollector {
            onStreamFinished: {
                const ids = {};
                for (const l of text.split("\n")) { const f = page.fields(l); if (f.length >= 2 && f[0] && f[1]) ids[f[1]] = f[0]; }
                page.savedIds = ids;
                page.replaceList("saved", Object.keys(ids));
            }
        }
    }
    Process {
        id: scan
        command: ["nmcli", "-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY,FREQ", "device", "wifi", "list", "--rescan", "auto"]
        stdout: StdioCollector {
            onStreamFinished: {
                const seen = {}, out = [];
                for (const l of text.split("\n")) {
                    const f = page.fields(l);
                    if (f.length < 4 || !f[1] || seen[f[1]]) continue;
                    seen[f[1]] = true;
                    out.push({ ssid: f[1], signal: parseInt(f[2]) || 0, secure: f[3] !== "" && f[3] !== "--", inUse: f[0] === "*",
                               security: f[3] !== "--" ? f[3] : "", freq: parseInt(f[4] || "") || 0 });
                }
                out.sort((a, b) => (b.inUse - a.inUse) || (b.signal - a.signal));
                page.replaceList("nets", out);
                page.scanning = false;
            }
        }
    }
    // The Wi-Fi device and its address (for the card at the top).
    Process {
        id: addr
        command: ["sh", "-c", 'for d in $(nmcli -t -f DEVICE,TYPE device | awk -F: \'$2=="wifi"{print $1}\'); do a=$(nmcli -g IP4.ADDRESS device show "$d" 2>/dev/null | head -n1); printf "%s:%s\\n" "$d" "${a%%/*}"; [ -n "$a" ] && break; done']
        stdout: StdioCollector {
            onStreamFinished: {
                const l = text.trim().split("\n").filter(x => x.indexOf(":") > 0).pop() || "";
                page.device = l.split(":")[0] || "";
                page.ip = l.split(":").slice(1).join(":");
            }
        }
    }
    // Properties: the details NetworkManager has for the device (only read).
    Process {
        id: details
        command: ["nmcli", "-t", "-f", "IP4.ADDRESS,IP4.GATEWAY,IP4.DNS,IP6.ADDRESS,GENERAL.HWADDR", "device", "show", page.device || "wlan0"]
        stdout: StdioCollector {
            onStreamFinished: {
                const out = {};
                for (const l of text.split("\n")) {
                    const i = l.indexOf(":");
                    if (i <= 0) continue;
                    const k = l.slice(0, i).replace(/\[\d+\]$/, ""), v = l.slice(i + 1).replace(/\\:/g, ":");
                    if (v && !out[k]) out[k] = v;
                }
                page.props = out;
            }
        }
    }
    function band(f) { return f >= 5925 ? "6 GHz" : f >= 4900 ? "5 GHz" : f > 0 ? "2.4 GHz" : ""; }
    function signalWord(s) { return s > 75 ? "Excellent signal" : s > 50 ? "Good signal" : s > 25 ? "Fair signal" : "Weak signal"; }
    function disconnect(n) { page.run(["nmcli", "connection", "down"].concat(page.savedIds[n.ssid] ? ["uuid", page.savedIds[n.ssid]] : ["id", n.ssid]), "Disconnected from " + n.ssid); }
    // Forget: the saved connection is removed (its password too); asked once more first.
    function forget(n) {
        forgetAsk = false; showProps = false;
        if (savedIds[n.ssid]) run(["nmcli", "connection", "delete", "uuid", savedIds[n.ssid]], "Forgot " + n.ssid);
        else run(["nmcli", "connection", "delete", "id", n.ssid], "Forgot " + n.ssid);
    }
    // A network that doesn't say its name: nmcli connects to it by name, with "hidden yes".
    function connectHidden(ssid, pw) {
        const name = ssid.trim();
        if (!name) return;
        busy = name; message = ""; hiddenOpen = false;
        if (pw) run(["sh", "-c", 'printf "%s\\n" "$HEARTH_WIFI_PSK" | exec nmcli --ask device wifi connect "$1" hidden yes', "sh", name], "Connected to " + name, { HEARTH_WIFI_PSK: pw });
        else run(["nmcli", "device", "wifi", "connect", name, "hidden", "yes"], "Connected to " + name);
    }

    Process {
        id: action
        property string okText
        property string psk: ""
        // a binding, not an assignment in run(): assigning a JS object to environment threw on Hearth 1.0's Quickshell
        environment: ({ HEARTH_WIFI_PSK: action.psk })
        stdout: StdioCollector {}
        stderr: StdioCollector { id: actErr }
        onExited: code => { action.psk = ""; page.busy = ""; page.message = code === 0 ? action.okText : (actErr.text.trim().split("\n").pop() || "That didn't work"); page.refresh(); }
    }
    function run(cmd, okText, env) { action.okText = okText; action.psk = (env && env.HEARTH_WIFI_PSK) || ""; action.command = cmd; action.running = true; }
    function connect(n, pw) {
        busy = n.ssid; asking = ""; message = "";
        // The password goes to nmcli on its input (it asks for it with --ask), through the environment, which only
        // you can read: on its command line every account on the device could see it (ps, /proc).
        if (pw) run(["sh", "-c", 'printf "%s\\n" "$HEARTH_WIFI_PSK" | exec nmcli --ask device wifi connect "$1"', "sh", n.ssid], "Connected to " + n.ssid, { HEARTH_WIFI_PSK: pw });
        else if (savedIds[n.ssid]) run(["nmcli", "connection", "up", "uuid", savedIds[n.ssid]], "Connected to " + n.ssid);
        else run(["nmcli", "device", "wifi", "connect", n.ssid], "Connected to " + n.ssid);
    }
    function bars(s) { return s > 75 ? "▂▄▆█" : s > 50 ? "▂▄▆" : s > 25 ? "▂▄" : "▂"; }

    // ---- the network you're on
    Rectangle {
        id: currentCard
        visible: page.on && page.current !== null
        Layout.fillWidth: true
        implicitHeight: curCol.implicitHeight + 2 * SetStyle.pad
        radius: SetStyle.radius
        color: SetStyle.card
        border.width: 1; border.color: SetStyle.tint(Theme.text, 0.05)
        ColumnLayout {
            id: curCol
            anchors { left: parent.left; right: parent.right; top: parent.top; margins: SetStyle.pad }
            spacing: 12
            RowLayout {
                Layout.fillWidth: true
                spacing: 16
                Rectangle {
                    implicitWidth: 56; implicitHeight: 56; radius: Math.max(6, Theme.r(10))
                    color: SetStyle.tint(Theme.accent, 0.18)
                    Icon { anchors.centerIn: parent; name: "wifi"; size: 28; color: Theme.accent }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Text { Layout.fillWidth: true; text: page.current ? page.current.ssid : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 20; font.weight: Font.DemiBold; elide: Text.ElideRight }
                    Text {
                        Layout.fillWidth: true; wrapMode: Text.WordWrap
                        text: !page.current ? "" : ["Connected" + (page.current.secure ? ", secured" : ", open"), page.signalWord(page.current.signal) + " " + page.bars(page.current.signal),
                                                    page.band(page.current.freq), page.ip].filter(x => x).join(" · ")
                        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                    }
                }
                Flow {
                    Layout.alignment: Qt.AlignVCenter
                    Layout.maximumWidth: 420
                    spacing: 8
                    SetButton { icon: "info"; text: page.showProps ? "Hide properties" : "Properties"; onActivated: { page.showProps = !page.showProps; if (page.showProps) details.running = true; } }
                    SetButton { text: "Disconnect"; onActivated: if (page.current) page.disconnect(page.current) }
                    SetButton { visible: !!page.current && page.saved.includes(page.current.ssid); icon: "trash"; text: "Forget"; ghost: true; onActivated: page.forgetAsk = true }
                }
            }
            // Forget, asked once more
            RowLayout {
                visible: page.forgetAsk
                Layout.fillWidth: true
                spacing: 8
                Text { Layout.fillWidth: true; wrapMode: Text.WordWrap; text: page.current ? "Forget " + page.current.ssid + "? This device won't join it by itself any more, and you'll need its password again." : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: 13 }
                SetButton { text: "Forget"; primary: true; onActivated: if (page.current) page.forget(page.current) }
                SetButton { text: "Cancel"; ghost: true; onActivated: page.forgetAsk = false }
            }
            // Properties (only read)
            GridLayout {
                visible: page.showProps
                Layout.fillWidth: true
                columns: 2; columnSpacing: 24; rowSpacing: 6
                Repeater {
                    model: !page.current ? [] : [
                        ["Security", page.current.security || (page.current.secure ? "Secured" : "Open")],
                        ["Band", page.band(page.current.freq) + (page.current.freq ? " (" + page.current.freq + " MHz)" : "")],
                        ["Signal", page.current.signal + "%"],
                        ["IPv4 address", page.props["IP4.ADDRESS"] || page.ip],
                        ["Gateway", page.props["IP4.GATEWAY"] || ""],
                        ["DNS", page.props["IP4.DNS"] || ""],
                        ["IPv6 address", page.props["IP6.ADDRESS"] || ""],
                        ["Physical address (MAC)", page.props["GENERAL.HWADDR"] || ""]
                    ].filter(r => r[1] && r[1].trim() !== "")
                    delegate: Item {
                        required property var modelData
                        Layout.columnSpan: 2
                        Layout.fillWidth: true
                        implicitHeight: pr.implicitHeight
                        RowLayout {
                            id: pr
                            anchors { left: parent.left; right: parent.right }
                            Text { Layout.preferredWidth: 200; text: modelData[0]; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
                            Text { Layout.fillWidth: true; text: modelData[1]; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                        }
                    }
                }
            }
        }
    }

    SetSwitch {
        icon: "wifi"; title: "Wi-Fi"; sub: page.on ? (page.current ? "Connected to " + page.current.ssid : "Not connected") : "Off"
        checked: page.on
        onToggled: v => { page.on = v; page.run(["nmcli", "radio", "wifi", v ? "on" : "off"], v ? "Wi-Fi on" : "Wi-Fi off"); }
    }
    Text { visible: page.message !== ""; text: page.message; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; Layout.leftMargin: 4 }

    SetCardGroup {
        visible: page.on
        title: page.scanning ? "Networks nearby · looking…" : "Networks nearby"
        Repeater {
            model: page.on ? page.shownNets : []
            delegate: ColumnLayout {
                id: netItem
                required property var modelData
                Layout.fillWidth: true
                spacing: 8
                SetRow {
                    icon: "wifi"
                    title: netItem.modelData.ssid
                    chip: netItem.modelData.inUse ? "Connected" : ""
                    sub: netItem.modelData.inUse ? "Connected" + (netItem.modelData.secure ? ", secured" : "") : page.busy === netItem.modelData.ssid ? "Connecting…" : page.saved.includes(netItem.modelData.ssid) ? "Saved" : netItem.modelData.secure ? "Secured" : "Open: anyone can see what you send"
                    value: page.bars(netItem.modelData.signal) + (netItem.modelData.secure ? "  🔒" : "")
                    onActivated: {
                        const n = netItem.modelData;
                        if (n.inUse) page.disconnect(n);
                        else if (n.secure && !page.saved.includes(n.ssid)) { page.asking = n.ssid; Qt.callLater(() => pw.focusInput()); }
                        else page.connect(n, "");
                    }
                }
                RowLayout {
                    visible: page.asking === netItem.modelData.ssid
                    Layout.fillWidth: true
                    Layout.leftMargin: 52
                    spacing: 8
                    SetField { id: pw; placeholder: "Password for " + netItem.modelData.ssid; secret: true; onSubmitted: t => page.connect(netItem.modelData, t) }
                    SetRow { Layout.preferredWidth: 140; Layout.fillWidth: false; title: "Connect"; chevron: false; onActivated: page.connect(netItem.modelData, pw.text) }
                }
            }
        }
        SetRow {
            visible: page.on && (page.showAll ? page.otherNets.length > Math.max(0, 6 - page.firstNets.length) : page.nets.length > page.shownNets.length)
            icon: "wifi"; chevron: false
            title: page.showAll ? "Show fewer networks" : "Show all networks (" + (page.nets.length - page.shownNets.length) + " more)"
            onActivated: page.showAll = !page.showAll
        }
        // ---- a network that doesn't say its name
        SetRow {
            icon: "plus"; title: "Add a hidden network"; sub: "A network that doesn't show its name: type it and its password"
            chevron: true; chevronIcon: page.hiddenOpen ? "chevu" : "chevd"
            onActivated: { page.hiddenOpen = !page.hiddenOpen; if (page.hiddenOpen) Qt.callLater(() => hiddenName.focusInput()); }
        }
        ColumnLayout {
            visible: page.hiddenOpen
            Layout.fillWidth: true
            Layout.leftMargin: 52
            spacing: 6
            SetField { id: hiddenName; placeholder: "Network name (SSID)"; onSubmitted: t => hiddenPw.focusInput() }
            SetField { id: hiddenPw; placeholder: "Password (leave empty for an open network)"; secret: true; onSubmitted: t => { page.connectHidden(hiddenName.text, t); hiddenName.text = ""; hiddenPw.text = ""; } }
            SetRow { Layout.preferredWidth: 160; Layout.fillWidth: false; title: "Connect"; chevron: false
                onActivated: { page.connectHidden(hiddenName.text, hiddenPw.text); hiddenName.text = ""; hiddenPw.text = ""; } }
        }
        SetRow { visible: page.on; icon: "restart"; title: "Look again"; chevron: false; onActivated: page.refresh() }
    }
    Text { visible: !page.on; text: "Turn on Wi-Fi to see networks."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
}
