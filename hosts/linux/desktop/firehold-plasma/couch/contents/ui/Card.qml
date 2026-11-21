// The mockups' .card: a dark, slightly see-through card with a faint line and 18 px corners.
import QtQuick

Rectangle {
    property real k: 1
    radius: 18 * k
    color: "#db1e1a18"
    border.width: 1
    border.color: "#12ffffff"
}
