import QtQuick
import QtQuick.Templates as T
import PluScreenTime

// What the daemon sees and how it counts: whether it runs, which desktop it
// follows, whether the compositor tells it when nobody's there, and the
// settings for that.
StyledDialog {
    id: root

    /// The daemon's `Status`, or null.
    property var status: null

    signal settingsEdited()

    // The idle timeout's stops, in minutes.
    readonly property var stops: [1, 2, 3, 5, 10, 15, 20, 30, 45, 60]
    readonly property var settings: status ? status.settings : null
    readonly property int stopIndex: {
        if (!settings)
            return 3
        const minutes = settings.idle_timeout_secs / 60
        let best = 0
        for (let i = 0; i < stops.length; i++) {
            if (Math.abs(stops[i] - minutes) < Math.abs(stops[best] - minutes))
                best = i
        }
        return best
    }

    function save(change) {
        const next = Object.assign({}, root.settings, change)
        if (ScreenTime.setSettings(JSON.stringify(next)))
            root.settingsEdited()
    }

    headline: qsTr("Tracking")
    symbol: "timer"
    iconShape: "clover8"
    preferredWidth: 520

    component StateRow: Item {
        property string label
        property string value
        property string symbol
        property bool good: true

        width: parent.width
        height: 44

        ShapeIcon {
            id: badge
            anchors.verticalCenter: parent.verticalCenter
            size: 32
            iconSize: 17
            shape: parent.good ? "cookie9" : "pixelCircle"
            symbol: parent.symbol
            color: parent.good ? Appearance.colors.tonal(Appearance.colors.good) : Appearance.colors.errorContainer
            iconColor: parent.good ? Appearance.colors.good : Appearance.colors.onErrorContainer
        }
        StyledText {
            anchors.left: badge.right
            anchors.leftMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            text: parent.label
        }
        StyledText {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            width: Math.min(implicitWidth, parent.width * 0.55)
            horizontalAlignment: Text.AlignRight
            text: parent.value
            color: Appearance.colors.onSurfaceVariant
            font.pixelSize: Appearance.fontSize.smallie
            elide: Text.ElideRight
        }
    }

    StateRow {
        label: qsTr("Daemon")
        symbol: root.status ? "check" : "link_off"
        good: root.status !== null
        value: root.status ? qsTr("Running") : qsTr("Not running")
    }
    StyledText {
        width: parent.width
        visible: root.status === null
        text: qsTr("Start it with: systemctl --user enable --now pluscreentimed")
        color: Appearance.colors.onSurfaceVariant
        font.pixelSize: Appearance.fontSize.smaller
        font.family: Appearance.monoFontFamily
        wrapMode: Text.Wrap
    }
    StateRow {
        visible: root.status !== null
        label: qsTr("Desktop")
        symbol: "desktop_windows"
        good: root.status !== null && root.status.desktop !== null
        value: !root.status || !root.status.desktop ? qsTr("None it can follow")
             : root.status.desktop === "kwin" ? "KDE Plasma" : "Hyprland"
    }
    StateRow {
        visible: root.status !== null
        label: qsTr("Idle detection")
        symbol: "bedtime"
        good: root.status !== null && root.status.idle_detection
        value: root.status && root.status.idle_detection ? qsTr("From the compositor") : qsTr("Unavailable; counts until locked")
    }
    StateRow {
        visible: root.status !== null
        label: qsTr("Right now")
        symbol: "radio_button_checked"
        good: root.status !== null && root.status.state === "active"
        value: {
            if (!root.status)
                return ""
            switch (root.status.state) {
            case "active": return root.status.session_start
                                  ? qsTr("Counting, for %1").arg(Format.duration(Date.now() - root.status.session_start))
                                  : qsTr("Counting")
            case "idle": return qsTr("Away")
            case "locked": return qsTr("Locked")
            default: return qsTr("Not counting")
            }
        }
    }

    Item {
        width: parent.width
        height: 4
    }

    SectionLabel {
        width: parent.width
        inset: 0
        visible: root.settings !== null
        text: qsTr("Counting")
    }

    Column {
        width: parent.width
        visible: root.settings !== null
        spacing: 2

        StyledText {
            width: parent.width
            text: qsTr("Away after %1 without input").arg(Format.duration(root.stops[root.stopIndex] * 60000))
        }
        StyledText {
            width: parent.width
            text: qsTr("That stretch isn't counted. Videos and games that keep the screen awake still are.")
            color: Appearance.colors.onSurfaceVariant
            font.pixelSize: Appearance.fontSize.smaller
            wrapMode: Text.Wrap
        }
        StyledSlider {
            width: parent.width
            from: 0
            to: root.stops.length - 1
            stepSize: 1
            snapMode: T.Slider.SnapAlways
            value: root.stopIndex
            format: value => root.stops[Math.round(value)] + " min"
            onMoved: {
                const minutes = root.stops[Math.round(value)]
                if (minutes * 60 !== root.settings.idle_timeout_secs)
                    root.save({ idle_timeout_secs: minutes * 60 })
            }
        }
    }

    RippleButton {
        id: titles

        width: parent.width
        height: 56
        visible: root.settings !== null
        buttonRadius: Appearance.rounding.small
        colBackgroundHover: Appearance.colors.layer3Hover
        colRipple: Appearance.colors.layer3Active
        bounce: false
        onClicked: root.save({ record_titles: !root.settings.record_titles })

        contentItem: Item {
            Column {
                anchors.left: parent.left
                anchors.leftMargin: 4
                anchors.right: toggle.left
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                spacing: 1

                StyledText {
                    text: qsTr("Keep window titles")
                }
                StyledText {
                    width: parent.width
                    text: qsTr("Shows what an app was used for. They stay on this machine.")
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smaller
                    elide: Text.ElideRight
                }
            }
            StyledSwitch {
                id: toggle
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                checked: root.settings ? root.settings.record_titles : false
                onClicked: titles.clicked()
            }
        }
    }

    buttons: [
        TextButton {
            variant: TextButton.Tonal
            text: qsTr("Done")
            onClicked: root.close()
        }
    ]
}
