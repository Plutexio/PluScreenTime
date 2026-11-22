import QtQuick
import PluScreenTime

// An app in a list: its icon, its name over a bar of its share in its
// category's colour, and its time. Picking it opens it in the inspector.
RippleButton {
    id: root

    property var app: null
    property real total: 0
    /// The longest bar's time; bars are drawn against it.
    property real scale: 1
    property string note
    property bool selected: false

    implicitHeight: 56
    buttonRadius: Appearance.rounding.normal
    colBackground: selected ? Appearance.colors.secondaryContainer : "transparent"
    colBackgroundHover: selected ? Appearance.colors.secondaryContainerHover : Appearance.colors.layer2Hover
    colRipple: Appearance.colors.layer2Active
    bounce: false
    leftPadding: 8
    rightPadding: 12

    contentItem: Item {
        AppIcon {
            id: icon
            anchors.verticalCenter: parent.verticalCenter
            size: 36
            source: root.app && root.app.icon ? "file://" + root.app.icon : ""
            name: root.app ? root.app.name : ""
        }

        StyledText {
            id: time
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            text: Format.duration(root.total)
            font.pixelSize: Appearance.fontSize.smallie
            font.weight: Font.DemiBold
            tabularNumbers: true
        }

        Column {
            anchors.left: icon.right
            anchors.leftMargin: 12
            anchors.right: time.left
            anchors.rightMargin: 14
            anchors.verticalCenter: parent.verticalCenter
            spacing: 6

            Row {
                width: parent.width
                spacing: 8

                StyledText {
                    id: name
                    width: Math.min(implicitWidth, parent.width - (noteText.visible ? noteText.implicitWidth + 8 : 0))
                    text: root.app ? root.app.name : ""
                    font.pixelSize: Appearance.fontSize.smallie
                    elide: Text.ElideRight
                }
                StyledText {
                    id: noteText
                    anchors.baseline: name.baseline
                    visible: root.note.length > 0
                    text: root.note
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smallest
                }
            }

            Item {
                width: parent.width
                height: 6

                Rectangle {
                    width: parent.width
                    height: parent.height
                    radius: 3
                    color: Appearance.colors.layer3
                }
                Rectangle {
                    width: Math.max(height, parent.width * Math.min(1, root.total / Math.max(1, root.scale)))
                    height: parent.height
                    radius: 3
                    color: Categories.color(root.app ? root.app.category : "other")

                    Behavior on width {
                        ElementMove {}
                    }
                }
            }
        }
    }
}
