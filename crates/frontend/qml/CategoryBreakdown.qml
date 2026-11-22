import QtQuick
import PluScreenTime

// Where the time went by category: one share bar across, the categories in
// their fixed order on it, and a row for each with its time and percent.
Column {
    id: root

    /// `Summary.categories`: [{ id, total }], most used first.
    property var categories: []
    property real total: 0

    spacing: 12

    // The shares on one bar, 2 px of card between them.
    Item {
        width: parent.width
        height: 14
        visible: root.total > 0

        Row {
            anchors.fill: parent
            spacing: 2

            Repeater {
                model: root.categories.slice().sort((a, b) => Categories.order(a.id) - Categories.order(b.id))

                Rectangle {
                    required property int index
                    required property var modelData
                    readonly property int count: root.categories.length

                    width: Math.max(2, (parent.width - 2 * (count - 1)) * modelData.total / Math.max(1, root.total))
                    height: parent.height
                    color: Categories.color(modelData.id)
                    topLeftRadius: index === 0 ? 7 : 0
                    bottomLeftRadius: topLeftRadius
                    topRightRadius: index === count - 1 ? 7 : 0
                    bottomRightRadius: topRightRadius
                }
            }
        }
    }

    Repeater {
        model: root.categories

        Item {
            required property var modelData
            readonly property var info: Categories.info(modelData.id)

            width: root.width
            height: 30

            ShapeIcon {
                id: badge
                anchors.verticalCenter: parent.verticalCenter
                size: 28
                iconSize: 15
                shape: info.shape
                symbol: info.symbol
                color: Appearance.colors.tonal(Categories.color(modelData.id))
                iconColor: Categories.color(modelData.id)
            }
            StyledText {
                anchors.left: badge.right
                anchors.leftMargin: 10
                anchors.right: percent.left
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                text: info.name
                font.pixelSize: Appearance.fontSize.smallie
                elide: Text.ElideRight
            }
            StyledText {
                id: percent
                anchors.right: time.left
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                width: 40
                horizontalAlignment: Text.AlignRight
                text: Format.percent(modelData.total, root.total)
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smaller
                tabularNumbers: true
            }
            StyledText {
                id: time
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                width: 86
                horizontalAlignment: Text.AlignRight
                text: Format.duration(modelData.total)
                font.pixelSize: Appearance.fontSize.smallie
                font.weight: Font.DemiBold
                tabularNumbers: true
            }
        }
    }
}
