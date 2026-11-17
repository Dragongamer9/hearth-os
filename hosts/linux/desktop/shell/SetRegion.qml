// Hearth Settings › System › Region & language: the language apps and the login screen use, and how numbers, dates,
// money, measurements and paper are written (the region's own ways, or one of them from another region). The whole
// device's (localectl, through hearth-admin locale), from the next sign-in; changed only when you pick. The clock's
// 12 or 24 hours and the time zone are on Date & time.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Region & language"
    property var st: null              // hearth-region status
    property var look: null            // preview of the formats in use
    property string message: ""
    property bool messageBad: false
    property bool signOut: false
    readonly property var options: st ? st.locales.map(l => ({ value: l.id, label: l.name })) : []
    readonly property string formats: st ? (st.formats.LC_TIME || st.formats.LC_NUMERIC || st.lang) : ""
    function nameOf(id) { const l = st ? st.locales.find(x => x.id === id) : null; return l ? l.name : id; }

    function say(t, bad) { message = t; messageBad = !!bad; }
    function load() { job.run(["status"], r => { page.st = r; page.preview(); }, e => page.say(e, true)); }
    function preview() {
        if (!st) return;
        const f = st.formats;
        job.run(["preview", f.LC_NUMERIC || st.lang], n => {
            job.run(["preview", f.LC_TIME || st.lang], t => {
                job.run(["preview", f.LC_MONETARY || st.lang], m => {
                    job.run(["preview", f.LC_MEASUREMENT || st.lang], u => {
                        job.run(["preview", f.LC_PAPER || st.lang], p => page.look = { number: n.number, date: t.longDate, time: t.time, money: m.money, metric: u.metric, paper: p.paper });
                    });
                });
            });
        });
    }
    Component.onCompleted: load()
    SetJob { id: job; helper: "hearth-region" }
    SetJob { id: admin; helper: "/usr/libexec/nimbus/hearth-admin"; admin: true }
    // Every key sent each time (LANG plus the formats you chose), so a change never drops another one.
    function save(changes, ok) {
        say("");
        const v = Object.assign({ LANG: st.lang }, st.formats, changes);
        const args = ["locale"];
        for (const k of ["LANG", "LC_NUMERIC", "LC_TIME", "LC_MONETARY", "LC_MEASUREMENT", "LC_PAPER"])
            if (v[k] !== undefined && v[k] !== "" && (k === "LANG" || v[k] !== v.LANG)) args.push(k + "=" + v[k]);
        admin.run(args, () => { page.signOut = true; page.say(ok); page.load(); }, e => page.say(e, true), "", "save");
    }

    SetCardGroup {
        title: "Language"
        SetDropdown {
            icon: "globe"; title: "Language"; sub: "Menus, apps and the login screen. Apps that don't have it stay in English"
            options: page.options
            current: page.st ? page.st.lang : ""
            boxWidth: 320
            onChosen: v => page.save({ LANG: v }, "Language set to " + page.nameOf(v) + ". Sign out and in to see it everywhere.")
        }
    }
    SetCardGroup {
        title: "Formats"
        SetDropdown {
            icon: "globe"; title: "Region"; sub: "How dates, numbers and money are written"
            options: page.options
            current: page.formats
            boxWidth: 320
            onChosen: v => page.save({ LC_NUMERIC: v, LC_TIME: v, LC_MONETARY: v, LC_MEASUREMENT: v, LC_PAPER: v }, "Formats of " + page.nameOf(v) + ".")
        }
        SetRow {
            interactive: false; chevron: false; icon: "info"
            title: page.look ? page.look.date + " · " + page.look.time : "…"
            sub: page.look ? page.look.number + " · " + page.look.money + " · " + (page.look.metric ? "Metric" : "US units") + " · " + page.look.paper + " paper" : ""
        }
        SetExpander {
            icon: "gear"; title: "Change one of them"; sub: "Numbers, dates, money, measurements or paper from another region"
            SetDropdown { title: "Numbers"; options: page.options; current: page.st ? (page.st.formats.LC_NUMERIC || page.st.lang) : ""; boxWidth: 300; onChosen: v => page.save({ LC_NUMERIC: v }, "Saved.") }
            SetDropdown { title: "Dates and times"; options: page.options; current: page.st ? (page.st.formats.LC_TIME || page.st.lang) : ""; boxWidth: 300; onChosen: v => page.save({ LC_TIME: v }, "Saved.") }
            SetDropdown { title: "Money"; options: page.options; current: page.st ? (page.st.formats.LC_MONETARY || page.st.lang) : ""; boxWidth: 300; onChosen: v => page.save({ LC_MONETARY: v }, "Saved.") }
            SetDropdown { title: "Measurement units"; options: page.options; current: page.st ? (page.st.formats.LC_MEASUREMENT || page.st.lang) : ""; boxWidth: 300; onChosen: v => page.save({ LC_MEASUREMENT: v }, "Saved.") }
            SetDropdown { title: "Paper size"; options: page.options; current: page.st ? (page.st.formats.LC_PAPER || page.st.lang) : ""; boxWidth: 300; onChosen: v => page.save({ LC_PAPER: v }, "Saved.") }
        }
        SetRow {
            icon: "clock"; title: "Date & time"; sub: "Time zone, 24-hour clock, setting the time automatically"
            chevron: true
            onActivated: SettingsNav.openPage("datetime")
        }
        SetRow {
            visible: page.signOut
            icon: "logout"; chevron: false; title: "Sign out now"; sub: "Apps pick up the new language and formats when you sign in again"
            onActivated: Quickshell.execDetached(["nimbus-mode", "logout"])
        }
        Text {
            visible: page.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
}
