// Hearth Settings › Personalization › Window effects (Mantel on KWin, docs/KWIN-SETTINGS.md): KWin's own effects, Hearth's
// controls. How fast animations run (kdeglobals [KDE] AnimationDurationFactor), the overview and window views, the window
// switcher's look, blur and see-through windows, wobbly windows, minimize, open and close animations, dimming, the
// pointer effects and the rest of KWin 6's effects (kwinrc [Plugins] and each effect's [Effect-*] group). Only what
// you change is written; KWin takes it at once.
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Window effects"
    KwinSettings { id: kw }
    // AnimationDurationFactor: KDE's steps (kcms/compositing).
    readonly property var speeds: [{ value: 4, label: "Very slow" }, { value: 2, label: "Slow" }, { value: 1, label: "Normal" },
        { value: 0.5, label: "Fast" }, { value: 0.25, label: "Very fast" }, { value: 0, label: "Instant" }]
    function speedNow() {
        const f = kw.n("kdeglobals", "KDE", "AnimationDurationFactor", 1);
        let best = speeds[2].value, d = 99;
        for (const s of speeds) if (Math.abs(s.value - f) < d) { d = Math.abs(s.value - f); best = s.value; }
        return best;
    }
    // One of several effects that do the same job (KWin's exclusive categories): turning one on turns the others off.
    function pickOne(ids, chosen) {
        const g = {};
        for (const id of ids) if (kw.effectOn(id) !== (id === chosen)) g[id + "Enabled"] = id === chosen;
        if (Object.keys(g).length) kw.setMany("kwinrc", { Plugins: g });
    }
    function onOf(ids) { for (const id of ids) if (kw.effectOn(id)) return id; return "none"; }
    function opt(id, label) { return { value: id, label: label, disabled: id !== "none" && !kw.hasEffect(id) }; }
    function effSub(id, text) { return kw.hasEffect(id) ? text : text + ". Not in this KWin"; }

    KwinGate {
        kw: kw
        what: "Window effects"

        SetHeader { text: "Animations" + (kw.job.what === "save" ? " · saving…" : "") }
        SetChoice {
            icon: "motion"; title: "Animation speed"
            sub: "How fast windows open, close, move and switch. Reduce motion in Accessibility also stills Mantel's own"
            options: page.speeds
            current: page.speedNow()
            onChosen: v => kw.set("kdeglobals", "KDE", "AnimationDurationFactor", v)
        }
        SetChoice {
            icon: "window"; title: "When a window opens or closes"
            options: [page.opt("scale", "Grow"), page.opt("glide", "Glide"), page.opt("fade", "Fade"), page.opt("none", "Nothing")]
            current: page.onOf(["scale", "glide", "fade"])
            onChosen: v => page.pickOne(["scale", "glide", "fade"], v)
        }
        SetChoice {
            icon: "window"; title: "When a window is minimized"
            options: [page.opt("squash", "Squash"), page.opt("magiclamp", "Magic lamp"), page.opt("none", "Nothing")]
            current: page.onOf(["squash", "magiclamp"])
            onChosen: v => page.pickOne(["squash", "magiclamp"], v)
        }
        SetChoice {
            icon: "home"; title: "When the desktop is shown"
            sub: "Windows move out of the way, or shrink to the middle of the screen"
            options: [page.opt("windowaperture", "Move aside"), page.opt("eyeonscreen", "Eye on screen"), page.opt("none", "Nothing")]
            current: page.onOf(["windowaperture", "eyeonscreen"])
            onChosen: v => page.pickOne(["windowaperture", "eyeonscreen"], v)
        }
        SetLinkCard { target: "kwin-desktops"; title: "Switching between desktops"; sub: "Slide, fade or nothing: in Virtual desktops" }
        SetSwitch {
            icon: "window"; title: "Maximize and full screen animate"
            sub: page.effSub("maximize", "Windows grow smoothly when maximized")
            checked: kw.effectOn("maximize"); enabled: kw.hasEffect("maximize")
            onToggled: v => kw.setEffect("maximize", v)
        }
        SetSwitch {
            icon: "menu"; title: "Menus and pop-ups fade and slide"
            checked: kw.effectOn("fadingpopups") || kw.effectOn("slidingpopups")
            onToggled: v => kw.setMany("kwinrc", { Plugins: { fadingpopupsEnabled: v, slidingpopupsEnabled: v } })
        }
        SetSwitch {
            icon: "window"; title: "Dialogs slide out of their window"
            sub: page.effSub("sheet", "A message from an app drops out of the top of its window")
            checked: kw.effectOn("sheet"); enabled: kw.hasEffect("sheet")
            onToggled: v => kw.setEffect("sheet", v)
        }
        SetSwitch {
            icon: "trash"; title: "Windows fall apart when closed"
            sub: page.effSub("fallapart", "Closing a window breaks it into pieces")
            checked: kw.effectOn("fallapart"); enabled: kw.hasEffect("fallapart")
            onToggled: v => kw.setEffect("fallapart", v)
        }
        SetSwitch {
            icon: "power"; title: "Fade in when you sign in, fade out when you sign out"
            checked: kw.effectOn("login")
            onToggled: v => kw.setMany("kwinrc", { Plugins: { loginEnabled: v, logoutEnabled: v } })
        }
        SetSwitch {
            icon: "rocket"; title: "Bounce the pointer while an app starts"
            checked: kw.effectOn("startupfeedback")
            onToggled: v => kw.setEffect("startupfeedback", v)
        }

        // ---------------------------------------------------- overview and window views
        SetHeader { text: "Seeing your windows" }
        SetSwitch {
            icon: "grid"; title: "Overview"
            sub: page.effSub("overview", "All your windows and desktops at once, with search. Super+W, four fingers up on a touchpad, or the hot corner")
            checked: kw.effectOn("overview"); enabled: kw.hasEffect("overview")
            onToggled: v => kw.setEffect("overview", v)
        }
        SetSwitch {
            visible: kw.effectOn("overview")
            icon: "grid"; title: "Overview: leave out minimized windows"
            checked: kw.b("kwinrc", "Effect-overview", "IgnoreMinimized", false)
            onToggled: v => kw.set("kwinrc", "Effect-overview", "IgnoreMinimized", v)
        }
        SetSwitch {
            visible: kw.effectOn("overview")
            icon: "search"; title: "Overview: typing searches"
            sub: "Off: typing only filters the windows shown"
            checked: !kw.b("kwinrc", "Effect-overview", "FilterWindows", true)
            onToggled: v => kw.set("kwinrc", "Effect-overview", "FilterWindows", !v)
        }
        SetRow {
            visible: kw.effectOn("overview")
            icon: "desktops"; title: "Desktop grid"
            sub: "Every desktop side by side, to move windows between them: Super+G, or four fingers down. Part of the overview in KWin 6"
            chevron: false
            onActivated: kw.shortcut("grid")
        }
        SetSwitch {
            icon: "window"; title: "Present windows"
            sub: page.effSub("windowview", "Every window side by side to pick one (Ctrl+F9, or Ctrl+F10 for this app's)")
            checked: kw.effectOn("windowview"); enabled: kw.hasEffect("windowview")
            onToggled: v => kw.setEffect("windowview", v)
        }
        SetLinkCard { target: "kwin-edges"; title: "Hot corners"; sub: "Which corner opens the overview, the grid or every window" }

        // ---------------------------------------------------- window switcher
        SetHeader { text: "Window switcher (Alt+Tab)" }
        SetDropdown {
            icon: "window"; title: "Look"
            sub: "What Alt+Tab shows while you hold it"
            options: (kw.st && kw.st.switchers ? kw.st.switchers : [{ id: "thumbnail_grid", label: "Thumbnail grid" }])
                .map(s => ({ value: s.id, label: s.hearth ? s.label + " (Hearth)" : s.label }))
                .concat([{ value: "__none", label: "No switcher, just switch" }])
            current: kw.b("kwinrc", "TabBox", "ShowTabBox", true) ? kw.v("kwinrc", "TabBox", "LayoutName", "thumbnail_grid") : "__none"
            onChosen: v => v === "__none" ? kw.set("kwinrc", "TabBox", "ShowTabBox", false)
                                          : kw.setMany("kwinrc", { TabBox: { ShowTabBox: true, LayoutName: v } })
        }
        SetSwitch {
            icon: "eye"; title: "Show the window you're on"
            sub: "The others fade while Alt+Tab is held"
            checked: kw.b("kwinrc", "TabBox", "HighlightWindows", true)
            onToggled: v => kw.set("kwinrc", "TabBox", "HighlightWindows", v)
        }
        SetChoice {
            icon: "desktops"; title: "Which windows"
            options: [{ value: 1, label: "This desktop" }, { value: 0, label: "Every desktop" }]
            current: kw.n("kwinrc", "TabBox", "DesktopMode", 1)
            onChosen: v => kw.set("kwinrc", "TabBox", "DesktopMode", v)
        }
        SetChoice {
            icon: "window"; title: "Minimized windows"
            options: [{ value: 0, label: "Included" }, { value: 1, label: "Left out" }, { value: 2, label: "Only those" }]
            current: kw.n("kwinrc", "TabBox", "MinimizedMode", 0)
            onChosen: v => kw.set("kwinrc", "TabBox", "MinimizedMode", v)
        }
        SetChoice {
            icon: "grid"; title: "Order"
            options: [{ value: 0, label: "Most recent first" }, { value: 1, label: "As they are stacked" }]
            current: kw.n("kwinrc", "TabBox", "SwitchingMode", 0)
            onChosen: v => kw.set("kwinrc", "TabBox", "SwitchingMode", v)
        }

        // ---------------------------------------------------- glass
        SetHeader { text: "Blur and see-through windows" }
        SetSwitch {
            icon: "sparkle"; title: "Blur behind see-through windows"
            sub: page.effSub("blur", "Panels, menus and windows that let the background show through blur it")
            checked: kw.effectOn("blur"); enabled: kw.hasEffect("blur")
            onToggled: v => kw.setEffect("blur", v)
        }
        SetSlider {
            visible: kw.effectOn("blur")
            icon: "sparkle"; title: "How much blur"
            value: (kw.n("kwinrc", "Effect-blur", "BlurStrength", 15) - 1) / 14
            step: 1 / 14
            valueText: Math.round(kw.n("kwinrc", "Effect-blur", "BlurStrength", 15))
            onMoved: v => kw.set("kwinrc", "Effect-blur", "BlurStrength", Math.round(1 + v * 14))
        }
        SetSlider {
            visible: kw.effectOn("blur")
            icon: "sparkle"; title: "Grain in the blur"
            sub: "A little grain hides banding"
            value: kw.n("kwinrc", "Effect-blur", "NoiseStrength", 5) / 14
            step: 1 / 14
            valueText: Math.round(kw.n("kwinrc", "Effect-blur", "NoiseStrength", 5))
            onMoved: v => kw.set("kwinrc", "Effect-blur", "NoiseStrength", Math.round(v * 14))
        }
        SetSwitch {
            icon: "sun"; title: "Keep text readable on glass"
            sub: page.effSub("backgroundcontrast", "More contrast behind see-through panels")
            checked: kw.effectOn("backgroundcontrast"); enabled: kw.hasEffect("backgroundcontrast")
            onToggled: v => kw.setEffect("backgroundcontrast", v)
        }
        SetSwitch {
            icon: "window"; title: "See-through windows"
            sub: page.effSub("translucency", "Windows become see-through while moved, when not in front, and menus")
            checked: kw.effectOn("translucency"); enabled: kw.hasEffect("translucency")
            onToggled: v => kw.setEffect("translucency", v)
        }
        Repeater {
            model: [{ key: "MoveResize", label: "While moved or resized", d: 80 }, { key: "Inactive", label: "Windows not in front", d: 100 },
                    { key: "Dialogs", label: "Dialogs", d: 100 }, { key: "Menus", label: "Menus", d: 100 }]
            delegate: SetSlider {
                required property var modelData
                visible: kw.effectOn("translucency")
                icon: "window"; title: modelData.label
                value: kw.n("kwinrc", "Effect-translucency", modelData.key, modelData.d) / 100
                valueText: Math.round(value * 100) + "% solid"
                onMoved: v => kw.set("kwinrc", "Effect-translucency", modelData.key, Math.max(10, Math.round(v * 100)))
            }
        }

        // ---------------------------------------------------- fun
        SetHeader { text: "Moving windows" }
        SetSwitch {
            icon: "wave"; title: "Wobbly windows"
            sub: page.effSub("wobblywindows", "Windows wobble like jelly while you move them")
            checked: kw.effectOn("wobblywindows"); enabled: kw.hasEffect("wobblywindows")
            onToggled: v => kw.setEffect("wobblywindows", v)
        }
        SetChoice {
            visible: kw.effectOn("wobblywindows")
            icon: "wave"; title: "How wobbly"
            options: [{ value: 0, label: "A little" }, { value: 1, label: "Some" }, { value: 2, label: "Medium" }, { value: 3, label: "Lots" }, { value: 4, label: "Most" }]
            current: kw.n("kwinrc", "Effect-wobblywindows", "WobblynessLevel", 0)
            onChosen: v => kw.set("kwinrc", "Effect-wobblywindows", "WobblynessLevel", v)
        }
        SetSwitch {
            icon: "grid"; title: "Show where a window snaps"
            sub: page.effSub("snaphelper", "Lines to the middle of the screen while you move a window")
            checked: kw.effectOn("snaphelper"); enabled: kw.hasEffect("snaphelper")
            onToggled: v => kw.setEffect("snaphelper", v)
        }
        SetSwitch {
            icon: "window"; title: "Window you click slides to the front"
            sub: page.effSub("slideback", "Windows above it slide away and back")
            checked: kw.effectOn("slideback"); enabled: kw.hasEffect("slideback")
            onToggled: v => kw.setEffect("slideback", v)
        }
        SetSwitch {
            icon: "image"; title: "A picture of the window you're on, at the side"
            sub: page.effSub("thumbnailaside", "Super+Ctrl+T puts one there")
            checked: kw.effectOn("thumbnailaside"); enabled: kw.hasEffect("thumbnailaside")
            onToggled: v => kw.setEffect("thumbnailaside", v)
        }

        // ---------------------------------------------------- focus
        SetHeader { text: "Dimming" }
        SetSwitch {
            icon: "moon"; title: "Dim windows you're not using"
            sub: page.effSub("diminactive", "Only the window in front stays bright")
            checked: kw.effectOn("diminactive"); enabled: kw.hasEffect("diminactive")
            onToggled: v => kw.setEffect("diminactive", v)
        }
        SetSlider {
            visible: kw.effectOn("diminactive")
            icon: "moon"; title: "How much"
            value: kw.n("kwinrc", "Effect-diminactive", "Strength", 25) / 100
            onMoved: v => kw.set("kwinrc", "Effect-diminactive", "Strength", Math.max(5, Math.round(v * 100)))
        }
        SetSwitch {
            visible: kw.effectOn("diminactive")
            icon: "window"; title: "Dim the bar and dock too"
            checked: kw.b("kwinrc", "Effect-diminactive", "DimPanels", false)
            onToggled: v => kw.set("kwinrc", "Effect-diminactive", "DimPanels", v)
        }
        SetSwitch {
            icon: "shield"; title: "Dim the screen when asked for your password"
            sub: page.effSub("dimscreen", "When an app asks for admin rights, everything else dims")
            checked: kw.effectOn("dimscreen"); enabled: kw.hasEffect("dimscreen")
            onToggled: v => kw.setEffect("dimscreen", v)
        }
        SetSwitch {
            icon: "warn"; title: "Grey out apps that stopped answering"
            checked: kw.effectOn("frozenapp")
            onToggled: v => kw.setEffect("frozenapp", v)
        }
        SetSwitch {
            icon: "window"; title: "Darken a window while its dialog is open"
            checked: kw.effectOn("dialogparent")
            onToggled: v => kw.setEffect("dialogparent", v)
        }

        // ---------------------------------------------------- pointer
        SetHeader { text: "Pointer" }
        SetSwitch {
            icon: "mouse"; title: "Shake the pointer to find it"
            sub: page.effSub("shakecursor", "Wiggle the mouse and the pointer grows for a moment")
            checked: kw.effectOn("shakecursor"); enabled: kw.hasEffect("shakecursor")
            onToggled: v => kw.setEffect("shakecursor", v)
        }
        SetSlider {
            visible: kw.effectOn("shakecursor")
            icon: "mouse"; title: "How big it grows"
            value: (kw.n("kwinrc", "Effect-shakecursor", "Magnification", 3) - 1.5) / 4.5
            valueText: kw.n("kwinrc", "Effect-shakecursor", "Magnification", 3).toFixed(1) + "×"
            onMoved: v => kw.set("kwinrc", "Effect-shakecursor", "Magnification", Math.round((1.5 + v * 4.5) * 2) / 2)
        }
        SetSwitch {
            icon: "mouse"; title: "Show mouse clicks"
            sub: page.effSub("mouseclick", "Rings where you click (Super+* turns them on and off): handy for recording")
            checked: kw.effectOn("mouseclick"); enabled: kw.hasEffect("mouseclick")
            onToggled: v => kw.setEffect("mouseclick", v)
        }
        SetSwitch {
            icon: "search"; title: "Find the pointer with Super+Ctrl"
            sub: page.effSub("trackmouse", "Hold Super and Ctrl: a ring shows where the pointer is")
            checked: kw.effectOn("trackmouse"); enabled: kw.hasEffect("trackmouse")
            onToggled: v => kw.setEffect("trackmouse", v)
        }
        SetSwitch {
            icon: "edit"; title: "Draw on the screen"
            sub: page.effSub("mousemark", "Hold Super+Shift and drag to draw; Super+Shift+F11 clears it")
            checked: kw.effectOn("mousemark"); enabled: kw.hasEffect("mousemark")
            onToggled: v => kw.setEffect("mousemark", v)
        }
        SetSwitch {
            icon: "mouse"; title: "Hide the pointer while you type"
            sub: page.effSub("hidecursor", "It comes back when you move the mouse")
            checked: kw.effectOn("hidecursor") && kw.b("kwinrc", "Effect-hidecursor", "HideOnTyping", true); enabled: kw.hasEffect("hidecursor")
            onToggled: v => kw.setMany("kwinrc", { Plugins: { hidecursorEnabled: v || kw.n("kwinrc", "Effect-hidecursor", "InactivityDuration", 0) > 0 }, "Effect-hidecursor": { HideOnTyping: v } })
        }
        SetSwitch {
            icon: "pad"; title: "Show touches on a touchscreen"
            sub: page.effSub("touchpoints", "Circles where fingers touch")
            checked: kw.effectOn("touchpoints"); enabled: kw.hasEffect("touchpoints")
            onToggled: v => kw.setEffect("touchpoints", v)
        }
    }
}
