// Calculator widget (1.14): type a sum or press the keys; the answer shows as you type (widget-tools.js calc: brackets,
// %, powers, sqrt, sin…, never eval). Enter keeps it in the tape above; Copy puts the answer on the clipboard.
// Degrees or radians and the tape are kept in its settings (widgets.json); nothing is saved while you only type.
import QtQuick
import QtQuick.Layouts
import Quickshell
import "widget-tools.js" as T

WidgetBase {
    id: w
    implicitWidth: 300
    implicitHeight: 380
    readonly property bool deg: c("degrees", true) === true
    readonly property var tape: (list("tape") || []).slice(-4)
    readonly property var res: T.calc(field.text, deg)
    readonly property string answer: res.value === null || res.value === undefined ? "" : T.fmt(res.value)
    function press(k) {
        if (k === "C") { field.text = ""; return; }
        if (k === "⌫") { field.text = field.text.slice(0, -1); return; }
        if (k === "=") { enter(); return; }
        field.text += k;
        field.input.forceActiveFocus();
    }
    function enter() {
        if (answer === "") return;
        set("tape", (list("tape") || []).concat([field.text + " = " + answer]).slice(-12));
        field.text = answer;
    }
    function copy(t) { if (t) { Quickshell.execDetached(["wl-copy", "--", t]); SettingsNav.toast("Copied " + t); } }

    ColumnLayout {
        anchors { fill: parent; margins: 14 * w.k }
        spacing: 6 * w.k
        RowLayout {
            Layout.fillWidth: true
            Icon { name: "plus"; size: 15; color: Theme.muted }
            Text { Layout.fillWidth: true; text: "Calculator"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
            WBtn { text: w.deg ? "DEG" : "RAD"; flat: true; size: 22; onClicked: w.set("degrees", !w.deg) }
        }
        Repeater {
            model: w.tape
            delegate: Text {
                required property string modelData
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignRight
                text: modelData; elide: Text.ElideLeft
                color: Theme.muted; font.family: Theme.font; font.pixelSize: Math.round(12 * w.k)
                MouseArea { anchors.fill: parent; onClicked: field.text = parent.modelData.split(" = ").pop() }
            }
        }
        WField {
            id: field
            Layout.fillWidth: true
            placeholder: "Type a sum: 12*(3+4)"
            onAccepted: w.enter()
        }
        RowLayout {
            Layout.fillWidth: true
            Text {
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignRight
                text: w.res.error ? w.res.error : w.answer !== "" ? "= " + w.answer : " "
                color: w.res.error ? Theme.muted : Theme.text
                font.family: Theme.font; font.pixelSize: w.res.error ? 13 : Math.round(26 * w.k); font.weight: Font.Bold
                elide: Text.ElideLeft
            }
            WBtn { visible: w.answer !== ""; icon: "copy"; flat: true; size: 28; onClicked: w.copy(w.answer) }
        }
        GridLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            columns: 4
            rowSpacing: 5; columnSpacing: 5
            Repeater {
                model: ["C", "(", ")", "÷", "7", "8", "9", "×", "4", "5", "6", "−", "1", "2", "3", "+", "0", ".", "⌫", "="]
                delegate: WBtn {
                    required property string modelData
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    size: 30 * w.k
                    text: modelData
                    on: modelData === "="
                    onClicked: w.press(modelData === "÷" ? "÷" : modelData === "×" ? "×" : modelData === "−" ? "-" : modelData)
                }
            }
        }
    }
}
