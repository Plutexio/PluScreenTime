import QtQuick
import PluScreenTime

// Time per bucket as bars: stacked by category in their fixed order (or one
// colour, for a single app), on a recessive grid with round steps of time.
// Each bar's whole column answers the pointer with a tooltip; a click on one
// is reported, so a week's day can open.
Item {
    id: root

    /// `Summary.buckets` ({ total, categories }), or plain ms with `single`.
    property var buckets: []
    property var edges: []
    property string span: "day"
    property bool single: false
    property color singleColor: Appearance.colors.primary
    /// The top of the scale in ms; 0 picks one from the data.
    property real scaleMax: 0
    property bool clickable: false

    signal bucketClicked(int index)

    readonly property real axisWidth: 46
    readonly property real labelHeight: 22
    readonly property real gap: 2
    readonly property int hovered: hover.hovered ? Math.floor((hover.point.position.x - plot.x) / slotWidth) : -1
    readonly property int hoveredIndex: hovered >= 0 && hovered < buckets.length ? hovered : -1
    readonly property real slotWidth: buckets.length > 0 ? plot.width / buckets.length : plot.width
    readonly property real barWidth: Math.max(4, Math.min(slotWidth * 0.62, 44))

    function valueOf(bucket) {
        return single ? bucket : bucket.total
    }

    readonly property real dataMax: {
        let max = 0
        for (const bucket of buckets)
            max = Math.max(max, valueOf(bucket))
        return max
    }

    // A round step, so the grid reads 15 min, 30 min, 1 h, 2 h…, and at most
    // four lines of it.
    readonly property real step: {
        const highest = scaleMax > 0 ? scaleMax : dataMax
        const minute = 60000
        const steps = [5, 10, 15, 30, 60, 120, 180, 240, 360, 480, 720, 1440].map(m => m * minute)
        for (const candidate of steps) {
            if (highest / candidate <= 4)
                return candidate
        }
        return steps[steps.length - 1]
    }
    readonly property real scaleTop: scaleMax > 0 ? scaleMax : Math.max(step, Math.ceil(dataMax / step) * step)

    /// The bucket that holds now, drawn in the label's accent.
    readonly property int nowIndex: {
        const now = Date.now()
        for (let i = 0; i + 1 < edges.length; i++) {
            if (now >= edges[i] && now < edges[i + 1])
                return i
        }
        return -1
    }

    function stack(bucket) {
        if (single)
            return bucket > 0 ? [{ id: "", value: bucket, color: singleColor }] : []
        const parts = []
        for (const id in bucket.categories)
            parts.push({ id: id, value: bucket.categories[id], color: Categories.color(id) })
        parts.sort((a, b) => Categories.order(a.id) - Categories.order(b.id))
        return parts
    }

    function axisLabel(ms) {
        const minutes = Math.round(ms / 60000)
        return minutes < 60 ? minutes + " min" : (minutes / 60) + " h"
    }

    // The bars grow in when the range changes, not with every refresh.
    property real grown: 1
    property real shownFrom: 0
    onEdgesChanged: {
        const from = edges.length > 0 ? edges[0] : 0
        if (from !== shownFrom) {
            shownFrom = from
            if (!Appearance.performanceMode)
                grow.restart()
        }
    }
    NumberAnimation {
        id: grow
        target: root
        property: "grown"
        from: 0
        to: 1
        duration: Appearance.motion(650)
        easing.type: Easing.BezierSpline
        easing.bezierCurve: Appearance.curves.expressiveDefaultSpatial
    }

    HoverHandler {
        id: hover
        cursorShape: root.clickable && root.hoveredIndex >= 0 ? Qt.PointingHandCursor : Qt.ArrowCursor
    }
    TapHandler {
        enabled: root.clickable
        onTapped: if (root.hoveredIndex >= 0) root.bucketClicked(root.hoveredIndex)
    }

    Item {
        id: plot

        x: root.axisWidth
        y: 8
        width: root.width - root.axisWidth
        height: root.height - root.labelHeight - y

        // The grid: a line and a label for each step, the baseline firmer.
        Repeater {
            model: Math.round(root.scaleTop / root.step) + 1

            Item {
                required property int index
                readonly property real value: index * root.step

                y: plot.height - value / root.scaleTop * plot.height
                width: plot.width

                Rectangle {
                    width: parent.width
                    height: 1
                    color: index === 0 ? Appearance.colors.outlineVariant : Qt.alpha(Appearance.colors.outlineVariant, 0.45)
                }
                StyledText {
                    x: -root.axisWidth
                    width: root.axisWidth - 10
                    y: -height / 2
                    visible: index > 0
                    horizontalAlignment: Text.AlignRight
                    text: root.axisLabel(parent.value)
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smallest
                    tabularNumbers: true
                }
            }
        }

        Repeater {
            model: root.buckets.length

            Item {
                id: slot

                required property int index
                readonly property var parts: root.stack(root.buckets[index])
                readonly property bool dimmed: root.hoveredIndex >= 0 && root.hoveredIndex !== index

                x: index * root.slotWidth
                width: root.slotWidth
                height: plot.height

                Rectangle {
                    anchors.fill: parent
                    anchors.leftMargin: 1
                    anchors.rightMargin: 1
                    radius: Appearance.rounding.verySmall
                    color: Appearance.colors.onSurface
                    opacity: root.hoveredIndex === slot.index ? 0.05 : 0

                    Behavior on opacity {
                        NumberAnimation { duration: Appearance.duration(120) }
                    }
                }

                Item {
                    id: bar

                    anchors.horizontalCenter: parent.horizontalCenter
                    width: root.barWidth
                    height: parent.height
                    opacity: slot.dimmed ? 0.45 : 1

                    Behavior on opacity {
                        NumberAnimation { duration: Appearance.duration(150) }
                    }

                    Repeater {
                        model: slot.parts

                        Rectangle {
                            id: piece

                            required property int index
                            required property var modelData
                            readonly property real below: {
                                let sum = 0
                                for (let i = 0; i < index; i++)
                                    sum += slot.parts[i].value
                                return sum
                            }
                            readonly property bool last: index === slot.parts.length - 1
                            readonly property real base: below / root.scaleTop * plot.height * root.grown
                            readonly property real full: modelData.value / root.scaleTop * plot.height * root.grown

                            width: bar.width
                            // A 2 px gap of surface between pieces.
                            height: Math.max(0, full - (index > 0 ? root.gap : 0))
                            y: bar.height - base - full
                            visible: height >= 0.5
                            color: modelData.color
                            topLeftRadius: last ? Math.min(4, width / 2) : 0
                            topRightRadius: topLeftRadius

                            Behavior on height {
                                enabled: !grow.running
                                ElementMoveFast {}
                            }
                            Behavior on y {
                                enabled: !grow.running
                                ElementMoveFast {}
                            }
                        }
                    }
                }
            }
        }

        // The tooltip of the bar under the pointer.
        Rectangle {
            id: tip

            readonly property int index: root.hoveredIndex
            readonly property var bucket: index >= 0 ? root.buckets[index] : null
            readonly property var parts: bucket ? root.stack(bucket).slice().sort((a, b) => b.value - a.value).slice(0, 5) : []

            visible: index >= 0
            z: 10
            width: Math.max(150, tipColumn.implicitWidth + 24)
            height: tipColumn.implicitHeight + 20
            radius: Appearance.rounding.small
            color: Appearance.colors.surfaceContainerHighest
            border.width: 1
            border.color: Appearance.colors.layer0Border
            x: {
                const centre = (index + 0.5) * root.slotWidth
                const right = centre + root.barWidth / 2 + 10
                return right + width <= plot.width ? right : Math.max(0, centre - root.barWidth / 2 - 10 - width)
            }
            y: Math.max(0, Math.min(plot.height - height, hover.point.position.y - plot.y - height / 2))

            Column {
                id: tipColumn

                x: 12
                y: 10
                spacing: 4

                StyledText {
                    text: tip.index >= 0 && tip.index + 1 < root.edges.length
                          ? Range.bucketTitle(root.span, root.edges[tip.index], root.edges[tip.index + 1]) : ""
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smaller
                }
                StyledText {
                    text: tip.bucket ? Format.duration(root.valueOf(tip.bucket)) : ""
                    font.pixelSize: Appearance.fontSize.large
                    font.weight: Font.DemiBold
                    tabularNumbers: true
                }
                Repeater {
                    model: root.single ? [] : tip.parts

                    Row {
                        required property var modelData
                        spacing: 8

                        Rectangle {
                            anchors.verticalCenter: parent.verticalCenter
                            width: 10
                            height: 10
                            radius: 3
                            color: modelData.color
                        }
                        StyledText {
                            width: 96
                            text: Categories.name(modelData.id)
                            font.pixelSize: Appearance.fontSize.smaller
                            elide: Text.ElideRight
                        }
                        StyledText {
                            text: Format.duration(modelData.value)
                            color: Appearance.colors.onSurfaceVariant
                            font.pixelSize: Appearance.fontSize.smaller
                            tabularNumbers: true
                        }
                    }
                }
                StyledText {
                    visible: root.clickable && tip.bucket !== null && root.valueOf(tip.bucket) > 0
                    text: qsTr("Click to open the day")
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smallest
                }
            }
        }
    }

    // What each bar is: hours every six, or the days.
    Repeater {
        model: root.buckets.length

        StyledText {
            required property int index

            x: plot.x + index * root.slotWidth + (root.span === "week" ? 0 : -root.slotWidth / 2)
            y: plot.y + plot.height + 4
            width: root.slotWidth
            horizontalAlignment: Text.AlignHCenter
            text: index < root.edges.length ? Range.bucketLabel(root.span, root.edges[index], index) : ""
            color: index === root.nowIndex ? Appearance.colors.primary : Appearance.colors.onSurfaceVariant
            font.pixelSize: Appearance.fontSize.smaller
            font.weight: index === root.nowIndex ? Font.DemiBold : Font.Normal
            tabularNumbers: true
        }
    }
}
