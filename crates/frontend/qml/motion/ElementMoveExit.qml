import QtQuick
import PluScreenTime

// Something leaving: quicker than it came, accelerating away.
NumberAnimation {
    duration: Appearance.motion(200)
    easing.type: Easing.BezierSpline
    easing.bezierCurve: Appearance.curves.emphasizedAccel
}
