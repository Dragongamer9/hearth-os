// A live picture of one window (Everything open), the way Plasma's task manager makes its window previews: KWin's
// screencast of that window through PipeWire. Loaded only while Everything open is on screen; if it can't load, the
// card keeps its colour.
import QtQuick
import org.kde.pipewire as PipeWire
import org.kde.taskmanager as TaskManager

PipeWire.PipeWireSourceItem {
    id: pic
    property string uuid: ""
    nodeId: cast.nodeId
    visible: cast.nodeId > 0
    TaskManager.ScreencastingRequest {
        id: cast
        uuid: pic.uuid
    }
}
