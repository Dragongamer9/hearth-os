// Clock widget: digital or analog, with your cities under it (each city's time and whether it's already tomorrow
// there). Settings: the look, seconds, and the cities (type a city or a time zone; hearth-widgets zones gives the
// offsets, looked at again every half hour so summer time is right). In the bar: the time and your first city.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import "widget-layout.js" as L

WidgetBase {
    id: w
    readonly property string look: c("style", "digital")
    readonly property bool secs: c("seconds", false) === true
    readonly property var cities: list("cities") || []
    property var offsets: ({})             // tz → minutes
    property var allZones: []
    property double now: Date.now()
    implicitWidth: compact ? barRow.implicitWidth : 300
    implicitHeight: compact ? 24 : 220

    Timer { interval: w.secs || w.look === "analog" ? 1000 : 10000; running: w.live; repeat: true; triggeredOnStart: true; onTriggered: w.now = Date.now() }
    Process {
        id: zp
        command: ["python3", w.helper, "zones"].concat(w.cities)
        running: w.cities.length > 0
        stdout: StdioCollector { onStreamFinished: { try { const o = JSON.parse(text.trim().split("\n").pop()); const m = {}; for (const z of o.zones || []) if (z.offset !== null) m[z.tz] = z.offset; w.offsets = m; } catch (e) {} } }
    }
    onCitiesChanged: zp.running = true
    Timer { interval: 1800000; running: w.live && w.cities.length > 0; repeat: true; onTriggered: zp.running = true }
    Process {
        id: allp
        command: ["python3", w.helper, "all-zones"]
        stdout: StdioCollector { onStreamFinished: { try { w.allZones = JSON.parse(text.trim().split("\n").pop()).zones || []; } catch (e) {} } }
    }
    onConfiguringChanged: if (configuring && !allZones.length) allp.running = true

    readonly property date here: new Date(now)
    readonly property string hereText: Qt.formatTime(here, Settings.clock24 ? (secs ? "HH:mm:ss" : "HH:mm") : (secs ? "h:mm:ss AP" : "h:mm AP"))
    function cityName(tz) { return String(tz).split("/").pop().replace(/_/g, " "); }
    function cityLine(tz) {
        const off = offsets[tz];
        if (off === undefined) return "…";
        const t = L.cityTime(now, off);
        return L.fmtTime(t, Settings.clock24);
    }
    function cityDay(tz) {
        const off = offsets[tz];
        return off === undefined ? "" : L.dayWord(L.cityTime(now, off).day, here.getDay());
    }

    // ---------------------------------------------------------------- the bar
    RowLayout {
        id: barRow
        visible: w.compact
        anchors.verticalCenter: parent.verticalCenter
        spacing: 6
        Icon { name: "globe"; size: 15; color: Theme.muted }
        Text {
            text: w.cities.length ? w.cityName(w.cities[0]) + " " + w.cityLine(w.cities[0]) : w.hereText
            color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold
        }
    }

    // ---------------------------------------------------------------- on the desktop
    ColumnLayout {
        visible: !w.compact && !w.configuring
        anchors { fill: parent; margins: 14 * w.k }
        spacing: 6 * w.k
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 60
            Text {
                visible: w.look === "digital"
                anchors.fill: parent
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                text: w.hereText
                color: Theme.text
                font.family: Theme.font
                font.pixelSize: 160
                fontSizeMode: Text.Fit
                minimumPixelSize: 14
                font.weight: Font.Bold
            }
            Canvas {
                id: face
                visible: w.look === "analog"
                anchors.centerIn: parent
                width: Math.min(parent.width, parent.height); height: width
                property double t: w.now
                onTChanged: requestPaint()
                onPaint: {
                    const g = getContext("2d");
                    g.reset();
                    const r = width / 2 - 3, cx = width / 2, cy = height / 2, d = new Date(t);
                    g.beginPath(); g.arc(cx, cy, r, 0, Math.PI * 2);
                    g.fillStyle = Qt.rgba(Theme.bg.r, Theme.bg.g, Theme.bg.b, 0.5); g.fill();
                    g.lineWidth = 2; g.strokeStyle = Theme.line; g.stroke();
                    for (let i = 0; i < 12; i++) {
                        const a = i * Math.PI / 6, l = i % 3 === 0 ? 0.16 : 0.08;
                        g.beginPath(); g.moveTo(cx + Math.sin(a) * r * (0.92 - l), cy - Math.cos(a) * r * (0.92 - l));
                        g.lineTo(cx + Math.sin(a) * r * 0.92, cy - Math.cos(a) * r * 0.92);
                        g.lineWidth = i % 3 === 0 ? 3 : 1.5; g.strokeStyle = Theme.text; g.stroke();
                    }
                    function hand(a, len, wd, col) {
                        g.beginPath(); g.moveTo(cx, cy); g.lineTo(cx + Math.sin(a) * r * len, cy - Math.cos(a) * r * len);
                        g.lineWidth = wd; g.lineCap = "round"; g.strokeStyle = col; g.stroke();
                    }
                    const h = d.getHours() % 12, m = d.getMinutes(), s = d.getSeconds();
                    hand((h + m / 60) * Math.PI / 6, 0.5, 5, Theme.text);
                    hand((m + s / 60) * Math.PI / 30, 0.75, 3.5, Theme.text);
                    if (w.secs) hand(s * Math.PI / 30, 0.82, 1.5, Theme.accent);
                    g.beginPath(); g.arc(cx, cy, 4, 0, Math.PI * 2); g.fillStyle = Theme.accent; g.fill();
                }
            }
        }
        Text {
            Layout.alignment: Qt.AlignHCenter
            text: Qt.formatDate(w.here, "dddd, d MMMM")
            color: Theme.muted; font.family: Theme.font; font.pixelSize: Math.round(14 * w.k)
        }
        Repeater {
            model: w.cities.slice(0, Math.max(1, Math.floor((w.height - 150) / (26 * w.k)) + 1))
            delegate: RowLayout {
                required property var modelData
                Layout.fillWidth: true
                spacing: 8
                Text { Layout.fillWidth: true; text: w.cityName(modelData); color: Theme.text; font.family: Theme.font; font.pixelSize: Math.round(14 * w.k); elide: Text.ElideRight }
                Text { text: w.cityDay(modelData); color: Theme.muted; font.family: Theme.font; font.pixelSize: Math.round(12 * w.k) }
                Text { text: w.cityLine(modelData); color: Theme.text; font.family: Theme.font; font.pixelSize: Math.round(15 * w.k); font.weight: Font.DemiBold }
            }
        }
    }

    // ---------------------------------------------------------------- settings
    ColumnLayout {
        visible: !w.compact && w.configuring
        anchors { fill: parent; margins: 14 }
        spacing: 8
        Text { text: "Clock"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
        RowLayout {
            spacing: 6
            WBtn { text: "Digital"; on: w.look === "digital"; onClicked: w.set("style", "digital") }
            WBtn { text: "Analog"; on: w.look === "analog"; onClicked: w.set("style", "analog") }
            WBtn { text: "Seconds"; on: w.secs; onClicked: w.set("seconds", !w.secs) }
        }
        Text { text: "Cities"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: w.cities
                delegate: WBtn { required property var modelData; text: w.cityName(modelData) + "  ×"; onClicked: w.set("cities", w.cities.filter(x => x !== modelData)) }
            }
        }
        WField {
            id: cityField
            Layout.fillWidth: true
            placeholder: "Add a city (Tokyo, New York…)"
            onAccepted: if (matches.length) { w.set("cities", w.cities.concat([matches[0]])); text = ""; }
            readonly property var matches: {
                const q = text.trim().toLowerCase().replace(/ /g, "_");
                if (q.length < 2) return [];
                return w.allZones.filter(z => z.toLowerCase().indexOf(q) >= 0 && w.cities.indexOf(z) < 0).slice(0, 4);
            }
        }
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: cityField.matches
                delegate: WBtn { required property var modelData; text: modelData.replace(/_/g, " "); flat: true; onClicked: { w.set("cities", w.cities.concat([modelData])); cityField.text = ""; } }
            }
        }
        Item { Layout.fillHeight: true }
    }
}
