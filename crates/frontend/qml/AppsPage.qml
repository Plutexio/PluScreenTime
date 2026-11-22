import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import PluScreenTime

// Every app ever used, with its time over the whole history and when it was
// last used. The one picked opens in the inspector, where its category can
// change.
Item {
    id: root

    property int revision: 0
    property real selectedApp: -1
    /// time or name.
    property string sort: "time"

    signal appRequested(var app)
    signal sortEdited(string sort)

    property var apps: []

    function reload() {
        const json = ScreenTime.apps()
        apps = json ? JSON.parse(json).apps : []
    }

    onRevisionChanged: Qt.callLater(reload)
    onVisibleChanged: if (visible) reload()
    Component.onCompleted: reload()

    readonly property var shown: {
        const query = search.text.trim().toLowerCase()
        let list = apps.filter(a => a.total > 0 || a.last_used)
        if (query)
            list = list.filter(a => a.app.name.toLowerCase().includes(query)
                                    || Categories.name(a.app.category).toLowerCase().includes(query))
        if (sort === "name")
            list = list.slice().sort((a, b) => a.app.name.localeCompare(b.app.name))
        return list
    }
    readonly property real longest: apps.reduce((max, a) => Math.max(max, a.total), 1)
    readonly property real grandTotal: apps.reduce((sum, a) => sum + a.total, 0)

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: Appearance.sizes.pageMargin
        anchors.rightMargin: Appearance.sizes.pageMargin
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            SearchField {
                id: search
                Layout.preferredWidth: 300
                placeholder: qsTr("Search apps or categories")
            }

            SegmentedControl {
                currentIndex: root.sort === "name" ? 1 : 0
                segments: [
                    Segment { text: qsTr("Time"); symbol: "hourglass" },
                    Segment { text: qsTr("Name"); symbol: "sort_by_alpha" }
                ]
                onActivated: index => root.sortEdited(index === 1 ? "name" : "time")
            }

            Item {
                Layout.fillWidth: true
            }

            StyledText {
                text: qsTr("%1 apps · %2 in all").arg(root.shown.length).arg(Format.duration(root.grandTotal))
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smallie
                tabularNumbers: true
                elide: Text.ElideRight
                Layout.maximumWidth: 300
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.bottomMargin: Appearance.sizes.pageMargin
            radius: Appearance.rounding.large
            color: Appearance.colors.layer2

            ListView {
                id: list

                anchors.fill: parent
                anchors.margins: 10
                clip: true
                spacing: 2
                boundsBehavior: Flickable.StopAtBounds
                model: root.shown

                T.ScrollBar.vertical: StyledScrollBar {}

                delegate: AppRow {
                    required property var modelData

                    width: list.width - 8
                    app: modelData.app
                    total: modelData.total
                    scale: root.longest
                    note: Categories.name(modelData.app.category)
                          + (modelData.last_used ? "  ·  " + Format.ago(modelData.last_used) : "")
                    selected: modelData.app.id === root.selectedApp
                    onClicked: root.appRequested(modelData.app)
                }
            }

            EmptyState {
                anchors.centerIn: parent
                visible: root.shown.length === 0
                symbol: search.text ? "search_off" : "apps"
                title: search.text ? qsTr("Nothing Matches") : qsTr("No Apps Yet")
                description: search.text ? qsTr("No app matches “%1”.").arg(search.text)
                                          : qsTr("Apps show up here once you've spent time in them.")
            }
        }
    }
}
