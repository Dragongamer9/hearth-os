// Hearth Settings › About › Info Center (1.14, KDE Info Center's technical pages, in Hearth's look): one page per part
// (props part: cpu, gpu, memory, pci, usb, graphics, sensors, modules, interrupts; Energy is Battery health & history), with the parts as tabs along
// the top. Read by hearth-info; nothing is changed. Copy puts the page's text on the clipboard for a bug report.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Info Center"
    property var info: null
    readonly property var tabs: [{ p: "cpu", id: "info-cpu", t: "Processor" }, { p: "gpu", id: "info-gpu", t: "Graphics card" }, { p: "memory", id: "info-memory", t: "Memory" },
        { p: "graphics", id: "info-graphics", t: "Vulkan & OpenGL" }, { p: "pci", id: "info-pci", t: "PCI" }, { p: "usb", id: "info-usb", t: "USB" }, { p: "energy", id: "info-energy", t: "Energy" },
        { p: "sensors", id: "info-sensors", t: "Temperatures & fans" }, { p: "modules", id: "info-modules", t: "Kernel modules" }, { p: "interrupts", id: "info-interrupts", t: "Interrupts" }]
    readonly property string which: part || "cpu"
    property string find: ""
    function load() { page.info = null; job.run([which], r => page.info = r, e => page.info = { sections: [{ title: "Couldn't read it", rows: [["Problem", e]] }] }); }
    property bool ready: false
    Component.onCompleted: { ready = true; load(); }
    onWhichChanged: if (ready) reload.restart()
    Timer { id: reload; interval: 1; onTriggered: page.load() }
    SetJob { id: job; helper: "hearth-info" }
    // Temperatures and fans (and interrupts' counts) move: read again every 2 s while the page is open, without
    // clearing it first.
    Timer { interval: 2000; repeat: true; running: page.visible && (page.which === "sensors" || page.which === "interrupts"); onTriggered: if (!job.busy) job.run([page.which], r => page.info = r) }
    function asText() {
        if (!info) return "";
        if (info.items) return info.items.map(i => [i.id, i.kind, i.title, i.sub, i.value].filter(x => x).join(" · ")).join("\n");
        return info.sections.map(s => s.title + "\n" + (s.rows || []).map(r => "  " + r[0] + ": " + r[1]).join("\n")).join("\n\n");
    }

    Flow {
        Layout.fillWidth: true
        spacing: 6
        Repeater {
            model: page.tabs
            delegate: Rectangle {
                required property var modelData
                readonly property bool lit: modelData.p === page.which
                implicitWidth: tl.implicitWidth + 28; implicitHeight: 34; radius: 17
                color: lit ? Theme.accent : tm.containsMouse ? SetStyle.cardHover : SetStyle.card
                Text { id: tl; anchors.centerIn: parent; text: modelData.t; color: lit ? Theme.accentInk : Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: lit ? Font.DemiBold : Font.Normal }
                MouseArea { id: tm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: if (!lit) SettingsNav.openPage(modelData.id) }
            }
        }
    }
    SetRow { visible: !page.info; interactive: false; icon: "info"; title: "Reading…" }

    // Sections of name / value rows (cpu, gpu, memory, graphics).
    Repeater {
        model: page.info && page.info.sections ? page.info.sections : []
        delegate: SetPanel {
            id: sec
            required property var modelData
            implicitHeight: secCol.implicitHeight + 32
            ColumnLayout {
                id: secCol
                x: 20; y: 16; width: parent.width - 40
                spacing: 6
                Text { text: sec.modelData.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
                // Memory: how full, as a bar.
                Rectangle {
                    visible: !!sec.modelData.meter
                    Layout.fillWidth: true; implicitHeight: 10; radius: 5; color: Theme.panel3
                    Rectangle { height: parent.height; radius: 5; color: Theme.accent; width: sec.modelData.meter ? parent.width * Math.min(1, sec.modelData.meter[0] / Math.max(1, sec.modelData.meter[1])) : 0 }
                }
                GridLayout {
                    Layout.fillWidth: true
                    columns: 2; columnSpacing: 24; rowSpacing: 5
                    Repeater {
                        model: (sec.modelData.rows || []).length * 2
                        delegate: Text {
                            required property int index
                            readonly property var row: sec.modelData.rows[Math.floor(index / 2)]
                            Layout.fillWidth: index % 2 === 1
                            Layout.preferredWidth: index % 2 === 0 ? 190 : -1
                            text: row ? row[index % 2] : ""
                            color: index % 2 === 0 ? Theme.muted : Theme.text
                            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
                        }
                    }
                }
                // Temperatures as bars, warm colours near their limit (sensors).
                Repeater {
                    model: sec.modelData.gauges || []
                    delegate: RowLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: 10
                        Text { Layout.preferredWidth: 190; text: modelData.label; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                        Rectangle {
                            Layout.fillWidth: true; implicitHeight: 10; radius: 5; color: Theme.panel3
                            Rectangle {
                                height: parent.height; radius: 5
                                width: parent.width * Math.max(0.02, Math.min(1, modelData.value / Math.max(1, modelData.top || 100)))
                                color: modelData.hot ? Theme.danger : modelData.value / Math.max(1, modelData.top || 100) > 0.75 ? SetStyle.warn : Theme.accent
                                Behavior on width { NumberAnimation { duration: 400 } }
                            }
                        }
                        Text { Layout.preferredWidth: 70; horizontalAlignment: Text.AlignRight; text: Math.round(modelData.value) + " °C"; color: modelData.hot ? Theme.danger : Theme.text; font.family: Theme.font; font.pixelSize: 13; font.weight: Font.DemiBold }
                        Text { Layout.preferredWidth: 150; text: modelData.text.indexOf("(") >= 0 ? modelData.text.replace(/^.*\(/, "").replace(/\)$/, "") : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; elide: Text.ElideRight }
                    }
                }
                // Each CPU thread's speed now, as little bars.
                Row {
                    visible: !!sec.modelData.bars
                    spacing: 3
                    Repeater {
                        model: sec.modelData.bars || []
                        delegate: Rectangle {
                            required property var modelData
                            width: 9; height: 46; radius: 2; color: Theme.panel3
                            Rectangle { anchors.bottom: parent.bottom; width: parent.width; radius: 2; color: Theme.accent; height: parent.height * Math.min(1, modelData / Math.max(1, sec.modelData.top || 1)) }
                        }
                    }
                }
            }
        }
    }
    // Lists (pci, usb).
    SetField { visible: !!page.info && !!page.info.items; placeholder: page.which === "modules" ? "Find a module" : page.which === "interrupts" ? "Find an interrupt" : "Find a device"; onTextChanged: page.find = text.toLowerCase() }
    SetCardGroup {
        visible: !!page.info && !!page.info.items
        title: page.info && page.info.items ? page.info.items.length + (page.which === "modules" ? " modules loaded" : page.which === "interrupts" ? " interrupts" : " devices") + (page.info.note ? " · " + page.info.note : "") : ""
        Repeater {
            model: page.info && page.info.items ? page.info.items.filter(i => page.find === "" || (i.title + " " + i.sub + " " + i.kind + " " + i.value).toLowerCase().indexOf(page.find) >= 0).slice(0, 80) : []
            delegate: SetRow {
                required property var modelData
                interactive: false; chevron: false
                icon: page.which === "usb" ? "link" : page.which === "modules" ? "box" : page.which === "interrupts" ? "bolt" : /VGA|3D|Display/.test(modelData.kind) ? "display" : /Audio|audio/.test(modelData.kind) ? "vol" : /Network|Ethernet/.test(modelData.kind) ? "wired" : /USB/.test(modelData.kind) ? "link" : /SATA|NVM|storage|Non-Volatile/.test(modelData.kind) ? "drive" : "cpu"
                title: modelData.title || modelData.id
                sub: [modelData.kind, modelData.sub, modelData.id].filter(x => x).join(" · ")
                value: modelData.value
            }
        }
    }
    SetRow {
        visible: !!page.info
        icon: "copy"; chevron: false; title: "Copy this page"; sub: "As text, for a bug report or a forum post"
        onActivated: { Quickshell.execDetached(["sh", "-c", 'printf "%s" "$1" | wl-copy', "sh", page.asText()]); SettingsNav.toast("Copied"); }
    }
}
