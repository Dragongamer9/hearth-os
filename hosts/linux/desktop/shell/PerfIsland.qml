// The Gaming look's performance island (Islands.qml): FPS (only while MangoHud logs it; hearth-stats leaves it out
// otherwise and so does this), CPU and GPU load with their temperatures, memory, and a small CPU graph.
import QtQuick
import QtQuick.Layouts

RowLayout {
    id: perf
    spacing: 14
    Stats { id: stats; running: perf.visible }
    Stat { visible: stats.fps >= 0; label: "FPS"; value: String(stats.fps) }
    Stat { visible: stats.cpu >= 0; label: "CPU"; value: stats.cpu + "%"; sub: stats.cpuTemp >= 0 ? stats.cpuTemp + "°" : "" }
    Stat { visible: stats.gpu >= 0; label: "GPU"; value: stats.gpu + "%"; sub: stats.gpuTemp >= 0 ? stats.gpuTemp + "°" : "" }
    Stat { visible: stats.ram >= 0; label: "RAM"; value: stats.ram.toFixed(1) + "G" }
    Canvas {
        id: graph
        implicitWidth: 72; implicitHeight: 24
        visible: stats.history.length > 1
        property var data: stats.history
        onDataChanged: requestPaint()
        onPaint: {
            const c = getContext("2d"), d = data, w = width, h = height;
            c.reset();
            if (d.length < 2) return;
            c.strokeStyle = Theme.css(Theme.accent, 1); c.lineWidth = 1.6;
            c.beginPath();
            for (let i = 0; i < d.length; i++) {
                const x = i * w / (d.length - 1), y = h - 2 - (h - 4) * d[i] / 100;
                if (i === 0) c.moveTo(x, y); else c.lineTo(x, y);
            }
            c.stroke();
        }
    }
    Text { visible: !graph.visible && stats.cpu < 0; text: "Reading…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }

    component Stat: ColumnLayout {
        id: st
        property string label
        property string value
        property string sub: ""
        spacing: -2
        Text { text: st.label; color: Theme.muted; font.family: Theme.font; font.pixelSize: 9; font.bold: true; font.letterSpacing: 1.5 }
        Row {
            spacing: 3
            Text { text: st.value; color: Theme.text; font.family: "DejaVu Sans Mono"; font.pixelSize: 15; font.bold: true }
            Text { visible: st.sub !== ""; anchors.bottom: parent.bottom; text: st.sub; color: Theme.flame; font.family: Theme.font; font.pixelSize: 10 }
        }
    }
}
