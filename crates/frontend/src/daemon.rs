//! The `ScreenTime` QML singleton: questions to the daemon, answered as JSON.
//!
//! The daemon is on this machine and answers from its own database in a few
//! milliseconds, so each call waits for its answer. A call that fails (the
//! daemon isn't running) returns "" and sets `connected` false; the next one
//! tries again.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        #[qproperty(bool, connected)]
        /// Why the last call failed, or "".
        #[qproperty(QString, problem)]
        type ScreenTime = super::ScreenTimeRust;

        /// `Status` (see the protocol), or "".
        #[qinvokable]
        fn status(self: Pin<&mut Self>) -> QString;

        /// `Summary` of the range `edges` (a JSON array of times) cuts up.
        #[qinvokable]
        fn summary(self: Pin<&mut Self>, edges: &QString) -> QString;

        /// `AppDetail` of app `app` over `edges`.
        #[qinvokable]
        fn app_detail(self: Pin<&mut Self>, app: f64, edges: &QString) -> QString;

        /// `AppList`: every app, with its time over all history.
        #[qinvokable]
        fn apps(self: Pin<&mut Self>) -> QString;

        /// Puts the app in `category`, or back in its own with "". Returns
        /// the app as it is now.
        #[qinvokable]
        fn set_category(self: Pin<&mut Self>, app: f64, category: &QString) -> QString;

        /// Sends `Settings` as JSON; returns them as kept.
        #[qinvokable]
        fn set_settings(self: Pin<&mut Self>, json: &QString) -> QString;
    }
}

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::pin::Pin;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use pluscreentime_protocol::{Request, Settings};

const TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Default)]
pub struct ScreenTimeRust {
    connected: bool,
    problem: QString,
    stream: Option<BufReader<UnixStream>>,
}

impl ScreenTimeRust {
    fn connect() -> Result<BufReader<UnixStream>> {
        let path = pluscreentime_protocol::socket_path().context("XDG_RUNTIME_DIR isn't set")?;
        let stream = UnixStream::connect(&path).map_err(|e| {
            anyhow!("the daemon isn't running ({e}); start it with: systemctl --user start pluscreentimed")
        })?;
        stream.set_read_timeout(Some(TIMEOUT))?;
        stream.set_write_timeout(Some(TIMEOUT))?;
        Ok(BufReader::new(stream))
    }

    fn exchange(stream: &mut BufReader<UnixStream>, line: &str) -> Result<String> {
        let socket = stream.get_mut();
        socket.write_all(line.as_bytes())?;
        socket.write_all(b"\n")?;
        let mut answer = String::new();
        if stream.read_line(&mut answer)? == 0 {
            bail!("the daemon closed the connection");
        }
        Ok(answer)
    }

    /// The answer's `ok` value, as JSON.
    fn ask(&mut self, request: &Request) -> Result<String> {
        let line = serde_json::to_string(request)?;
        // A connection kept from before may have gone with an old daemon;
        // one fresh try then.
        let answer = match self.stream.as_mut().map(|s| Self::exchange(s, &line)) {
            Some(Ok(answer)) => answer,
            _ => {
                self.stream = None;
                let mut stream = Self::connect()?;
                let answer = Self::exchange(&mut stream, &line)?;
                self.stream = Some(stream);
                answer
            }
        };
        let mut value: serde_json::Value =
            serde_json::from_str(&answer).context("the daemon's answer makes no sense")?;
        if let Some(error) = value.get("error").and_then(|e| e.as_str()) {
            bail!("{error}");
        }
        Ok(value
            .get_mut("ok")
            .map(serde_json::Value::take)
            .unwrap_or_default()
            .to_string())
    }
}

fn edges(json: &QString) -> Result<Vec<i64>> {
    let edges: Vec<f64> = serde_json::from_str(&json.to_string()).context("malformed edges")?;
    Ok(edges.into_iter().map(|e| e as i64).collect())
}

impl qobject::ScreenTime {
    fn call(mut self: Pin<&mut Self>, request: Result<Request>) -> QString {
        let result = request.and_then(|request| self.as_mut().rust_mut().ask(&request));
        match result {
            Ok(json) => {
                self.as_mut().set_connected(true);
                self.as_mut().set_problem(QString::default());
                QString::from(&json)
            }
            Err(e) => {
                // A refused request still means the daemon is there.
                let reached = self.rust().stream.is_some();
                self.as_mut().set_connected(reached);
                self.as_mut().set_problem(QString::from(&format!("{e:#}")));
                QString::default()
            }
        }
    }

    pub fn status(self: Pin<&mut Self>) -> QString {
        self.call(Ok(Request::Status))
    }

    pub fn summary(self: Pin<&mut Self>, json: &QString) -> QString {
        let request = edges(json).map(|edges| Request::Summary { edges });
        self.call(request)
    }

    pub fn app_detail(self: Pin<&mut Self>, app: f64, json: &QString) -> QString {
        let request = edges(json).map(|edges| Request::AppDetail { app: app as i64, edges });
        self.call(request)
    }

    pub fn apps(self: Pin<&mut Self>) -> QString {
        self.call(Ok(Request::Apps))
    }

    pub fn set_category(self: Pin<&mut Self>, app: f64, category: &QString) -> QString {
        let category = Some(category.to_string()).filter(|c| !c.is_empty());
        self.call(Ok(Request::SetCategory { app: app as i64, category }))
    }

    pub fn set_settings(self: Pin<&mut Self>, json: &QString) -> QString {
        let request = serde_json::from_str::<Settings>(&json.to_string())
            .context("malformed settings")
            .map(|settings| Request::SetSettings { settings });
        self.call(request)
    }
}
