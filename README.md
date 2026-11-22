# PluScreenTime

How long you spent at the screen, and in which apps. A small daemon follows
the focused window and keeps the history; a Material 3 (Qt Quick) app shows
it, in the style of [PluSystemMonitor](https://github.com/Plutexio/PluSystemMonitor).

## What it can do

- **Day and week**: the total and how it compares to the day or week before,
  time per hour or per day stacked by category, pickups and the longest
  session. A day also shows its sessions on a timeline; a week's day opens
  with a click.
- **Apps**: where the time went, app by app, over a day, a week or all
  history. Pick one to see its own chart, how often you switched to it, its
  longest stretch and the window titles it spent the time under.
- **Categories**: Web, Social, Games, Development and others, found from the
  apps' desktop entries and changeable per app.
- **Only time you were there**: after a few minutes without input (5 by
  default) the time stops counting, from the last input on. Videos and games
  that keep the screen awake still count; a locked screen or a sleeping
  machine never does.
- **Themes**: follows your wallpaper's colours (Matugen) or picks from
  Catppuccin, Gruvbox, Nord, Tokyo Night and others, or your own.

It follows **Hyprland** (through its IPC socket) and **KDE Plasma 6** (through
a small KWin script the daemon loads). Idle time comes from the compositor
(`ext-idle-notify-v1`).

## Installing

You need Rust, a C compiler and Qt 6.6+ (`qt6-base`, `qt6-declarative`,
`qt6-svg` for SVG icons).

```sh
./install.sh
```

It installs everything to `~/.local` and starts the daemon as a systemd user
service. Open **PluScreenTime** from the application menu. Run `./install.sh`
again to update, and `./install.sh --uninstall` to remove it (the history
stays). `./install.sh --help` lists the other options.

## Where things are

- History: `~/.local/share/pluscreentime/history.db` (SQLite)
- Daemon settings: `~/.config/pluscreentime/daemon.toml` (set from the app)
- App preferences: `~/.config/pluscreentime/app.toml`
- Themes of your own: `~/.config/pluscreentime/themes/`
- The daemon's socket: `$XDG_RUNTIME_DIR/pluscreentime.sock`

Everything stays on this machine. Window titles can be turned off under
**Tracking**.

## Development

```sh
cargo run --release -p pluscreentimed   # daemon
cargo run --release -p pluscreentime    # app, asks the daemon over its socket
cargo test
```

`PLUSCREENTIMED_LOG=debug` makes the daemon say which window it sees. The
workspace has the daemon (`crates/daemon`), the app (`crates/frontend`) and
what they say to each other (`crates/protocol`): a line of JSON each way, so
`socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/pluscreentime.sock` and
`{"type":"status"}` work too.
