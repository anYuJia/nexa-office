#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${VERSION:-0.1.0}"
APP="$ROOT/target/package/Nexa Office.app"
PKG="$ROOT/target/package/NexaOffice-${VERSION}.pkg"

cargo build --release --locked -p nexa-app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp "$ROOT/target/release/nexa-office" "$APP/Contents/MacOS/nexa-office"
cp "$ROOT/packaging/macos/Info.plist" "$APP/Contents/Info.plist"

if command -v pkgbuild >/dev/null 2>&1; then
  pkgbuild --root "$APP" --identifier com.nexa.office --version "$VERSION" "$PKG"
else
  ditto -c -k --sequesterRsrc --keepParent "$APP" "$ROOT/target/package/NexaOffice-${VERSION}-macOS.zip"
fi
