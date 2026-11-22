import QtQuick
import PluScreenTime

// Radii, widths and other quick changes, with no overshoot.
NumberAnimation {
    duration: Appearance.motion(200)
    easing.type: Easing.BezierSpline
    easing.bezierCurve: Appearance.curves.expressiveEffects
}
