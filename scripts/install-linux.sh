#!/usr/bin/env bash
# Install a locally built Photos into the user's home so the desktop menu finds it.
#
#   scripts/install-linux.sh [--prefix ~/.local] [--uninstall]
#
# Layout, which is what Tauri looks for on Linux (exe_dir/../lib/<product>):
#   <prefix>/bin/<product>              the binary
#   <prefix>/lib/<product>/models       the AI models
#   <prefix>/lib/<product>/ffmpeg       the ffmpeg sidecars
#   <prefix>/share/applications/<id>.desktop
#   <prefix>/share/icons/hicolor/512x512/apps/<id>.png
#
# Build first: cd src-vite && pnpm build; cd ../src-tauri && cargo build --release
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PREFIX="${HOME}/.local"
UNINSTALL=0
while [ $# -gt 0 ]; do
  case "$1" in
    --prefix) PREFIX="$2"; shift 2 ;;
    --uninstall) UNINSTALL=1; shift ;;
    -h|--help) sed -n '2,14p' "$0" | sed 's/^# \?//'; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

conf="$ROOT/src-tauri/tauri.conf.json"
PRODUCT="$(python3 -c "import json;print(json.load(open('$conf'))['productName'])")"
APP_ID="$(python3 -c "import json;print(json.load(open('$conf'))['identifier'])")"
DESKTOP="$PREFIX/share/applications/$APP_ID.desktop"
ICON="$PREFIX/share/icons/hicolor/512x512/apps/$APP_ID.png"

if [ "$UNINSTALL" = 1 ]; then
  rm -f "$PREFIX/bin/$PRODUCT" "$DESKTOP" "$ICON"
  rm -rf "${PREFIX:?}/lib/$PRODUCT"
  command -v update-desktop-database >/dev/null && update-desktop-database "$PREFIX/share/applications" || true
  echo "removed $PRODUCT from $PREFIX"
  exit 0
fi

BIN="$ROOT/src-tauri/target/release/$PRODUCT"
[ -x "$BIN" ] || { echo "no binary at $BIN; build it first (see the header of this script)" >&2; exit 1; }

install -Dm755 "$BIN" "$PREFIX/bin/$PRODUCT"
for res in models ffmpeg; do
  src="$ROOT/src-tauri/resources/$res"
  if [ -d "$src" ]; then
    mkdir -p "$PREFIX/lib/$PRODUCT/$res"
    cp -f "$src"/* "$PREFIX/lib/$PRODUCT/$res/"
    chmod +x "$PREFIX/lib/$PRODUCT/$res"/* 2>/dev/null || true
  else
    echo "warning: $src is missing; run scripts/download_models.sh and scripts/download_ffmpeg_sidecar.sh" >&2
  fi
done

install -Dm644 "$ROOT/src-tauri/icons/icon.png" "$ICON"
mkdir -p "$(dirname "$DESKTOP")"
cat > "$DESKTOP" <<EOF
[Desktop Entry]
Type=Application
Name=$PRODUCT
GenericName=Photo Manager
Comment=Browse, search and organise your photo library
Exec=$PREFIX/bin/$PRODUCT %F
Icon=$APP_ID
Terminal=false
Categories=Graphics;Photography;Viewer;
Keywords=photo;photos;image;gallery;picture;album;library;
MimeType=image/jpeg;image/png;image/heif;image/webp;image/tiff;image/gif;video/mp4;video/quicktime;
StartupWMClass=$PRODUCT
StartupNotify=true
EOF
chmod 644 "$DESKTOP"
command -v update-desktop-database >/dev/null && update-desktop-database "$PREFIX/share/applications" || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -qtf "$PREFIX/share/icons/hicolor" 2>/dev/null || true

echo "installed $PRODUCT:"
echo "  binary    $PREFIX/bin/$PRODUCT"
echo "  resources $PREFIX/lib/$PRODUCT"
echo "  launcher  $DESKTOP"
case ":$PATH:" in *":$PREFIX/bin:"*) ;; *) echo "  note: $PREFIX/bin is not on your PATH" ;; esac
