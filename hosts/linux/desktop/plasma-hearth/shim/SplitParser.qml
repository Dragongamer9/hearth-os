// Quickshell's SplitParser (plasma-hearth): the program's output, a line (or piece) at a time.
import QtQuick
QtObject {
    property string splitMarker: "\n"
    signal read(string data)
}
