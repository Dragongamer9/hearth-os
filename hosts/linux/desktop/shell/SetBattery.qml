// Hearth Settings › Power › Battery health & history (1.14, KDE Info Center's energy page, in Hearth's look): the
// battery's health (how much it holds now against new), its charge cycles, and two graphs from UPower's history: the
// charge and the power used, over the last hours, day or week. Plus the batteries of your mice, keyboards,
// controllers and headphones (UPower, Bluetooth ones too). Reads only.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.UPower

SetPage {
    id: page
    title: "Battery health & history"
    property var st: null
    property var charge: []
    property var rate: []
    property int hours: 24
    readonly property var bat: st && st.batteries && st.batteries.length ? st.batteries[0] : null
    function load() {
        job.run(["status"], r => { page.st = r; page.loadHistory(); }, () => page.st = { batteries: [] });
    }
    function loadHistory() {
        if (!bat) return;
        job.run(["history", bat.path, "charge", String(hours)], r => page.charge = r.points || [], () => page.charge = []);
        job.run(["history", bat.path, "rate", String(hours)], r => page.rate = r.points || [], () => page.rate = []);
    }
    onHoursChanged: loadHistory()
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-powerset" }
    readonly property var others: (UPower.devices ? UPower.devices.values : []).filter(d => d && !d.isLaptopBattery && d.isPresent !== false && d.percentage > 0 && d.type !== UPowerDeviceType.LinePower && d.type !== UPowerDeviceType.Ups && d.type !== UPowerDeviceType.Battery)

    // The health as a ring, then the numbers.
    SetPanel {
        visible: !!page.bat
        Layout.fillWidth: true
        implicitHeight: 150
        RowLayout {
            anchors { fill: parent; leftMargin: 26; rightMargin: 26 }
            spacing: 26
            Item {
                implicitWidth: 104; implicitHeight: 104
                Canvas {
                    id: ring
                    anchors.fill: parent
                    readonly property real h: page.bat && page.bat.health ? Math.min(100, page.bat.health) / 100 : 0
                    onHChanged: requestPaint()
                    onPaint: {
                        const c = getContext("2d"); c.reset();
                        c.lineWidth = 10; c.lineCap = "round";
                        c.strokeStyle = Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.1);
                        c.beginPath(); c.arc(52, 52, 44, 0, Math.PI * 2); c.stroke();
                        c.strokeStyle = h >= 0.8 ? Theme.accent : h >= 0.6 ? SetStyle.warn : Theme.danger;
                        c.beginPath(); c.arc(52, 52, 44, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * h); c.stroke();
                    }
                }
                Column {
                    anchors.centerIn: parent
                    Text { anchors.horizontalCenter: parent.horizontalCenter; text: page.bat && page.bat.health ? page.bat.health + "%" : "—"; color: Theme.text; font.family: Theme.font; font.pixelSize: 24; font.weight: Font.DemiBold }
                    Text { anchors.horizontalCenter: parent.horizontalCenter; text: "health"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                }
            }
            GridLayout {
                Layout.fillWidth: true
                columns: 3; rowSpacing: 10; columnSpacing: 30
                Repeater {
                    model: page.bat ? [
                        ["Charge cycles", page.bat.cycles !== null ? String(page.bat.cycles) : "Not reported"],
                        ["Holds now", page.bat.energyFull ? page.bat.energyFull.toFixed(1) + " Wh" : "—"],
                        ["Held when new", page.bat.energyDesign ? page.bat.energyDesign.toFixed(1) + " Wh" : "—"],
                        ["Charge", page.bat.percentage !== null ? Math.round(page.bat.percentage) + "% · " + page.bat.state : "—"],
                        ["Using now", page.bat.energyRate ? page.bat.energyRate.toFixed(1) + " W" : "—"],
                        ["Battery", [page.bat.vendor, page.bat.model, page.bat.technology].filter(x => x).join(" · ") || page.bat.name]
                    ] : []
                    delegate: ColumnLayout {
                        required property var modelData
                        spacing: 1
                        Text { text: modelData[0]; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                        Text { text: modelData[1]; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.Medium }
                    }
                }
            }
        }
    }
    SetRow {
        visible: !!page.st && !page.bat
        interactive: false; icon: "info"; title: "This PC has no battery"; sub: "Batteries of your devices are below"
    }
    SetChoice {
        visible: !!page.bat
        icon: "clock"; title: "Show the last"
        options: [{ value: 6, label: "6 hours" }, { value: 24, label: "Day" }, { value: 168, label: "Week" }]
        current: page.hours
        onChosen: v => page.hours = v
    }
    // A graph: line and a soft fill, time along the bottom.
    component Graph: SetPanel {
        id: g
        property string head
        property string unit
        property var points: []
        property real topValue: 100
        Layout.fillWidth: true
        implicitHeight: 220
        Text { x: 18; y: 14; text: g.head; color: Theme.text; font.family: Theme.font; font.pixelSize: 15; font.weight: Font.DemiBold }
        Text {
            x: 18; y: 14; width: parent.width - 36; horizontalAlignment: Text.AlignRight
            text: g.points.length ? "" : "No history yet: UPower keeps it while the PC runs"
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12
        }
        Canvas {
            id: cv
            x: 18; y: 44; width: parent.width - 36; height: parent.height - 66
            onWidthChanged: requestPaint()
            Connections { target: g; function onPointsChanged() { cv.requestPaint(); } }
            onPaint: {
                const c = getContext("2d"); c.reset();
                c.strokeStyle = Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.08); c.lineWidth = 1;
                for (let i = 0; i <= 4; i++) { const y = Math.round(height * i / 4) + 0.5; c.beginPath(); c.moveTo(0, y); c.lineTo(width, y); c.stroke(); }
                const p = g.points;
                if (!p.length) return;
                const t0 = p[0][0], t1 = Math.max(t0 + 1, p[p.length - 1][0]);
                const top = Math.max(g.topValue, ...p.map(q => q[1])) || 1;
                const X = t => (t - t0) / (t1 - t0) * width, Y = v => height - Math.max(0, v) / top * height;
                c.beginPath(); c.moveTo(X(p[0][0]), height);
                for (const q of p) c.lineTo(X(q[0]), Y(q[1]));
                c.lineTo(X(p[p.length - 1][0]), height); c.closePath();
                c.fillStyle = Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.16); c.fill();
                c.beginPath();
                p.forEach((q, i) => i ? c.lineTo(X(q[0]), Y(q[1])) : c.moveTo(X(q[0]), Y(q[1])));
                c.strokeStyle = Theme.accent; c.lineWidth = 2; c.stroke();
                c.fillStyle = Theme.muted; c.font = "11px sans-serif";
                c.fillText(Math.round(top) + " " + g.unit, 4, 12);
            }
        }
        Text { x: 18; y: parent.height - 20; text: page.hours >= 168 ? "A week ago" : page.hours >= 24 ? "A day ago" : page.hours + " hours ago"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
        Text { x: parent.width - 18 - width; y: parent.height - 20; text: "Now"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
    }
    Graph { visible: !!page.bat; head: "Charge"; unit: "%"; topValue: 100; points: page.charge }
    Graph { visible: !!page.bat; head: "Power used"; unit: "W"; topValue: 10; points: page.rate }

    SetCardGroup {
        title: "Your devices' batteries"
        SetRow {
            visible: page.others.length === 0
            interactive: false; icon: "bt"; title: "No other batteries"; sub: "Wireless mice, keyboards, controllers and headphones show their charge here"
        }
        Repeater {
            model: page.others
            delegate: SetRow {
                required property var modelData
                interactive: false; chevron: false
                icon: modelData.type === UPowerDeviceType.Mouse ? "mouse" : modelData.type === UPowerDeviceType.Keyboard ? "kbd"
                    : modelData.type === UPowerDeviceType.GamingInput ? "pad" : modelData.type === UPowerDeviceType.Headset || modelData.type === UPowerDeviceType.Headphones ? "headphones" : "bt"
                title: modelData.model || modelData.nativePath || "Device"
                value: Math.round(modelData.percentage * 100) + "%"
                chip: modelData.percentage < 0.15 ? "Low" : ""; chipKind: "warn"
            }
        }
    }
}
