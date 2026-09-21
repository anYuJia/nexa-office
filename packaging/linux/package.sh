#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VERSION="${VERSION:-0.1.0}"
ARCH="${ARCH:-$(uname -m)}"
STAGE="$ROOT/target/package/nexa-office_${VERSION}_${ARCH}"

cargo build --release --locked -p nexa-app
rm -rf "$STAGE"
mkdir -p "$STAGE/usr/bin" "$STAGE/usr/share/applications" "$STAGE/usr/share/mime/packages"
install -m 0755 "$ROOT/target/release/nexa-office" "$STAGE/usr/bin/nexa-office"
install -m 0644 "$ROOT/packaging/linux/nexa-office.desktop" "$STAGE/usr/share/applications/nexa-office.desktop"
install -m 0644 "$ROOT/packaging/linux/nexa-office.xml" "$STAGE/usr/share/mime/packages/nexa-office.xml"

mkdir -p "$STAGE/DEBIAN"
cat > "$STAGE/DEBIAN/control" <<EOF
Package: nexa-office
Version: $VERSION
Section: office
Priority: optional
Architecture: amd64
Maintainer: Nexa Office contributors
Description: Native lightweight Rust office suite
EOF

if command -v dpkg-deb >/dev/null 2>&1; then
  dpkg-deb --build "$STAGE" "$ROOT/target/package/nexa-office_${VERSION}_${ARCH}.deb"
else
  tar -C "$STAGE" -czf "$ROOT/target/package/nexa-office_${VERSION}_${ARCH}.tar.gz" usr
fi
