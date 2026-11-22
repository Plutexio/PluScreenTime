pragma Singleton
import QtQuick
import PluScreenTime

// The spans the app looks at, worked out in local time: a day cut into its
// hours, a week (Monday first) into its days. Edges come from the calendar,
// not from adding milliseconds, so days with a daylight saving change have
// 23 or 25 hours and still line up.
QtObject {
    function dayStart(ms) {
        const d = new Date(ms)
        d.setHours(0, 0, 0, 0)
        return d.getTime()
    }

    function weekStart(ms) {
        const d = new Date(dayStart(ms))
        d.setDate(d.getDate() - (d.getDay() + 6) % 7)
        return d.getTime()
    }

    function start(span, anchor) {
        return span === "week" ? weekStart(anchor) : dayStart(anchor)
    }

    /// The same span `steps` back (negative) or on.
    function shifted(span, anchor, steps) {
        const d = new Date(start(span, anchor))
        d.setDate(d.getDate() + steps * (span === "week" ? 7 : 1))
        return d.getTime()
    }

    function end(span, anchor) {
        return shifted(span, anchor, 1)
    }

    /// Ascending times cutting the span into hours or days.
    function edges(span, anchor) {
        const first = new Date(start(span, anchor))
        const out = []
        const count = span === "week" ? 7 : 24
        for (let i = 0; i <= count; i++) {
            const d = new Date(first)
            if (span === "week")
                d.setDate(d.getDate() + i)
            else
                d.setHours(i)
            const t = d.getTime()
            // A skipped hour gives the same time twice.
            if (out.length === 0 || t > out[out.length - 1])
                out.push(t)
        }
        return out
    }

    function contains(span, anchor, ms) {
        return ms >= start(span, anchor) && ms < end(span, anchor)
    }

    function isCurrent(span, anchor) {
        return contains(span, anchor, Date.now())
    }

    /// "Today", "Yesterday", "Saturday", "Sat 14 Nov"; "This Week", "Last
    /// Week", "9 – 15 Nov".
    function title(span, anchor) {
        const now = Date.now()
        if (span === "week") {
            const thisWeek = weekStart(now)
            const first = weekStart(anchor)
            if (first === thisWeek)
                return qsTr("This Week")
            if (first === shifted("week", now, -1))
                return qsTr("Last Week")
            const last = shifted("day", end("week", anchor), -1)
            const a = new Date(first), b = new Date(last)
            if (a.getMonth() === b.getMonth())
                return a.getDate() + " – " + Format.dayMonth(last)
            return Format.dayMonth(first) + " – " + Format.dayMonth(last)
        }
        const day = dayStart(anchor)
        const today = dayStart(now)
        if (day === today)
            return qsTr("Today")
        if (day === shifted("day", now, -1))
            return qsTr("Yesterday")
        if (day > shifted("day", now, -7))
            return Format.dayNames[new Date(day).getDay()]
        return Format.shortDate(day)
    }

    /// What a bucket is called under its bar.
    function bucketLabel(span, edge, index) {
        if (span === "week")
            return Format.shortDayNames[new Date(edge).getDay()]
        const hour = new Date(edge).getHours()
        return hour % 6 === 0 ? Format.pad(hour) : ""
    }

    /// What a bucket is called in its tooltip.
    function bucketTitle(span, from, to) {
        if (span === "week")
            return Format.dayNames[new Date(from).getDay()] + ", " + Format.dayMonth(from)
        return Format.clock(from) + " – " + Format.clock(to)
    }
}
