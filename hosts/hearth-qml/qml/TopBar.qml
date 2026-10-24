// The top of Game Mode: the menu, the page's name, the Experimental mark, search, Quick Access, the clock, the battery.
import QtQuick

Rectangle {
    id: bar
    property string title: ""
    height: Theme.u(52)
    gradient: Gradient {
        GradientStop { position: 0; color: Theme.bg2 }
        GradientStop { position: 1; color: Theme.alpha(Theme.bg2, 0.6) }
    }
    Rectangle { anchors { left: parent.left; right: parent.right; bottom: parent.bottom } height: 1; color: Theme.alpha("#ffffff", 0.03) }

    Row {
        anchors { left: parent.left; leftMargin: Theme.gut - Theme.u(8); verticalCenter: parent.verticalCenter }
        spacing: Theme.u(8)
        Btn { kind: "icon"; icon: "layout"; navKey: "top-menu"; navScope: "view"; onActivated: App.openLayer("menu"); width: Theme.u(38); height: Theme.u(38) }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: bar.title
            color: Theme.text
            font.family: Theme.font
            font.pixelSize: Theme.u(16)
            font.weight: Font.DemiBold
        }
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            radius: Theme.u(6)
            color: Theme.alpha(Theme.warn, 0.16)
            border.color: Theme.alpha(Theme.warn, 0.5)
            width: exp.implicitWidth + Theme.u(14)
            height: exp.implicitHeight + Theme.u(6)
            Text {
                id: exp
                anchors.centerIn: parent
                text: "Qt renderer · Experimental"
                color: Theme.warn
                font.family: Theme.font
                font.pixelSize: Theme.u(11)
                font.weight: Font.Bold
            }
        }
    }
    Row {
        anchors { right: parent.right; rightMargin: Theme.gut - Theme.u(8); verticalCenter: parent.verticalCenter }
        spacing: Theme.u(6)
        Btn { kind: "icon"; icon: "search"; navKey: "top-search"; onActivated: App.openLayer("search"); width: Theme.u(38); height: Theme.u(38) }
        Btn { kind: "icon"; icon: "sliders"; navKey: "top-qam"; onActivated: App.openLayer("qam"); width: Theme.u(38); height: Theme.u(38) }
        Row {
            anchors.verticalCenter: parent.verticalCenter
            spacing: Theme.u(5)
            visible: !!(App.sys && App.sys.battery && App.sys.battery.present)
            Icon { name: "bolt"; color: Theme.dim; size: Theme.u(16); anchors.verticalCenter: parent.verticalCenter; visible: !!(App.sys && App.sys.battery && App.sys.battery.charging) }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: App.sys && App.sys.battery ? App.sys.battery.pct + "%" : ""
                color: Theme.dim
                font.family: Theme.font
                font.pixelSize: Theme.u(13)
            }
        }
        Text {
            id: clock
            anchors.verticalCenter: parent.verticalCenter
            leftPadding: Theme.u(8)
            color: Theme.text
            font.family: Theme.font
            font.pixelSize: Theme.u(15)
            font.weight: Font.DemiBold
            text: Qt.formatTime(new Date(), Qt.locale().timeFormat(Locale.ShortFormat))
            Timer { interval: 10000; running: true; repeat: true; onTriggered: clock.text = Qt.formatTime(new Date(), Qt.locale().timeFormat(Locale.ShortFormat)) }
        }
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: Theme.u(30); height: width; radius: Theme.u(8)
            color: Theme.accent
            Text {
                anchors.centerIn: parent
                text: bar.initials(App.profile.name)
                color: Theme.accentInk
                font.family: Theme.font
                font.pixelSize: Theme.u(12)
                font.weight: Font.Bold
            }
        }
    }
    function initials(n) {
        const p = String(n || "?").trim().split(/\s+/).filter(x => x);
        return ((p[0] || "?")[0] + (p.length > 1 ? p[p.length - 1][0] : "")).toUpperCase();
    }
}
