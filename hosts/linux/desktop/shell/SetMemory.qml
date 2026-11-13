// Hearth Settings › System › Memory (1.11): the same Swap & settings as Task Manager › Performance › Memory
// (MemorySettings.qml, hearth-memory). Opening it writes nothing; Save sends only what you changed.
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Memory"
    MemorySettings { Layout.fillWidth: true }
}
