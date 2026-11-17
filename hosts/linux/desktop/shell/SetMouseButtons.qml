// Hearth Settings › Bluetooth & devices › Mouse & touchpad › Mouse buttons (Mantel on KWin, docs/KWIN-SETTINGS.md):
// what a mouse's middle, back, forward and other extra buttons do. Press a button in the box to add it, then pick what
// it does: another mouse button, a key or a key combination (pressed in the key box), or nothing. These are KWin's
// button rebinds (kcminputrc [ButtonRebinds][Mouse], keys ExtraButtonN / MiddleButton), written through
// hearth-kwin-settings mouse-rebind; KWin watches the file, so a change works at once. Left and right are never
// changed here. Opening the page writes nothing. Deep link: mouse-buttons:press opens the "press a button" box.
import QtQuick
import QtQuick.Layouts

SetPage {
    id: page
    title: "Mouse buttons"
    KwinSettings { id: kw }

    readonly property var saved: (kw.files.kcminputrc || {})["ButtonRebinds][Mouse"] || ({})
    property var added: []          // buttons pressed in the box this time, not changed yet
    property bool pressing: false   // the "press a button" box is open
    property string keyFor: ""      // the button whose key is being pressed in the key box
    property string note: ""
    property bool noteBad: false
    function say(t, bad) { note = t; noteBad = !!bad; }

    // Qt's other names for the same buttons (hearth-kwin-settings MOUSE_ALIASES).
    readonly property var aliasOf: ({ BackButton: "ExtraButton1", XButton1: "ExtraButton1", ForwardButton: "ExtraButton2",
                                      XButton2: "ExtraButton2", TaskButton: "ExtraButton3", MidButton: "MiddleButton" })
    function norm(k) { return aliasOf[k] || k; }
    function valid(b) { return b === "MiddleButton" || /^ExtraButton([1-9]|1[0-9]|2[0-4])$/.test(b); }
    function order(b) { return b === "MiddleButton" ? 0 : parseInt(b.slice(11)); }
    function actionOf(b) {
        for (const k in saved) if (norm(k) === b) return String(saved[k] || "");
        return "";
    }
    // Back and forward always; any other button once it's changed or pressed in the box.
    readonly property var buttons: {
        const s = ["ExtraButton1", "ExtraButton2"];
        Object.keys(saved).map(norm).concat(added).forEach(b => { if (valid(b) && s.indexOf(b) < 0) s.push(b); });
        return s.sort((a, b) => order(a) - order(b));
    }
    function label(b) {
        if (b === "MiddleButton") return "Middle button (the wheel)";
        if (b === "ExtraButton1") return "Back button (on the side, nearer you)";
        if (b === "ExtraButton2") return "Forward button (on the side, further forward)";
        return "Extra button " + b.slice(11);
    }
    // A pressed button (Qt.MouseButton) as KWin's name for it; "" for left and right.
    function nameOf(btn) {
        if (btn === Qt.MiddleButton) return "MiddleButton";
        const n = Math.round(Math.log(btn) / Math.LN2) - 2;    // Qt.ExtraButton1 (Back) is 1 << 3
        return n >= 1 && n <= 24 ? "ExtraButton" + n : "";
    }
    // MouseButton,N is the Linux button code (linux/input-event-codes.h), as KWin sends it.
    readonly property var actions: [
        { value: "", label: "As it is" }, { value: "MouseButton,275", label: "Back" }, { value: "MouseButton,276", label: "Forward" },
        { value: "MouseButton,274", label: "Middle-click" }, { value: "MouseButton,273", label: "Right-click" },
        { value: "Key,Ctrl+C", label: "Copy (Ctrl+C)" }, { value: "Key,Ctrl+V", label: "Paste (Ctrl+V)" },
        { value: "Key,Ctrl+Z", label: "Undo (Ctrl+Z)" }, { value: "Key,Ctrl+Shift+Z", label: "Redo (Ctrl+Shift+Z)" },
        { value: "Key,Ctrl+Tab", label: "Next tab" }, { value: "Key,Ctrl+Shift+Tab", label: "Previous tab" },
        { value: "Key,Ctrl+W", label: "Close the tab (Ctrl+W)" }, { value: "Key,Meta+W", label: "Overview of windows" },
        { value: "Key,Volume Up", label: "Volume up" }, { value: "Key,Volume Down", label: "Volume down" },
        { value: "Key,Media Play", label: "Play / pause" }, { value: "Key,Print", label: "Screenshot" },
        { value: "Disabled", label: "Nothing (the button does nothing)" }]
    function describe(v) {
        const a = actions.find(x => x.value === v);
        if (a) return a.label;
        if (v.startsWith("Key,")) return "The key " + v.slice(4);
        if (v.startsWith("MouseButton,")) return "Mouse button " + v.slice(12);
        return v;
    }
    function optionsFor(cur) {
        let o = actions.slice();
        if (cur && !o.find(a => a.value === cur)) o.push({ value: cur, label: describe(cur) });
        return o.concat([{ value: "@key", label: "A key you press…" }]);
    }
    function setAction(b, v) {
        if (v === "@key") { page.keyFor = b; page.pressing = false; keyBox.forceActiveFocus(); return; }
        page.say("");
        kw.job.run(["mouse-rebind", b], () => {
            page.added = page.added.filter(x => x !== b);
            kw.refresh();
            page.say(label(b) + ": " + (v ? describe(v) : "as it is") + ".");
        }, e => page.say(e, true), v || " ", "save");
    }
    function caught(btn) {
        const b = nameOf(btn);
        if (!b) { page.say("That's the " + (btn === Qt.RightButton ? "right" : "left") + " button: it always stays as it is. Press another one.", true); return; }
        page.pressing = false;
        if (page.added.indexOf(b) < 0 && page.buttons.indexOf(b) < 0) page.added = page.added.concat([b]);
        page.say(label(b) + " found: pick what it does below.");
    }

    // ---- the key box: the next key or key combination pressed is what the button sends
    readonly property var keyNames: ({
        [Qt.Key_Left]: "Left", [Qt.Key_Right]: "Right", [Qt.Key_Up]: "Up", [Qt.Key_Down]: "Down", [Qt.Key_Tab]: "Tab",
        [Qt.Key_Backtab]: "Tab", [Qt.Key_Return]: "Return", [Qt.Key_Enter]: "Enter", [Qt.Key_Space]: "Space",
        [Qt.Key_Backspace]: "Backspace", [Qt.Key_Delete]: "Del", [Qt.Key_Insert]: "Ins", [Qt.Key_Home]: "Home",
        [Qt.Key_End]: "End", [Qt.Key_PageUp]: "PgUp", [Qt.Key_PageDown]: "PgDown", [Qt.Key_Print]: "Print",
        [Qt.Key_VolumeUp]: "Volume Up", [Qt.Key_VolumeDown]: "Volume Down", [Qt.Key_VolumeMute]: "Volume Mute",
        [Qt.Key_MediaPlay]: "Media Play", [Qt.Key_MediaTogglePlayPause]: "Media Play", [Qt.Key_MediaNext]: "Media Next",
        [Qt.Key_MediaPrevious]: "Media Previous", [Qt.Key_Minus]: "-", [Qt.Key_Period]: "." })
    function keyText(e) {
        let k = "";
        if (e.key >= Qt.Key_A && e.key <= Qt.Key_Z) k = String.fromCharCode(e.key);
        else if (e.key >= Qt.Key_0 && e.key <= Qt.Key_9) k = String.fromCharCode(e.key);
        else if (e.key >= Qt.Key_F1 && e.key <= Qt.Key_F24) k = "F" + (e.key - Qt.Key_F1 + 1);
        else k = keyNames[e.key] || "";
        if (!k) return "";
        const m = [];
        if (e.modifiers & Qt.ControlModifier) m.push("Ctrl");
        if (e.modifiers & Qt.AltModifier) m.push("Alt");
        if ((e.modifiers & Qt.ShiftModifier) || e.key === Qt.Key_Backtab) m.push("Shift");
        if (e.modifiers & Qt.MetaModifier) m.push("Meta");
        return m.concat([k]).join("+");
    }

    function openAnchor() {
        if (SettingsNav.pageAnchor === "press") { SettingsNav.pageAnchor = ""; page.pressing = true; }
    }
    Component.onCompleted: openAnchor()
    Connections { target: SettingsNav; function onPageAnchorChanged() { page.openAnchor(); } }

    KwinGate {
        kw: kw
        what: "Mouse button settings"

        SetHeader { text: "Your mouse's buttons" + (kw.job.what === "save" ? " · saving…" : "") }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4; Layout.bottomMargin: 6
            text: "Give the side buttons and any extra buttons of your mouse another job: a different click, a key or a " +
                  "shortcut. The left and right buttons always stay as they are. It works at once, in every app."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
        Repeater {
            model: page.buttons
            delegate: SetDropdown {
                required property string modelData
                icon: "mouse"; title: page.label(modelData)
                sub: page.actionOf(modelData) ? "Now: " + page.describe(page.actionOf(modelData)) : "Does what the mouse's maker set"
                options: page.optionsFor(page.actionOf(modelData))
                current: page.actionOf(modelData)
                onChosen: v => page.setAction(modelData, v)
            }
        }
        SetRow {
            icon: "plus"; title: page.pressing ? "Cancel" : "Change another button"
            sub: page.pressing ? "Close the box without adding a button" : "Press it in the box that opens, so Hearth knows which one it is"
            onActivated: { page.pressing = !page.pressing; page.keyFor = ""; page.say(""); }
        }

        // ---- "press the button" box
        Rectangle {
            visible: page.pressing
            Layout.fillWidth: true; Layout.preferredHeight: 180; Layout.topMargin: 6
            radius: SetStyle.bigRadius
            color: press.containsMouse ? SetStyle.cardHover : SetStyle.card
            border.width: 2; border.color: Theme.accent
            ColumnLayout {
                anchors.centerIn: parent
                spacing: 8
                Icon { Layout.alignment: Qt.AlignHCenter; name: "mouse"; size: 44; color: Theme.accent }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "Point here and press the mouse button you want to change"
                    color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "The middle button (press the wheel), back, forward or any other extra button"
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                }
            }
            MouseArea {
                id: press
                anchors.fill: parent
                hoverEnabled: true
                acceptedButtons: Qt.AllButtons
                onPressed: m => { m.accepted = true; page.caught(m.button); }
            }
        }

        // ---- the key box
        Rectangle {
            visible: page.keyFor !== ""
            Layout.fillWidth: true; Layout.preferredHeight: 150; Layout.topMargin: 6
            radius: SetStyle.bigRadius
            color: SetStyle.card
            border.width: 2; border.color: keyBox.activeFocus ? Theme.accent : Theme.line
            Item {
                id: keyBox
                anchors.fill: parent
                focus: page.keyFor !== ""
                Keys.onPressed: e => {
                    e.accepted = true;
                    if (e.key === Qt.Key_Escape) { page.keyFor = ""; page.say(""); return; }
                    const t = page.keyText(e);
                    if (!t) return;                 // a modifier on its own: wait for the rest
                    const b = page.keyFor;
                    page.keyFor = "";
                    page.setAction(b, "Key," + t);
                }
            }
            MouseArea { anchors.fill: parent; onClicked: keyBox.forceActiveFocus() }
            ColumnLayout {
                anchors.centerIn: parent
                spacing: 8
                Icon { Layout.alignment: Qt.AlignHCenter; name: "kbd"; size: 40; color: Theme.accent }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "Press the key or key combination for " + page.label(page.keyFor).replace(/ \(.*\)$/, "").toLowerCase()
                    color: Theme.text; font.family: Theme.font; font.pixelSize: 17; font.weight: Font.DemiBold
                }
                Text {
                    Layout.alignment: Qt.AlignHCenter
                    text: "Hold Ctrl, Alt, Shift or Super with it for a shortcut · Esc cancels"
                    color: Theme.muted; font.family: Theme.font; font.pixelSize: 13
                }
            }
        }

        Text {
            visible: page.note !== ""
            Layout.fillWidth: true; Layout.leftMargin: 4; Layout.topMargin: 4
            text: page.note; color: page.noteBad ? Theme.danger : Theme.flame
            font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
        }
        Text {
            Layout.fillWidth: true; Layout.leftMargin: 4; Layout.topMargin: 8
            text: "A button already changed here is caught as what it now does: set it back to \"As it is\" first to change it again. " +
                  "Pen buttons are on Drawing tablet & touchscreen."
            color: Theme.muted; font.family: Theme.font; font.pixelSize: 12; wrapMode: Text.WordWrap
        }
    }
}
