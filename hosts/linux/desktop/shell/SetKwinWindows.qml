// Hearth Settings › Personalization › Window behaviour (Mantel on KWin, docs/KWIN-SETTINGS.md): which window gets the
// keyboard (click, follows the pointer, under the pointer), raising on hover with its delay, whether a click brings a
// window to the front, how hard KWin stops windows taking the focus, what the title bar does on a double-click, the
// wheel and a middle click, what Super (or Alt) plus a drag does, and where new windows open. KWin keeps these in
// kwinrc [Windows] and [MouseBindings]; hearth-kwin-settings writes only the keys changed here, and a choice set back to
// KWin's default (Hearth's too) takes the key out of your file. Under labwc it's greyed (KwinGate).
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Window behaviour"
    KwinSettings { id: kw }

    // A choice back on its default is taken out of your kwinrc, so the system value counts again.
    function put(group, key, value, dflt) { kw.set("kwinrc", group, key, value === dflt ? null : value); }
    function ms(group, key, dflt) { return kw.n("kwinrc", group, key, dflt); }

    readonly property string policy: kw.v("kwinrc", "Windows", "FocusPolicy", "ClickToFocus")
    readonly property bool byPointer: policy !== "ClickToFocus"
    readonly property string mod: kw.v("kwinrc", "MouseBindings", "CommandAllKey", "Meta") === "Alt" ? "Alt" : "Super"

    readonly property var policies: [
        { value: "ClickToFocus", label: "Click a window", desc: "The window you click gets the keyboard" },
        { value: "FocusFollowsMouse", label: "Follows the pointer", desc: "The window the pointer moves onto gets it, and keeps it over the desktop" },
        { value: "FocusUnderMouse", label: "Under the pointer", desc: "Whichever window is under the pointer has it" },
        { value: "FocusStrictlyUnderMouse", label: "Strictly under the pointer", desc: "The same, and no window has it over the desktop" }]
    readonly property var stealing: [
        { value: 0, label: "Off", desc: "Any window can take the keyboard when it opens" },
        { value: 1, label: "Low", desc: "KWin's usual: a window you didn't ask for waits if you're typing" },
        { value: 2, label: "Medium", desc: "Stricter about windows that open by themselves" },
        { value: 3, label: "High", desc: "Only windows you just opened get the keyboard" },
        { value: 4, label: "Extreme", desc: "Only a click gives a window the keyboard" }]
    readonly property var doubleClick: [
        { value: "Maximize", label: "Maximize and restore" },
        { value: "Maximize (vertical only)", label: "Maximize up and down" },
        { value: "Maximize (horizontal only)", label: "Maximize side to side" },
        { value: "Minimize", label: "Minimize" },
        { value: "Shade", label: "Roll up into the title bar" },
        { value: "Lower", label: "Send it behind the others" },
        { value: "OnAllDesktops", label: "Show on every desktop" },
        { value: "Close", label: "Close" },
        { value: "Nothing", label: "Nothing" }]
    readonly property var wheel: [
        { value: "Nothing", label: "Nothing" },
        { value: "Raise/Lower", label: "Bring to front / send behind" },
        { value: "Shade/Unshade", label: "Roll up / roll down" },
        { value: "Maximize/Restore", label: "Maximize / restore" },
        { value: "Above/Below", label: "Keep above / keep below" },
        { value: "Previous/Next Desktop", label: "Previous / next desktop" },
        { value: "Change Opacity", label: "See-through more / less" }]
    readonly property var middle: [
        { value: "Nothing", label: "Nothing" },
        { value: "Lower", label: "Send it behind the others" },
        { value: "Toggle raise and lower", label: "Bring to front or send behind" },
        { value: "Minimize", label: "Minimize" },
        { value: "Close", label: "Close" },
        { value: "Operations menu", label: "Open the window menu" }]
    readonly property var drag: [
        { value: "Move", label: "Move the window" },
        { value: "Resize", label: "Resize the window" },
        { value: "Operations menu", label: "Open the window menu" },
        { value: "Toggle raise and lower", label: "Bring to front or send behind" },
        { value: "Minimize", label: "Minimize" },
        { value: "Nothing", label: "Nothing" }]
    readonly property var placements: [
        { value: "Centered", label: "In the middle of the screen", desc: "KWin's usual" },
        { value: "Smart", label: "Where it covers the least", desc: "Over as little of your other windows as it can" },
        { value: "UnderMouse", label: "Under the pointer" },
        { value: "Maximizing", label: "Maximized", desc: "Windows that can be maximized open full size" },
        { value: "ZeroCornered", label: "Top-left corner" },
        { value: "Random", label: "Anywhere" }]
    function descOf(list, v) { const o = list.find(x => x.value === v); return o && o.desc ? o.desc : ""; }

    KwinGate {
        kw: kw
        what: "Window behaviour settings"

        SetHeader { text: "Focus" + (kw.job.what === "save" ? " · saving…" : "") }
        SetDropdown {
            icon: "kbd"; title: "Which window gets the keyboard"
            sub: page.descOf(page.policies, page.policy)
            options: page.policies
            current: page.policy
            onChosen: v => page.put("Windows", "FocusPolicy", v, "ClickToFocus")
        }
        SetSlider {
            enabled: page.byPointer; opacity: page.byPointer ? 1 : 0.45
            icon: "clock"; title: "Wait before the focus follows"
            sub: page.byPointer ? "How long the pointer rests on a window before it gets the keyboard" : "When the focus follows the pointer"
            value: page.ms("Windows", "DelayFocusInterval", 300) / 2000
            step: 0.025
            valueText: Math.round(value * 2000) + " ms"
            onMoved: v => page.put("Windows", "DelayFocusInterval", Math.round(v * 40) * 50, 300)
        }
        SetSwitch {
            enabled: page.byPointer; opacity: page.byPointer ? 1 : 0.45
            icon: "window"; title: "Raise the window under the pointer"
            sub: page.byPointer ? "It comes to the front when the pointer rests on it" : "When the focus follows the pointer"
            checked: kw.b("kwinrc", "Windows", "AutoRaise", false)
            onToggled: v => page.put("Windows", "AutoRaise", v, false)
        }
        SetSlider {
            visible: page.byPointer && kw.b("kwinrc", "Windows", "AutoRaise", false)
            icon: "clock"; title: "Raise after"
            value: page.ms("Windows", "AutoRaiseInterval", 750) / 3000
            step: 1 / 60
            valueText: Math.round(value * 3000) + " ms"
            onMoved: v => page.put("Windows", "AutoRaiseInterval", Math.round(v * 60) * 50, 750)
        }
        SetSwitch {
            icon: "mouse"; title: "Clicking a window brings it to the front"
            sub: "Off: a window you click in stays where it is in the stack"
            checked: kw.b("kwinrc", "Windows", "ClickRaise", true)
            onToggled: v => page.put("Windows", "ClickRaise", v, true)
        }
        SetSwitch {
            icon: "mouse"; title: "The pointer comes first"
            sub: "When a window closes or you switch desktops, the window under the pointer gets the keyboard, not the last one you used"
            checked: kw.b("kwinrc", "Windows", "NextFocusPrefersMouse", false)
            onToggled: v => page.put("Windows", "NextFocusPrefersMouse", v, false)
        }
        SetDropdown {
            icon: "pin"; title: "Stop windows taking the keyboard"
            sub: page.descOf(page.stealing, kw.n("kwinrc", "Windows", "FocusStealingPreventionLevel", 1))
            options: page.stealing
            current: kw.n("kwinrc", "Windows", "FocusStealingPreventionLevel", 1)
            onChosen: v => page.put("Windows", "FocusStealingPreventionLevel", v, 1)
        }

        SetHeader { text: "Title bar" }
        SetDropdown {
            icon: "window"; title: "Double-click"
            options: page.doubleClick
            current: kw.v("kwinrc", "Windows", "TitlebarDoubleClickCommand", "Maximize")
            onChosen: v => page.put("Windows", "TitlebarDoubleClickCommand", v, "Maximize")
        }
        SetDropdown {
            icon: "mouse"; title: "Middle-click"
            options: page.middle
            current: kw.v("kwinrc", "MouseBindings", "CommandActiveTitlebar2", "Nothing")
            onChosen: v => {
                page.put("MouseBindings", "CommandActiveTitlebar2", v, "Nothing");
                page.put("MouseBindings", "CommandInactiveTitlebar2", v, "Nothing");
            }
        }
        SetDropdown {
            icon: "mouse"; title: "Turning the wheel"
            options: page.wheel
            current: kw.v("kwinrc", "MouseBindings", "CommandTitlebarWheel", "Nothing")
            onChosen: v => page.put("MouseBindings", "CommandTitlebarWheel", v, "Nothing")
        }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: "A right-click on the title bar opens the window menu (with Snap and More actions)."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }

        SetHeader { text: page.mod + " + drag anywhere in a window" }
        SetChoice {
            icon: "kbd"; title: "Hold"
            sub: "Pressed on its own, Super still opens the app menu"
            options: [{ value: "Meta", label: "Super" }, { value: "Alt", label: "Alt" }]
            current: page.mod === "Alt" ? "Alt" : "Meta"
            onChosen: v => page.put("MouseBindings", "CommandAllKey", v, "Meta")
        }
        SetDropdown {
            icon: "mouse"; title: page.mod + " + left button"
            options: page.drag
            current: kw.v("kwinrc", "MouseBindings", "CommandAll1", "Move")
            onChosen: v => page.put("MouseBindings", "CommandAll1", v, "Move")
        }
        SetDropdown {
            icon: "mouse"; title: page.mod + " + right button"
            options: page.drag
            current: kw.v("kwinrc", "MouseBindings", "CommandAll3", "Resize")
            onChosen: v => page.put("MouseBindings", "CommandAll3", v, "Resize")
        }
        SetDropdown {
            icon: "mouse"; title: page.mod + " + middle button"
            options: page.drag
            current: kw.v("kwinrc", "MouseBindings", "CommandAll2", "Toggle raise and lower")
            onChosen: v => page.put("MouseBindings", "CommandAll2", v, "Toggle raise and lower")
        }
        SetDropdown {
            icon: "mouse"; title: page.mod + " + wheel"
            options: page.wheel
            current: kw.v("kwinrc", "MouseBindings", "CommandAllWheel", "Nothing")
            onChosen: v => page.put("MouseBindings", "CommandAllWheel", v, "Nothing")
        }

        SetHeader { text: "New windows" }
        SetDropdown {
            icon: "window"; title: "Where new windows open"
            sub: page.descOf(page.placements, kw.v("kwinrc", "Windows", "Placement", "Centered"))
            options: page.placements
            current: kw.v("kwinrc", "Windows", "Placement", "Centered")
            onChosen: v => page.put("Windows", "Placement", v, "Centered")
        }
        SetLinkCard { target: "kwin-rules"; title: "Just one app's windows"; sub: "A window rule opens them where, how big and how you want" }
        Text {
            visible: kw.message !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4
            text: kw.message; color: kw.messageBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
    }
}
