// Hearth Settings › Date & time: automatic time (timedatectl set-ntp), the time zone (search, or region then
// city), setting the clock by hand, and a 12- or 24-hour clock (Settings.clock24, which the taskbar follows).
// Changes that need root go through hearth-admin.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "Date & time"
    property var st: null              // hearth-system time
    property var zones: []             // [{zone, region, city, offset}]
    property string region: ""         // the region whose cities are listed ("" = the list of regions)
    property string message: ""
    property bool messageBad: false
    property bool zoneChanged: false   // the taskbar clock catches up after the desktop restarts
    readonly property var regions: {
        const seen = {}, out = [];
        for (const z of zones) { if (!seen[z.region]) { seen[z.region] = 0; out.push(z.region); } seen[z.region]++; }
        return out.map(r => ({ name: r, count: seen[r] }));
    }
    readonly property string query: search.text.trim().toLowerCase()
    readonly property var matches: query === "" ? [] : zones.filter(z => (z.city + " " + z.region + " " + z.zone + " utc" + z.offset).toLowerCase().includes(query)).slice(0, 40)
    readonly property var cities: region === "" ? [] : zones.filter(z => z.region === region)

    function say(t, bad) { message = t; messageBad = !!bad; }
    // "2026-10-19T19:40:27-07:00" -> the time and date as they are in the chosen zone (not this process's).
    function nowText() {
        if (!st || !st.now) return "";
        const m = st.now.match(/^(\d{4})-(\d\d)-(\d\d)T(\d\d):(\d\d)/);
        if (!m) return "";
        const h = parseInt(m[4]), min = m[5];
        const t = Settings.clock24 ? (m[4] + ":" + min) : (((h + 11) % 12 + 1) + ":" + min + (h < 12 ? " AM" : " PM"));
        const d = new Date(parseInt(m[1]), parseInt(m[2]) - 1, parseInt(m[3]));
        return t + " · " + Qt.formatDate(d, "dddd, MMMM d, yyyy");
    }
    function load() { job.run(["time"], r => page.st = r); }
    Component.onCompleted: { load(); job.run(["timezones"], r => page.zones = r.zones || []); }
    Timer { interval: 20000; repeat: true; running: page.visible; onTriggered: page.load() }

    SetJob { id: job; helper: "hearth-system" }
    SetJob { id: admin; helper: "/usr/libexec/nimbus/hearth-admin"; admin: true }
    function runAdmin(args, label, okText, after) {
        say("");
        admin.run(args, () => { page.say(okText); if (after) after(); page.load(); }, e => { page.say(e, true); page.load(); }, "", label);
    }
    function setZone(z) {
        if (st && z.zone === st.timezone) return;
        runAdmin(["timezone", z.zone], "zone", "Time zone set to " + z.city + ".", () => { page.zoneChanged = true; search.text = ""; page.region = ""; });
    }

    SetRow {
        interactive: false; chevron: false
        icon: "clock"; title: page.nowText() || "…"
        sub: page.st ? (page.st.timezone.replace(/_/g, " ") + (page.st.ntp ? (page.st.synced ? " · set from the internet" : " · waiting for the internet") : " · set by hand")) : "Checking…"
    }
    SetChoice {
        title: "Clock"; sub: "How the taskbar shows the time"
        options: [{ value: false, label: "12-hour (2:05 PM)" }, { value: true, label: "24-hour (14:05)" }]
        current: Settings.clock24
        onChosen: v => Settings.clock24 = v
    }
    SetSwitch {
        visible: !!page.st && page.st.available
        icon: "restart"; title: "Set the time automatically"
        sub: admin.what === "ntp" ? "Saving…" : !page.st || page.st.canNtp ? "Uses the internet to keep the clock right" : "This system has no time service; set the clock by hand"
        checked: !!page.st && page.st.ntp
        onToggled: v => page.runAdmin(["ntp", v ? "on" : "off"], "ntp", v ? "The clock will be set from the internet." : "Automatic time is off.")
    }

    // ------------------------------------------------ by hand (automatic time off)
    SetHeader { text: "Set the clock"; visible: !!page.st && page.st.available && !page.st.ntp }
    RowLayout {
        visible: !!page.st && page.st.available && !page.st.ntp
        Layout.fillWidth: true
        spacing: 8
        SetField { id: dateField; placeholder: "Date, like 2026-10-20"; onSubmitted: timeField.focusInput() }
        SetField { id: timeField; placeholder: "Time, like 14:05"; onSubmitted: setBtn.activated() }
    }
    SetRow {
        id: setBtn
        visible: !!page.st && page.st.available && !page.st.ntp
        icon: "check"; chevron: false; title: admin.what === "time" ? "Setting…" : "Set the clock"
        sub: "Date as year-month-day, time in 24-hour form"
        onActivated: {
            const d = dateField.text.trim(), t = timeField.text.trim().replace(/^(\d):/, "0$1:");
            if (!/^\d{4}-\d\d-\d\d$/.test(d)) return page.say("Type the date like 2026-10-20.", true);
            if (!/^\d\d:\d\d$/.test(t)) return page.say("Type the time like 14:05.", true);
            page.runAdmin(["time-set", d + " " + t], "time", "Clock set.", () => { dateField.text = ""; timeField.text = ""; });
        }
    }

    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
    SetRow {
        visible: page.zoneChanged
        icon: "restart"; chevron: false
        title: "Restart the desktop now"; sub: "The taskbar clock shows the new time zone after the desktop restarts (your apps stay open)"
        onActivated: Qt.quit()
    }

    // ------------------------------------------------ time zone
    SetHeader { text: "Time zone" + (admin.what === "zone" ? " · saving…" : ""); visible: !!page.st && page.st.admin }
    SetField { id: search; visible: !!page.st && page.st.admin; placeholder: "Search for a city, country region or UTC offset" }
    Repeater {
        model: page.matches
        delegate: SetRow {
            required property var modelData
            icon: "globe"; chevron: false
            title: modelData.city
            sub: modelData.region + " · UTC" + modelData.offset
            value: page.st && page.st.timezone === modelData.zone ? "In use" : ""
            onActivated: page.setZone(modelData)
        }
    }
    Text {
        visible: page.query !== "" && page.matches.length === 0
        text: "No time zone matches \"" + search.text + "\""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
    }
    SetRow {
        id: allRegions
        visible: page.query === "" && page.region !== ""
        icon: "back"; chevron: false; title: "All regions"; sub: page.region
        onActivated: { const was = page.region; page.region = ""; Qt.callLater(() => { const i = page.regions.findIndex(r => r.name === was); const it = regionList.itemAt(Math.max(0, i)); if (it) it.forceActiveFocus(); }); }
    }
    Repeater {
        id: regionList
        model: page.query === "" && page.region === "" && page.st && page.st.admin ? page.regions : []
        delegate: SetRow {
            required property var modelData
            icon: "globe"
            title: modelData.name
            value: page.st && page.st.timezone.split("/")[0] === modelData.name ? "In use" : modelData.count + " places"
            onActivated: { page.region = modelData.name; Qt.callLater(() => allRegions.forceActiveFocus()); }
        }
    }
    Repeater {
        model: page.query === "" ? page.cities : []
        delegate: SetRow {
            required property var modelData
            icon: "clock"; chevron: false
            title: modelData.city
            sub: "UTC" + modelData.offset
            value: page.st && page.st.timezone === modelData.zone ? "In use" : ""
            onActivated: page.setZone(modelData)
        }
    }
    Text {
        visible: !!page.st && !page.st.admin
        Layout.fillWidth: true
        text: "The time zone and automatic time are set here on Hearth, the OS."
        color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
