#!/usr/bin/env bash
# Builds PluScreenTime and installs the app and its daemon, which runs as a
# systemd user service. Run ./install.sh --help for the options.
set -euo pipefail

usage() {
    cat <<EOF
Usage: ./install.sh [OPTIONS]

Options:
  --prefix <DIR>   Where to install [default: ~/.local]
                   A system prefix like /usr/local uses sudo for the copying.
  --no-build       Install what is already in target/release
  --no-service     Don't enable and start the daemon's systemd user service
  --uninstall      Remove everything this script installed (keeps the history in
                   ~/.local/share/pluscreentime and settings in ~/.config/pluscreentime)
  -h, --help       Print this help
EOF
}

prefix="$HOME/.local"
build=1
service=1
uninstall=0
while (($#)); do
    case "$1" in
        --prefix) prefix="${2:?--prefix needs a value}"; shift ;;
        --prefix=*) prefix="${1#*=}" ;;
        --no-build) build=0 ;;
        --no-service) service=0 ;;
        --uninstall) uninstall=1 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown option: $1" >&2; usage >&2; exit 1 ;;
    esac
    shift
done

prefix="${prefix%/}"
src="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bindir="$prefix/bin"
appdir="$prefix/share/applications"
icondir="$prefix/share/icons/hicolor/scalable/apps"
unitdir="$prefix/share/systemd/user"  # in systemd's user unit search path for ~/.local, /usr/local and /usr
fontdir="$prefix/share/pluscreentime/fonts"

