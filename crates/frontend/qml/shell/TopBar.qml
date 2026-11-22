import QtQuick
import PluScreenTime

// The content sheet's top bar. It has no background: the page scrolls up
// under it, and every button stands on a shape of its own. The page's title
// on the left with whatever belongs beside it, its actions and the window's
// controls on the right. Its empty stretches move the window, and a double
// click maximises it.
Item {
    id: root

    required property Window window
    property string title
    /// Beside the title (the overview's arrows).
    property alias leading: leadingRow.data
    /// Before the window's controls.
    default property alias actions: actionRow.data

    DragHandler {
        target: null
        onActiveChanged: if (active) root.window.startSystemMove()
    }
    TapHandler {
        onDoubleTapped: root.window.visibility === Window.Maximized ? root.window.showNormal() : root.window.showMaximized()
    }

    Row {
        id: heading

        x: Appearance.sizes.pageMargin
        anchors.verticalCenter: parent.verticalCenter
        spacing: 14

        TitleText {
            id: pageTitle
            anchors.verticalCenter: parent.verticalCenter
            width: Math.min(implicitWidth, Math.max(0, actionRow.x - heading.x - leadingRow.width - 60))
            level: "headline"
            text: root.title
            elide: Text.ElideRight
            animateChange: true
        }
        Row {
            id: leadingRow
            anchors.verticalCenter: parent.verticalCenter
            spacing: 4
        }
    }

    Row {
        id: actionRow

        anchors.right: controls.left
        anchors.rightMargin: 16
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8
    }

    WindowControls {
        id: controls
        anchors.right: parent.right
        anchors.rightMargin: 14
        anchors.verticalCenter: parent.verticalCenter
        window: root.window
    }
}
