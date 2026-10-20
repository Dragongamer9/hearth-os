// Hearth Settings › Printers (CUPS): your printers, adding one CUPS finds on the network or over USB (driverless
// IPP Everywhere first, then a driver for its make and model), or by its network address; the default printer,
// a test page, and removing one. Adding, removing and the default go through hearth-admin.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Printers"
    property var st: null              // hearth-system printers
    property var found: null           // printers CUPS found (null = not looked yet)
    property string picked: ""
    property string confirm: ""
    property string message: ""
    property bool messageBad: false
    readonly property var printers: st ? st.printers : []
    readonly property var newFound: (found || []).filter(d => !printers.some(p => p.uri === d.uri))

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["printers"], r => page.st = r); }
    Component.onCompleted: load()
    Timer { interval: 15000; repeat: true; running: page.visible && !admin.busy; onTriggered: page.load() }
    SetJob { id: job; helper: "hearth-system" }
    SetJob { id: admin; helper: "/usr/libexec/nimbus/hearth-admin"; admin: true }
    function runAdmin(args, label, okText, after) {
        say("");
        admin.run(args, r => { page.say(okText); if (after) after(r); page.load(); }, e => { page.say(e, true); page.load(); }, "", label);
    }
    function stateText(p) {
        const s = p.state === "printing" ? "Printing" : p.state === "disabled" ? "Paused" : "Ready";
        return s + (p.reason && p.state !== "idle" ? " · " + p.reason : "") + (p.location ? " · " + p.location : "");
    }
    function kindOf(uri) {
        return /^usb:/.test(uri) ? "USB" : /^(dnssd|ipp|ipps|http|https|socket|lpd):/.test(uri) ? "Network" : "";
    }
    // A CUPS printer name from what the printer calls itself (letters, numbers, - _ .), not already used.
    function nameFor(d) {
        let n = (d.info || d.makeModel || "Printer").replace(/[^A-Za-z0-9_.-]+/g, "_").replace(/^[^A-Za-z0-9]+|_+$/g, "").slice(0, 60) || "Printer";
        let k = n, i = 2;
        while (printers.some(p => p.name.toLowerCase() === k.toLowerCase())) k = n + "_" + i++;
        return k;
    }
    function add(d) {
        const name = nameFor(d);
        const info = (d.info || d.makeModel || name).replace(/[\x00-\x1f\x7f]/g, "").slice(0, 120);
        runAdmin(["printer-add", name, d.uri, "--info", info], "add", info + " is ready to use.", () => { page.found = null; });
    }

    SetRow {
        visible: !!page.st && !page.st.cups
        interactive: false; chevron: false
        icon: "info"; title: "The printing service isn't running"
        sub: "Hearth uses CUPS for printing. It starts by itself when something prints; if this stays, restart the device."
    }

    SetHeader { text: "My printers"; visible: page.printers.length > 0 }
    Repeater {
        model: page.printers
        delegate: ColumnLayout {
            id: pr
            required property var modelData
            Layout.fillWidth: true
            spacing: 8
            SetRow {
                icon: "printer"
                title: pr.modelData.info || pr.modelData.name
                sub: page.stateText(pr.modelData) + (page.kindOf(pr.modelData.uri) ? " · " + page.kindOf(pr.modelData.uri) : "")
                value: pr.modelData.default ? "Default" : ""
                onActivated: { page.picked = page.picked === pr.modelData.name ? "" : pr.modelData.name; page.confirm = ""; }
            }
            ColumnLayout {
                visible: page.picked === pr.modelData.name
                Layout.fillWidth: true
                Layout.leftMargin: 52
                spacing: 8
                SetRow {
                    icon: "printer"; chevron: false; title: job.what === "test" ? "Sending…" : "Print a test page"
                    onActivated: job.run(["printer-test", pr.modelData.name], () => page.say("Test page sent to " + (pr.modelData.info || pr.modelData.name) + "."), e => page.say(e, true), "", "test")
                }
                SetRow {
                    visible: !pr.modelData.default && !!page.st && page.st.canAdmin
                    icon: "check"; chevron: false; title: "Make it the default"
                    onActivated: page.runAdmin(["printer-default", pr.modelData.name], "default", (pr.modelData.info || pr.modelData.name) + " is now the default printer.")
                }
                SetRow {
                    visible: page.confirm !== pr.modelData.name && !!page.st && page.st.canAdmin
                    icon: "trash"; danger: true; chevron: false; title: "Remove this printer…"
                    onActivated: page.confirm = pr.modelData.name
                }
                SetRow {
                    visible: page.confirm === pr.modelData.name
                    icon: "trash"; danger: true; chevron: false; title: admin.what === "remove" ? "Removing…" : "Yes, remove " + (pr.modelData.info || pr.modelData.name)
                    onActivated: page.runAdmin(["printer-remove", pr.modelData.name], "remove", "Printer removed.", () => { page.picked = ""; page.confirm = ""; })
                }
                SetRow { visible: page.confirm === pr.modelData.name; title: "Cancel"; chevron: false; onActivated: page.confirm = "" }
            }
        }
    }
    Text {
        visible: !!page.st && page.st.cups && page.printers.length === 0
        text: "No printers yet."; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }

    SetHeader { text: "Add a printer"; visible: !!page.st && page.st.canAdmin && page.st.cups }
    SetRow {
        visible: !!page.st && page.st.canAdmin && page.st.cups
        icon: "search"; chevron: false
        title: admin.what === "discover" ? "Looking for printers…" : page.found === null ? "Look for printers" : "Look again"
        sub: "On your network and plugged in by USB. Turn the printer on first."
        onActivated: { page.say(""); admin.run(["printer-discover"], r => page.found = r.devices || [], e => page.say(e, true), "", "discover"); }
    }
    Repeater {
        model: page.newFound
        delegate: SetRow {
            required property var modelData
            icon: "printer"; chevron: false
            title: modelData.info || modelData.makeModel || modelData.uri
            sub: (page.kindOf(modelData.uri) || "Printer") + (modelData.makeModel && modelData.makeModel !== modelData.info ? " · " + modelData.makeModel : "") + (modelData.location ? " · " + modelData.location : "")
            value: admin.what === "add" ? "" : "Add"
            onActivated: page.add(modelData)
        }
    }
    Text {
        visible: page.found !== null && page.newFound.length === 0 && admin.what !== "discover"
        Layout.fillWidth: true
        text: "No new printers found. Check it's on and on the same network, or type its address below."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetField { id: addr; visible: !!page.st && page.st.canAdmin && page.st.cups; placeholder: "Or its network address, like 192.168.1.20"; onSubmitted: addrBtn.activated() }
    SetRow {
        id: addrBtn
        visible: !!page.st && page.st.canAdmin && page.st.cups
        icon: "printer"; chevron: false; title: admin.what === "add" ? "Adding…" : "Add by address"
        sub: "For network printers that work without a driver (most printers from the last ten years)"
        onActivated: {
            const h = addr.text.trim();
            if (!/^[A-Za-z0-9]([A-Za-z0-9.-]{0,250}[A-Za-z0-9])?$|^\[[0-9A-Fa-f:]{2,45}\]$/.test(h)) return page.say("Type the printer's address, like 192.168.1.20 or printer.local.", true);
            page.add({ uri: "ipp://" + h + "/ipp/print", info: "Printer at " + h, makeModel: "" });
            addr.text = "";
        }
    }
    SetRow {
        visible: !!page.st && page.st.configTool
        icon: "gear"; title: "More printer settings"; sub: "Drivers, paper and other options (Print Settings)"
        onActivated: Quickshell.execDetached(["system-config-printer"])
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    Text {
        visible: !!page.st && !page.st.canAdmin
        Layout.fillWidth: true
        text: "Printers are added here on Hearth, the OS."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
