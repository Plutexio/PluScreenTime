mod apps;
mod config;
mod desktop;
mod hyprland;
mod icons;
mod idle;
mod kwin;
mod lock;
mod queries;
mod server;
mod steam;
mod store;
mod tracker;

use std::io::IsTerminal;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::sync::mpsc;
use tracing::info;

use tracker::{Tracker, now_ms};

const USAGE: &str = "\
Usage: pluscreentimed [OPTIONS]

Follows the focused window on Hyprland or KDE Plasma and keeps the history of
screen time, which the PluScreenTime app shows.

Options:
  --database <FILE>  Where the history is kept
                     [default: ~/.local/share/pluscreentime/history.db]
  --socket <PATH>    Where the app asks [default: $XDG_RUNTIME_DIR/pluscreentime.sock]
  --version          Print the version and exit
  -h, --help         Print this help";

struct Args {
    database: PathBuf,
    socket: PathBuf,
}

fn parse_args() -> Result<Args> {
    let mut database = pluscreentime_protocol::data_dir().map(|dir| dir.join("history.db"));
    let mut socket = pluscreentime_protocol::socket_path();
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().with_context(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--database" => database = Some(value()?.into()),
            "--socket" => socket = Some(value()?.into()),
            "--version" => {
                println!(
                    "pluscreentimed {} (protocol {})",
                    env!("CARGO_PKG_VERSION"),
                    pluscreentime_protocol::VERSION
                );
                std::process::exit(0);
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other => bail!("unknown argument {other}\n\n{USAGE}"),
        }
    }
    Ok(Args {
        database: database.context("can't tell where the history goes; pass --database")?,
        socket: socket.context("XDG_RUNTIME_DIR isn't set; pass --socket")?,
    })
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_ansi(std::io::stdout().is_terminal())
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("PLUSCREENTIMED_LOG")
                .unwrap_or_else(|_| "info".into()),
        )
        .init();
    let args = parse_args()?;

    let store = store::Store::open(&args.database)?;
    info!("history: {}", args.database.display());
    let settings_path = pluscreentime_protocol::config_dir().map(|dir| dir.join("daemon.toml"));
    let settings = settings_path
        .as_deref()
        .map(config::load)
        .unwrap_or_default();
    let idle_timeout = Arc::new(AtomicU32::new(settings.idle_timeout_secs));
    let mut tracker = Tracker::new(store, settings, settings_path, idle_timeout.clone(), now_ms());

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = server::bind(&args.socket)?;
            info!("listening on {}", args.socket.display());

            let (events_tx, mut events) = mpsc::channel(256);
            let (queries_tx, mut queries) = mpsc::channel::<server::Query>(32);
            tokio::spawn(hyprland::run(events_tx.clone()));
            tokio::spawn(kwin::run(events_tx.clone()));
            tokio::spawn(lock::run(events_tx.clone()));
            idle::spawn(events_tx.clone(), idle_timeout);
            let server = tokio::spawn(server::serve(listener, queries_tx));

            let mut ticker = tokio::time::interval(Duration::from_secs(1));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            let shutdown = shutdown_signal();
            tokio::pin!(shutdown);
            let result = loop {
                tokio::select! {
                    Some(event) = events.recv() => tracker.handle(event, now_ms()),
                    Some(query) = queries.recv() => {
                        let _ = query.reply.send(tracker.answer(query.request, now_ms()));
                    }
                    _ = ticker.tick() => tracker.tick(now_ms()),
                    _ = &mut shutdown => {
                        info!("shutting down");
                        break Ok(());
                    }
                }
                if server.is_finished() {
                    break server.await.context("the server stopped")?;
                }
            };
            tracker.shutdown(now_ms());
            let _ = std::fs::remove_file(&args.socket);
            result
        })
}

async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};
    let mut term = signal(SignalKind::terminate()).expect("installing SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
}
