// Hearth Settings › Printers › Scanners (1.14, KDE's scanner check, in Hearth's look): the scanners SANE finds (USB and,
// through sane-airscan, network ones), and Test scan: a quick low-resolution scan shown here, to see that one really
// works. Scanning documents is Hearth Tools' Printers & Scanner. Reads only (hearth-printq); the test picture is kept
// in memory ($XDG_RUNTIME_DIR) only.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Scanners"
    property var list: null
    property string testing: ""
    property string picture: ""
    property int stamp: 0
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { list = null; job.run(["scanners"], r => page.list = r.scanners || [], e => { page.list = []; page.say(e, true); }); }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-printq" }

    SetCardGroup {
        title: page.list === null ? "Looking for scanners…" : page.list.length ? "Scanners found" : "No scanners found"
        SetRow {
            visible: !!page.list && page.list.length === 0
            interactive: false; icon: "info"; title: "Nothing answered"
            sub: "Turn the scanner on and plug it in, or put it on the same network. Many printers with a scanner show up here"
        }
        Repeater {
            model: page.list || []
            delegate: SetRow {
                required property var modelData
                icon: "printer"; chevron: false
                title: modelData.name
                sub: (modelData.device.startsWith("airscan:") || modelData.device.startsWith("escl:") ? "On the network · " : /usb/i.test(modelData.device) ? "USB · " : "")
                    + (page.testing === modelData.device ? "Scanning a test… (up to a minute)" : "Test scan")
                chip: page.picture !== "" && page.testing === "" && page.lastOk === modelData.device ? "Works" : ""
                onActivated: if (!job.busy) {
                    page.testing = modelData.device; page.say("");
                    job.run(["scan-test", modelData.device], r => { page.picture = r.file; page.stamp++; page.lastOk = page.testing; page.testing = ""; page.say("It scanned. This is the test picture."); },
                        e => { page.testing = ""; page.say(e, true); }, "", "test");
                }
            }
        }
        SetRow { icon: "sync"; chevron: false; title: "Look again"; onActivated: if (!job.busy) page.load() }
        SetRow { icon: "file"; chevron: true; title: "Scan to PDF"; sub: "Several pages into one PDF"; onActivated: SettingsNav.openPage("scan-pdf") }
    }
    property string lastOk: ""
    SetPanel {
        visible: page.picture !== ""
        implicitHeight: 340
        Image {
            anchors { fill: parent; margins: 14 }
            source: page.picture ? "file://" + page.picture + "?" + page.stamp : ""
            fillMode: Image.PreserveAspectFit; cache: false; asynchronous: true
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
