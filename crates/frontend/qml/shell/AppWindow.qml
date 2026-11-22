import QtQuick
import PluScreenTime

// The frameless window on layer0 with its edges, the preferences and the
// theme. The app puts its drawer and sheet inside (they land in the frame)
// and its dialogs beside them.
Window {
    id: window

    property string appName: "PluScreenTime"
    property var prefs: JSON.parse(Config.preferences())
    property int page: prefs.page ?? 0
    /// Where messages show; the app's sheet holds it.
    property Snackbar snackbar: null
    readonly property bool maximized: visibility === Window.Maximized || visibility === Window.FullScreen
    readonly property real gutter: Appearance.sizes.gutter
    readonly property bool systemDark: Application.styleHints.colorScheme !== Qt.ColorScheme.Light
    readonly property bool wantsDark: prefs.theme_mode === "light" ? false : prefs.theme_mode === "dark" ? true : systemDark

    readonly property alias frame: frame
    readonly property alias appearanceDialog: appearanceDialog
    default property alias content: frame.data

    /// `--open <name>` on the command line, for screenshots of a dialog.
    signal openRequested(string name)

    function savePrefs() {
        Config.savePreferences(JSON.stringify(prefs))
    }

    function setPref(key, value) {
        prefs[key] = value
        prefs = Object.assign({}, prefs)
        savePrefs()
    }

    function applyScheme() {
        Appearance.applyScheme(JSON.parse(Config.scheme(prefs.theme, wantsDark)))
    }

    // Crossfade: freeze the current look in a snapshot, switch underneath,
    // fade the snapshot out.
    function changeTheme(change) {
        if (Appearance.performanceMode) {
            change()
            applyScheme()
            return
        }
        const dpr = Screen.devicePixelRatio
        window.contentItem.grabToImage(result => {
            snapshot.source = result.url
            snapshot.opacity = 1
            change()
            applyScheme()
            fade.restart()
        }, Qt.size(window.width * dpr, window.height * dpr))
    }

    width: 1280
    height: 860
    minimumWidth: 820
    minimumHeight: 560
    visible: true
    flags: Qt.Window | Qt.FramelessWindowHint
    color: "transparent"
    title: appName

    onPageChanged: {
        if (prefs.page !== page)
            setPref("page", page)
    }
    onWantsDarkChanged: applyScheme()

    Binding {
        target: Appearance
        property: "reduceMotionSetting"
        value: window.prefs.reduce_motion ?? false
    }
    Binding {
        target: Appearance
        property: "performanceMode"
        value: window.prefs.performance_mode ?? false
    }
    Binding {
        target: Appearance
        property: "noShapes"
        value: window.prefs.no_shapes ?? false
    }

    // The window itself: layer0, rounded unless maximised.
    Rectangle {
        id: frame

        anchors.fill: parent
        radius: window.maximized ? 0 : Appearance.rounding.window
        color: Appearance.colors.layer0
        border.width: window.maximized ? 0 : 1
        border.color: Appearance.colors.layer0Border

        WindowEdges {
            anchors.fill: parent
            visible: !window.maximized
            window: window
            z: 100
        }
    }

    AppearanceDialog {
        id: appearanceDialog
        prefs: window.prefs
        dark: window.wantsDark
        onThemeSelected: id => window.changeTheme(() => window.setPref("theme", id))
        onModeSelected: mode => window.changeTheme(() => window.setPref("theme_mode", mode))
        onPreferenceEdited: (key, value) => window.setPref(key, value)
    }

    Connections {
        target: Config
        function onErrorOccurred(message) { window.snackbar?.show(message) }
        function onSchemeFilesChanged() { window.applyScheme() }
    }

    Image {
        id: snapshot

        anchors.fill: parent
        z: 100
        opacity: 0
        visible: opacity > 0

        NumberAnimation on opacity {
            id: fade
            running: false
            to: 0
            duration: 450
            easing.type: Easing.InOutQuad
            onFinished: snapshot.source = ""
        }
    }

    Component.onCompleted: applyScheme()

    // `--open <dialog>`: open a dialog on start, for screenshots of it.
    Timer {
        readonly property int flag: Qt.application.arguments.indexOf("--open")

        running: flag > 0 && flag + 1 < Qt.application.arguments.length
        interval: 1500
        onTriggered: {
            const name = Qt.application.arguments[flag + 1]
            if (name === "appearance")
                appearanceDialog.open()
            else
                window.openRequested(name)
        }
    }

    // `--screenshot <file.png>`: let data and animations settle, save the window, quit.
    Timer {
        readonly property int flag: Qt.application.arguments.indexOf("--screenshot")

        running: flag > 0 && flag + 1 < Qt.application.arguments.length
        interval: 4000
        onTriggered: window.contentItem.grabToImage(result => {
            result.saveToFile(Qt.application.arguments[flag + 1])
            Qt.quit()
        })
    }
}
