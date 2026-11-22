import QtQuick
import PluScreenTime

// A card on the sheet (layer2): a heading with its symbol on a shape and a
// squiggle running on to whatever sits at its end, then the content stacked
// below it.
Item {
    id: root

    property string title
    property string symbol
    property string shape: "cookie9"
    property alias trailing: header.trailing
    default property alias content: body.data
    property real padding: 20
    property real spacing: 14

    implicitHeight: body.y + body.implicitHeight + padding

    Rectangle {
        anchors.fill: parent
        radius: Appearance.rounding.large
        color: Appearance.colors.layer2
    }

    SectionHeader {
        id: header
        x: root.padding
        y: root.padding - 4
        width: parent.width - 2 * root.padding
        visible: root.title.length > 0
        title: root.title
        symbol: root.symbol
        iconShape: root.shape
        titleSize: Appearance.fontSize.larger
    }

    Column {
        id: body
        x: root.padding
        y: header.visible ? header.y + header.height + root.spacing - 4 : root.padding
        width: parent.width - 2 * root.padding
        spacing: root.spacing
    }
}
