// Hearth Settings › AI › Describe it (the owner, Nov 24, 2026: Describe it sat on "Your AI is making it…" for minutes on
// his laptop's slow CPU Ollama while his PC's AI was there over Hearth Link). The same choice as the Rust Settings'
// Settings › AI (hosts/hearth-ui/apps/settings/src/cats/ai.rs describe_card), for Mantel's own Settings: where Describe
// it asks first, Fastest available (a slow engine on this device after your other devices) / This device / My other
// devices (hearth-ai-check use describe {"from": …}, ai-uses.json through hearth_settings_io), and which AI that is now.
// Opening the page writes nothing; the pick is saved only when you make it.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Describe it"
    property var st: null                 // hearth-ai-check status
    property string msg: ""
    readonly property var use: st && st.uses ? st.uses.describe : null

    SetJob { id: job; helper: "hearth-ai-check" }
    function refresh() { job.run(["status"], r => page.st = r, e => page.msg = e); }
    Component.onCompleted: refresh()

    SetHeader { text: "Describe it uses" }
    SetChoice {
        icon: "sparkle"; title: "Where Describe it asks first"
        sub: !page.use ? "Checking…"
            : page.use.ready ? "Now: " + (page.use.via || page.use.label) + (page.use.model ? " · " + page.use.model : "")
                + (page.use.slow ? " · slow on this device" + (page.use.words ? " (about " + Math.round(page.use.words) + " words a second)" : "") : "")
            : "Nothing answers there right now: the fastest that answers is asked instead"
        options: [{ value: "fastest", label: "Fastest available" }, { value: "here", label: "This device" }, { value: "link", label: "My other devices" }]
        current: page.use && page.use.from ? page.use.from : "fastest"
        // hearth-ai-check says {"ok": false, "error"} when it can't: shown, never taken as done
        onChosen: v => job.run(["use", "describe", JSON.stringify({ from: v })], r => { page.msg = r && r.ok === false ? (r.error || "That didn't work.") : ""; page.refresh(); }, e => page.msg = e)
    }
    Text {
        Layout.fillWidth: true; Layout.leftMargin: 4; wrapMode: Text.WordWrap
        text: "Fastest available: your other devices over Hearth Link before a slow engine on this device (one that writes under 8 words a second, or has no graphics card). Nothing here changes by itself."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true; wrapMode: Text.WordWrap
        text: page.msg; color: Theme.danger; font.family: Theme.font; font.pixelSize: 14
    }
}
