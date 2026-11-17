// A plain card for a settings page's own drawing (a graph, a picture, a row of tiles): the cards' colour, corners
// and edge, with whatever is put inside it (1.14).
import QtQuick
import QtQuick.Layouts

Rectangle {
    Layout.fillWidth: true
    radius: SetStyle.bigRadius
    color: SetStyle.card
    border.color: SetStyle.tint(Theme.text, 0.06)
}
