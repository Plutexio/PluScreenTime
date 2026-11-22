pragma Singleton
import QtQuick
import PluScreenTime

// The categories apps fall in, in the order their colours are given out
// (as the daemon's protocol lists them). "Other" is the neutral one.
QtObject {
    readonly property var list: [
        { id: "web", name: qsTr("Web"), symbol: "language", shape: "cookie9" },
        { id: "creativity", name: qsTr("Creativity"), symbol: "brush", shape: "softBoom" },
        { id: "productivity", name: qsTr("Productivity"), symbol: "work", shape: "clover4" },
        { id: "utilities", name: qsTr("Utilities"), symbol: "build", shape: "clover8" },
        { id: "social", name: qsTr("Social"), symbol: "forum", shape: "flower" },
        { id: "games", name: qsTr("Games"), symbol: "sports_esports", shape: "puffy" },
        { id: "development", name: qsTr("Development"), symbol: "code", shape: "cookie7" },
        { id: "entertainment", name: qsTr("Entertainment"), symbol: "movie", shape: "sunny" },
        { id: "other", name: qsTr("Other"), symbol: "category", shape: "pentagon" }
    ]

    readonly property var ids: list.map(c => c.id)

    function info(id) {
        return list.find(c => c.id === id) ?? list[list.length - 1]
    }

    function name(id) {
        return info(id).name
    }

    function color(id) {
        const colors = Appearance.colors
        return id !== "other" && colors[id] !== undefined ? colors[id] : colors.outline
    }

    /// Where it goes in a stack: the fixed order, never by size.
    function order(id) {
        const i = ids.indexOf(id)
        return i < 0 ? ids.length : i
    }
}
