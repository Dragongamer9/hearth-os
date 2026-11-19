// A small line icon (24 × 24 drawing, scaled to `size`): the mockups' emoji drawn as lines, so they look the same
// with any font. Names: pad, headset, play, pause, next, download, link, bell, speaker, wifi, moon, keyboard, sun,
// camera, bolt, tv, monitor, lock, power, gear, search, mouse, bluetooth, couch, record, hash, chevron, check, refresh.
import QtQuick
import QtQuick.Shapes

Item {
    id: g
    property string name: "pad"
    property real size: 18
    property color color: "#f2ede8"
    property bool filled: false
    width: size; height: size
    readonly property var paths: ({
        pad: "M7 8h10a4 4 0 0 1 4 4v1.5a3 3 0 0 1-5.2 2L14.5 14h-5l-1.3 1.5A3 3 0 0 1 3 13.5V12a4 4 0 0 1 4-4zM7.5 10.5v3M6 12h3M15.5 11.2h.01M17.5 13h.01",
        headset: "M4 15v-3a8 8 0 0 1 16 0v3M4 15a2 2 0 0 0 2 2h1v-5H6a2 2 0 0 0-2 2zM20 15a2 2 0 0 1-2 2h-1v-5h1a2 2 0 0 1 2 2z",
        play: "M8 5l11 7-11 7z",
        pause: "M8 5v14M16 5v14",
        next: "M6 6l8 6-8 6zM17 6v12",
        download: "M12 4v11M7 10l5 5 5-5M5 20h14",
        link: "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
        bell: "M6 16V11a6 6 0 0 1 12 0v5l2 2H4zM10 20a2 2 0 0 0 4 0",
        speaker: "M4 9h4l5-4v14l-5-4H4zM16 9a4 4 0 0 1 0 6M18.5 6.5a7.5 7.5 0 0 1 0 11",
        wifi: "M2.5 9a14 14 0 0 1 19 0M5.5 12.5a9.5 9.5 0 0 1 13 0M8.5 16a5 5 0 0 1 7 0M12 19.5h.01",
        moon: "M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z",
        keyboard: "M3 7h18v10H3zM6.5 10h.01M9.5 10h.01M12.5 10h.01M15.5 10h.01M17.5 10h.01M7.5 14h9",
        sun: "M12 8a4 4 0 1 1 0 8 4 4 0 0 1 0-8zM12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4",
        camera: "M4 8h3l2-2h6l2 2h3v11H4zM12 10a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7z",
        bolt: "M13 2L5 13h6l-1 9 8-11h-6z",
        tv: "M3 6h18v11H3zM8 21h8M12 17v4",
        monitor: "M4 4h16v12H4zM9 20h6M12 16v4",
        lock: "M6 11h12v9H6zM8.5 11V8a3.5 3.5 0 0 1 7 0v3",
        power: "M12 3v8M7 6a7.5 7.5 0 1 0 10 0",
        gear: "M12 9a3 3 0 1 1 0 6 3 3 0 0 1 0-6zM12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1",
        search: "M10.5 4a6.5 6.5 0 1 1 0 13 6.5 6.5 0 0 1 0-13zM15.5 15.5L20 20",
        mouse: "M12 3a6 6 0 0 1 6 6v6a6 6 0 0 1-12 0V9a6 6 0 0 1 6-6zM12 7v3",
        bluetooth: "M7 7l10 10-5 4V3l5 4L7 17",
        couch: "M4 11V8a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v3M3 11h3v4h12v-4h3v6H3zM5 17v2M19 17v2",
        record: "M12 6a6 6 0 1 1 0 12 6 6 0 0 1 0-12z",
        hash: "M5 9h14M5 15h14M10 4L8 20M16 4l-2 16",
        chevron: "M9 5l7 7-7 7",
        check: "M5 12.5l4.5 4.5L19 7",
        refresh: "M19 12a7 7 0 1 1-2-4.9M19 4v4h-4"
    })
    Shape {
        anchors.centerIn: parent
        width: 24; height: 24
        scale: g.size / 24
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: g.color
            strokeWidth: 2
            fillColor: g.filled ? g.color : "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: g.paths[g.name] || g.paths.pad }
        }
    }
}
