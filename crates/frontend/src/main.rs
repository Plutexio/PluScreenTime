mod config;
mod daemon;
mod paths;
mod prefs;
mod theme;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    let mut app = QGuiApplication::new();
    if let Some(app) = app.as_mut() {
        app.set_application_name(&QString::from("PluScreenTime"));
    }
    // The installed `.desktop` file, so Wayland compositors show its icon.
    QGuiApplication::set_desktop_file_name(&QString::from("pluscreentime"));

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/PluScreenTime/qml/Main.qml"));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
