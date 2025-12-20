#!/usr/bin/env bash
set -euo pipefail

APP_NAME="LaReview"
BUNDLE_ID="io.github.puemos.lareview"
TARGET="${1:-$(rustc -vV | sed -n 's/^host: //p')}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "${ROOT_DIR}"

cargo build --release --target "${TARGET}"

BIN_SRC="target/${TARGET}/release/lareview"
APP_DIR="target/${TARGET}/release/${APP_NAME}.app"
BIN_DST="${APP_DIR}/Contents/MacOS/${APP_NAME}-bin"
WRAPPER_DST="${APP_DIR}/Contents/MacOS/${APP_NAME}"
RES_DIR="${APP_DIR}/Contents/Resources"

rm -rf "${APP_DIR}"
mkdir -p "$(dirname "${BIN_DST}")" "${RES_DIR}"

cp "${BIN_SRC}" "${BIN_DST}"
chmod +x "${BIN_DST}"

cat > "${WRAPPER_DST}" << 'EOF_WRAPPER'
#!/bin/sh
DIR="$(cd "$(dirname "$0")" && pwd)"
PATH="$(/usr/libexec/path_helper -s | awk -F'"' '/PATH=/{print $2}')"
if [ -z "$PATH" ]; then
  PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
fi
if [ -n "${LAREVIEW_EXTRA_PATH:-}" ]; then
  PATH="${LAREVIEW_EXTRA_PATH}:$PATH"
fi
export PATH
exec "$DIR/LaReview-bin" "$@"
EOF_WRAPPER
chmod +x "${WRAPPER_DST}"

VERSION="$(cargo pkgid | sed 's/.*#//')"

cat > "${APP_DIR}/Contents/Info.plist" << EOF_PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>

  <key>CFBundleDisplayName</key>
  <string>${APP_NAME}</string>

  <key>CFBundleExecutable</key>
  <string>${APP_NAME}</string>

  <key>CFBundleIdentifier</key>
  <string>${BUNDLE_ID}</string>

  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>

  <key>CFBundleName</key>
  <string>${APP_NAME}</string>

  <key>CFBundlePackageType</key>
  <string>APPL</string>

  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>

  <key>CFBundleVersion</key>
  <string>${VERSION}</string>

  <key>LSMinimumSystemVersion</key>
  <string>10.15</string>

  <key>NSHighResolutionCapable</key>
  <true/>

  <key>CFBundleIconFile</key>
  <string>${APP_NAME}</string>
</dict>
</plist>
EOF_PLIST

ICON_PNG="assets/icons/icon-512.png"
ICONSET="$(mktemp -d)/${APP_NAME}.iconset"
mkdir -p "${ICONSET}"

sips -z 16 16     "${ICON_PNG}" --out "${ICONSET}/icon_16x16.png"
sips -z 32 32     "${ICON_PNG}" --out "${ICONSET}/icon_16x16@2x.png"
sips -z 32 32     "${ICON_PNG}" --out "${ICONSET}/icon_32x32.png"
sips -z 64 64     "${ICON_PNG}" --out "${ICONSET}/icon_32x32@2x.png"
sips -z 128 128   "${ICON_PNG}" --out "${ICONSET}/icon_128x128.png"
sips -z 256 256   "${ICON_PNG}" --out "${ICONSET}/icon_128x128@2x.png"
sips -z 256 256   "${ICON_PNG}" --out "${ICONSET}/icon_256x256.png"
sips -z 512 512   "${ICON_PNG}" --out "${ICONSET}/icon_256x256@2x.png"
sips -z 512 512   "${ICON_PNG}" --out "${ICONSET}/icon_512x512.png"
cp "${ICON_PNG}" "${ICONSET}/icon_512x512@2x.png"

iconutil -c icns "${ICONSET}" -o "${RES_DIR}/${APP_NAME}.icns"

echo "Built ${APP_DIR}"
