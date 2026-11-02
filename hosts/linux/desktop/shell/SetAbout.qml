// Hearth Settings › About: what this device is.
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

SetPage {
    id: page
    title: "About"
    property var info: ({})
    Process {
        running: true
        command: ["sh", "-c", [
            ". /etc/os-release 2>/dev/null; echo \"os=${PRETTY_NAME:-Linux}\"; echo \"ver=${IMAGE_VERSION:-$VERSION_ID}\"",
            // Hearth OS's own version and its release date (the image writes both; older images have no date).
            "echo \"hver=$(cat /usr/share/nimbus/version 2>/dev/null)\"; echo \"hdate=$(cat /usr/share/nimbus/version-date 2>/dev/null)\"",
            "echo \"host=$(cat /etc/hostname 2>/dev/null || hostname)\"",
            "echo \"model=$(cat /sys/devices/virtual/dmi/id/product_name 2>/dev/null)\"",
            "echo \"vendor=$(cat /sys/devices/virtual/dmi/id/sys_vendor 2>/dev/null)\"",
            "echo \"cpu=$(sed -n 's/^model name[[:space:]]*: //p' /proc/cpuinfo | head -1)\"",
            "echo \"cores=$(nproc 2>/dev/null)\"",
            "echo \"mem=$(awk '/MemTotal/ {printf \"%.0f GB\", $2/1048576}' /proc/meminfo)\"",
            "echo \"gpu=$(lspci 2>/dev/null | sed -n 's/.*\\(VGA\\|3D\\|Display\\)[^:]*: //p' | head -1)\"",
            "echo \"disk=$(df -h --output=avail,size /var/home 2>/dev/null | tail -1 | awk '{print $1\" free of \"$2}')\"",
            "echo \"kernel=$(uname -r)\"",
            "echo \"shell=$(quickshell --version 2>/dev/null | head -1)\"",
            "echo \"user=$(id -un)\""
        ].join("; ")]
        stdout: StdioCollector {
            onStreamFinished: {
                const o = {};
                for (const l of text.split("\n")) { const i = l.indexOf("="); if (i > 0) o[l.slice(0, i)] = l.slice(i + 1).trim(); }
                page.info = o;
            }
        }
    }
    // Hearth OS version numbers as people see them (docs/UPDATES.md, "Version numbers"): from 1.0 on MAJOR.MINOR, so
    // 1.0.0 shows as "1.0" (0.x stays whole), with the release date next to it: "1.0 · November 2, 2026".
    function shownVersion(v) {
        const m = /^(\d+)\.(\d+)\.0$/.exec(v || "");
        return m && Number(m[1]) >= 1 ? m[1] + "." + m[2] : (v || "");
    }
    function shownDate(d) {
        const m = /^(\d{4})-(\d\d)-(\d\d)$/.exec(d || "");
        return m ? new Date(Number(m[1]), Number(m[2]) - 1, Number(m[3])).toLocaleDateString(Qt.locale(), "MMMM d, yyyy") : "";
    }
    readonly property string hearthVersion: {
        if (!info.hver || info.hver === "0.0.0") return "";
        const d = shownDate(info.hdate);
        return shownVersion(info.hver) + (d ? " · " + d : "");
    }
    readonly property var facts: [
        ["Device", [info.vendor, info.model].filter(Boolean).join(" ")],
        ["Name", info.host], ["Signed in as", info.user],
        ["Processor", info.cpu ? info.cpu + (info.cores ? " (" + info.cores + " threads)" : "") : ""],
        ["Graphics", info.gpu], ["Memory", info.mem], ["Storage", info.disk],
        ["System", info.os], ["Version", hearthVersion || info.ver], ["Kernel", info.kernel], ["Desktop", info.shell ? "Mantel · " + info.shell : ""]
    ].filter(r => r[1])

    RowLayout {
        Layout.fillWidth: true; Layout.bottomMargin: 10
        spacing: 16
        Image { source: Qt.resolvedUrl("images/hearth-logo.png"); Layout.preferredWidth: 64; Layout.preferredHeight: 64; fillMode: Image.PreserveAspectFit; smooth: true }
        ColumnLayout {
            spacing: 2
            Text { text: "Hearth"; color: Theme.text; font.family: Theme.font; font.pixelSize: 22; font.weight: Font.DemiBold }
            Text { text: page.hearthVersion ? "Hearth OS " + page.hearthVersion : page.info.ver ? "Version " + page.info.ver : ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        }
    }
    Repeater {
        model: page.facts
        delegate: SetRow {
            required property var modelData
            interactive: false; chevron: false
            implicitHeight: 52
            title: modelData[0]; value: modelData[1]
        }
    }
}
