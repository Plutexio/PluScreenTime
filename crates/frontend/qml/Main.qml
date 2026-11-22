import QtQuick
import PluScreenTime

// PluScreenTime's window: the drawer on layer0, the content sheet beside it
// with a top bar that has no background, and the inspector sheet that opens
// on a spring when an app is picked. 8 px of window between them all.
AppWindow {
    id: window

    readonly property var pageInfo: [
        { title: qsTr("Overview"), key: "overview" },
        { title: qsTr("Apps"), key: "apps" }
    ]

    /// day or week, and a time inside the one shown.
    property string span: prefs.span === "week" ? "week" : "day"
    property real anchor: Date.now()
    /// Bumped whenever what's shown should be fetched again.
    property int revision: 0
    property var status: null
    property real today: 0
    property real selectedApp: -1

    function setSpan(next) {
        if (next !== span)
            setPref("span", next)
    }

    function step(steps) {
        const next = Range.shifted(span, anchor, steps)
        if (next <= Date.now())
            anchor = next
    }

    function refresh() {
        const json = ScreenTime.status()
        status = json ? JSON.parse(json) : null
        const edges = Range.edges("day", Date.now())
        const todayJson = ScreenTime.summary(JSON.stringify([edges[0], edges[edges.length - 1]]))
        today = todayJson ? JSON.parse(todayJson).total : 0
    }

    function pickApp(app) {
        selectedApp = selectedApp === app.id ? -1 : app.id
    }

    appName: "PluScreenTime"
    snackbar: messages

    onPageChanged: selectedApp = -1
    onOpenRequested: name => {
        if (name === "tracking")
            trackingDialog.open()
        else if (name === "back")
            window.step(-1)
        else if (name.startsWith("app:"))
            window.selectedApp = Number(name.slice(4))
    }

    // The status and today's total every few seconds; what's on screen when
    // it includes now (older days don't change).
    Timer {
        running: true
        repeat: true
        triggeredOnStart: true
        interval: 5000
        onTriggered: window.refresh()
    }
    Timer {
        running: Range.isCurrent(window.span, window.anchor) || window.page === 1
        repeat: true
        interval: 30000
        onTriggered: window.revision++
    }
    // Back from away (or the daemon back), everything again.
    Connections {
        target: ScreenTime
        function onConnectedChanged() {
            if (ScreenTime.connected)
                window.revision++
        }
    }

    Sidebar {
        id: sidebar

        x: window.gutter
        y: 0
        width: preferredWidth
        height: parent.height
        window: window
        page: window.page
        status: window.status
        today: window.today
        preferredWidth: window.prefs.sidebar_width ?? Appearance.sizes.sidebarWidth
        onPageRequested: index => window.page = index
        onTrackingRequested: trackingDialog.open()
        onAppearanceRequested: window.appearanceDialog.open()
        onCurrentAppRequested: app => window.pickApp(app)
        onWidthEdited: width => window.setPref("sidebar_width", width)
    }

    // The content sheet.
    Rectangle {
        id: sheet

        x: sidebar.x + sidebar.width + window.gutter
        y: window.gutter
        width: inspector.x - window.gutter - x
        height: parent.height - 2 * window.gutter
        radius: Appearance.rounding.large
        color: Appearance.colors.layer1
        clip: true

        // Faint shapes behind the page's title, a layout of its own for
        // each page, cut off by the sheet's top edge and kept out of
        // the page below.
        Item {
            width: parent.width
            height: Appearance.sizes.topBarHeight
            clip: true

            Stickers {
                anchors.fill: parent
                key: window.pageInfo[window.page].key
                count: 4
                minimumSize: 50
                maximumSize: 110
                fromY: -0.5
                toY: 0
                strength: 0.8
            }
        }

        Item {
            anchors.fill: parent

            OverviewPage {
                anchors.fill: parent
                anchors.topMargin: Appearance.sizes.topBarHeight
                span: window.span
                anchor: window.anchor
                revision: window.revision
                selectedApp: window.selectedApp
                visible: opacity > 0
                opacity: window.page === 0 ? 1 : 0
                onAppRequested: app => window.pickApp(app)
                onDayRequested: day => {
                    window.anchor = day
                    window.setSpan("day")
                }

                Behavior on opacity {
                    NumberAnimation { duration: Appearance.duration(220) }
                }
            }

            AppsPage {
                anchors.fill: parent
                anchors.topMargin: Appearance.sizes.topBarHeight
                revision: window.revision
                selectedApp: window.selectedApp
                sort: window.prefs.apps_sort ?? "time"
                visible: opacity > 0
                opacity: window.page === 1 ? 1 : 0
                onAppRequested: app => window.pickApp(app)
                onSortEdited: sort => window.setPref("apps_sort", sort)

                Behavior on opacity {
                    NumberAnimation { duration: Appearance.duration(220) }
                }
            }
        }

        TopBar {
            id: topBar

            width: parent.width
            height: Appearance.sizes.topBarHeight
            window: window
            title: window.page === 0 ? Range.title(window.span, window.anchor) : window.pageInfo[window.page].title

            leading: [
                ToolbarButton {
                    visible: window.page === 0
                    size: 36
                    iconSize: 20
                    symbol: "chevron_left"
                    shape: "cookie7"
                    effect: ToolbarButton.NudgeLeft
                    tooltip: window.span === "week" ? qsTr("Week Before") : qsTr("Day Before")
                    onClicked: window.step(-1)
                },
                ToolbarButton {
                    visible: window.page === 0
                    enabled: !Range.isCurrent(window.span, window.anchor)
                    opacity: enabled ? 1 : 0.38
                    size: 36
                    iconSize: 20
                    symbol: "chevron_right"
                    shape: "cookie7"
                    effect: ToolbarButton.NudgeRight
                    tooltip: window.span === "week" ? qsTr("Week After") : qsTr("Day After")
                    onClicked: window.step(1)
                }
            ]

            TextButton {
                anchors.verticalCenter: parent.verticalCenter
                visible: window.page === 0 && !Range.isCurrent(window.span, window.anchor)
                variant: TextButton.Tonal
                symbol: "today"
                text: window.span === "week" ? qsTr("This Week") : qsTr("Today")
                onClicked: window.anchor = Date.now()
            }
            SegmentedControl {
                anchors.verticalCenter: parent.verticalCenter
                visible: window.page === 0
                currentIndex: window.span === "week" ? 1 : 0
                segments: [
                    Segment { text: qsTr("Day"); symbol: "calendar_view_day" },
                    Segment { text: qsTr("Week"); symbol: "calendar_view_week" }
                ]
                onActivated: index => window.setSpan(index === 1 ? "week" : "day")
            }
        }

        Snackbar {
            id: messages
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 16
            z: 50
        }

        // What fills the sheet clips square; these round its corners off.
        CornerCaps {
            anchors.fill: parent
            radius: sheet.radius
            color: Appearance.colors.layer0
            z: 60
        }
    }

    // The inspector: the app picked. It opens on a spring, so the page
    // beside it gives a little too, and stays shut while the page would be
    // left narrower than 404 px.
    Item {
        id: inspector

        readonly property bool wanted: window.selectedApp >= 0
        readonly property bool roomy: window.frame.width - sheet.x - Appearance.sizes.sideSheetWidth - 2 * window.gutter >= 404
        property real openness: wanted && roomy ? 1 : 0

        x: window.frame.width - window.gutter - openness * (Appearance.sizes.sideSheetWidth + window.gutter)
        y: window.gutter
        width: Appearance.sizes.sideSheetWidth
        height: parent.height - 2 * window.gutter
        visible: openness > 0.001

        Behavior on openness {
            enabled: !Appearance.performanceMode
            SpringAnimation { spring: 4; damping: 0.32; epsilon: 0.002 }
        }

        AppDetails {
            anchors.fill: parent
            appId: window.selectedApp
            span: window.span
            anchor: window.anchor
            revision: window.revision
            onCloseRequested: window.selectedApp = -1
            onCategoryChanged: window.revision++
        }
    }

    TrackingDialog {
        id: trackingDialog
        status: window.status
        onAboutToShow: window.refresh()
        onSettingsEdited: window.refresh()
    }

    Shortcut {
        sequence: "Escape"
        enabled: window.selectedApp >= 0
        onActivated: window.selectedApp = -1
    }
    Shortcut {
        sequence: "Left"
        enabled: window.page === 0
        onActivated: window.step(-1)
    }
    Shortcut {
        sequence: "Right"
        enabled: window.page === 0
        onActivated: window.step(1)
    }
}
