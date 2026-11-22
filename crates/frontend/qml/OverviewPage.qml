import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import PluScreenTime

// A day or a week at the screen: the total and how it compares, pickups and
// the longest stretch, the time across the hours or days by category, the
// stretches of a day, and the apps and categories it went to.
Item {
    id: root

    property string span: "day"
    property real anchor: Date.now()
    /// Bumped to fetch again (new data, the daemon back).
    property int revision: 0
    property real selectedApp: -1
    property bool connected: ScreenTime.connected

    signal appRequested(var app)
    signal dayRequested(real day)

    property var summary: null
    /// The span before, for comparing.
    property var previous: null
    property bool showAllApps: false

    readonly property var edges: summary ? summary.edges : Range.edges(span, anchor)
    readonly property real total: summary ? summary.total : 0
    readonly property bool current: Range.isCurrent(span, anchor)
    readonly property bool future: Range.start(span, anchor) > Date.now()

    // Days of a week that have begun, for its daily average.
    readonly property int daysSoFar: {
        if (span !== "week")
            return 1
        let days = 0
        for (let i = 0; i + 1 < edges.length; i++) {
            if (edges[i] <= Date.now())
                days++
        }
        return Math.max(1, days)
    }
    readonly property real average: total / daysSoFar

    function reload() {
        const edges = Range.edges(span, anchor)
        const json = ScreenTime.summary(JSON.stringify(edges))
        summary = json ? JSON.parse(json) : null
        // The day before up to the same time of day, while today runs on;
        // the week before as a daily average.
        const before = Range.shifted(span, anchor, -1)
        let end = Range.end(span, before)
        if (span === "day" && current)
            end = Math.min(end, before + (Date.now() - edges[0]))
        const previousJson = ScreenTime.summary(JSON.stringify([before, Math.max(before + 1, end)]))
        previous = previousJson ? JSON.parse(previousJson) : null
    }

    onSpanChanged: {
        showAllApps = false
        Qt.callLater(reload)
    }
    onAnchorChanged: Qt.callLater(reload)
    onRevisionChanged: Qt.callLater(reload)
    Component.onCompleted: reload()

    readonly property string comparison: {
        if (!previous || future)
            return ""
        if (span === "week") {
            const before = previous.total / 7
            if (before <= 0)
                return qsTr("Nothing the week before")
            const change = Math.round((average - before) / before * 100)
            if (change === 0)
                return qsTr("Same as the week before")
            return qsTr("%1% %2 than the week before").arg(Math.abs(change)).arg(change > 0 ? qsTr("more") : qsTr("less"))
        }
        if (previous.total <= 0)
            return current ? qsTr("Nothing yesterday by now") : qsTr("Nothing the day before")
        const difference = total - previous.total
        const when = current ? qsTr("than yesterday by now") : qsTr("than the day before")
        if (Math.abs(difference) < 60000)
            return current ? qsTr("About the same as yesterday by now") : qsTr("About the same as the day before")
        return qsTr("%1 %2 %3").arg(Format.duration(Math.abs(difference))).arg(difference > 0 ? qsTr("more") : qsTr("less")).arg(when)
    }
    readonly property bool more: previous !== null && (span === "week" ? average > previous.total / 7 : total > previous.total)

    // The busiest day of a week.
    readonly property int busiestDay: {
        if (!summary || span !== "week")
            return -1
        let best = -1
        for (let i = 0; i < summary.buckets.length; i++) {
            if (summary.buckets[i].total > 0 && (best < 0 || summary.buckets[i].total > summary.buckets[best].total))
                best = i
        }
        return best
    }

    Flickable {
        id: flick

        anchors.fill: parent
        contentWidth: width
        contentHeight: content.implicitHeight + Appearance.sizes.pageMargin
        boundsBehavior: Flickable.StopAtBounds
        clip: true
        visible: root.connected || root.summary !== null

        T.ScrollBar.vertical: StyledScrollBar {}

        ColumnLayout {
            id: content

            x: Appearance.sizes.pageMargin
            width: flick.width - 2 * Appearance.sizes.pageMargin
            spacing: Appearance.sizes.shelfGap

            // The total, and the figures beside it.
            GridLayout {
                Layout.fillWidth: true
                columns: content.width > 900 ? 4 : 3
                columnSpacing: Appearance.sizes.shelfGap
                rowSpacing: Appearance.sizes.shelfGap

                Rectangle {
                    id: hero

                    Layout.fillWidth: true
                    Layout.columnSpan: content.width > 900 ? 1 : 3
                    Layout.preferredWidth: content.width > 900 ? 2 : 1
                    Layout.preferredHeight: 132
                    radius: Appearance.rounding.large
                    color: Appearance.colors.primaryContainer
                    clip: true

                    Stickers {
                        anchors.fill: parent
                        key: root.span + Range.start(root.span, root.anchor)
                        count: 3
                        minimumSize: 40
                        maximumSize: 100
                        fromY: 0.2
                        toY: 1.2
                        strength: 0.9
                    }

                    Column {
                        x: 22
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width - 44
                        spacing: 4

                        StyledText {
                            text: root.span === "week" ? qsTr("Screen time this week") : qsTr("Screen time")
                            color: Appearance.colors.onPrimaryContainer
                            opacity: 0.8
                            font.pixelSize: Appearance.fontSize.smallie
                            font.weight: Font.DemiBold
                        }
                        TitleText {
                            width: parent.width
                            level: "display"
                            text: Format.duration(root.total)
                            color: Appearance.colors.onPrimaryContainer
                            elide: Text.ElideRight
                            animateChange: true
                        }
                        Row {
                            spacing: 6
                            visible: root.comparison.length > 0 || root.span === "week"

                            MaterialSymbol {
                                anchors.verticalCenter: parent.verticalCenter
                                visible: root.comparison.length > 0 && root.previous && root.previous.total > 0
                                text: root.more ? "trending_up" : "trending_down"
                                iconSize: 18
                                color: Appearance.colors.onPrimaryContainer
                            }
                            StyledText {
                                anchors.verticalCenter: parent.verticalCenter
                                text: root.span === "week"
                                      ? qsTr("%1 a day").arg(Format.duration(root.average)) + (root.comparison ? "  ·  " + root.comparison : "")
                                      : root.comparison
                                color: Appearance.colors.onPrimaryContainer
                                font.pixelSize: Appearance.fontSize.smallie
                                tabularNumbers: true
                            }
                        }
                    }
                }

                StatTile {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    label: qsTr("Pickups")
                    symbol: "touch_app"
                    shape: "flower"
                    value: root.summary ? root.summary.pickups : "0"
                    note: {
                        if (!root.summary || root.summary.pickups === 0)
                            return qsTr("None yet")
                        if (root.span === "week")
                            return qsTr("About %1 a day").arg(Math.round(root.summary.pickups / root.daysSoFar))
                        return qsTr("First at %1").arg(Format.clock(root.summary.first_activity))
                    }
                }

                StatTile {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    label: qsTr("Longest session")
                    symbol: "timelapse"
                    shape: "clover4"
                    value: Format.duration(root.summary ? root.summary.longest_session : 0)
                    note: qsTr("Without stepping away")
                }

                StatTile {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    readonly property var topApp: root.summary && root.summary.apps.length > 0 ? root.summary.apps[0] : null

                    label: root.span === "week" ? qsTr("Busiest day") : qsTr("Most used")
                    symbol: root.span === "week" ? "calendar_today" : "star"
                    shape: "sunny"
                    value: root.span === "week"
                           ? (root.busiestDay >= 0 ? Format.dayNames[new Date(root.edges[root.busiestDay]).getDay()] : "—")
                           : (topApp ? topApp.app.name : "—")
                    note: root.span === "week"
                          ? (root.busiestDay >= 0 ? Format.duration(root.summary.buckets[root.busiestDay].total) : "")
                          : (topApp ? Format.duration(topApp.total) : "")
                }
            }

            Panel {
                Layout.fillWidth: true
                title: root.span === "week" ? qsTr("By Day") : qsTr("By Hour")
                symbol: "bar_chart"
                shape: "cookie7"

                UsageChart {
                    width: parent.width
                    height: 230
                    buckets: root.summary ? root.summary.buckets : []
                    edges: root.edges
                    span: root.span
                    // A day's hours fill up to the hour.
                    scaleMax: root.span === "day" ? 3600000 : 0
                    clickable: root.span === "week"
                    onBucketClicked: index => root.dayRequested(root.edges[index])
                }

                // The legend: each category in the chart, in stack order.
                Flow {
                    width: parent.width
                    spacing: 16
                    visible: root.summary !== null && root.summary.categories.length > 0

                    Repeater {
                        model: root.summary ? root.summary.categories.slice().sort((a, b) => Categories.order(a.id) - Categories.order(b.id)) : []

                        Row {
                            required property var modelData
                            spacing: 6

                            Rectangle {
                                anchors.verticalCenter: parent.verticalCenter
                                width: 10
                                height: 10
                                radius: 3
                                color: Categories.color(modelData.id)
                            }
                            StyledText {
                                text: Categories.name(modelData.id)
                                color: Appearance.colors.onSurfaceVariant
                                font.pixelSize: Appearance.fontSize.smaller
                            }
                        }
                    }
                }
            }

            Panel {
                Layout.fillWidth: true
                visible: root.span === "day"
                title: qsTr("Sessions")
                symbol: "schedule"
                shape: "clover8"
                trailing: StyledText {
                    text: root.summary && root.summary.sessions.length > 0
                          ? qsTr("%1 – %2").arg(Format.clock(root.summary.first_activity)).arg(Format.clock(root.summary.last_activity))
                          : ""
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smaller
                    tabularNumbers: true
                }

                Item {
                    width: parent.width
                    height: 22
                }
                SessionTimeline {
                    width: parent.width
                    sessions: root.summary ? root.summary.sessions : []
                    from: root.edges[0]
                    to: root.edges[root.edges.length - 1]
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Appearance.sizes.shelfGap

                Panel {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 3
                    Layout.alignment: Qt.AlignTop
                    title: qsTr("Apps")
                    symbol: "apps"
                    shape: "flower"
                    spacing: 6
                    trailing: StyledText {
                        text: root.summary ? root.summary.apps.length + (root.summary.apps.length === 1 ? qsTr(" app") : qsTr(" apps")) : ""
                        color: Appearance.colors.onSurfaceVariant
                        font.pixelSize: Appearance.fontSize.smaller
                    }

                    Repeater {
                        model: root.summary ? root.summary.apps.slice(0, root.showAllApps ? root.summary.apps.length : 8) : []

                        AppRow {
                            required property var modelData

                            width: parent.width
                            app: modelData.app
                            total: modelData.total
                            scale: root.summary.apps[0].total
                            note: Format.percent(modelData.total, root.total)
                            selected: modelData.app.id === root.selectedApp
                            onClicked: root.appRequested(modelData.app)
                        }
                    }

                    TextButton {
                        visible: root.summary !== null && root.summary.apps.length > 8
                        variant: TextButton.Text
                        symbol: root.showAllApps ? "expand_less" : "expand_more"
                        text: root.showAllApps ? qsTr("Show Fewer") : qsTr("Show All %1").arg(root.summary ? root.summary.apps.length : 0)
                        onClicked: root.showAllApps = !root.showAllApps
                    }

                    StyledText {
                        width: parent.width
                        visible: root.summary !== null && root.summary.apps.length === 0
                        topPadding: 8
                        bottomPadding: 8
                        text: root.future ? qsTr("This hasn't happened yet.")
                                          : root.current ? qsTr("Nothing yet. Time spent in apps shows up here as it goes.")
                                                         : qsTr("No time at the screen was recorded.")
                        color: Appearance.colors.onSurfaceVariant
                        wrapMode: Text.Wrap
                    }
                }

                Panel {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 2
                    Layout.alignment: Qt.AlignTop
                    title: qsTr("Categories")
                    symbol: "donut_large"
                    shape: "sunny"

                    CategoryBreakdown {
                        width: parent.width
                        categories: root.summary ? root.summary.categories : []
                        total: root.total
                    }

                    StyledText {
                        width: parent.width
                        visible: root.total === 0
                        text: qsTr("Apps are sorted into categories as they're used. Pick an app to move it to another.")
                        color: Appearance.colors.onSurfaceVariant
                        wrapMode: Text.Wrap
                    }
                }
            }
        }
    }

    EmptyState {
        anchors.centerIn: parent
        visible: !flick.visible
        symbol: "link_off"
        title: qsTr("The Daemon Isn't Running")
        description: qsTr("PluScreenTime's daemon keeps the history. Start it with “systemctl --user enable --now pluscreentimed”, or run ./install.sh again.")
    }
}
