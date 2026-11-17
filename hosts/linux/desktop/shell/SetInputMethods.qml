// Hearth Settings › Region & language › Chinese, Japanese & Korean typing (1.14, KDE's input methods with fcitx5, in
// Hearth's look). Optional and off unless you turn it on: fcitx5 isn't part of Hearth OS, so Get it downloads it from
// Flathub for you (with Mozc for Japanese, Pinyin and others for Chinese, Hangul for Korean). Turning it on makes fcitx5
// KWin's input method (your ~/.config/kwinrc, the one key) in place of Hearth's on-screen keyboard; turning it off takes
// that key out again, so the on-screen keyboard is back. Nothing else changes, and it's used from the next sign-in.
import QtQuick
import QtQuick.Layouts
import Quickshell

SetPage {
    id: page
    title: "Chinese, Japanese & Korean typing"
    property var st: null
    property string message: ""
    property bool messageBad: false
    function say(t, bad) { message = t; messageBad = !!bad; }
    SetJob { id: job; helper: "hearth-region" }
    Component.onCompleted: job.run(["im"], r => page.st = r, e => page.say(e, true))
    readonly property bool kwin: Quickshell.env("HEARTH_WM") === "kwin"

    SetBanner {
        visible: !page.kwin
        Layout.fillWidth: true
        kind: "info"; icon: "info"; title: "Works with Mantel on KWin"
        sub: "fcitx5 types into apps through KWin. Settings › Your desktop picks the window manager"
    }
    SetCardGroup {
        title: "Input methods (fcitx5)"
        SetRow {
            visible: !!page.st && !page.st.installed
            icon: "update"; chevron: false
            title: job.what === "install" ? "Downloading… " + (job.progress >= 0 ? Math.round(job.progress * 100) + "%" : "") : "Get Chinese, Japanese and Korean typing"
            sub: "fcitx5 from Flathub, with Mozc (Japanese), Pinyin (Chinese) and Hangul (Korean). About 150 MB, for you only"
            onActivated: if (!job.busy) { page.say(""); job.run(["im-install"], r => { page.st = r; page.say("Installed. Turn it on below."); }, e => page.say(e, true), "", "install"); }
        }
        SetSwitch {
            visible: !!page.st && page.st.installed
            icon: "kbd"; title: "Type with fcitx5"
            sub: "Switch languages with Ctrl+Space. Used from the next sign-in"
            checked: !!page.st && page.st.on
            enabled: page.kwin || (!!page.st && page.st.on)
            onToggled: v => job.run(["im-set", v ? "on" : "off"], r => { page.st = r; page.say(v ? "On from the next sign-in. Hearth's on-screen keyboard is off while fcitx5 is on." : "Off from the next sign-in: Hearth's on-screen keyboard is back."); }, e => page.say(e, true))
        }
        SetBanner {
            visible: !!page.st && page.st.on
            Layout.fillWidth: true
            kind: "warn"; title: "Hearth's on-screen keyboard is off"
            sub: "KWin takes one input method at a time. Turn fcitx5 off here to have the on-screen keyboard back"
        }
    }
    SetCardGroup {
        title: "What you get"
        RowLayout {
            Layout.fillWidth: true; spacing: 10
            Repeater {
                model: [{ g: "日本語", t: "Japanese", s: "Mozc: kana and kanji" }, { g: "中文", t: "Chinese", s: "Pinyin, Shuangpin, Wubi" }, { g: "한국어", t: "Korean", s: "Hangul" }]
                delegate: SetPanel {
                    required property var modelData
                    implicitHeight: 110
                    Text { x: 18; y: 16; text: modelData.g; color: Theme.accent; font.pixelSize: 30 }
                    Text { x: 18; y: 62; text: modelData.t; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; font.weight: Font.DemiBold }
                    Text { x: 18; y: 82; text: modelData.s; color: Theme.muted; font.family: Theme.font; font.pixelSize: 12 }
                }
            }
        }
    }
    Text {
        visible: page.message !== ""
        Layout.fillWidth: true; Layout.leftMargin: 4
        text: page.message; color: page.messageBad ? Theme.danger : Theme.flame
        font.family: Theme.font; font.pixelSize: 13; wrapMode: Text.WordWrap
    }
}
