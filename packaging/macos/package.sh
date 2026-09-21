#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PACKAGE_VERSION="${PACKAGE_VERSION:-${VERSION:-0.1.0}}"
RELEASE_VERSION="${RELEASE_VERSION:-$PACKAGE_VERSION}"
OUT="$ROOT/target/package"
STAGE="$OUT/macos-root"
APP="$STAGE/Applications/Nexa Office.app"
PKG="$OUT/NexaOffice-${RELEASE_VERSION}.pkg"
ZIP="$OUT/NexaOffice-${RELEASE_VERSION}-macOS.zip"

cargo build --release --locked -p nexa-app
rm -rf "$STAGE" "$PKG" "$ZIP"
mkdir -p "$APP/Contents/MacOS"
install -m 0755 "$ROOT/target/release/nexa-office" "$APP/Contents/MacOS/nexa-office"
install -m 0644 "$ROOT/packaging/macos/Info.plist" "$APP/Contents/Info.plist"

/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $PACKAGE_VERSION" "$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $PACKAGE_VERSION" "$APP/Contents/Info.plist" 2>/dev/null || /usr/libexec/PlistBuddy -c "Add :CFBundleVersion string $PACKAGE_VERSION" "$APP/Contents/Info.plist"

pkgbuild --root "$STAGE" --install-location "/" --identifier com.nexa.office --version "$PACKAGE_VERSION" "$PKG"
ditto -c -k --sequesterRsrc --keepParent "$APP" "$ZIP"

echo "package=$PKG"
echo "portable=$ZIP"
