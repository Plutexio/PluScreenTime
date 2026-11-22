import QtQuick
import PluScreenTime

// One figure on a card of its own: its symbol on a shape, what it is, the
// value drawn round, and a note under it.
Rectangle {
    id: root

    property string label
    property string value
    property string note
    property string symbol
    property string shape: "cookie9"

    implicitHeight: 132
    radius: Appearance.rounding.large
    color: Appearance.colors.layer2

    HoverHandler {
        id: hover
    }

    ShapeIcon {
        id: badge
        x: 18
        y: 16
        size: 34
        iconSize: 18
        shape: root.shape
        symbol: root.symbol
        hovered: hover.hovered
    }

    StyledText {
        anchors.left: badge.right
        anchors.leftMargin: 10
        anchors.right: parent.right
        anchors.rightMargin: 16
        anchors.verticalCenter: badge.verticalCenter
        text: root.label
        color: Appearance.colors.onSurfaceVariant
        font.pixelSize: Appearance.fontSize.smallie
        font.weight: Font.DemiBold
        elide: Text.ElideRight
    }

    Column {
        anchors.left: parent.left
        anchors.leftMargin: 18
        anchors.right: parent.right
        anchors.rightMargin: 16
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 16
        spacing: 2

        TitleText {
            width: parent.width
            text: root.value
            font.pixelSize: Appearance.fontSize.huge
            elide: Text.ElideRight
            animateChange: true
        }
        StyledText {
            width: parent.width
            visible: root.note.length > 0
            text: root.note
            color: Appearance.colors.onSurfaceVariant
            font.pixelSize: Appearance.fontSize.smaller
            elide: Text.ElideRight
        }
    }
}
