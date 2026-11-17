// News widget: the newest headlines from any RSS or Atom feed (a news site, a blog, a subreddit's .rss, a YouTube
// channel's feed). Click a headline to read it in your browser. Looked at every 15 minutes while on screen.
// Settings: the feed (a few to start from, or paste any address).
import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io

WidgetBase {
    id: w
    readonly property string url: c("url", "")
    readonly property var picks: [
        { n: "BBC News", u: "https://feeds.bbci.co.uk/news/rss.xml" }, { n: "Hacker News", u: "https://hnrss.org/frontpage" },
        { n: "GamingOnLinux", u: "https://www.gamingonlinux.com/article_rss.php" }, { n: "Phoronix", u: "https://www.phoronix.com/rss.php" },
        { n: "The Verge", u: "https://www.theverge.com/rss/index.xml" }]
    property var data: ({ items: [] })
    property bool busy: false
    implicitWidth: 400; implicitHeight: 360

    Process {
        id: get
        command: ["python3", w.helper, "feed", w.url]
        running: w.live && w.url !== ""
        onRunningChanged: w.busy = running
        stdout: StdioCollector { onStreamFinished: { try { w.data = JSON.parse(text.trim().split("\n").pop()); } catch (e) { w.data = { items: [], error: "Couldn't read that feed" }; } } }
    }
    onUrlChanged: { data = { items: [] }; if (url) get.running = true; }
    Timer { interval: 900000; running: w.live && w.url !== ""; repeat: true; onTriggered: if (!get.running) get.running = true }
    function ago(s) {
        const t = Date.parse(s);
        if (isNaN(t)) return "";
        const m = Math.round((Date.now() - t) / 60000);
        return m < 1 ? "now" : m < 60 ? m + " min ago" : m < 1440 ? Math.round(m / 60) + " h ago" : Math.round(m / 1440) + " d ago";
    }

    ColumnLayout {
        visible: !w.configuring && w.url !== ""
        anchors { fill: parent; margins: 14 }
        spacing: 4
        RowLayout {
            Layout.fillWidth: true
            Icon { name: "globe"; size: 18; color: Theme.accent }
            Text { Layout.fillWidth: true; text: w.data.title || "News"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold; elide: Text.ElideRight }
            Text { visible: w.busy; text: "…"; color: Theme.muted; font.family: Theme.font; font.pixelSize: 14 }
        }
        Text { visible: !!w.data.error; Layout.fillWidth: true; wrapMode: Text.WordWrap; text: w.data.error || ""; color: Theme.muted; font.family: Theme.font; font.pixelSize: 13 }
        ListView {
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 2
            model: w.data.items || []
            delegate: Rectangle {
                required property var modelData
                width: ListView.view.width
                height: col.implicitHeight + 10
                radius: Theme.r(6)
                color: nm.containsMouse ? Qt.rgba(Theme.text.r, Theme.text.g, Theme.text.b, 0.1) : "transparent"
                ColumnLayout {
                    id: col
                    anchors { left: parent.left; right: parent.right; verticalCenter: parent.verticalCenter; leftMargin: 6; rightMargin: 6 }
                    spacing: 1
                    Text { Layout.fillWidth: true; text: modelData.title; color: Theme.text; font.family: Theme.font; font.pixelSize: 14; wrapMode: Text.WordWrap; maximumLineCount: 2; elide: Text.ElideRight }
                    Text { Layout.fillWidth: true; visible: text !== ""; text: w.ago(modelData.when); color: Theme.muted; font.family: Theme.font; font.pixelSize: 11 }
                }
                MouseArea { id: nm; anchors.fill: parent; hoverEnabled: true; cursorShape: Qt.PointingHandCursor; onClicked: w.open(modelData.link) }
            }
        }
    }
    ColumnLayout {
        visible: w.configuring || w.url === ""
        anchors { fill: parent; margins: 14 }
        spacing: 8
        Text { text: "Pick a news feed"; color: Theme.text; font.family: Theme.font; font.pixelSize: 16; font.weight: Font.DemiBold }
        Flow {
            Layout.fillWidth: true
            spacing: 6
            Repeater {
                model: w.picks
                delegate: WBtn { required property var modelData; text: modelData.n; on: w.url === modelData.u; onClicked: { w.set("url", modelData.u); w.doneConfiguring(); } }
            }
        }
        WField { Layout.fillWidth: true; placeholder: "Or paste a feed's address (https://…)"; onAccepted: if (/^https?:\/\//.test(text.trim())) { w.set("url", text.trim()); text = ""; w.doneConfiguring(); } }
        Item { Layout.fillHeight: true }
    }
}
