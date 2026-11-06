// A titled group of setting cards ("Sharing", "Keep it safe"): a heading, then the cards close together.
import QtQuick
import QtQuick.Layouts

ColumnLayout {
    id: g
    property string title
    default property alias cards: inner.data
    Layout.fillWidth: true
    spacing: 0
    SetHeader { visible: g.title !== ""; text: g.title; Layout.topMargin: 14 }
    ColumnLayout { id: inner; Layout.fillWidth: true; spacing: 3 }
}
