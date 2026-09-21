#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${VERSION:-0.1.0}"
OUT="$ROOT/target/package"
STAGE="$OUT/macos-root"
APP="$STAGE/Applications/Nexa Office.app"
PKG="$OUT/NexaOffice-${VERSION}.pkg"
ZIP="$OUT/NexaOffice-${VERSION}-macOS.zip"

cargo build --release --locked -p nexa-app
rm -rf "$STAGE" "$PKG" "$ZIP"
mkdir -p "$APP/Contents/MacOS"
install -m 0755 "$ROOT/target/release/nexa-office" "$APP/Contents/MacOS/nexa-office"
install -m 0644 "$ROOT/packaging/macos/Info.plist" "$APP/Contents/Info.plist"

/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $VERSION" "$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $VERSION" "$APP/Contents/Info.plist" 2>/dev/null || /usr/libexec/PlistBuddy -c "Add :CFBundleVersion string $VERSION" "$APP/Contents/Info.plist"

pkgbuild --root "$STAGE" --install-location "/" --identifier com.nexa.office --version "$VERSION" "$PKG"
ditto -c -k --sequesterRsrc --keepParent "$APP" "$ZIP"

echo "package=$PKG"
echo "portable=$ZIP"
