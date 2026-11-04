// The blur for Glass's faux frost (Frost.qml). Its own file so a system without QtQuick.Effects only loses the blur.
import QtQuick
import QtQuick.Effects

MultiEffect {
    blurEnabled: true
    blur: 1.0
    blurMax: 48
    saturation: 0.15
}
