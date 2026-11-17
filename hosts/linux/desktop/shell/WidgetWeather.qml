// Weather widget: now (temperature, feels like, wind) and the next days, from Open-Meteo (free, no account or key)
// through hearth-widgets. The place is picked by name in its settings (type it, pick from the list); °C or °F.
// Looked at every 15 minutes while it's on screen. In the bar: the temperature and the place.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import "widget-layout.js" as L

WidgetBase {
    id: w
    readonly property var place: c("place", null)
    readonly property bool imperial: c("units", "metric") === "imperial"
    property var wx: null
    property var results: []
    property string searchErr: ""
    property int pick: 0
    implicitWidth: compact ? barRow.implicitWidth : 330
    implicitHeight: compact ? 24 : 230

    Process {
        id: get
        command: w.place ? ["python3", w.helper, "weather", String(w.place.lat), String(w.place.lon)].concat(w.imperial ? ["imperial"] : []) : ["true"]
        running: w.live && !!w.place
        stdout: StdioCollector { onStreamFinished: { try { const o = JSON.parse(text.trim().split("\n").pop()); if (!o.error) w.wx = o; } catch (e) {} } }
    }
    onPlaceChanged: { wx = null; if (place) get.running = true; }
    onImperialChanged: if (place) get.running = true
    Timer { interval: 900000; running: w.live && !!w.place; repeat: true; onTriggered: get.running = true }
    Process {
        id: search
        stdout: StdioCollector {
            onStreamFinished: {
                try { const o = JSON.parse(text.trim().split("\n").pop()); w.results = o.places || []; w.searchErr = o.error || (w.results.length ? "" : "No place called that"); }
                catch (e) { w.results = []; w.searchErr = "Couldn't look that up"; }
                w.pick = 0;
            }
        }
    }
    function find(q) {
        if (!q.trim()) return;
        searchErr = "Looking…";
        search.command = ["python3", helper, "places", q.trim()];
        search.running = true;
    }
    function choose(p) {
        set("place", { label: p.label, name: p.name, lat: p.lat, lon: p.lon });
        results = [];
        doneConfiguring();
    }
    function deg(v) { return v === null || v === undefined ? "–" : Math.round(v) + "°"; }

    // ---------------------------------------------------------------- the bar
    RowLayout {
        id: barRow
        visible: w.compact
        anchors.verticalCenter: parent.verticalCenter
        spacing: 4
        WeatherIcon { Layout.preferredWidth: 22; Layout.preferredHeight: 22; kind: w.wx ? L.weatherKind(w.wx.code, w.wx.day) : "cloud" }
        Text { text: w.wx ? w.deg(w.wx.temp) + "  " + (w.place ? w.place.name : "") : w.place ? w.place.name : "Weather"; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
    }

    // ---------------------------------------------------------------- on the desktop
    ColumnLayout {
        visible: !w.compact && !w.configuring && !!w.place
        anchors { fill: parent; margins: 14 * w.k }
        spacing: 4
        RowLayout {
            Layout.fillWidth: true
            spacing: 10
            WeatherIcon { Layout.preferredWidth: 72 * w.k; Layout.preferredHeight: 72 * w.k; kind: w.wx ? L.weatherKind(w.wx.code, w.wx.day) : "cloud" }
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0
                Text { text: w.wx ? w.deg(w.wx.temp) : "…"; color: Theme.text; font.family: Theme.font; font.pixelSize: Math.round(44 * w.k); font.weight: Font.Bold }
                Text { Layout.fillWidth: true; text: w.wx ? L.weatherWord(w.wx.code) : "Looking outside…"; color: Theme.text; font.family: Theme.font; font.pixelSize: Math.round(15 * w.k); elide: Text.ElideRight }
            }
            ColumnLayout {
                spacing: 2
                Text { Layout.alignment: Qt.AlignRight; text: w.place ? w.place.name : ""; color: Theme.text; font.family: Theme.font; font.pixelSize: Math.round(14 * w.k); font.weight: Font.DemiBold }
                Text { Layout.alignment: Qt.AlignRight; visible: !!w.wx; text: w.wx ? "Feels " + w.deg(w.wx.feels) : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: Math.round(12 * w.k) }
                Text { Layout.alignment: Qt.AlignRight; visible: !!w.wx; text: w.wx ? "Wind " + Math.round(w.wx.wind || 0) + " " + w.wx.windUnit : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: Math.round(12 * w.k) }
            }
        }
        Item { Layout.fillHeight: true }
        RowLayout {
            Layout.fillWidth: true
            visible: !!w.wx && w.height > 170
            spacing: 4
            Repeater {
                model: w.wx ? w.wx.days.slice(0, Math.max(3, Math.min(6, Math.floor(w.width / 60)))) : []
                delegate: ColumnLayout {
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    spacing: 1
                    Text { Layout.alignment: Qt.AlignHCenter; text: index === 0 ? "Today" : modelData.day; color: Theme.muted; font.family: Theme.font; font.pixelSize: Math.round(12 * w.k) }
                    WeatherIcon { Layout.alignment: Qt.AlignHCenter; Layout.preferredWidth: 30 * w.k; Layout.preferredHeight: 30 * w.k; kind: L.weatherKind(modelData.code, true) }
                    Text { Layout.alignment: Qt.AlignHCenter; text: w.deg(modelData.max) + " " + w.deg(modelData.min); color: Theme.text; font.family: Theme.font; font.pixelSize: Math.round(12 * w.k) }
                }
            }
        }
    }
    ColumnLayout {
        visible: !w.compact && !w.configuring && !w.place
        anchors.centerIn: parent
        spacing: 10
        WeatherIcon { Layout.alignment: Qt.AlignHCenter; Layout.preferredWidth: 60; Layout.preferredHeight: 60; kind: "sun-cloud" }
        WBtn { Layout.alignment: Qt.AlignHCenter; text: "Pick a place"; on: true; onClicked: w.wantsSettings() }
    }

    // ---------------------------------------------------------------- settings
    ColumnLayout {
        visible: !w.compact && w.configuring
        anchors { fill: parent; margins: 14 }
        spacing: 6
        Text { text: "Weather"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
        WField {
            id: q
            Layout.fillWidth: true
            placeholder: w.place ? w.place.label : "Type a town or city, then Enter"
            grab: w.configuring
            onDownPressed: w.pick = Math.min(w.results.length - 1, w.pick + 1)
            onUpPressed: w.pick = Math.max(0, w.pick - 1)
            // Enter looks the name up; Enter again (the same name) takes the place picked in the list.
            onAccepted: { if (w.results.length && text.trim() === lastQ) w.choose(w.results[w.pick]); else { lastQ = text.trim(); w.find(text); } }
            property string lastQ: ""
        }
        Repeater {
            model: w.results.slice(0, 5)
            delegate: Rectangle {
                required property var modelData
                required property int index
                Layout.fillWidth: true
                height: 28
                radius: Theme.r(6)
                color: index === w.pick || pm.containsMouse ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.25) : "transparent"
                Text { anchors { fill: parent; leftMargin: 8 } verticalAlignment: Text.AlignVCenter; text: modelData.label; color: Theme.text; font.family: Theme.font; font.pixelSize: 13; elide: Text.ElideRight }
                MouseArea { id: pm; anchors.fill: parent; hoverEnabled: true; onClicked: w.choose(modelData) }
            }
        }
        Text { visible: w.searchErr !== "" && !w.results.length; text: w.searchErr; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        Item { Layout.fillHeight: true }
        RowLayout {
            spacing: 6
            WBtn { text: "°C"; on: !w.imperial; onClicked: w.set("units", "metric") }
            WBtn { text: "°F"; on: w.imperial; onClicked: w.set("units", "imperial") }
        }
    }
}
