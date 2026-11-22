pragma Singleton
import QtQuick

// Turning times and durations into text the same way everywhere.
QtObject {
    readonly property var dayNames: [qsTr("Sunday"), qsTr("Monday"), qsTr("Tuesday"), qsTr("Wednesday"),
                                     qsTr("Thursday"), qsTr("Friday"), qsTr("Saturday")]
    readonly property var shortDayNames: [qsTr("Sun"), qsTr("Mon"), qsTr("Tue"), qsTr("Wed"), qsTr("Thu"),
                                          qsTr("Fri"), qsTr("Sat")]
    readonly property var monthNames: [qsTr("Jan"), qsTr("Feb"), qsTr("Mar"), qsTr("Apr"), qsTr("May"),
                                       qsTr("Jun"), qsTr("Jul"), qsTr("Aug"), qsTr("Sep"), qsTr("Oct"),
                                       qsTr("Nov"), qsTr("Dec")]

    // "3 h 12 min", "45 min", "40 s": hours and minutes, seconds only for
    // less than a minute.
    function duration(ms) {
        const s = Math.max(0, Math.floor(ms / 1000))
        if (s < 60)
            return s + " s"
        const h = Math.floor(s / 3600)
        const m = Math.floor((s % 3600) / 60)
        if (h === 0)
            return m + " min"
        return m === 0 ? h + " h" : h + " h " + m + " min"
    }

    // "3:12": for tight spots.
    function compact(ms) {
        const m = Math.round(Math.max(0, ms) / 60000)
        if (m < 60)
            return m + "m"
        return Math.floor(m / 60) + "h" + (m % 60 > 0 ? " " + (m % 60) + "m" : "")
    }

    function pad(n) {
        return n < 10 ? "0" + n : "" + n
    }

    // "14:05"
    function clock(ms) {
        const d = new Date(ms)
        return pad(d.getHours()) + ":" + pad(d.getMinutes())
    }

    // "Sat 21 Nov"
    function shortDate(ms) {
        const d = new Date(ms)
        return shortDayNames[d.getDay()] + " " + d.getDate() + " " + monthNames[d.getMonth()]
    }

    // "21 Nov", with the year when it isn't this one.
    function dayMonth(ms) {
        const d = new Date(ms)
        const year = d.getFullYear() !== new Date().getFullYear() ? " " + d.getFullYear() : ""
        return d.getDate() + " " + monthNames[d.getMonth()] + year
    }

    // "just now", "5 min ago", "Yesterday", "21 Nov"
    function ago(ms) {
        const now = Date.now()
        const minutes = Math.floor((now - ms) / 60000)
        if (minutes < 1)
            return qsTr("Just now")
        if (minutes < 60)
            return qsTr("%1 min ago").arg(minutes)
        const today = new Date(); today.setHours(0, 0, 0, 0)
        if (ms >= today.getTime())
            return qsTr("Today, %1").arg(clock(ms))
        const yesterday = new Date(today); yesterday.setDate(today.getDate() - 1)
        if (ms >= yesterday.getTime())
            return qsTr("Yesterday, %1").arg(clock(ms))
        return dayMonth(ms)
    }

    function percent(part, whole) {
        if (whole <= 0)
            return "0%"
        const p = part / whole * 100
        return (p > 0 && p < 1 ? "<1" : Math.round(p)) + "%"
    }
}
