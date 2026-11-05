// Hearth Settings › Home server › Family & home (docs/HOME-SERVER.md): a Family folder everyone can use, the people who
// can open your shares, and security cameras (hidden until a camera is found on your home network, the same
// hide-until-seen rule as the UPS; Overview › Show hidden items shows them anyway). No third-party cloud: everything
// stays on this PC and your own devices.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Family & home"
    property var f: null
    property bool looking: false
    property string msg: ""
    property bool bad: false
    readonly property bool camShown: !!f && !!f.seen && !!f.seen.camera && f.seen.camera.shown
    function say(t, b) { msg = t; bad = !!b; }
    function refresh() { job.run(["family"], r => page.f = r, e => page.f = { error: e }); }
    Component.onCompleted: refresh()
    SetJob { id: job; helper: "hearth-home-server" }
    SetJob { id: act; helper: "hearth-home-server" }

    Text {
        Layout.fillWidth: true
        text: "Folders for the whole family, who can open what, and cameras. It all stays on this PC and your own devices: no outside cloud."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }

    SetHeader { text: "Shared family folder" }
    SetRow {
        visible: !!page.f && !page.f.family
        icon: "folder"; title: "Make a Family folder"
        sub: "Home › Family, read & write for you and every person you added (People): holiday photos, school things, shared notes and lists"
        onActivated: act.run(["folder-share", "family"], () => { page.say("Family folder shared ✓ Everyone in People can open it"); page.refresh(); }, e => page.say(e, true))
    }
    SetRow {
        visible: !!page.f && !!page.f.family
        icon: "folder"; title: "Family folder"
        sub: page.f && page.f.family ? page.f.family.path + " · " + (page.f.family.write ? "read & write" : "read only") + " · " + (Object.keys(page.f.family.people || {}).length ? "you, " + Object.keys(page.f.family.people).join(", ") : "only you so far") : ""
        onActivated: SettingsNav.openPage("hs-shares")
    }
    SetRow {
        icon: "user"; title: "People"
        sub: "Family and friends with their own password and only the folders you pick"
        value: page.f && page.f.people.length ? page.f.people.length + (page.f.people.length === 1 ? " person" : " people") : ""
        onActivated: SettingsNav.openPage("hs-people")
    }
    SetRow {
        icon: "calendar"; chevron: false; interactive: false; title: "Calendar, notes and lists"
        sub: "Keep them as files in the Family folder (any notes app on a phone can open a shared folder). A shared calendar server isn't part of Hearth yet"
    }

    SetHeader { text: "Security cameras" }
    SetRow {
        icon: "cam"; title: page.looking ? "Looking for cameras…" : "Look for cameras"
        sub: "Asks your home network for cameras (ONVIF). Finding one shows the camera items; it turns nothing on"
             + (page.f && page.f.camerasAt ? " · last looked " + new Date(page.f.camerasAt).toLocaleString(Qt.locale(), "d MMM h:mm AP") : "")
        onActivated: if (!page.looking) { page.looking = true; act.run(["cameras-scan"], r => { page.looking = false; page.say(r.cameras.length ? r.cameras.length + (r.cameras.length === 1 ? " camera found" : " cameras found") : "No cameras answered"); page.refresh(); }, e => { page.looking = false; page.say(e, true); }); }
    }
    Repeater {
        model: page.camShown && page.f ? page.f.cameras : []
        delegate: SetRow {
            required property var modelData
            icon: "cam"; title: modelData.name; sub: modelData.ip + " · opens the camera's own page"
            onActivated: Quickshell.execDetached(["xdg-open", modelData.url])
        }
    }
    SetRow {
        visible: page.camShown
        icon: "info"; chevron: false; interactive: false; title: "Recording to this PC"
        sub: "Hearth doesn't record cameras yet. Your camera's own app or page can save clips to a shared folder (most cameras can save to an SMB share: use a person's name and password from People)"
    }
    Text {
        visible: !!page.f && !page.camShown
        Layout.fillWidth: true
        text: "Camera items show once a camera is found (or with Show hidden items on the Overview page)."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Text {
        Layout.fillWidth: true; Layout.topMargin: 6
        text: "Not here yet: parental controls (play time) and ad blocking for the whole home."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
    }
    Text {
        visible: page.msg !== ""
        Layout.fillWidth: true
        text: page.msg; color: page.bad ? Theme.danger : Theme.flame; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
