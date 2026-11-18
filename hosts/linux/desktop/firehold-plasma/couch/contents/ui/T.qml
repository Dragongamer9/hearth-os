// Firehold's text: its font, its colour, no wrapping unless asked.
import QtQuick

Text {
    property QtObject couchRef: null
    color: "#f6ede6"
    font.family: couchRef ? couchRef.fontName : "Noto Sans"
    elide: Text.ElideRight
    textFormat: Text.PlainText
    renderType: Text.NativeRendering
}
