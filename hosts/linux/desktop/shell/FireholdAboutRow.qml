// Settings › About's "Firehold (preview)" (a hidden preview): there only once it's unlocked (firehold.json's
// "firehold.unlocked"; read only here). Opens Firehold's Couch and Desk settings. Loaded by file name from SetAbout.qml.
// TODO(merge with feature/hearth-compositor): the unlock (tap the version 7 times) and its row come from there; keep
// one row, and read its flag if it's stored elsewhere.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetRow {
    id: fr
    property bool unlocked: false
    visible: unlocked
    chevron: true
    icon: "pad"; title: "Firehold (preview)"
    sub: "The 2.0 desktop: Couch for the controller and the TV, Desk for the mouse and keyboard"
    onActivated: Quickshell.execDetached(["quickshell", "ipc", "-p", Quickshell.shellDir, "call", "firehold", "open", "settings"])
    FileView {
        path: Quickshell.env("HOME") + "/.config/nimbus/firehold.json"
        watchChanges: true
        printErrors: false
        onFileChanged: reload()
        onLoaded: { let o = null; try { o = JSON.parse(text()); } catch (e) {} fr.unlocked = !!o && o["firehold.unlocked"] === true; }
        onLoadFailed: fr.unlocked = false
    }
}
