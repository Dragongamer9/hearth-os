// Quickshell's SystemClock (plasma-hearth): the time, updated once a minute (or second), on the minute.
import QtQuick

Item {
    id: c
    enum Precision { Hours, Minutes, Seconds }
    property int precision: SystemClock.Minutes
    property var date: new Date()
    readonly property int hours: date.getHours()
    readonly property int minutes: date.getMinutes()
    readonly property int seconds: date.getSeconds()
    Timer {
        id: t
        running: c.enabled
        repeat: false
        interval: c.precision === SystemClock.Seconds ? 1000 - (new Date()).getMilliseconds() : 60000 - (new Date()).getSeconds() * 1000 - (new Date()).getMilliseconds() + 50
        onTriggered: { c.date = new Date(); interval = c.precision === SystemClock.Seconds ? 1000 : 60000 - c.date.getSeconds() * 1000 - c.date.getMilliseconds() + 50; restart(); }
    }
}
