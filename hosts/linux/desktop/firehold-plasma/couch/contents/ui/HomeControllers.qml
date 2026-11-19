// Controllers (mockup 01): each controller and headset with its battery; a low one in the warning colour with
// "charge soon". The kernel's and UPower's batteries (hearth-firehold-plasma home). Hidden when none report one.
import QtQuick
import "fhp.js" as H

Card {
    id: c
    property QtObject couch
    k: couch.k
    readonly property var list: (couch.cards.controllers || []).filter(function (d) { return d.kind === "pad" || d.kind === "headset"; })
    visible: list.length > 0
    height: col.implicitHeight + 28 * k
    Column {
        id: col
        x: 20 * c.k; y: 14 * c.k
        width: parent.width - 40 * c.k
        spacing: 10 * c.k
        T { couchRef: c.couch; text: "Controllers"; font.pixelSize: 18 * c.k; font.weight: Font.Bold }
        Flow {
            width: parent.width
            spacing: 10 * c.k
            Repeater {
                model: c.list
                delegate: Pill {
                    required property var modelData
                    couch: c.couch
                    fontSize: 14
                    padX: 14
                    text: H.deviceLabel(modelData)
                    textColor: modelData.low ? H.P.warn : H.P.tx
                    Glyph { name: modelData.kind === "headset" ? "headset" : "pad"; size: 15 * c.k; color: modelData.low ? H.P.warn : "#d8d1ca" }
                }
            }
        }
    }
}
