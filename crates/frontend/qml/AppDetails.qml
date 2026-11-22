import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import PluScreenTime

// The inspector for the picked app, over the overview's span: its time and
// how it went, a chart of it alone, its category (to change), and the window
// titles it spent the time under.
Rectangle {
    id: root

    property real appId: -1
    property string span: "day"
    property real anchor: Date.now()
    property int revision: 0
    property var detail: null

    signal closeRequested()
    signal categoryChanged()

    function reload() {
        if (appId < 0) {
            detail = null
            return
        }
        const json = ScreenTime.appDetail(appId, JSON.stringify(Range.edges(span, anchor)))
        detail = json ? JSON.parse(json) : null
    }

    onAppIdChanged: reload()
    onSpanChanged: Qt.callLater(reload)
    onAnchorChanged: Qt.callLater(reload)
    onRevisionChanged: Qt.callLater(reload)

    readonly property var app: detail ? detail.app : null
    readonly property color accent: Categories.color(app ? app.category : "other")

    radius: Appearance.rounding.large
    color: Appearance.colors.layer1

    // Banner: a Steam game's artwork, else the icon on a big pale flower
    // among stickers.
    Item {
        id: banner

        width: parent.width
        height: 150

        RoundedImage {
            id: bannerArt
            anchors.fill: parent
            radius: Appearance.rounding.large
            topOnly: true
            source: root.app && root.app.art ? "file://" + root.app.art : ""
            opacity: 0.85
        }
        Rectangle {
            anchors.fill: parent
            visible: bannerArt.visible
            gradient: Gradient {
                GradientStop { position: 0.4; color: Qt.alpha(Appearance.colors.layer1, 0.0) }
                GradientStop { position: 1.0; color: Appearance.colors.layer1 }
            }
        }
        Stickers {
            anchors.fill: parent
            visible: !bannerArt.visible
            key: root.app ? root.app.key : ""
            count: 3
            minimumSize: 40
            maximumSize: 110
        }
        MaterialShape {
            anchors.centerIn: parent
            visible: !bannerArt.visible
            width: 116
            height: 116
            shape: root.app ? Categories.info(root.app.category).shape : "flower"
            color: Appearance.colors.tonal(root.accent)
        }
        AppIcon {
            anchors.centerIn: parent
            visible: !bannerArt.visible
            source: root.app && root.app.icon ? "file://" + root.app.icon : ""
            name: root.app ? root.app.name : ""
            size: 72
        }

        IconButton {
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.margins: 12
            variant: IconButton.Tonal
            symbol: "close"
            size: 36
            iconSize: 20
            tooltip: qsTr("Close")
            onClicked: root.closeRequested()
        }
    }

    Flickable {
        id: flick

        anchors.top: banner.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 8
        contentWidth: width
        contentHeight: column.implicitHeight + 20
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        T.ScrollBar.vertical: StyledScrollBar {}

        ColumnLayout {
            id: column

            x: 20
            y: 4
            width: flick.width - 40
            spacing: 14

            Column {
                Layout.fillWidth: true
                spacing: 6

                TitleText {
                    width: parent.width
                    text: root.app ? root.app.name : ""
                    font.pixelSize: Appearance.fontSize.huge
                    elide: Text.ElideRight
                }

                // The category, as a chip that opens the choice.
                RippleButton {
                    id: categoryChip

                    readonly property var info: Categories.info(root.app ? root.app.category : "other")

                    height: 32
                    leftPadding: 8
                    rightPadding: 12
                    buttonRadius: Appearance.rounding.full
                    colBackground: Appearance.colors.tonal(root.accent)
                    colBackgroundHover: Appearance.colors.layer2Hover
                    tooltip: qsTr("Change the category")
                    onClicked: categoryMenu.open()

                    contentItem: Row {
                        spacing: 6

                        MaterialSymbol {
                            anchors.verticalCenter: parent.verticalCenter
                            text: categoryChip.info.symbol
                            iconSize: 17
                            fill: 1
                            color: root.accent
                        }
                        StyledText {
                            anchors.verticalCenter: parent.verticalCenter
                            text: categoryChip.info.name
                            font.pixelSize: Appearance.fontSize.smallie
                            font.weight: Font.DemiBold
                        }
                        MaterialSymbol {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "arrow_drop_down"
                            iconSize: 18
                            color: Appearance.colors.onSurfaceVariant
                        }
                    }

                    CategoryMenu {
                        id: categoryMenu
                        y: parent.height + 6
                        current: root.app ? root.app.category : ""
                        custom: root.app ? root.app.custom_category : false
                        onPicked: category => {
                            if (ScreenTime.setCategory(root.appId, category)) {
                                root.reload()
                                root.categoryChanged()
                            }
                        }
                    }
                }
            }

            GridLayout {
                Layout.fillWidth: true
                columns: 2
                columnSpacing: 16
                rowSpacing: 12

                Repeater {
                    model: [
                        { label: Range.title(root.span, root.anchor), value: Format.duration(root.detail ? root.detail.total : 0), strong: true },
                        { label: qsTr("Switched to"), value: !root.detail ? "" : root.detail.opens === 1 ? qsTr("Once") : qsTr("%1 times").arg(root.detail.opens) },
                        { label: qsTr("Longest stretch"), value: Format.duration(root.detail ? root.detail.longest : 0) },
                        { label: qsTr("Last used"), value: root.detail && root.detail.last_used ? Format.ago(root.detail.last_used) : qsTr("Never") }
                    ]

                    Column {
                        required property var modelData

                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        spacing: 2

                        StyledText {
                            width: parent.width
                            text: modelData.label
                            color: Appearance.colors.onSurfaceVariant
                            font.pixelSize: Appearance.fontSize.smaller
                            elide: Text.ElideRight
                        }
                        StyledText {
                            width: parent.width
                            text: modelData.value
                            color: Appearance.colors.onSurface
                            font.pixelSize: modelData.strong ? Appearance.fontSize.larger : Appearance.fontSize.small
                            font.weight: Font.DemiBold
                            elide: Text.ElideRight
                            tabularNumbers: true
                        }
                    }
                }
            }

            UsageChart {
                Layout.fillWidth: true
                Layout.preferredHeight: 130
                single: true
                singleColor: root.accent
                buckets: root.detail ? root.detail.buckets : []
                edges: Range.edges(root.span, root.anchor)
                span: root.span
            }

            SectionLabel {
                Layout.fillWidth: true
                inset: 0
                text: qsTr("Window Titles")
            }

            Repeater {
                model: root.detail ? root.detail.titles : []

                Item {
                    id: titleRow

                    required property var modelData

                    Layout.fillWidth: true
                    implicitHeight: 40

                    HoverHandler {
                        id: titleHover
                    }

                    StyledText {
                        id: titleTime
                        anchors.right: parent.right
                        y: 2
                        text: Format.duration(titleRow.modelData.total)
                        color: Appearance.colors.onSurfaceVariant
                        font.pixelSize: Appearance.fontSize.smaller
                        tabularNumbers: true
                    }
                    StyledText {
                        anchors.left: parent.left
                        anchors.right: titleTime.left
                        anchors.rightMargin: 10
                        y: 2
                        text: titleRow.modelData.title
                        font.pixelSize: Appearance.fontSize.smallie
                        elide: Text.ElideRight
                    }
                    Rectangle {
                        y: 26
                        width: parent.width
                        height: 4
                        radius: 2
                        color: Appearance.colors.layer3

                        Rectangle {
                            width: Math.max(4, parent.width * titleRow.modelData.total / Math.max(1, root.detail.titles[0].total))
                            height: parent.height
                            radius: 2
                            color: root.accent
                        }
                    }
                    Loader {
                        anchors.fill: parent
                        active: titleHover.hovered
                        sourceComponent: StyledToolTip {
                            visible: true
                            text: titleRow.modelData.title
                        }
                    }
                }
            }

            StyledText {
                Layout.fillWidth: true
                visible: root.detail !== null && root.detail.titles.length === 0
                text: root.detail && root.detail.total > 0
                      ? qsTr("No titles were kept. Turn them on under Tracking.")
                      : qsTr("Not used in this span.")
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smallie
                wrapMode: Text.Wrap
            }

            StyledText {
                Layout.fillWidth: true
                visible: root.detail !== null
                topPadding: 6
                text: root.detail && root.app ? qsTr("First seen %1 · %2").arg(Format.dayMonth(root.detail.first_seen)).arg(root.app.key) : ""
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smallest
                elide: Text.ElideMiddle
            }
        }
    }
}
