import QtQuick
import PluScreenTime

// A change of size along Material's emphasized curve.
NumberAnimation {
    duration: Appearance.motion(300)
    easing.type: Easing.BezierSpline
    easing.bezierCurve: Appearance.curves.emphasized
}
