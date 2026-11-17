// Hearth Settings › Power › Apps keeping it awake (1.14, KDE's "applications blocking sleep", in Hearth's look): the
// apps and services holding the screen on or the PC awake right now (logind's inhibitors through hearth-powerset),
// what each says it's doing, and Sleep anyway. Reads only; refreshed every few seconds while open.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Apps keeping it awake"
    property var list: null
    function load() { job.run(["status"], r => page.replaceList("list", r.inhibitors || []), () => page.list = []); }
    Component.onCompleted: load()
    Timer { interval: 6000; repeat: true; running: page.visible; onTriggered: page.load() }
    SetJob { id: job; helper: "hearth-powerset" }
    function words(i) {
        const w = [];
        if (i.what.indexOf("idle") >= 0) w.push("keeps the screen on");
        if (i.what.indexOf("sleep") >= 0) w.push(i.mode === "delay" ? "delays sleep a moment" : "keeps the PC awake");
        if (i.what.indexOf("shutdown") >= 0) w.push(i.mode === "delay" ? "delays shutting down" : "holds off shutting down");
        return w.join(", ");
    }
    readonly property var blocking: (list || []).filter(i => i.mode === "block")
    readonly property var delaying: (list || []).filter(i => i.mode !== "block")

    SetCardGroup {
        title: page.blocking.length ? "Keeping it awake now" : "Nothing is keeping it awake"
        SetRow {
            visible: !!page.list && page.blocking.length === 0
            interactive: false; icon: "check"; title: "The PC can sleep and the screen can turn off on time"
            sub: "Videos, games and downloads that need it awake show up here while they do"
        }
        Repeater {
            model: page.blocking
            delegate: SetRow {
                required property var modelData
                interactive: false; chevron: false
                icon: modelData.what.indexOf("idle") >= 0 ? "display" : "sleep"
                title: modelData.who + (modelData.app && modelData.app !== modelData.who ? " (" + modelData.app + ")" : "")
                sub: (modelData.why ? modelData.why + " · " : "") + page.words(modelData)
                chip: "Blocking"; chipKind: "warn"
            }
        }
    }
    SetCardGroup {
        title: "Asking for a moment first"
        visible: page.delaying.length > 0
        Repeater {
            model: page.delaying
            delegate: SetRow {
                required property var modelData
                interactive: false; chevron: false; icon: "clock"
                title: modelData.who; sub: (modelData.why ? modelData.why + " · " : "") + page.words(modelData)
            }
        }
    }
    SetCardGroup {
        title: "Sleep anyway"
        SetRow {
            icon: "sleep"; title: "Sleep now"; sub: "Puts the PC to sleep even while apps ask to keep it awake"; chevron: false
            onActivated: Buttons.sleep("Settings › Power › Apps keeping it awake")
        }
    }
}
