// Hearth Settings › Printers › Scan to PDF (1.14, KDE's multi-page scanning, in Hearth's look): pick a scanner, its
// resolution, colour and source, then Scan a page as many times as you like (or Scan every page in the feeder), put
// the pages in order, take one out, and Save as one PDF in Documents. hearth-scan does it; the pages wait in memory
// ($XDG_RUNTIME_DIR) until you save or clear them. Opening the page only reads (the scanners and the pages so far).
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Scan to PDF"
    property var scanners: null
    property string device: ""
    property var opts: null
    property int dpi: 150
    property string mode: ""
    property string source: ""
    property var pages: []
    property int stamp: 0
    property string saved: ""
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    SetJob { id: find; helper: "hearth-printq" }
    SetJob { id: job; helper: "hearth-scan" }
    function got(r) { pages = r.pages || []; stamp++; }
    function pick(d) {
        device = d; opts = null; source = ""; mode = "";
        job.run(["options", d], r => {
            page.opts = r;
            page.mode = r.modes.indexOf("Color") >= 0 ? "Color" : (r.modes[0] || "");
            page.source = r.sources.find(s => /flatbed/i.test(s)) || (r.sources[0] || "");
            if (r.resolutions.indexOf(page.dpi) < 0) page.dpi = r.resolutions.indexOf(300) >= 0 ? 300 : r.resolutions[0];
        }, e => page.say(e, true));
    }
    Component.onCompleted: {
        job.run(["pages"], r => page.got(r));
        find.run(["scanners"], r => { page.scanners = r.scanners || []; if (page.scanners.length) page.pick(page.scanners[0].device); },
                 e => { page.scanners = []; page.say(e, true); });
    }
    readonly property bool feederPicked: /adf|feeder|duplex/i.test(source)

    SetCardGroup {
        title: page.scanners === null ? "Looking for scanners…" : page.scanners.length ? "Scanner" : "No scanners found"
        SetRow {
            visible: !!page.scanners && page.scanners.length === 0
            interactive: false; icon: "info"; title: "Nothing answered"; sub: "Turn the scanner on and plug it in, or put it on the same network"
        }
        SetDropdown {
            visible: !!page.scanners && page.scanners.length > 0
            icon: "printer"; title: "Scanner"; boxWidth: 360
            options: (page.scanners || []).map(s => ({ value: s.device, label: s.name }))
            current: page.device
            onChosen: v => page.pick(v)
        }
        SetDropdown {
            visible: !!page.opts
            icon: "image"; title: "Resolution"; sub: "150 for reading, 300 for printing again"
            options: page.opts ? page.opts.resolutions.map(r => ({ value: r, label: r + " dpi" })) : []
            current: page.dpi
            onChosen: v => page.dpi = v
        }
        SetDropdown {
            visible: !!page.opts && page.opts.modes.length > 1
            icon: "palette"; title: "Colour"
            options: page.opts ? page.opts.modes.map(m => ({ value: m, label: m === "Color" ? "Colour" : m === "Gray" ? "Greyscale" : m === "Lineart" ? "Black and white" : m })) : []
            current: page.mode
            onChosen: v => page.mode = v
        }
        SetDropdown {
            visible: !!page.opts && page.opts.sources.length > 1
            icon: "doc"; title: "From"
            options: page.opts ? page.opts.sources.map(s => ({ value: s, label: s })) : []
            current: page.source
            onChosen: v => page.source = v
        }
    }
    SetCardGroup {
        title: page.pages.length ? page.pages.length + (page.pages.length === 1 ? " page" : " pages") + " so far" : "Pages"
        SetRow {
            visible: !!page.opts
            icon: "plus"; chevron: false
            title: job.what === "scan" ? "Scanning…" : page.feederPicked ? "Scan every page in the feeder" : page.pages.length ? "Scan the next page" : "Scan a page"
            sub: page.feederPicked ? "All the paper in the feeder, one page after another" : "Put the page on the glass, then press. Repeat for each page"
            onActivated: if (!job.busy) { page.say(""); page.saved = "";
                job.run([page.feederPicked ? "feeder" : "page", page.device, String(page.dpi), page.mode, page.source], r => page.got(r), e => page.say(e, true), "", "scan"); }
        }
        Flow {
            visible: page.pages.length > 0
            Layout.fillWidth: true
            spacing: 12
            Repeater {
                model: page.pages
                delegate: Rectangle {
                    id: th
                    required property var modelData
                    required property int index
                    width: 150; height: 230; radius: 10; color: SetStyle.card2
                    Image {
                        x: 10; y: 10; width: 130; height: 172
                        source: "file://" + th.modelData.path + "?" + page.stamp
                        fillMode: Image.PreserveAspectFit; asynchronous: true; cache: false; sourceSize.width: 260
                    }
                    Text { x: 12; y: 192; text: "Page " + (th.index + 1); color: Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                    Row {
                        anchors { right: parent.right; rightMargin: 8; bottom: parent.bottom; bottomMargin: 8 }
                        spacing: 4
                        SetButton { visible: th.index > 0; icon: "chevu"; text: ""; ghost: true; onActivated: job.run(["up", String(th.index)], r => page.got(r)) }
                        SetButton { icon: "trash"; text: ""; ghost: true; onActivated: job.run(["remove", String(th.index)], r => page.got(r)) }
                    }
                }
            }
        }
        SetField {
            id: nameField
            visible: page.pages.length > 0
            placeholder: "Name, like Tax return 2026 (or the date)"
        }
        SetRow {
            visible: page.pages.length > 0
            icon: "file"; chevron: false; title: "Save as PDF"; sub: "One PDF of every page, in Documents"
            onActivated: if (!job.busy) job.run(["save", nameField.text], r => { page.saved = r.file; page.say("Saved " + r.file.replace(/^.*\//, "") + " (" + r.pages + (r.pages === 1 ? " page)." : " pages).")); }, e => page.say(e, true))
        }
        SetRow {
            visible: page.saved !== ""
            icon: "open"; chevron: false; title: "Open the PDF"; sub: page.saved.replace(Quickshell.env("HOME"), "~")
            onActivated: Quickshell.execDetached(["xdg-open", page.saved])
        }
        SetRow {
            visible: page.pages.length > 0
            icon: "close"; danger: true; chevron: false; title: "Start again"; sub: "Clears the pages here (a saved PDF stays)"
            onActivated: job.run(["clear"], r => { page.got(r); page.saved = ""; })
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
