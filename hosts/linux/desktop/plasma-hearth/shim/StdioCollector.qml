// Quickshell's StdioCollector (plasma-hearth): the program's output, all of it, when it ends.
import QtQuick
QtObject {
    property string text: ""
    signal streamFinished()
}
