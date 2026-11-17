// Hearth Settings › Gaming › Performance (1.14): Game Mode's resolution, refresh rate, frame limit and HDR, set from
// Desktop Mode (Display's "Related" row leads here; mockup settings-A2-display-mouse.png). Everything goes through
// hearth-gamemode-display (session.conf's NIMBUS_GAMESCOPE_ARGS line only: atomic, .bak, under the lock); nothing is
// written until you pick something. It's used the next time Game Mode starts; a mode the screen can't show doesn't
// leave the PC stuck (the session starts Game Mode at the screen's own mode and says so).
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Performance"
    property var st: null
    property string msg: ""
    SetJob { id: job; helper: "hearth-gamemode-display" }
    function load() { job.run(["get"], r => { page.st = r; }, e => page.msg = e); }
    function set(ch, said) { job.run(["set", JSON.stringify(ch)], r => { page.st = r; page.msg = ""; SettingsNav.toast(said + " · from the next time Game Mode starts"); }, e => page.msg = e); }
    Component.onCompleted: load()

    readonly property var scr: st ? st.screen : null
    readonly property var modes: scr ? scr.modes : []
    readonly property var size: st && st.w ? st.w + "x" + st.h : "auto"
    readonly property var rates: {
        const m = st && st.w ? modes.find(x => x.w === st.w && x.h === st.h) : modes[0];
        return m ? m.rates : [];
    }

    SetBanner {
        visible: page.msg !== ""
        kind: "bad"; title: "That didn't work"; sub: page.msg
    }
    SetHeader { text: "Game Mode's screen" }
    SetChoice {
        icon: "window"; title: "Resolution"
        sub: page.scr && page.scr.model ? "On " + page.scr.model + (page.modes.length ? " · its own is " + page.modes[0].w + " × " + page.modes[0].h : "") : "The screen's own unless you pick one"
        options: [{ value: "auto", label: "The screen's own" }].concat(page.modes.slice(0, SetStyle.big ? 8 : 3).map(m => ({ value: m.w + "x" + m.h, label: m.w + " × " + m.h })))
            .concat(page.size !== "auto" && !page.modes.slice(0, SetStyle.big ? 8 : 3).some(m => m.w + "x" + m.h === page.size) ? [{ value: page.size, label: page.size.replace("x", " × ") }] : [])
        current: page.size
        onChosen: v => { if (v === "auto") page.set({ w: 0, h: 0, r: 0 }, "The screen's own resolution"); else { const [w, h] = v.split("x").map(Number); page.set({ w: w, h: h }, w + " × " + h); } }
    }
    SetChoice {
        visible: page.rates.length > 1
        icon: "bolt"; title: "Refresh rate"; sub: "Higher is smoother; lower saves battery"
        options: [{ value: 0, label: "Highest" }].concat(page.rates.slice(0, SetStyle.big ? 6 : 3).map(r => ({ value: r, label: r + " Hz" })))
        current: page.st ? page.st.r : 0
        onChosen: v => page.set({ r: v }, v ? v + " Hz" : "The highest refresh rate")
    }
    SetChoice {
        icon: "rocket"; title: "Frame limit"
        sub: page.st && !page.st.has.limit ? "This PC's gamescope can't limit the frame rate" : "Steadier and cooler; games can still set their own in Steam's Quick Access"
        options: [{ value: 0, label: "Off" }, { value: 30, label: "30" }, { value: 40, label: "40" }, { value: 60, label: "60" }, { value: 90, label: "90" }, { value: 120, label: "120" }]
            .map(o => ({ value: o.value, label: o.label, disabled: !!page.st && !page.st.has.limit && o.value !== 0 }))
        current: page.st ? page.st.limit : 0
        onChosen: v => page.set({ limit: v }, v ? "Limited to " + v + " frames a second" : "No frame limit")
    }
    SetSwitch {
        icon: "sparkle"; title: "HDR"
        sub: !page.scr ? "" : !page.scr.hdr ? "This screen doesn't have HDR" : page.st && !page.st.has.hdr ? "This PC's gamescope can't do HDR" : "Brighter, more vivid picture in games and videos that have it"
        enabled: !!page.scr && page.scr.hdr && !!page.st && page.st.has.hdr
        checked: !!page.st && page.st.hdr
        onToggled: v => page.set({ hdr: v }, v ? "HDR on" : "HDR off")
    }
    SetRow {
        icon: "info"; chevron: false
        title: "Used the next time Game Mode starts"
        sub: "If the screen can't show what you picked, Game Mode starts at the screen's own mode and tells you."
    }
}
