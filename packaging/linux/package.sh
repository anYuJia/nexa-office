#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PACKAGE_VERSION="${PACKAGE_VERSION:-${VERSION:-0.1.0}}"
RELEASE_VERSION="${RELEASE_VERSION:-$PACKAGE_VERSION}"
if [[ -n "${DEB_VERSION:-}" ]]; then
  :
elif [[ "$RELEASE_VERSION" == *-* ]]; then
  DEB_VERSION="${RELEASE_VERSION%%-*}~${RELEASE_VERSION#*-}"
else
  DEB_VERSION="$RELEASE_VERSION"
fi
HOST_ARCH="${ARCH:-$(uname -m)}"

case "$HOST_ARCH" in
  x86_64|amd64) DEB_ARCH="amd64" ;;
  aarch64|arm64) DEB_ARCH="arm64" ;;
  *) echo "unsupported Debian architecture: $HOST_ARCH" >&2; exit 2 ;;
esac

OUT="$ROOT/target/package"
STAGE="$OUT/nexa-office_${RELEASE_VERSION}_${DEB_ARCH}"
DEB="$OUT/nexa-office_${RELEASE_VERSION}_${DEB_ARCH}.deb"
TAR="$OUT/nexa-office_${RELEASE_VERSION}_${DEB_ARCH}.tar.gz"

cargo build --release --locked -p nexa-app
rm -rf "$STAGE" "$DEB" "$TAR"
mkdir -p "$STAGE/usr/bin" "$STAGE/usr/share/applications" "$STAGE/usr/share/mime/packages"
install -m 0755 "$ROOT/target/release/nexa-office" "$STAGE/usr/bin/nexa-office"
install -m 0644 "$ROOT/packaging/linux/nexa-office.desktop" "$STAGE/usr/share/applications/nexa-office.desktop"
install -m 0644 "$ROOT/packaging/linux/nexa-office.xml" "$STAGE/usr/share/mime/packages/nexa-office.xml"

mkdir -p "$STAGE/DEBIAN"
cat > "$STAGE/DEBIAN/control" <<EOF
Package: nexa-office
Version: $DEB_VERSION
Section: office
Priority: optional
Architecture: $DEB_ARCH
Maintainer: Nexa Office contributors
Description: Native lightweight Rust office suite
EOF

dpkg-deb --root-owner-group --build "$STAGE" "$DEB"
tar -C "$STAGE" -czf "$TAR" usr

echo "package=$DEB"
echo "portable=$TAR"
