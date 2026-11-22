import QtQuick
import PluScreenTime

// A day as one strip: each stretch at the screen a block on it, the hours
// marked under it, and now as a line. A block answers the pointer with when
// it was and how long.
Item {
    id: root

    /// `Summary.sessions`: [start, end] pairs.
    property var sessions: []
    property real from: 0
    property real to: 1

    readonly property real span: Math.max(1, to - from)
    readonly property int hoveredIndex: {
        if (!hover.hovered)
            return -1
        const t = from + (hover.point.position.x - track.x) / track.width * span
        // The nearest block within a few pixels, so short ones can be hit.
        const slack = 6 / track.width * span
        let best = -1, distance = Infinity
        for (let i = 0; i < sessions.length; i++) {
            const [start, end] = sessions[i]
            const d = t < start ? start - t : t > end ? t - end : 0
            if (d <= slack && d < distance) {
                best = i
                distance = d
            }
        }
        return best
    }

    function xOf(ms) {
        return (ms - from) / span * track.width
    }

    implicitHeight: 64

    HoverHandler {
        id: hover
    }

    Rectangle {
        id: track

        width: parent.width
        height: 28
        radius: Appearance.rounding.verySmall
        color: Appearance.colors.layer3

        Repeater {
            model: root.sessions

            Rectangle {
                required property int index
                required property var modelData

                x: root.xOf(modelData[0])
                width: Math.max(2, root.xOf(modelData[1]) - x)
                height: parent.height
                radius: Math.min(4, width / 2)
                color: root.hoveredIndex === index ? Appearance.colors.primary : Qt.alpha(Appearance.colors.primary, 0.75)

                Behavior on color {
                    ElementMoveFastColor {}
                }
            }
        }

        // Now, while it's on the strip.
        Rectangle {
            readonly property real now: Date.now()
            visible: now >= root.from && now < root.to
            x: root.xOf(now) - 1
            y: -4
            width: 2
            height: parent.height + 8
            radius: 1
            color: Appearance.colors.onSurface
        }
    }

    Repeater {
        model: 5

        StyledText {
            required property int index
            readonly property real edge: {
                const d = new Date(root.from)
                d.setHours(index * 6)
                return d.getTime()
            }

            x: Math.max(0, Math.min(root.width - width, root.xOf(edge) - width / 2))
            y: track.height + 6
            text: Format.pad(index * 6) + ":00"
            color: Appearance.colors.onSurfaceVariant
            font.pixelSize: Appearance.fontSize.smallest
            tabularNumbers: true
        }
    }

    Rectangle {
        readonly property var session: root.hoveredIndex >= 0 ? root.sessions[root.hoveredIndex] : null

        visible: session !== null
        z: 10
        width: tipText.implicitWidth + 20
        height: tipText.implicitHeight + 12
        radius: Appearance.rounding.verySmall
        color: Appearance.colors.tooltip
        x: session ? Math.max(0, Math.min(root.width - width, root.xOf((session[0] + session[1]) / 2) - width / 2)) : 0
        y: -height - 6

        StyledText {
            id: tipText
            anchors.centerIn: parent
            text: parent.session ? Format.clock(parent.session[0]) + " – " + Format.clock(parent.session[1])
                                   + "  ·  " + Format.duration(parent.session[1] - parent.session[0]) : ""
            color: Appearance.colors.onTooltip
            font.pixelSize: Appearance.fontSize.smaller
            tabularNumbers: true
        }
    }
}