say() { printf '\033[1;36m::\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }

# Copy into the prefix with sudo only when it isn't ours to write.
sudo=()
probe="$prefix"
while [[ ! -e "$probe" ]]; do probe="$(dirname "$probe")"; done
if [[ ! -w "$probe" ]]; then
    sudo=(sudo)
fi

# The user service can only be managed from the user's own session.
user_systemd=1
if [[ $EUID -eq 0 ]] || ! systemctl --user show-environment &>/dev/null; then
    user_systemd=0
fi

refresh_menus() {
    if command -v update-desktop-database &>/dev/null; then
        "${sudo[@]}" update-desktop-database -q "$appdir" || true
    fi
    if [[ -f "$prefix/share/icons/hicolor/index.theme" ]] && command -v gtk-update-icon-cache &>/dev/null; then
        "${sudo[@]}" gtk-update-icon-cache -q -t -f "$prefix/share/icons/hicolor" || true
    fi
    if command -v kbuildsycoca6 &>/dev/null; then
        kbuildsycoca6 &>/dev/null || true
    fi
}

if ((uninstall)); then
    # Only the service this prefix installed: another prefix's daemon isn't ours to stop.
    if ((user_systemd)) &&
        [[ "$(systemctl --user show -p FragmentPath --value pluscreentimed.service 2>/dev/null)" == "$unitdir/pluscreentimed.service" ]]; then
        say "Stopping the daemon service"
        systemctl --user disable --now pluscreentimed.service || true
    fi
    say "Removing files from $prefix"
    "${sudo[@]}" rm -f "$bindir/pluscreentimed" "$bindir/pluscreentime" "$unitdir/pluscreentimed.service" \
        "$appdir/pluscreentime.desktop" "$icondir/pluscreentime.svg"
    "${sudo[@]}" rm -rf "$prefix/share/pluscreentime"
    if ((user_systemd)); then
        systemctl --user daemon-reload
    fi
    refresh_menus
    say "Done. The history is still in ~/.local/share/pluscreentime, settings in ~/.config/pluscreentime."
    exit 0
fi

if ((build)); then
    if [[ $EUID -eq 0 ]]; then
        warn "building as root; run the script as your user, it asks for sudo when it needs it"
    fi
    if ! command -v cargo &>/dev/null; then
        echo "cargo is missing; install Rust (https://rustup.rs) and a C compiler" >&2
        exit 1
    fi
    say "Building (release)"
    cargo build --release --manifest-path "$src/Cargo.toml" -p pluscreentimed -p pluscreentime
fi

target="$(cargo metadata --format-version 1 --no-deps --manifest-path "$src/Cargo.toml" 2>/dev/null |
    sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p' || true)"
target="${target:-$src/target}/release"
for bin in pluscreentimed pluscreentime; do
    [[ -x "$target/$bin" ]] || { echo "$target/$bin is missing; run without --no-build" >&2; exit 1; }
done

say "Installing to $prefix"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
# `install` replaces files by unlinking them first, so a running daemon is fine.
"${sudo[@]}" install -Dm755 "$target/pluscreentimed" "$bindir/pluscreentimed"
"${sudo[@]}" install -Dm755 "$target/pluscreentime" "$bindir/pluscreentime"
sed "s|@BINDIR@|$bindir|g" "$src/packaging/pluscreentimed.service" >"$tmp/pluscreentimed.service"
"${sudo[@]}" install -Dm644 "$tmp/pluscreentimed.service" "$unitdir/pluscreentimed.service"
"${sudo[@]}" install -Dm644 "$src/packaging/pluscreentime.svg" "$icondir/pluscreentime.svg"
sed "s|@BINDIR@|$bindir|g" "$src/packaging/pluscreentime.desktop" >"$tmp/pluscreentime.desktop"
"${sudo[@]}" install -Dm644 "$tmp/pluscreentime.desktop" "$appdir/pluscreentime.desktop"
refresh_menus

# The interface's two faces, unless the system (or PluSystemMonitor) has them:
# Google Sans Flex for text and Material Symbols Rounded for icons. Pinned
# like PluSystemMonitor pins them.
fetch_font() {
    local family="$1" file="$2" url="$3" sum="$4"
    if fc-list -q "$family" 2>/dev/null || [[ -f "$fontdir/$file" ]] ||
        [[ -f "$prefix/share/plusysmon/fonts/$file" ]]; then
        return
    fi
    if ! command -v curl &>/dev/null; then
        warn "curl is missing, so $family isn't installed; the app falls back to other faces"
        return
    fi
    say "Downloading $family"
    if curl -fsSL "$url" -o "$tmp/$file" && echo "$sum  $tmp/$file" | sha256sum -c --quiet -; then
        "${sudo[@]}" install -Dm644 "$tmp/$file" "$fontdir/$file"
    else
        warn "couldn't download $family; the app falls back to other faces"
    fi
}
fetch_font "Google Sans Flex" GoogleSansFlex.ttf \
    "https://raw.githubusercontent.com/google/fonts/a0e3dbcdc3a3ecfafff3f071159ae0221628922d/ofl/googlesansflex/GoogleSansFlex%5BGRAD,ROND,opsz,slnt,wdth,wght%5D.ttf" \
    c31a482fbecbf2e07e6890134d20078723aadf732c9b9c6c9a44f86f8265b6fe
fetch_font "Material Symbols Rounded" MaterialSymbolsRounded.ttf \
    "https://raw.githubusercontent.com/google/material-design-icons/737e3324305806514d7909874fa1818ae1808232/variablefont/MaterialSymbolsRounded%5BFILL,GRAD,opsz,wght%5D.ttf" \
    95b24392bb49efd1bc3e92cff4e2452ad094461bab7c97e7d8723fab97e330ca
if [[ -f "$fontdir/GoogleSansFlex.ttf" && ! -f "$fontdir/OFL.txt" ]] && command -v curl &>/dev/null; then
    curl -fsSL "https://raw.githubusercontent.com/google/fonts/a0e3dbcdc3a3ecfafff3f071159ae0221628922d/ofl/googlesansflex/OFL.txt" \
        -o "$tmp/OFL.txt" && "${sudo[@]}" install -Dm644 "$tmp/OFL.txt" "$fontdir/OFL.txt" || true
fi

if ((service)); then
    if ((user_systemd)); then
        say "Starting the daemon (systemd user service pluscreentimed)"
        systemctl --user daemon-reload
        systemctl --user enable pluscreentimed.service
        # Restart rather than start, so an update runs the new binary.
        systemctl --user restart pluscreentimed.service
        sleep 1
        if [[ "$(systemctl --user is-active pluscreentimed.service)" != active ]]; then
            warn "the daemon didn't come up; see: journalctl --user -u pluscreentimed"
        fi
    else
        warn "can't reach your systemd user session (running as root?). Start the daemon as your user with:"
        echo "    systemctl --user daemon-reload && systemctl --user enable --now pluscreentimed"
    fi
fi

say "Done. PluScreenTime is in your application menu (or run $bindir/pluscreentime)."
echo "The daemon follows Hyprland and KDE Plasma; it starts counting once one of them runs."
case ":$PATH:" in
    *":$bindir:"*) ;;
    *) warn "$bindir is not in your PATH, so 'pluscreentime' only works with the full path" ;;
esac
