import QtQuick
import PluScreenTime

// The navigation drawer, straight on the window's layer0 with no background
// of its own: the app and today's time at the top, the pages, what has the
// focus right now, and the settings at the bottom with the daemon's state.
// Its right edge drags it wider or narrower.
Item {
    id: root

    required property Window window
    property int page: 0
    property real preferredWidth: Appearance.sizes.sidebarWidth
    /// The daemon's `Status`, or null while it can't be reached.
    property var status: null
    /// Today's total, in ms.
    property real today: 0

    signal pageRequested(int index)
    signal trackingRequested()
    signal appearanceRequested()
    signal currentAppRequested(var app)
    signal widthEdited(real width)

    readonly property string stateText: {
        if (!status)
            return qsTr("Daemon not running")
        switch (status.state) {
        case "active": return qsTr("Tracking")
        case "idle": return qsTr("Away (idle)")
        case "locked": return qsTr("Locked")
        default: return qsTr("No supported desktop")
        }
    }

    property var pages: [
        { name: qsTr("Overview"), symbol: "insights", shape: "cookie9" },
        { name: qsTr("Apps"), symbol: "apps", shape: "flower" }
    ]

    // The window's top edge moves it, here as on the top bar.
    Item {
        id: header

        width: parent.width
        height: Appearance.sizes.topBarHeight

        DragHandler {
            target: null
            onActiveChanged: if (active) root.window.startSystemMove()
        }
        TapHandler {
            onDoubleTapped: root.window.visibility === Window.Maximized ? root.window.showNormal() : root.window.showMaximized()
        }
        HoverHandler {
            id: headerHover
        }

        ShapeIcon {
            id: badge
            x: 12
            anchors.verticalCenter: parent.verticalCenter
            size: 38
            iconSize: 20
            shape: "cookie12"
            symbol: "hourglass"
            color: Appearance.colors.primaryContainer
            iconColor: Appearance.colors.onPrimaryContainer
            hovered: headerHover.hovered
        }

        Column {
            anchors.left: badge.right
            anchors.leftMargin: 12
            anchors.right: parent.right
            anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            spacing: 1

            TitleText {
                width: parent.width
                text: "PluScreenTime"
                font.pixelSize: Appearance.fontSize.larger
                elide: Text.ElideRight
            }
            StyledText {
                width: parent.width
                text: root.status ? qsTr("%1 today").arg(Format.duration(root.today)) : root.stateText
                tabularNumbers: true
                font.pixelSize: Appearance.fontSize.smaller
                color: Appearance.colors.onSurfaceVariant
                elide: Text.ElideRight
            }
        }
    }

    Column {
        id: navigation

        anchors.top: header.bottom
        anchors.topMargin: 8
        width: parent.width
        spacing: 2

        SectionLabel {
            width: parent.width
            text: qsTr("Screen Time")
        }

        Repeater {
            model: root.pages

            NavigationItem {
                required property int index
                required property var modelData

                width: navigation.width
                text: modelData.name
                symbol: modelData.symbol
                iconShape: modelData.shape
                selected: root.page === index
                onClicked: root.pageRequested(index)
            }
        }
    }

    // What has the focus right now, and for how long.
    Rectangle {
        id: now

        readonly property var current: root.status && root.status.state === "active" ? root.status.current : null

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: 8
        anchors.rightMargin: 8
        anchors.bottom: settings.top
        anchors.bottomMargin: 12
        height: 64
        radius: Appearance.rounding.normal
        color: nowHover.hovered && current ? Appearance.colors.layer1Hover : Appearance.colors.layer1
        visible: root.width > 150

        Behavior on color {
            ElementMoveFastColor {}
        }

        HoverHandler {
            id: nowHover
            cursorShape: now.current ? Qt.PointingHandCursor : Qt.ArrowCursor
        }
        TapHandler {
            enabled: now.current !== null
            onTapped: root.currentAppRequested(now.current.app)
        }

        AppIcon {
            id: nowIcon
            x: 12
            anchors.verticalCenter: parent.verticalCenter
            size: 36
            visible: now.current !== null
            source: now.current && now.current.app.icon ? "file://" + now.current.app.icon : ""
            name: now.current ? now.current.app.name : ""
        }
        ShapeIcon {
            x: 12
            anchors.verticalCenter: parent.verticalCenter
            visible: now.current === null
            size: 36
            iconSize: 18
            shape: root.status ? "cookie9" : "pixelCircle"
            symbol: !root.status ? "link_off" : root.status.state === "locked" ? "lock" : "bedtime"
            color: Appearance.colors.surfaceContainerHigh
            iconColor: Appearance.colors.onSurfaceVariant
        }

        Column {
            anchors.left: nowIcon.right
            anchors.leftMargin: 10
            anchors.right: parent.right
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            spacing: 1

            StyledText {
                width: parent.width
                text: now.current ? qsTr("Now") : root.stateText
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smallest
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
            StyledText {
                width: parent.width
                text: now.current ? now.current.app.name : (root.status ? qsTr("Not counting") : qsTr("Nothing is counted"))
                font.pixelSize: Appearance.fontSize.smallie
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
            StyledText {
                width: parent.width
                visible: now.current !== null
                text: now.current ? Format.duration(Date.now() - now.current.since) : ""
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smallest
                tabularNumbers: true
            }
        }
    }

    Column {
        id: settings

        anchors.bottom: parent.bottom
        anchors.bottomMargin: 8
        width: parent.width
        spacing: 2

        SectionLabel {
            width: parent.width
            text: qsTr("Settings")
        }

        NavigationItem {
            width: settings.width
            text: qsTr("Tracking")
            symbol: "timer"
            iconShape: "clover8"
            onClicked: root.trackingRequested()

            trailing: Item {
                width: 22
                height: 22

                readonly property bool counting: root.status !== null && root.status.state === "active"

                // A ring that spreads from the dot now and then, while the
                // time counts and nothing asks for stillness.
                MaterialShape {
                    id: pulse
                    anchors.centerIn: parent
                    width: 12
                    height: 12
                    shape: "cookie9"
                    color: Qt.alpha(dot.color, 0.5)
                    opacity: 0
                }
                MaterialShape {
                    id: dot
                    anchors.centerIn: parent
                    width: 12
                    height: 12
                    // Unreachable is the pixel circle, as no network is.
                    shape: root.status ? "cookie9" : "pixelCircle"
                    keepsShape: true
                    color: !root.status ? Appearance.colors.error
                         : parent.counting ? Appearance.colors.good : Appearance.colors.warn

                    Behavior on color {
                        ElementMoveFastColor {}
                    }
                }
                Timer {
                    running: parent.counting && !Appearance.reduceMotion
                    interval: 2000
                    repeat: true
                    onTriggered: pulseAnimation.restart()
                }
                ParallelAnimation {
                    id: pulseAnimation
                    NumberAnimation { target: pulse; property: "scale"; from: 0.8; to: 2.2; duration: 900; easing.type: Easing.OutCubic }
                    NumberAnimation { target: pulse; property: "opacity"; from: 0.9; to: 0; duration: 900; easing.type: Easing.OutCubic }
                }
            }
        }

        NavigationItem {
            width: settings.width
            text: qsTr("Appearance")
            symbol: "palette"
            iconShape: "flower"
            onClicked: root.appearanceRequested()
        }
    }

    // Dragging the right edge sets the drawer's width.
    Item {
        x: parent.width - 4
        width: 8
        height: parent.height

        HoverHandler {
            cursorShape: Qt.SizeHorCursor
        }
        DragHandler {
            id: resize

            property real startWidth

            target: null
            xAxis.enabled: true
            yAxis.enabled: false
            onActiveChanged: {
                if (active)
                    startWidth = root.width
                else
                    root.widthEdited(root.preferredWidth)
            }
            onTranslationChanged: {
                if (active) {
                    const max = Math.min(Appearance.sizes.sidebarMaxWidth, root.window.width * 0.28)
                    root.preferredWidth = Math.max(Appearance.sizes.sidebarMinWidth, Math.min(max, startWidth + translation.x))
                }
            }
        }
    }
}
