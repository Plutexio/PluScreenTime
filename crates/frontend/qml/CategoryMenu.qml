import QtQuick
import PluScreenTime

// Picking an app's category: each one on its shape in its colour, the
// current one ticked, and a way back to the one found for the app.
StyledPopover {
    id: root

    property string current
    /// The current one was picked by hand.
    property bool custom: false

    /// A category's ID, or "" for the one found.
    signal picked(string category)

    title: qsTr("Category")
    symbol: "category"
    iconShape: "pentagon"
    width: 260

    contentItem: Column {
        spacing: 2

        Repeater {
            model: Categories.list

            RippleButton {
                id: option

                required property var modelData
                readonly property bool chosen: modelData.id === root.current

                width: parent.width
                height: 40
                buttonRadius: Appearance.rounding.small
                colBackground: chosen ? Appearance.colors.secondaryContainer : "transparent"
                colBackgroundHover: Appearance.colors.layer2Hover
                bounce: false
                leftPadding: 6
                rightPadding: 10
                onClicked: {
                    root.close()
                    if (!chosen)
                        root.picked(modelData.id)
                }

                contentItem: Item {
                    ShapeIcon {
                        id: badge
                        anchors.verticalCenter: parent.verticalCenter
                        size: 28
                        iconSize: 15
                        shape: option.modelData.shape
                        symbol: option.modelData.symbol
                        color: Appearance.colors.tonal(Categories.color(option.modelData.id))
                        iconColor: Categories.color(option.modelData.id)
                        hovered: option.hovered
                    }
                    StyledText {
                        anchors.left: badge.right
                        anchors.leftMargin: 10
                        anchors.verticalCenter: parent.verticalCenter
                        text: option.modelData.name
                        font.pixelSize: Appearance.fontSize.smallie
                        font.weight: option.chosen ? Font.DemiBold : Font.Normal
                    }
                    MaterialSymbol {
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        visible: option.chosen
                        text: "check"
                        iconSize: 18
                        color: Appearance.colors.onSecondaryContainer
                    }
                }
            }
        }

        TextButton {
            width: parent.width
            visible: root.custom
            variant: TextButton.Text
            symbol: "restart_alt"
            text: qsTr("Use the One Found")
            onClicked: {
                root.close()
                root.picked("")
            }
        }
    }
}
