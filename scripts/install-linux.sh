#!/usr/bin/env bash
#
# Build and install Я.Мяу for the current user on Linux.
#
# Usage:
#   ./scripts/install-linux.sh
#   ./scripts/install-linux.sh --prefix "$HOME/.local"
#   ./scripts/install-linux.sh --no-build
#   ./scripts/install-linux.sh --uninstall
#
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
BUILD=1
UNINSTALL=0

usage() {
    cat <<'USAGE'
Build and install Я.Мяу for the current user.

Options:
  --prefix PATH   Installation prefix (default: $HOME/.local)
  --no-build      Use an existing target/release/rsyameow
  --uninstall     Remove the installed binary, icon and desktop entry
  -h, --help      Show this help

Files installed:
  $PREFIX/bin/rsyameow
  $PREFIX/share/applications/rsyameow.desktop
  $PREFIX/share/icons/hicolor/scalable/apps/rsyameow.svg
USAGE
}

while (($#)); do
    case "$1" in
        --prefix)
            [[ $# -ge 2 ]] || { echo "install: --prefix requires a path" >&2; exit 2; }
            PREFIX="$2"
            shift 2
            ;;
        --prefix=*)
            PREFIX="${1#*=}"
            shift
            ;;
        --no-build)
            BUILD=0
            shift
            ;;
        --uninstall)
            UNINSTALL=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "install: unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

PREFIX="${PREFIX%/}"
BIN_DIR="$PREFIX/bin"
APP_DIR="$PREFIX/share/applications"
ICON_DIR="$PREFIX/share/icons/hicolor/scalable/apps"
BIN="$BIN_DIR/rsyameow"
DESKTOP="$APP_DIR/rsyameow.desktop"
ICON="$ICON_DIR/rsyameow.svg"

refresh_caches() {
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APP_DIR" >/dev/null 2>&1 || true
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true
    fi
}

if ((UNINSTALL)); then
    rm -f "$BIN" "$DESKTOP" "$ICON"
    refresh_caches
    echo "Я.Мяу удалён из $PREFIX"
    exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "install: cargo not found; установите Rust и повторите запуск" >&2
    exit 1
fi

if ((BUILD)); then
    echo "Собираю релизную версию…"
    (cd "$ROOT" && cargo build --release --locked -p rsyameow)
fi

if [[ ! -x "$ROOT/target/release/rsyameow" ]]; then
    echo "install: $ROOT/target/release/rsyameow не найден; запустите без --no-build" >&2
    exit 1
fi

install -Dm755 "$ROOT/target/release/rsyameow" "$BIN"
install -Dm644 "$ROOT/packaging/linux/rsyameow.svg" "$ICON"

# Substitute the absolute executable path into the desktop entry. Quoted Exec
# keeps the entry valid even when the home directory contains spaces.
tmp_desktop="$(mktemp)"
trap 'rm -f "$tmp_desktop"' EXIT
sed "s|@EXEC@|$BIN|g" "$ROOT/packaging/linux/rsyameow.desktop.in" >"$tmp_desktop"
install -Dm644 "$tmp_desktop" "$DESKTOP"
rm -f "$tmp_desktop"
trap - EXIT

refresh_caches

echo "Готово: Я.Мяу установлен."
echo "  binary: $BIN"
echo "  desktop: $DESKTOP"
echo "  icon: $ICON"

case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *)
        echo
        echo "Добавьте $BIN_DIR в PATH, если хотите запускать rsyameow из терминала:"
        echo "  export PATH=\"$BIN_DIR:\$PATH\""
        ;;
esac
